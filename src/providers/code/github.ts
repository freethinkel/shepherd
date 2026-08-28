import { execFile } from "node:child_process";
import { promisify } from "node:util";
import type { Change, ChangeComment, CodeProvider, CreateChangeInput } from "../../domain/types.ts";

const exec = promisify(execFile);

/** ponytail: the gh CLI already does auth and API — no HTTP client of our own. */
const gh = async (cwd: string, args: string[]) =>
  (await exec("gh", args, { cwd, maxBuffer: 8 << 20 })).stdout.trim();

/**
 * `statusCheckRollup` mixes CheckRun (status + conclusion) and StatusContext (state).
 * SKIPPED and NEUTRAL do not block a merge on GitHub either, so they count as success.
 */
export function githubChecks(rollup: unknown[]): "pending" | "success" | "failure" {
  let pending = false;
  for (const item of rollup as any[]) {
    const verdict: string =
      item.__typename === "StatusContext" ? item.state : (item.conclusion ?? "");
    if (item.__typename !== "StatusContext" && item.status !== "COMPLETED") pending = true;
    else if (verdict === "PENDING" || verdict === "EXPECTED") pending = true;
    else if (!["SUCCESS", "SKIPPED", "NEUTRAL"].includes(verdict)) return "failure";
  }
  return pending ? "pending" : "success";
}

export class GitHubCodeProvider implements CodeProvider {
  readonly name = "github";

  async check(repoPath?: string): Promise<void> {
    await exec("gh", ["auth", "status"], repoPath ? { cwd: repoPath } : {}).catch(() => {
      throw new Error("gh is not authenticated: run `gh auth login`");
    });
  }

  async createChange(input: CreateChangeInput): Promise<Omit<Change, "runId">> {
    const existing = await this.findByBranch(input.repoPath, input.branch);
    if (existing) return existing;
    await gh(input.repoPath, [
      "pr",
      "create",
      "--head",
      input.branch,
      "--base",
      input.baseBranch,
      "--title",
      input.title,
      "--body",
      input.body,
    ]);
    const created = await this.findByBranch(input.repoPath, input.branch);
    if (!created) throw new Error("github: pull request was not created");
    return created;
  }

  async getChange(id: string, repoPath: string): Promise<Omit<Change, "runId">> {
    const raw = await gh(repoPath, [
      "pr",
      "view",
      id,
      "--json",
      "number,url,state,reviewDecision,statusCheckRollup",
    ]);
    const pr = JSON.parse(raw);
    return {
      ...this.toChange(pr),
      approved: pr.reviewDecision === "APPROVED",
      checks: githubChecks(pr.statusCheckRollup ?? []),
    };
  }

  async mergeChange(id: string, repoPath: string): Promise<void> {
    await gh(repoPath, ["pr", "merge", id, "--squash"]);
  }

  /** Line comments, review summaries and plain comments are three endpoints on GitHub. */
  async listComments(id: string, repoPath: string): Promise<ChangeComment[]> {
    const api = (path: string) =>
      gh(repoPath, ["api", "--paginate", `repos/{owner}/{repo}/${path}`]).then(
        (raw) => JSON.parse(raw || "[]") as any[],
      );
    const [line, reviews, plain] = await Promise.all([
      api(`pulls/${id}/comments`),
      api(`pulls/${id}/reviews`),
      api(`issues/${id}/comments`),
    ]);
    const toComment = (c: any, path?: string, line?: number): ChangeComment => ({
      author: c.user?.login ?? "unknown",
      body: c.body ?? "",
      path,
      line,
      createdAt: new Date(c.created_at ?? c.submitted_at),
    });
    return [
      ...line.map((c) => toComment(c, c.path, c.line ?? c.original_line ?? undefined)),
      ...reviews.filter((r) => r.body?.trim()).map((r) => toComment(r)),
      ...plain.map((c) => toComment(c)),
    ].sort((a, b) => a.createdAt.getTime() - b.createdAt.getTime());
  }

  private async findByBranch(
    repoPath: string,
    branch: string,
  ): Promise<Omit<Change, "runId"> | undefined> {
    const raw = await gh(repoPath, [
      "pr",
      "list",
      "--head",
      branch,
      "--state",
      "all",
      "--limit",
      "1",
      "--json",
      "number,url,state",
    ]).catch(() => "[]");
    const [pr] = JSON.parse(raw || "[]");
    return pr ? this.toChange(pr) : undefined;
  }

  private toChange(pr: any): Omit<Change, "runId"> {
    return {
      id: String(pr.number),
      provider: this.name,
      url: pr.url,
      status: pr.state === "MERGED" ? "merged" : pr.state === "CLOSED" ? "closed" : "open",
    };
  }
}
