import type { Rgb } from "./palette.ts";

const channel = (value: number) => {
  const c = value / 255;
  return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
};

/** WCAG relative luminance, 0 for black and 1 for white. */
export const luminance = ([r, g, b]: Rgb): number =>
  0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);

/** WCAG contrast ratio, from 1 (identical) to 21 (black on white). */
export const contrast = (a: Rgb, b: Rgb): number => {
  const [light, dark] = [luminance(a), luminance(b)].toSorted((x, y) => y - x);
  return (light! + 0.05) / (dark! + 0.05);
};

/** Linear blend; `amount` 0 keeps `from`, 1 returns `to`. */
export const mix = (from: Rgb, to: Rgb, amount: number): Rgb =>
  from.map((value, i) => Math.round(value + (to[i]! - value) * amount)) as Rgb;

/**
 * Pushes a colour away from the background until it is legible, blending toward white on a dark
 * background and toward black on a light one. Gives up at the extreme rather than looping, so a
 * colour that cannot reach the target still comes back as readable as it gets.
 */
export const ensureContrast = (color: Rgb, bg: Rgb, ratio: number): Rgb => {
  if (contrast(color, bg) >= ratio) return color;
  const target: Rgb = luminance(bg) < 0.5 ? [255, 255, 255] : [0, 0, 0];
  let best = color;
  for (let amount = 0.1; amount <= 1.0001; amount += 0.1) {
    best = mix(color, target, amount);
    if (contrast(best, bg) >= ratio) return best;
  }
  return best;
};
