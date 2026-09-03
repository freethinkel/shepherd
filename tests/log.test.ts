import assert from "node:assert/strict";
import { test } from "node:test";
import { briefError } from "../src/shared/log.ts";

const ESC = "\u001B";

test("a CLI failure becomes one line, not a page of help text", () => {
  const err = new Error(
    `Command failed: jira project list\n${ESC}[0;33mThe tool needs a Jira API token to function.\n\n` +
      "For cloud server: generate the token using this link: https://id.atlassian.com\n" +
      `Once you are done, run 'jira init'.\n${ESC}[0m`,
  );
  assert.equal(briefError(err), "Command failed: jira project list");
});

test("colour codes and blank leading lines are dropped", () => {
  assert.equal(briefError(`${ESC}[0;31m\n\n  boom  \nrest`), "boom");
  assert.equal(briefError(new Error("")), "unknown error");
});

test("a single long line is cut, not left to grow", () => {
  const brief = briefError(new Error("x".repeat(400)), 50);
  assert.equal(brief.length, 51);
  assert.ok(brief.endsWith("\u2026"));
});
