import { RGBA, type CliRenderer } from "@opentui/core";

import type { Palette } from "../types/palette.ts";
import { contrast, ensureContrast, mix } from "./contrast.ts";

export type Rgb = [number, number, number];
export type TerminalColors = { fg: Rgb; bg: Rgb; palette: Rgb[] };

const SLOTS = 16;
/** Body text has to clear this; decorative text only needs the lower bar. */
const TEXT_CONTRAST = 4.5;
const MUTED_CONTRAST = 3;

const fromHex = (hex: string | null): Rgb | null => {
  const match = /^#?([0-9a-f]{6})$/i.exec(hex ?? "");
  if (!match) return null;
  const value = match[1]!;
  return [0, 2, 4].map((i) => parseInt(value.slice(i, i + 2), 16)) as Rgb;
};

const rgba = ([r, g, b]: Rgb) => RGBA.fromInts(r, g, b, 255);

/**
 * Asks the renderer for the terminal's real colours. `getPalette` owns the OSC conversation,
 * so this only converts the answer. Null means the terminal stayed silent or answered partially,
 * and the caller falls back to ANSI indices.
 */
export const detectPalette = async (
  renderer: CliRenderer,
  timeout = 500,
): Promise<TerminalColors | null> => {
  try {
    const colors = await renderer.getPalette({ timeout });
    const fg = fromHex(colors.defaultForeground);
    const bg = fromHex(colors.defaultBackground);
    if (!fg || !bg) return null;
    const palette: Rgb[] = [];
    for (let slot = 0; slot < SLOTS; slot++) {
      const parsed = fromHex(colors.palette[slot] ?? null);
      if (!parsed) return null;
      palette.push(parsed);
    }
    return { fg, bg, palette };
  } catch {
    return null;
  }
};

/** ANSI ships each hue twice; take whichever half reads better on this background. */
const pickHue = (palette: Rgb[], normal: number, bright: number, bg: Rgb): Rgb => {
  const a = palette[normal]!;
  const b = palette[bright]!;
  return contrast(a, bg) >= contrast(b, bg) ? a : b;
};

/**
 * Turns the colours the terminal reported into the roles the UI needs.
 *
 * ANSI has no `border` or `selection` slot, so those are mixed from the real foreground and
 * background instead of borrowing a hue. Everything carrying text is pushed to a legible
 * contrast ratio, which is the part indices alone cannot guarantee.
 */
export const buildPalette = (colors: TerminalColors): Palette => {
  const { bg, palette } = colors;
  const fg = ensureContrast(colors.fg, bg, TEXT_CONTRAST);
  const hue = (normal: number, bright: number) =>
    rgba(ensureContrast(pickHue(palette, normal, bright, bg), bg, TEXT_CONTRAST));

  return {
    fg: rgba(fg),
    bg: rgba(bg),
    muted: rgba(ensureContrast(mix(fg, bg, 0.45), bg, MUTED_CONTRAST)),
    border: rgba(mix(bg, fg, 0.25)),
    selectionBg: rgba(mix(bg, fg, 0.14)),
    accent: hue(4, 12),
    danger: hue(1, 9),
    success: hue(2, 10),
    warning: hue(3, 11),
  };
};

/**
 * Keeps the palette in step with the terminal.
 *
 * `theme_mode` only fires when the terminal flips between dark and light, so swapping one dark
 * theme for another is announced by nothing at all. Focus coming back is the practical second
 * cue — changing a theme means leaving the terminal and returning to it. A terminal that ignored
 * the first query is not asked again on focus: that would burn a timeout per activation.
 */
export const watchPalette = (
  renderer: CliRenderer,
  onColors: (colors: TerminalColors) => void,
): (() => void) => {
  let supported = true;

  const reload = async () => {
    renderer.clearPaletteCache();
    const colors = await detectPalette(renderer);
    supported = colors !== null;
    if (colors) onColors(colors);
  };

  const onThemeMode = () => void reload();
  const onFocus = () => {
    if (supported) void reload();
  };

  void reload();
  renderer.on("theme_mode", onThemeMode);
  renderer.on("focus", onFocus);
  return () => {
    renderer.off("theme_mode", onThemeMode);
    renderer.off("focus", onFocus);
  };
};
