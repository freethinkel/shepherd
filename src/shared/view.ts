// One state view for the CLI (and a future TUI). Knows nothing about Linear/GitHub/Herdr.
import { deriveProjectStatus, isRunActive, needsAttention } from "./domain/status.ts";
import type { AgentRun, Change, Project, ProjectStatus, RunStatus, Task } from "./domain/types.ts";
import * as db from "../core/persistence/db.ts";

export interface TaskView {
  task: Task;
  run?: AgentRun | undefined;
  runs: AgentRun[];
  status: RunStatus;
}

export interface ProjectView {
  project: Project;
  status: ProjectStatus;
  tasks: TaskView[];
  counts: {
    working: number;
    blocked: number;
    queued: number;
    review: number;
    done: number;
    failed: number;
  };
  attention: boolean;
}

/**
 * The live picture, which is the configured projects and nothing else. A project dropped from
 * the config keeps its rows — the runs and events of past work are history, not clutter — but a
 * dashboard listing projects nobody orchestrates any more is just wrong. `shepherd runs` and
 * `shepherd events` still see everything.
 */
export function overview(database: db.Db, configured?: Iterable<string>): ProjectView[] {
  const allowed = configured ? new Set(configured) : undefined;
  return db
    .listProjects(database)
    .filter((project) => !allowed || allowed.has(project.id))
    .map((project) => projectView(database, project));
}

export function projectView(database: db.Db, project: Project): ProjectView {
  const tasks: TaskView[] = db.listTasks(database, project.id).map((task) => {
    const runs = db.runsForTask(database, task.id);
    const run = runs.findLast((r) => isRunActive(r.status)) ?? runs.at(-1);
    return { task, runs, run, status: run?.status ?? "queued" };
  });
  const counts = {
    working: tasks.filter((t) => ["starting", "planning", "working"].includes(t.status)).length,
    blocked: tasks.filter((t) => t.status === "blocked").length,
    queued: tasks.filter((t) => t.status === "queued").length,
    review: tasks.filter((t) => t.status === "review").length,
    done: tasks.filter((t) => t.status === "completed").length,
    failed: tasks.filter((t) => t.status === "failed").length,
  };
  return {
    project,
    status: deriveProjectStatus(
      tasks.filter((t) => t.run).map((t) => t.status),
      counts.queued,
    ),
    tasks,
    counts,
    attention: tasks.some((t) => t.run && needsAttention(t.status)),
  };
}

export interface AgentView {
  project: string;
  taskId: string;
  agent: string;
  kind: string;
  status: RunStatus;
  workspace: string;
  runId: string;
}

export function agents(database: db.Db): AgentView[] {
  return db
    .activeRuns(database)
    .filter((r) => r.herdrAgentId)
    .map((run) => ({
      project: db.getProject(database, run.projectId)?.name ?? run.projectId,
      taskId: run.taskId,
      agent: run.agentKind,
      kind: run.agentKind,
      status: run.status,
      workspace: run.herdrWorkspaceId,
      runId: run.id,
    }));
}

export interface EventRow {
  at: string;
  type: string;
  data?: string | undefined;
}

/** Everything the detail pane of a run shows, in one read. */
export interface RunDetail {
  run: AgentRun;
  task?: Task | undefined;
  project?: Project | undefined;
  change?: Change | undefined;
  events: EventRow[];
}

export function runView(database: db.Db, runId: string, eventLimit = 12): RunDetail | undefined {
  const run = db.getRun(database, runId);
  if (!run) return undefined;
  return {
    run,
    task: db.getTask(database, run.taskId),
    project: db.getProject(database, run.projectId),
    change: db.getChangeForRun(database, run.id),
    events: db.listEvents(database, eventLimit, run.id).map((e: any) => ({
      at: String(e.at),
      type: String(e.type),
      data: e.data ?? undefined,
    })),
  };
}
