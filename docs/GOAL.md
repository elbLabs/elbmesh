# Goal

Elbmesh is an architecture-as-code execution substrate for event-sourced systems built by humans and coding agents. It should not become only another Rust event-sourcing library.

## Thesis

```text
Agents need architecture, not just prompts.
```

Teams should describe a business capability once, then use that description for modelling, generated contracts, runtime enforcement, documentation, and agent tooling.

```text
Model capability
-> architecture manifest and schemas
-> Rust contracts and capability docs
-> explicit handwritten behavior
-> architecture checks and flow explanations
```

The canonical vocabulary is in [GLOSSARY.md](GLOSSARY.md). Technical scope is in [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md).

## Product Contract

QuickThought or another modeller exports one architecture manifest containing:

- Resources, Components, Actions, Events, Policies, Reactions/Workflows, Views, and Queries.
- Schema IDs and versions.
- Resource ownership, authority, and freshness.
- External Systems and declared External Operations.
- Message metadata and storage bindings.
- Generator identity and manifest hash.

That manifest drives JSON Schemas, Rust bindings, capability documentation, agent metadata, and architecture checks. Generated artifacts must not become independent sources of truth.

Developers continue to write business behavior: Action handlers, Event apply logic, projections, and external response mapping.

## V1 Scope

The Rust-first runtime provides:

- Typed `Resource`, `Action`, `Event`, `Handle<Action>`, and `Apply<Event>` contracts.
- Action execution with typed failures and receipts.
- Resource Event storage plus separate Action, Operation, and Reaction journals.
- Declared External Operations with idempotency and durable retry boundaries.
- Event-to-Action Reactions.
- Rebuildable Views and simple declared Queries.
- NATS and Restate adapters hidden behind framework APIs.
- Generated capability docs and binding stubs.
- Agent-usable architecture checks and flow explanations.

## Non-Negotiable Rules

- A Resource is an event-sourced aggregate root; Components are owned state.
- An Action targets and appends Events to exactly one Resource stream.
- Failed or denied Actions append no Resource Events unless failure is explicitly a domain fact.
- Resource replay uses stored Resource Events only and never calls external systems.
- Cross-Resource behavior uses Reactions that invoke Actions.
- External calls use declared External Operations.
- Resource Events contain selected domain facts, not execution or provider diagnostics.
- Action, Operation, and Reaction journals remain separate from Resource Event streams.
- Views derive from Events, remain rebuildable, and do not own business truth.
- V1 has no hard domain delete or cyclic Reaction graph.

## Reference Proof

The reference flow is:

```text
Offer -> Sales Order -> Order Confirmation -> Invoice
```

It must demonstrate typed Resources and Actions, cross-Resource Reactions, a materialized View, a mocked LexOffice-like External Operation, and this recovery case:

```text
External API succeeds.
Resource Event append fails once.
Retry does not repeat the external call.
The Resource Event is recorded exactly once.
```

## Success And Stop Criteria

Elbmesh succeeds when an agent can add a capability such as `Void Invoice` by following generated contracts, implementing explicit behavior, running architecture checks, and explaining the resulting flow without inferring the design from source alone.

Stop treating Elbmesh as a standalone product if generated contracts drift, external operations stay hidden in handlers, architecture checks catch nothing meaningful, or agent-led changes are no safer than ordinary code search.
