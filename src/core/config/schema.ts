import { existsSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { isAbsolute, join, resolve } from "node:path";
import { parse as parseYaml } from "yaml";
import { z } from "zod";

export const expandPath = (p: string) =>
  isAbsolute(p) ? p : resolve(p.startsWith("~") ? homedir() + p.slice(1) : p);

const AgentRoleSchema = z.object({
  /** Agent kind for Herdr; defaults to the project's `agent`. */
  kind: z.string().optional(),
  prompt: z.string().default(""),
  /** Skill the agent is told to use first; empty means none is named. */
  skill: z.string().default(""),
  /** Command-line arguments for the agent itself; unset means the kind's defaults. */
  args: z.array(z.string()).optional(),
});

/** Per-project role override: an unset field is inherited from the global section. */
const AgentRoleOverrideSchema = z.object({
  kind: z.string().optional(),
  prompt: z.string().optional(),
  skill: z.string().optional(),
  args: z.array(z.string()).optional(),
});

const ProjectSchema = z.object({
  name: z.string(),
  repository: z.string(),
  task_project: z.string().optional(),
  /** Shorthand for the dev agent kind (codex, claude, ...). */
  agent: z.string().optional(),
  /** Full role override: [projects.agents.dev] / [projects.agents.review]. */
  agents: z
    .object({
      plan: AgentRoleOverrideSchema.optional(),
      dev: AgentRoleOverrideSchema.optional(),
      review: AgentRoleOverrideSchema.optional(),
    })
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
  /** Settings shared by every task provider. */
  task_provider: z
    .looseObject({
      /** Whose tasks to take: "me" (assigned to the key owner), an email, or "any". */
      assignee: z.string().default("me"),
    })
    .prefault({}),
  /** Settings shared by every code provider. */
  code_provider: z.looseObject({ base_branch: z.string().default("main") }).prefault({}),
  /**
   * Per-provider settings, keyed by provider name. A plugin is named after its file,
   * so ~/.config/shepherd/providers/jira.ts reads [task_providers.jira].
   */
  task_providers: z.record(z.string(), z.looseObject({})).prefault({}),
  code_providers: z.record(z.string(), z.looseObject({})).prefault({}),
  herdr: z.object({ socket: z.string().default("auto") }).prefault({}),
  /**
   * Agent roles. `prompt` is what goes before the task text:
   * plain text or the agent's own slash command ("/code-review", "/brainstorm").
   */
  agents: z
    .object({
      /** Empty prompt and skill — no planning pass, the dev agent starts straight away. */
      plan: AgentRoleSchema.prefault({}),
      dev: AgentRoleSchema.prefault({}),
      /** Empty prompt — no review agent is started, the run just waits for a human. */
      review: AgentRoleSchema.prefault({}),
    })
    .prefault({}),
  orchestrator: z
    .object({
      max_concurrent_runs: z.number().int().positive().default(3),
      /** How many failed runs a task may collect before it stops being retried. */
      max_attempts: z.number().int().positive().default(3),
      /** How many times a failed validation is handed back to the agent before giving up. */
      max_validation_rounds: z.number().int().positive().default(3),
      max_review_rounds: z.number().int().positive().default(3),
      auto_merge: z.boolean().default(true),
      /**
       * How long an agent may stay idle right after a prompt before that idle counts.
       * Without it the poll can overtake the agent: `agent start` leaves it idle, the
       * prompt is still unread, and two quiet polls look exactly like finished work.
       */
      agent_settle_ms: z.number().int().nonnegative().default(45_000),
      /** Deadline for a run that is still working. Waiting for a human in review does not count. */
      run_timeout_ms: z
        .number()
        .int()
        .positive()
        .default(4 * 60 * 60_000),
      /** How often Herdr is polled for agent state. */
      poll_interval_ms: z.number().int().positive().default(5_000),
      /** How often tasks are pulled from the tracker. */
      task_sync_interval_ms: z.number().int().positive().default(60_000),
      /** How often an open pull request is re-checked. Slower than the agent poll on purpose. */
      change_poll_interval_ms: z.number().int().positive().default(60_000),
      /** Where worktrees land. Unset means herdr decides (~/.herdr/worktrees/<repo>/<branch>). */
      worktrees: z.string().optional(),
    })
    .prefault({}),
  projects: z.array(ProjectSchema).default([]),
});

export type Config = z.output<typeof ConfigSchema>;
export type ProjectConfig = z.output<typeof ProjectSchema>;

/** ~/.config/shepherd/config.yaml unless a project-local shepherd.yaml sits next to us. */
export function userConfigPath(): string {
  const base = process.env.XDG_CONFIG_HOME ?? join(homedir(), ".config");
  return join(base, "shepherd", "config.yaml");
}

export function configPath(): string {
  if (process.env.SHEPHERD_CONFIG) return expandPath(process.env.SHEPHERD_CONFIG);
  const local = resolve("shepherd.yaml");
  return existsSync(local) ? local : userConfigPath();
}

export function loadConfig(path = configPath()): Config {
  const config = ConfigSchema.parse(parseYaml(readFileSync(path, "utf8")));
  for (const [key, value] of Object.entries(config.env)) process.env[key] ??= value;
  return config;
}

export const EXAMPLE_CONFIG = `# Secrets: the file is created with mode 600. Environment variables override these values.
env:
  LINEAR_API_KEY: ""
  # GITHUB_TOKEN is not needed, gh CLI auth is used

task_provider:
  # whose tasks agents pick up: "me" | email | "any" (including unassigned)
  assignee: me

code_provider:
  base_branch: main

herdr:
  socket: auto

orchestrator:
  max_concurrent_runs: 3
  # max_attempts: 3                  # failed runs per task before it needs a human
  # agent_settle_ms: 45000           # grace period after a prompt before idle means "done"
  # max_validation_rounds: 3         # failed validations handed back to the agent
  # max_review_rounds: 3             # review comments handed back to the agent before giving up
  # auto_merge: true                 # merge when a human approved and checks are green
  # run_timeout_ms: 14400000         # 4h deadline for a run that never finishes
  # all intervals are optional, defaults are shown
  # poll_interval_ms: 5000           # how often Herdr is asked about agent state
  # task_sync_interval_ms: 60000     # how often tasks are pulled from the tracker
  # change_poll_interval_ms: 60000   # how often an open pull request is re-checked

agents:
  # plan:
  #   skill: superpowers:writing-plans  # unset prompt and skill: no planning pass at all
  # dev:
  #   prompt: /brainstorm      # what the dev agent prompt starts with
  #   skill: ""                # "Use the <skill> skill." leads the prompt
  #   args: []                 # agent's own flags; unset means the kind's defaults
  review:
    kind: claude
    prompt: /code-review       # empty value starts no review agent

projects:
  - name: Phocus
    repository: ~/Projects/phocus
    task_project: Phocus
    agent: codex
    # validate: pnpm test
    # per-project role override, unset fields are inherited from agents.*
    # agents:
    #   review:
    #     prompt: /code-review --strict

# settings for a plugin, keyed by its file name in ~/.config/shepherd/providers/
# task_providers:
#   jira:
#     url: https://company.atlassian.net
#     email: you@company.com
`;
