# Review Loop and Merge on Approval Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A task moved back to Todo while its run waits in review sends the review comments to the dev agent and goes round again; a change approved by a human with green checks is merged by shepherd.

**Architecture:** Both behaviours live in `Workflow.checkChange`, the only method that runs while a run is in `review`. Rework reuses `sendBack()` (the validation retry path) and the `ReviewRejected` event as its round counter; merge reuses `CodeProvider.mergeChange()`, which both providers already implement. The forge gains two read methods (`listComments`, and `approved`/`checks` on `getChange`), nothing else changes shape. No LLM is involved.

**Tech Stack:** TypeScript on nub, `node:test`, `node:sqlite`, `gh` / `glab` CLIs.

**Spec:** `docs/superpowers/specs/2026-08-28-review-loop-design.md`

## Global Constraints

- Package manager and runner is `nub`. Typecheck: `nub run typecheck`. All tests: `nub run test`. One file: `nub --node --test tests/workflow.test.ts`. One test: `nub --node --test --test-name-pattern "<name>" tests/workflow.test.ts`.
- Imports carry the `.ts` extension; type-only imports use `import type`.
- `exactOptionalPropertyTypes` is on: optional fields are declared `foo?: T | undefined`.
- `domain/` and `orchestrator/policies.ts` know nothing about GitHub, GitLab or Herdr.
- New "did we already do X" state is an event in the `events` table, never a column.
- Errors from CLI tools reach the log through `briefError()`.
- Comments explain why a rule exists, not what the code does.
- Format with `nub run fmt` before each commit.

---

### Task 1: Domain types and config keys

**Files:**

- Modify: `src/domain/types.ts:77-83` (`Change`), `src/domain/types.ts:100-105` (`CodeProvider`)
- Modify: `src/config/schema.ts:70-97` (orchestrator block) and the `[orchestrator]` block of `EXAMPLE_CONFIG` in the same file

**Interfaces:**

- Produces:
  - `Change.approved?: boolean | undefined` and `Change.checks?: "pending" | "success" | "failure" | undefined`
  - `interface ChangeComment { author: string; body: string; path?: string | undefined; line?: number | undefined; createdAt: Date }`
  - `CodeProvider.listComments?(id: string, repoPath: string): Promise<ChangeComment[]>` (optional, so existing plugins keep loading)
  - `config.orchestrator.max_review_rounds: number` (default 3), `config.orchestrator.auto_merge: boolean` (default true)

- [ ] **Step 1: Extend `Change` and add `ChangeComment` in `src/domain/types.ts`**

Replace the `Change` interface with:

```ts
export interface Change {
  id: string; // "42" — PR/MR number
  runId: string;
  provider: string;
  url: string;
  status: "open" | "merged" | "closed";
  /** Read live from the forge, never stored: a human's approval is the merge signal. */
  approved?: boolean | undefined;
  /** "success" also when the repository runs no checks at all. */
  checks?: "pending" | "success" | "failure" | undefined;
}

export interface ChangeComment {
  author: string;
  body: string;
  path?: string | undefined;
  line?: number | undefined;
  createdAt: Date;
}
```

- [ ] **Step 2: Add `listComments` to `CodeProvider`**

```ts
export interface CodeProvider {
  check?(repoPath?: string): Promise<void>;
  createChange(input: CreateChangeInput): Promise<Omit<Change, "runId">>;
  getChange(id: string, repoPath: string): Promise<Omit<Change, "runId">>;
  mergeChange(id: string, repoPath: string): Promise<void>;
  /** Optional: a plugin without it still gets rework, the agent is just pointed at the URL. */
  listComments?(id: string, repoPath: string): Promise<ChangeComment[]>;
}
```

- [ ] **Step 3: Add the two config keys to the `orchestrator` object in `src/config/schema.ts`**

After `max_validation_rounds`:

```ts
      /** How many times review comments are handed back before the run gives up. */
      max_review_rounds: z.number().int().positive().default(3),
      /** Merge once a human approved and checks are green. Off means a human merges too. */
      auto_merge: z.boolean().default(true),
```

And in `EXAMPLE_CONFIG`, after the `max_validation_rounds` line:

```toml
# max_review_rounds = 3             # review comments handed back to the agent before giving up
# auto_merge = true                 # merge when a human approved and checks are green
```

- [ ] **Step 4: Typecheck and run tests**

