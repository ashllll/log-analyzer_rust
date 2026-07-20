import { createContext, useContext, useEffect, useMemo, useState } from "react";

export type AppearanceMode = "system" | "light" | "dark";
type ResolvedAppearance = Exclude<AppearanceMode, "system">;

interface AppearanceValue {
  mode: AppearanceMode;
  resolvedMode: ResolvedAppearance;
  setMode: (mode: AppearanceMode) => void;
}

const STORAGE_KEY = "log-analyzer-appearance";
const DARK_QUERY = "(prefers-color-scheme: dark)";
const AppearanceContext = createContext<AppearanceValue | null>(null);

function readStoredMode(): AppearanceMode {
  const stored = localStorage.getItem(STORAGE_KEY);
  return stored === "light" || stored === "dark" || stored === "system"
    ? stored
    : "system";
}

function resolveSystemMode(): ResolvedAppearance {
  return window.matchMedia(DARK_QUERY).matches ? "dark" : "light";
}

export function AppearanceProvider({
  children,
}: {
  children: React.ReactNode;
}) {
  const [mode, setModeState] = useState<AppearanceMode>(readStoredMode);
  const [systemMode, setSystemMode] =
    useState<ResolvedAppearance>(resolveSystemMode);
  const resolvedMode = mode === "system" ? systemMode : mode;

  useEffect(() => {
    const media = window.matchMedia(DARK_QUERY);
    const handleChange = (event: MediaQueryListEvent) =>
      setSystemMode(event.matches ? "dark" : "light");
    setSystemMode(media.matches ? "dark" : "light");
    media.addEventListener("change", handleChange);
    return () => media.removeEventListener("change", handleChange);
  }, []);

  useEffect(() => {
    document.documentElement.dataset.theme = resolvedMode;
    document.documentElement.style.colorScheme = resolvedMode;
  }, [resolvedMode]);

  const value = useMemo<AppearanceValue>(
    () => ({
      mode,
      resolvedMode,
      setMode: (nextMode) => {
        localStorage.setItem(STORAGE_KEY, nextMode);
        setModeState(nextMode);
      },
    }),
    [mode, resolvedMode]
  );

  return (
    <AppearanceContext.Provider value={value}>
      {children}
    </AppearanceContext.Provider>
  );
}

export function useAppearance(): AppearanceValue {
  const value = useContext(AppearanceContext);
  if (!value)
    throw new Error("useAppearance must be used within AppearanceProvider");
  return value;
}
