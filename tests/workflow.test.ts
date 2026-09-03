import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { ConfigSchema } from "../src/core/config/schema.ts";
import type {
  AgentStatus,
  Change,
  ChangeComment,
  CodeProvider,
  Project,
  Task,
  TaskProvider,
} from "../src/shared/domain/types.ts";
import type { HerdrClient } from "../src/modules/herdr/client.ts";
import { remoteHost } from "../src/modules/orchestrator/policies.ts";
import { Workflow } from "../src/modules/orchestrator/workflow.ts";
import * as db from "../src/core/persistence/db.ts";
import { ProviderRegistry } from "../src/modules/providers/registry.ts";
import type { CustomProviders } from "../src/modules/providers/load.ts";

// The whole run lifecycle against fake Herdr and providers, but a real git repository:
// worktrees, commits and pushes are genuine, nothing touches the network.

const quiet: Parameters<typeof execFileSync>[2] = {
  encoding: "utf8",
  stdio: ["ignore", "pipe", "ignore"],
};
const git = (cwd: string, ...args: string[]) =>
  String(execFileSync("git", ["-C", cwd, ...args], quiet));

/** A bare "origin" plus a working clone, so push and rev-list are the real thing. */
function makeRepo() {
  const root = mkdtempSync(join(tmpdir(), "shepherd-repo-"));
  const origin = join(root, "origin.git");
  execFileSync("git", ["init", "--bare", "-b", "main", origin], quiet);
  mkdirSync(join(root, "worktrees"), { recursive: true });
  const work = join(root, "work");
  execFileSync("git", ["clone", origin, work], quiet);
  git(work, "config", "user.email", "check@example.com");
  git(work, "config", "user.name", "check");
  writeFileSync(join(work, "README.md"), "seed\n");
  git(work, "add", "-A");
  git(work, "commit", "-m", "seed");
  git(work, "push", "-u", "origin", "main");
  return { root, origin, work };
}

class FakeHerdr {
  status: AgentStatus = "working";
  prompts: string[] = [];
  tabs = 0;
  closed: string[] = [];
  worktreeCalls = 0;
  tail = "Should I use the new API or keep the shim?";
  constructor(private readonly worktreePath: string) {}

