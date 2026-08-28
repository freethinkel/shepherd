import type { Config, ProjectConfig } from "../config/schema.ts";
import type { Project } from "../domain/types.ts";
import { briefError } from "../log.ts";
import type { ProviderRegistry } from "../providers/registry.ts";
import * as db from "../persistence/db.ts";
import { resolveRepository } from "../repositories/git.ts";
import { canStart, isTaskAvailable, projectId } from "./policies.ts";
import type { Workflow } from "./workflow.ts";

export interface SchedulerDeps {
  db: db.Db;
  config: Config;
  registry: ProviderRegistry;
  workflow: Workflow;
  projectConfigs: Map<string, ProjectConfig>;
  log?: (msg: string) => void;
}

export class Scheduler {
  private lastTaskSync = 0;
  /** Credential checks shell out, so their answer is kept for a while. */
  private readonly codeChecks = new Map<string, { at: number; reason?: string | undefined }>();

  constructor(private readonly deps: SchedulerDeps) {}

  private log(msg: string) {
    this.deps.log?.(msg);
  }

  /** Projects and repositories from the config into SQLite. Idempotent. */
  async syncProjects(): Promise<void> {
    for (const cfg of this.deps.config.projects) {
      try {
        const repo = await resolveRepository(cfg.repository);
        db.upsertRepository(this.deps.db, repo);
        db.upsertProject(this.deps.db, {
          id: projectId(cfg.name),
          name: cfg.name,
          repositoryId: repo.id,
          taskProviderProjectId: cfg.task_project,
        });
      } catch (err: any) {
        // a broken project in the config must not blind the others
        this.log(`project ${cfg.name}: ${briefError(err)}`);
      }
    }
  }

  /**
   * Every registered tracker is asked about every project. A tracker that has no project
   * with this name returns nothing, so the project name alone decides where tasks come from.
   * The answer records its provider, because status updates must go back to the same tracker.
   */
  async syncTasks(): Promise<void> {
    const providers = this.deps.registry.allTaskProviders();
    for (const project of db.listProjects(this.deps.db)) {
      for (const { name, provider } of providers) {
        try {
          const tasks = await provider.listTasks({
            projectId: project.id,
            taskProviderProjectId: project.taskProviderProjectId,
            assignee: this.deps.config.task_provider.assignee as string,
          });
          for (const task of tasks) {
            db.upsertTask(this.deps.db, { ...task, projectId: project.id, provider: name });
          }
        } catch (err: any) {
          this.log(`sync ${project.name} via ${name}: ${briefError(err)}`);
        }
      }
    }
    this.lastTaskSync = Date.now();
  }

  async tick(): Promise<void> {
    if (Date.now() - this.lastTaskSync > this.deps.config.orchestrator.task_sync_interval_ms) {
      await this.syncTasks();
    }
    for (const run of db.activeRuns(this.deps.db)) {
      await this.deps.workflow.advance(run);
    }
    await this.dispatch();
  }

  /**
   * Whether the project can produce a change at all. An agent that works for an hour
   * and then cannot open a merge request has wasted the hour, so this is checked first.
   */
  private async codeBlocked(project: Project): Promise<string | undefined> {
    const cached = this.codeChecks.get(project.id);
    if (cached && Date.now() - cached.at < 60_000) return cached.reason;
    let reason: string | undefined;
    try {
      const repo = db.getRepository(this.deps.db, project.repositoryId);
      await this.deps.registry.codeForRemote(repo?.remote).check?.(repo?.path);
    } catch (err: any) {
      reason = briefError(err);
    }
    this.codeChecks.set(project.id, { at: Date.now(), reason });
    return reason;
  }

  /** Start new runs within max_concurrent_runs. */
  async dispatch(): Promise<void> {
    const max = this.deps.config.orchestrator.max_concurrent_runs;
    for (const { project, task } of this.availableWork()) {
      if (!canStart(db.activeRuns(this.deps.db).length, max)) return;
      const blocked = await this.codeBlocked(project);
      if (blocked) {
        this.log(`${project.name}: ${task.id} not started, ${blocked}`);
        continue;
      }
      this.log(`starting ${task.id} (${project.name})`);
      await this.deps.workflow.start(project, task);
    }
  }

  /** Tasks ready to start: one per project per pass, so a single project cannot hog the queue. */
  availableWork(): { project: Project; task: import("../domain/types.ts").Task }[] {
    const out = [];
    for (const project of db.listProjects(this.deps.db)) {
      if (!this.deps.projectConfigs.has(project.id)) continue;
      for (const task of db.listTasks(this.deps.db, project.id)) {
        // runs from before a `shepherd reset` do not count against max_attempts
        const resetAt = db.lastEventAt(this.deps.db, task.id, "TaskReset");
        const runs = db
          .runsForTask(this.deps.db, task.id)
          .filter((r) => !resetAt || r.startedAt > resetAt);
        if (isTaskAvailable(task, runs, this.deps.config.orchestrator.max_attempts)) {
          out.push({ project, task });
          break;
        }
      }
    }
    return out;
  }

  async loop(signal?: AbortSignal): Promise<void> {
    await this.syncProjects();
    await this.syncTasks();
    while (!signal?.aborted) {
      await this.tick().catch((err) => this.log(`tick: ${briefError(err)}`));
      await new Promise((r) => setTimeout(r, this.deps.config.orchestrator.poll_interval_ms));
    }
  }
}
