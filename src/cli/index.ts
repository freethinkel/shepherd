import { execFile } from "node:child_process";
import { chmodSync, existsSync, mkdirSync, statSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { parseArgs } from "node:util";

import { promisify } from "node:util";
import { createApp, type App } from "../app.ts";
import { EXAMPLE_CONFIG, configPath, loadConfig, userConfigPath } from "../config/schema.ts";
import { isRunActive } from "../domain/status.ts";
import * as db from "../persistence/db.ts";
import { removeWorktree, resolveRepository } from "../repositories/git.ts";
import { loadCustomProviders, providersDir } from "../providers/load.ts";
import * as daemon from "./daemon.ts";
import * as view from "../view.ts";
import { icon, projectsTree, table } from "./render.ts";

const exec = promisify(execFile);

const log = (msg: string) => console.error(`· ${msg}`);
const out = (msg: string) => console.log(msg);

const USAGE = `shepherd — a control plane on top of Herdr

  shepherd init                 create shepherd.toml
  shepherd doctor               check the environment
  shepherd projects             tree of projects and tasks
  shepherd status               cross-project summary
  shepherd tasks [project]      tasks from the tracker
  shepherd agents               live agents
  shepherd runs [project]       run history
  shepherd run                  start the orchestration loop (foreground)
  shepherd daemon <cmd>         install | uninstall | start | stop | logs
  shepherd run <task-id>        start a single run
  shepherd stop <run-id>
  shepherd retry <run-id>
  shepherd open <run-id>        focus the run's Herdr workspace
  shepherd review [run-id]      start review agents for runs awaiting review
  shepherd cleanup              remove worktrees/workspaces of finished runs
`;

async function main() {
  // `shepherd runs | head` closes the pipe before we are done — not an error
  process.stdout.on("error", (err: NodeJS.ErrnoException) => {
    if (err.code === "EPIPE") process.exit(0);
  });

  const { positionals } = parseArgs({ allowPositionals: true, strict: false });
  const [command = "status", arg] = positionals as string[];

  if (command === "help" || command === "--help") return out(USAGE);
  if (command === "init") return init();
  if (command === "doctor") return doctor();

  const app = await createApp(log);
  switch (command) {
    case "projects": {
      await app.scheduler.syncProjects();
      return out(projectsTree(view.overview(app.db)));
    }
    case "status": {
      await app.scheduler.syncProjects();
      const views = view.overview(app.db);
      const pid = daemon.runningPid();
      out(pid ? `daemon: running (pid ${pid})` : `daemon: stopped${daemon.installed() ? "" : " · shepherd daemon install"}`);
      return out(
        table(
          ["PROJECT", "AGENTS", "QUEUED", "ATTENTION"],
          views.map((v) => [
            v.project.name,
            [
              v.counts.working ? `${v.counts.working} working` : "",
              v.counts.blocked ? `${v.counts.blocked} blocked` : "",
              v.counts.done ? `${v.counts.done} done` : "",
            ].filter(Boolean).join(", ") || "—",
            v.counts.queued,
            v.attention ? "yes" : "no",
          ]),
        ),
      );
    }
    case "tasks": {
      await app.scheduler.syncProjects();
      await app.scheduler.syncTasks();
      const projects = db.listProjects(app.db).filter((p) => !arg || p.id === arg || p.name === arg);
      return out(
        table(
          ["", "TASK", "TITLE", "PROJECT", "STATUS"],
          projects.flatMap((p) =>
            view.projectView(app.db, p).tasks.map((t) => [
              icon(t.status), t.task.id, t.task.title.slice(0, 40), p.name, t.status,
            ]),
          ),
        ),
      );
    }
    case "agents": {
      const rows = view.agents(app.db);
      if (rows.length === 0) return out("no active agents");
      return out(
        table(
          ["PROJECT", "TASK", "AGENT", "STATUS", "WORKSPACE", "RUN"],
          rows.map((a) => [a.project, a.taskId, a.kind, a.status, a.workspace, a.runId]),
        ),
      );
    }
    case "runs": {
      const project = arg ? db.listProjects(app.db).find((p) => p.id === arg || p.name === arg) : undefined;
      const runs = db.listRuns(app.db, project?.id);
      return out(
        table(
          ["RUN", "TASK", "STATUS", "BRANCH", "CHANGE", "STARTED"],
          runs.map((r) => [
            r.id, r.taskId, r.status, r.branch, r.changeId ?? "—",
            r.startedAt.toISOString().slice(0, 16).replace("T", " "),
          ]),
        ),
      );
    }
    case "run":
      return arg ? runTask(app, arg) : loop(app);
    case "stop": {
      const run = requireRun(app, arg);
      await app.workflow.stop(run);
      return out(`run ${run.id} stopped`);
    }
    case "retry": {
      const previous = requireRun(app, arg);
      if (isRunActive(previous.status)) await app.workflow.stop(previous, "restarted");
      const project = db.getProject(app.db, previous.projectId)!;
      const task = db.getTask(app.db, previous.taskId)!;
      const run = await app.workflow.start(project, task);
      return out(`new run ${run.id} (${run.status})`);
    }
    case "open": {
      const run = requireRun(app, arg);
      await app.herdr.focusWorkspace(run.herdrWorkspaceId);
      return out(`workspace ${run.herdrWorkspaceId} focused`);
    }
    case "review":
      return startReviews(app, arg);
    case "daemon":
      return manageDaemon(arg);
    case "cleanup":
      return cleanup(app);
    default:
      out(USAGE);
      process.exitCode = 1;
  }
}

function requireRun(app: App, id?: string) {
  if (!id) throw new Error("<run-id> is required");
  const run = db.getRun(app.db, id);
  if (!run) throw new Error(`run ${id} not found`);
  return run;
}

function init() {
  const path = process.env.SHEPHERD_CONFIG ?? userConfigPath();
  if (existsSync(path)) return out(`${path} already exists`);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, EXAMPLE_CONFIG, { mode: 0o600 }); // the file holds keys
  out(`created ${path}\nnext: put LINEAR_API_KEY into [env] and run shepherd doctor`);
}

