import { join } from "node:path";
import { expandPath } from "../../core/config/schema.ts";
import type { Config, ProjectConfig } from "../../core/config/schema.ts";
import type { AgentRun, ChangeComment, Project, Task } from "../../shared/domain/types.ts";

export const slug = (s: string) =>
  s
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");

export const projectId = (name: string) => slug(name);

/** Herdr agent name: [a-z][a-z0-9_-]{0,31}, unique among live agents. */
export function agentName(project: Project, task: Task): string {
  const name = `${slug(project.name)}-${slug(task.id)}`.slice(0, 32);
  return /^[a-z]/.test(name) ? name : `a-${name}`.slice(0, 32);
}

/** Workspace label: phocus / LIN-42 / lut-importer */
export function workspaceLabel(project: Project, task: Task): string {
  return `${slug(project.name)} / ${task.id} / ${slug(task.title).slice(0, 32)}`;
}

/** The worktree path is computed once and always absolute — the same value goes into SQLite. */
export function worktreePath(worktreesDir: string, project: Project, task: Task): string {
  return join(expandPath(worktreesDir), `${slug(project.name)}-${slug(task.id)}`);
}

export function canStart(activeRuns: number, maxConcurrent: number): boolean {
  return activeRuns < maxConcurrent;
}

/**
 * A task is available when it has no run other than failed ones, is not done,
 * and has not burned through max_attempts. Without that cap a run failing at
 * startup would be restarted every tick forever.
 */
export function isTaskAvailable(task: Task, runs: AgentRun[], maxAttempts = 3): boolean {
  if (task.status === "done") return false;
  if (runs.some((r) => r.status !== "failed")) return false;
  return runs.length < maxAttempts;
}

/** Settings handed to a provider: the shared section plus its own [task_providers.<name>] block. */
export function providerSettings(
  role: "task_provider" | "code_provider",
  config: Config,
  name: string,
): Record<string, unknown> {
  const shared = config[role] as Record<string, unknown>;
  const own = (config[`${role}s`] as Record<string, Record<string, unknown>>)[name] ?? {};
  return { ...shared, ...own, name };
}

/**
 * Run `work` after whatever is already queued under `key`. Tracker updates are not awaited by
 * the caller, so two of them for one task can land in the order the network felt like: a slow
 * `in_progress` overwriting the `in_review` that was made after it.
 */
export function enqueue<K>(
  chains: Map<K, Promise<void>>,
  key: K,
  work: () => Promise<void>,
  onError: (err: unknown) => void,
): Promise<void> {
  const next = (chains.get(key) ?? Promise.resolve()).then(() => work()).catch(onError);
  // deliberately not the plain .then(work): a broken chain would swallow the next update
  chains.set(key, next);
  return next;
}

/**
 * Which tracker a `shepherd task` command talks to. A task synced into SQLite remembers its
 * tracker; a brand-new one has nobody to ask, so a single registered tracker is the only
 * unambiguous case.
 */
export function pickTaskProvider(
  registered: string[],
  requested?: string | undefined,
  known?: string | undefined,
): string {
  const name = requested ?? known ?? (registered.length === 1 ? registered[0] : undefined);
  if (!name) throw new Error(`--provider is required; registered: ${registered.join(", ")}`);
  return name;
}

