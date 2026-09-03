import { useKeyboard, useRenderer, useTerminalDimensions } from "@opentui/solid";
import {
  createEffect,
  createMemo,
  createSignal,
  For,
  onCleanup,
  onMount,
  Show,
  type Accessor,
  type JSX,
} from "solid-js";

import type { ScrollBoxRenderable } from "@opentui/core";

import type { App } from "../../../core/app.ts";
import * as actions from "../../../shared/actions.ts";
import { List, type ListItem } from "../../../shared/components/list.tsx";
import { Scroll } from "../../../shared/components/scroll.tsx";
import { Text } from "../../../shared/components/text.tsx";
import { briefError } from "../../../shared/log.ts";
import * as view from "../../../shared/view.ts";
import {
  setTerminalColors,
  setThemeMode,
  useTheme,
  watchPalette,
  type Palette,
} from "../../theme/index.ts";
import { ICONS, STATUS_COLOR } from "../constants/status.ts";
import { quit } from "../helpers/quit.ts";

const TICK_MS = 1500;
const LOG_LINES = 200;
/** Below this the two columns stop fitting: a phone over ssh gets one column and a detail screen. */
const WIDE_COLS = 100;
const KEYS =
  "j/k move · tab pane · r retry · s stop · o open · v review · x reset · S sync · q quit";
const KEYS_NARROW = "j/k · enter open · r retry · s stop · o open · q quit";
const KEYS_DETAIL = "j/k scroll · esc back · r retry · s stop · o open · q quit";

