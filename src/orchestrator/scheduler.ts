import type { Config, ProjectConfig } from "../config/schema.ts";
import type { Project, TaskProvider } from "../domain/types.ts";
import * as db from "../persistence/db.ts";
import { resolveRepository } from "../repositories/git.ts";
import { canStart, isTaskAvailable, projectId } from "./policies.ts";
import type { Workflow } from "./workflow.ts";

export interface SchedulerDeps {
  db: db.Db;
  config: Config;
  tasks: TaskProvider;
  workflow: Workflow;
  projectConfigs: Map<string, ProjectConfig>;
  log?: (msg: string) => void;
}

export class Scheduler {
  private lastTaskSync = 0;

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
        this.log(`project ${cfg.name}: ${err.message}`);
      }
    }
  }

  /** The tracker owns tasks; locally we keep only a mirror. */
  async syncTasks(): Promise<void> {
    for (const project of db.listProjects(this.deps.db)) {
      try {
        const tasks = await this.deps.tasks.listTasks({
          projectId: project.id,
          taskProviderProjectId: project.taskProviderProjectId,
          assignee: this.deps.config.task_provider.assignee,
        });
        for (const task of tasks) db.upsertTask(this.deps.db, { ...task, projectId: project.id });
      } catch (err: any) {
        this.log(`sync ${project.name}: ${err.message}`);
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

  /** Start new runs within max_concurrent_runs. */
  async dispatch(): Promise<void> {
    const max = this.deps.config.orchestrator.max_concurrent_runs;
    for (const { project, task } of this.availableWork()) {
      if (!canStart(db.activeRuns(this.deps.db).length, max)) return;
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
        if (isTaskAvailable(task, db.runsForTask(this.deps.db, task.id))) {
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
      await this.tick().catch((err) => this.log(`tick: ${err.message}`));
      await new Promise((r) => setTimeout(r, this.deps.config.orchestrator.poll_interval_ms));
    }
  }
}
