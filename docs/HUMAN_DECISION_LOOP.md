# Human Decision Loop

Routine delivery is autonomous. Use this exception only when the issue, accepted tests, glossary, ADRs, and architecture rules do not determine a safe answer.

## Use It For

- Business or domain semantics.
- Resource versus Component ownership.
- Action/Event naming with different domain meaning.
- Authority, freshness, Policy, or External Operation semantics.
- Material architecture trade-offs.
- Scope changes that alter acceptance criteria or dependencies.
- Conflicts between accepted tests/fixtures and the task card or architecture.
- Reviewer escalations that existing contracts cannot resolve.

Do not invoke it for formatting, compiler errors, routine handoffs, labels, or implementation details already decided by the repository.

## Request

Use `.github/ISSUE_TEMPLATE/decision-request.md`. Keep the request concrete:

- State why human judgment is required.
- Link the issue/PR, dependencies, capability, and relevant docs/ADRs.
- Present at most three options and recommend one.
- Explain domain consequences before implementation consequences.
- State which acceptance criterion or dependency changes.

Work stops until the answer is recorded. The default recommendation is not permission to proceed silently.

## Resume

After the decision:

1. Record it in the issue or pull request.
2. Add or supersede an ADR when architecture changed.
3. Update acceptance criteria, dependencies, vocabulary, and tests as needed.
4. Resume with fresh role sessions.

Accepted tests and fixtures remain immutable to Implementers. Follow the conflict handoff in [DEVELOPMENT_WORKFLOW.md](DEVELOPMENT_WORKFLOW.md).

## Accepted-Test Defect Decision

When a Reviewer reports an accepted test defect as a blocker, no role changes it until the issue records explicit human confirmation and authorized paths. The Orchestrator then starts a fresh Test Writer to decide whether valid semantic red exists. Missing non-test behavior requires the canonical red/green flow; only already-correct non-test behavior whose corrected tests pass immediately may use an explicitly named test-contract correction. Passing test-contract correction proof is never red proof.

The correction report includes the human confirmation, authorized test or fixture paths, old/new hashes, the exact focused passing proof, and why semantic red is impossible. If any non-test behavior is missing or the decision changes scope or architecture, stop and return to the canonical tests-first flow.
