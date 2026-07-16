# Documentation

Each active document owns one concern. Follow links instead of copying rules between files.

## Active Documents

| Document | Owns |
| --- | --- |
| [Goal](GOAL.md) | Product thesis, success criteria, and non-negotiable architecture rules |
| [Glossary](GLOSSARY.md) | Canonical domain and runtime vocabulary |
| [Development Workflow](DEVELOPMENT_WORKFLOW.md) | Delivery sequence, role authority, statuses, evidence, and quality gates |
| [Delivery Roadmap](DELIVERY_ROADMAP.md) | Capability dependencies and milestone checkpoints |
| [Agent Delivery Harness](AGENT_DELIVERY_HARNESS.md) | OpenCode-specific delegation, permissions, limitations, and reload behavior |
| [Agent Skills](AGENT_SKILLS.md) | Skill catalog and concrete-skill contract |
| [Human Decision Loop](HUMAN_DECISION_LOOP.md) | Exceptional semantic decision handling |
| [Implementation Plan](IMPLEMENTATION_PLAN.md) | Technical boundaries and remaining implementation areas |
| [NATS Test Harness](NATS_ADAPTER_TEST_HARNESS.md) | Local NATS adapter test commands and key formats |
| [Restate Test Harness](RESTATE_ADAPTER_TEST_HARNESS.md) | Local Restate adapter test commands and runtime setup |

## Generated Artifacts

Do not edit these manually:

- [Resource Capabilities](RESOURCE_CAPABILITIES.md)
- [Resource Capabilities JSON](resource-capabilities.json)
- [Rust Binding Stubs](resource-bindings.rs)

## Historical Review Artifacts

These preserve what was known at a checkpoint; they are not current delivery instructions:

- [Execution Trace Model](EXECUTION_TRACE_MODEL.md)
- [Runtime Debt And Failure Modes](RUNTIME_DEBT_AND_FAILURE_MODES.md)
- [Offer Demonstration Run Plan](OFFER_DEMONSTRATION_RUN_PLAN.md)
- [Phase 4 Reference Flow Checkpoint](PHASE_4_REFERENCE_FLOW_CHECKPOINT.md)
- [Phase 6 Reactions And Views Checkpoint](PHASE_6_REACTIONS_VIEWS_CHECKPOINT.md)
- [Phase 8 NATS And External Operations Checkpoint](PHASE_8_NATS_EXTERNAL_OPERATIONS_CHECKPOINT.md)

## Architecture Decisions

1. [Use Domain-Friendly Vocabulary Over Raw Event-Sourcing Terms](adr/0001-event-sourcing-framework-vocabulary.md)
2. [Resources Are Aggregates, Components Are Owned State](adr/0002-resource-component-boundary.md)
3. [Separate Actions, Events, Receipts, and Execution Journals](adr/0003-actions-events-receipts-and-journals.md)
4. [External Operations Are First-Class Execution Metadata](adr/0004-external-operations-and-restate.md)
5. [Use Separate NATS Streams for Domain and Execution Records](adr/0005-nats-streams-and-message-metadata.md)
6. [Modeler Generates Contracts, Developers Write Behavior](adr/0006-schema-generated-bindings-and-handwritten-behavior.md)
7. [Model Workflows as Event-Action Graphs](adr/0007-reactions-and-workflow-graphs.md)
8. [Views Are Materialized Read Models With Declared Queries](adr/0008-views-queries-and-nats-storage.md)
9. [Start With a Typed Core Based on the Existing Message Infrastructure](adr/0009-initial-core-implementation-from-message-infra.md)
10. [Use Typed Action Errors and Given/When/Then Scenarios](adr/0010-typed-action-errors-and-scenario-tests.md)
11. [Import Non-Event-Sourced Data as Provenanced Starting Facts](adr/0011-importing-non-event-sourced-data.md)
12. [Copy Eventually Concepts Without Taking a Dependency](adr/0012-copy-eventually-concepts-without-dependency.md)
13. [Agent Skills Are First-Class Repo Artifacts](adr/0013-agent-skills-are-first-class-repo-artifacts.md)
14. [Use Phased MR-Based Multi-Agent Delivery](adr/0014-phased-mr-based-multi-agent-delivery.md)
15. [Use GitHub Issues as the Operational Queue](adr/0015-use-github-issues-as-operational-queue.md)
16. [Use Human Decision Requests for Domain and Architecture Gates](adr/0016-use-human-decision-requests-for-domain-and-architecture-gates.md)
17. [Use Dependency-Ordered Issue Delivery and Publisher-Managed Statuses](adr/0017-dependency-ordered-issue-delivery-and-publisher-managed-statuses.md)
