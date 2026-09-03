// What a human can ask for on a run, shared by the CLI and the TUI. Progress goes to `log`
// rather than stdout, because the TUI has no stdout to print to.
import type { App } from "../core/app.ts";
import { isRunActive } from "./domain/status.ts";
import type { AgentRun } from "./domain/types.ts";
import * as db from "../core/persistence/db.ts";
import { deleteBranch } from "./git.ts";

export type Log = (msg: string) => void;

export function requireRun(app: App, id?: string): AgentRun {
  if (!id) throw new Error("<run-id> is required");
  const run = db.getRun(app.db, id);
  if (!run) throw new Error(`run ${id} not found`);
  return run;
}

export async function stop(app: App, runId?: string): Promise<string> {
  const run = requireRun(app, runId);
  await app.workflow.stop(run);
  return `run ${run.id} stopped`;
}

export async function retry(app: App, runId?: string): Promise<string> {
  const previous = requireRun(app, runId);
  if (isRunActive(previous.status)) await app.workflow.stop(previous, "restarted");
  const project = db.getProject(app.db, previous.projectId)!;
  const task = db.getTask(app.db, previous.taskId)!;
  const run = await app.workflow.start(project, task);
  return `new run ${run.id} (${run.status})`;
}

export async function open(app: App, runId?: string): Promise<string> {
  const run = requireRun(app, runId);
  await app.herdr.focusWorkspace(run.herdrWorkspaceId);
  return `workspace ${run.herdrWorkspaceId} focused`;
}

/** Runs sitting in review without a review agent: started after a config change or a failed launch. */
export async function review(app: App, runId: string | undefined, log: Log): Promise<string> {
  const runs = runId
    ? [requireRun(app, runId)]
    : db.listRuns(app.db).filter((r) => r.status === "review");
  let started = 0;
  for (const run of runs) {
    if (await app.workflow.ensureReviewAgent(run)) {
      log(`${run.taskId}: review agent started (${run.herdrWorkspaceId})`);
      started++;
    }
  }
  return started ? `review agents started: ${started}` : "nothing to start";
}

/**
 * Back to square one: stop the agents, drop the worktree, the workspace and the local branch,
 * and mark the runs failed. The remote branch and any open change are left alone, because
 * throwing away a merge request nobody asked to close is not ours to decide.
 */
export async function reset(
  app: App,
  taskId: string | undefined,
  restart: boolean,
  log: Log,
): Promise<string> {
  if (!taskId) throw new Error("<task-id> is required");
  const task = db.getTask(app.db, taskId);
  if (!task) throw new Error(`task ${taskId} not found`);
  const project = db.getProject(app.db, task.projectId);
  const repo = project ? db.getRepository(app.db, project.repositoryId) : undefined;

  // Order matters: an active run is what keeps the task off the queue. Marking the runs
  // failed first would free the task, and the daemon would start a new one mid-cleanup,
  // on a worktree being deleted underneath it.
  const runs = db.runsForTask(app.db, task.id);
  for (const run of runs) {
    if (run.herdrAgentId) await app.herdr.stopAgent(run.herdrAgentId);
    if (run.herdrWorkspaceId && (await app.herdr.workspaceExists(run.herdrWorkspaceId))) {
      await app.herdr
        .removeWorktree(run.herdrWorkspaceId)
        .catch(() => app.herdr.closeWorkspace(run.herdrWorkspaceId));
      db.closeWorkspaceRow(app.db, run.herdrWorkspaceId);
      log(`workspace ${run.herdrWorkspaceId} removed`);
    }
    if (repo) await deleteBranch(repo.path, run.branch);
    const change = db.getChangeForRun(app.db, run.id);
    if (change) log(`left open: ${change.url}`);
  }
  for (const run of runs) {
    if (isRunActive(run.status)) await app.workflow.stop(run, "reset by user");
  }
  db.setTaskStatus(app.db, task.id, "todo");
  db.appendEvent(app.db, "TaskReset", { taskId: task.id, projectId: task.projectId });
  log(`${task.id} reset`);

  if (restart && project) {
    const run = await app.workflow.start(project, task);
    return `run ${run.id} -> ${run.status}`;
  }
  return `the daemon picks it up on the next tick, or run: shepherd run ${task.id}`;
}
