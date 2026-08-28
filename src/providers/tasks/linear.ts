import type { Task, TaskFilter, TaskProvider, TaskStatus } from "../../domain/types.ts";

const API = "https://api.linear.app/graphql";

export interface LinearState {
  id: string;
  name: string;
  type: string;
}

/** The key comes from the environment only; the config keeps no secrets. */
function apiKey(): string {
  const key = process.env.LINEAR_API_KEY;
  if (!key) throw new Error("LINEAR_API_KEY is not set");
  return key;
}

async function gql<T>(query: string, variables: Record<string, unknown> = {}): Promise<T> {
  const res = await fetch(API, {
    method: "POST",
    headers: { "content-type": "application/json", authorization: apiKey() },
    body: JSON.stringify({ query, variables }),
  });
  const body = (await res.json()) as { data?: T; errors?: { message: string }[] };
  if (body.errors?.length)
    throw new Error(`linear: ${body.errors.map((e) => e.message).join("; ")}`);
  if (!res.ok || !body.data) throw new Error(`linear: HTTP ${res.status}`);
  return body.data;
}

const ISSUE_FIELDS = `id identifier title description url state { id name type } team { id key }`;

/**
 * Orchestrator task status -> Linear workflow state.
 * Both "In Progress" and "In Review" have type `started`, so match by name first.
 */
export function targetState(status: TaskStatus, states: LinearState[]): LinearState | undefined {
  const byType = (t: string) => states.find((s) => s.type === t);
  const started = (re: RegExp) => states.find((s) => s.type === "started" && re.test(s.name));
  switch (status) {
    case "todo":
      return byType("unstarted") ?? byType("backlog");
    case "in_review":
      return started(/review/i) ?? byType("started");
    case "done":
      return byType("completed");
    default:
      return started(/progress/i) ?? byType("started");
  }
}

/**
 * An available task = the Todo column of the given project, assigned to the given person.
 * Backlog is left alone: those tasks are still being written.
 */
export function buildIssueFilter(filter: TaskFilter): Record<string, unknown> {
  const where: Record<string, unknown> = { state: { type: { eq: "unstarted" } } };
  if (filter.taskProviderProjectId) where.project = { name: { eq: filter.taskProviderProjectId } };
  if (filter.assignee === "me") where.assignee = { isMe: { eq: true } };
  else if (filter.assignee && filter.assignee !== "any")
    where.assignee = { email: { eq: filter.assignee } };
  return where;
}

export class LinearTaskProvider implements TaskProvider {
  private stateCache = new Map<string, LinearState[]>();

  async listTasks(filter: TaskFilter): Promise<Task[]> {
    const where = buildIssueFilter(filter);
    const data = await gql<{ issues: { nodes: any[] } }>(
      `query($filter: IssueFilter, $first: Int!) {
         issues(filter: $filter, first: $first) { nodes { ${ISSUE_FIELDS} } } }`,
      { filter: where, first: filter.limit ?? 25 },
    );
    return data.issues.nodes.map((n) => this.toTask(n, filter.projectId ?? ""));
  }

  async getTask(id: string): Promise<Task> {
    const data = await gql<{ issue: any }>(
      `query($id: String!) { issue(id: $id) { ${ISSUE_FIELDS} } }`,
      { id },
    );
    if (!data.issue) throw new Error(`linear: issue ${id} not found`);
    return this.toTask(data.issue, "");
  }

  /** Claiming a task; a double claim is rejected by the unique index in SQLite. */
  async claimTask(id: string): Promise<void> {
    await this.updateStatus(id, "in_progress");
  }

  async updateStatus(id: string, status: TaskStatus): Promise<void> {
    const issue = await gql<{ issue: any }>(
      `query($id: String!) { issue(id: $id) { id team { id } } }`,
      { id },
    );
    const states = await this.statesFor(issue.issue.team.id);
    const state = targetState(status, states);
    if (!state) return;
    await gql(
      `mutation($id: String!, $stateId: String!) {
         issueUpdate(id: $id, input: { stateId: $stateId }) { success } }`,
      { id: issue.issue.id, stateId: state.id },
    );
  }

  async addComment(id: string, body: string): Promise<void> {
    const issue = await gql<{ issue: any }>(`query($id: String!) { issue(id: $id) { id } }`, {
      id,
    });
    await gql(
      `mutation($issueId: String!, $body: String!) {
         commentCreate(input: { issueId: $issueId, body: $body }) { success } }`,
      { issueId: issue.issue.id, body },
    );
  }

  private async statesFor(teamId: string): Promise<LinearState[]> {
    const cached = this.stateCache.get(teamId);
    if (cached) return cached;
    const data = await gql<{ team: { states: { nodes: LinearState[] } } }>(
      `query($id: String!) { team(id: $id) { states { nodes { id name type } } } }`,
      { id: teamId },
    );
    this.stateCache.set(teamId, data.team.states.nodes);
    return data.team.states.nodes;
  }

  private toTask(n: any, projectId: string): Task {
    return {
      id: n.identifier,
      providerId: n.id,
      projectId,
      title: n.title,
      description: n.description ?? undefined,
      url: n.url,
      status:
        n.state?.type === "completed"
          ? "done"
          : n.state?.type === "started"
            ? "in_progress"
            : "todo",
    };
  }
}
