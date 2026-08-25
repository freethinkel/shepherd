import { randomUUID } from "node:crypto";
import type { Config, ProjectConfig } from "../config/schema.ts";
import { taskStatusForRun } from "../domain/status.ts";
import type { AgentRun, CodeProvider, Project, RunStatus, Task, TaskProvider } from "../domain/types.ts";
import type { HerdrClient } from "../herdr/client.ts";
import * as db from "../persistence/db.ts";
import * as git from "../repositories/git.ts";
import * as policy from "./policies.ts";

export interface WorkflowDeps {
  db: db.Db;
  herdr: HerdrClient;
  tasks: TaskProvider;
  code: CodeProvider;
  config: Config;
  projectConfigs: Map<string, ProjectConfig>;
  log?: (msg: string) => void;
}

/** One workflow step = one run state transition, recorded in SQLite. */
export class Workflow {
  private readonly busy = new Set<string>();
  private readonly idleTicks = new Map<string, number>();

  constructor(private readonly deps: WorkflowDeps) {}

  private log(msg: string) {
    this.deps.log?.(msg);
  }

  private event(type: string, run: AgentRun, data?: unknown) {
    db.appendEvent(this.deps.db, type, {
      runId: run.id, taskId: run.taskId, projectId: run.projectId, data,
    });
  }

  private transition(run: AgentRun, status: RunStatus, patch: Parameters<typeof db.updateRun>[2] = {}) {
    db.updateRun(this.deps.db, run.id, { status, ...patch });
    run.status = status;
    this.event("RunStatusChanged", run, { status });
    this.syncTaskStatus(run, status);
  }

  private syncTaskStatus(run: AgentRun, status: RunStatus) {
    const taskStatus = taskStatusForRun(status);
    db.setTaskStatus(this.deps.db, run.taskId, taskStatus);
    this.event("TaskStatusChanged", run, { status: taskStatus });
    this.deps.tasks
      .updateStatus(run.taskId, taskStatus)
      .catch((err) => this.log(`task ${run.taskId}: ${err.message}`));
  }

  /** Create a run and bring an agent up in Herdr. */
  async start(project: Project, task: Task): Promise<AgentRun> {
    const cfg = this.projectConfig(project);
    const repo = db.getRepository(this.deps.db, project.repositoryId);
    if (!repo) throw new Error(`repository for project ${project.name} is not registered`);

    const dev = policy.resolveAgentRole("dev", this.deps.config, cfg);
    const branch = git.branchName(task.id, task.title);
    const worktreeName = `${policy.slug(project.name)}-${policy.slug(task.id)}`;
    const worktree = policy.worktreePath(this.deps.config.orchestrator.worktrees, project, task);
    const run: AgentRun = {
      id: `run_${randomUUID().replace(/-/g, "").slice(0, 10)}`,
      projectId: project.id,
      taskId: task.id,
      herdrWorkspaceId: "",
      herdrAgentId: policy.agentName(project, task),
      agentKind: dev.kind,
      branch,
      worktreePath: worktree,
      status: "queued",
      startedAt: new Date(),
    };
    // the unique index in SQLite prevents claiming the same task twice
    db.insertRun(this.deps.db, {
      ...run,
      attempt: db.runsForTask(this.deps.db, task.id).length + 1,
    });
    this.event("RunCreated", run, { branch });

    try {
      await this.deps.tasks.claimTask(task.id).catch((err) => this.log(`claim ${task.id}: ${err.message}`));
      this.transition(run, "starting");

      await git.createWorktree({
        repoRoot: repo.path,
        branch,
        base: cfg.base_branch ?? this.deps.config.code_provider.base_branch ?? repo.defaultBranch,
        worktreesDir: this.deps.config.orchestrator.worktrees,
        name: worktreeName,
      });

      const ws = await this.deps.herdr.createWorkspace({
        label: policy.workspaceLabel(project, task),
        cwd: worktree,
      });
      db.recordWorkspace(this.deps.db, { id: ws.workspaceId, runId: run.id, label: ws.label, cwd: worktree });
      db.updateRun(this.deps.db, run.id, { herdrWorkspaceId: ws.workspaceId });
      run.herdrWorkspaceId = ws.workspaceId;

      await this.deps.herdr.spawnAgent({ name: run.herdrAgentId, kind: dev.kind, paneId: ws.paneId });
      this.event("AgentStarted", run, { agent: run.herdrAgentId, workspace: ws.workspaceId });

      await this.deps.herdr.prompt(
        run.herdrAgentId,
        policy.buildPrompt(task, {
          branch,
          validate: cfg.validate,
          prefix: dev.prompt,
        }),
      );
      this.transition(run, "working");
      return run;
    } catch (err: any) {
      this.fail(run, err.message ?? String(err));
      return run;
    }
  }

