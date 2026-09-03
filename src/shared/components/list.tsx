import type { ScrollBoxRenderable } from "@opentui/core";
import { useKeyboard } from "@opentui/solid";
import { createEffect, For } from "solid-js";

import { useTheme, type Palette } from "../../modules/theme/index.ts";
import { keepVisible } from "../helpers/keep-visible.ts";
import { Scroll } from "./scroll.tsx";
import { Text } from "./text.tsx";

export type ListItem<T> = {
  /** Status glyph or any short prefix, coloured by `color`. */
  mark?: string;
  title: string;
  /** Right-aligned trailing note: a status word, a count. */
  note?: string;
  color?: keyof Palette;
  value: T;
};

type Props<T> = {
  items: ListItem<T>[];
  index: number;
  onIndex: (index: number) => void;
  onSelect?: (value: T) => void;
  /** False while another pane owns the keyboard. */
  focused?: boolean;
};

/** One row per item, so the scroll maths is just the index. */
export const List = <T,>(props: Props<T>) => {
  const theme = useTheme();
  let scroll: ScrollBoxRenderable | undefined;

  // The list can shrink under the cursor when a sync drops a task.
  createEffect(() => {
    const last = props.items.length - 1;
    if (props.index > last) props.onIndex(Math.max(0, last));
  });
  createEffect(() => keepVisible(scroll, props.index));

  useKeyboard((key) => {
    if (props.focused === false) return;
    const last = props.items.length - 1;
    if (key.name === "down" || key.name === "j") props.onIndex(Math.min(props.index + 1, last));
    else if (key.name === "up" || key.name === "k") props.onIndex(Math.max(props.index - 1, 0));
    else if (key.name === "return") {
      const item = props.items[props.index];
      if (item) props.onSelect?.(item.value);
    }
  });

  return (
    <Scroll ref={(el: ScrollBoxRenderable) => (scroll = el)} flexGrow={1} scrollY>
      <For each={props.items}>
        {(item, i) => (
          <box
            flexDirection="row"
            flexShrink={0}
            paddingLeft={1}
            paddingRight={1}
            backgroundColor={i() === props.index ? theme().selectionBg : "transparent"}
          >
            <Text flexShrink={0} wrapMode="none" color={item.color ?? "fg"}>
              {`${item.mark ?? " "} `}
            </Text>
            {/* Wrapping would break the one-row-per-item the scroll maths relies on. */}
            <Text flexGrow={1} wrapMode="none" color={i() === props.index ? "fg" : "muted"}>
              {item.title}
            </Text>
            <Text flexShrink={0} wrapMode="none" color="muted">
              {item.note ?? ""}
            </Text>
          </box>
        )}
      </For>
    </Scroll>
  );
};
