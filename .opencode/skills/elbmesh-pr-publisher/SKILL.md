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

Require issue/branch/base provenance, exact role-reported paths, accepted red/green proof, Reviewer findings/blocker state and complete Human Review Briefing, required CI state, and every exposed role task/session ID.

## Permitted Edit Surface

No repository file edits. Publication is limited to exact-path Git staging/commit/push and issue/pull-request operations allowed by the agent frontmatter. Never use a shell path to author or rewrite files.

The broad `gh issue edit *` permission is intentionally accepted for autonomous publication, but operational behavior remains restricted to the exact paired status commands below. OpenCode permissions are defense in depth, not a sandbox. GitHub branch protection, required CI, and independent review are the hard boundary for repository acceptance.

The human explicitly accepts the residual risk of wrong issue mutation created by broad issue-edit autonomy. The mandatory issue provenance preflight reduces that residual risk but cannot eliminate it.

## Allowed Publication Lifecycle

1. Before any push or GitHub mutation, complete a provenance preflight: verify that the current branch is non-`main` and exactly matches the branch reported in task-card provenance, verify that the pull request head matches that same branch, and verify that the target issue matches the issue task-card provenance. Stop on any branch, pull-request, issue, or other provenance mismatch.
2. Verify base/head, status, working diff, cached diff, and exact role path set.
3. Stage only accepted test/fixture paths, create/push a separate red test-only commit, open a linked draft pull request, append a red stage delta to the issue, create the current pull request summary, and automatically set or keep `status:implementation`.
4. Stage only Implementer-reported implementation/documentation paths, create/push the distinct green commit, append a green stage delta to the issue, and refresh the current pull request summary.
5. Only after no-blocker Reviewer evidence and required CI pass, append a readiness stage delta to the issue, refresh the current pull request summary, change the issue to `status:review` while marking the pull request ready, and return its URL.

Issue evidence is append-only and stage-specific. Append one stage delta as a new issue comment for red, green, rework, or readiness. A stage delta is not cumulative: do not repeat prior-stage evidence. Include only the current stage's role task IDs, role session IDs, exact changed paths, stage commit SHA, exact commands and concise command results, blocker status, and PR URL. Readiness also includes the Reviewer task ID, reviewed range, findings, CI state, and residual risks.

Keep one concise pull request body as the current human review summary and update it in place at every publication stage. Report current state, scope, changed paths, red and green commits, current head, verification summary, review and CI state, blockers, residual risks, and links to the issue audit trail. Replace stale pending fields. Do not post routine evidence comments on the pull request; pull request comments are reserved for human review discussion and actionable findings.

At readiness, publish the Reviewer-validated `Human Review Briefing` verbatim at the top of the current pull request body. Preserve its explanation, Mermaid graph, review order, risks, and approval criteria without adding technical claims. Fill the remaining current-state, commit, verification, and audit-link sections from verified Publisher evidence. Repeat this replacement after a later accepted Reviewer rework report; do not post the briefing as a pull request comment.

Exactly one of `status:implementation` and `status:review` must be active on the issue. Use only these complete paired transitions, which remove the opposite status before adding the target status; never use add-only, remove-only, simultaneous-status, arbitrary-label, or mixed issue-edit forms:

```bash
gh issue edit <issue> --remove-label status:review --add-label status:implementation
gh issue edit <issue> --remove-label status:implementation --add-label status:review
```

After accepted red publication, use the first command to set or keep `status:implementation`. Use the second command only after no-blocker Reviewer evidence and required CI pass, while marking the pull request ready. The broader permission exists only to enable autonomous publication; these paired forms remain the required operational behavior.

After the provenance preflight succeeds, publish the verified current branch only with `git push origin HEAD` or, when establishing its upstream, `git push --set-upstream origin HEAD`. Keep the command generic rather than hardcoding an issue branch or introducing a typed push helper.

Direct literal `main` pushes, force pushes, refspec pushes to the base, all other base-branch publication paths, and pull request base edits are prohibited.

## Required Outputs

Return issue/branch/base/head, separate red and green commit SHAs, linked pull request, status result, append-only issue evidence links, current pull request body state, ready state, PR URL, exact publication commands/results, residual risks, and blockers.

## Verification

Use only commands allowed by the agent frontmatter, including exact-path variants of:

```bash
git status --short --branch
git branch --show-current
git diff -- <reported-path>
git diff --cached -- <reported-path>
gh issue view <issue>
gh pr view <pr>
gh pr checks <pr>
```

## Architecture Rules Preserved

Preserve role-reported Resource/Action/Event/Reaction/View paths and architecture evidence without authorship; keep accepted tests immutable and red/green commits separate; retain append-only issue evidence and a current pull request body; and never merge, enable auto-merge, push the base branch, edit the pull request base, use broad staging, or bypass declared External Operation and journal boundaries. Reviewer and required CI prerequisites authorize readiness publication only, never merge authority. Only a human may review and merge.
