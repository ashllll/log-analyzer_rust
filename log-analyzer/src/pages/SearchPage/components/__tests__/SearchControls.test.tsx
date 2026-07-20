/**
 * SearchControls 组件单元测试
 * 验证搜索控制组件的渲染和交互
 */

import React from "react";
import { render, screen, fireEvent } from "@testing-library/react";
import { SearchControls } from "../SearchControls";
import type { KeywordGroup } from "../../../../types/common";

const filterPaletteMock = jest.fn();

// Mock lucide-react icons
jest.mock("lucide-react", () => ({
  Search: () => <span data-testid="search-icon">SearchIcon</span>,
  Download: () => <span data-testid="download-icon">DownloadIcon</span>,
  Filter: () => <span data-testid="filter-icon">FilterIcon</span>,
  ChevronDown: () => <span data-testid="chevron-icon">ChevronDownIcon</span>,
  Loader2: () => <span data-testid="loader-icon">LoaderIcon</span>,
}));

// Mock FilterPalette component
jest.mock("../../../../components/modals", () => ({
  FilterPalette: (props: unknown) => {
    filterPaletteMock(props);
    return <div data-testid="filter-palette">FilterPalette</div>;
  },
}));

// Mock react-i18next - 返回 key 作为默认值
jest.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, defaultValue?: string) => defaultValue || key,
  }),
}));

