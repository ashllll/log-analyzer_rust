import { Monitor, Moon, Sun } from "lucide-react";
import { useAppearance, type AppearanceMode } from "./AppearanceProvider";

const nextMode: Record<AppearanceMode, AppearanceMode> = {
  system: "light",
  light: "dark",
  dark: "system",
};

export function AppearanceControl() {
  const { mode, setMode } = useAppearance();
  const Icon = mode === "system" ? Monitor : mode === "light" ? Sun : Moon;
  return (
    <button
      type="button"
      className="ui-pressable grid h-8 w-8 place-items-center rounded-full border border-border-base bg-bg-elevated/70 text-text-muted hover:bg-bg-hover hover:text-text-main"
      aria-label={`Appearance: ${mode}. Activate to switch to ${nextMode[mode]}.`}
      title={`Appearance: ${mode}`}
      onClick={() => setMode(nextMode[mode])}
    >
      <Icon size={14} aria-hidden="true" />
    </button>
  );
}