  /** Advance the run based on the agent state reported by Herdr. */
  async advance(run: AgentRun): Promise<void> {
    if (this.busy.has(run.id)) return;
    this.busy.add(run.id);
    try {
      switch (run.status) {
        case "starting":
        case "working":
        case "blocked":
          await this.observeAgent(run);
          break;
        case "validating":
          await this.validate(run);
          break;
        case "creating_change":
          await this.createChange(run);
          break;
        case "review":
          await this.checkChange(run);
          break;
        default:
          break;
      }
    } catch (err: any) {
      this.fail(run, err.message ?? String(err));
    } finally {
      this.busy.delete(run.id);
    }
  }

  private async observeAgent(run: AgentRun): Promise<void> {
    const status = await this.deps.herdr.getAgentStatus(run.herdrAgentId);
    if (status === "unknown") {
      const alive = await this.deps.herdr.workspaceExists(run.herdrWorkspaceId);
      if (!alive) this.fail(run, "herdr workspace is gone");
      return;
    }
    if (status === "blocked") {
      if (run.status !== "blocked") {
        const tail = await this.deps.herdr.readAgent(run.herdrAgentId, 40).catch(() => "");
        const reason = tail.split("\n").filter((l) => l.trim()).slice(-6).join("\n");
        this.event("AgentBlocked", run, { reason });
        this.transition(run, "blocked", { blockedReason: reason });
        this.deps.tasks
          .addComment(run.taskId, `Agent is blocked in Herdr (\`${run.herdrWorkspaceId}\`):\n\n${reason}`)
          .catch(() => {});
      }
      return;
    }
    if (status === "working") {
      this.idleTicks.delete(run.id);
      if (run.status !== "working") this.transition(run, "working");
      return;
    }
    // idle | done — require two polls in a row so a pause is not mistaken for completion
    const ticks = (this.idleTicks.get(run.id) ?? 0) + 1;
    this.idleTicks.set(run.id, ticks);
    if (ticks >= 2) {
      this.idleTicks.delete(run.id);
      this.transition(run, "validating");
    }
  }

  private async validate(run: AgentRun): Promise<void> {
    const cfg = this.projectConfigById(run.projectId);
    const base = cfg?.base_branch ?? this.deps.config.code_provider.base_branch;
    this.event("ValidationStarted", run);

    if ((await git.commitCount(run.worktreePath, base)) === 0) {
      this.event("ValidationFinished", run, { ok: false, reason: "no commits" });
      await this.sendBack(run, policy.noCommitsFeedback(run.branch));
      return;
    }
    if (cfg?.validate) {
      const result = await git.runCommand(run.worktreePath, cfg.validate);
      this.event("ValidationFinished", run, { ok: result.ok, command: cfg.validate });
      if (!result.ok) {
        await this.sendBack(run, policy.validationFeedback(cfg.validate, result.output));
        return;
      }
    } else {
      this.event("ValidationFinished", run, { ok: true, skipped: true });
    }
    this.transition(run, "creating_change");
  }

  /** A failed validation goes back to the same agent; the run continues. */
  private async sendBack(run: AgentRun, message: string): Promise<void> {
    await this.deps.herdr.prompt(run.herdrAgentId, message);
    this.transition(run, "working");
  }

  private async createChange(run: AgentRun): Promise<void> {
    const existing = db.getChangeForRun(this.deps.db, run.id);
    if (existing) {
      this.transition(run, "review", { changeId: existing.id });
      return;
    }
    const task = db.getTask(this.deps.db, run.taskId);
    const cfg = this.projectConfigById(run.projectId);
    await git.pushBranch(run.worktreePath, run.branch);
    const change = await this.deps.code.createChange({
      repoPath: run.worktreePath,
      branch: run.branch,
      baseBranch: cfg?.base_branch ?? this.deps.config.code_provider.base_branch,
      title: task ? `${task.id}: ${task.title}` : run.branch,
      body: task ? policy.changeBody(task, run) : run.branch,
    });
    db.recordChange(this.deps.db, { ...change, runId: run.id });
    this.event("ChangeCreated", run, change);
    this.deps.tasks.addComment(run.taskId, `Pull request: ${change.url}`).catch(() => {});
    this.transition(run, "review", { changeId: change.id });
    await this.ensureReviewAgent(run, change.url);
  }

  /** The run closes once a human merges the change. */
  private async checkChange(run: AgentRun): Promise<void> {
    const change = db.getChangeForRun(this.deps.db, run.id);
    if (!change) {
      this.transition(run, "creating_change");
      return;
    }
    await this.ensureReviewAgent(run, change.url);
    const fresh = await this.deps.code.getChange(change.id, run.worktreePath);
    if (fresh.status === "merged") {
      this.transition(run, "completed", { finishedAt: new Date() });
      this.event("RunCompleted", run, { change: fresh.url });
      await this.deps.herdr.closeWorkspace(run.herdrWorkspaceId).catch(() => {});
      db.closeWorkspaceRow(this.deps.db, run.herdrWorkspaceId);
    } else if (fresh.status === "closed") {
      this.fail(run, `change ${fresh.url} was closed`);
    }
  }

  /**
   * The review agent lives in its own tab of the same workspace.
   * Its verdict goes into pull request comments — the orchestrator never parses its answer.
   */
  async ensureReviewAgent(run: AgentRun, changeUrl?: string): Promise<boolean> {
    const role = policy.resolveAgentRole("review", this.deps.config, this.projectConfigById(run.projectId));
    if (!role.prompt.trim()) return false;
    if (db.hasEvent(this.deps.db, run.id, "ReviewAgentStarted")) return false;
    const url = changeUrl ?? db.getChangeForRun(this.deps.db, run.id)?.url;
    const task = db.getTask(this.deps.db, run.taskId);
    if (!url || !task) return false;
    try {
      const tab = await this.deps.herdr.createTab({
        workspaceId: run.herdrWorkspaceId,
        cwd: run.worktreePath,
        label: "review",
      });
      const name = policy.reviewAgentName(run.herdrAgentId);
      await this.deps.herdr.spawnAgent({ name, kind: role.kind, paneId: tab.paneId });
      await this.deps.herdr.prompt(
        name,
        policy.reviewPrompt(role.prompt, task, { changeUrl: url, branch: run.branch }),
      );
      this.event("ReviewAgentStarted", run, { agent: name, tab: tab.tabId });
      return true;
    } catch (err: any) {
      // review is a bonus step: the pull request already exists, so do not fail the run
      this.event("ReviewAgentFailed", run, { error: err.message ?? String(err) });
      this.log(`review ${run.id}: ${err.message ?? err}`);
      return false;
    }
  }

  async stop(run: AgentRun, reason = "stopped by user"): Promise<void> {
    await this.deps.herdr.stopAgent(run.herdrAgentId).catch(() => {});
    this.fail(run, reason);
  }

  private fail(run: AgentRun, error: string): void {
    db.updateRun(this.deps.db, run.id, { status: "failed", error, finishedAt: new Date() });
    run.status = "failed";
    this.event("RunFailed", run, { error });
    this.log(`run ${run.id} failed: ${error}`);
  }

  private projectConfig(project: Project): ProjectConfig {
    const cfg = this.deps.projectConfigs.get(project.id);
    if (!cfg) throw new Error(`project ${project.name} is not in config`);
    return cfg;
  }

  private projectConfigById(id: string): ProjectConfig | undefined {
    return this.deps.projectConfigs.get(id);
  }
}
