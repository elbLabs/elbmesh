# Glossary

These terms are canonical across models, manifests, generated code, runtime APIs, docs, and agent instructions.

## Domain Model

| Term | Meaning | Core rule |
| --- | --- | --- |
| Resource | Top-level business object; technically an event-sourced aggregate root | Owns Components, exposes Actions, records Events, and rebuilds from its Event stream |
| Component | State owned inside one Resource | Has no independent Event stream and changes only through its Resource |
| Action | Invokable business capability; technically a command | Targets one Resource, appends to one Resource stream, and uses declared External Operations for external calls |
| Event | Durable past-tense domain fact | Belongs to one Resource stream and is the only input to Resource replay |
| Receipt | Caller-facing Action result | Summarizes completion but is not replay state |
| Policy | Rule governing behavior or access | May allow, deny, or require approval; does not replace Resource behavior |
| Reaction | Typed Event-to-Action subscription | Invokes one Action and never mutates a Resource directly |
| Workflow | Named graph or subgraph of Reactions | Composes `Action -> Event -> Reaction -> Action`; cycles are excluded in v1 |
| View | Materialized read model derived from Events | May combine Resources but remains rebuildable and non-authoritative |
| Query | Declared read capability against one View | Does not change Resources or emit Events; list queries use declared indexes |

## External And Execution Terms

| Term | Meaning | Core rule |
| --- | --- | --- |
| External System | Provider or integration boundary | Execution metadata, not a business Resource |
| External Operation | Declared external read or write used by an Action | Uses deterministic identity/idempotency and records execution separately from Resource Events |
| ActionJournal | Action lifecycle record store | Contains called/completed/rejected/failed records, never Resource replay facts |
| OperationJournal | External Operation lifecycle record store | Keyed by `operation_id`; completed results are reused before retrying a provider |
| ReactionJournal | Reaction lifecycle record store | Keeps trigger/completion records outside Resource Event streams |
| Resource State | Current state reconstructed from Resource Events | Deterministic and free of external calls |
| Resource View | Read-optimized or enriched Resource representation | Never used for event-sourced replay |

## Generated Artifacts

| Term | Meaning | Core rule |
| --- | --- | --- |
| Architecture Manifest | Canonical machine-readable architecture contract | Drives schemas, bindings, docs, checks, and agent metadata |
| Capability Document | Human or machine view of declared manifest capabilities | Includes generator metadata and manifest hash; does not imply live infrastructure |
| Binding Stub | Generated Rust type shell | Declares names and schema constants, not business behavior or provider registration |

## Import Terms

| Term | Meaning | Core rule |
| --- | --- | --- |
| Import Action | Action that onboards external or legacy state | Emits explicit imported/linking Events with provenance |
| Imported Event | Replayable starting fact from an external source | Marked as imported; not presented as native history |
| Synthetic Event | Inferred historical fact created during import | Allowed only when source data supports the inference and marked synthetic/imported |

## Naming

- Actions are imperative: `Create Offer`, `Accept Offer`, `Void Invoice`.
- Events are past-tense facts: `Offer Created`, `Offer Accepted`, `Invoice Voided`.
- Receipts may use completion language: `Create Offer Completed`.
- Custom Actions name their Events explicitly.

Common defaults:

| Action verb | Event suffix |
| --- | --- |
| create | Created |
| update | Updated |
| archive | Archived |
| cancel | Cancelled |
| void | Voided |
| submit | Submitted |
| approve | Approved |
| reject | Rejected |
| publish | Published |
| sync | Synced |

## Source Metadata

| Field | Values |
| --- | --- |
| Authority | `internal`, `external`, `mixed`, `derived` |
| Freshness | `stored`, `observed_on_action`, `live_on_read`, `subscribed`, `derived` |
| Event origin | `native`, `imported`, `synthetic`, `external_observation` |

V1 supports stored and observed-on-Action data. Background sync, live external enrichment, arbitrary SQL-like queries, hard domain delete, cyclic Workflows, and generic compensation graphs remain out of scope.
