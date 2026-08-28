import { execFile } from "node:child_process";
import { promisify } from "node:util";
import type { Change, CodeProvider, CreateChangeInput } from "../../domain/types.ts";

const exec = promisify(execFile);

/** ponytail: the gh CLI already does auth and API — no HTTP client of our own. */
const gh = async (cwd: string, args: string[]) =>
  (await exec("gh", args, { cwd, maxBuffer: 8 << 20 })).stdout.trim();

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
    const raw = await gh(repoPath, ["pr", "view", id, "--json", "number,url,state"]);
    return this.toChange(JSON.parse(raw));
  }

  async mergeChange(id: string, repoPath: string): Promise<void> {
    await gh(repoPath, ["pr", "merge", id, "--squash"]);
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