async function doctor() {
  const checks: [string, () => Promise<string>][] = [
    ["config", async () => {
      const path = configPath();
      const config = loadConfig(path);
      const mode = statSync(path).mode & 0o777;
      if (Object.keys(config.env).length > 0 && (mode & 0o077) !== 0) {
        chmodSync(path, 0o600); // the config holds secrets — nobody else needs to read it
        out(`· mode ${mode.toString(8)} on ${path} tightened to 600`);
      }
      return `${path} (${config.projects.length} projects)`;
    }],
    ["herdr", async () => (await exec("herdr", ["--version"])).stdout.trim()],
    ["git", async () => (await exec("git", ["--version"])).stdout.trim()],
    ["gh", async () => {
      const r = await exec("gh", ["auth", "status"]);
      return `${r.stdout}${r.stderr}`.split("\n").find((l) => l.includes("Logged in"))?.trim() ?? "ok";
    }],
    ["LINEAR_API_KEY", async () => (process.env.LINEAR_API_KEY ? "set" : Promise.reject(new Error("not set")))],
  ];
  let ok = true;
  for (const [name, check] of checks) {
    try {
      out(`✓ ${name.padEnd(16)} ${await check()}`);
    } catch (err: any) {
      ok = false;
      out(`✗ ${name.padEnd(16)} ${String(err.message ?? err).split("\n")[0]}`);
    }
  }
  const custom = await loadCustomProviders();
  for (const error of custom.errors) out(`✗ provider        ${error}`);
  if (custom.loaded.length) out(`✓ providers       ${custom.loaded.join(", ")} (${providersDir()})`);
  for (const p of loadConfig().projects) {
    try {
      const repo = await resolveRepository(p.repository);
      out(`✓ repo ${p.name.padEnd(11)} ${repo.path} (${repo.defaultBranch})`);
    } catch (err: any) {
      ok = false;
      out(`✗ repo ${p.name.padEnd(11)} ${err.message}`);
    }
  }
  if (!ok) process.exitCode = 1;
}