Run: `nub run typecheck && nub run test`
Expected: both pass (nothing consumes the new fields yet).

- [ ] **Step 5: Commit**

```bash
nub run fmt
git add src/domain/types.ts src/config/schema.ts
git commit -m "Add review-loop types and config keys"
```

---

### Task 2: Pure rules — comment filtering and the rework prompt

**Files:**

- Modify: `src/orchestrator/policies.ts` (after `validationFeedback`)
- Test: `tests/policies.test.ts`

**Interfaces:**

- Consumes: `ChangeComment` from Task 1.
- Produces:
  - `commentsSince(comments: ChangeComment[], since: Date | undefined): ChangeComment[]`
  - `reviewFeedback(url: string, comments: ChangeComment[]): string`

- [ ] **Step 1: Write the failing tests**

Append to `tests/policies.test.ts` (extend the existing import from `../src/orchestrator/policies.ts` with `commentsSince, reviewFeedback`, and add `import type { ChangeComment } from "../src/domain/types.ts";`):

```ts
const comment = (body: string, at: string, extra: Partial<ChangeComment> = {}): ChangeComment => ({
  author: "reviewer",
  body,
  createdAt: new Date(at),
  ...extra,
});

test("review feedback only carries comments newer than the last round", () => {
  const all = [comment("old", "2026-01-01T00:00:00Z"), comment("new", "2026-01-02T00:00:00Z")];
  assert.deepEqual(
    commentsSince(all, new Date("2026-01-01T12:00:00Z")).map((c) => c.body),
    ["new"],
  );
  assert.equal(commentsSince(all, undefined).length, 2);
});

test("review feedback names the file and line, and points at the change when there is nothing", () => {
  const text = reviewFeedback("https://fake/mr/7", [
    comment("rename this", "2026-01-02T00:00:00Z", { path: "src/a.ts", line: 12 }),
    comment("LGTM otherwise", "2026-01-02T00:00:00Z"),
  ]);
  assert.match(text, /src\/a\.ts:12/);
  assert.match(text, /rename this/);
  assert.match(text, /commit .* same branch/i);

  const empty = reviewFeedback("https://fake/mr/7", []);
  assert.match(empty, /https:\/\/fake\/mr\/7/);
  assert.match(empty, /read the review/i);
});
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `nub --node --test tests/policies.test.ts`
Expected: FAIL — `commentsSince` / `reviewFeedback` are not exported.

- [ ] **Step 3: Implement both functions in `src/orchestrator/policies.ts`**

Add `ChangeComment` to the `import type { ... } from "../domain/types.ts"` line, then append after `noCommitsFeedback`:

```ts
/** Round two must not re-fix round one, so only what arrived since the last hand-back is sent. */
export function commentsSince(comments: ChangeComment[], since: Date | undefined): ChangeComment[] {
  return since ? comments.filter((c) => c.createdAt > since) : comments;
}

/**
 * What the dev agent gets when a human sends the task back. Without a comment list
 * (a plugin forge, or comments left somewhere we cannot read) the agent reads the
 * change itself: it has the forge CLI, the orchestrator does not need to.
 */
export function reviewFeedback(url: string, comments: ChangeComment[]): string {
  const body =
    comments.length === 0
      ? `Read the review on ${url} and address it.`
      : comments
          .map((c) => {
            const where = c.path ? ` (${c.path}${c.line ? `:${c.line}` : ""})` : "";
            return `- ${c.author}${where}: ${c.body.trim()}`;
          })
          .join("\n");
  return [
    `The change ${url} was sent back for rework:`,
    "",
    body,
    "",
    "Address every point and commit the fixes to the same branch.",
  ].join("\n");
}
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `nub --node --test tests/policies.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
nub run fmt
git add src/orchestrator/policies.ts tests/policies.test.ts
git commit -m "Add review feedback rules"
```

---

### Task 3: Rework — Todo during review hands comments back to the agent

**Files:**

- Modify: `src/orchestrator/workflow.ts:340-361` (`checkChange`), `src/orchestrator/workflow.ts:363-397` (`ensureReviewAgent`)
- Test: `tests/workflow.test.ts` (`FakeCode` at line 133, new tests at the end)

**Interfaces:**

