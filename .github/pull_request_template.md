Closes #

## Summary

-

## Dependency, Capability, And Task

- Task issue:
- Depends on / blocks:
- Capability or milestone:

## Tests

- [ ] Tests were written before implementation
- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --all`

## Architecture Impact

- [ ] Action targets exactly one Resource
- [ ] Event belongs to exactly one Resource stream
- [ ] Replay/apply remains deterministic
- [ ] External calls use declared External Operations
- [ ] Resource Events and execution journals remain separate
- [ ] Views derive from Events and are rebuildable
- [ ] Not applicable; explain:

## Rust Quality

- [ ] Public/runtime errors are named errors
- [ ] Domain Action errors implement `ActionFailure` where relevant
- [ ] No raw string errors at framework boundaries unless wrapped in a named error
- [ ] No `anyhow` in core framework public boundaries
- [ ] No speculative abstraction
- [ ] No unplanned refactor

## Docs

- [ ] Docs updated
- [ ] ADR updated or added
- [ ] No docs needed; explain:

## Notes For Reviewer

-
