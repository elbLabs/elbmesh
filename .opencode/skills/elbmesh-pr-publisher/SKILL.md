---
name: elbmesh-pr-publisher
description: Use when publishing accepted Elbmesh role handoffs as separate red and green commits, automating issue status, and marking a reviewed pull request ready without merging.
---

# Elbmesh PR Publisher

Use this skill to turn accepted Test Writer, Implementer, and Reviewer reports into an auditable draft-to-ready pull request. The Publisher changes Git/GitHub publication state but never modifies repository files.

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

Also read `docs/AGENT_DELIVERY_HARNESS.md`, the expanded issue/dependencies, and immutable reports from every completed role.

## Required Inputs

Require issue/branch/base provenance, exact role-reported paths, accepted red/green proof, Reviewer findings/blocker state, required CI state, and every exposed role task/session ID.

## Permitted Edit Surface

No repository file edits. Publication is limited to exact-path Git staging/commit/push and narrowly allowed issue/pull-request operations. Never use a shell path to author or rewrite files.

## Allowed Publication Lifecycle

1. Verify branch/base/head, status, working diff, cached diff, and exact role path set.
2. Stage only accepted test/fixture paths, create/push a separate red test-only commit, open a linked draft pull request, append red evidence, and automatically set or keep `status:implementation`.
3. Stage only Implementer-reported implementation/documentation paths, create/push the distinct green commit, and append cumulative green evidence.
4. Only after no-blocker Reviewer evidence and required CI pass, append readiness evidence, change the issue to `status:review` while marking the pull request ready, and return its URL.

Green and readiness evidence is append-only: append new comments on both the GitHub issue and pull request without rewriting prior evidence. Cumulative evidence includes role task IDs, role session IDs, exact changed paths, red commit SHA, green commit SHA, exact commands, command results, review task ID, blocker status, CI state, residual risks, and PR URL.

## Required Outputs

Return issue/branch/base/head, separate red and green commit SHAs, linked pull request, status result, append-only evidence links, ready state, PR URL, exact publication commands/results, residual risks, and blockers.

## Verification

Use only commands allowed by the agent frontmatter, including exact-path variants of:

```bash
git status --short --branch
git diff -- <reported-path>
git diff --cached -- <reported-path>
gh issue view <issue>
gh pr view <pr>
gh pr checks <pr>
```

## Architecture Rules Preserved

Preserve role-reported Resource/Action/Event/Reaction/View paths and architecture evidence without authorship; keep accepted tests immutable and red/green commits separate; retain append-only evidence; and never merge, enable auto-merge, push the base branch, use broad staging, or bypass declared External Operation and journal boundaries. Only a human may review and merge.
