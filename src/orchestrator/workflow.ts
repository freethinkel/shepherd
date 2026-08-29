import { randomUUID } from "node:crypto";
import type { Config, ProjectConfig } from "../config/schema.ts";
import { taskStatusForRun } from "../domain/status.ts";
import type { AgentRun, Change, ChangeComment, Project, RunStatus, Task } from "../domain/types.ts";
import type { HerdrClient } from "../herdr/client.ts";
import type { ProviderRegistry } from "../providers/registry.ts";
import * as db from "../persistence/db.ts";
import * as git from "../repositories/git.ts";
import { briefError } from "../log.ts";
import * as policy from "./policies.ts";

export interface WorkflowDeps {
  db: db.Db;
  herdr: HerdrClient;
  registry: ProviderRegistry;
  config: Config;
  projectConfigs: Map<string, ProjectConfig>;
  log?: (msg: string) => void;
}

/** One workflow step = one run state transition, recorded in SQLite. */
export class Workflow {
  private readonly busy = new Set<string>();
  private readonly idleTicks = new Map<string, number>();
  /** When the agent was last given something to do, and whether it started doing it. */
  private readonly prompted = new Map<string, { at: number; sawWorking: boolean }>();
  // ponytail: last check kept in memory. A restart re-checks once, which costs one API call.
  private readonly lastChangeCheck = new Map<string, number>();

  constructor(private readonly deps: WorkflowDeps) {}

  /** Updates go back to the tracker the task came from, recorded at sync time. */
  private taskProvider(taskId: string) {
    const name = db.getTask(this.deps.db, taskId)?.provider;
    return this.deps.registry.tasks(name ?? this.deps.registry.taskProviderNames()[0]!);
  }

  /** The repository's remote decides where the change is opened. */
  private codeProvider(run: AgentRun) {
    const project = db.getProject(this.deps.db, run.projectId);
    const repo = project ? db.getRepository(this.deps.db, project.repositoryId) : undefined;
    return this.deps.registry.codeForRemote(repo?.remote);
  }

  private log(msg: string) {
    this.deps.log?.(msg);
  }

  private event(type: string, run: AgentRun, data?: unknown) {
    db.appendEvent(this.deps.db, type, {
      runId: run.id,
      taskId: run.taskId,
      projectId: run.projectId,
      data,
    });
  }

  private transition(
    run: AgentRun,
    status: RunStatus,
    patch: Parameters<typeof db.updateRun>[2] = {},
  ) {
    db.updateRun(this.deps.db, run.id, { status, ...patch });
    run.status = status;
    this.event("RunStatusChanged", run, { status });
    this.syncTaskStatus(run, status);
  }

  private syncTaskStatus(run: AgentRun, status: RunStatus) {
    const taskStatus = taskStatusForRun(status);
    db.setTaskStatus(this.deps.db, run.taskId, taskStatus);
    this.event("TaskStatusChanged", run, { status: taskStatus });
    this.taskProvider(run.taskId)
      .updateStatus(run.taskId, taskStatus)
      .catch((err) => this.log(`task ${run.taskId}: ${briefError(err)}`));
  }

  /** Create a run and bring an agent up in Herdr. */
  async start(project: Project, task: Task): Promise<AgentRun> {
    const cfg = this.projectConfig(project);
    const repo = db.getRepository(this.deps.db, project.repositoryId);
    if (!repo) throw new Error(`repository for project ${project.name} is not registered`);

    const dev = policy.resolveAgentRole("dev", this.deps.config, cfg);
    const branch = git.branchName(task.id, task.title);
    const run: AgentRun = {
      id: `run_${randomUUID().replace(/-/g, "").slice(0, 10)}`,
      projectId: project.id,
      taskId: task.id,
      herdrWorkspaceId: "",
      herdrAgentId: policy.agentName(project, task),
      agentKind: dev.kind,
      branch,
      worktreePath: "", // filled in once herdr reports where the worktree landed
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
      await this.taskProvider(task.id)
        .claimTask(task.id)
        .catch((err) => this.log(`claim ${task.id}: ${briefError(err)}`));
      this.transition(run, "starting");

      // herdr creates the worktree and its workspace together, and reopens the same
      // one on a retry instead of piling up a second workspace for the task
      const ws = await this.deps.herdr.openOrCreateWorktree({
        repoPath: repo.path,
        branch,
        base: cfg.base_branch ?? this.deps.config.code_provider.base_branch ?? repo.defaultBranch,
        label: policy.workspaceLabel(project, task),
        path: this.deps.config.orchestrator.worktrees
          ? policy.worktreePath(this.deps.config.orchestrator.worktrees, project, task)
          : undefined,
      });
      run.worktreePath = ws.path;
      db.recordWorkspace(this.deps.db, {
        id: ws.workspaceId,
        runId: run.id,
        label: ws.label,
        cwd: ws.path,
      });
      db.updateRun(this.deps.db, run.id, {
        herdrWorkspaceId: ws.workspaceId,
        worktreePath: ws.path,
      });
      run.herdrWorkspaceId = ws.workspaceId;

      await this.spawnAgent(run, dev, ws);
      this.event("AgentStarted", run, { agent: run.herdrAgentId, workspace: ws.workspaceId });

      const feedback = await this.resumeChange(run);
      this.prompted.set(run.id, { at: Date.now(), sawWorking: false });
      await this.deps.herdr.prompt(
        run.herdrAgentId,
        [
          policy.buildPrompt(task, { branch, validate: cfg.validate, prefix: dev.prompt }),
          ...(feedback ? ["", feedback] : []),
        ].join("\n"),
      );
      this.transition(run, "working");
      return run;
    } catch (err: any) {
      this.fail(run, briefError(err, 500));
      return run;
    }
  }

