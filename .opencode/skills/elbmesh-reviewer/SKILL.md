---
name: elbmesh-reviewer
description: Use when reviewing Elbmesh changes for bugs, architecture drift, missing tests, stale docs, evidence validity, and event-sourcing boundary violations.
---

# Elbmesh Reviewer

Use this skill for the single active final pull request review. `elbmesh-reviewer` reports merge readiness or blockers; a human performs the merge and retains all merge authority.

The optional compatibility/manual `elbmesh-mr-reviewer` skill is not an additional required stage and does not own or report merge readiness.

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

Also read the expanded issue/dependencies, complete pull request range, immutable role reports, publication comments, and current checks.

## Permitted Edit Surface

None. Remain read-only and do not mutate Git, files, issues, pull requests, labels, or merge state.

## Review Focus

Report findings first by severity. Check acceptance criteria, missing tests, unplanned scope, Resource/Action/Event ownership, typed errors, replay purity, journal separation, External Operation idempotency, Reaction execution through Actions, View rebuildability, documentation/config drift, exact changed paths, and evidence validity.

## Required Outputs

Return role task/session ID, issue/branch/revision range, findings with references, exact command results, residual risks, blocker state, and final pull request merge-readiness report. A no-blocker report is not merge authority.

## Verification

Run only these exact commands:

```bash
git status --short --branch
git log --oneline --decorate origin/main..HEAD
git diff --name-status origin/main...HEAD
git diff --check origin/main...HEAD
codehud . --diff origin/main
gh pr view --json number,title,body,state,isDraft,baseRefName,headRefName,url
gh pr checks
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

## Architecture Rules Preserved

Preserve Resource/Action/Event boundaries, deterministic replay, declared External Operations, journal/Event separation, Reactions invoking Actions, rebuildable Views, immutable role evidence, read-only review, and human-only merge.
