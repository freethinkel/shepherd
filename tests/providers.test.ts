import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { githubApproved, githubChecks } from "../src/modules/providers/code/github.ts";
import {
  gitlabApproved,
  gitlabChecks,
  projectPathFromRemote,
} from "../src/modules/providers/code/gitlab.ts";
import { loadCustomProviders } from "../src/modules/providers/load.ts";
import { buildIssueFilter, targetState } from "../src/modules/providers/tasks/linear.ts";

test("Linear: In Progress and In Review are both `started`, so match by name", () => {
  const states = [
    { id: "1", name: "In Review", type: "started" },
    { id: "2", name: "In Progress", type: "started" },
    { id: "3", name: "Todo", type: "unstarted" },
    { id: "4", name: "Backlog", type: "backlog" },
    { id: "5", name: "Done", type: "completed" },
  ];
  assert.equal(targetState("in_progress", states)?.name, "In Progress");
  assert.equal(targetState("waiting_for_agent", states)?.name, "In Progress");
  assert.equal(targetState("in_review", states)?.name, "In Review");
  assert.equal(targetState("done", states)?.name, "Done");
  assert.equal(targetState("todo", states)?.name, "Todo");
});

test("Linear: only Todo, and only what is assigned", () => {
  assert.deepEqual(buildIssueFilter({ assignee: "me", taskProviderProjectId: "Mochi" }), {
    state: { type: { eq: "unstarted" } },
    project: { name: { eqIgnoreCase: "Mochi" } },
    assignee: { isMe: { eq: true } },
  });
  assert.deepEqual(buildIssueFilter({ assignee: "a@b.c" }).assignee, { email: { eq: "a@b.c" } });
  assert.equal(buildIssueFilter({ assignee: "any" }).assignee, undefined);
  assert.equal(buildIssueFilter({}).assignee, undefined);
});

test("GitLab: every remote form maps to the same project path", () => {
  for (const remote of [
    "git@gitlab.com:group/sub/repo.git",
    "https://gitlab.com/group/sub/repo.git",
    "ssh://git@gitlab.company.io/group/sub/repo.git",
    "https://gitlab.company.io/group/sub/repo\n",
  ]) {
    assert.equal(projectPathFromRemote(remote), "group/sub/repo");
  }
});

test("plugins register under their file name and a broken one is isolated", async () => {
  const dir = mkdtempSync(join(tmpdir(), "shepherd-providers-"));
  writeFileSync(join(dir, "jira.ts"), `export const taskProvider = (s) => ({ url: s.url });\n`);
  writeFileSync(
    join(dir, "gitlab.ts"),
    `export const codeProvider = () => ({ kind: "gitlab" });\n`,
  );
  writeFileSync(join(dir, "broken.ts"), `throw new Error("boom");\n`);
  writeFileSync(join(dir, "notes.md"), `not a module\n`);

  const custom = await loadCustomProviders([dir]);
  assert.deepEqual(custom.loaded, ["gitlab.ts", "jira.ts"]);
  assert.equal(custom.errors.length, 1);
  assert.match(custom.errors[0]!, /^broken\.ts:/);
  assert.equal((custom.tasks.jira as any)({ url: "https://jira" }).url, "https://jira");
  assert.ok(custom.code.gitlab);
});

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

test("GitHub: without a required-review rule the latest reviews decide", () => {
  assert.equal(githubApproved({ reviewDecision: "APPROVED", latestReviews: [] }), true);
  assert.equal(
    githubApproved({ reviewDecision: "REVIEW_REQUIRED", latestReviews: [{ state: "APPROVED" }] }),
    false,
  );
  assert.equal(
    githubApproved({ reviewDecision: "", latestReviews: [{ state: "APPROVED" }] }),
    true,
  );
  assert.equal(
    githubApproved({
      reviewDecision: "",
      latestReviews: [{ state: "APPROVED" }, { state: "CHANGES_REQUESTED" }],
    }),
    false,
  );
  assert.equal(githubApproved({ reviewDecision: null, latestReviews: null }), false);
});

test("GitLab: pipeline status collapses to one verdict, and no pipeline means green", () => {
  assert.equal(gitlabChecks(null), "success");
  assert.equal(gitlabChecks({ status: "success" }), "success");
  assert.equal(gitlabChecks({ status: "skipped" }), "success");
  assert.equal(gitlabChecks({ status: "running" }), "pending");
  assert.equal(gitlabChecks({ status: "created" }), "pending");
  assert.equal(gitlabChecks({ status: "failed" }), "failure");
  assert.equal(gitlabChecks({ status: "canceled" }), "failure");
});

test("GitLab: approved requires a human, not just a satisfied (possibly empty) rule set", () => {
  assert.equal(gitlabApproved({ approved: true, approved_by: [] }), false);
  assert.equal(gitlabApproved({ approved: true, approved_by: [{}] }), true);
  assert.equal(gitlabApproved({}), false);
  assert.equal(gitlabApproved({ approved: false, approved_by: [{}] }), false);
});