export const Dashboard = (props: { app: App }) => {
  const renderer = useRenderer();
  const dimensions = useTerminalDimensions();
  const wide = () => dimensions().width >= WIDE_COLS;
  const [projects, setProjects] = createSignal<view.ProjectView[]>([]);
  /** Only ever "detail" on a narrow terminal, where the panes cannot sit side by side. */
  const [screen, setScreen] = createSignal<"list" | "detail">("list");
  let logScroll: ScrollBoxRenderable | undefined;
  const [projectIndex, setProjectIndex] = createSignal(0);
  const [taskIndex, setTaskIndex] = createSignal(0);
  const [pane, setPane] = createSignal<"projects" | "tasks">("tasks");
  const [status, setStatus] = createSignal("");
  const [log, setLog] = createSignal("");
  const [busy, setBusy] = createSignal(false);

  const project = () => projects()[projectIndex()];
  const task = () => project()?.tasks[taskIndex()];
  const run = () => task()?.run;

  /** SQLite only: a frame never waits on herdr, the tracker or the forge. */
  const refresh = () => setProjects(view.overview(props.app.db, props.app.projectConfigs.keys()));

  onMount(() => {
    refresh();
    const timer = setInterval(refresh, TICK_MS);
    onCleanup(() => clearInterval(timer));

    void renderer.waitForThemeMode().then((mode) => mode && setThemeMode(mode));
    onCleanup(watchPalette(renderer, setTerminalColors));
  });

  // The agent's own output, pulled on its own schedule so a slow herdr cannot hold up a frame.
  createEffect(() => {
    const agent = run()?.herdrAgentId;
    if (!agent) return setLog("");
    let dropped = false;
    onCleanup(() => (dropped = true));
    void props.app.herdr
      .readAgent(agent, LOG_LINES)
      .catch(() => "")
      .then((tail) => !dropped && setLog(tail));
  });

  const detail = createMemo(() => {
    const current = run();
    return current ? view.runView(props.app.db, current.id) : undefined;
  });

  /** One action at a time: a push or a workspace teardown takes seconds and must not overlap. */
  const act = (label: string, fn: () => Promise<string>) => {
    if (busy()) return setStatus("busy — one action at a time");
    setBusy(true);
    setStatus(`${label}…`);
    fn()
      .then(setStatus)
      .catch((err: unknown) => setStatus(`${label} failed: ${briefError(err, 200)}`))
      .finally(() => {
        setBusy(false);
        refresh();
      });
  };

  useKeyboard((key) => {
    const name = key.name?.toLowerCase();
    if (!wide() && screen() === "detail") {
      if (name === "escape") return setScreen("list");
      if (name === "j" || name === "down") return scrollLog(3);
      if (name === "k" || name === "up") return scrollLog(-3);
    }
    if (name === "return" && !wide() && screen() === "list") return setScreen("detail");
    switch (name) {
      case "q":
        return quit(renderer, busy(), setStatus);
      case "c":
        return key.ctrl ? quit(renderer, busy(), setStatus) : undefined;
      case "tab":
        return setPane((p) => (p === "projects" ? "tasks" : "projects"));
      case "r":
        return act("retry", () => actions.retry(props.app, run()?.id));
      case "o":
        return act("open", () => actions.open(props.app, run()?.id));
      case "v":
        return act("review", () => actions.review(props.app, run()?.id, () => {}));
      case "x":
        return act("reset", () => actions.reset(props.app, task()?.task.id, false, () => {}));
      case "s":
        // shift+s is the only key that goes to the network
        return key.shift
          ? act("sync", async () => {
              await props.app.scheduler.syncProjects();
              await props.app.scheduler.syncTasks();
              return "synced";
            })
          : act("stop", () => actions.stop(props.app, run()?.id));
      default:
        return undefined;
    }
  });

  const scrollLog = (lines: number) => {
    if (logScroll) logScroll.scrollTop = Math.max(0, logScroll.scrollTop + lines);
  };

  const projectItems = (): ListItem<number>[] =>
    projects().map((p, i) => ({
      mark: ICONS[p.status === "idle" ? "queued" : "working"],
      title: p.project.name,
      note: [p.counts.working && `${p.counts.working}▶`, p.counts.review && `${p.counts.review}◍`]
        .filter(Boolean)
        .join(" "),
      color: p.attention ? "danger" : "fg",
      value: i,
    }));

  const taskItems = (): ListItem<number>[] =>
    (project()?.tasks ?? []).map((t, i) => ({
      mark: ICONS[t.status],
      title: `${t.task.id}  ${t.task.title}`,
      color: STATUS_COLOR[t.status],
      value: i,
    }));

  const ProjectsPane = (props: { height?: number }) => (
    <Panel title="Projects" height={props.height}>
      <List
        items={projectItems()}
        index={projectIndex()}
        onIndex={(i) => {
          setProjectIndex(i);
          setTaskIndex(0);
        }}
        focused={pane() === "projects" && screen() === "list"}
      />
    </Panel>
  );

  const TasksPane = () => (
    <Panel title="Tasks" grow>
      <List
        items={taskItems()}
        index={taskIndex()}
        onIndex={setTaskIndex}
        focused={pane() === "tasks" && screen() === "list"}
      />
    </Panel>
  );

  const AgentLog = () => (
    <Panel title="Agent log" grow>
      <Scroll
        ref={(el: ScrollBoxRenderable) => (logScroll = el)}
        flexGrow={1}
        scrollY
        stickyScroll
        stickyStart="bottom"
      >
        <Text wrapMode="none" color="muted">
          {log() || "no agent output"}
        </Text>
      </Scroll>
    </Panel>
  );

  return (
    <box flexDirection="column" flexGrow={1}>
      <box flexDirection="row" flexShrink={0} paddingLeft={1} paddingRight={1}>
        <Text flexGrow={1} wrapMode="none" color="accent">
          shepherd
        </Text>
        <Show when={wide()}>
          <Text flexShrink={0} wrapMode="none" color="muted">
            {`${projects().length} projects`}
          </Text>
        </Show>
      </box>

      <Show
        when={wide()}
        fallback={
          <Show
            when={screen() === "list"}
            fallback={
              <box flexDirection="column" flexGrow={1}>
                <Panel title="Run" grow>
                  <RunDetail detail={detail()} task={task()} events={3} />
                </Panel>
                <AgentLog />
              </box>
            }
          >
            <box flexDirection="column" flexGrow={1}>
              {/* one project needs no picker, and a phone has no rows to spare */}
              <Show when={projects().length > 1}>
                <ProjectsPane height={6} />
              </Show>
              <TasksPane />
            </box>
          </Show>
        }
      >
        <box flexDirection="row" flexGrow={1}>
          <box flexDirection="column" width={36}>
            <ProjectsPane height={10} />
            <TasksPane />
          </box>
          <box flexDirection="column" flexGrow={1}>
            <Panel title="Run" height={16}>
              <RunDetail detail={detail()} task={task()} events={6} />
            </Panel>
            <AgentLog />
          </box>
        </box>
      </Show>

      <box flexDirection="row" flexShrink={0} paddingLeft={1} paddingRight={1}>
        <Text flexGrow={1} wrapMode="none" color={busy() ? "warning" : "muted"}>
          {status() || (wide() ? KEYS : screen() === "detail" ? KEYS_DETAIL : KEYS_NARROW)}
        </Text>
      </box>
    </box>
  );
};

