# ADR 0018: Keep Pull Requests Reviewable With Single-Source Delivery Evidence

Status: Accepted

Date: 2026-07-17

Supersedes: ADR 0017 evidence placement only. ADR 0017 dependency ordering, Publisher-managed statuses, provenance checks, and human-only merge authority remain accepted.

## Context

ADR 0017 required red, green, rework, and readiness evidence to be append-only on both the GitHub Issue and pull request. Each later comment repeated the complete earlier history. Pull requests with small diffs accumulated tens of thousands of characters of duplicated role provenance, commands, results, and final-state fields.

The duplicated comments obscured the code diff and left the immutable pull request body describing an obsolete red or draft state. The same evidence already existed on the linked issue, so duplication made human review harder without improving auditability.

## Decision

Use one source for immutable audit evidence and one surface for current review state:

```text
GitHub Issue comments = append-only audit trail
Pull request body = current human review summary
Pull request comments = human review discussion and actionable findings
```

The Publisher appends one stage-specific issue comment for each red, green, rework, or readiness publication. A stage delta is not cumulative. It records only the new stage's role task/session IDs, exact changed paths, commit SHA, exact commands and concise results, blocker status, and pull request URL. A readiness delta also records the review task, reviewed range, findings, CI state, and residual risks. Links connect the stage deltas without copying prior content.

The Publisher creates or updates one concise pull request body at every publication stage. The body reports the current state, scope, changed paths, red and green commits, current head, verification summary, review and CI state, blockers, residual risks, and links to the issue audit trail. It replaces stale pending fields instead of preserving them as current truth.

After findings, the final Reviewer produces a Human Review Briefing of no more than 700 words containing a 60-second summary, change map, one evidence-backed Mermaid graph, architecture impact, risk map, suggested review order, proof, approval criteria, open questions, non-goals, and residual risks. Runtime graphs show declared Action/Event flow; other changes use manifest ownership, dependency, delivery, or decision flow. Every edge and claim must be supported by the diff, manifest/capability documents, tests, or accepted role evidence.

At readiness, the Publisher places the accepted Reviewer briefing verbatim at the top of the current pull request body and fills publication fields from verified evidence. The Publisher does not author technical claims. A later accepted rework briefing replaces the earlier briefing in the body.

Routine delivery evidence comments are not posted on pull requests. Full command output belongs in role reports or CI logs; public evidence records the exact command and a concise result or failure excerpt. Human review comments and GitHub review objects remain unchanged.

## Existing Pull Requests

Existing issue evidence remains the audit source. After verifying that a duplicated pull request comment has matching issue evidence, maintainers may minimize the pull request comment as outdated rather than deleting audit history. Existing pull request bodies may be rewritten as current review summaries.

## Consequences

- Human reviewers see the current decision surface before historical process detail.
- Immutable evidence remains available on the issue without being duplicated.
- Rework adds a small delta instead of reproducing the complete delivery history.
- Pull request bodies are intentionally mutable and are not the audit source.
- Human reviewers receive a bounded explanation, graph, risk map, review route, and explicit approval criteria.
- Publisher permissions no longer allow routine pull request comments.
- Tests must enforce issue-only stage deltas, current pull request bodies, and denied routine evidence comments.

## Rejected Approaches

Do not keep cumulative evidence on both the issue and pull request. Do not use the mutable pull request body as the only audit record. Do not delete historical evidence when minimizing duplicated pull request comments is sufficient.
