---
name: ulw-plan
description: >
  Decision-complete planning before code. Writes plans/<slug>.md and does not
  edit product code. Use when the user says ulw-plan, /ulw-plan, plan first, or
  the work needs clear decisions before implementation.
metadata:
  short-description: "Plan only — write plans/<slug>.md"
argument-hint: "<what to build>"
---

# ulw-plan

Plan only. No product code edits.

**Goal:** $ARGUMENTS

## Steps

1. Parallel research:
   - `spawn_subagent` `explore` (codebase map)
   - `spawn_subagent` `librarian` when docs/contracts matter
2. Ask narrow clarifying questions only when a missing decision would change architecture.
3. Optional: `spawn_subagent` `oracle` for gap analysis of the draft approach.
4. Write one decision-complete plan to `plans/<slug>.md` with:
   - Goal and non-goals
   - Key decisions and why
   - Checkbox work items (`- [ ]`) ordered for execution
   - Verification / Manual QA notes per major item
5. Do not implement. Offer `/start-work` on the plan when ready.

## Continuity

Track plan path in `.grok/boulder.json` if useful for resume across turns.
