import type { ScrollBoxRenderable } from "@opentui/core";

/** Rows kept between the cursor and the edge of the view — vim's scrolloff. */
export const SCROLL_OFF = 2;

/**
 * Scrolls the box just enough to bring the given line range into view, keeping SCROLL_OFF rows
 * of lead. The scrollbox clamps both ends, so the lead simply runs out at the top and bottom.
 */
export const keepVisible = (
  scroll: ScrollBoxRenderable | undefined,
  top: number,
  height = 1,
  pad = SCROLL_OFF,
) => {
  if (!scroll) return;
  const first = Math.max(0, top - pad);
  const last = top + height + pad;
  if (first < scroll.scrollTop) scroll.scrollTop = first;
  else if (last > scroll.scrollTop + scroll.viewport.height)
    scroll.scrollTop = last - scroll.viewport.height;
};
