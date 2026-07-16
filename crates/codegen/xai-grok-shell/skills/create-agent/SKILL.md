---
name: create-agent
description: >
  Interactively create a custom Grok subagent definition (.grok/agents/<name>.md).
  Use when the user wants a custom agent, custom subagent, /create-agent, or to
  extend spawn_subagent types beyond built-ins.
metadata:
  short-description: "Create a custom .grok/agents definition"
---

# Create Agent

Create a custom agent definition that appears in `spawn_subagent`'s `subagent_type` roster.

## Built-ins (do not recreate)

`general-purpose`, `explore`, `plan`, `librarian`, `oracle`, `reviewer`

Project `.grok/agents/<same-name>.md` can **shadow** a built-in. User-level `~/.grok/agents/` cannot.

## Step 1: Gather information

Ask one at a time:

1. **Name** — kebab-case, 2–64 chars (e.g. `frontend-polish`)
2. **Scope** — Project (`.grok/agents/`) recommended, or User (`~/.grok/agents/`)
3. **Role** — what this agent is for, and when the parent should spawn it
4. **Tools** — full access, read-only, or a short allowlist

## Step 2: Write the file

Create `.grok/agents/<name>.md` (or `~/.grok/agents/<name>.md`):

```markdown
---
name: frontend-polish
description: UI polish agent — visual QA, spacing, and interaction fixes.
promptMode: extend
permissionMode: default
tools:
  - read_file
  - grep
  - list_dir
  - search_replace
  - run_terminal_command
discoverSkills: true
inheritSkills: true
---

You are a frontend polish specialist for this repo.
Focus on visual consistency and interaction quality.
Return absolute paths and a short before/after summary.
```

### Useful frontmatter

| Field | Notes |
|-------|--------|
| `description` | Shown in the spawn roster — make it trigger-friendly |
| `permissionMode` | `plan` for read-only planners |
| `tools` / `disallowedTools` | Allowlist or denylist |
| `model` | slug or `inherit` |
| `isolation` | `worktree` for risky edits |
| `promptMode` | `extend` (default) stacks on Grok Build; `full` replaces |

## Step 3: Verify

Confirm the file parses (valid YAML frontmatter + body). Tell the user they can spawn it with:

`spawn_subagent(subagent_type="<name>", ...)`

Manage agents in the UI via `/config-agents` (alias `/agents`).
