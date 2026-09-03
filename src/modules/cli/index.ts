import { execFile, spawnSync } from "node:child_process";
import { chmodSync, existsSync, mkdirSync, statSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { parseArgs } from "node:util";

import { promisify } from "node:util";
import * as actions from "../../shared/actions.ts";
import { createApp, type App } from "../../core/app.ts";
import {
  EXAMPLE_CONFIG,
  configPath,
  loadConfig,
  userConfigPath,
} from "../../core/config/schema.ts";
import { isRunActive } from "../../shared/domain/status.ts";
import type { TaskStatus } from "../../shared/domain/types.ts";
import * as db from "../../core/persistence/db.ts";
import { briefError } from "../../shared/log.ts";
import { codeProviderForRemote, pickTaskProvider, projectId } from "../orchestrator/policies.ts";
import { resolveRepository } from "../../shared/git.ts";
import { loadCustomProviders, providersDirs } from "../providers/load.ts";
import * as daemon from "./daemon.ts";
import * as view from "../../shared/view.ts";
import { icon, projectsTree, table } from "./render.ts";

const exec = promisify(execFile);

const log = (msg: string) => console.error(`· ${msg}`);
const out = (msg: string) => console.log(msg);

const USAGE = `shepherd — a control plane on top of Herdr

  shepherd init                 create shepherd.yaml
  shepherd doctor               check the environment
  shepherd projects             tree of projects and tasks
  shepherd status               cross-project summary
  shepherd ui                   full-screen dashboard: statuses, agent log, retry
  shepherd tasks [project]      tasks from the tracker
  shepherd task <cmd>           new | show | edit | status | comment | rm
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

  shepherd task new <project> <title...>   create a task in that project's tracker
  shepherd task show <task-id>             title, status, description, link
  shepherd task edit <task-id> [title...]  rename, and --body rewrites the description
  shepherd task status <task-id> <status>  todo | in_progress | in_review | done
  shepherd task comment <task-id> <body...>
  shepherd task rm <task-id>               archive it in the tracker
  ... --body <text> for new/edit, --provider <name> when several trackers are registered
`;

async function main() {
  // `shepherd runs | head` closes the pipe before we are done — not an error
  process.stdout.on("error", (err: NodeJS.ErrnoException) => {
    if (err.code === "EPIPE") process.exit(0);
  });

  const { positionals } = parseArgs({ allowPositionals: true, strict: false });
  const [command = "status", arg] = positionals as string[];

  // --help never reaches positionals: parseArgs is lenient here and files flags under values
  if (command === "help" || process.argv.slice(2).some((a) => a === "--help" || a === "-h"))
    return out(USAGE);
  if (command === "init") return init();
  if (command === "logs") return showLogs(positionals.slice(1) as string[]);
  if (command === "doctor") return doctor();

  const app = await createApp(log);
  switch (command) {
    case "projects": {
      await app.scheduler.syncProjects();
      return out(projectsTree(view.overview(app.db, app.projectConfigs.keys())));
    }
    case "status": {
      await app.scheduler.syncProjects();
      const views = view.overview(app.db, app.projectConfigs.keys());
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
    case "stop":
      return out(await actions.stop(app, arg));
    case "retry":
      return out(await actions.retry(app, arg));
    case "open":
      return out(await actions.open(app, arg));
    case "review":
      return out(await actions.review(app, arg, out));
    case "reset":
      return out(
        await actions.reset(
          app,
          arg,
          positionals.includes("--run") || process.argv.includes("--run"),
          out,
        ),
      );
    case "ui":
      return (await import("../tui/index.tsx")).run(app);
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
    case "task":
      // straight from argv: the top-level parseArgs is lenient, so it scatters --body's value
      // into the positionals and the title swallows it
      return taskCommand(app, process.argv.slice(3));
    case "daemon":
      return manageDaemon(arg);
    case "cleanup":
      return cleanup(app);
    default:
      out(USAGE);
      process.exitCode = 1;
  }
}

/** A task the sync has not seen yet: EGR-27 belongs to whichever tracker owns the other EGR-* keys. */
function siblingProvider(app: App, taskId: string): string | undefined {
  const prefix = `${taskId.split("-")[0]?.toLowerCase()}-`;
  return db.listTasks(app.db).find((t) => t.id.toLowerCase().startsWith(prefix) && t.provider)
    ?.provider;
}

const TASK_STATUSES: TaskStatus[] = [
  "todo",
  "in_progress",
  "waiting_for_agent",
  "in_review",
  "done",
];

/** CRUD against whichever tracker the task belongs to — the orchestrator itself never writes tasks. */
async function taskCommand(app: App, argv: string[]): Promise<void> {
  const { values, positionals } = parseArgs({
    args: argv,
    allowPositionals: true,
    options: { body: { type: "string" }, provider: { type: "string" } },
  });
  const [action, target, ...rest] = positionals;
  const body = values.body as string | undefined;

  const providerFor = (taskId?: string, hint?: string) => {
    const known =
      (taskId
        ? (db.getTask(app.db, taskId)?.provider ?? siblingProvider(app, taskId))
        : undefined) ?? hint;
    const name = pickTaskProvider(
      app.registry.taskProviderNames(),
      values.provider as string | undefined,
      known,
    );
    return { name, provider: app.registry.tasks(name) };
  };
  const unsupported = (name: string, action: string): never => {
    throw new Error(`${name} cannot ${action} tasks`);
  };

  switch (action) {
    case "new": {
      if (!target || rest.length === 0)
        throw new Error("usage: shepherd task new <project> <title...>");
      const project = app.projectConfigs.get(projectId(target)) ?? app.projectConfigs.get(target);
      if (!project) throw new Error(`unknown project "${target}"`);
      // a project that has ever synced a task knows its own tracker; without that hint
      // two registered trackers would need --provider for every single `task new`
      const seen = db.listTasks(app.db, projectId(project.name)).find((t) => t.provider)?.provider;
      const { name, provider } = providerFor(undefined, seen);
      if (!provider.createTask) return void unsupported(name, "create");
      const task = await provider.createTask({
        title: rest.join(" "),
        description: body,
        taskProviderProjectId: project.task_project,
      });
      return out(`${task.id}  ${task.title}${task.url ? `\n${task.url}` : ""}`);
    }
    case "show": {
      if (!target) throw new Error("usage: shepherd task show <task-id>");
      const { provider } = providerFor(target);
      const task = await provider.getTask(target);
      return out(
        [
          `${task.id}  ${task.title}`,
          `status: ${task.status}`,
          task.url ? `url: ${task.url}` : undefined,
          task.description ? `\n${task.description}` : undefined,
        ]
          .filter(Boolean)
          .join("\n"),
      );
    }
    case "edit": {
      if (!target)
        throw new Error("usage: shepherd task edit <task-id> [title...] [--body <text>]");
      const title = rest.length ? rest.join(" ") : undefined;
      if (title === undefined && body === undefined)
        throw new Error("nothing to change: pass a title or --body");
      const { name, provider } = providerFor(target);
      if (!provider.editTask) return void unsupported(name, "edit");
      await provider.editTask(target, { title, description: body });
      return out(`${target} updated`);
    }
    case "status": {
      const status = rest[0] as TaskStatus | undefined;
      if (!target || !status || !TASK_STATUSES.includes(status))
        throw new Error(`usage: shepherd task status <task-id> <${TASK_STATUSES.join("|")}>`);
      const { provider } = providerFor(target);
      await provider.updateStatus(target, status);
      return out(`${target} -> ${status}`);
    }
    case "comment": {
      if (!target || rest.length === 0)
        throw new Error("usage: shepherd task comment <task-id> <body...>");
      const { provider } = providerFor(target);
      await provider.addComment(target, rest.join(" "));
      return out(`comment added to ${target}`);
    }
    case "rm": {
      if (!target) throw new Error("usage: shepherd task rm <task-id>");
      const { name, provider } = providerFor(target);
      if (!provider.archiveTask) return void unsupported(name, "archive");
      await provider.archiveTask(target);
      return out(`${target} archived`);
    }
    default:
      out(USAGE);
      process.exitCode = 1;
      return;
  }
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
      const bin = /\/(node|bun)$/.test(process.execPath) ? process.argv[1]! : process.execPath;
      const path = await daemon.install(bin);
      const linger = daemon.lingerHint();
      return out(
        `service installed: ${path}\nlog: ${daemon.logPath()}` + (linger ? `\n${linger}` : ""),
      );
    }
    case "uninstall":
      await daemon.uninstall();
      return out("service removed");
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
        `service: ${daemon.installed() ? "installed" : "not installed"}\n` +
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
