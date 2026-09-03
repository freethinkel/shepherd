import assert from "node:assert/strict";
import { test } from "node:test";
import {
  deriveProjectStatus,
  needsAttention,
  normalizeAgentStatus,
  taskStatusForRun,
} from "../src/shared/domain/status.ts";

test("normalizes the Herdr vocabulary", () => {
  assert.equal(normalizeAgentStatus("blocked"), "blocked");
  assert.equal(normalizeAgentStatus("weird"), "unknown");
  assert.equal(normalizeAgentStatus(undefined), "unknown");
});

test("project status follows the priority order", () => {
  assert.equal(deriveProjectStatus(["working", "blocked", "completed"], 2), "blocked");
  assert.equal(deriveProjectStatus(["working"], 0), "working");
  assert.equal(deriveProjectStatus(["creating_change"], 0), "validating");
  assert.equal(deriveProjectStatus(["completed"], 3), "queued");
  assert.equal(deriveProjectStatus([], 0), "idle");
});

test("run status maps back to the tracker", () => {
  assert.equal(taskStatusForRun("blocked"), "waiting_for_agent");
  assert.equal(taskStatusForRun("review"), "in_review");
  assert.equal(taskStatusForRun("completed"), "done");
  assert.equal(taskStatusForRun("failed"), "todo", "a failed run hands the task back");
});

test("waiting for review is not a problem to flag", () => {
  assert.equal(needsAttention("blocked"), true);
  assert.equal(needsAttention("failed"), true);
  assert.equal(needsAttention("review"), false);
});
