# ADR 0015: Use GitHub Issues as the Operational Queue

Status: Accepted

Date: 2026-07-04

## Context

Elbmesh should be built through phased, MR-based, multi-agent delivery. The phased plan describes the roadmap, but agents need an operational queue with ownership, status, comments, links to PRs, and review history.

A repo-local Markdown queue would be easy to edit but weak for multiple agents working concurrently.

## Decision

Use GitHub Issues as the operational source of truth for implementation task cards.

Use this mapping:

```text
Issue = task card / planned work
Branch = implementation workspace
Pull Request = review and merge artifact
Docs = architecture source of truth
```

Rules:

```text
No issue, no implementation.
No failing tests or explicit test plan, no implementation.
One implementation issue maps to one PR unless the Orchestrator explicitly splits it.
Every PR closes or links its issue.
The Orchestrator manages desired queue state, dependencies, and phase sequencing.
The MR Reviewer reports merge readiness only after quality gates pass; a human performs the merge and retains all merge authority.
```

The Orchestrator manages desired queue state.
Because Bash is denied, the shell-free Orchestrator reports readiness and requests each issue-label transition; a human applies every label mutation.

## Labels

Use phase labels:

```text
phase:0-rails
phase:1-core
phase:2-journals
phase:2.5-visibility
phase:3-manifest
phase:4-reference-flow
phase:5-reactions
phase:6-views
phase:7-nats
phase:8-external-restate
phase:9-generation-docs
phase:10-cli-agentic
```

Use workflow labels:

```text
status:planned
status:tests-needed
status:tests-ready
status:implementation
status:review
status:blocked
status:decision-needed
status:merged
```

Use agent and quality labels:

```text
agent:orchestrator
agent:test-writer
agent:implementer
agent:reviewer
needs:docs
needs:adr
needs:architecture-check
needs:named-errors
needs:human-decision
```

## Consequences

The phased delivery plan remains the roadmap.

GitHub Issues become the working queue.

Pull Requests become the enforced review/merge boundary.

Agents can coordinate through issue labels and comments instead of editing a queue file concurrently.

The repo still keeps issue and PR templates so new work starts with the right gates.

## Rejected Approach

Do not use only `work/mr-queue/*.md` as the canonical queue.

Repo-local task files can be useful as fixtures or exported reports, but GitHub Issues provide better coordination, review history, and ownership for multiple agents.