  /** An open change already on the branch is taken over, with its review comments. */
  private async resumeChange(run: AgentRun): Promise<string | undefined> {
    const provider = this.codeProvider(run);
    const existing = await provider.findChange?.(run.branch, run.worktreePath).catch((err) => {
      this.log(`find change ${run.id}: ${briefError(err)}`);
      return undefined;
    });
    if (!existing || existing.status !== "open") return undefined;
    db.recordChange(this.deps.db, { ...existing, runId: run.id });
    this.event("ChangeResumed", run, { change: existing.url });
    return policy.reviewFeedback(existing.url, await this.newComments(run, existing.id));
  }

  /** Comments since the change was opened or last handed back; all of them on a fresh database. */
  private async newComments(run: AgentRun, changeId: string): Promise<ChangeComment[]> {
    const since = [
      db.lastEventAt(this.deps.db, run.taskId, "ReviewRejected"),
      db.lastEventAt(this.deps.db, run.taskId, "ChangeCreated"),
    ]
      .filter((d): d is Date => d !== undefined)
      .sort((a, b) => b.getTime() - a.getTime())[0];
    const provider = this.codeProvider(run);
    const comments = provider.listComments
      ? await provider.listComments(changeId, run.worktreePath).catch((err) => {
          this.log(`comments ${run.id}: ${briefError(err)}`);
          return [];
        })
      : [];
    return policy.commentsSince(comments, since);
  }

  /**
   * A reopened workspace may already hold the agent from a previous run, and its root pane
   * may be busy. Reuse a live agent with our name, otherwise start one in a free pane.
   */
  private async spawnAgent(
    run: AgentRun,
    role: policy.AgentRole,
    ws: { workspaceId: string; paneId: string },
  ): Promise<void> {
    const live = await this.deps.herdr.listAgents().catch(() => []);
    const ours = live.find((a) => a.name === run.herdrAgentId);
    if (ours) {
      this.event("AgentReused", run, { agent: ours.name, pane: ours.paneId });
      return;
    }
    const paneBusy = live.some((a) => a.paneId === ws.paneId);
    const paneId = paneBusy
      ? (
          await this.deps.herdr.createTab({
            workspaceId: ws.workspaceId,
            cwd: run.worktreePath,
            label: "agent",
          })
        ).paneId
      : ws.paneId;
    await this.deps.herdr.spawnAgent({
      name: run.herdrAgentId,
      kind: role.kind,
      paneId,
      args: role.args,
    });
  }

