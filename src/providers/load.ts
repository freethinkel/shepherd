import { existsSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";
import { configPath } from "../config/schema.ts";
import type { CodeProvider, TaskProvider } from "../domain/types.ts";

/** A factory receives its whole config section: url, project and any other settings live there. */
export type ProviderFactory<T> = (settings: Record<string, unknown>) => T;

export interface CustomProviders {
  tasks: Record<string, ProviderFactory<TaskProvider>>;
  code: Record<string, ProviderFactory<CodeProvider>>;
  loaded: string[];
  errors: string[];
}

export const providersDir = () => process.env.SHEPHERD_PROVIDERS ?? join(dirname(configPath()), "providers");

/**
 * Providers from the folder next to the config: every file exports
 * `taskProviders` and/or `codeProviders` — a "name -> factory" map.
 * The core knows nothing about them; adding Jira or GitLab needs no orchestrator changes.
 */
export async function loadCustomProviders(dir = providersDir()): Promise<CustomProviders> {
  const result: CustomProviders = { tasks: {}, code: {}, loaded: [], errors: [] };
  if (!existsSync(dir)) return result;
  for (const file of readdirSync(dir).sort()) {
    if (!/\.(ts|mts|js|mjs)$/.test(file)) continue;
    try {
      const mod = await import(pathToFileURL(join(dir, file)).href);
      Object.assign(result.tasks, mod.taskProviders ?? {});
      Object.assign(result.code, mod.codeProviders ?? {});
      result.loaded.push(file);
    } catch (err: any) {
      // a broken plugin must not take the whole CLI down — doctor reports it
      result.errors.push(`${file}: ${err.message ?? err}`);
    }
  }
  return result;
}
