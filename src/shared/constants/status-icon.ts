import type { RunStatus } from "../domain/types.ts";

export const ICONS: Record<RunStatus, string> = {
  queued: "○",
  starting: "●",
  planning: "◔",
  working: "●",
  blocked: "◉",
  validating: "◍",
  creating_change: "◍",
  review: "◍",
  completed: "✓",
  failed: "✗",
};

export const icon = (status: RunStatus) => ICONS[status] ?? "·";