  /** Advance the run based on the agent state reported by Herdr. */
  async advance(run: AgentRun): Promise<void> {
    if (this.busy.has(run.id)) return;
    if (this.expired(run)) return;
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
      this.fail(run, briefError(err, 500));
    } finally {
      this.busy.delete(run.id);
    }
  }

  /**
   * A hung agent, or a question nobody answers, would otherwise hold its slot forever.
   * Waiting in review is excluded: a pull request may legitimately sit for days.
   */
  private expired(run: AgentRun): boolean {
    if (run.status === "review") return false;
    const limit = this.deps.config.orchestrator.run_timeout_ms;
    const age = Date.now() - run.startedAt.getTime();
    if (age < limit) return false;
    this.fail(run, `run timed out after ${Math.round(age / 60_000)} min in status ${run.status}`);
    return true;
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
        const reason = tail
          .split("\n")
          .filter((l) => l.trim())
          .slice(-6)
          .join("\n");
        this.event("AgentBlocked", run, { reason });
        this.transition(run, "blocked", { blockedReason: reason });
        this.taskProvider(run.taskId)
          .addComment(
            run.taskId,
            `Agent is blocked in Herdr (\`${run.herdrWorkspaceId}\`):\n\n${reason}`,
          )
          .catch(() => {});
      }
      return;
    }
    if (status === "working") {
      this.idleTicks.delete(run.id);
      const prompt = this.prompted.get(run.id);
      if (prompt) prompt.sawWorking = true;
      if (run.status !== "working") this.transition(run, "working");
      return;
    }

    // The agent is idle after `agent start` too, with the prompt still unread. Until it has
    // been seen working at least once, idle only counts after the settle window.
    const prompt = this.prompted.get(run.id);
    if (prompt && !prompt.sawWorking) {
      if (Date.now() - prompt.at < this.deps.config.orchestrator.agent_settle_ms) return;
      this.event("AgentNeverStarted", run, { afterMs: Date.now() - prompt.at });
      this.prompted.delete(run.id);
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

  /**
   * A failed validation goes back to the same agent, but not forever: a check that
   * can never pass would keep the pair looping and the slot occupied.
   */
  private async sendBack(run: AgentRun, message: string): Promise<void> {
    const rounds = db.countEvents(this.deps.db, run.id, "ValidationRejected");
    const max = this.deps.config.orchestrator.max_validation_rounds;
    if (rounds >= max) {
      this.fail(run, `validation still failing after ${max} attempts`);
      return;
    }
    this.event("ValidationRejected", run, { round: rounds + 1 });
    this.prompted.set(run.id, { at: Date.now(), sawWorking: false });
    this.idleTicks.delete(run.id);
    await this.deps.herdr.prompt(run.herdrAgentId, message);
    this.transition(run, "working");
  }

  private async createChange(run: AgentRun): Promise<void> {
    await git.pushBranch(run.worktreePath, run.branch);
    const existing = db.getChangeForRun(this.deps.db, run.id);
    if (existing) {
      this.transition(run, "review", { changeId: existing.id });
      return;
    }
    const task = db.getTask(this.deps.db, run.taskId);
    const cfg = this.projectConfigById(run.projectId);
    const change = await this.codeProvider(run).createChange({
      repoPath: run.worktreePath,
      branch: run.branch,
      baseBranch: cfg?.base_branch ?? this.deps.config.code_provider.base_branch,
      title: task ? `${task.id}: ${task.title}` : run.branch,
      body: task ? policy.changeBody(task, run) : run.branch,
    });
    db.recordChange(this.deps.db, { ...change, runId: run.id });
    this.event("ChangeCreated", run, change);
    this.taskProvider(run.taskId)
      .addComment(run.taskId, `Pull request: ${change.url}`)
      .catch(() => {});
    this.transition(run, "review", { changeId: change.id });
    await this.ensureReviewAgent(run, change.url);
  }

  private async checkChange(run: AgentRun): Promise<void> {
    const change = db.getChangeForRun(this.deps.db, run.id);
    if (!change) {
      this.transition(run, "creating_change");
      return;
    }
    // ponytail: a failed tracker write leaves todo synced and reworks on its own; bounded by the cap
    if (db.getTask(this.deps.db, run.taskId)?.status === "todo") {
      await this.rework(run, change);
      return;
    }
    await this.ensureReviewAgent(run, change.url);

    const since = Date.now() - (this.lastChangeCheck.get(run.id) ?? 0);
    if (since < this.deps.config.orchestrator.change_poll_interval_ms) return;
    this.lastChangeCheck.set(run.id, Date.now());

    const fresh = await this.codeProvider(run).getChange(change.id, run.worktreePath);
    if (fresh.status === "merged") {
      this.transition(run, "completed", { finishedAt: new Date() });
      this.event("RunCompleted", run, { change: fresh.url });
      await this.deps.herdr.closeWorkspace(run.herdrWorkspaceId).catch(() => {});
      db.closeWorkspaceRow(this.deps.db, run.herdrWorkspaceId);
    } else if (fresh.status === "closed") {
      this.fail(run, `change ${fresh.url} was closed`);
    } else if (fresh.approved && fresh.checks === "success") {
      await this.merge(run, change);
    }
  }

  private async merge(run: AgentRun, change: Change): Promise<void> {
    if (!this.deps.config.orchestrator.auto_merge) return;
    if (db.countEvents(this.deps.db, run.id, "ChangeMerged") > 0) return;
    const failures = db.countEvents(this.deps.db, run.id, "MergeFailed");
    if (failures >= 3) return;
    try {
      await this.codeProvider(run).mergeChange(change.id, run.worktreePath);
      this.event("ChangeMerged", run, { change: change.url });
    } catch (err: any) {
      const error = briefError(err);
      this.event("MergeFailed", run, { error });
      this.log(`merge ${run.id}: ${error}`);
      if (failures === 0) {
        this.taskProvider(run.taskId)
          .addComment(run.taskId, `Approved, but merging ${change.url} failed:\n\n\`${error}\``)
          .catch(() => {});
      }
    }
  }

  private async rework(run: AgentRun, change: Change): Promise<void> {
    const rounds = db.countEvents(this.deps.db, run.id, "ReviewRejected");
    const max = this.deps.config.orchestrator.max_review_rounds;
    if (rounds >= max) {
      this.event("ReviewRoundsExhausted", run, { rounds });
      this.fail(run, `still sent back after ${max} review rounds`);
      return;
    }
    const comments = await this.newComments(run, change.id);
    this.prompted.set(run.id, { at: Date.now(), sawWorking: false });
    this.idleTicks.delete(run.id);
    await this.deps.herdr.prompt(run.herdrAgentId, policy.reviewFeedback(change.url, comments));
    this.event("ReviewRejected", run, { round: rounds + 1, comments: comments.length });
    this.transition(run, "working");
  }

  /**
   * The review agent lives in its own tab of the same workspace.
   * Its verdict goes into pull request comments — the orchestrator never parses its answer.
   */
  async ensureReviewAgent(run: AgentRun, changeUrl?: string): Promise<boolean> {
    const role = policy.resolveAgentRole(
      "review",
      this.deps.config,
      this.projectConfigById(run.projectId),
    );
    if (!role.prompt.trim()) return false;
    if (
      db.countEvents(this.deps.db, run.id, "ReviewAgentStarted") >
      db.countEvents(this.deps.db, run.id, "ReviewRejected")
    ) {
      return false;
    }
    const url = changeUrl ?? db.getChangeForRun(this.deps.db, run.id)?.url;
    const task = db.getTask(this.deps.db, run.taskId);
    if (!url || !task) return false;
    try {
      const name = policy.reviewAgentName(run.herdrAgentId);
      const live = await this.deps.herdr.listAgents().catch(() => []);
      let tabId: string | undefined;
      if (!live.some((a) => a.name === name)) {
        const tab = await this.deps.herdr.createTab({
          workspaceId: run.herdrWorkspaceId,
          cwd: run.worktreePath,
          label: "review",
        });
        tabId = tab.tabId;
        await this.deps.herdr.spawnAgent({
          name,
          kind: role.kind,
          paneId: tab.paneId,
          args: role.args,
        });
      }
      await this.deps.herdr.prompt(
        name,
        policy.reviewPrompt(role.prompt, task, { changeUrl: url, branch: run.branch }),
      );
      this.event("ReviewAgentStarted", run, { agent: name, tab: tabId });
      return true;
    } catch (err: any) {
      // review is a bonus step: the pull request already exists, so do not fail the run
      this.event("ReviewAgentFailed", run, { error: briefError(err) });
      this.log(`review ${run.id}: ${briefError(err)}`);
      return false;
    }
  }

  async stop(run: AgentRun, reason = "stopped by user"): Promise<void> {
    await this.deps.herdr.stopAgent(run.herdrAgentId).catch(() => {});
    this.fail(run, reason);
  }

  /** A failed run hands the task back with the reason, so nothing silently rots in In Progress. */
  private fail(run: AgentRun, error: string): void {
    db.updateRun(this.deps.db, run.id, { status: "failed", error, finishedAt: new Date() });
    run.status = "failed";
    this.event("RunFailed", run, { error });
    this.log(`run ${run.id} failed: ${error}`);

    this.syncTaskStatus(run, "failed"); // back to the tracker's Todo column
    const attempts = db.runsForTask(this.deps.db, run.taskId).length;
    const max = this.deps.config.orchestrator.max_attempts;
    this.taskProvider(run.taskId)
      .addComment(
        run.taskId,
        [
          `Run \`${run.id}\` failed on attempt ${attempts} of ${max}.`,
          "",
          "```",
          error.slice(0, 2000),
          "```",
          `Branch \`${run.branch}\`, Herdr workspace \`${run.herdrWorkspaceId || "none"}\`.`,
          attempts >= max
            ? "Automatic retries stopped. Fix the cause and run `shepherd retry` to try again."
            : "The task is back in the queue and will be picked up again.",
        ].join("\n"),
      )
      .catch((err) => this.log(`comment ${run.taskId}: ${briefError(err)}`));
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
