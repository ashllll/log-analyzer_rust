import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AppearanceProvider, useAppearance } from "../AppearanceProvider";

type ThemeListener = (event: MediaQueryListEvent) => void;

function AppearanceProbe() {
  const { mode, resolvedMode, setMode } = useAppearance();
  return (
    <div>
      <output>
        {mode}:{resolvedMode}
      </output>
      <button onClick={() => setMode("light")}>Light</button>
    </div>
  );
}

describe("AppearanceProvider", () => {
  let listener: ThemeListener | undefined;

  beforeEach(() => {
    localStorage.clear();
    delete document.documentElement.dataset.theme;
    listener = undefined;
    window.matchMedia = jest.fn().mockImplementation(() => ({
      matches: true,
      media: "(prefers-color-scheme: dark)",
      onchange: null,
      addEventListener: (_type: string, next: ThemeListener) => {
        listener = next;
      },
      removeEventListener: jest.fn(),
      addListener: jest.fn(),
      removeListener: jest.fn(),
      dispatchEvent: jest.fn(),
    }));
  });

  it("resolves system appearance and applies it to the document", () => {
    render(
      <AppearanceProvider>
        <AppearanceProbe />
      </AppearanceProvider>
    );

    expect(screen.getByText("system:dark")).toBeInTheDocument();
    expect(document.documentElement.dataset.theme).toBe("dark");
  });

  it("persists an explicit appearance selection", async () => {
    const user = userEvent.setup();
    render(
      <AppearanceProvider>
        <AppearanceProbe />
      </AppearanceProvider>
    );

    await user.click(screen.getByRole("button", { name: "Light" }));

    expect(screen.getByText("light:light")).toBeInTheDocument();
    expect(document.documentElement.dataset.theme).toBe("light");
    expect(localStorage.getItem("log-analyzer-appearance")).toBe("light");
  });

  it("reacts to operating-system changes while in system mode", () => {
    render(
      <AppearanceProvider>
        <AppearanceProbe />
      </AppearanceProvider>
    );

    act(() => listener?.({ matches: false } as MediaQueryListEvent));

    expect(screen.getByText("system:light")).toBeInTheDocument();
    expect(document.documentElement.dataset.theme).toBe("light");
  });
});
