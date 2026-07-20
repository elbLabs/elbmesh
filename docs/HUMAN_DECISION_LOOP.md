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

Accepted tests and fixtures remain immutable to Implementers.

An Implementer-discovered accepted-test or fixture conflict with the task card or architecture must stop with the Implementer reporting it to the Orchestrator. Only after explicit human confirmation may a fresh Test Writer revise an authorized path to produce canonical semantic red followed by green; this route must not use immediately passing test-contract correction.

## Accepted-Test Defect Decision

Immediately passing test-contract correction has one sole entry: a final Reviewer's path-specific accepted test blocker, followed by explicit human confirmation and a fresh Test Writer proving that non-test behavior is already correct and legitimate semantic red is impossible, so the corrected test would pass immediately. This final-Reviewer requirement does not make the Reviewer the sole entry to every accepted-test revision; Implementer-conflict revisions use the canonical semantic-red/green route above. Passing test-contract correction proof is never red proof.

The correction report includes the human confirmation, authorized test or fixture paths, old/new hashes, the exact focused passing proof, and why semantic red is impossible. If any non-test behavior is missing or the decision changes scope or architecture, stop and return to the canonical tests-first flow.
