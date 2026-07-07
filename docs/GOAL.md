# Goal

Elbmesh should not become another event-sourcing framework.

The project exists to prove a stronger thesis:

```text
Agents need architecture, not just prompts.
```

The goal is to build an event-sourced architecture substrate that humans can model, Rust services can execute, and coding agents can safely understand, extend, and verify.

## Product Thesis

Most event-sourcing frameworks help developers implement aggregates, commands, events, projections, and stores. Elbmesh should make the architecture itself explicit, inspectable, enforceable, and useful to agents.

The differentiator is not event sourcing by itself. The differentiator is the loop:

```text
Model business capability
-> generate architecture manifest and schemas
-> generate Rust contracts and capability docs
-> implement explicit behavior
-> check architecture rules
-> explain flows to humans and agents
```

If that loop works, agents can make changes with less guesswork and fewer architectural mistakes.

## Core Outcome

The framework should let a team describe business capability once and use that description across modelling, code generation, runtime enforcement, documentation, and agent tooling.

The system should answer:

- What Resources exist?
- What Actions can be invoked?
- What Events can be recorded?
- What Policies govern behavior?
- What Workflows coordinate multiple Resources?
- What External Operations are allowed?
- What Views and Queries are available?
- What files should a developer or agent edit to add behavior?
- What architecture rules must never be violated?

## Why QuickThought Matters

QuickThought is the modelling program that should make people care about the implementation.

Its job is not to draw diagrams. Its job is to create an executable architecture contract.

QuickThought should model:

- `Resource`: event-sourced aggregate root.
- `Component`: owned state inside a Resource.
- `Action`: business capability targeting one Resource.
- `Event`: durable domain fact recorded in one Resource stream.
- `Policy`: DMN/FGA/business rule governing Actions and Resources.
- `Workflow`: graph of Event-to-Action Reactions across Resources.
- `External System`: provider or integration boundary.
- `External Operation`: declared external read/write used by an Action.
- `View`: materialized read model derived from Events.
- `Query`: declared read capability against a View.

QuickThought should export a canonical architecture manifest that drives all generated artifacts.

```text
QuickThought model
-> architecture.manifest.json
-> JSON Schemas
-> Rust bindings
-> human docs
-> agent metadata
-> architecture checks
```

## Architecture Manifest

The architecture manifest is the central contract between modelling, generated code, runtime, docs, and agent tools.

It should contain:

- Resources, Components, Actions, Events, Policies, Workflows, Views, and Queries.
- Schema IDs and schema versions.
- Action input, Event payload, and Receipt contracts.
- Resource ownership, authority, and freshness metadata.
- External Systems and External Operations.
- Declared Event-to-Action Reactions.
- NATS subject patterns and required message metadata.
- Policy hooks and approval requirements.
- Generated artifact metadata, including manifest hash and generator version.

The manifest should be flexible enough to be produced by QuickThought, edited by a future CLI, or used as a fixture inside this framework repository.

## Framework Scope

V1 should be Rust-first.

The framework should provide:

- Runtime traits for `Resource`, `Action`, `Event`, `Handle<Action>`, and `Apply<Event>`.
- Typed Action and Event dispatch through generated enums.
- One registered handler per Resource and Action version.
- One explicit apply implementation per Event version.
- NATS JetStream as the authoritative Resource Event store.
- Separate NATS execution journals for Actions, Operations, and Reactions.
- Restate hidden behind framework execution APIs.
- Declared External Operations with idempotency and operation journaling.
- Materialized Views and simple declared Queries.
- Generated human and machine-readable capability documentation.
- Architecture checks that coding agents can run before claiming completion.

Developer behavior should remain explicit. The framework can generate contracts and boilerplate, but developers should write Action handlers, Event apply logic, projection logic, and external response mapping.

## Core Rules

These rules define the architecture agents must preserve:

- A Resource is an event-sourced aggregate root.
- A Component is owned state inside a Resource.
- An Action targets exactly one Resource.
- An Action appends Events to exactly one Resource stream.
- Failed or denied Actions do not append Resource Events unless the failure is explicitly modelled as a business fact.
- Cross-Resource behavior happens through Workflows/Reactions.
- Resource replay uses stored Resource Events only.
- Replay never calls external systems.
- External calls require declared External Operations.
- External operation records live in execution journals, not Resource Event streams.
- Resource Events store selected domain facts, not raw provider responses.
- Resource Views may enrich or optimize reads, but they are not replay state.
- No hard domain delete in v1.

## CLI And Agent Tooling

The CLI should make architecture changes guided and checkable. These commands are not just developer convenience. They are the rails that let coding agents change the system safely.

### `add-action`

Adds a new business capability to a Resource.

Example:

```bash
elbmesh add-action Invoice "Void Invoice"
```

Expected behavior:

- Updates the architecture manifest.
- Creates or links Action input and Receipt schemas.
- Optionally creates the primary Event.
- Optionally attaches Policies and External Operations.
- Generates handler stubs and tests where appropriate.
- Regenerates capability docs and agent metadata.

