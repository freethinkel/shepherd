import { join } from "node:path";
import { expandPath } from "../config/schema.ts";
import type { Config, ProjectConfig } from "../config/schema.ts";
import type { AgentRun, ChangeComment, Project, Task } from "../domain/types.ts";

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

export interface AgentRole {
  kind: string;
  prompt: string;
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
  role: "dev" | "review",
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
  ctx: { branch: string; validate?: string | undefined; prefix?: string | undefined },
): string {
  return withPrefix(
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
  );
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
