// ponytail: launchd does the daemonizing, no supervisor of our own.
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
const plistPath = () => join(homedir(), "Library", "LaunchAgents", `${LABEL}.plist`);
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

export async function install(bin: string): Promise<string> {
  const path = plistPath();
  mkdirSync(dirname(path), { recursive: true });
  mkdirSync(stateDir(), { recursive: true });
  writeFileSync(path, plist(bin));
  await exec("launchctl", ["bootout", `${domain()}/${LABEL}`]).catch(() => {}); // in case of a reinstall
  await exec("launchctl", ["bootstrap", domain(), path]);
  return path;
}

export async function uninstall(): Promise<void> {
  await exec("launchctl", ["bootout", `${domain()}/${LABEL}`]).catch(() => {});
  rmSync(plistPath(), { force: true });
}

export const start = () => exec("launchctl", ["kickstart", "-k", `${domain()}/${LABEL}`]);
export const stop = () => exec("launchctl", ["bootout", `${domain()}/${LABEL}`]);
export const installed = () => existsSync(plistPath());
