# Oteryn v2 Foundation Programme — Current Status

- Status: Canonical current execution-status overlay
- Date: 2026-08-07
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: current foundation gate progression and next-action interpretation
- Does not supersede: accepted ADR semantics, gate definitions, product decisions or historical evidence

## Purpose

Keep the live foundation programme state unambiguous when long-lived backlog, global-register, gap-register, baseline or coordinator documents still contain progress sentences written before a later cross-repository closeout completed.

This file is authoritative only for **current execution status and gate readiness**. Accepted architecture remains authoritative in ADRs and dedicated contracts. Stable gate definitions remain authoritative in `FOUNDATION_DECISION_BACKLOG.md` and the wider decision horizon remains authoritative in `GLOBAL_ARCHITECTURE_DECISION_REGISTER.md`.

When an older document says that the `blakinio/otclient` source-only historical marker is still pending, that progress statement is stale and is superseded by the exact evidence below. No other architectural statement is superseded implicitly.

## Exact migration and closeout evidence

### Destination cutover — complete

- source snapshot used for the accepted migration: `blakinio/otclient@c923ad8a1dff17b4933a6110931b0823cec2c590`;
- canonical destination: `blakinio/Oteryn-v2`;
- canonical client path: `apps/client`;
- destination PR: `blakinio/Oteryn-v2#50`;
- destination squash merge: `78988f72a80cc904aa9176ae850c50d4efa0b0f0`;
- result: accepted 19-member Rust workspace and ADR-0011 `pre-native-protocol` client state.

### Source-only historical marker — complete

- source-marker PR: `blakinio/otclient#274`;
- exact source-marker head: `0bb7f92ae420fc3e81a4ade62a9b9b994c894f0c`;
- exact-head `Rust Client` run `31155904330`: `PASS`;
- exact-head repository `CI` run `31155910869`: `PASS`;
- unresolved review threads: `0`;
- source-marker squash merge: `8c56c45c6c25147470ce3ca23e639a31d9085e47`;
- effect: `blakinio/otclient/oteryn-client/**` is explicitly `HISTORICAL / NON-CANONICAL`, new Oteryn v2 Rust-client work is redirected to `blakinio/Oteryn-v2`, and the source history/provenance remains preserved.

### Source-marker lifecycle archive — complete

- lifecycle PR: `blakinio/otclient#275`;
- lifecycle exact head: `1e888ba073742c26bf9a1cae5786a059a270fa00`;
- repository `CI` run `31156414051`: `PASS`;
- lifecycle squash merge: `26f7646ea26b27c9ac4bf617b8cb0d63c89bdfda`;
- `blakinio/otclient/main` after closeout: `26f7646ea26b27c9ac4bf617b8cb0d63c89bdfda`;
- task ownership: released and archived.

## Current ordered foundation state

| Gate / programme step | Current status | Consequence |
|---|---|---|
| `FND-01` | `ACCEPTED AND APPLIED` | Workspace/dependency/migration contract is complete. |
| `VSL-02` destination cutover | `COMPLETE` | Canonical migrated client/workspace exists in Oteryn-v2. |
| `VSL-02` source-only closeout | `COMPLETE` | Historical/non-canonical marker and archive are merged in `blakinio/otclient`. |
| `FND-ID-01` | `NEXT ORDERED GATE` | Its source-marker start condition is satisfied; the full contract may now be drafted and accepted through its own bounded task/PR. |
| `FND-02` | `BLOCKED ON OWN CONTRACT SEQUENCE` | Do not freeze canonical protocol schemas/codecs before identifier semantics required by the wire boundary are accepted. |
| `FND-03` | `BLOCKED ON OWN CONTRACT` | No authoritative runtime implementation claim yet. |
| `FND-04` | `BLOCKED ON OWN CONTRACT` | No production admission/lease implementation claim yet. |
| `DUR-01`…`DUR-04`, `ANL-01`… | `LATER GATES` | Existing ordering and architecture requirements remain unchanged. |

## `FND-ID-01` readiness boundary

The source-marker prerequisite is complete, but `FND-ID-01` itself is **not complete**.

Existing owner-accepted inputs include at least:

- `FND-ID-01_OWNER_ACCEPTED_BASELINE.md` — semantic identity classes and accepted world/channel/instance/party scope;
- `UUIDV7_DURABLE_IDENTITY_OWNER_BASELINE.md` — strongly typed UUIDv7 direction for Oteryn-owned durable cross-boundary identities plus runtime/session/content handle separation;
- `INSTANCE_SCOPE_AND_RUNTIME_OWNER_BASELINE.md` — instance identity and runtime-ownership consequences;
- `FND-ID-01_WORLD_CHANNEL_ID_ISSUANCE_OWNER_BASELINE.md` — Platform World Registry/topology authority is the canonical issuer of `WorldId` and `ChannelId`; GameNodes consume assigned identity and use ownership fencing rather than minting replacement channel identities;
- other accepted owner baselines that constrain privacy, presence and cross-boundary identity exposure.

These are mandatory inputs to the complete `FND-ID-01` contract. They do not authorize protocol, runtime, database or admission implementation by themselves.

## Stale progress-only reconciliation

The following long-lived documents may contain sentences written before 2026-08-07 saying the source-only marker is still pending or that `FND-ID-01` cannot start until it is merged:

- `FOUNDATION_DECISION_BACKLOG.md`;
- `GLOBAL_ARCHITECTURE_DECISION_REGISTER.md`;
- `ARCHITECTURE_ANALYSIS_GAP_REGISTER.md`;
- `PRODUCT_DIRECTION_BASELINE.md` and ADR-0010 historical programme-effect text;
- `docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md`;
- earlier task/evidence snapshots.

For **execution status only**, interpret those pending-marker sentences using this current-status overlay and the live non-owning foundation programme checkpoint. Their architecture definitions, unresolved-question lists, gate semantics and accepted decisions remain in force.

Do not mass-rewrite historical ADRs or evidence merely to replace old progress wording. When a shared register is next materially revised, its progress section should be reconciled to this status.

## Current next action

Create one bounded `FND-ID-01` contract task in `blakinio/Oteryn-v2`, consume all owner-accepted identifier baselines, complete the minimum cross-boundary identifier catalogue/owner/issuer/scope/visibility/encoding constraints, perform independent audit and exact-head validation, then merge/archive before `FND-02` freezes dependent protocol identity fields.

`GAME-VISION-01` analysis may continue in parallel when it does not redefine accepted foundation identity, repository, protocol, Platform or persistence boundaries.

## Non-authorization

This status reconciliation does not authorize implementation of `protocol-oteryn`, authoritative runtime, Game Session admission, character leases, persistence schemas, durable gameplay, source-repository changes or production operations.
