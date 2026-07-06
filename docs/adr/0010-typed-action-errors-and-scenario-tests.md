# ADR 0010: Use Typed Action Errors and Given/When/Then Scenarios

Status: Accepted

Date: 2026-07-03

## Context

Action failures can be domain-significant without being Resource Events. Tests should assert those failures precisely instead of matching strings.

The `eventually-rs` given/when/then testing style is a good fit for event-sourced behaviour:

```text
Given historical Events
When an Action is invoked
Then these Events are emitted
```

or:

```text
Given historical Events
When an Action is invoked
Then this typed domain error is returned
```

## Decision

`Handle<Action>` has an associated typed error:

```rust
impl Handle<CreateOfferV1> for Offer {
    type Error = OfferError;
}
```

Domain errors implement `ActionFailure` and expose a stable error code.

The core crate provides `ActionScenario<Resource>` for tests:

```rust
ActionScenario::<Offer>::new()
    .given(vec![OfferCreatedV1 { ... }])
    .when(CreateOfferV1 { ... })
    .then_error(OfferError::AlreadyExists)
    .assert()
    .await;
```

## Consequences

Domain failures stay out of Resource event streams unless explicitly modelled as business Events.

Tests can assert exact domain errors.

The same style can later support generated Action/Event enums and cross-Resource reaction scenarios.
