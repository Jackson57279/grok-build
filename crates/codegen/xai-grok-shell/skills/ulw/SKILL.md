---
name: ulw
description: >
  Ultrawork mode for Grok Build — parallel research, then plan, implement, and
  verify with evidence. Use when the user says ulw, ultrawork, "keep going until
  done", or wants maximum-precision outcome-first orchestration.
metadata:
  short-description: "Ultrawork — research, implement, verify"
argument-hint: "<goal>"
---

# Ultrawork (ulw)

Stay on the **Grok Build** harness. Goals, not recipes. Done means verified.

**Goal:** $ARGUMENTS

## Mode

1. Fan out research **before** edits:
   - `spawn_subagent` `explore` — local implementation map
   - `spawn_subagent` `librarian` — docs / contracts / external truth when needed
2. Optionally `spawn_subagent` `oracle` for hard design trade-offs.
3. Implement yourself or via `general-purpose` workers. Prefer parallel independent slices.
4. Before claiming done: `spawn_subagent` `reviewer` (or `/check-work`) and fix until `VERDICT: PASS`.

## Rules

- Do not mark complete on hopeful narration — attach commands, paths, and outcomes.
- Prefer `background: true` for independent research; wait when results unblock the next step.
- Brief subagents like senior engineers: goal, context you already know, acceptance criteria.
- Keep continuity under `.grok/ulw/` if the work spans turns (`brief.md` + `ledger.jsonl`).

## Related

- `/ulw-plan` — decision-complete plan only
- `/start-work` — execute a plan checklist
- `/ulw-loop` — loop until verified
- `/create-agent` — custom subagent definitions in `.grok/agents/`
