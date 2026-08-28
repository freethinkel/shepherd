import { loadConfig, type Config, type ProjectConfig } from "./config/schema.ts";
import { HerdrClient } from "./herdr/client.ts";
import { Scheduler } from "./orchestrator/scheduler.ts";
import { Workflow } from "./orchestrator/workflow.ts";
import { projectId } from "./orchestrator/policies.ts";
import { loadCustomProviders, type CustomProviders } from "./providers/load.ts";
import { ProviderRegistry } from "./providers/registry.ts";
import * as db from "./persistence/db.ts";

export interface App {
  config: Config;
  db: db.Db;
  herdr: HerdrClient;
  registry: ProviderRegistry;
  workflow: Workflow;
  scheduler: Scheduler;
  projectConfigs: Map<string, ProjectConfig>;
  providers: CustomProviders;
}

export async function createApp(log: (msg: string) => void = () => {}): Promise<App> {
  const config = loadConfig();
  const custom = await loadCustomProviders();
  for (const error of custom.errors) log(`provider ${error}`);
  const database = db.openDb();
  const projectConfigs = new Map(config.projects.map((p) => [projectId(p.name), p]));
  const registry = new ProviderRegistry(config, custom);
  const herdr = new HerdrClient();
  const workflow = new Workflow({ db: database, herdr, registry, config, projectConfigs, log });
  const scheduler = new Scheduler({
    db: database,
    config,
    registry,
    workflow,
    projectConfigs,
    log,
  });
  return {
    config,
    db: database,
    herdr,
    registry,
    workflow,
    scheduler,
    projectConfigs,
    providers: custom,
  };
}
