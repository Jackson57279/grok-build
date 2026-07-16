---
name: ulw-loop
description: >
  Self-referential verified completion loop until evidence proves the goal done.
  Use when the user says ulw-loop, /ulw-loop, ultrawork loop, or wants durable
  goal-driven iteration.
metadata:
  short-description: "Loop until verified completion"
argument-hint: "<task>"
---

# ulw-loop

Keep working until the goal is verified by evidence, not by hopeful status.

**Task:** $ARGUMENTS

## Bootstrap

1. Parse the task.
2. Create `.grok/ulw-loop/` with `brief.md` (goal + acceptance) and `ledger.jsonl`.
3. Split acceptance into checkable criteria.

## Loop

For each unmet criterion:

1. Plan the next slice
2. Implement (yourself or `general-purpose` workers via `spawn_subagent`)
3. Self-QA + spawn `reviewer` (or Manual QA through the real surface)
4. Append evidence to the ledger
5. Update the brief/criteria status

Stop only when every criterion has evidence, or after three materially different failed approaches on the same blocker (then ask one precise question).

## Notes

- Prefer `background: true` + wait over polling narration.
- Cap attempts reasonably; do not infinite-loop without new evidence.
- Resume from `.grok/ulw-loop/` on the next turn if interrupted.
