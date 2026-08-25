import type {
  AgentStatus,
  ProjectStatus,
  RunStatus,
  TaskStatus,
} from "./types.ts";

/** Herdr owns agent state. We only normalize its vocabulary. */
export function normalizeAgentStatus(raw: string | undefined): AgentStatus {
  switch (raw) {
    case "working":
    case "blocked":
    case "idle":
    case "done":
      return raw;
    default:
      return "unknown";
  }
}

const RUN_ACTIVE: RunStatus[] = [
  "queued",
  "starting",
  "working",
  "blocked",
  "validating",
  "creating_change",
  "review",
];

export function isRunActive(status: RunStatus): boolean {
  return RUN_ACTIVE.includes(status);
}

/** Needs a human: the orchestrator cannot move this run any further. */
export function needsAttention(status: RunStatus): boolean {
  return status === "blocked" || status === "failed" || status === "review";
}

/** Tracker task status derived from the run status. */
export function taskStatusForRun(status: RunStatus): TaskStatus {
  switch (status) {
    case "queued":
      return "todo";
    case "blocked":
      return "waiting_for_agent";
    case "review":
      return "in_review";
    case "completed":
      return "done";
    default:
      return "in_progress";
  }
}

const PROJECT_PRIORITY: ProjectStatus[] = [
  "blocked",
  "working",
  "validating",
  "review",
  "queued",
  "idle",
];

function runToProjectStatus(status: RunStatus): ProjectStatus | undefined {
  switch (status) {
    case "blocked":
      return "blocked";
    case "starting":
    case "working":
      return "working";
    case "validating":
    case "creating_change":
      return "validating";
    case "review":
      return "review";
    case "queued":
      return "queued";
    default:
      return undefined; // completed / failed do not affect the live project status
  }
}

/** Project status is derived from its runs rather than stored separately. */
export function deriveProjectStatus(
  runStatuses: RunStatus[],
  queuedTasks = 0,
): ProjectStatus {
  const candidates = runStatuses
    .map(runToProjectStatus)
    .filter((s): s is ProjectStatus => s !== undefined);
  if (queuedTasks > 0) candidates.push("queued");
  for (const status of PROJECT_PRIORITY) {
    if (candidates.includes(status)) return status;
  }
  return "idle";
}