async function loop(app: App) {
  daemon.lockLoop();
  const controller = new AbortController();
  process.on("SIGINT", () => {
    out("\nstopping (agents in Herdr keep running)");
    controller.abort();
  });
  out(`orchestration started, max_concurrent_runs=${app.config.orchestrator.max_concurrent_runs}`);
  await app.scheduler.loop(controller.signal);
}

/** A single run: driven to a terminal state right here. */
async function runTask(app: App, taskId: string) {
  await app.scheduler.syncProjects();
  let task = db.getTask(app.db, taskId);
  if (!task) {
    await app.scheduler.syncTasks();
    task = db.getTask(app.db, taskId);
  }
  if (!task) throw new Error(`task ${taskId} not found`);
  const project = db.getProject(app.db, task.projectId);
  if (!project) throw new Error(`project of task ${taskId} is not configured`);

  const existing = db.runsForTask(app.db, task.id).findLast((r) => isRunActive(r.status));
  let run = existing ?? (await app.workflow.start(project, task));
  out(`run ${run.id} → ${run.status}`);
  while (isRunActive(run.status)) {
    await new Promise((r) => setTimeout(r, app.config.orchestrator.poll_interval_ms));
    await app.workflow.advance(run);
    const fresh = db.getRun(app.db, run.id)!;
    if (fresh.status !== run.status) out(`${icon(fresh.status)} ${fresh.status}`);
    run = fresh;
    if (run.status === "blocked") {
      out(`agent is waiting for an answer: shepherd open ${run.id}\n${run.blockedReason ?? ""}`);
      break;
    }
    if (run.status === "review") {
      out(`change: ${db.getChangeForRun(app.db, run.id)?.url}`);
      break;
    }
  }
}

/** Runs sitting in review without a review agent: started after a config change or a failed launch. */
async function startReviews(app: App, runId?: string) {
  const runs = runId
    ? [requireRun(app, runId)]
    : db.listRuns(app.db).filter((r) => r.status === "review");
  let started = 0;
  for (const run of runs) {
    if (await app.workflow.ensureReviewAgent(run)) {
      out(`${run.taskId}: review agent started (${run.herdrWorkspaceId})`);
      started++;
    }
  }
  out(started ? `review agents started: ${started}` : "nothing to start");
}

async function manageDaemon(action = "status") {
  switch (action) {
    case "install": {
      const bin = process.execPath.endsWith("/node") ? process.argv[1]! : process.execPath;
      const path = await daemon.install(bin);
      return out(`launchd agent installed: ${path}\nlog: ${daemon.logPath()}`);
    }
    case "uninstall":
      await daemon.uninstall();
      return out("launchd agent removed");
    case "start":
      await daemon.start();
      return out("started");
    case "stop":
      await daemon.stop();
      return out("stopped");
    case "logs":
      return out(daemon.logPath());
    default: {
      const pid = daemon.runningPid();
      return out(
        `agent: ${daemon.installed() ? "installed" : "not installed"}\n` +
          `loop: ${pid ? `running (pid ${pid})` : "stopped"}\nlog: ${daemon.logPath()}`,
      );
    }
  }
}

async function cleanup(app: App) {
  let removed = 0;
  for (const run of db.listRuns(app.db)) {
    if (isRunActive(run.status)) continue;
    const project = db.getProject(app.db, run.projectId);
    const repo = project ? db.getRepository(app.db, project.repositoryId) : undefined;
    if (repo) await removeWorktree(repo.path, run.worktreePath);
    if (run.herdrWorkspaceId && (await app.herdr.workspaceExists(run.herdrWorkspaceId))) {
      await app.herdr.closeWorkspace(run.herdrWorkspaceId).catch(() => {});
      db.closeWorkspaceRow(app.db, run.herdrWorkspaceId);
    }
    removed++;
  }
  out(`runs cleaned up: ${removed}`);
}

main().catch((err) => {
  console.error(`error: ${err.message ?? err}`);
  process.exitCode = 1;
});
