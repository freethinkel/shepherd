// ponytail: the OS service manager does the daemonizing, no supervisor of our own.
import { execFile } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { homedir, userInfo } from "node:os";
import { dirname, join } from "node:path";
import { promisify } from "node:util";

const exec = promisify(execFile);

export const LABEL = "dev.shepherd.orchestrator";
export const stateDir = () => process.env.SHEPHERD_STATE_DIR ?? join(homedir(), ".shepherd");
export const logPath = () => join(stateDir(), "shepherd.log");
export const pidPath = () => join(stateDir(), "daemon.pid");
const mac = process.platform === "darwin";
const unitPath = () =>
  mac
    ? join(homedir(), "Library", "LaunchAgents", `${LABEL}.plist`)
    : join(homedir(), ".config", "systemd", "user", `${LABEL}.service`);
const domain = () => `gui/${userInfo().uid}`;

const isAlive = (pid: number) => {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
};

/** Who is running the orchestration loop right now, if anyone. */
export function runningPid(): number | undefined {
  const file = pidPath();
  if (!existsSync(file)) return undefined;
  const pid = Number(readFileSync(file, "utf8").trim());
  if (!pid || !isAlive(pid)) {
    rmSync(file, { force: true }); // left over from a dead process
    return undefined;
  }
  return pid;
}

/** A second orchestrator would double max_concurrent_runs, so the loop is exclusive. */
export function lockLoop(): void {
  const running = runningPid();
  if (running) throw new Error(`orchestrator already running (pid ${running})`);
  mkdirSync(stateDir(), { recursive: true });
  writeFileSync(pidPath(), String(process.pid));
  const release = () => rmSync(pidPath(), { force: true });
  process.on("exit", release);
  for (const signal of ["SIGINT", "SIGTERM"] as const) {
    process.on(signal, () => {
      release();
      process.exit(0);
    });
  }
}

function plist(bin: string): string {
  const entry = (key: string, value: string) => `  <key>${key}</key>\n  <string>${value}</string>`;
  return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
${entry("Label", LABEL)}
  <key>ProgramArguments</key>
  <array>
    <string>${bin}</string>
    <string>run</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
${entry("StandardOutPath", logPath())}
${entry("StandardErrorPath", logPath())}
${entry("WorkingDirectory", homedir())}
  <key>EnvironmentVariables</key>
  <dict>
${entry("PATH", process.env.PATH ?? "/usr/bin:/bin")}
  </dict>
</dict>
</plist>
`;
}

function unit(bin: string): string {
  return `[Unit]
Description=shepherd orchestrator

[Service]
ExecStart=${bin} run
Restart=always
RestartSec=5
WorkingDirectory=${homedir()}
Environment=PATH=${process.env.PATH ?? "/usr/bin:/bin"}
StandardOutput=append:${logPath()}
StandardError=append:${logPath()}

[Install]
WantedBy=default.target
`;
}

const launchctl = (...args: string[]) => exec("launchctl", args);
const systemctl = (...args: string[]) => exec("systemctl", ["--user", ...args]);

export async function install(bin: string): Promise<string> {
  const path = unitPath();
  mkdirSync(dirname(path), { recursive: true });
  mkdirSync(stateDir(), { recursive: true });
  writeFileSync(path, mac ? plist(bin) : unit(bin));
  if (mac) {
    await launchctl("bootout", `${domain()}/${LABEL}`).catch(() => {}); // in case of a reinstall
    await launchctl("bootstrap", domain(), path);
  } else {
    await systemctl("daemon-reload");
    await systemctl("enable", "--now", LABEL);
  }
  return path;
}

export async function uninstall(): Promise<void> {
  if (mac) await launchctl("bootout", `${domain()}/${LABEL}`).catch(() => {});
  else await systemctl("disable", "--now", LABEL).catch(() => {});
  rmSync(unitPath(), { force: true });
  if (!mac) await systemctl("daemon-reload").catch(() => {});
}

const loaded = () =>
  launchctl("print", `${domain()}/${LABEL}`)
    .then(() => true)
    .catch(() => false);

/** Starts the agent, bootstrapping it again if `stop` had unloaded it. */
export async function start(): Promise<void> {
  if (!mac) return void (await systemctl("start", LABEL));
  if (await loaded()) await launchctl("kickstart", `${domain()}/${LABEL}`);
  else await launchctl("bootstrap", domain(), unitPath());
}

export const restart = () =>
  mac ? launchctl("kickstart", "-k", `${domain()}/${LABEL}`) : systemctl("restart", LABEL);
export const stop = () =>
  mac ? launchctl("bootout", `${domain()}/${LABEL}`) : systemctl("stop", LABEL);
export const installed = () => existsSync(unitPath());

/** A user unit dies with the login session unless lingering is on; launchd has no such rule. */
export const lingerHint = () =>
  mac ? undefined : `if it must survive logout: loginctl enable-linger ${userInfo().username}`;
