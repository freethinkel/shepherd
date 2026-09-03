// Orchestrator domain. Knows nothing about Linear / GitHub / Codex / Herdr internals.

export type TaskStatus = "todo" | "in_progress" | "waiting_for_agent" | "in_review" | "done";

export type RunStatus =
  | "queued"
  | "starting"
  | "planning"
  | "working"
  | "blocked"
  | "validating"
  | "creating_change"
  | "review"
  | "completed"
  | "failed";

/** Normalized agent state owned by Herdr. */
export type AgentStatus = "idle" | "working" | "blocked" | "done" | "unknown";

export type ProjectStatus = "blocked" | "working" | "validating" | "review" | "queued" | "idle";

export interface Repository {
  id: string;
  path: string;
  remote?: string | undefined;
  defaultBranch: string;
}

export interface Project {
  id: string;
  name: string;
  repositoryId: string;
  taskProviderProjectId?: string | undefined;
}

export interface Task {
  id: string; // human-readable key: LIN-42
  providerId: string; // provider-internal id
  projectId: string;
  title: string;
  description?: string | undefined;
  status: TaskStatus;
  url?: string | undefined;
  /** Which registered provider this task came from, so updates go back to the same tracker. */
  provider?: string | undefined;
}

export interface AgentRun {
  id: string;
  projectId: string;
  taskId: string;
  herdrWorkspaceId: string;
  herdrAgentId: string;
  agentKind: string;
  branch: string;
  worktreePath: string;
  status: RunStatus;
  startedAt: Date;
  finishedAt?: Date | undefined;
  error?: string | undefined;
  changeId?: string | undefined;
  blockedReason?: string | undefined;
}

export interface Change {
  id: string; // "42" — PR/MR number
  runId: string;
  provider: string;
  url: string;
  status: "open" | "merged" | "closed";
  approved?: boolean | undefined;
  checks?: "pending" | "success" | "failure" | undefined;
}

export interface ChangeComment {
  author: string;
  body: string;
  path?: string | undefined;
  line?: number | undefined;
  createdAt: Date;
}

export interface TaskFilter {
  projectId?: string | undefined;
  taskProviderProjectId?: string | undefined;
  /** "me" | email | "any". Unset — no assignee filter. */
  assignee?: string | undefined;
  limit?: number | undefined;
}

export interface CreateTaskInput {
  title: string;
  description?: string | undefined;
  /** Tracker-side project name — the `task_project` of a shepherd project. */
  taskProviderProjectId?: string | undefined;
}

export interface TaskEdit {
  title?: string | undefined;
  description?: string | undefined;
}

export interface TaskProvider {
  listTasks(filter: TaskFilter): Promise<Task[]>;
  getTask(id: string): Promise<Task>;
  claimTask(id: string): Promise<void>;
  updateStatus(id: string, status: TaskStatus): Promise<void>;
  addComment(id: string, body: string): Promise<void>;
  /** Optional: only `shepherd task` needs these, the orchestrator never writes tasks. */
  createTask?(input: CreateTaskInput): Promise<Task>;
  editTask?(id: string, edit: TaskEdit): Promise<void>;
  archiveTask?(id: string): Promise<void>;
}

export interface CreateChangeInput {
  repoPath: string;
  branch: string;
  baseBranch: string;
  title: string;
  body: string;
}

export interface CodeProvider {
  /**
   * Optional preflight: throw with a human-readable reason when credentials are missing.
   * The repository path matters, because a CLI resolves the host from its remote.
   */
  check?(repoPath?: string): Promise<void>;
  createChange(input: CreateChangeInput): Promise<Omit<Change, "runId">>;
  getChange(id: string, repoPath: string): Promise<Omit<Change, "runId">>;
  mergeChange(id: string, repoPath: string): Promise<void>;
  listComments?(id: string, repoPath: string): Promise<ChangeComment[]>;
  findChange?(branch: string, repoPath: string): Promise<Omit<Change, "runId"> | undefined>;
}
