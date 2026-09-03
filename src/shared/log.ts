const ANSI = /\u001B\[[0-9;]*m/g;

/**
 * One line out of an error. Command-line tools answer failures with pages of help text,
 * and logging that verbatim every minute is how a log file reaches six megabytes.
 */
export function briefError(err: unknown, limit = 200): string {
  const raw = String((err as { message?: unknown })?.message ?? err).replace(ANSI, "");
  const line =
    raw
      .split("\n")
      .map((s) => s.trim())
      .find(Boolean) ?? "unknown error";
  return line.length > limit ? `${line.slice(0, limit)}\u2026` : line;
}