- Consumes: `commentsSince`, `reviewFeedback` (Task 2); `Change`, `ChangeComment`, `config.orchestrator.max_review_rounds` (Task 1); existing `sendBack`, `db.countEvents`, `db.lastEventAt`, `db.hasEvent`.
- Produces: events `ReviewRejected { round }` and `ReviewRoundsExhausted`; `ensureReviewAgent` restarts once per round.

- [ ] **Step 1: Teach `FakeCode` in `tests/workflow.test.ts` to serve comments**

Replace the `FakeCode` class with:

```ts
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
  async getChange(): Promise<Omit<Change, "runId">> {
    this.polled++;
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
}
```

Add `ChangeComment` to the `import type { ... } from "../src/domain/types.ts"` list.

- [ ] **Step 2: Write the failing tests**

Append to `tests/workflow.test.ts`:

```ts
/** A run parked in review with its change recorded, the state a human sees a pull request in. */
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
  h.code.comments = [
    { author: "egor", body: "rename this", path: "src/a.ts", line: 3, createdAt: new Date() },
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
  await h.workflow.advance(reload(h, run.id)); // existing change -> review
  assert.equal(reload(h, run.id).status, "review");
  assert.equal(h.code.created, 0);
  assert.equal(db.getTask(h.db, "T-1")?.status, "in_review");
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
});
```

- [ ] **Step 3: Run the tests to see them fail**

Run: `nub --node --test --test-name-pattern "review" tests/workflow.test.ts`
Expected: the three new tests FAIL (run stays in `review`, no `ReviewRejected` event); the existing "a review agent lands in a sibling tab, exactly once" still passes.

- [ ] **Step 4: Add the rework branch to `checkChange` in `src/orchestrator/workflow.ts`**

Replace the beginning of `checkChange` (up to and including `await this.ensureReviewAgent(run, change.url);`) with:

```ts
  /** The run closes once a human merges the change, or goes round again when they send it back. */
  private async checkChange(run: AgentRun): Promise<void> {
    const change = db.getChangeForRun(this.deps.db, run.id);
    if (!change) {
      this.transition(run, "creating_change");
      return;
    }
    // A task back in Todo while its run waits in review is a human saying "rework".
    // Our own fail() also parks tasks in Todo, but by then the run is failed, not in review.
    if (db.getTask(this.deps.db, run.taskId)?.status === "todo") {
      await this.rework(run, change);
      return;
    }
    await this.ensureReviewAgent(run, change.url);
```

Then add the method right after `checkChange`:

```ts
  /**
   * Review comments go back to the same agent, capped like validation: a reviewer and
   * an agent who never agree would otherwise trade the task forever.
   */
  private async rework(run: AgentRun, change: Change): Promise<void> {
    const rounds = db.countEvents(this.deps.db, run.id, "ReviewRejected");
    const max = this.deps.config.orchestrator.max_review_rounds;
    if (rounds >= max) {
      this.event("ReviewRoundsExhausted", run, { rounds });
      this.fail(run, `still sent back after ${max} review rounds`);
      return;
    }
    // only what arrived since this change was opened or last handed back
    const since = [
      db.lastEventAt(this.deps.db, run.taskId, "ReviewRejected"),
      db.lastEventAt(this.deps.db, run.taskId, "ChangeCreated"),
    ]
      .filter((d): d is Date => d !== undefined)
      .sort((a, b) => b.getTime() - a.getTime())[0];
    const provider = this.codeProvider(run);
    const comments = provider.listComments
      ? await provider.listComments(change.id, run.worktreePath).catch((err) => {
          this.log(`comments ${run.id}: ${briefError(err)}`);
          return [];
        })
      : [];
    this.event("ReviewRejected", run, { round: rounds + 1, comments: comments.length });
    this.prompted.set(run.id, { at: Date.now(), sawWorking: false });
    this.idleTicks.delete(run.id);
    await this.deps.herdr.prompt(
      run.herdrAgentId,
      policy.reviewFeedback(change.url, policy.commentsSince(comments, since)),
    );
    this.transition(run, "working");
  }
```

Add `Change` to the `import type { AgentRun, ... } from "../domain/types.ts"` line.

Note `sendBack` is not called directly because its counter is `ValidationRejected`; the prompt/idle bookkeeping is the same three lines and is repeated on purpose rather than parameterised.

- [ ] **Step 5: Let the review agent run once per round in `ensureReviewAgent`**

Replace the line `if (db.hasEvent(this.deps.db, run.id, "ReviewAgentStarted")) return false;` with:

```ts
// once per round: after a hand-back the agent reviews the new push, not the old one
const started = db.lastEventAt(this.deps.db, run.taskId, "ReviewAgentStarted");
const rejected = db.lastEventAt(this.deps.db, run.taskId, "ReviewRejected");
if (started && (!rejected || started > rejected)) return false;
```

And replace the `try` body so a live review agent is prompted instead of spawned twice:

```ts
    try {
      const name = policy.reviewAgentName(run.herdrAgentId);
      const live = await this.deps.herdr.listAgents().catch(() => []);
      let tabId: string | undefined;
      if (!live.some((a) => a.name === name)) {
        const tab = await this.deps.herdr.createTab({
          workspaceId: run.herdrWorkspaceId,
          cwd: run.worktreePath,
          label: "review",
        });
        tabId = tab.tabId;
        await this.deps.herdr.spawnAgent({ name, kind: role.kind, paneId: tab.paneId });
      }
      await this.deps.herdr.prompt(
        name,
        policy.reviewPrompt(role.prompt, task, { changeUrl: url, branch: run.branch }),
      );
      this.event("ReviewAgentStarted", run, { agent: name, tab: tabId });
      return true;
    } catch (err: any) {
```

`lastEventAt` is keyed by task, which is right here: a retried run creates a new change and a new `ChangeCreated`, so an older run's events never win the comparison.

- [ ] **Step 6: Run the workflow tests**

Run: `nub --node --test tests/workflow.test.ts`
Expected: PASS, including "a review agent lands in a sibling tab, exactly once" (now enforced by the timestamp comparison rather than `hasEvent`).

- [ ] **Step 7: Typecheck, full suite, commit**

Run: `nub run typecheck && nub run test`
Expected: PASS

```bash
nub run fmt
git add src/orchestrator/workflow.ts tests/workflow.test.ts
git commit -m "Send review comments back to the agent when a task returns to Todo"
```

---

### Task 4: Merge after human approval

**Files:**

- Modify: `src/orchestrator/workflow.ts` (`checkChange`, the poll section)
- Test: `tests/workflow.test.ts`

**Interfaces:**

- Consumes: `Change.approved` / `Change.checks` (Task 1), `config.orchestrator.auto_merge` (Task 1), `CodeProvider.mergeChange`.
- Produces: events `ChangeMerged` and `MergeFailed { error }`.

- [ ] **Step 1: Write the failing tests**

Append to `tests/workflow.test.ts`:

```ts
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
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `nub --node --test --test-name-pattern "merge" tests/workflow.test.ts`
Expected: FAIL — `merged` stays 0.

- [ ] **Step 3: Add the merge step to the poll in `checkChange`**

Replace the tail of `checkChange` (from `const fresh = await this.codeProvider(run).getChange(...)` to the end of the method) with:

```ts
    const fresh = await this.codeProvider(run).getChange(change.id, run.worktreePath);
    if (fresh.status === "merged") {
      this.transition(run, "completed", { finishedAt: new Date() });
      this.event("RunCompleted", run, { change: fresh.url });
      await this.deps.herdr.closeWorkspace(run.herdrWorkspaceId).catch(() => {});
      db.closeWorkspaceRow(this.deps.db, run.herdrWorkspaceId);
    } else if (fresh.status === "closed") {
      this.fail(run, `change ${fresh.url} was closed`);
    } else if (fresh.approved && fresh.checks === "success") {
      await this.merge(run, change);
    }
  }

  /**
   * The human's approval is the decision; shepherd only presses the button. A merge that
   * keeps failing (conflict, protected branch) is not the run's fault, so the run stays in
   * review for a human, and the attempts are capped so the log is not spammed every minute.
   */
  private async merge(run: AgentRun, change: Change): Promise<void> {
    if (!this.deps.config.orchestrator.auto_merge) return;
    const failures = db.countEvents(this.deps.db, run.id, "MergeFailed");
    if (failures >= 3) return;
    try {
      await this.codeProvider(run).mergeChange(change.id, run.worktreePath);
      this.event("ChangeMerged", run, { change: change.url });
    } catch (err: any) {
      const error = briefError(err);
      this.event("MergeFailed", run, { error });
      this.log(`merge ${run.id}: ${error}`);
      if (failures === 0) {
        this.taskProvider(run.taskId)
          .addComment(run.taskId, `Approved, but merging ${change.url} failed:\n\n\`${error}\``)
          .catch(() => {});
      }
    }
  }
