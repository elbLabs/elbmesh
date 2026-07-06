---
name: Implementation Task
about: Phase-scoped Elbmesh implementation work
title: "[Phase X] "
labels: ["status:planned"]
assignees: ""
---

## Phase

Phase:

## Goal

What framework behavior or capability should exist?

## Architecture Context

Relevant docs and ADRs:

- `docs/PHASED_DELIVERY_PLAN.md`
- `docs/DEVELOPMENT_WORKFLOW.md`
- `docs/GLOSSARY.md`

Relevant glossary terms:

-

Affected crates/modules:

-

## Acceptance Criteria

- Given ... When ... Then ...
- Given ... When ... Then ...

## Tests To Write First

- [ ] Scenario/unit tests:
- [ ] Contract tests:
- [ ] Integration tests:
- [ ] Architecture-rule tests:

## Non-Goals

-

## Quality Gates

- [ ] Tests were written before implementation
- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --all`
- [ ] Public/runtime errors are named errors
- [ ] Domain Action errors implement `ActionFailure` where relevant
- [ ] Docs updated or no-docs-needed explained
- [ ] No unplanned refactors or speculative abstractions

## Documentation Updates

- [ ] ADR needed
- [ ] Glossary update needed
- [ ] Development workflow update needed
- [ ] Phased delivery plan update needed
- [ ] Capability docs update needed
- [ ] No docs needed because:

## Architecture Rules

- [ ] Action targets exactly one Resource
- [ ] Event belongs to exactly one Resource stream
- [ ] Replay/apply code remains deterministic
- [ ] External calls happen only through declared External Operations
- [ ] Resource Events and execution journals remain separate
- [ ] Views derive from Events and are rebuildable

## Dependencies

Depends on:

Blocks:
