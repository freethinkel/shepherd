import { icon } from "../../shared/constants/status-icon.ts";
import type { ProjectView } from "../../shared/view.ts";

export { icon };

export function table(headers: string[], rows: (string | number)[][]): string {
  const all = [headers, ...rows.map((r) => r.map(String))];
  const widths = headers.map((_, i) => Math.max(...all.map((r) => (r[i] ?? "").length)));
  return all
    .map((row) =>
      row
        .map((cell, i) => String(cell).padEnd(widths[i]!))
        .join("  ")
        .trimEnd(),
    )
    .join("\n");
}

/** The main screen: projects, tasks, agents, statuses. */
export function projectsTree(views: ProjectView[]): string {
  const out: string[] = ["PROJECTS"];
  for (const view of views) {
    const summary = [
      view.counts.working ? `${view.counts.working} running` : "",
      view.counts.blocked ? `${view.counts.blocked} blocked` : "",
      view.counts.review ? `${view.counts.review} in review` : "",
      view.counts.queued ? `${view.counts.queued} queued` : "",
    ]
      .filter(Boolean)
      .join(", ");
    out.push(
      `${icon(view.status === "idle" ? "queued" : "working")} ${view.project.name}${summary ? `  (${summary})` : ""}`,
    );
    view.tasks.forEach((task, i) => {
      const branch = i === view.tasks.length - 1 ? "└──" : "├──";
      const agent = task.run ? task.run.agentKind : "—";
      out.push(
        `  ${branch} ${icon(task.status)} ${task.task.id.padEnd(8)} ${task.task.title.slice(0, 28).padEnd(28)} ${agent.padEnd(8)} ${task.status}`,
      );
    });
    if (view.tasks.length === 0) out.push("  └── (no tasks)");
  }
  return out.join("\n");
}