### `add-event`

Adds a stored domain fact to a Resource.

Example:

```bash
elbmesh add-event Invoice "Invoice Voided"
```

Expected behavior:

- Updates the architecture manifest.
- Creates the Event schema and version metadata.
- Generates or updates Event dispatch bindings.
- Creates an `Apply<Event>` stub.
- Regenerates capability docs and agent metadata.

### `add-projection`

Adds a materialized View derived from Events.

Example:

```bash
elbmesh add-projection Receivables
```

Expected behavior:

- Updates the architecture manifest.
- Declares source Events.
- Declares view documents and indexes.
- Generates projection stubs.
- Declares rebuild and checkpoint behavior.
- Makes it clear that the View is derived and not directly mutable.

### `check-architecture`

Validates implementation and generated artifacts against the manifest.

Example:

```bash
elbmesh check-architecture
```

Checks should include:

- Every Action targets exactly one Resource.
- Every Event belongs to exactly one Resource.
- Every Action version has exactly one registered handler.
- Every Event version has an explicit apply implementation.
- Resource handlers append only to their own Resource stream.
- External HTTP calls appear only through declared External Operations.
- Replay/apply code does not call external systems.
- Workflows/Reactions invoke Actions rather than mutating Resource state directly.
- Schemas are versioned.
- NATS metadata fields are present.
- Generated Markdown and JSON agent metadata match the manifest hash.

This is the most important command for coding agents.

### `explain-flow`

Explains what happens from an Action or Event through Policies, Events, Workflows, External Operations, and Views.

Example:

```bash
elbmesh explain-flow "Accept Offer"
```

Expected output should answer:

- Which Resource does the Action target?
- Which Policies are checked?
- Which Events can be recorded?
- Which Reactions or Workflows are triggered?
- Which downstream Actions are invoked?
- Which External Operations are used?
- Which Views are updated?
- Which Resources are affected directly or indirectly?

## Generated Capability Docs

The manifest should generate both human and machine-readable capability documentation.

Outputs:

```text
RESOURCE_CAPABILITIES.md
resource-capabilities.json
```

Both outputs must be generated from the same manifest and include:

- Manifest hash.
- Generator version.
- Resource list.
- Action list.
- Event list.
- Policy hooks.
- External Operations.
- Workflow/Reactions.
- Views and Queries.
- Implementation entry points.
- Architecture rules relevant to each Resource.

The Markdown is for humans. The JSON is for agents and automation. They must stay in sync because they are generated from the same source.

## Reference App

The project needs one polished reference app.

The first reference flow should be:

```text
Offer -> Sales Order -> Order Confirmation -> Invoice
```

It should include:

- Resources for Offer, Sales Order, Order Confirmation, and Invoice.
- Components such as line items, money, addresses, customer references, and approval state.
- Actions such as Create Offer, Accept Offer, Create Sales Order, Create Order Confirmation, and Create Invoice.
- Events such as Offer Created, Offer Accepted, Sales Order Created, Order Confirmation Created, and Invoice Created.
- Reactions connecting the flow.
- A mocked LexOffice-like External System.
- A declared External Operation for document creation.
- A View such as Document Flow Status or Receivables.
- Generated capability docs and agent metadata.

The hardest acceptance test should prove external operation recovery:

```text
External API call succeeds.
Resource Event append fails once.
Restate retries the append.
The external API is not called twice.
The Resource Event is recorded exactly once.
```

## Success Criteria

The project succeeds if an agent can safely add a new business capability by using the manifest, generated docs, CLI, and architecture checks.

The first strong demo should be:

```text
Ask an agent to add "Void Invoice" to the reference app.
```

The agent should be able to:

- Add the Action and Event through the CLI.
- Understand the Resource contract from generated docs.
- Implement the `Handle<Action>` trait.
- Implement the `Apply<Event>` trait.
- Attach or update a Policy if required.
- Update any relevant View projection.
- Run `check-architecture`.
- Run tests.
- Use `explain-flow` to show the resulting behavior.

The project is worth building only if this workflow is materially safer and clearer than asking an agent to infer the architecture from source files alone.

## Kill Criteria

Do not continue building this as a standalone framework if the architecture and agent tooling do not become the product.

Failure signs:

- The framework is only a Rust CQRS/event-sourcing library.
- Agents still need to infer all architecture from code search.
- Generated docs drift from runtime behavior.
- External operations remain hidden in handwritten handlers.
- `check-architecture` cannot catch meaningful violations.
- The reference app does not demonstrate safer agent-led change.

If those are true, use or contribute to an existing event-sourcing library instead.

## Positioning

The public pitch should be:

```text
Elbmesh is an architecture-as-code execution substrate for event-sourced systems built by humans and coding agents.
```

Not:

```text
Elbmesh is another Rust event-sourcing framework.
```
