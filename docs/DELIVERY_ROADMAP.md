# Delivery Roadmap

GitHub Issues and their explicit dependencies are the delivery source of truth. GitHub Issue dependency links define delivery order. This roadmap supplies capability and milestone context only; [DEVELOPMENT_WORKFLOW.md](DEVELOPMENT_WORKFLOW.md) owns the delivery process.

An issue may start when its dependencies are resolved and its task card is complete. Independent issues may proceed concurrently when their edit surfaces and architecture contracts do not conflict.

## Capability Dependencies

| Capability | Depends on | Milestone evidence |
| --- | --- | --- |
| Delivery rails | Goal, glossary, ADRs | One issue completes test-first publication, review, CI, and human merge with auditable evidence |
| Typed Resource runtime | Delivery rails | Scenarios prove replay, rejection, append, metadata, versions, and typed failures |
| Execution journals | Typed runtime | Action/Operation/Reaction records remain separate from Resource Events under success and failure |
| Architecture manifest | Stable vocabulary and runtime contracts | Invalid ownership, undeclared operations, and invalid Reaction graphs fail with named findings |
| Reference business flow | Runtime and manifest | Offer-to-Invoice behavior is scenario-tested and inspectable |
| Reaction runtime | Events, Action execution, journal identity | Reactions invoke Actions with deterministic downstream identity |
| Views and Queries | Stable Events and Reactions | Views rebuild from Events and Queries use declared indexes |
| NATS adapters | Stable in-memory port contracts | Feature-gated adapters pass shared contracts without changing domain behavior |
| External Operations and Restate | ActionContext and OperationJournal | Retry after append failure does not duplicate provider calls or Resource Events |
| Generated capabilities and bindings | Manifest stability | Markdown, JSON, and Rust stubs share manifest hash and generator version |
| CLI and agent tooling | Manifest, generated artifacts, checks | Agents can check architecture and explain flows without inferring them from source |

## Milestone Checkpoints

Create a checkpoint when a coherent capability becomes demonstrable, a dependency boundary is about to change, or known debt may invalidate dependent work. A checkpoint should answer:

- Can a human understand and demonstrate the flow without reading source?
- Do tests cover success, rejection, failure, and recovery?
- Which debt or ambiguity affects the next dependency?
- Do adapters and tools preserve the logical model?

Useful evidence includes a flow diagram, coverage/failure matrix, debt register, demonstration run, and decision list. Existing checkpoint documents are historical records, not current gates.

## Remaining Areas

- Harden replay, idempotency, and partial-commit semantics.
- Complete the Offer-to-Invoice reference capability.
- Stabilize generated contracts and drift checks.
- Add architecture-check and flow-explanation tooling.
- Keep NATS and Restate integration explicit and feature-gated.

Candidates become deliverable only through a complete dependency-linked GitHub Issue.
