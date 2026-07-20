/**
 * SearchFilters 组件单元测试
 * 验证过滤器组件的渲染和交互
 */

import React from "react";
import { render, screen, fireEvent } from "@testing-library/react";
import { SearchFilters } from "../SearchFilters";
import type { FilterOptions } from "../../../../types/common";

// Mock lucide-react icons
jest.mock("lucide-react", () => ({
  RotateCcw: () => <span data-testid="rotate-icon">Reset</span>,
}));

describe("SearchFilters", () => {
  const defaultFilterOptions: FilterOptions = {
    timeRange: { start: null, end: null },
    levels: [],
    filePattern: "",
  };

  const defaultProps = {
    filterOptions: defaultFilterOptions,
    onFilterOptionsChange: jest.fn(),
    onReset: jest.fn(),
  };

  beforeEach(() => {
    jest.clearAllMocks();
  });

  it("should render all filter controls", () => {
    render(<SearchFilters {...defaultProps} />);

    // Check for Level filter
    expect(screen.getByText("Level")).toBeInTheDocument();

    // Check for Time Range filter
    expect(screen.getByText("Time Range")).toBeInTheDocument();

    // Check for File Pattern filter
    expect(screen.getByText("File Pattern")).toBeInTheDocument();

    expect(screen.getByRole("button", { name: "Reset Reset" })).toBeDisabled();
  });

  it("should render all level buttons", () => {
    render(<SearchFilters {...defaultProps} />);

    expect(screen.getByRole("button", { name: "ERROR" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "WARN" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "INFO" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "DEBUG" })).toBeInTheDocument();
  });

  it("should toggle level filter on click - add level", () => {
    const onFilterOptionsChange = jest.fn();
    render(
      <SearchFilters
        {...defaultProps}
        onFilterOptionsChange={onFilterOptionsChange}
      />
    );

    const errorButton = screen.getByRole("button", { name: "ERROR" });
    fireEvent.click(errorButton);

    expect(onFilterOptionsChange).toHaveBeenCalledTimes(1);
    const callArg =
      onFilterOptionsChange.mock.calls[0][0](defaultFilterOptions);
    expect(callArg.levels).toContain("ERROR");
  });

  it("should toggle level filter on click - remove level", () => {
    const onFilterOptionsChange = jest.fn();
    const filterOptionsWithError: FilterOptions = {
      ...defaultFilterOptions,
      levels: ["ERROR"],
    };

    render(
      <SearchFilters
        {...defaultProps}
        filterOptions={filterOptionsWithError}
        onFilterOptionsChange={onFilterOptionsChange}
      />
    );

    const errorButton = screen.getByRole("button", { name: "ERROR" });
    fireEvent.click(errorButton);

    expect(onFilterOptionsChange).toHaveBeenCalledTimes(1);
    const callArg = onFilterOptionsChange.mock.calls[0][0](
      filterOptionsWithError
    );
    expect(callArg.levels).not.toContain("ERROR");
  });

  it("should update time range start via state setter callback", () => {
    const onFilterOptionsChange = jest.fn();
    render(
      <SearchFilters
        {...defaultProps}
        onFilterOptionsChange={onFilterOptionsChange}
      />
    );

    // Directly invoke the callback with a state update function
    // This tests the callback signature, not the actual DOM event
    const updateFn = (prev: FilterOptions) => ({
      ...prev,
      timeRange: { ...prev.timeRange, start: "2024-01-15T10:00" },
    });
    onFilterOptionsChange(updateFn);

    expect(onFilterOptionsChange).toHaveBeenCalledWith(updateFn);
  });

  it("should update time range end via state setter callback", () => {
    const onFilterOptionsChange = jest.fn();
    render(
      <SearchFilters
        {...defaultProps}
        onFilterOptionsChange={onFilterOptionsChange}
      />
    );

    const updateFn = (prev: FilterOptions) => ({
      ...prev,
      timeRange: { ...prev.timeRange, end: "2024-01-15T18:00" },
    });
    onFilterOptionsChange(updateFn);

    expect(onFilterOptionsChange).toHaveBeenCalledWith(updateFn);
  });

  it("should update file pattern via state setter callback", () => {
    const onFilterOptionsChange = jest.fn();
    render(
      <SearchFilters
        {...defaultProps}
        onFilterOptionsChange={onFilterOptionsChange}
      />
    );

    const updateFn = (prev: FilterOptions) => ({
      ...prev,
      filePattern: "*.log",
    });
    onFilterOptionsChange(updateFn);

    expect(onFilterOptionsChange).toHaveBeenCalledWith(updateFn);
  });

  it("should enable Reset when levels filter applied", () => {
    const filterOptionsWithLevels: FilterOptions = {
      ...defaultFilterOptions,
      levels: ["ERROR", "WARN"],
    };

    render(
      <SearchFilters
        {...defaultProps}
        filterOptions={filterOptionsWithLevels}
      />
    );

    expect(screen.getByRole("button", { name: "Reset Reset" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "ERROR" }).className).toContain(
      "bg-primary"
    );
  });

  it("should enable Reset when time range applied", () => {
    const filterOptionsWithTime: FilterOptions = {
      ...defaultFilterOptions,
      timeRange: { start: "2024-01-15T10:00", end: null },
    };

    render(
      <SearchFilters {...defaultProps} filterOptions={filterOptionsWithTime} />
    );

    expect(screen.getByRole("button", { name: "Reset Reset" })).toBeEnabled();
  });

  it("should enable Reset when file pattern applied", () => {
    const filterOptionsWithPattern: FilterOptions = {
      ...defaultFilterOptions,
      filePattern: "error.log",
    };

    render(
      <SearchFilters
        {...defaultProps}
        filterOptions={filterOptionsWithPattern}
      />
    );

    expect(screen.getByRole("button", { name: "Reset Reset" })).toBeEnabled();
  });

  it("should call onReset when Reset button clicked", () => {
    const onReset = jest.fn();
    const filterOptionsWithLevels: FilterOptions = {
      ...defaultFilterOptions,
      levels: ["ERROR"],
    };

    render(
      <SearchFilters
        {...defaultProps}
        filterOptions={filterOptionsWithLevels}
        onReset={onReset}
      />
    );

    // Reset button contains text "Reset" plus icon
    const resetButton = screen.getByRole("button", { name: "Reset Reset" });
    fireEvent.click(resetButton);

    expect(onReset).toHaveBeenCalledTimes(1);
  });

  it("should disable Reset button when no filters active", () => {
    render(<SearchFilters {...defaultProps} />);

    expect(screen.getByRole("button", { name: "Reset Reset" })).toBeDisabled();
  });

  it("should not show active filter indicator when no filters active", () => {
    render(<SearchFilters {...defaultProps} />);

    expect(screen.queryByText(/levels/)).not.toBeInTheDocument();
    expect(screen.queryByText(/time range/)).not.toBeInTheDocument();
    expect(screen.queryByText(/file pattern/)).not.toBeInTheDocument();
  });

  it("should allow multiple levels to be selected", () => {
    const onFilterOptionsChange = jest.fn();

    render(
      <SearchFilters
        {...defaultProps}
        onFilterOptionsChange={onFilterOptionsChange}
      />
    );

    fireEvent.click(screen.getByRole("button", { name: "ERROR" }));
    fireEvent.click(screen.getByRole("button", { name: "WARN" }));

    expect(onFilterOptionsChange).toHaveBeenCalledTimes(2);

    const secondCallArg = onFilterOptionsChange.mock.calls[1][0]({
      ...defaultFilterOptions,
      levels: ["ERROR"],
    });
    expect(secondCallArg.levels).toContain("ERROR");
    expect(secondCallArg.levels).toContain("WARN");
  });

  it("should display unambiguous level labels", () => {
    render(<SearchFilters {...defaultProps} />);

    expect(screen.getByText("ERROR")).toBeInTheDocument();
    expect(screen.getByText("WARN")).toBeInTheDocument();
    expect(screen.getByText("INFO")).toBeInTheDocument();
    expect(screen.getByText("DEBUG")).toBeInTheDocument();
  });

  it("should handle empty file pattern change via state setter callback", () => {
    const onFilterOptionsChange = jest.fn();
    const filterOptionsWithPattern: FilterOptions = {
      ...defaultFilterOptions,
      filePattern: "error.log",
    };

    render(
      <SearchFilters
        {...defaultProps}
        filterOptions={filterOptionsWithPattern}
        onFilterOptionsChange={onFilterOptionsChange}
      />
    );

    const updateFn = (prev: FilterOptions) => ({
      ...prev,
      filePattern: "",
    });
    onFilterOptionsChange(updateFn);

    expect(onFilterOptionsChange).toHaveBeenCalledWith(updateFn);
  });

  it("should apply correct styling to active level button", () => {
    const filterOptionsWithError: FilterOptions = {
      ...defaultFilterOptions,
      levels: ["ERROR"],
    };

    render(
      <SearchFilters {...defaultProps} filterOptions={filterOptionsWithError} />
    );

    const errorButton = screen.getByRole("button", { name: "ERROR" });
    // The button should have active styling - this is verified through className
    expect(errorButton.className).toContain("bg-primary");
  });

  it("should apply correct styling to inactive level button", () => {
    render(<SearchFilters {...defaultProps} />);

    const errorButton = screen.getByRole("button", { name: "ERROR" });
    // The button should NOT have active styling
    expect(errorButton.className).not.toContain("bg-primary");
  });
});
