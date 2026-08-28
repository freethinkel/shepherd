import assert from "node:assert/strict";
import { homedir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { ConfigSchema } from "../src/config/schema.ts";
import type { AgentRun, Project, Task } from "../src/domain/types.ts";
import {
  agentName,
  buildPrompt,
  codeProviderForRemote,
  isTaskAvailable,
  providerSettings,
  resolveAgentRole,
  reviewAgentName,
  withPrefix,
  workspaceLabel,
  worktreePath,
} from "../src/orchestrator/policies.ts";
import { branchName } from "../src/repositories/git.ts";

const project: Project = { id: "phocus", name: "Phocus", repositoryId: "/repo" };
const task: Task = {
  id: "LIN-42",
  providerId: "uuid",
  projectId: "phocus",
  title: "LUT importer",
  status: "todo",
};
const run = (status: AgentRun["status"]): AgentRun => ({
  id: "run_1",
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

test("names stay valid for herdr and readable for humans", () => {
  assert.equal(agentName(project, task), "phocus-lin-42");
  assert.equal(workspaceLabel(project, task), "phocus / LIN-42 / lut-importer");
  assert.equal(branchName(task.id, task.title), "agent/lin-42-lut-importer");
  assert.match(agentName({ ...project, name: "9lives" }, task), /^[a-z][a-z0-9_-]{0,31}$/);
  assert.match(reviewAgentName(agentName(project, task)), /^[a-z][a-z0-9_-]{0,31}$/);
  assert.ok(reviewAgentName("a".repeat(32)).length <= 32);
});

test("worktree override is absolute, because git -C does not know ~", () => {
  assert.equal(
    worktreePath("~/.shepherd/worktrees", project, task),
    join(homedir(), ".shepherd/worktrees/phocus-lin-42"),
  );
});

test("a task is off limits while it has a run that did not fail", () => {
  assert.equal(isTaskAvailable(task, []), true);
  assert.equal(isTaskAvailable(task, [run("working")]), false);
  assert.equal(isTaskAvailable(task, [run("completed")]), false);
  assert.equal(isTaskAvailable(task, [run("failed")]), true);
});

test("retries are capped, or a run failing at startup restarts every tick", () => {
  assert.equal(isTaskAvailable(task, [run("failed"), run("failed")], 3), true);
  assert.equal(isTaskAvailable(task, [run("failed"), run("failed"), run("failed")], 3), false);
});

test("the agent command goes before the task text", () => {
  assert.equal(withPrefix("/code-review", "PR #1"), "/code-review PR #1");
  assert.equal(withPrefix("  ", "PR #1"), "PR #1");
  assert.ok(
    buildPrompt(task, { branch: "b", prefix: "/brainstorm" }).startsWith("/brainstorm Task LIN-42"),
  );
  assert.ok(buildPrompt(task, { branch: "b" }).startsWith("Task LIN-42"));
});

test("roles: a project overrides the global section field by field", () => {
  const config = ConfigSchema.parse({
    agents: { dev: { kind: "codex" }, review: { kind: "claude", prompt: "/code-review" } },
    projects: [
      { name: "Mochi", repository: "~/m" },
      {
        name: "Fmc",
        repository: "~/f",
        agent: "claude",
        agents: { review: { prompt: "/code-review --strict" } },
      },
    ],
  });
  const [mochi, fmc] = config.projects;
  assert.deepEqual(resolveAgentRole("dev", config, mochi), { kind: "codex", prompt: "" });
  assert.deepEqual(resolveAgentRole("review", config, mochi), {
    kind: "claude",
    prompt: "/code-review",
  });
  assert.equal(resolveAgentRole("dev", config, fmc).kind, "claude");
  assert.deepEqual(resolveAgentRole("review", config, fmc), {
    kind: "claude",
    prompt: "/code-review --strict",
  });
});

test("review falls back to the dev agent kind", () => {
  const config = ConfigSchema.parse({
    projects: [{ name: "X", repository: "~/x", agent: "codex" }],
  });
  assert.equal(resolveAgentRole("review", config, config.projects[0]).kind, "codex");
});

test("the remote picks the code provider", () => {
  assert.equal(codeProviderForRemote("git@github.com:me/repo.git"), "github");
  assert.equal(codeProviderForRemote("https://gitlab.com/me/repo.git"), "gitlab");
  assert.equal(codeProviderForRemote("https://gitlab.company.internal/me/repo.git"), "gitlab");
  assert.equal(codeProviderForRemote("git@ssh.dev.azure.com:v3/org/proj/repo"), undefined);
});

test("a self-hosted forge has to be named: the host gives nothing away", () => {
  const remote = "ssh://git@ci.company.net:10022/team/repo.git";
  assert.equal(codeProviderForRemote(remote), undefined);
  assert.equal(codeProviderForRemote(remote, { gitlab: ["ci.company.net"] }), "gitlab");
});

test("a provider gets the shared section plus its own block", () => {
  const config = ConfigSchema.parse({
    task_provider: { assignee: "me" },
    task_providers: { jira: { url: "https://j", email: "me@c" } },
  });
  assert.deepEqual(providerSettings("task_provider", config, "jira"), {
    assignee: "me",
    url: "https://j",
    email: "me@c",
    name: "jira",
  });
  assert.deepEqual(providerSettings("task_provider", config, "linear"), {
    assignee: "me",
    name: "linear",
  });
});
