---
name: elbmesh-operations
description: Use when the Elbmesh Orchestrator needs a complete task-card GitHub Issue created or an isolated Git worktree listed, fetched, and added.
---

# Elbmesh Operations

Use this skill for narrow delivery setup that the shell-free Orchestrator cannot perform directly.

## Read First

```text
docs/GOAL.md
docs/GLOSSARY.md
docs/DEVELOPMENT_WORKFLOW.md
docs/HUMAN_DECISION_LOOP.md
docs/DELIVERY_ROADMAP.md
docs/AGENT_SKILLS.md
docs/IMPLEMENTATION_PLAN.md
docs/adr/
```

Also read the complete task-card payload, explicit issue dependencies, and exact repository/branch/base/path provenance supplied by the Orchestrator.

## Permitted Edit Surface

None. Repository Edit and nested Task delegation are denied. The only permitted shell operations are `gh issue create`, `gh issue view`, `git fetch`, `git fetch origin`, `git worktree list`, and `git worktree add`, subject to the agent's narrower flag denials.

Issue creation publishes the supplied task card only. Do not add labels or other issue metadata and do not edit, close, reopen, or otherwise mutate an existing issue. Worktree addition may materialize the requested checkout, but no file in any worktree may be modified.

## Setup Sequence

```text
Validate complete task-card or worktree provenance.
View referenced dependency issues when needed.
Create the exact task-card issue and verify its number, body, and URL when requested.
List existing worktrees before any add.
Fetch the default remote only when the supplied base needs refreshing.
Add the exact non-conflicting worktree without force or branch reset.
Return immutable setup provenance to the Orchestrator.
```

Run one command at a time. Never use separators, pipes, redirection, command substitution, scripts, interpreters, aliases, or shell functions. Stop on missing provenance, a path/branch collision, or any request outside the allowlist.

## Required Outputs

Return issue number/URL and verification when created; repository, worktree path, branch, and base revision when added; exact commands and results; collision/provenance checks; limitations; and blockers.

## Verification

No build or repository gate applies to this non-editing role. Verify issue creation with `gh issue view` and worktree creation with `git worktree list`; report exact output. The Test Writer, Implementer, Reviewer, Publisher, and CI retain their existing verification responsibilities.

## Architecture Rules Preserved

Preserve Resource, Action, Event, Reaction, and View boundaries; complete dependency-linked task cards; one branch/worktree per independent issue; tests before implementation; immutable accepted tests; separate red and green commits; append-only stage-specific issue evidence; a current pull request body; Publisher-owned publication/status transitions; and human-only merge. Never commit, push, merge, delete branches, remove worktrees, mutate pull requests, mutate existing issues, or spawn nested tasks.
