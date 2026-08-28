import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { githubChecks } from "../src/providers/code/github.ts";
import { projectPathFromRemote } from "../src/providers/code/gitlab.ts";
import { loadCustomProviders } from "../src/providers/load.ts";
import { buildIssueFilter, targetState } from "../src/providers/tasks/linear.ts";

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
    project: { name: { eq: "Mochi" } },
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