```

- [ ] **Step 4: Run the workflow tests**

Run: `nub --node --test tests/workflow.test.ts`
Expected: PASS. The happy-path test still merges only when `state = "merged"` because `FakeCode.approved` defaults to `false`.

- [ ] **Step 5: Typecheck, full suite, commit**

Run: `nub run typecheck && nub run test`
Expected: PASS

```bash
nub run fmt
git add src/orchestrator/workflow.ts tests/workflow.test.ts
git commit -m "Merge a change once a human approved it and checks are green"
```

---

### Task 5: GitHub — approval, checks and comments

**Files:**

- Modify: `src/providers/code/github.ts`
- Test: `tests/providers.test.ts`

**Interfaces:**

- Consumes: `Change.approved`/`checks`, `ChangeComment`, `CodeProvider.listComments` (Task 1).
- Produces: exported pure helper `githubChecks(rollup: unknown[]): "pending" | "success" | "failure"`; `GitHubCodeProvider.getChange` fills `approved`/`checks`; `GitHubCodeProvider.listComments`.

- [ ] **Step 1: Write the failing test for the checks mapping**

Append to `tests/providers.test.ts` (add `import { githubChecks } from "../src/providers/code/github.ts";`):

```ts
test("GitHub: the check rollup collapses to one verdict, and no checks means green", () => {
  assert.equal(githubChecks([]), "success");
  assert.equal(
    githubChecks([{ __typename: "CheckRun", status: "COMPLETED", conclusion: "SUCCESS" }]),
    "success",
  );
  assert.equal(
    githubChecks([{ __typename: "CheckRun", status: "IN_PROGRESS", conclusion: null }]),
    "pending",
  );
  assert.equal(
    githubChecks([
      { __typename: "CheckRun", status: "COMPLETED", conclusion: "SUCCESS" },
      { __typename: "StatusContext", state: "FAILURE" },
    ]),
    "failure",
  );
  assert.equal(
    githubChecks([{ __typename: "CheckRun", status: "COMPLETED", conclusion: "SKIPPED" }]),
    "success",
  );
});
```

- [ ] **Step 2: Run it to see it fail**

Run: `nub --node --test tests/providers.test.ts`
Expected: FAIL — `githubChecks` is not exported.

- [ ] **Step 3: Implement in `src/providers/code/github.ts`**

Add `ChangeComment` to the type import. Add the helper above the class:

```ts
/**
 * `statusCheckRollup` mixes CheckRun (status + conclusion) and StatusContext (state).
 * SKIPPED and NEUTRAL do not block a merge on GitHub either, so they count as success.
 */
export function githubChecks(rollup: unknown[]): "pending" | "success" | "failure" {
  let pending = false;
  for (const item of rollup as any[]) {
    const verdict: string =
      item.__typename === "StatusContext" ? item.state : (item.conclusion ?? "");
    if (item.__typename !== "StatusContext" && item.status !== "COMPLETED") pending = true;
    else if (verdict === "PENDING" || verdict === "EXPECTED") pending = true;
    else if (!["SUCCESS", "SKIPPED", "NEUTRAL"].includes(verdict)) return "failure";
  }
  return pending ? "pending" : "success";
}
```

Replace `getChange`:

```ts
  async getChange(id: string, repoPath: string): Promise<Omit<Change, "runId">> {
    const raw = await gh(repoPath, [
      "pr",
      "view",
      id,
      "--json",
      "number,url,state,reviewDecision,statusCheckRollup",
    ]);
    const pr = JSON.parse(raw);
    return {
      ...this.toChange(pr),
      approved: pr.reviewDecision === "APPROVED",
      checks: githubChecks(pr.statusCheckRollup ?? []),
    };
  }
