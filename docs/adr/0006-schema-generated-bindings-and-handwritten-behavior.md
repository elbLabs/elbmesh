# ADR 0006: Modeler Generates Contracts, Developers Write Behavior

Status: Accepted

Date: 2026-07-03

## Context

The future modeller should produce schemas and execution metadata. Rust developers still need explicit domain behavior for handlers, reducers, projections, and external response mapping.

## Decision

The modeler produces JSON Schema plus an execution manifest. Rust macros/codegen produce typed bindings from that contract.

Generated output should include:

```text
Action DTOs
Event DTOs
Receipt DTOs
Resource IDs
Component structs
Action/Event enums
schema IDs and versions
NATS subject/metadata bindings
External Operation declarations
policy hook bindings
dispatch boilerplate
agent/human capability docs
```

Developers write:

```text
Action handler logic
Event apply/reducer logic
View projection logic
External response mapping
Policy integration details where needed
```

Prefer explicit trait implementations over annotated free methods, especially for AI-assisted development.

## Consequences

Generated code remains contract-focused.

Business behavior remains explicit, searchable, and compiler-checked.

Recommended developer style:

```rust
impl Handle<CreateOfferV1> for OfferResource {
    async fn handle(
        state: &OfferState,
        action: CreateOfferV1,
        ctx: ActionContext,
    ) -> ActionResult<OfferEvent> {
        // domain decision logic
    }
}

impl Apply<OfferCreatedV1> for OfferState {
    fn apply(&mut self, event: OfferCreatedV1) {
        // replay logic
    }
}
```

For Events, old schema versions must replay forever. In v1, implement one `Apply` per Event version.
