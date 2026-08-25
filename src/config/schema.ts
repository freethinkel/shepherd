import { existsSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { isAbsolute, join, resolve } from "node:path";
import { parse as parseToml } from "smol-toml";
import { z } from "zod";

export const expandPath = (p: string) =>
  isAbsolute(p) ? p : resolve(p.startsWith("~") ? homedir() + p.slice(1) : p);

const AgentRoleSchema = z.object({
  /** Agent kind for Herdr; defaults to the project's `agent`. */
  kind: z.string().optional(),
  prompt: z.string().default(""),
});

/** Per-project role override: an unset field is inherited from the global section. */
const AgentRoleOverrideSchema = z.object({
  kind: z.string().optional(),
  prompt: z.string().optional(),
});

const ProjectSchema = z.object({
  name: z.string(),
  repository: z.string(),
  task_project: z.string().optional(),
  /** Shorthand for the dev agent kind (codex, claude, ...). */
  agent: z.string().optional(),
  /** Full role override: [projects.agents.dev] / [projects.agents.review]. */
  agents: z
    .object({ dev: AgentRoleOverrideSchema.optional(), review: AgentRoleOverrideSchema.optional() })
    .optional(),
  /** Validation command, run inside the worktree. Empty — validation is skipped. */
  validate: z.string().optional(),
  base_branch: z.string().optional(),
});

export const ConfigSchema = z.object({
  /**
   * Provider secrets. The file lives in ~/.config with mode 600, not in the repository.
   * Environment variables win — the config only fills in what is missing.
   */
  env: z.record(z.string(), z.string()).prefault({}),
  task_provider: z
    .looseObject({
      type: z.string().default("linear"),
      team: z.string().optional(),
      /** Whose tasks to take: "me" (assigned to the key owner), an email, or "any". */
      assignee: z.string().default("me"),
    })
    .prefault({}),
  code_provider: z
    .looseObject({ type: z.string().default("github"), base_branch: z.string().default("main") })
    .prefault({}),
  herdr: z.object({ socket: z.string().default("auto") }).prefault({}),
  /**
   * Agent roles. `prompt` is what goes before the task text:
   * plain text or the agent's own slash command ("/code-review", "/brainstorm").
   */
  agents: z
    .object({
      dev: AgentRoleSchema.prefault({}),
      /** Empty prompt — no review agent is started, the run just waits for a human. */
      review: AgentRoleSchema.prefault({}),
    })
    .prefault({}),
  orchestrator: z
    .object({
      max_concurrent_runs: z.number().int().positive().default(3),
      /** How often Herdr is polled for agent state. */
      poll_interval_ms: z.number().int().positive().default(5_000),
      /** How often tasks are pulled from the tracker. */
      task_sync_interval_ms: z.number().int().positive().default(60_000),
      worktrees: z.string().default("~/.shepherd/worktrees"),
    })
    .prefault({}),
  projects: z.array(ProjectSchema).default([]),
});

export type Config = z.output<typeof ConfigSchema>;
export type ProjectConfig = z.output<typeof ProjectSchema>;

/** ~/.config/shepherd/config.toml unless a project-local shepherd.toml sits next to us. */
export function userConfigPath(): string {
  const base = process.env.XDG_CONFIG_HOME ?? join(homedir(), ".config");
  return join(base, "shepherd", "config.toml");
}

export function configPath(): string {
  if (process.env.SHEPHERD_CONFIG) return expandPath(process.env.SHEPHERD_CONFIG);
  const local = resolve("shepherd.toml");
  return existsSync(local) ? local : userConfigPath();
}

export function loadConfig(path = configPath()): Config {
  const config = ConfigSchema.parse(parseToml(readFileSync(path, "utf8")));
  for (const [key, value] of Object.entries(config.env)) process.env[key] ??= value;
  return config;
}

export const EXAMPLE_CONFIG = `# Secrets: the file is created with mode 600. Environment variables override these values.
[env]
LINEAR_API_KEY = ""
# GITHUB_TOKEN is not needed — gh CLI auth is used

[task_provider]
type = "linear"
# whose tasks agents pick up: "me" | email | "any" (including unassigned)
assignee = "me"

[code_provider]
type = "github"
base_branch = "main"

[herdr]
socket = "auto"

[orchestrator]
max_concurrent_runs = 3

[agents.dev]
# prompt = "/brainstorm"     # what the dev agent prompt starts with

[agents.review]
kind = "claude"
prompt = "/code-review"      # empty — no review agent is started

[[projects]]
name = "Phocus"
repository = "~/Projects/phocus"
task_project = "Phocus"
agent = "codex"
# validate = "pnpm test"

# per-project role override — unset fields are inherited from [agents.*]
# [projects.agents.review]
# prompt = "/code-review --strict"
`;
