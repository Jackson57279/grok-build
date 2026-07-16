---
name: start-work
description: >
  Execute a plans/<slug>.md checklist until every checkbox is done with
  verification evidence. Use when the user says start-work, /start-work, or
  wants a planned change carried to completion.
metadata:
  short-description: "Execute plan checkboxes to completion"
argument-hint: "[plan-name]"
---

# start-work

Execute a decision-complete plan until every checkbox is checked.

**Plan:** $ARGUMENTS

## Steps

1. Resolve the plan under `plans/` (or `.grok/plans/` if present). If ambiguous, list candidates and pick the best match.
2. Write or update `.grok/boulder.json` with the plan path and progress.
3. For each unchecked top-level `- [ ]` item:
   - Decompose into concrete work
   - Implement (yourself or `general-purpose` subagents)
   - Verify: diagnostics, relevant tests/build; spawn `reviewer` for independent PASS/FAIL when the slice is non-trivial
   - Append evidence to `.grok/start-work/ledger.jsonl`
   - Flip `- [ ]` → `- [x]`
4. When all checkboxes are done, print `ORCHESTRATION COMPLETE` and summarize evidence.

## Rules

- Do not mark items done without verification evidence.
- Prefer smallest correct changes.
- Use `explore` / `librarian` for research slices; keep orchestration in this session.
