// Minimal logic check with no network and no Herdr: nub run check
import assert from "node:assert/strict";
import { existsSync, mkdtempSync, writeFileSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import { join } from "node:path";
import { deriveProjectStatus, normalizeAgentStatus, taskStatusForRun } from "./domain/status.ts";
import {
  agentName, buildPrompt, isTaskAvailable, resolveAgentRole, reviewAgentName, withPrefix,
  workspaceLabel, worktreePath,
} from "./orchestrator/policies.ts";
import { ConfigSchema } from "./config/schema.ts";
import * as db from "./persistence/db.ts";
import { branchName } from "./repositories/git.ts";
import * as view from "./view.ts";
import type { AgentRun, Project, Task } from "./domain/types.ts";
import { buildIssueFilter, targetState } from "./providers/tasks/linear.ts";

// an unassigned task is still being written, so no agent picks it up
assert.deepEqual(buildIssueFilter({ assignee: "me", taskProviderProjectId: "Mochi" }), {
  state: { type: { eq: "unstarted" } },
  project: { name: { eq: "Mochi" } },
  assignee: { isMe: { eq: true } },
});
assert.deepEqual(buildIssueFilter({ assignee: "a@b.c" }).assignee, { email: { eq: "a@b.c" } });
assert.equal(buildIssueFilter({ assignee: "any" }).assignee, undefined);
assert.equal(buildIssueFilter({}).assignee, undefined);

// in Linear both "In Progress" and "In Review" have type started, and API order is arbitrary
const linearStates = [
  { id: "1", name: "In Review", type: "started" },
  { id: "2", name: "In Progress", type: "started" },
  { id: "3", name: "Todo", type: "unstarted" },
  { id: "4", name: "Backlog", type: "backlog" },
  { id: "5", name: "Done", type: "completed" },
];
assert.equal(targetState("in_progress", linearStates)?.name, "In Progress");
assert.equal(targetState("waiting_for_agent", linearStates)?.name, "In Progress");
assert.equal(targetState("in_review", linearStates)?.name, "In Review");
assert.equal(targetState("done", linearStates)?.name, "Done");
assert.equal(targetState("todo", linearStates)?.name, "Todo");

// Herdr state normalization
assert.equal(normalizeAgentStatus("blocked"), "blocked");
assert.equal(normalizeAgentStatus("weird"), "unknown");

// project status priority: blocked outranks working
assert.equal(deriveProjectStatus(["working", "blocked", "completed"], 2), "blocked");
assert.equal(deriveProjectStatus(["working"], 0), "working");
assert.equal(deriveProjectStatus(["completed"], 3), "queued");
assert.equal(deriveProjectStatus([], 0), "idle");
assert.equal(deriveProjectStatus(["creating_change"], 0), "validating");

// run status -> tracker task status
assert.equal(taskStatusForRun("blocked"), "waiting_for_agent");
assert.equal(taskStatusForRun("review"), "in_review");
assert.equal(taskStatusForRun("completed"), "done");

const project: Project = { id: "phocus", name: "Phocus", repositoryId: "/repo" };
const task: Task = { id: "LIN-42", providerId: "uuid", projectId: "phocus", title: "LUT importer", status: "todo" };
assert.equal(agentName(project, task), "phocus-lin-42");
assert.equal(workspaceLabel(project, task), "phocus / LIN-42 / lut-importer");
assert.equal(branchName(task.id, task.title), "agent/lin-42-lut-importer");
// the worktree path must be absolute: git -C does not understand "~"
assert.equal(worktreePath("~/.shepherd/worktrees", project, task), join(homedir(), ".shepherd/worktrees/phocus-lin-42"));
assert.ok(agentName({ ...project, name: "9lives" }, task).match(/^[a-z][a-z0-9_-]{0,31}$/));

// roles: the project overrides the global section, unset fields are inherited
const cfg = ConfigSchema.parse({
  agents: { dev: { kind: "codex" }, review: { kind: "claude", prompt: "/code-review" } },
  projects: [
    { name: "Mochi", repository: "~/m" },
    { name: "Fmc", repository: "~/f", agent: "claude", agents: { review: { prompt: "/code-review --strict" } } },
  ],
});
const [mochi, fmc] = cfg.projects;
assert.deepEqual(resolveAgentRole("dev", cfg, mochi), { kind: "codex", prompt: "" });
assert.deepEqual(resolveAgentRole("review", cfg, mochi), { kind: "claude", prompt: "/code-review" });
// project.agent changes the dev agent, while the review prompt is overridden locally
assert.equal(resolveAgentRole("dev", cfg, fmc).kind, "claude");
assert.deepEqual(resolveAgentRole("review", cfg, fmc), { kind: "claude", prompt: "/code-review --strict" });
// with no global kind, review runs on the same agent as development
const bare = ConfigSchema.parse({ projects: [{ name: "X", repository: "~/x", agent: "codex" }] });
assert.equal(resolveAgentRole("review", bare, bare.projects[0]).kind, "codex");

// the agent command goes before the task text; an empty prefix adds nothing
assert.equal(withPrefix("/code-review", "PR #1"), "/code-review PR #1");
assert.equal(withPrefix("  ", "PR #1"), "PR #1");
assert.ok(buildPrompt(task, { branch: "b", prefix: "/brainstorm" }).startsWith("/brainstorm Task LIN-42"));
assert.ok(buildPrompt(task, { branch: "b" }).startsWith("Task LIN-42"));
// the review agent name must stay valid for herdr (<= 32 chars)
assert.ok(reviewAgentName(agentName(project, task)).match(/^[a-z][a-z0-9_-]{0,31}$/));
assert.ok(reviewAgentName("a".repeat(32)).length <= 32);

// a task cannot be restarted while it has a non-failed run
const run = (status: AgentRun["status"]): AgentRun => ({
  id: "run_1", projectId: "phocus", taskId: "LIN-42", herdrWorkspaceId: "w1", herdrAgentId: "phocus-lin-42",
  agentKind: "codex", branch: "agent/lin-42", worktreePath: "/tmp/x", status, startedAt: new Date(),
});
assert.equal(isTaskAvailable(task, []), true);
assert.equal(isTaskAvailable(task, [run("working")]), false);
assert.equal(isTaskAvailable(task, [run("completed")]), false);
assert.equal(isTaskAvailable(task, [run("failed")]), true);

// persistence: one task, one active run
const database = db.openDb(join(mkdtempSync(join(tmpdir(), "shepherd-")), "state.db"));
db.upsertRepository(database, { id: "/repo", path: "/repo", defaultBranch: "main" });
db.upsertProject(database, project);
db.upsertTask(database, task);
db.insertRun(database, run("working"));
assert.throws(() => db.insertRun(database, { ...run("working"), id: "run_2" }), /UNIQUE|constraint/i);

db.updateRun(database, "run_1", { status: "blocked", blockedReason: "which API should I use?" });
const [projectState] = view.overview(database);
assert.equal(projectState?.status, "blocked");
assert.equal(projectState?.attention, true);
assert.equal(projectState?.tasks[0]?.run?.blockedReason, "which API should I use?");

// after a failure the task is available again and a new run can be inserted
db.updateRun(database, "run_1", { status: "failed", error: "boom" });
db.insertRun(database, { ...run("queued"), id: "run_2" });
assert.equal(db.runsForTask(database, "LIN-42").length, 2);
assert.equal(db.activeRuns(database).length, 1);
db.appendEvent(database, "RunFailed", { runId: "run_1", data: { error: "boom" } });
assert.equal(db.listEvents(database).length, 1);

// a pid file left by a dead process must not block startup
process.env.SHEPHERD_STATE_DIR = mkdtempSync(join(tmpdir(), "shepherd-state-"));
const { lockLoop, pidPath, runningPid } = await import("./cli/daemon.ts");
writeFileSync(pidPath(), "999999"); // no such process
assert.equal(runningPid(), undefined);
assert.equal(existsSync(pidPath()), false);
lockLoop();
assert.equal(runningPid(), process.pid);
assert.throws(() => lockLoop(), /already running/);

// custom providers are picked up from the folder; a broken file does not break the rest
const providersDir = mkdtempSync(join(tmpdir(), "shepherd-providers-"));
writeFileSync(join(providersDir, "jira.ts"), `export const taskProviders = { jira: (s) => ({ url: s.url }) };\n`);
writeFileSync(join(providersDir, "gitlab.ts"), `export const codeProviders = { gitlab: () => ({ kind: "gitlab" }) };\n`);
writeFileSync(join(providersDir, "broken.ts"), `throw new Error("boom");\n`);
writeFileSync(join(providersDir, "notes.md"), `not a module\n`);
const { loadCustomProviders } = await import("./providers/load.ts");
const custom = await loadCustomProviders(providersDir);
assert.deepEqual(custom.loaded, ["gitlab.ts", "jira.ts"]);
assert.equal(custom.errors.length, 1);
assert.match(custom.errors[0]!, /^broken\.ts:/);
assert.equal((custom.tasks.jira as any)({ url: "https://jira" }).url, "https://jira");
assert.ok(custom.code.gitlab);

console.log("selfcheck ok");
