---
name: init-deep
description: >
  Generate hierarchical AGENTS.md context so agents start from local guidance
  in a large repository. Use when the user says init-deep, /init-deep, or wants
  project memory for Grok Build agents.
metadata:
  short-description: "Seed AGENTS.md project memory"
---

# init-deep

Build durable project guidance for Grok Build agents.

## Steps

1. Map the repo: top-level layout, primary languages, build/test entrypoints.
2. Spawn parallel `explore` subagents for major packages/areas when the tree is large.
3. Write or update hierarchical `AGENTS.md` files:
   - Root `AGENTS.md` — product overview, how to build/test, critical invariants
   - Nested `AGENTS.md` under large packages — local conventions only
4. Prefer short, actionable rules over essays. Cite real paths and commands.
5. Do not invent APIs. If unsure, leave a clearly marked `TODO` for humans.

## Output

Summarize which `AGENTS.md` files were created/updated and the build/test commands recorded.
