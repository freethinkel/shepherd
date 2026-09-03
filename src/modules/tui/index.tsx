import { render } from "@opentui/solid";

import type { App } from "../../core/app.ts";
import { ThemeProvider } from "../theme/index.ts";
import { Dashboard } from "./ui/app.tsx";

/** Runs until the user quits; `quit()` ends the process, so this never resolves. */
export async function run(app: App): Promise<void> {
  render(() => (
    <ThemeProvider>
      <Dashboard app={app} />
    </ThemeProvider>
  ));
  await new Promise<void>(() => {});
}
