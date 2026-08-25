import { loadConfig, type Config, type ProjectConfig } from "./config/schema.ts";
import type { CodeProvider, TaskProvider } from "./domain/types.ts";
import { HerdrClient } from "./herdr/client.ts";
import { Scheduler } from "./orchestrator/scheduler.ts";
import { Workflow } from "./orchestrator/workflow.ts";
import { projectId } from "./orchestrator/policies.ts";
import { GitHubCodeProvider } from "./providers/code/github.ts";
import { LinearTaskProvider } from "./providers/tasks/linear.ts";
import { loadCustomProviders, providersDir, type CustomProviders, type ProviderFactory } from "./providers/load.ts";
import * as db from "./persistence/db.ts";

export interface App {
  config: Config;
  db: db.Db;
  herdr: HerdrClient;
  tasks: TaskProvider;
  code: CodeProvider;
  workflow: Workflow;
  scheduler: Scheduler;
  projectConfigs: Map<string, ProjectConfig>;
  providers: CustomProviders;
}

const builtinTaskProviders: Record<string, ProviderFactory<TaskProvider>> = {
  linear: () => new LinearTaskProvider(),
};
const builtinCodeProviders: Record<string, ProviderFactory<CodeProvider>> = {
  github: () => new GitHubCodeProvider(),
};

function pick<T>(
  kind: string,
  type: string,
  registry: Record<string, ProviderFactory<T>>,
  settings: Record<string, unknown>,
): T {
  const factory = registry[type];
  if (!factory) {
    throw new Error(
      `unknown ${kind} provider "${type}"; available: ${Object.keys(registry).join(", ")}` +
        ` (custom ones live in ${providersDir()})`,
    );
  }
  return factory(settings);
}

export async function createApp(log: (msg: string) => void = () => {}): Promise<App> {
  const config = loadConfig();
  const custom = await loadCustomProviders();
  for (const error of custom.errors) log(`provider ${error}`);
  const database = db.openDb();
  const projectConfigs = new Map(config.projects.map((p) => [projectId(p.name), p]));
  const tasks = pick("task", config.task_provider.type, { ...builtinTaskProviders, ...custom.tasks }, config.task_provider);
  const code = pick("code", config.code_provider.type, { ...builtinCodeProviders, ...custom.code }, config.code_provider);
  const herdr = new HerdrClient();
  const workflow = new Workflow({ db: database, herdr, tasks, code, config, projectConfigs, log });
  const scheduler = new Scheduler({ db: database, config, tasks, workflow, projectConfigs, log });
  return { config, db: database, herdr, tasks, code, workflow, scheduler, projectConfigs, providers: custom };
}
