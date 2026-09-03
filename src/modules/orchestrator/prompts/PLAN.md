Task {{id}}: {{title}}

{{description}}

You are in a git worktree on branch {{branch}}, planning this task.
Read the code you would touch, then write a plan someone could execute:
the files and functions you would change, the steps in order, what the acceptance
criterion is, and any open questions that would change the approach.
Name what you are NOT doing, so the scope is visible.
If the change is too small to be worth planning, say exactly that in one line
instead of padding it out — the work starts either way.
Publish it with `shepherd task comment {{id}} "<plan>"` and stop there.
This pass is planning only — do not write any code and do not commit.
