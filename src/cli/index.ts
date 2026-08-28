import { execFile, spawnSync } from "node:child_process";
import { chmodSync, existsSync, mkdirSync, statSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { parseArgs } from "node:util";

import { promisify } from "node:util";
import { createApp, type App } from "../app.ts";
import { EXAMPLE_CONFIG, configPath, loadConfig, userConfigPath } from "../config/schema.ts";
import { isRunActive } from "../domain/status.ts";
import * as db from "../persistence/db.ts";
import { briefError } from "../log.ts";
import { codeProviderForRemote } from "../orchestrator/policies.ts";
import { deleteBranch, resolveRepository } from "../repositories/git.ts";
import { loadCustomProviders, providersDirs } from "../providers/load.ts";
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
  shepherd daemon <cmd>         install | start | restart | stop | uninstall | logs
  shepherd run <task-id>        start a single run
  shepherd stop <run-id>
  shepherd retry <run-id>
  shepherd open <run-id>        focus the run's Herdr workspace
  shepherd review [run-id]      start review agents for runs awaiting review
  shepherd reset <task-id>      throw away the task's runs, worktree and branch
  shepherd reset <task-id> --run  ... and start a fresh run right away
  shepherd logs [-f] [-n N]     daemon log (default: last 50 lines)
  shepherd events [run-id]      state transitions recorded in SQLite
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
  if (command === "logs") return showLogs(positionals.slice(1) as string[]);
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
      out(
        pid
          ? `daemon: running (pid ${pid})`
          : `daemon: stopped${daemon.installed() ? "" : " · shepherd daemon install"}`,
      );
      return out(
        table(
          ["PROJECT", "AGENTS", "QUEUED", "REVIEW", "ATTENTION"],
          views.map((v) => [
            v.project.name,
            [
              v.counts.working ? `${v.counts.working} working` : "",
              v.counts.blocked ? `${v.counts.blocked} blocked` : "",
              v.counts.done ? `${v.counts.done} done` : "",
            ]
              .filter(Boolean)
              .join(", ") || "-",
            v.counts.queued,
            v.counts.review,
            v.attention ? "yes" : "no",
          ]),
        ),
      );
    }
    case "tasks": {
      await app.scheduler.syncProjects();
      await app.scheduler.syncTasks();
      const projects = db
        .listProjects(app.db)
        .filter((p) => !arg || p.id === arg || p.name === arg);
      return out(
        table(
          ["", "TASK", "TITLE", "PROJECT", "STATUS"],
          projects.flatMap((p) =>
            view
              .projectView(app.db, p)
              .tasks.map((t) => [
                icon(t.status),
                t.task.id,
                t.task.title.slice(0, 40),
                p.name,
                t.status,
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
      const project = arg
        ? db.listProjects(app.db).find((p) => p.id === arg || p.name === arg)
        : undefined;
      const runs = db.listRuns(app.db, project?.id);
      return out(
        table(
          ["RUN", "TASK", "STATUS", "BRANCH", "CHANGE", "STARTED"],
          runs.map((r) => [
            r.id,
            r.taskId,
            r.status,
            r.branch,
            r.changeId ?? "—",
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
    case "reset":
      return resetTask(app, arg, positionals.includes("--run") || process.argv.includes("--run"));
    case "events": {
      const rows = db.listEvents(app.db, Number(process.env.SHEPHERD_EVENT_LIMIT ?? 40), arg);
      return out(
        table(
          ["AT", "TYPE", "RUN", "TASK", "DATA"],
          rows
            .reverse()
            .map((e: any) => [
              String(e.at).slice(5, 19).replace("T", " "),
              e.type,
              e.run_id ?? "-",
              e.task_id ?? "-",
              (e.data ?? "").slice(0, 60),
            ]),
        ),
      );
    }
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
    [
      "config",
      async () => {
        const path = configPath();
        const config = loadConfig(path);
        const mode = statSync(path).mode & 0o777;
        if (Object.keys(config.env).length > 0 && (mode & 0o077) !== 0) {
          chmodSync(path, 0o600); // the config holds secrets — nobody else needs to read it
          out(`· mode ${mode.toString(8)} on ${path} tightened to 600`);
        }
        return `${path} (${config.projects.length} projects)`;
      },
    ],
    ["herdr", async () => (await exec("herdr", ["--version"])).stdout.trim()],
    ["git", async () => (await exec("git", ["--version"])).stdout.trim()],
    [
      "gh",
      async () => {
        const r = await exec("gh", ["auth", "status"]);
        return (
          `${r.stdout}${r.stderr}`
            .split("\n")
            .find((l) => l.includes("Logged in"))
            ?.trim() ?? "ok"
        );
      },
    ],
    [
      "LINEAR_API_KEY",
      async () => (process.env.LINEAR_API_KEY ? "set" : Promise.reject(new Error("not set"))),
    ],
  ];
  let ok = true;
  for (const [name, check] of checks) {
    try {
      out(`✓ ${name.padEnd(16)} ${await check()}`);
    } catch (err: any) {
      ok = false;
      out(`✗ ${name.padEnd(16)} ${briefError(err)}`);
    }
  }
  const custom = await loadCustomProviders();
  for (const error of custom.errors) out(`✗ provider        ${error}`);
  if (custom.loaded.length)
    out(`✓ providers       ${custom.loaded.join(", ")} (${providersDirs().join(", ")})`);
  let needsGitlab = false;
  let gitlabRepo: string | undefined;
  const config = loadConfig();
  const gitlabHosts = (c: ReturnType<typeof loadConfig>) => ({
    gitlab: (c.code_providers.gitlab?.hosts as string[] | undefined) ?? [],
  });
  for (const p of config.projects) {
    try {
      const repo = await resolveRepository(p.repository);
      if (codeProviderForRemote(repo.remote ?? "", gitlabHosts(config)) === "gitlab") {
        needsGitlab = true;
        gitlabRepo ??= repo.path; // glab resolves the host from the repository's remote
      }
      out(`✓ repo ${p.name.padEnd(11)} ${repo.path} (${repo.defaultBranch})`);
    } catch (err: any) {
      ok = false;
      out(`✗ repo ${p.name.padEnd(11)} ${briefError(err)}`);
    }
  }
  // only asked when some repository actually lives on GitLab
  if (needsGitlab) {
    const glab = await exec("glab", ["auth", "status"], { cwd: gitlabRepo ?? process.cwd() })
      .then(() => true)
      .catch(() => false);
    if (glab) out(`✓ glab             logged in`);
    else if (process.env.GITLAB_TOKEN) out(`✓ GITLAB_TOKEN     set (glab is logged out)`);
    else {
      ok = false;
      out(`✗ gitlab           run \`glab auth login\` or set GITLAB_TOKEN`);
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

/**
 * Back to square one: stop the agents, drop the worktree, the workspace and the local branch,
 * and mark the runs failed. The remote branch and any open change are left alone, because
 * throwing away a merge request nobody asked to close is not ours to decide.
 */
async function resetTask(app: App, taskId?: string, restart = false) {
  if (!taskId) throw new Error("<task-id> is required");
  const task = db.getTask(app.db, taskId);
  if (!task) throw new Error(`task ${taskId} not found`);
  const project = db.getProject(app.db, task.projectId);
  const repo = project ? db.getRepository(app.db, project.repositoryId) : undefined;

  // Order matters: an active run is what keeps the task off the queue. Marking the runs
  // failed first would free the task, and the daemon would start a new one mid-cleanup,
  // on a worktree being deleted underneath it.
  const runs = db.runsForTask(app.db, task.id);
  for (const run of runs) {
    if (run.herdrAgentId) await app.herdr.stopAgent(run.herdrAgentId);
    if (run.herdrWorkspaceId && (await app.herdr.workspaceExists(run.herdrWorkspaceId))) {
      await app.herdr
        .removeWorktree(run.herdrWorkspaceId)
        .catch(() => app.herdr.closeWorkspace(run.herdrWorkspaceId));
      db.closeWorkspaceRow(app.db, run.herdrWorkspaceId);
      out(`workspace ${run.herdrWorkspaceId} removed`);
    }
    if (repo) await deleteBranch(repo.path, run.branch);
    const change = db.getChangeForRun(app.db, run.id);
    if (change) out(`left open: ${change.url}`);
  }
  for (const run of runs) {
    if (isRunActive(run.status)) await app.workflow.stop(run, "reset by user");
  }
  db.setTaskStatus(app.db, task.id, "todo");
  db.appendEvent(app.db, "TaskReset", { taskId: task.id, projectId: task.projectId });
  out(`${task.id} reset`);

  if (restart && project) {
    const run = await app.workflow.start(project, task);
    out(`run ${run.id} -> ${run.status}`);
  } else {
    out(`the daemon picks it up on the next tick, or run: shepherd run ${task.id}`);
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

/** ponytail: tail already does this well, we only remember where the file is. */
function showLogs(args: string[]): void {
  const follow = args.includes("-f") || args.includes("--follow");
  const nIndex = args.findIndex((a) => a === "-n");
  const lines = nIndex >= 0 ? (args[nIndex + 1] ?? "50") : "50";
  spawnSync("tail", [...(follow ? ["-f"] : []), "-n", lines, daemon.logPath()], {
    stdio: "inherit",
  });
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
    case "restart":
      await daemon.restart();
      return out("restarted");
    case "stop":
      await daemon.stop();
      return out("stopped");
    case "logs":
      return showLogs([]);
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
    if (run.herdrWorkspaceId && (await app.herdr.workspaceExists(run.herdrWorkspaceId))) {
      // herdr owns the worktree, so removing the workspace takes the checkout with it
      await app.herdr
        .removeWorktree(run.herdrWorkspaceId)
        .catch(() => app.herdr.closeWorkspace(run.herdrWorkspaceId));
      db.closeWorkspaceRow(app.db, run.herdrWorkspaceId);
    }
    removed++;
  }
  out(`runs cleaned up: ${removed}`);
}

main().catch((err) => {
  console.error(`error: ${briefError(err, 500)}`);
  process.exitCode = 1;
});
