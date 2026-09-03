import assert from "node:assert/strict";
import { existsSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import type { AgentRun, Project, Task } from "../src/shared/domain/types.ts";
import * as db from "../src/core/persistence/db.ts";
import * as view from "../src/shared/view.ts";

const project: Project = { id: "phocus", name: "Phocus", repositoryId: "/repo" };
const task: Task = {
  id: "LIN-42",
  providerId: "uuid",
  projectId: "phocus",
  title: "LUT importer",
  status: "todo",
};
const run = (status: AgentRun["status"], id = "run_1"): AgentRun => ({
  id,
  projectId: "phocus",
  taskId: "LIN-42",
  herdrWorkspaceId: "w1",
  herdrAgentId: "phocus-lin-42",
  agentKind: "codex",
  branch: "agent/lin-42",
  worktreePath: "/tmp/x",
  status,
  startedAt: new Date(),
});

function freshDb() {
  const database = db.openDb(join(mkdtempSync(join(tmpdir(), "shepherd-")), "state.db"));
  db.upsertRepository(database, { id: "/repo", path: "/repo", defaultBranch: "main" });
  db.upsertProject(database, project);
  db.upsertTask(database, task);
  return database;
}

test("a task cannot be claimed twice", () => {
  const database = freshDb();
  db.insertRun(database, run("working"));
  assert.throws(() => db.insertRun(database, run("working", "run_2")), /UNIQUE|constraint/i);
});

test("the tracker a task came from is remembered", () => {
  const database = freshDb();
  assert.equal(db.getTask(database, task.id)?.provider, undefined);
  db.upsertTask(database, { ...task, provider: "jira" });
  assert.equal(db.getTask(database, task.id)?.provider, "jira");
  db.upsertTask(database, { ...task, provider: "linear" });
  assert.equal(db.getTask(database, task.id)?.provider, "linear");
});

test("a failed run frees the task and a new run can start", () => {
  const database = freshDb();
  db.insertRun(database, run("working"));
  db.updateRun(database, "run_1", { status: "failed", error: "boom" });
  db.insertRun(database, run("queued", "run_2"));
  assert.equal(db.runsForTask(database, "LIN-42").length, 2);
  assert.equal(db.activeRuns(database).length, 1);
});

test("events are the audit trail and the round counter", () => {
  const database = freshDb();
  db.insertRun(database, run("working"));
  assert.equal(db.countEvents(database, "run_1", "ValidationRejected"), 0);
  db.appendEvent(database, "ValidationRejected", { runId: "run_1" });
  db.appendEvent(database, "ValidationRejected", { runId: "run_1" });
  db.appendEvent(database, "RunFailed", { runId: "run_1", data: { error: "boom" } });
  assert.equal(db.countEvents(database, "run_1", "ValidationRejected"), 2);
  assert.equal(db.hasEvent(database, "run_1", "RunFailed"), true);
  assert.equal(db.hasEvent(database, "run_1", "ReviewAgentStarted"), false);
  assert.equal(db.listEvents(database).length, 3);
});

test("the dashboard reads blocked and review differently", () => {
  const database = freshDb();
  db.insertRun(database, run("working"));
  db.updateRun(database, "run_1", { status: "blocked", blockedReason: "which API should I use?" });
  const [blocked] = view.overview(database);
  assert.equal(blocked?.status, "blocked");
  assert.equal(blocked?.attention, true);
  assert.equal(blocked?.tasks[0]?.run?.blockedReason, "which API should I use?");

  db.updateRun(database, "run_1", { status: "review" });
  const [inReview] = view.overview(database);
  assert.equal(inReview?.attention, false, "a pull request waiting for a human is normal");
  assert.equal(inReview?.counts.review, 1);
  assert.equal(inReview?.status, "review");
});

test("a pid file left by a dead process does not block startup", async () => {
  process.env.SHEPHERD_STATE_DIR = mkdtempSync(join(tmpdir(), "shepherd-state-"));
  const { lockLoop, pidPath, runningPid } = await import("../src/modules/cli/daemon.ts");
  writeFileSync(pidPath(), "999999"); // no such process
  assert.equal(runningPid(), undefined);
  assert.equal(existsSync(pidPath()), false);
  lockLoop();
  assert.equal(runningPid(), process.pid);
  assert.throws(() => lockLoop(), /already running/);
});

test("a reset is a marker in the journal, so earlier runs can be ignored", () => {
  const database = freshDb();
  db.insertRun(database, run("failed"));
  assert.equal(db.lastEventAt(database, "LIN-42", "TaskReset"), undefined);
  db.appendEvent(database, "TaskReset", { taskId: "LIN-42" });
  const at = db.lastEventAt(database, "LIN-42", "TaskReset");
  assert.ok(at instanceof Date);
  assert.ok(Date.now() - at.getTime() < 5_000);
});

test("an event that was never appended is absent, not present", () => {
  const database = freshDb();
  db.insertRun(database, run("review"));
  // the driver answers a miss with null: `!== undefined` would call every event present
  assert.equal(db.hasEvent(database, "run_1", "ChangeMerged"), false);
  db.appendEvent(database, "ChangeMerged", { runId: "run_1" });
  assert.equal(db.hasEvent(database, "run_1", "ChangeMerged"), true);
  assert.equal(db.hasEvent(database, "run_1", "MergeFailed"), false);
});

test("the detail view of a run carries its task, change and newest events", () => {
  const database = freshDb();
  db.insertRun(database, run("review"));
  db.recordChange(database, {
    id: "14",
    runId: "run_1",
    provider: "github",
    url: "https://example.test/pull/14",
    status: "open",
  });
  db.appendEvent(database, "ChangeCreated", { runId: "run_1", taskId: "LIN-42" });
  db.appendEvent(database, "ReviewRejected", { runId: "run_1", taskId: "LIN-42" });

  const detail = view.runView(database, "run_1");
  assert.equal(detail?.task?.title, "LUT importer");
  assert.equal(detail?.project?.name, "Phocus");
  assert.equal(detail?.change?.url, "https://example.test/pull/14");
  assert.deepEqual(
    detail?.events.map((e) => e.type),
    ["ReviewRejected", "ChangeCreated"],
    "newest first, the way the pane prints them",
  );
  assert.equal(view.runView(database, "nope"), undefined);
});
