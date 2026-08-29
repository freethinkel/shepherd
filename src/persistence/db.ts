import { mkdirSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { DatabaseSync } from "node:sqlite";
import type { AgentRun, Change, Project, Repository, RunStatus, Task } from "../domain/types.ts";
import { isRunActive } from "../domain/status.ts";

// ponytail: the whole schema is applied on open (CREATE IF NOT EXISTS).
// Real migrations once a second schema version ships to production.
const SCHEMA = `
-- busy_timeout first: setting journal_mode itself needs a lock, and with a zero timeout
-- it fails instantly whenever the daemon happens to be writing
PRAGMA busy_timeout = 5000;
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS repositories (
  id TEXT PRIMARY KEY, path TEXT NOT NULL UNIQUE,
  remote TEXT, default_branch TEXT NOT NULL DEFAULT 'main');

CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE,
  repository_id TEXT NOT NULL REFERENCES repositories(id),
  task_provider_project_id TEXT);

CREATE TABLE IF NOT EXISTS tasks (
  id TEXT PRIMARY KEY, provider_id TEXT NOT NULL,
  project_id TEXT NOT NULL REFERENCES projects(id),
  title TEXT NOT NULL, description TEXT, status TEXT NOT NULL, url TEXT,
  provider TEXT NOT NULL DEFAULT '', synced_at TEXT NOT NULL);

CREATE TABLE IF NOT EXISTS agent_runs (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id),
  task_id TEXT NOT NULL REFERENCES tasks(id),
  herdr_workspace_id TEXT NOT NULL DEFAULT '',
  herdr_agent_id TEXT NOT NULL DEFAULT '',
  agent_kind TEXT NOT NULL, branch TEXT NOT NULL, worktree_path TEXT NOT NULL,
  status TEXT NOT NULL, started_at TEXT NOT NULL, finished_at TEXT,
  error TEXT, change_id TEXT, blocked_reason TEXT,
  attempt INTEGER NOT NULL DEFAULT 1);

-- a task cannot be claimed twice
CREATE UNIQUE INDEX IF NOT EXISTS one_active_run_per_task ON agent_runs(task_id)
  WHERE status NOT IN ('completed', 'failed');

CREATE TABLE IF NOT EXISTS workspaces (
  id TEXT PRIMARY KEY, run_id TEXT NOT NULL UNIQUE REFERENCES agent_runs(id),
  label TEXT NOT NULL, cwd TEXT NOT NULL, closed_at TEXT);

CREATE TABLE IF NOT EXISTS changes (
  id TEXT NOT NULL, run_id TEXT NOT NULL UNIQUE REFERENCES agent_runs(id),
  provider TEXT NOT NULL, url TEXT NOT NULL, status TEXT NOT NULL,
  created_at TEXT NOT NULL, PRIMARY KEY (provider, id));

CREATE TABLE IF NOT EXISTS events (
  seq INTEGER PRIMARY KEY AUTOINCREMENT, at TEXT NOT NULL, type TEXT NOT NULL,
  project_id TEXT, task_id TEXT, run_id TEXT, data TEXT);
`;

export type Db = DatabaseSync;

export function openDb(
  path = process.env.SHEPHERD_DB ?? join(homedir(), ".shepherd", "state.db"),
): Db {
  const file = resolve(path);
  mkdirSync(dirname(file), { recursive: true });
  const db = new DatabaseSync(file);
  db.exec(SCHEMA);
  // ponytail: one column added after the fact. A real migration runner can wait for the second one.
  const columns = db.prepare(`SELECT name FROM pragma_table_info('tasks')`).all() as {
    name: string;
  }[];
  if (!columns.some((c) => c.name === "provider")) {
    db.exec(`ALTER TABLE tasks ADD COLUMN provider TEXT NOT NULL DEFAULT ''`);
  }
  return db;
}

const n = (v: unknown) => (v === undefined || v === null ? null : (v as string));
const iso = (d?: Date) => (d ? d.toISOString() : null);

export function upsertRepository(db: Db, repo: Repository): void {
  db.prepare(
    `INSERT INTO repositories (id, path, remote, default_branch) VALUES (?, ?, ?, ?)
     ON CONFLICT(path) DO UPDATE SET remote = excluded.remote, default_branch = excluded.default_branch`,
  ).run(repo.id, repo.path, n(repo.remote), repo.defaultBranch);
}

export function upsertProject(db: Db, project: Project): void {
  db.prepare(
    `INSERT INTO projects (id, name, repository_id, task_provider_project_id) VALUES (?, ?, ?, ?)
     ON CONFLICT(name) DO UPDATE SET repository_id = excluded.repository_id,
       task_provider_project_id = excluded.task_provider_project_id`,
  ).run(project.id, project.name, project.repositoryId, n(project.taskProviderProjectId));
}

export function upsertTask(db: Db, task: Task): void {
  db.prepare(
    `INSERT INTO tasks (id, provider_id, project_id, title, description, status, url, provider, synced_at)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
     ON CONFLICT(id) DO UPDATE SET title = excluded.title, description = excluded.description,
       status = excluded.status, url = excluded.url, provider = excluded.provider,
       synced_at = excluded.synced_at`,
  ).run(
    task.id,
    task.providerId,
    task.projectId,
    task.title,
    n(task.description),
    task.status,
    n(task.url),
    task.provider ?? "",
    new Date().toISOString(),
  );
}

export function setTaskStatus(db: Db, taskId: string, status: string): void {
  db.prepare(`UPDATE tasks SET status = ? WHERE id = ?`).run(status, taskId);
}

export const listProjects = (db: Db): Project[] =>
  (db.prepare(`SELECT * FROM projects ORDER BY name`).all() as any[]).map(rowToProject);

export const getProject = (db: Db, id: string): Project | undefined => {
  const row = db.prepare(`SELECT * FROM projects WHERE id = ?`).get(id) as any;
  return row ? rowToProject(row) : undefined;
};

export const getRepository = (db: Db, id: string): Repository | undefined => {
  const row = db.prepare(`SELECT * FROM repositories WHERE id = ?`).get(id) as any;
  return row
    ? {
        id: row.id,
        path: row.path,
        remote: row.remote ?? undefined,
        defaultBranch: row.default_branch,
      }
    : undefined;
};

export const listTasks = (db: Db, projectId?: string): Task[] =>
  (
    db
      .prepare(`SELECT * FROM tasks ${projectId ? "WHERE project_id = ?" : ""} ORDER BY id`)
      .all(...(projectId ? [projectId] : [])) as any[]
  ).map(rowToTask);

export const getTask = (db: Db, id: string): Task | undefined => {
  const row = db.prepare(`SELECT * FROM tasks WHERE id = ?`).get(id) as any;
  return row ? rowToTask(row) : undefined;
};

export function insertRun(db: Db, run: AgentRun & { attempt?: number }): void {
  db.prepare(
    `INSERT INTO agent_runs (id, project_id, task_id, herdr_workspace_id, herdr_agent_id, agent_kind,
       branch, worktree_path, status, started_at, attempt)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
  ).run(
    run.id,
    run.projectId,
    run.taskId,
    run.herdrWorkspaceId,
    run.herdrAgentId,
    run.agentKind,
    run.branch,
    run.worktreePath,
    run.status,
    run.startedAt.toISOString(),
    run.attempt ?? 1,
  );
}

type RunPatch = Partial<
  Pick<
    AgentRun,
    | "status"
    | "error"
    | "changeId"
    | "blockedReason"
    | "herdrWorkspaceId"
    | "herdrAgentId"
    | "finishedAt"
    | "worktreePath"
  >
>;

export function updateRun(db: Db, id: string, patch: RunPatch): void {
  const cols: Record<string, unknown> = {
    status: patch.status,
    error: patch.error,
    change_id: patch.changeId,
    blocked_reason: patch.blockedReason,
    herdr_workspace_id: patch.herdrWorkspaceId,
    herdr_agent_id: patch.herdrAgentId,
    worktree_path: patch.worktreePath,
    finished_at: iso(patch.finishedAt),
  };
  const entries = Object.entries(cols).filter(([, v]) => v !== undefined);
  if (entries.length === 0) return;
  db.prepare(
    `UPDATE agent_runs SET ${entries.map(([k]) => `${k} = ?`).join(", ")} WHERE id = ?`,
  ).run(...entries.map(([, v]) => n(v)), id);
}

export const getRun = (db: Db, id: string): AgentRun | undefined => {
  const row = db
    .prepare(`SELECT * FROM agent_runs WHERE id = ? OR id LIKE ?`)
    .get(id, `${id}%`) as any;
  return row ? rowToRun(row) : undefined;
};

export const listRuns = (db: Db, projectId?: string): AgentRun[] =>
  (
    db
      .prepare(
        `SELECT * FROM agent_runs ${projectId ? "WHERE project_id = ?" : ""} ORDER BY started_at DESC`,
      )
      .all(...(projectId ? [projectId] : [])) as any[]
  ).map(rowToRun);

export const activeRuns = (db: Db): AgentRun[] => listRuns(db).filter((r) => isRunActive(r.status));

export const runsForTask = (db: Db, taskId: string): AgentRun[] =>
  (
    db
      .prepare(`SELECT * FROM agent_runs WHERE task_id = ? ORDER BY started_at`)
      .all(taskId) as any[]
  ).map(rowToRun);

export function recordWorkspace(
  db: Db,
  ws: { id: string; runId: string; label: string; cwd: string },
): void {
  db.prepare(
    `INSERT INTO workspaces (id, run_id, label, cwd) VALUES (?, ?, ?, ?)
     ON CONFLICT(id) DO UPDATE SET label = excluded.label`,
  ).run(ws.id, ws.runId, ws.label, ws.cwd);
}

export function closeWorkspaceRow(db: Db, id: string): void {
  db.prepare(`UPDATE workspaces SET closed_at = ? WHERE id = ?`).run(new Date().toISOString(), id);
}

export function recordChange(db: Db, change: Change): void {
  db.prepare(
    `INSERT INTO changes (id, run_id, provider, url, status, created_at)
     VALUES (?, ?, ?, ?, ?, ?)
     ON CONFLICT(provider, id) DO UPDATE SET
       run_id = excluded.run_id, url = excluded.url, status = excluded.status`,
  ).run(
    change.id,
    change.runId,
    change.provider,
    change.url,
    change.status,
    new Date().toISOString(),
  );
}

export const getChangeForRun = (db: Db, runId: string): Change | undefined => {
  const row = db.prepare(`SELECT * FROM changes WHERE run_id = ?`).get(runId) as any;
  return row
    ? { id: row.id, runId: row.run_id, provider: row.provider, url: row.url, status: row.status }
    : undefined;
};

export function appendEvent(
  db: Db,
  type: string,
  ctx: { projectId?: string; taskId?: string; runId?: string; data?: unknown } = {},
): void {
  db.prepare(
    `INSERT INTO events (at, type, project_id, task_id, run_id, data) VALUES (?, ?, ?, ?, ?, ?)`,
  ).run(
    new Date().toISOString(),
    type,
    n(ctx.projectId),
    n(ctx.taskId),
    n(ctx.runId),
    ctx.data === undefined ? null : JSON.stringify(ctx.data),
  );
}

export const hasEvent = (db: Db, runId: string, type: string): boolean =>
  db.prepare(`SELECT 1 FROM events WHERE run_id = ? AND type = ? LIMIT 1`).get(runId, type) !==
  undefined;

export const countEvents = (db: Db, runId: string, type: string): number =>
  (
    db
      .prepare(`SELECT count(*) AS n FROM events WHERE run_id = ? AND type = ?`)
      .get(runId, type) as { n: number }
  ).n;

/** When something last happened to a task, used to forget runs from before a reset. */
export const lastEventAt = (db: Db, taskId: string, type: string): Date | undefined => {
  const row = db
    .prepare(`SELECT at FROM events WHERE task_id = ? AND type = ? ORDER BY seq DESC LIMIT 1`)
    .get(taskId, type) as { at: string } | undefined;
  return row ? new Date(row.at) : undefined;
};

export const listEvents = (db: Db, limit = 50, runId?: string) =>
  db
    .prepare(`SELECT * FROM events ${runId ? "WHERE run_id = ?" : ""} ORDER BY seq DESC LIMIT ?`)
    .all(...(runId ? [runId] : []), limit) as any[];

const rowToProject = (r: any): Project => ({
  id: r.id,
  name: r.name,
  repositoryId: r.repository_id,
  taskProviderProjectId: r.task_provider_project_id ?? undefined,
});

const rowToTask = (r: any): Task => ({
  id: r.id,
  providerId: r.provider_id,
  projectId: r.project_id,
  title: r.title,
  description: r.description ?? undefined,
  status: r.status,
  url: r.url ?? undefined,
  provider: r.provider || undefined,
});

const rowToRun = (r: any): AgentRun => ({
  id: r.id,
  projectId: r.project_id,
  taskId: r.task_id,
  herdrWorkspaceId: r.herdr_workspace_id,
  herdrAgentId: r.herdr_agent_id,
  agentKind: r.agent_kind,
  branch: r.branch,
  worktreePath: r.worktree_path,
  status: r.status as RunStatus,
  startedAt: new Date(r.started_at),
  finishedAt: r.finished_at ? new Date(r.finished_at) : undefined,
  error: r.error ?? undefined,
  changeId: r.change_id ?? undefined,
  blockedReason: r.blocked_reason ?? undefined,
});