export function remoteHost(remote: string): string {
  return (
    remote
      .trim()
      .replace(/^[a-z+]+:\/\//, "")
      .replace(/^[^@]*@/, "")
      .split(/[:/]/)[0] ?? ""
  );
}

/**
 * Which code provider owns a repository, decided by its origin remote.
 * Self-hosted forges are named in [code_providers.<name>] hosts, because a host
 * like ci.company.net says nothing about what runs on it.
 */
export function codeProviderForRemote(
  remote: string,
  hosts: Record<string, string[]> = {},
): string | undefined {
  const host = remoteHost(remote);
  for (const [name, owned] of Object.entries(hosts)) {
    if (owned.some((h) => h === host)) return name;
  }
  if (host.includes("github")) return "github";
  if (host.includes("gitlab")) return "gitlab";
  if (host.includes("bitbucket")) return "bitbucket";
  return undefined;
}

/**
 * A start that failed only because the name is still registered. Herdr says either that the name
 * is used, or that the agent behind it lost its terminal; both clear with a rename.
 */
export function isStaleAgentName(error: string): boolean {
  return /name .* is already used|no longer owns the target terminal/i.test(error);
}

export interface AgentRole {
  kind: string;
  prompt: string;
  skill: string;
  args: string[];
}

/** An agent that stops to ask for permission never finishes a run nobody is watching. */
const DEFAULT_AGENT_ARGS: Record<string, string[]> = {
  claude: ["--dangerously-skip-permissions"],
};

/**
 * Agent role for a project. Priority: [projects.agents.*] → project.agent → [agents.*] → codex.
 * Review defaults to the same agent kind as development.
 */
export function resolveAgentRole(
  role: "plan" | "dev" | "review",
  config: Config,
  project: ProjectConfig | undefined,
): AgentRole {
  const global = config.agents[role];
  const local = project?.agents?.[role];
  const devKind = project?.agents?.dev?.kind ?? project?.agent ?? config.agents.dev.kind ?? "codex";
  const kind = local?.kind ?? (role === "dev" ? devKind : (global.kind ?? devKind));
  return {
    kind,
    prompt: local?.prompt ?? global.prompt,
    skill: local?.skill ?? global.skill,
    args: local?.args ?? global.args ?? DEFAULT_AGENT_ARGS[kind] ?? [],
  };
}

/** A command or text before the task: "/code-review" plus the body in one prompt. */
export function withPrefix(prefix: string, body: string): string {
  return prefix.trim() ? `${prefix.trim()} ${body}` : body;
}

export function reviewAgentName(devAgent: string): string {
  return `${devAgent.slice(0, 28)}-rev`;
}

export function reviewPrompt(
  prefix: string,
  task: Task,
  ctx: { changeUrl: string; branch: string },
): string {
  return withPrefix(
    prefix,
    [
      ctx.changeUrl,
      "",
      `Task ${task.id}: ${task.title}`,
      `Branch ${ctx.branch}. Leave findings as pull request comments.`,
    ].join("\n"),
  );
}

export function buildPrompt(
  task: Task,
  ctx: {
    branch: string;
    validate?: string | undefined;
    prefix?: string | undefined;
    skill?: string | undefined;
  },
): string {
  return withSkill(
    ctx.skill ?? "",
    withPrefix(
      ctx.prefix ?? "",
      [
        `Task ${task.id}: ${task.title}`,
        task.description ? `\n${task.description}` : "",
        `\nYou are working in a dedicated git worktree on branch ${ctx.branch}.`,
        `Implement the task and commit your changes to that branch.`,
        ctx.validate ? `Before finishing, make sure \`${ctx.validate}\` passes.` : "",
        `Do not push the branch or open a pull request — the orchestrator does that.`,
        `If the requirements are ambiguous, stop and ask instead of guessing.`,
      ]
        .filter(Boolean)
        .join("\n"),
    ),
  );
}

/**
 * The planning pass. The plan reaches the tracker because the agent puts it there — the workflow
 * observes an idle agent, it never reads what the agent wrote.
 */
export function buildPlanPrompt(
  task: Task,
  ctx: { branch: string; prefix?: string | undefined; skill?: string | undefined },
): string {
  return withSkill(
    ctx.skill ?? "",
    withPrefix(
      ctx.prefix ?? "",
      [
        `Task ${task.id}: ${task.title}`,
        task.description ? `\n${task.description}` : "",
        `\nYou are in a git worktree on branch ${ctx.branch}, planning this task.`,
        `Read the code you would touch, then write an implementation plan.`,
        `Publish it with \`shepherd task comment ${task.id} "<plan>"\` and stop there.`,
        `This pass is planning only — do not write any code and do not commit.`,
      ]
        .filter(Boolean)
        .join("\n"),
    ),
  );
}

/** "Use the X skill." in front of everything else; an unset skill leaves the prompt alone. */
export function withSkill(skill: string, body: string): string {
  return skill.trim() ? `Use the ${skill.trim()} skill. ${body}` : body;
}

export function validationFeedback(command: string, output: string): string {
  return [
    `Validation failed: \`${command}\``,
    "```",
    output.slice(-4000),
    "```",
    "Fix it and commit the fix to the same branch.",
  ].join("\n");
}

export function noCommitsFeedback(branch: string): string {
  return `Branch ${branch} has no commits. Commit your work, otherwise the task cannot go to review.`;
}

export function commentsSince(comments: ChangeComment[], since: Date | undefined): ChangeComment[] {
  return since ? comments.filter((c) => c.createdAt > since) : comments;
}

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

export function changeBody(task: Task, run: AgentRun): string {
  return [
    task.url ? `Task: ${task.url}` : `Task: ${task.id}`,
    "",
    task.description?.slice(0, 2000) ?? "",
    "",
    `---`,
    `Generated by shepherd: run \`${run.id}\`, agent \`${run.agentKind}\`, workspace \`${run.herdrWorkspaceId}\`.`,
  ].join("\n");
}
