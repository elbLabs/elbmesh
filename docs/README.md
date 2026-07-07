# Documentation

This documentation captures the current design decisions from the domain-modelling grilling session.

## Start Here

1. [Goal](GOAL.md)
2. [Glossary](GLOSSARY.md)
3. [Development Workflow](DEVELOPMENT_WORKFLOW.md)
4. [Agent Skills](AGENT_SKILLS.md)
5. [Human Decision Loop](HUMAN_DECISION_LOOP.md)
6. [Phased Delivery Plan](PHASED_DELIVERY_PLAN.md)
7. [Runtime Debt And Failure Modes](RUNTIME_DEBT_AND_FAILURE_MODES.md)
8. [Execution Trace Model](EXECUTION_TRACE_MODEL.md)
9. [Offer Demonstration And Visualization Run Plan](OFFER_DEMONSTRATION_RUN_PLAN.md)
10. [Phase 4 Reference Flow Checkpoint](PHASE_4_REFERENCE_FLOW_CHECKPOINT.md)
11. [Implementation Plan](IMPLEMENTATION_PLAN.md)
12. [Architecture Decision Records](adr/)

## ADR Index

1. [ADR 0001: Use Domain-Friendly Vocabulary Over Raw Event-Sourcing Terms](adr/0001-event-sourcing-framework-vocabulary.md)
2. [ADR 0002: Resources Are Aggregates, Components Are Owned State](adr/0002-resource-component-boundary.md)
3. [ADR 0003: Separate Actions, Events, Receipts, and Execution Journals](adr/0003-actions-events-receipts-and-journals.md)
4. [ADR 0004: External Operations Are First-Class Execution Metadata](adr/0004-external-operations-and-restate.md)
5. [ADR 0005: Use Separate NATS Streams for Domain and Execution Records](adr/0005-nats-streams-and-message-metadata.md)
6. [ADR 0006: Modeler Generates Contracts, Developers Write Behavior](adr/0006-schema-generated-bindings-and-handwritten-behavior.md)
7. [ADR 0007: Model Workflows as Event-Action Graphs](adr/0007-reactions-and-workflow-graphs.md)
8. [ADR 0008: Views Are Materialized Read Models With Declared Queries](adr/0008-views-queries-and-nats-storage.md)
9. [ADR 0009: Start With a Typed Core Based on the Existing Message Infrastructure](adr/0009-initial-core-implementation-from-message-infra.md)
10. [ADR 0010: Use Typed Action Errors and Given/When/Then Scenarios](adr/0010-typed-action-errors-and-scenario-tests.md)
11. [ADR 0011: Import Non-Event-Sourced Data as Provenanced Starting Facts](adr/0011-importing-non-event-sourced-data.md)
12. [ADR 0012: Copy Eventually Concepts Without Taking a Dependency](adr/0012-copy-eventually-concepts-without-dependency.md)
13. [ADR 0013: Agent Skills Are First-Class Repo Artifacts](adr/0013-agent-skills-are-first-class-repo-artifacts.md)
14. [ADR 0014: Use Phased MR-Based Multi-Agent Delivery](adr/0014-phased-mr-based-multi-agent-delivery.md)
15. [ADR 0015: Use GitHub Issues as the Operational Queue](adr/0015-use-github-issues-as-operational-queue.md)
16. [ADR 0016: Use Human Decision Requests for Domain and Architecture Gates](adr/0016-use-human-decision-requests-for-domain-and-architecture-gates.md)

## Current V1 Thesis

```text
Resource = event-sourced aggregate root.
Component = owned state inside a Resource.
Action = command.
Event = stored domain fact.
Reaction = Event -> Action subscription.
Workflow = named graph/subgraph of Reactions.
View = materialized read model.
Query = declared read capability against a View.
```

The first implementation should prove the Offer to Invoice flow with NATS, Restate, a mocked external API, execution journals, and one materialized View.
