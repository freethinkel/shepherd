// One state view for the CLI (and a future TUI). Knows nothing about Linear/GitHub/Herdr.
import { deriveProjectStatus, isRunActive, needsAttention } from "./domain/status.ts";
import type { AgentRun, Project, ProjectStatus, RunStatus, Task } from "./domain/types.ts";
import * as db from "./persistence/db.ts";

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

export function overview(database: db.Db): ProjectView[] {
  return db.listProjects(database).map((project) => projectView(database, project));
}

export function projectView(database: db.Db, project: Project): ProjectView {
  const tasks: TaskView[] = db.listTasks(database, project.id).map((task) => {
    const runs = db.runsForTask(database, task.id);
    const run = runs.findLast((r) => isRunActive(r.status)) ?? runs.at(-1);
    return { task, runs, run, status: run?.status ?? "queued" };
  });
  const counts = {
    working: tasks.filter((t) => ["starting", "working"].includes(t.status)).length,
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
