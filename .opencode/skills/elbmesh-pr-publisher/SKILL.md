---
name: elbmesh-pr-publisher
description: Use when publishing accepted Elbmesh role handoffs as separate red and green commits on a draft pull request, then marking it ready after review without merging.
---

# Elbmesh PR Publisher

Use this skill to turn accepted Test Writer, Implementer, and Reviewer reports into an auditable draft-to-ready pull request. The Publisher changes Git and GitHub publication state but never modifies repository files.

## Read First

```text
docs/GOAL.md
docs/GLOSSARY.md
docs/DEVELOPMENT_WORKFLOW.md
docs/HUMAN_DECISION_LOOP.md
docs/PHASED_DELIVERY_PLAN.md
docs/AGENT_SKILLS.md
docs/AGENT_DELIVERY_HARNESS.md
docs/IMPLEMENTATION_PLAN.md
docs/adr/
```

## Required Inputs

```text
GitHub issue task card and issue number
reported base revision and intended issue branch
accepted Test Writer report with exact test and fixture paths and red proof
accepted Implementer report with exact implementation and documentation paths and green proof
Reviewer report with revision range, findings, CI state, and blocker status
```

## Responsibilities

```text
Keep repository files unchanged.
Verify branch, status, unstaged diff, and staged diff before every publication action.
Stage only exact paths reported by the role responsible for the current stage.
Preserve role reports and commit provenance without rewriting them.
Create a separate red test commit and green implementation/docs commit.
Link the draft pull request to its issue.
Append green and review evidence to the pull request and issue.
Mark the pull request ready only after a no-blocker review and required CI.
Return the pull request URL and publication evidence.
```

## Allowed Publication Lifecycle

1. From the reported base, create the issue branch after confirming the worktree state.
2. Stage only accepted Test Writer test and fixture paths, verify the cached diff, create the red test-only commit, and push.
3. Open a draft pull request linked to or closing the issue. Carry the Test Writer provenance and red proof in the pull request body.
4. After accepted green proof, stage only reported implementation and documentation paths, verify the cached diff, create the distinct green commit, and push.
5. Append green proof in a new pull request comment; never replace accepted red evidence.
6. After the Reviewer reports no blockers and required CI passes, append review evidence in new PR and issue comments and mark the pull request ready.
7. Return the pull request URL, branch and commit revisions, evidence links, and residual risks to the Orchestrator.

Rework repeats the Implementer, green publication, Reviewer, and readiness checks with fresh role sessions and new evidence. It does not rewrite accepted earlier commits or reports.

## Verification

Before staging:

```text
Confirm the current branch and reported base/HEAD provenance.
Inspect git status, the working-tree diff for every reported path, and the complete cached diff.
Confirm no unreported path is staged and no required path is absent from the role report.
```

After staging and before committing:

```text
Inspect git status and the complete cached diff again.
Confirm the staged path set exactly equals the current role-reported path set.
Confirm red contains only accepted tests/fixtures or green contains only reported implementation/docs.
```

After publication:

```text
Confirm the pushed branch and commit revision.
Confirm the pull request remains draft until no-blocker review and required CI.
Confirm the pull request links the issue and contains append-only role evidence.
Confirm the final ready transition and return the pull request URL.
```

## Safety Rules

```text
Do not modify files.
Do not stage unreported paths or broad pathspecs.
Do not use shell separators, pipes, redirection, command substitution, scripts, or interpreters.
Do not edit or delete previously accepted evidence.
Stop when status, diff, provenance, role paths, or gate evidence disagree.
Treat OpenCode permissions as defense in depth, not as a sandbox.
```

## Human-Only Merge

All merge operations are prohibited. Never invoke `git merge`, `gh pr merge`, auto-merge, merge APIs, UI merge actions, base-branch pushes, squash merge, rebase merge, or an equivalent path. The Publisher may mark a reviewed pull request ready and report its URL; only a human may review and merge it.
