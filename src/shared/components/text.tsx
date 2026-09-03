import type { TextProps } from "@opentui/solid";
import { splitProps } from "solid-js";

import { useTheme, type Palette } from "../../modules/theme/index.ts";

type Props = Omit<TextProps, "fg" | "bg"> & {
  color?: keyof Palette;
  background?: keyof Palette;
};

/** Text that names a palette role instead of a colour, so themes stay swappable. */
export const Text = (props: Props) => {
  const theme = useTheme();
  const [local, rest] = splitProps(props, ["color", "background"]);
  return (
    <text
      fg={theme()[local.color ?? "fg"]}
      bg={local.background ? theme()[local.background] : "transparent"}
      {...rest}
    />
  );
};
