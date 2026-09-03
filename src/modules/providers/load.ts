import { existsSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";
import { configPath, userConfigPath } from "../../core/config/schema.ts";
import { briefError } from "../../shared/log.ts";
import type { CodeProvider, TaskProvider } from "../../shared/domain/types.ts";

/** A factory receives its whole config section: url, project and any other settings live there. */
export type ProviderFactory<T> = (settings: Record<string, unknown>) => T;

export interface CustomProviders {
  tasks: Record<string, ProviderFactory<TaskProvider>>;
  code: Record<string, ProviderFactory<CodeProvider>>;
  loaded: string[];
  errors: string[];
}

/**
 * Where plugins are looked for: next to the active config and in the user config folder.
 * A project-local shepherd.yaml therefore still sees ~/.config/shepherd/providers.
 */
export function providersDirs(): string[] {
  if (process.env.SHEPHERD_PROVIDERS) return [process.env.SHEPHERD_PROVIDERS];
  const dirs = [
    join(dirname(configPath()), "providers"),
    join(dirname(userConfigPath()), "providers"),
  ];
  return [...new Set(dirs)];
}

export const providersDir = () => providersDirs()[0]!;

/**
 * Providers from the folder next to the config. A file exports `taskProvider` or `codeProvider`
 * and is registered under its own file name, or exports a `taskProviders`/`codeProviders` map
 * when one file carries several.
 * The core knows nothing about them; adding Jira or GitLab needs no orchestrator changes.
 */
export async function loadCustomProviders(dirs = providersDirs()): Promise<CustomProviders> {
  const result: CustomProviders = { tasks: {}, code: {}, loaded: [], errors: [] };
  for (const dir of typeof dirs === "string" ? [dirs] : dirs) {
    if (!existsSync(dir)) continue;
    for (const file of readdirSync(dir).sort()) {
      if (!/\.(ts|mts|js|mjs)$/.test(file)) continue;
      try {
        const mod = await import(pathToFileURL(join(dir, file)).href);
        // A file that exports one factory registers under its own name: jira.ts -> "jira".
        const name = file.replace(/\.(ts|mts|js|mjs)$/, "");
        if (mod.taskProvider) result.tasks[name] = mod.taskProvider;
        if (mod.codeProvider) result.code[name] = mod.codeProvider;
        Object.assign(result.tasks, mod.taskProviders ?? {});
        Object.assign(result.code, mod.codeProviders ?? {});
        result.loaded.push(file);
      } catch (err: any) {
        // a broken plugin must not take the whole CLI down, doctor reports it
        result.errors.push(`${file}: ${briefError(err)}`);
      }
    }
  }
  return result;
}