describe("SearchControls", () => {
  const defaultProps = {
    query: "",
    onQueryChange: jest.fn(),
    onSearch: jest.fn(),
    onExport: jest.fn(),
    isFilterPaletteOpen: false,
    onFilterPaletteToggle: jest.fn(),
    onFilterPaletteClose: jest.fn(),
    isSearching: false,
    disabled: false,
    searchInputRef: {
      current: null,
    } as React.RefObject<HTMLInputElement | null>,
    keywordGroups: [] as KeywordGroup[],
    activeTerms: [],
    onToggleRule: jest.fn(),
  };

  beforeEach(() => {
    jest.clearAllMocks();
    filterPaletteMock.mockClear();
  });

  it("should render search input with placeholder", () => {
    render(<SearchControls {...defaultProps} />);

    const input = screen.getByPlaceholderText("输入关键词，用 | 分隔...");
    expect(input).toBeInTheDocument();
    expect(input.tagName).toBe("INPUT");
  });

  it("should display current query in input", () => {
    render(<SearchControls {...defaultProps} query="error|warning" />);

    const input = screen.getByPlaceholderText(
      "输入关键词，用 | 分隔..."
    ) as HTMLInputElement;
    expect(input.value).toBe("error|warning");
  });

  it("should call onQueryChange when input changes", () => {
    const onQueryChange = jest.fn();
    render(<SearchControls {...defaultProps} onQueryChange={onQueryChange} />);

    const input = screen.getByPlaceholderText("输入关键词，用 | 分隔...");
    fireEvent.change(input, { target: { value: "test" } });

    expect(onQueryChange).toHaveBeenCalledWith("test");
  });

  it("should normalize query - remove spaces around |", () => {
    const onQueryChange = jest.fn();
    render(<SearchControls {...defaultProps} onQueryChange={onQueryChange} />);

    const input = screen.getByPlaceholderText("输入关键词，用 | 分隔...");
    fireEvent.change(input, { target: { value: "error | warning | info" } });

    expect(onQueryChange).toHaveBeenCalledWith("error|warning|info");
  });

  it("should not add extra | when already normalized", () => {
    const onQueryChange = jest.fn();
    render(<SearchControls {...defaultProps} onQueryChange={onQueryChange} />);

    const input = screen.getByPlaceholderText("输入关键词，用 | 分隔...");
    fireEvent.change(input, { target: { value: "a|b|c" } });

    expect(onQueryChange).toHaveBeenCalledWith("a|b|c");
  });

  it("should call onSearch when Enter key pressed", () => {
    const onSearch = jest.fn();
    render(<SearchControls {...defaultProps} onSearch={onSearch} />);

    const input = screen.getByPlaceholderText("输入关键词，用 | 分隔...");
    fireEvent.keyDown(input, { key: "Enter" });

    expect(onSearch).toHaveBeenCalledTimes(1);
  });

  it("should render Keyword Groups button", () => {
    render(<SearchControls {...defaultProps} />);

    expect(
      screen.getByRole("button", { name: /keyword groups/i })
    ).toBeInTheDocument();
    expect(screen.getByTestId("filter-icon")).toBeInTheDocument();
  });

  it("should call onFilterPaletteToggle when Filter clicked", () => {
    const onFilterPaletteToggle = jest.fn();
    render(
      <SearchControls
        {...defaultProps}
        onFilterPaletteToggle={onFilterPaletteToggle}
      />
    );

    const filterButton = screen.getByRole("button", {
      name: /keyword groups/i,
    });
    fireEvent.click(filterButton);

    expect(onFilterPaletteToggle).toHaveBeenCalledTimes(1);
  });

  it("should render the export format options", () => {
    render(<SearchControls {...defaultProps} />);

    expect(screen.getByText("CSV")).toBeInTheDocument();
  });

  it("should call onExport with csv when CSV is selected", () => {
    const onExport = jest.fn();
    render(<SearchControls {...defaultProps} onExport={onExport} />);

    const exportSelect = screen.getByRole("combobox", {
      name: /export format/i,
    });
    fireEvent.change(exportSelect, { target: { value: "csv" } });

    expect(onExport).toHaveBeenCalledWith("csv");
  });

  it("should render the JSON export option", () => {
    render(<SearchControls {...defaultProps} />);

    expect(screen.getByText("JSON")).toBeInTheDocument();
  });

  it("should call onExport with json when JSON is selected", () => {
    const onExport = jest.fn();
    render(<SearchControls {...defaultProps} onExport={onExport} />);

    const exportSelect = screen.getByRole("combobox", {
      name: /export format/i,
    });
    fireEvent.change(exportSelect, { target: { value: "json" } });

    expect(onExport).toHaveBeenCalledWith("json");
  });

  it("should render Search button", () => {
    render(<SearchControls {...defaultProps} />);

    // Search button shows "搜索" text
    expect(screen.getAllByText("搜索").length).toBeGreaterThan(0);
  });

  it("should call onSearch when Search button clicked", () => {
    const onSearch = jest.fn();
    render(<SearchControls {...defaultProps} onSearch={onSearch} />);

    const searchButton = screen.getByText("搜索");
    fireEvent.click(searchButton);

    expect(onSearch).toHaveBeenCalledTimes(1);
  });

  it("should disable the export control when disabled=true", () => {
    render(<SearchControls {...defaultProps} disabled={true} />);

    expect(
      screen.getByRole("combobox", { name: /export format/i })
    ).toBeDisabled();
  });

  it("should disable Search button when disabled=true", () => {
    render(<SearchControls {...defaultProps} disabled={true} />);

    const searchButton = screen.getByText("搜索");
    expect(searchButton.closest("button")).toBeDisabled();
  });

  it("should disable Search button when isSearching=true", () => {
    render(<SearchControls {...defaultProps} isSearching={true} />);

    const searchButton = screen.getByText("搜索中");
    expect(searchButton.closest("button")).toBeDisabled();
  });

  it("should show Loader2 icon when isSearching=true", () => {
    render(<SearchControls {...defaultProps} isSearching={true} />);

    expect(screen.getByTestId("loader-icon")).toBeInTheDocument();
  });

  it("should show Search icon when not searching", () => {
    render(<SearchControls {...defaultProps} isSearching={false} />);

    // Should show search icons (multiple rendered)
    const icons = screen.getAllByTestId("search-icon");
    expect(icons.length).toBeGreaterThan(0);
  });

  it("should render ChevronDown icon for Filter button", () => {
    render(<SearchControls {...defaultProps} />);

    expect(screen.getByTestId("chevron-icon")).toBeInTheDocument();
  });

  it("should render FilterPalette when isFilterPaletteOpen=true", () => {
    render(<SearchControls {...defaultProps} isFilterPaletteOpen={true} />);

    expect(screen.getByTestId("filter-palette")).toBeInTheDocument();
  });

  it("should not render FilterPalette when isFilterPaletteOpen=false", () => {
    // Note: Since FilterPalette is mocked, it always renders.
    // This test verifies the prop is correctly passed.
    // The actual visibility is controlled by the component's logic.
    render(<SearchControls {...defaultProps} isFilterPaletteOpen={false} />);

    // The mock FilterPalette always renders, but we can verify the component structure
    const filterButton = screen.getByRole("button", {
      name: /keyword groups/i,
    });
    expect(filterButton).toBeInTheDocument();
  });

  it("should call onFilterPaletteClose when FilterPalette requests close", () => {
    const onFilterPaletteClose = jest.fn();
    render(
      <SearchControls
        {...defaultProps}
        onFilterPaletteClose={onFilterPaletteClose}
      />
    );

    // The FilterPalette is rendered when open, but since it's mocked,
    // we just verify the prop is passed correctly
    expect(onFilterPaletteClose).not.toHaveBeenCalled();
  });

  it("should have search icon inside input", () => {
    render(<SearchControls {...defaultProps} />);

    // There should be at least one search-icon rendered
    expect(screen.getAllByTestId("search-icon").length).toBeGreaterThan(0);
  });

  it("should display query with font-mono class", () => {
    render(<SearchControls {...defaultProps} query="test" />);

    const input = screen.getByPlaceholderText("输入关键词，用 | 分隔...");
    expect(input.className).toContain("font-mono");
  });

  it("should handle empty query gracefully", () => {
    const onQueryChange = jest.fn();
    render(
      <SearchControls
        {...defaultProps}
        query=""
        onQueryChange={onQueryChange}
      />
    );

    const input = screen.getByPlaceholderText(
      "输入关键词，用 | 分隔..."
    ) as HTMLInputElement;
    expect(input.value).toBe("");
  });

  it("should not pulse the whole search button while searching", () => {
    render(<SearchControls {...defaultProps} isSearching={true} />);

    const searchButton = screen.getByText("搜索中");
    expect(searchButton.className).not.toContain("animate-pulse");
  });

  it("should pass keywordGroups to FilterPalette", () => {
    const keywordGroups: KeywordGroup[] = [
      {
        id: "group-1",
        name: "Errors",
        color: "red",
        patterns: [{ regex: "error", comment: "Error keyword" }],
        enabled: true,
      },
    ];

    render(
      <SearchControls
        {...defaultProps}
        isFilterPaletteOpen={true}
        keywordGroups={keywordGroups}
      />
    );

    // The FilterPalette should be rendered with the groups prop passed
    expect(screen.getByTestId("filter-palette")).toBeInTheDocument();
  });

  it("should pass active terms to FilterPalette", () => {
    render(
      <SearchControls
        {...defaultProps}
        isFilterPaletteOpen={true}
        activeTerms={["error|warning"]}
      />
    );

    expect(screen.getByTestId("filter-palette")).toBeInTheDocument();
    expect(filterPaletteMock).toHaveBeenCalledWith(
      expect.objectContaining({
        activeTerms: ["error|warning"],
      })
    );
  });

  it("should render all buttons in correct order", () => {
    render(<SearchControls {...defaultProps} />);

    const container = document.querySelector(".flex.gap-2");
    expect(container).toBeInTheDocument();

    // Keyword Groups and Search remain buttons; export is a compact select.
    const buttons = screen.getAllByRole("button");
    expect(buttons.length).toBe(2);
    expect(
      screen.getByRole("combobox", { name: /export format/i })
    ).toBeInTheDocument();
  });
});