```

Add `listComments` after `mergeChange`:

```ts
  /** Line comments, review summaries and plain comments are three endpoints on GitHub. */
  async listComments(id: string, repoPath: string): Promise<ChangeComment[]> {
    const api = (path: string) =>
      gh(repoPath, ["api", "--paginate", `repos/{owner}/{repo}/${path}`]).then(
        (raw) => JSON.parse(raw || "[]") as any[],
      );
    const [line, reviews, plain] = await Promise.all([
      api(`pulls/${id}/comments`),
      api(`pulls/${id}/reviews`),
      api(`issues/${id}/comments`),
    ]);
    const toComment = (c: any, path?: string, line?: number): ChangeComment => ({
      author: c.user?.login ?? "unknown",
      body: c.body ?? "",
      path,
      line,
      createdAt: new Date(c.created_at ?? c.submitted_at),
    });
    return [
      ...line.map((c) => toComment(c, c.path, c.line ?? c.original_line ?? undefined)),
      ...reviews.filter((r) => r.body?.trim()).map((r) => toComment(r)),
      ...plain.map((c) => toComment(c)),
    ].sort((a, b) => a.createdAt.getTime() - b.createdAt.getTime());
  }
```

- [ ] **Step 4: Run the tests and typecheck**

Run: `nub run typecheck && nub --node --test tests/providers.test.ts`
Expected: PASS

- [ ] **Step 5: Check against a real pull request**

Run in any GitHub-backed clone with an open PR (replace `N`):
`gh pr view N --json reviewDecision,statusCheckRollup` and `gh api repos/{owner}/{repo}/pulls/N/comments --paginate | head -c 400`
Expected: the JSON shapes the code reads (`reviewDecision` string, rollup array with `__typename`, comments with `user.login`, `path`, `line`, `created_at`). If a field name differs, fix the mapping, not the test.

- [ ] **Step 6: Commit**

```bash
nub run fmt
git add src/providers/code/github.ts tests/providers.test.ts
git commit -m "GitHub: read approval, checks and review comments"
```

---

### Task 6: GitLab — approval, pipeline and notes

**Files:**

- Modify: `src/providers/code/gitlab.ts`
- Test: `tests/providers.test.ts`

**Interfaces:**

- Consumes: same as Task 5.
- Produces: exported pure helper `gitlabChecks(pipeline: { status?: string } | null | undefined)`; `GitLabCodeProvider.getChange` fills `approved`/`checks`; `GitLabCodeProvider.listComments`.

- [ ] **Step 1: Write the failing test**

Append to `tests/providers.test.ts` (extend the gitlab import with `gitlabChecks`):

```ts
test("GitLab: pipeline status collapses to one verdict, and no pipeline means green", () => {
  assert.equal(gitlabChecks(null), "success");
  assert.equal(gitlabChecks({ status: "success" }), "success");
  assert.equal(gitlabChecks({ status: "skipped" }), "success");
  assert.equal(gitlabChecks({ status: "running" }), "pending");
  assert.equal(gitlabChecks({ status: "created" }), "pending");
  assert.equal(gitlabChecks({ status: "failed" }), "failure");
  assert.equal(gitlabChecks({ status: "canceled" }), "failure");
});
```

- [ ] **Step 2: Run it to see it fail**

Run: `nub --node --test tests/providers.test.ts`
Expected: FAIL — `gitlabChecks` is not exported.

- [ ] **Step 3: Implement in `src/providers/code/gitlab.ts`**

Add `ChangeComment` to the type import. Add the helper above the class:

```ts
/** `head_pipeline` is null when the project runs no CI; that must not block a merge. */
export function gitlabChecks(
  pipeline: { status?: string } | null | undefined,
): "pending" | "success" | "failure" {
  const status = pipeline?.status;
  if (!status || status === "success" || status === "skipped" || status === "manual") {
    return "success";
  }
  if (
    ["created", "waiting_for_resource", "preparing", "pending", "running", "scheduled"].includes(
      status,
    )
  ) {
    return "pending";
  }
  return "failure";
}
```

Add one private accessor so both transports read the same endpoints:

```ts
  /** GET through glab when it is logged in, otherwise straight to the REST API. */
  private async get(repoPath: string, path: string): Promise<any> {
    if (await this.useGlab(repoPath)) {
      const project = this.settings.project
        ? encodeURIComponent(String(this.settings.project))
        : ":id"; // glab resolves :id to the current project from the remote
      return JSON.parse(await this.glab(repoPath, ["api", `projects/${project}${path}`]));
    }
    return this.api(`/projects/${await this.project(repoPath)}${path}`);
  }
```

Replace `getChange`:

```ts
  async getChange(id: string, repoPath: string): Promise<Omit<Change, "runId">> {
    const [mr, approvals] = await Promise.all([
      this.get(repoPath, `/merge_requests/${id}`),
      this.get(repoPath, `/merge_requests/${id}/approvals`),
    ]);
    return {
      ...this.toChange(mr),
      approved: approvals.approved === true,
      checks: gitlabChecks(mr.head_pipeline),
    };
  }
