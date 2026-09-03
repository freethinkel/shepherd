import type { ThemeMode } from "@opentui/core";
import {
  createContext,
  createMemo,
  createSignal,
  useContext,
  type Accessor,
  type JSX,
} from "solid-js";

import { THEMES } from "../constants/themes.ts";
import { buildPalette, type TerminalColors } from "../helpers/palette.ts";
import type { Palette } from "../types/palette.ts";

const ThemeContext = createContext<Accessor<Palette>>();

const [themeMode, setThemeMode] = createSignal<ThemeMode>("dark");
const [terminalColors, setTerminalColors] = createSignal<TerminalColors | null>(null);

export { setThemeMode, setTerminalColors };

export const ThemeProvider = (props: { children: JSX.Element }) => {
  // Memoised so the palette keeps its identity: rebuilding one per render would leak the
  // native colour handles the renderables hold on to.
  const palette = createMemo(() => {
    const reported = terminalColors();
    return reported ? buildPalette(reported) : THEMES[themeMode()];
  });
  return <ThemeContext.Provider value={palette}>{props.children}</ThemeContext.Provider>;
};

export const useTheme = (): Accessor<Palette> => {
  const theme = useContext(ThemeContext);
  if (!theme) throw new Error("useTheme must be used within ThemeProvider");
  return theme;
};
