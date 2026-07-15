---
name: elbmesh-mr-reviewer
description: Use only as a compatibility/manual deep-review skill outside the canonical delivery sequence.
---

# Elbmesh MR Reviewer

Use this optional compatibility/manual skill only when a human or active Reviewer requests supplemental deep review. It is not an additional required stage and does not own or report merge readiness. Only `elbmesh-reviewer` reports final pull request merge readiness; a human performs the merge.

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

Also read the expanded issue, complete pull request range, immutable role evidence, and current checks.

## Permitted Edit Surface

None. This is a read-only supplemental review skill.

## Responsibilities

Review issue scope, dependencies, tests-first provenance, Rust quality, architecture, docs, evidence, and residual risks. Return observations to the active `elbmesh-reviewer` or human without making a readiness determination.

## Required Outputs

Return findings ordered by severity, quality-gate observations, exact reviewed range, supplemental deep-review report, required follow-ups, and residual risks.

## Verification

When authorized, run exactly:

```bash
git status --short --branch
git diff --check origin/main...HEAD
codehud . --diff origin/main
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

## Architecture Rules Preserved

Check Resource/Action/Event ownership, deterministic replay, declared External Operations, journal/Event separation, Reaction execution through Actions, rebuildable Views, tests-first provenance, and read-only review. Never merge or claim final readiness.
