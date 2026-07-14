---
name: elbmesh-reviewer
description: Use when reviewing Elbmesh changes for bugs, architecture drift, missing tests, stale docs, and event-sourcing boundary violations.
---

# Elbmesh Reviewer

Use this skill to review completed pull request changes. `elbmesh-reviewer` is the single active final PR review role and reports merge readiness; a human performs the merge and retains all merge authority.

`elbmesh-mr-reviewer` is an optional compatibility/manual deep-review skill, not an additional required stage. It does not own or report merge readiness; only `elbmesh-reviewer` has that responsibility in the canonical delivery flow.

## Read First

```text
docs/GOAL.md
docs/GLOSSARY.md
docs/DEVELOPMENT_WORKFLOW.md
docs/HUMAN_DECISION_LOOP.md
docs/PHASED_DELIVERY_PLAN.md
docs/AGENT_SKILLS.md
docs/IMPLEMENTATION_PLAN.md
docs/adr/
```

## Review Focus

Report findings first, ordered by severity.

Check:

```text
Resource/Action/Event boundaries.
Typed errors and Receipts.
Expected version handling.
Event/journal separation.
External Operation idempotency.
Replay purity.
Reaction execution through Actions.
View rebuildability.
Missing tests.
Documentation drift.
Unplanned scope.
```

## Required PR Inspection

Inspect the task card, immutable role reports, branch range, exact changed paths, PR metadata and body, checks, and red/green/readiness evidence. Run only these exact read-only inspection and quality commands:

```text
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

Do not use wildcard or output-capable Git commands, mutate files or GitHub state, or merge. Report findings first, then the reviewed issue/branch/revision range, exact command results, residual risks, blocker status, and final PR merge readiness.

## Must Flag

```text
Action mutates multiple Resources directly.
Event belongs to no clear Resource stream.
External call is hidden in replay/apply or undeclared handler code.
Execution failure is stored as a Resource Event without domain modelling.
Generated docs or skills drift from canonical docs/manifest.
```
