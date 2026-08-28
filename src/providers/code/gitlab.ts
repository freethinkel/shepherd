import { execFile } from "node:child_process";
import { promisify } from "node:util";
import type { Change, CodeProvider, CreateChangeInput } from "../../domain/types.ts";

const exec = promisify(execFile);

/** group/subgroup/repo out of any remote form GitLab accepts. */
export function projectPathFromRemote(remote: string): string {
  return remote
    .trim()
    .replace(/^git@[^:]+:/, "")
    .replace(/^ssh:\/\/git@[^/]+\//, "")
    .replace(/^https?:\/\/[^/]+\//, "")
    .replace(/\.git$/, "")
    .replace(/^\/+|\/+$/g, "");
}

/**
 * GitLab merge requests through `glab`, which already holds the credentials,
 * falling back to the REST API with GITLAB_TOKEN when glab is missing or logged out.
 * Set transport = "token" in [code_providers.gitlab] to skip glab entirely.
 */
export class GitLabCodeProvider implements CodeProvider {
  readonly name = "gitlab";
  /** Keyed by repository: glab picks the host from the remote, so login differs per repo. */
  private readonly glabReady = new Map<string, Promise<boolean>>();

  constructor(private readonly settings: Record<string, unknown> = {}) {}

  async check(repoPath?: string): Promise<void> {
    if (await this.useGlab(repoPath)) return;
    this.token(); // throws with the "glab auth login or GITLAB_TOKEN" hint
  }

  async createChange(input: CreateChangeInput): Promise<Omit<Change, "runId">> {
    const existing = await this.findByBranch(input.repoPath, input.branch);
    if (existing) return existing; // one change per run, never a duplicate

    if (await this.useGlab(input.repoPath)) {
      await this.glab(input.repoPath, [
        "mr",
        "create",
        "--source-branch",
        input.branch,
        "--target-branch",
        input.baseBranch,
        "--title",
        input.title,
        "--description",
        input.body,
        ...(this.settings.squash === false ? [] : ["--squash-before-merge"]),
        ...(this.settings.remove_source_branch === false ? [] : ["--remove-source-branch"]),
        "--yes",
      ]);
      const created = await this.findByBranch(input.repoPath, input.branch);
      if (!created) throw new Error("gitlab: merge request was not created");
      return created;
    }

    const project = await this.project(input.repoPath);
    return this.toChange(
      await this.api(`/projects/${project}/merge_requests`, {
        method: "POST",
        body: JSON.stringify({
          source_branch: input.branch,
          target_branch: input.baseBranch,
          title: input.title,
          description: input.body,
          remove_source_branch: this.settings.remove_source_branch ?? true,
          squash: this.settings.squash ?? true,
          ...(this.settings.merge_request_defaults as object),
        }),
      }),
    );
  }

  async getChange(id: string, repoPath: string): Promise<Omit<Change, "runId">> {
    if (await this.useGlab(repoPath)) {
      return this.toChange(JSON.parse(await this.glab(repoPath, ["mr", "view", id, "-F", "json"])));
    }
    const project = await this.project(repoPath);
    return this.toChange(await this.api(`/projects/${project}/merge_requests/${id}`));
  }

  async mergeChange(id: string, repoPath: string): Promise<void> {
    if (await this.useGlab(repoPath)) {
      await this.glab(repoPath, [
        "mr",
        "merge",
        id,
        "--yes",
        ...(this.settings.squash === false ? [] : ["--squash"]),
      ]);
      return;
    }
    const project = await this.project(repoPath);
    await this.api(`/projects/${project}/merge_requests/${id}/merge`, {
      method: "PUT",
      body: JSON.stringify({ squash: this.settings.squash ?? true }),
    });
  }

  /**
   * glab is used when it is installed and logged in for this repository's host.
   * `glab auth status` run anywhere else checks gitlab.com, which says nothing
   * about a self-hosted instance.
   */
  private useGlab(repoPath?: string): Promise<boolean> {
    if (this.settings.transport === "token") return Promise.resolve(false);
    const key = repoPath ?? "";
    let ready = this.glabReady.get(key);
    if (!ready) {
      ready = exec("glab", ["auth", "status"], repoPath ? { cwd: repoPath } : {})
        .then(() => true)
        .catch(() => false);
      this.glabReady.set(key, ready);
    }
    return ready;
  }

  private async glab(cwd: string, args: string[]): Promise<string> {
    const repo = this.settings.project ? ["-R", String(this.settings.project)] : [];
    const { stdout } = await exec("glab", [...args, ...repo], { cwd, maxBuffer: 8 << 20 });
    return stdout.trim();
  }

  private token(): string {
    const value = process.env.GITLAB_TOKEN;
    if (!value) {
      throw new Error("no GitLab credentials: run `glab auth login` or set GITLAB_TOKEN");
    }
    return value;
  }

  /** Project path from settings, otherwise from the repository's own remote. */
  private async project(repoPath: string): Promise<string> {
    if (this.settings.project) return encodeURIComponent(String(this.settings.project));
    const { stdout } = await exec("git", ["-C", repoPath, "remote", "get-url", "origin"]);
    const path = projectPathFromRemote(stdout);
    if (!path) throw new Error(`cannot infer GitLab project from remote ${stdout.trim()}`);
    return encodeURIComponent(path);
  }

  private async api(path: string, init: RequestInit = {}): Promise<any> {
    const base = String(this.settings.url ?? "https://gitlab.com").replace(/\/$/, "");
    const res = await fetch(`${base}/api/v4${path}`, {
      ...init,
      headers: {
        "content-type": "application/json",
        "private-token": this.token(),
        ...init.headers,
      },
    });
    if (!res.ok) {
      throw new Error(`gitlab ${path}: HTTP ${res.status} ${(await res.text()).slice(0, 200)}`);
    }
    return res.json();
  }

  private async findByBranch(
    repoPath: string,
    branch: string,
  ): Promise<Omit<Change, "runId"> | undefined> {
    if (await this.useGlab(repoPath)) {
      const raw = await this.glab(repoPath, [
        "mr",
        "list",
        "--source-branch",
        branch,
        "--all",
        "-P",
        "1",
        "-F",
        "json",
      ]).catch(() => "[]");
      const [mr] = JSON.parse(raw || "[]");
      return mr ? this.toChange(mr) : undefined;
    }
    const project = await this.project(repoPath);
    const [mr] = await this.api(
      `/projects/${project}/merge_requests?source_branch=${encodeURIComponent(branch)}&per_page=1`,
    );
    return mr ? this.toChange(mr) : undefined;
  }

  private toChange(mr: any): Omit<Change, "runId"> {
    return {
      id: String(mr.iid),
      provider: this.name,
      url: mr.web_url,
      status: mr.state === "merged" ? "merged" : mr.state === "closed" ? "closed" : "open",
    };
  }
}
