import { execFile } from "node:child_process";
import { existsSync } from "node:fs";
import { promisify } from "node:util";
import { expandPath } from "../config/schema.ts";
import type { Repository } from "../domain/types.ts";

const exec = promisify(execFile);

const git = async (cwd: string, args: string[]) =>
  (await exec("git", ["-C", cwd, ...args], { maxBuffer: 8 << 20 })).stdout.trim();

/** Git is the source of truth for code. We only resolve the repo and drive branches. */
export async function resolveRepository(pathLike: string): Promise<Repository> {
  const path = expandPath(pathLike);
  if (!existsSync(path)) throw new Error(`repository not found: ${path}`);
  const root = await git(path, ["rev-parse", "--show-toplevel"]);
  const remote = await git(root, ["remote", "get-url", "origin"]).catch(() => undefined);
  const defaultBranch = await git(root, ["symbolic-ref", "--short", "refs/remotes/origin/HEAD"])
    .then((r) => r.replace(/^origin\//, ""))
    .catch(() => git(root, ["rev-parse", "--abbrev-ref", "HEAD"]));
  return { id: root, path: root, remote, defaultBranch };
}

export function branchName(taskId: string, title: string): string {
  const slug = title
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "")
    .slice(0, 40);
  return `agent/${taskId.toLowerCase()}${slug ? `-${slug}` : ""}`.replace(/-$/, "");
}

/** git errors are not swallowed: zero commits and a broken path are different problems. */
export async function commitCount(worktree: string, base: string): Promise<number> {
  const range = await git(worktree, ["rev-parse", "--verify", "--quiet", `origin/${base}`])
    .then(() => `origin/${base}..HEAD`)
    .catch(() => `${base}..HEAD`);
  return Number(await git(worktree, ["rev-list", "--count", range])) || 0;
}

/** Drops the local branch. The remote one is left alone: a change may already point at it. */
export async function deleteBranch(repoRoot: string, branch: string): Promise<void> {
  await git(repoRoot, ["branch", "-D", branch]).catch(() => {});
}

export const isDirty = async (worktree: string) =>
  (await git(worktree, ["status", "--porcelain"])).length > 0;

export async function pushBranch(worktree: string, branch: string): Promise<void> {
  await git(worktree, ["push", "--force-with-lease", "--set-upstream", "origin", branch]);
}

/** Validation: the configured command, run inside the worktree. */
export async function runCommand(
  cwd: string,
  command: string,
  timeoutMs = 15 * 60_000,
): Promise<{ ok: boolean; output: string }> {
  try {
    const { stdout, stderr } = await exec("/bin/sh", ["-lc", command], {
      cwd,
      timeout: timeoutMs,
      maxBuffer: 8 << 20,
    });
    return { ok: true, output: `${stdout}${stderr}`.trim() };
  } catch (err: any) {
    return { ok: false, output: `${err.stdout ?? ""}${err.stderr ?? err.message ?? ""}`.trim() };
  }
}