  async openOrCreateWorktree(input: { label: string; branch: string }) {
    this.worktreeCalls++;
    const path = join(this.worktreePath, input.branch.replace(/\//g, "-"));
    if (!existsSync(path)) {
      execFileSync(
        "git",
        ["-C", this.repoForWorktree, "worktree", "add", "-b", input.branch, path],
        quiet,
      );
    }
    return { workspaceId: "w1", tabId: "w1:t1", paneId: "w1:p1", label: input.label, path };
  }
  repoForWorktree = "";

  async createWorkspace(input: { label: string; cwd: string }) {
    return { workspaceId: "w1", tabId: "w1:t1", paneId: "w1:p1", label: input.label };
  }
  async createTab() {
    this.tabs++;
    return { tabId: `w1:t${this.tabs + 1}`, paneId: `w1:p${this.tabs + 1}` };
  }
  agents: { name: string; paneId: string }[] = [];
  /** Names herdr holds but `agent list` does not report — exactly what the failures showed. */
  unlisted = new Set<string>();
  cleared: string[] = [];
  startErrors: string[] = [];
  async listAgents() {
    return this.agents;
  }
  async spawnAgent(input: { name: string; kind: string; paneId: string }) {
    const err = this.startErrors.shift();
    if (err) throw new Error(err);
    this.agents.push({ name: input.name, paneId: input.paneId });
    return {
      name: input.name,
      paneId: input.paneId,
      workspaceId: "w1",
      kind: input.kind,
      status: "idle" as const,
    };
  }
  async prompt(_agent: string, text: string) {
    this.prompts.push(text);
  }
  async getAgentStatus(name?: string) {
    if (name && this.unlisted.has(name)) return "idle" as AgentStatus;
    // herdr knows nothing about a name that was never started
    if (name && !this.agents.some((a) => a.name === name)) return "unknown" as AgentStatus;
    return this.status;
  }
  async clearAgentName(name: string) {
    this.cleared.push(name);
    this.unlisted.delete(name);
    this.agents = this.agents.filter((a) => a.name !== name);
  }
  async readAgent() {
    return this.tail;
  }
  async workspaceExists() {
    return true;
  }
  async closeWorkspace(id: string) {
    this.closed.push(id);
  }
  async stopAgent() {}
}

class FakeTasks implements TaskProvider {
  remote: Task["status"] = "todo";
  statuses: string[] = [];
  comments: string[] = [];
  claimed = 0;
  async listTasks() {
    return [];
  }
  async getTask(id: string): Promise<Task> {
    return { id, providerId: id, projectId: "p", title: id, status: this.remote };
  }
  async claimTask() {
    this.claimed++;
  }
  async updateStatus(_id: string, status: string) {
    this.statuses.push(status);
  }
  async addComment(_id: string, body: string) {
    this.comments.push(body);
  }
}

class FakeCode implements CodeProvider {
  created = 0;
  polled = 0;
  merged = 0;
  state: "open" | "merged" | "closed" = "open";
  approved = false;
  checks: "pending" | "success" | "failure" = "success";
  comments: ChangeComment[] = [];
  async check() {}
  async createChange(): Promise<Omit<Change, "runId">> {
    this.created++;
    return { id: "7", provider: "fake", url: "https://fake/mr/7", status: "open" };
  }
  getChangeErrors = 0;
  async getChange(): Promise<Omit<Change, "runId">> {
    this.polled++;
    if (this.getChangeErrors > 0) {
      this.getChangeErrors--;
      throw new Error("Command failed: gh pr view 7 --json number,url,state");
    }
    return {
      id: "7",
      provider: "fake",
      url: "https://fake/mr/7",
      status: this.state,
      approved: this.approved,
      checks: this.checks,
    };
  }
  async mergeChange() {
    this.merged++;
  }
  async listComments() {
    return this.comments;
  }
  existing: Omit<Change, "runId"> | undefined;
  async findChange() {
    return this.existing;
  }
}

function harness(overrides: Record<string, unknown> = {}) {
  const repo = makeRepo();
  const { orchestrator, ...rest } = overrides;
  const config = ConfigSchema.parse({
    code_providers: { fake: { hosts: [remoteHost(repo.origin)] } },
    agents: { review: { prompt: "" } },
    projects: [{ name: "Demo", repository: repo.work, agent: "claude" }],
    ...rest,
    // merged last and separately: a spread override would drop the temporary worktree path
    // and the checks would start writing into the real ~/.shepherd
    orchestrator: {
      worktrees: join(repo.root, "worktrees"),
      change_poll_interval_ms: 300,
      agent_settle_ms: 0,
      ...(orchestrator as object),
    },
  });
  const database = db.openDb(join(repo.root, "state.db"));
  const herdr = new FakeHerdr(join(repo.root, "worktrees"));
  herdr.repoForWorktree = repo.work;
  const tasks = new FakeTasks();
  const code = new FakeCode();
  const custom: CustomProviders = {
    tasks: { fake: () => tasks },
    code: { fake: () => code },
    loaded: [],
    errors: [],
  };
  const registry = new ProviderRegistry(config, custom);
  const projectConfigs = new Map([["demo", config.projects[0]!]]);
  const workflow = new Workflow({
    db: database,
    herdr: herdr as unknown as HerdrClient,
    registry,
    config,
    projectConfigs,
    log: () => {},
    sleep: async () => {}, // the retry backoff is real time; the checks skip it
  });

  const project: Project = { id: "demo", name: "Demo", repositoryId: repo.work };
  const task: Task = {
    id: "T-1",
    providerId: "1",
    projectId: "demo",
    title: "Do the thing",
    status: "todo",
    provider: "fake",
  };
  db.upsertRepository(database, {
    id: repo.work,
    path: repo.work,
    remote: repo.origin,
    defaultBranch: "main",
  });
  db.upsertProject(database, project);
  db.upsertTask(database, task);
  return { repo, config, db: database, herdr, tasks, code, workflow, project, task };
}

const reload = (h: ReturnType<typeof harness>, id: string) => db.getRun(h.db, id)!;

test("the happy path: queued to merged", async () => {
  const h = harness();
  const run = await h.workflow.start(h.project, h.task);
  assert.equal(run.status, "working");
  assert.equal(h.tasks.claimed, 1);
  assert.match(h.herdr.prompts[0]!, /Do the thing/);
  assert.equal(db.getRun(h.db, run.id)?.herdrWorkspaceId, "w1");

  h.herdr.status = "working";
  await h.workflow.advance(reload(h, run.id));
  assert.equal(reload(h, run.id).status, "working");

  // one idle poll is not enough; a pause must not be mistaken for completion
  h.herdr.status = "done";
  await h.workflow.advance(reload(h, run.id));
  assert.equal(reload(h, run.id).status, "working");
  await h.workflow.advance(reload(h, run.id));
  assert.equal(reload(h, run.id).status, "validating");

  // nothing committed yet, so the agent is asked to commit and goes back to work
  await h.workflow.advance(reload(h, run.id));
  assert.equal(reload(h, run.id).status, "working");
  assert.match(h.herdr.prompts.at(-1)!, /no commits/i);
  assert.equal(h.code.created, 0);

  const worktree = reload(h, run.id).worktreePath;
  writeFileSync(join(worktree, "feature.txt"), "done\n");
  git(worktree, "add", "-A");
  git(worktree, "-c", "user.email=a@b.c", "-c", "user.name=a", "commit", "-m", "feat: the thing");

  await h.workflow.advance(reload(h, run.id));
  await h.workflow.advance(reload(h, run.id));
  assert.equal(reload(h, run.id).status, "validating");
  await h.workflow.advance(reload(h, run.id)); // validating -> creating_change
  await h.workflow.advance(reload(h, run.id)); // push + create -> checking
  await h.workflow.advance(reload(h, run.id)); // green checks -> review
  const inReview = reload(h, run.id);
  assert.equal(inReview.status, "review");
  assert.equal(inReview.changeId, "7");
  assert.equal(h.code.created, 1);
  assert.match(h.tasks.comments.at(-1)!, /https:\/\/fake\/mr\/7/);
  // the branch really reached origin
  assert.match(git(h.repo.origin, "branch", "--list", inReview.branch), /agent\/t-1/);

  // still open: nothing changes, and no second change is created
  const polledOnce = h.code.polled;
  await h.workflow.advance(reload(h, run.id));
  assert.equal(reload(h, run.id).status, "review");
  assert.equal(h.code.created, 1);
  // the open pull request is polled on its own slower interval, not every tick
  await h.workflow.advance(reload(h, run.id));
  assert.equal(h.code.polled, polledOnce, "two ticks inside the interval read the forge once");

  await new Promise((r) => setTimeout(r, 350)); // past the poll interval
  h.code.state = "merged";
  await h.workflow.advance(reload(h, run.id));
  const done = reload(h, run.id);
  assert.equal(done.status, "completed");
  assert.ok(done.finishedAt);
  assert.equal(db.getTask(h.db, "T-1")?.status, "done");
  assert.deepEqual(h.herdr.closed, ["w1"]);
  assert.equal(h.tasks.statuses.at(-1), "done");
});

test("an agent that has not started yet is not mistaken for a finished one", async () => {
  // agent_settle_ms left at its default: the agent stays idle right after `agent start`
  const h = harness({ orchestrator: { agent_settle_ms: 60_000 } });
  const run = await h.workflow.start(h.project, h.task);
  h.herdr.status = "done"; // still idle, the prompt is unread

  for (let i = 0; i < 5; i++) await h.workflow.advance(reload(h, run.id));
  assert.equal(reload(h, run.id).status, "working", "no premature validation");
  assert.equal(h.herdr.prompts.length, 1, "and no second prompt across the first one");

  // once it actually starts, the usual two idle polls end the work
  h.herdr.status = "working";
  await h.workflow.advance(reload(h, run.id));
  h.herdr.status = "done";
  await h.workflow.advance(reload(h, run.id));
  await h.workflow.advance(reload(h, run.id));
  assert.equal(reload(h, run.id).status, "validating");
});

test("a busy pane gets a new tab, and a live agent of ours is reused", async () => {
  const h = harness();
  await h.workflow.start(h.project, h.task);
  assert.equal(h.herdr.tabs, 0, "a free root pane needs no extra tab");
  assert.equal(h.herdr.agents.length, 1);

  // a reopened workspace already holds our agent from the previous run
  const again = harness();
  again.herdr.agents.push({ name: "demo-t-1", paneId: "w1:p1" });
  const reused = await again.workflow.start(again.project, again.task);
  assert.equal(again.herdr.agents.length, 1, "no second agent with the same name");
  assert.equal(db.hasEvent(again.db, reused.id, "AgentReused"), true);

  // someone else occupies the root pane: our agent goes into a fresh tab
  const busy = harness();
  busy.herdr.agents.push({ name: "someone-else", paneId: "w1:p1" });
  await busy.workflow.start(busy.project, busy.task);
  assert.equal(busy.herdr.tabs, 1);
  assert.equal(busy.herdr.agents.at(-1)?.paneId, "w1:p2");
});

test("a blocked agent stops the run and reports why", async () => {
  const h = harness();
  const run = await h.workflow.start(h.project, h.task);
  h.herdr.status = "blocked";
  await h.workflow.advance(reload(h, run.id));
  const blocked = reload(h, run.id);
  assert.equal(blocked.status, "blocked");
  assert.match(blocked.blockedReason!, /new API/);
  assert.equal(db.getTask(h.db, "T-1")?.status, "waiting_for_agent");
  assert.match(h.tasks.comments.at(-1)!, /blocked/i);

  // answering unblocks it without a new run
  h.herdr.status = "working";
  await h.workflow.advance(reload(h, run.id));
  assert.equal(reload(h, run.id).status, "working");
});

test("validation that never passes gives up instead of looping forever", async () => {
  const h = harness({ orchestrator: { max_validation_rounds: 2 } });
  h.config.projects[0]!.validate = "exit 1";
  const run = await h.workflow.start(h.project, h.task);
  const worktree = reload(h, run.id).worktreePath;
  writeFileSync(join(worktree, "x.txt"), "x\n");
  git(worktree, "add", "-A");
  git(worktree, "-c", "user.email=a@b.c", "-c", "user.name=a", "commit", "-m", "wip");

  for (let i = 0; i < 2; i++) {
    db.updateRun(h.db, run.id, { status: "validating" });
    await h.workflow.advance(reload(h, run.id));
    assert.equal(reload(h, run.id).status, "working");
    assert.match(h.herdr.prompts.at(-1)!, /exit 1/);
  }
  db.updateRun(h.db, run.id, { status: "validating" });
  await h.workflow.advance(reload(h, run.id));
  const failed = reload(h, run.id);
  assert.equal(failed.status, "failed");
  assert.match(failed.error!, /still failing after 2/);
  // the task is handed back with the reason instead of rotting in In Progress
  assert.equal(db.getTask(h.db, "T-1")?.status, "todo");
  assert.match(h.tasks.comments.at(-1)!, /attempt 1 of 3/);
});

test("a hung run gives its slot back", async () => {
  const h = harness({ orchestrator: { run_timeout_ms: 1 } });
  const run = await h.workflow.start(h.project, h.task);
  await new Promise((r) => setTimeout(r, 5));
  await h.workflow.advance(reload(h, run.id));
  const failed = reload(h, run.id);
  assert.equal(failed.status, "failed");
  assert.match(failed.error!, /timed out/);
});

test("a review agent lands in a sibling tab, exactly once", async () => {
  const h = harness({ agents: { review: { kind: "claude", prompt: "/code-review" } } });
  const run = await h.workflow.start(h.project, h.task);
  db.updateRun(h.db, run.id, { status: "review", changeId: "7" });
  db.recordChange(h.db, {
    id: "7",
    runId: run.id,
    provider: "fake",
    url: "https://fake/mr/7",
    status: "open",
  });

  await h.workflow.advance(reload(h, run.id));
  assert.equal(h.herdr.tabs, 1);
  assert.match(h.herdr.prompts.at(-1)!, /^\/code-review https:\/\/fake\/mr\/7/);
  await h.workflow.advance(reload(h, run.id));
  assert.equal(h.herdr.tabs, 1); // ReviewAgentStarted keeps it to one
});

/** A run parked in review with its change recorded, the state a human sees a pull request in. */
/** A run whose agent has finished and committed: the next advance creates the change. */
async function workedThrough(h: ReturnType<typeof harness>) {
  const run = await h.workflow.start(h.project, h.task);
  const worktree = reload(h, run.id).worktreePath;
  writeFileSync(join(worktree, "feature.txt"), "done\n");
  git(worktree, "add", "-A");
  git(worktree, "-c", "user.email=a@b.c", "-c", "user.name=a", "commit", "-m", "feat: the thing");
  h.herdr.status = "done";
  await h.workflow.advance(reload(h, run.id));
  await h.workflow.advance(reload(h, run.id)); // idle twice -> validating
  return run;
}

async function parkedInReview(h: ReturnType<typeof harness>) {
  const run = await h.workflow.start(h.project, h.task);
  db.updateRun(h.db, run.id, { status: "review", changeId: "7" });
  db.setTaskStatus(h.db, "T-1", "in_review");
  db.recordChange(h.db, {
    id: "7",
    runId: run.id,
    provider: "fake",
    url: "https://fake/mr/7",
    status: "open",
  });
  db.appendEvent(h.db, "ChangeCreated", { runId: run.id, taskId: "T-1" });
  h.herdr.prompts.length = 0;
  return run;
}

test("a task sent back to Todo during review returns the comments to the agent", async () => {
  const h = harness();
  const run = await parkedInReview(h);
  // dated past ChangeCreated on purpose: a real review comment is minutes late, and at
  // millisecond resolution `new Date()` here can tie with the event and be filtered out
  h.code.comments = [
    {
      author: "egor",
      body: "rename this",
      path: "src/a.ts",
      line: 3,
      createdAt: new Date(Date.now() + 1000),
    },
  ];

  // the tracker sync recorded that a human moved the task back
  db.setTaskStatus(h.db, "T-1", "todo");
  await h.workflow.advance(reload(h, run.id));

  assert.equal(reload(h, run.id).status, "working");
  assert.match(h.herdr.prompts.at(-1)!, /src\/a\.ts:3.*rename this/);
  assert.equal(h.tasks.statuses.at(-1), "in_progress");
  assert.equal(db.countEvents(h.db, run.id, "ReviewRejected"), 1);

  // back to review through the normal path, and the same change is reused
  h.herdr.status = "done";
  await h.workflow.advance(reload(h, run.id));
  await h.workflow.advance(reload(h, run.id));
  assert.equal(reload(h, run.id).status, "validating");
  const worktree = reload(h, run.id).worktreePath;
  writeFileSync(join(worktree, "fix.txt"), "fixed\n");
  git(worktree, "add", "-A");
  git(worktree, "-c", "user.email=a@b.c", "-c", "user.name=a", "commit", "-m", "fix: review");
  await h.workflow.advance(reload(h, run.id)); // validating -> creating_change
  await h.workflow.advance(reload(h, run.id)); // existing change -> checking
  await new Promise((r) => setTimeout(r, 350));
  await h.workflow.advance(reload(h, run.id)); // green checks -> review
  const back = reload(h, run.id);
  assert.equal(back.status, "review");
  assert.equal(h.code.created, 0);
  assert.equal(db.getTask(h.db, "T-1")?.status, "in_review");
  // the reviewer has to see the fix: the second round pushes even though the change exists
  assert.equal(git(h.repo.origin, "rev-parse", back.branch), git(worktree, "rev-parse", "HEAD"));
});

test("rework does not loop forever", async () => {
  const h = harness({ orchestrator: { max_review_rounds: 1 } });
  const run = await parkedInReview(h);

  db.setTaskStatus(h.db, "T-1", "todo");
  await h.workflow.advance(reload(h, run.id));
  assert.equal(reload(h, run.id).status, "working");

  db.updateRun(h.db, run.id, { status: "review" });
  db.setTaskStatus(h.db, "T-1", "todo");
  await h.workflow.advance(reload(h, run.id));
  assert.equal(reload(h, run.id).status, "failed");
  assert.match(reload(h, run.id).error!, /review rounds/i);
  assert.equal(db.countEvents(h.db, run.id, "ReviewRoundsExhausted"), 1);
});

test("the review agent comes back for the next round", async () => {
  const h = harness({ agents: { review: { kind: "claude", prompt: "/code-review" } } });
  const run = await parkedInReview(h);
  await h.workflow.advance(reload(h, run.id));
  assert.equal(h.herdr.tabs, 1);
  const prompts = h.herdr.prompts.length;

  db.setTaskStatus(h.db, "T-1", "todo");
  await h.workflow.advance(reload(h, run.id)); // -> working, ReviewRejected
  db.updateRun(h.db, run.id, { status: "review" });
  db.setTaskStatus(h.db, "T-1", "in_review");
  await h.workflow.advance(reload(h, run.id));
  // the agent is still alive in its tab, so it is prompted again rather than spawned twice
  assert.equal(h.herdr.tabs, 1);
  assert.match(h.herdr.prompts.at(-1)!, /^\/code-review https:\/\/fake\/mr\/7/);
  assert.equal(h.herdr.prompts.length, prompts + 2);

  // but a review agent that died with its pane is spawned again, in a tab of its own
  db.setTaskStatus(h.db, "T-1", "todo");
  await h.workflow.advance(reload(h, run.id));
  db.updateRun(h.db, run.id, { status: "review" });
  db.setTaskStatus(h.db, "T-1", "in_review");
  h.herdr.agents.length = 0;
  await h.workflow.advance(reload(h, run.id));
  assert.equal(h.herdr.tabs, 2);
});

test("an approved change with green checks is merged by shepherd", async () => {
  const h = harness();
  const run = await parkedInReview(h);
  h.code.approved = true;
  h.code.checks = "pending";
  await h.workflow.advance(reload(h, run.id));
  assert.equal(h.code.merged, 0, "not while checks are still running");

  await new Promise((r) => setTimeout(r, 350));
  h.code.checks = "success";
  await h.workflow.advance(reload(h, run.id));
  assert.equal(h.code.merged, 1);
  assert.equal(reload(h, run.id).status, "review", "completion waits for the forge to say merged");
  assert.equal(db.countEvents(h.db, run.id, "ChangeMerged"), 1);

  // the forge can lag reporting the merge (queue, gated pipeline, API lag) — the button
  // is not pressed twice while shepherd waits for it to catch up
  await new Promise((r) => setTimeout(r, 350));
  await h.workflow.advance(reload(h, run.id));
  assert.equal(h.code.merged, 1);
  assert.equal(db.countEvents(h.db, run.id, "MergeFailed"), 0);

  await new Promise((r) => setTimeout(r, 350));
  h.code.state = "merged";
  await h.workflow.advance(reload(h, run.id));
  assert.equal(reload(h, run.id).status, "completed");
});

test("auto_merge = false leaves merging to a human", async () => {
  const h = harness({ orchestrator: { auto_merge: false } });
  const run = await parkedInReview(h);
  h.code.approved = true;
  await h.workflow.advance(reload(h, run.id));
  assert.equal(h.code.merged, 0);
  assert.equal(reload(h, run.id).status, "review");
});

test("a merge that fails is reported once and stops being retried", async () => {
  const h = harness();
  const run = await parkedInReview(h);
  h.code.approved = true;
  h.code.mergeChange = async () => {
    h.code.merged++;
    throw new Error("merge conflict");
  };
  for (let i = 0; i < 5; i++) {
    await h.workflow.advance(reload(h, run.id));
    await new Promise((r) => setTimeout(r, 350));
  }
  assert.equal(h.code.merged, 3);
  assert.equal(reload(h, run.id).status, "review");
  assert.equal(h.tasks.comments.filter((c) => /merge conflict/.test(c)).length, 1);
});

test("a retried run takes over the change row of the change it finds again", async () => {
  const h = harness();
  const run1 = await parkedInReview(h);
  db.updateRun(h.db, run1.id, { status: "failed", finishedAt: new Date() });

  const run2 = await h.workflow.start(h.project, h.task);
  db.updateRun(h.db, run2.id, { status: "creating_change" });
  await h.workflow.advance(reload(h, run2.id));

  // the forge hands back the same open change, so the row has to follow the live run:
  // left on the dead one, the checks step finds nothing and bounces back here every tick
  assert.equal(db.getChangeForRun(h.db, run2.id)?.id, "7");
  assert.equal(reload(h, run2.id).status, "checking");
  assert.equal(h.code.created, 1);

  await h.workflow.advance(reload(h, run2.id));
  assert.equal(reload(h, run2.id).status, "review");
  assert.equal(h.code.created, 1, "no second change, and no second 'Pull request:' comment");
});

test("a task with an open change already on its branch resumes there with the review comments", async () => {
  const h = harness();
  h.code.existing = { id: "7", provider: "fake", url: "https://fake/mr/7", status: "open" };
  h.code.comments = [
    { author: "egor", body: "always true", path: "src/a.ts", line: 237, createdAt: new Date() },
  ];
  const run = await h.workflow.start(h.project, h.task);
  assert.equal(run.status, "working");
  assert.match(h.herdr.prompts[0]!, /Do the thing/);
  assert.match(h.herdr.prompts[0]!, /src\/a\.ts:237.*always true/);
  assert.equal(db.getChangeForRun(h.db, run.id)?.id, "7");
  assert.equal(db.countEvents(h.db, run.id, "ChangeResumed"), 1);

  // the fixes land on the same change: nothing new is opened
  const worktree = reload(h, run.id).worktreePath;
  writeFileSync(join(worktree, "fix.txt"), "fixed\n");
  git(worktree, "add", "-A");
  git(worktree, "-c", "user.email=a@b.c", "-c", "user.name=a", "commit", "-m", "fix: review");
  h.herdr.status = "done";
  await h.workflow.advance(reload(h, run.id));
  await h.workflow.advance(reload(h, run.id));
  await h.workflow.advance(reload(h, run.id)); // validating -> creating_change
  await h.workflow.advance(reload(h, run.id)); // push -> checking
  await new Promise((r) => setTimeout(r, 350));
  await h.workflow.advance(reload(h, run.id)); // green checks -> review
  assert.equal(reload(h, run.id).status, "review");
  assert.equal(h.code.created, 0);
  assert.equal(git(h.repo.origin, "rev-parse", run.branch), git(worktree, "rev-parse", "HEAD"));
});

test("a merged change on the branch is not resumed", async () => {
  const h = harness();
  h.code.existing = { id: "7", provider: "fake", url: "https://fake/mr/7", status: "merged" };
  const run = await h.workflow.start(h.project, h.task);
  assert.equal(db.getChangeForRun(h.db, run.id), undefined);
  assert.doesNotMatch(h.herdr.prompts[0]!, /sent back/);
});

test("Done in the tracker accepts the change the way an approval does", async () => {
  const h = harness();
  const run = await parkedInReview(h);
  h.tasks.remote = "done"; // a human closed the task instead of approving the pull request

  await h.workflow.advance(reload(h, run.id));
  assert.equal(h.code.merged, 1);
  assert.equal(db.countEvents(h.db, run.id, "TaskAccepted"), 1);

  await new Promise((r) => setTimeout(r, 350));
  h.code.state = "merged";
  await h.workflow.advance(reload(h, run.id));
  assert.equal(reload(h, run.id).status, "completed");
});

test("Done does not merge over failing checks", async () => {
  const h = harness();
  const run = await parkedInReview(h);
  h.tasks.remote = "done";
  h.code.checks = "failure";
  await h.workflow.advance(reload(h, run.id));
  assert.equal(h.code.merged, 0);
  assert.equal(reload(h, run.id).status, "review");
});

test("a planning pass runs before the work, in the same agent", async () => {
  const h = harness({ agents: { plan: { prompt: "/plan", skill: "superpowers:writing-plans" } } });
  const run = await h.workflow.start(h.project, h.task);
  assert.equal(run.status, "planning");
  assert.match(h.herdr.prompts[0]!, /Use the superpowers:writing-plans skill\./);
  assert.match(h.herdr.prompts[0]!, /shepherd task comment T-1/);

  // planning survives the agent being busy; it is not the work phase yet
  h.herdr.status = "working";
  await h.workflow.advance(reload(h, run.id));
  assert.equal(reload(h, run.id).status, "planning");

  // and one idle poll is a pause here too
  h.herdr.status = "done";
  await h.workflow.advance(reload(h, run.id));
  assert.equal(reload(h, run.id).status, "planning");

  await h.workflow.advance(reload(h, run.id));
  assert.equal(reload(h, run.id).status, "working");
  // the same agent is handed the work prompt, no second agent is started
  assert.equal(h.herdr.prompts.length, 2);
  assert.match(h.herdr.prompts[1]!, /Implement the task/);
  assert.equal(h.herdr.agents.length, 1);
});

test("without a plan role the dev agent starts straight away", async () => {
  const h = harness();
  const run = await h.workflow.start(h.project, h.task);
  assert.equal(run.status, "working");
  assert.match(h.herdr.prompts[0]!, /Implement the task/);
});

test("an agent herdr knows by name is reused even when the list does not report it", async () => {
  // "agent name X is already used; candidates: ... status=Idle" — it was alive all along
  const h = harness();
  h.herdr.unlisted.add("demo-t-1");
  const run = await h.workflow.start(h.project, h.task);
  assert.equal(h.herdr.agents.length, 0, "no second agent under a name herdr already holds");
  assert.equal(db.hasEvent(h.db, run.id, "AgentReused"), true);
  assert.equal(h.herdr.tabs, 0, "and no tab is opened for an agent we already have");
});

test("a name left behind by a dead agent is cleared, and the start retried once", async () => {
  const h = harness();
  h.herdr.startErrors = ["named agent demo-t-1 no longer owns the target terminal"];
  const run = await h.workflow.start(h.project, h.task);
  assert.deepEqual(h.herdr.cleared, ["demo-t-1"]);
  assert.equal(h.herdr.agents.length, 1);
  assert.equal(run.status, "working");
});

test("a start that fails for its own reason still fails the run", async () => {
  const h = harness();
  h.herdr.startErrors = [
    "timed out waiting for agent startup",
    "timed out waiting for agent startup",
  ];
  const run = await h.workflow.start(h.project, h.task);
  assert.equal(run.status, "failed");
  assert.deepEqual(h.herdr.cleared, [], "a timeout is not a stale name");
});

test("a forge read that blinks is retried; one that stays down skips the tick", async () => {
  const h = harness();
  const run = await parkedInReview(h);

  // two failures in a row are ridden out inside one tick
  await new Promise((r) => setTimeout(r, 350));
  h.code.getChangeErrors = 2;
  h.code.state = "merged";
  await h.workflow.advance(reload(h, run.id));
  assert.equal(reload(h, run.id).status, "completed");

  // and a read that never comes back leaves the run where it was instead of failing it
  const down = harness();
  const stuck = await parkedInReview(down);
  // the backoff is real time; the checks do not need to sit through it
  await new Promise((r) => setTimeout(r, 350));
  down.code.getChangeErrors = 99;
  await down.workflow.advance(reload(down, stuck.id));
  assert.equal(reload(down, stuck.id).status, "review", "a dead forge is not a failed run");
});

test("a new change waits for its checks before anyone is asked to review", async () => {
  const h = harness({ agents: { review: { kind: "claude", prompt: "/code-review" } } });
  const run = await workedThrough(h);
  await h.workflow.advance(reload(h, run.id)); // validating -> creating_change
  await h.workflow.advance(reload(h, run.id)); // push + create

  const created = reload(h, run.id);
  assert.equal(created.status, "checking", "the pull request exists, the pipeline has not run");
  assert.equal(h.tasks.statuses.at(-1), "in_progress", "nobody is told it is ready for review yet");
  assert.equal(
    h.herdr.agents.length,
    1,
    "and the reviewer is not started over a red-or-unknown CI",
  );

  h.code.checks = "pending";
  await h.workflow.advance(reload(h, run.id));
  assert.equal(reload(h, run.id).status, "checking");

  await new Promise((r) => setTimeout(r, 350));
  h.code.checks = "success";
  await h.workflow.advance(reload(h, run.id));
  assert.equal(reload(h, run.id).status, "review");
  assert.equal(h.tasks.statuses.at(-1), "in_review");
  assert.equal(h.herdr.agents.length, 2, "the reviewer starts once the pipeline is green");
});

test("a red pipeline goes back to the agent, and gives up after the cap", async () => {
  const h = harness({ orchestrator: { max_checks_rounds: 2 } });
  const run = await workedThrough(h);
  await h.workflow.advance(reload(h, run.id));
  await h.workflow.advance(reload(h, run.id));
  assert.equal(reload(h, run.id).status, "checking");

  h.code.checks = "failure";
  await h.workflow.advance(reload(h, run.id));
  const back = reload(h, run.id);
  assert.equal(back.status, "working", "the agent that wrote it fixes it");
  assert.match(h.herdr.prompts.at(-1)!, /https:\/\/fake\/mr\/7/);
  assert.match(h.herdr.prompts.at(-1)!, /checks|pipeline/i);
  assert.equal(db.countEvents(h.db, run.id, "ChecksRejected"), 1);

  // second round, then the cap: a human takes over instead of the run failing
  for (let i = 0; i < 2; i++) {
    db.updateRun(h.db, run.id, { status: "checking" });
    await new Promise((r) => setTimeout(r, 350));
    await h.workflow.advance(reload(h, run.id));
  }
  assert.equal(db.countEvents(h.db, run.id, "ChecksRejected"), 2, "capped at max_checks_rounds");
  assert.equal(reload(h, run.id).status, "review", "a change nobody can make green is left for us");
});

test("a repository with no CI at all is not held up", async () => {
  const h = harness();
  const run = await workedThrough(h);
  await h.workflow.advance(reload(h, run.id));
  await h.workflow.advance(reload(h, run.id));
  // the forge reports success when there is nothing to run
  await h.workflow.advance(reload(h, run.id));
  assert.equal(reload(h, run.id).status, "review");
});
