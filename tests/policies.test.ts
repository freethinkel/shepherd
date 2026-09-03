import assert from "node:assert/strict";
import { homedir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { ConfigSchema, EXAMPLE_CONFIG } from "../src/core/config/schema.ts";
import type { AgentRun, ChangeComment, Project, Task } from "../src/shared/domain/types.ts";
import { parse as parseYaml } from "yaml";
import {
  agentName,
  buildPlanPrompt,
  buildPrompt,
  retry,
  codeProviderForRemote,
  commentsSince,
  isTaskAvailable,
  providerSettings,
  resolveAgentRole,
  reviewAgentName,
  reviewFeedback,
  withPrefix,
  workspaceLabel,
  worktreePath,
} from "../src/modules/orchestrator/policies.ts";
import { branchName } from "../src/shared/git.ts";

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
  assert.deepEqual(resolveAgentRole("dev", config, mochi), {
    kind: "codex",
    prompt: "",
    skill: "",
    args: [],
  });
  assert.deepEqual(resolveAgentRole("review", config, mochi), {
    kind: "claude",
    prompt: "/code-review",
    skill: "",
    args: ["--dangerously-skip-permissions"],
  });
  assert.equal(resolveAgentRole("dev", config, fmc).kind, "claude");
  assert.deepEqual(resolveAgentRole("review", config, fmc), {
    kind: "claude",
    prompt: "/code-review --strict",
    skill: "",
    args: ["--dangerously-skip-permissions"],
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

test("claude is started with permission prompts off, and args stay overridable", () => {
  const cfg = (raw: object) => ConfigSchema.parse(raw);
  assert.deepEqual(
    resolveAgentRole("dev", cfg({ agents: { dev: { kind: "claude" } } }), undefined).args,
    ["--dangerously-skip-permissions"],
  );
  // an agent with no known default gets none
  assert.deepEqual(
    resolveAgentRole("dev", cfg({ agents: { dev: { kind: "codex" } } }), undefined).args,
    [],
  );
  // and an explicit list wins, including an empty one
  assert.deepEqual(
    resolveAgentRole(
      "dev",
      cfg({ agents: { dev: { kind: "claude", args: ["--foo"] } } }),
      undefined,
    ).args,
    ["--foo"],
  );
  assert.deepEqual(
    resolveAgentRole("dev", cfg({ agents: { dev: { kind: "claude", args: [] } } }), undefined).args,
    [],
  );
  // a project override beats the global list
  const project = cfg({
    agents: { dev: { kind: "claude", args: ["--foo"] } },
    projects: [{ name: "P", repository: "/repo", agents: { dev: { args: ["--bar"] } } }],
  });
  assert.deepEqual(resolveAgentRole("dev", project, project.projects[0]).args, ["--bar"]);
});

test("the example config `shepherd init` writes is valid", () => {
  const config = ConfigSchema.parse(parseYaml(EXAMPLE_CONFIG));
  assert.equal(config.projects[0]?.name, "Phocus");
  assert.equal(config.agents.review.prompt, "/code-review");
});

test("a skill is configured per role and overridden per project", () => {
  const cfg = (raw: object) => ConfigSchema.parse(raw);
  const global = cfg({ agents: { plan: { skill: "superpowers:writing-plans" } } });
  assert.equal(resolveAgentRole("plan", global, undefined).skill, "superpowers:writing-plans");

  const overridden = cfg({
    agents: { dev: { skill: "superpowers:test-driven-development" } },
    projects: [{ name: "P", repository: "/repo", agents: { dev: { skill: "flutter-architect" } } }],
  });
  assert.equal(
    resolveAgentRole("dev", overridden, overridden.projects[0]).skill,
    "flutter-architect",
  );
  // an empty override takes the skill off for that project alone
  const off = cfg({
    agents: { review: { skill: "dunk-review" } },
    projects: [{ name: "P", repository: "/repo", agents: { review: { skill: "" } } }],
  });
  assert.equal(resolveAgentRole("review", off, off.projects[0]).skill, "");
  assert.equal(resolveAgentRole("review", off, undefined).skill, "dunk-review");
});

const planTask: Task = {
  id: "MOC-1",
  providerId: "moc-1",
  title: "Add a share sheet",
  status: "todo",
  projectId: "mochi",
  provider: "linear",
};

test("a role's skill leads its prompt, in front of the configured prefix", () => {
  const prompt = buildPrompt(planTask, {
    branch: "moc-1-add-a-share-sheet",
    prefix: "/brainstorm",
    skill: "flutter-architect",
  });
  assert.match(prompt, /^Use the flutter-architect skill\. \/brainstorm Task MOC-1/);
  // no skill configured leaves the prompt exactly as it was
  assert.match(buildPrompt(planTask, { branch: "b", prefix: "/brainstorm" }), /^\/brainstorm Task/);
});

test("the plan prompt asks for a plan on the task and nothing else", () => {
  const prompt = buildPlanPrompt(planTask, {
    branch: "moc-1",
    skill: "superpowers:writing-plans",
  });
  assert.match(prompt, /^Use the superpowers:writing-plans skill\./);
  assert.match(prompt, /shepherd task comment MOC-1/);
  assert.match(prompt, /do not write any code/i);
  // a plan that cannot be executed is worse than no plan
  assert.match(prompt, /acceptance/i);
  assert.match(prompt, /open questions/i);
  // and a change too small to plan says so instead of padding the comment
  assert.match(prompt, /too small to be worth planning/i);
});

test("a flaky read is retried, a working one is not", async () => {
  const slept: number[] = [];
  const sleep = async (ms: number) => void slept.push(ms);

  let calls = 0;
  const flaky = await retry(
    async () => {
      calls++;
      if (calls < 3) throw new Error("Command failed: gh pr view 7");
      return "ok";
    },
    { sleep },
  );
  assert.equal(flaky, "ok");
  assert.equal(calls, 3);
  assert.deepEqual(slept, [1000, 2000], "backoff grows, and only between attempts");

  calls = 0;
  slept.length = 0;
  assert.equal(await retry(async () => ++calls, { sleep }), 1);
  assert.deepEqual(slept, [], "a call that works the first time never sleeps");
});

test("a read that never works throws the last error, after a bounded number of tries", async () => {
  let calls = 0;
  await assert.rejects(
    retry(
      async () => {
        calls++;
        throw new Error(`attempt ${calls}`);
      },
      { sleep: async () => {} },
    ),
    /attempt 3/,
  );
  assert.equal(calls, 3, "three attempts, not an unbounded loop");
});
