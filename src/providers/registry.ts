import type { Config } from "../config/schema.ts";
import type { CodeProvider, TaskProvider } from "../domain/types.ts";
import { codeProviderForRemote, providerSettings } from "../orchestrator/policies.ts";
import { GitHubCodeProvider } from "./code/github.ts";
import { GitLabCodeProvider } from "./code/gitlab.ts";
import { LinearTaskProvider } from "./tasks/linear.ts";
import { providersDirs, type CustomProviders, type ProviderFactory } from "./load.ts";

const builtinTasks: Record<string, ProviderFactory<TaskProvider>> = {
  linear: () => new LinearTaskProvider(),
};
const builtinCode: Record<string, ProviderFactory<CodeProvider>> = {
  github: () => new GitHubCodeProvider(),
  gitlab: (settings) => new GitLabCodeProvider(settings),
};

/**
 * Every registered provider, built in or dropped into the providers folder as a file.
 * Task providers are all asked for tasks; a code provider is chosen by the repository's remote.
 */
export class ProviderRegistry {
  private readonly instances = new Map<string, TaskProvider | CodeProvider>();
  private readonly taskFactories: Record<string, ProviderFactory<TaskProvider>>;
  private readonly codeFactories: Record<string, ProviderFactory<CodeProvider>>;

  constructor(
    private readonly config: Config,
    custom: CustomProviders,
  ) {
    this.taskFactories = { ...builtinTasks, ...custom.tasks };
    this.codeFactories = { ...builtinCode, ...custom.code };
  }

  taskProviderNames(): string[] {
    return Object.keys(this.taskFactories);
  }

  /**
   * Trackers worth asking: the built-in ones, plus any plugin that has a
   * [task_providers.<name>] section. A plugin without settings has no host to talk to,
   * so asking it would only fill the log with errors.
   */
  allTaskProviders(): { name: string; provider: TaskProvider }[] {
    return this.taskProviderNames()
      .filter((name) => name in builtinTasks || name in this.config.task_providers)
      .map((name) => ({ name, provider: this.tasks(name) }));
  }

  tasks(name: string): TaskProvider {
    return this.build("task_provider", this.taskFactories, name) as TaskProvider;
  }

  code(name: string): CodeProvider {
    return this.build("code_provider", this.codeFactories, name) as CodeProvider;
  }

  /** The remote decides: github.com means GitHub, a gitlab host means GitLab. */
  codeForRemote(remote: string | undefined): CodeProvider {
    const hosts = Object.fromEntries(
      Object.entries(this.config.code_providers).map(([name, settings]) => [
        name,
        (settings.hosts as string[] | undefined) ?? [],
      ]),
    );
    const byRemote = remote ? codeProviderForRemote(remote, hosts) : undefined;
    const names = Object.keys(this.codeFactories);
    const name = byRemote ?? (names.length === 1 ? names[0]! : undefined);
    if (!name) {
      throw new Error(
        `cannot tell which code provider fits remote "${remote ?? "none"}"; registered: ${names.join(", ")}`,
      );
    }
    return this.code(name);
  }

  private build<T>(
    role: "task_provider" | "code_provider",
    factories: Record<string, ProviderFactory<T>>,
    name: string,
  ): T {
    const cached = this.instances.get(`${role}:${name}`);
    if (cached) return cached as T;
    const factory = factories[name];
    if (!factory) {
      throw new Error(
        `unknown ${role} "${name}"; registered: ${Object.keys(factories).join(", ")}` +
          ` (plugins live in ${providersDirs().join(", ")})`,
      );
    }
    const provider = factory(providerSettings(role, this.config, name));
    this.instances.set(`${role}:${name}`, provider as TaskProvider | CodeProvider);
    return provider;
  }
}
