import type { RunStatus } from "../../../shared/domain/types.ts";
import type { Palette } from "../../theme/index.ts";

export { ICONS } from "../../../shared/constants/status-icon.ts";

/** Which palette role carries each run status. */
export const STATUS_COLOR: Record<RunStatus, keyof Palette> = {
  queued: "muted",
  starting: "accent",
  planning: "accent",
  working: "accent",
  blocked: "warning",
  validating: "fg",
  creating_change: "fg",
  checking: "fg",
  review: "accent",
  completed: "success",
  failed: "danger",
};