```

Add `listComments` after `mergeChange`:

```ts
  /** MR notes carry both line and general comments; system notes are status noise. */
  async listComments(id: string, repoPath: string): Promise<ChangeComment[]> {
    const notes: any[] = await this.get(repoPath, `/merge_requests/${id}/notes?per_page=100&sort=asc`);
    return notes
      .filter((n) => !n.system && n.body?.trim())
      .map((n) => ({
        author: n.author?.username ?? "unknown",
        body: n.body,
        path: n.position?.new_path ?? undefined,
        line: n.position?.new_line ?? undefined,
        createdAt: new Date(n.created_at),
      }));
  }
```

Note: `glab api` with `-R` — the existing `glab()` helper appends `-R <project>` when `settings.project` is set; `glab api` accepts it as well, so nothing changes there. If `glab api projects/:id/...` fails to resolve on a self-hosted instance in Step 5, replace `:id` with `encodeURIComponent(projectPathFromRemote(remote))` read from `git remote get-url origin` as `project()` does.

- [ ] **Step 4: Run the tests and typecheck**

Run: `nub run typecheck && nub --node --test tests/providers.test.ts`
Expected: PASS

- [ ] **Step 5: Check against a real merge request**

In a GitLab-backed clone with an open MR (replace `N`):
`glab api "projects/:id/merge_requests/N/approvals"` and `glab api "projects/:id/merge_requests/N/notes?per_page=2"`
Expected: `approved` boolean in the first, notes with `author.username`, `system`, `position`, `created_at` in the second.

- [ ] **Step 6: Commit**

```bash
nub run fmt
git add src/providers/code/gitlab.ts tests/providers.test.ts
git commit -m "GitLab: read approval, pipeline and notes"
```

---

### Task 7: Documentation

**Files:**

- Modify: `README.md:140-146` (review paragraph), `README.md:187` (lifecycle line), `README.md:211` ("Not there yet")
- Modify: `CLAUDE.md` (invariants list)

- [ ] **Step 1: Update the review paragraph in `README.md`**

Replace the paragraph starting "`prompt` is what goes in front of the task text" so its tail reads:

```markdown
`prompt` is what goes in front of the task text: plain text, or a slash command the agent itself
understands. The dev agent works in the workspace root tab. The review agent comes up as a sibling
tab in the same workspace right after the change is created, and gets the link to it. It leaves its
notes as comments on the merge request. The orchestrator does not parse its answer. A failed review
does not fail the run.

Sending the task back to Todo in the tracker while the change is open means "rework": the
comments on the change (since it was opened, or since the last round) go to the dev agent, the task
returns to In Progress, and after validation the same change goes back to In Review with a fresh
review pass. `max_review_rounds` (default 3) caps that loop. Once a human approves the change and
checks are green, shepherd merges it; `auto_merge = false` leaves that to a human as well.
```

- [ ] **Step 2: Update the lifecycle line and "Not there yet"**

Lifecycle:

```markdown
`queued → starting → working → (blocked) → validating → creating_change → review → completed | failed`,
with `review → working` when a human sends the task back to Todo.
```

"Not there yet": remove `auto-merge` from the sentence:

```markdown
A TUI, Jira and Bitbucket as built-ins rather than plugins.
```

- [ ] **Step 3: Add the invariants to `CLAUDE.md`**

In "Invariants worth knowing before touching orchestration", after the `ValidationRejected` bullet:

```markdown
- **Todo during `review` means rework.** `checkChange` reads the synced task status; `ReviewRejected`
  is the round counter for `max_review_rounds`, and its timestamp is the cutoff for which comments
  are sent. `ReviewAgentStarted` newer than the last `ReviewRejected` is what keeps the review
  agent to one pass per round.
- **Merge is executed, never decided.** `approved && checks === "success"` from `getChange` calls
  `mergeChange`; `MergeFailed` is capped at three so a conflict does not spam the log every minute,
  and the run stays in `review` for a human.
```

- [ ] **Step 4: Full check and commit**

Run: `nub run typecheck && nub run lint && nub run test`
Expected: PASS (lint warning in `src/log.ts` is pre-existing).

```bash
git add README.md CLAUDE.md
git commit -m "Document the review loop and merge on approval"
```
