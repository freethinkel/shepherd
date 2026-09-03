import type { RGBA } from "@opentui/core";

/** Colour roles the UI asks for by name, so no screen ever spells out a hex. */
export type Palette = {
  fg: RGBA;
  bg: RGBA;
  accent: RGBA;
  danger: RGBA;
  success: RGBA;
  warning: RGBA;
  muted: RGBA;
  /** Divider and frame lines. */
  border: RGBA;
  /** Background of the highlighted row in lists. */
  selectionBg: RGBA;
};