const Panel = (props: {
  title: string;
  height?: number | undefined;
  grow?: boolean;
  children: JSX.Element;
}) => {
  const theme = useTheme();
  return (
    <box
      border
      title={props.title}
      borderColor={theme().border}
      titleColor={theme().muted}
      flexDirection="column"
      flexGrow={props.grow ? 1 : 0}
      height={props.height ?? "auto"}
    >
      {props.children}
    </box>
  );
};

const RunDetail = (props: {
  detail: view.RunDetail | undefined;
  task: view.TaskView | undefined;
  /** A narrow screen has no rows to spare for the whole journal. */
  events: number;
}) => (
  <Show
    when={props.detail}
    fallback={
      <Text color="muted">
        {props.task ? `${props.task.status} — no run yet` : "nothing selected"}
      </Text>
    }
  >
    {(d: Accessor<view.RunDetail>) => (
      <box flexDirection="column" paddingLeft={1} paddingRight={1} overflow="hidden">
        <Field label="task">{`${d().task?.id ?? ""}  ${d().task?.title ?? ""}`}</Field>
        <Field label="run" color={STATUS_COLOR[d().run.status]}>
          {`${ICONS[d().run.status]} ${d().run.status}   attempt on ${d().run.branch}`}
        </Field>
        <Field label="agent">{`${d().run.agentKind}  ${d().run.herdrAgentId || "—"}`}</Field>
        <Show when={d().change}>
          {(change: Accessor<NonNullable<view.RunDetail["change"]>>) => (
            <>
              <Field label="change" color={change().approved ? "success" : "warning"}>
                {`#${change().id}  ${change().status}  ${
                  change().approved ? "approved" : "not approved"
                }  checks ${change().checks ?? "—"}`}
              </Field>
              <Field label="">{change().url}</Field>
            </>
          )}
        </Show>
        <Show when={d().run.blockedReason}>
          {(reason: Accessor<string>) => (
            <Field label="blocked" color="warning">
              {reason()}
            </Field>
          )}
        </Show>
        <Show when={d().run.error}>
          {(error: Accessor<string>) => (
            <Field label="error" color="danger">
              {error()}
            </Field>
          )}
        </Show>
        <For each={d().events.slice(0, props.events)}>
          {(event) => (
            <Field label="">{`${event.at.slice(11, 19)}  ${event.type}  ${event.data ?? ""}`}</Field>
          )}
        </For>
      </box>
    )}
  </Show>
);

const Field = (props: { label: string; color?: keyof Palette; children: JSX.Element }) => (
  <box flexDirection="row" flexShrink={0}>
    <Text flexShrink={0} wrapMode="none" color="muted">
      {props.label.padEnd(7)}
    </Text>
    <Text flexGrow={1} wrapMode="none" color={props.color ?? "fg"}>
      {props.children}
    </Text>
  </box>
);
