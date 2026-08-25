import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { normalizeAgentStatus } from "../domain/status.ts";
import type { AgentStatus } from "../domain/types.ts";

const exec = promisify(execFile);

export interface HerdrWorkspace {
  workspaceId: string;
  tabId: string;
  paneId: string;
  label: string;
}

export interface HerdrAgent {
  name: string;
  paneId: string;
  workspaceId: string;
  kind: string;
  status: AgentStatus;
  title?: string | undefined;
}

export interface HerdrEvent {
  type: "agent_status_changed" | "agent_gone";
  agent: string;
  workspaceId: string;
  status: AgentStatus;
  previous?: AgentStatus | undefined;
}

export class HerdrError extends Error {}

/** A thin wrapper over the herdr CLI. Herdr is the agent runtime; we do not duplicate it. */
export class HerdrClient {
  constructor(private readonly bin = process.env.HERDR_BIN ?? "herdr") {}

  private async call(args: string[]): Promise<any> {
    try {
      const { stdout } = await exec(this.bin, args, { maxBuffer: 8 << 20 });
      return JSON.parse(stdout).result;
    } catch (err: any) {
      const raw = String(err.stderr ?? err.message ?? err).trim();
      let message = raw;
      try {
        message = JSON.parse(raw).error?.message ?? JSON.parse(raw).error?.code ?? raw;
      } catch {}
      throw new HerdrError(`herdr ${args.join(" ")}: ${message}`);
    }
  }

  async version(): Promise<string> {
    const { stdout } = await exec(this.bin, ["--version"]);
    return stdout.trim();
  }

  async createWorkspace(input: { label: string; cwd: string }): Promise<HerdrWorkspace> {
    const r = await this.call([
      "workspace", "create", "--cwd", input.cwd, "--label", input.label, "--no-focus",
    ]);
    return {
      workspaceId: r.workspace.workspace_id,
      tabId: r.tab.tab_id,
      paneId: r.root_pane.pane_id,
      label: input.label,
    };
  }

  /** A tab in an existing workspace: workspace = task, tab = agent. */
  async createTab(input: { workspaceId: string; cwd: string; label: string }): Promise<{ tabId: string; paneId: string }> {
    const r = await this.call([
      "tab", "create", "--workspace", input.workspaceId, "--cwd", input.cwd,
      "--label", input.label, "--no-focus",
    ]);
    return { tabId: r.tab.tab_id, paneId: r.root_pane.pane_id };
  }

  async spawnAgent(input: {
    name: string; kind: string; paneId: string; timeoutMs?: number | undefined;
  }): Promise<HerdrAgent> {
    const r = await this.call([
      "agent", "start", input.name, "--kind", input.kind, "--pane", input.paneId,
      "--timeout", String(input.timeoutMs ?? 60_000),
    ]);
    const a = r.agent ?? r;
    return {
      name: input.name, paneId: a.pane_id ?? input.paneId, workspaceId: a.workspace_id ?? "",
      kind: input.kind, status: normalizeAgentStatus(a.agent_status),
    };
  }

  async prompt(agent: string, text: string): Promise<void> {
    await this.call(["agent", "prompt", agent, text]);
  }

  async getAgentStatus(agent: string): Promise<AgentStatus> {
    try {
      const r = await this.call(["agent", "get", agent]);
      return normalizeAgentStatus(r.agent?.agent_status);
    } catch {
      return "unknown"; // the agent is gone from the runtime
    }
  }

  async listAgents(): Promise<HerdrAgent[]> {
    const r = await this.call(["agent", "list"]);
    return (r.agents ?? []).map((a: any) => ({
      name: a.name ?? a.pane_id,
      paneId: a.pane_id,
      workspaceId: a.workspace_id,
      kind: a.agent,
      status: normalizeAgentStatus(a.agent_status),
      title: a.terminal_title_stripped,
    }));
  }

  async readAgent(agent: string, lines = 40): Promise<string> {
    const { stdout } = await exec(this.bin, [
      "agent", "read", agent, "--source", "recent-unwrapped", "--lines", String(lines),
    ], { maxBuffer: 8 << 20 });
    return stdout;
  }

  async stopAgent(agent: string): Promise<void> {
    await this.call(["agent", "send-keys", agent, "ctrl+c"]).catch(() => {});
  }

  async focusWorkspace(workspaceId: string): Promise<void> {
    await this.call(["workspace", "focus", workspaceId]);
  }

  async closeWorkspace(workspaceId: string): Promise<void> {
    await this.call(["workspace", "close", workspaceId]);
  }

  async workspaceExists(workspaceId: string): Promise<boolean> {
    const r = await this.call(["workspace", "list"]);
    return (r.workspaces ?? []).some((w: any) => w.workspace_id === workspaceId);
  }

  /**
   * ponytail: the herdr CLI has no event stream — poll `agent list` and yield the delta.
   * Swap for a socket subscription once herdr exposes one.
   */
  async *subscribeToEvents(intervalMs = 5_000, signal?: AbortSignal): AsyncIterable<HerdrEvent> {
    const seen = new Map<string, AgentStatus>();
    let first = true;
    while (!signal?.aborted) {
      const agents = await this.listAgents().catch(() => []);
      const alive = new Set<string>();
      for (const a of agents) {
        alive.add(a.name);
        const prev = seen.get(a.name);
        seen.set(a.name, a.status);
        if (!first && prev !== a.status) {
          yield { type: "agent_status_changed", agent: a.name, workspaceId: a.workspaceId, status: a.status, previous: prev };
        }
      }
      for (const [name, prev] of seen) {
        if (!alive.has(name)) {
          seen.delete(name);
          if (!first) yield { type: "agent_gone", agent: name, workspaceId: "", status: "unknown", previous: prev };
        }
      }
      first = false;
      await new Promise((r) => setTimeout(r, intervalMs));
    }
  }
}
