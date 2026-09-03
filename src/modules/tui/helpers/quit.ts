import type { CliRenderer } from "@opentui/core";

/**
 * Leaves the dashboard and the terminal as they were found.
 *
 * `destroy()` blocks forever while stdin is still in raw mode — it waits on input nobody will
 * send — so stdin is handed back first. Exiting the process afterwards is deliberate: a `herdr
 * read` still in flight would otherwise keep the loop alive and the terminal hostage.
 */
export const quit = (
  renderer: CliRenderer,
  busy: boolean,
  say: (message: string) => void,
  insisted = { value: false },
): void => {
  if (busy && !insisted.value) {
    // an action is mid-flight: let it write its row rather than losing it
    insisted.value = true;
    say("an action is still running — press q again to leave anyway");
    return;
  }
  try {
    process.stdin.setRawMode(false);
  } catch {
    // not a tty (a pipe, a test harness): there was no raw mode to hand back
  }
  process.stdin.pause();
  renderer.destroy();
  process.exit(0);
};
