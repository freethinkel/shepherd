import { RGBA, type ThemeMode } from "@opentui/core";

import type { Palette } from "../types/palette.ts";

// indexed/default colours render as ANSI codes — the terminal supplies the palette, so these
// match the user's terminal theme without a single hardcoded hex
const term = {
  fg: RGBA.defaultForeground(),
  bg: RGBA.defaultBackground(),
  red: RGBA.fromIndex(1),
  green: RGBA.fromIndex(2),
  yellow: RGBA.fromIndex(3),
  blue: RGBA.fromIndex(4),
  cyan: RGBA.fromIndex(6),
  gray: RGBA.fromIndex(8),
  lightGray: RGBA.fromIndex(7),
  brightBlue: RGBA.fromIndex(12),
};

/** Used until the terminal answers the colour query, and forever if it never does. */
export const THEMES: Record<ThemeMode, Palette> = {
  dark: {
    fg: term.fg,
    bg: term.bg,
    accent: term.brightBlue,
    danger: term.red,
    success: term.green,
    warning: term.yellow,
    muted: term.gray,
    border: term.gray,
    selectionBg: term.gray,
  },
  light: {
    fg: term.fg,
    bg: term.bg,
    accent: term.blue,
    danger: term.red,
    success: term.green,
    warning: term.yellow,
    muted: term.gray,
    border: term.lightGray,
    selectionBg: term.lightGray,
  },
};
