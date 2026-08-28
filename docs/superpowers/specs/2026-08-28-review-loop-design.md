# Review loop and merge on approval

## Problem

A run in `review` waits for the change to be merged and for nothing else. When a
human moves the task back to Todo with review comments, `syncTasks` records
`todo` in SQLite but the run stays in `review`; `isTaskAvailable` refuses the task
because a non-failed run exists. The signal is lost and both sides wait forever.
After approval a human also has to merge by hand, although `mergeChange()` already
exists in both code providers and is never called.

## Design

### Todo while in review means "rework"

In `Workflow.checkChange`, before polling the forge: if the task's synced status is
`todo`, read the change's comments, hand them to the dev agent, and move the run
back to `working`. Everything after that is the existing path: idle → validating →
creating_change (finds the existing change) → review.

- `CodeProvider.listComments(id, repoPath)` returns
  `{ author, body, path?, line?, createdAt }[]`. GitHub: line comments from
  `gh api repos/{owner}/{repo}/pulls/N/comments`, general ones from
  `gh pr view --json comments,reviews`. GitLab: MR notes via glab or REST.
- Only comments newer than the last `ReviewRejected` event are sent, so round two
  does not re-fix round one.
- `ReviewRejected { round }` is the counter; `orchestrator.max_review_rounds`
  (default 3) caps it, after which the run fails with a comment, like validation.
- Delivery reuses `sendBack()`: prompt the agent, `transition("working")`, which
  pushes In Progress back to the tracker.
- The review agent may start once per round: `ReviewAgentStarted` must be newer
  than the last `ReviewRejected`, otherwise the change is reviewed only once.
- A dead dev agent is not handled specially: `herdr.prompt` throws, `advance`
  fails the run, the task returns to Todo and `dispatch` reopens the same branch.

Our own `fail()` also puts a task in Todo, but by then the run is `failed`, so the
rework branch cannot fire on it.

### Merge after human approval

`Change` gains `approved: boolean` and `checks: "pending" | "success" | "failure"`.
GitHub reads `reviewDecision` and `statusCheckRollup`; GitLab reads
`approved`/`approvals` and `head_pipeline.status`. In the existing
`change_poll_interval_ms` poll: `approved && checks === "success"` →
`mergeChange()`, event `ChangeMerged`. The next poll sees `merged` and completes the
run as today. `orchestrator.auto_merge` (default true) turns it off.

No LLM anywhere in this change. The decision to merge stays with the human's
approval; the machine only executes it.

## Out of scope

Answering a blocked agent with `claude -p` is a separate change with its own risk
(cost, question loops) and comes after this one.

## Tests

`workflow.test.ts`: task in Todo during review → agent receives comments and the run
returns to review; rounds exhausted → failed; approved and green → merge called;
`auto_merge = false` → not called. `policies.test.ts`: comment filtering by date and
the feedback prompt.
