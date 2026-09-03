import type { ScrollBoxProps } from "@opentui/solid";
import { splitProps } from "solid-js";

import { useTheme } from "../../modules/theme/index.ts";

/**
 * A scrollbox whose bar follows the palette. The built-in one paints a fixed colour that
 * disappears — or turns muddy — depending on the terminal theme.
 */
export const Scroll = (props: ScrollBoxProps) => {
  const theme = useTheme();
  const [local, rest] = splitProps(props, ["scrollbarOptions"]);
  return (
    <scrollbox
      scrollbarOptions={{
        showArrows: false,
        trackOptions: { foregroundColor: theme().border, backgroundColor: "transparent" },
        ...local.scrollbarOptions,
      }}
      {...rest}
    />
  );
};
