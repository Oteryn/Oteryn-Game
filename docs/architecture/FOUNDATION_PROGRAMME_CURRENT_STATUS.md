# Oteryn v2 Foundation Programme — Current Status

- Status: Canonical current execution-status overlay
- Date: 2026-08-10
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: accepted foundation progression and next ordered architecture gates
- FND-04 lifecycle closeout merge: `adb0882a5ddbe42944fe955f5effb78fd5495422`
- Active Stage-B delivery: Issue #111 (`DUR-01`)
- Current phase represented by this delivery candidate: `DUR-01 FINAL DELIVERY / CLOSEOUT_PENDING`

## 1. Authority of this overlay

This document answers **what is accepted now and what may happen next**. Detailed review/CI/repair evidence lives in accepted contracts, archived task records and merged PRs.

If an older coordination/backlog/register paragraph describes FND-02, FND-03 or FND-04 as a live gate, that wording is historical execution narrative. Accepted contracts plus this overlay govern current progression. Stable decision IDs and future dependency requirements remain valid unless explicitly superseded.

`DUR-01 FINAL DELIVERY / CLOSEOUT_PENDING` is transition-safe: before the delivery merge it means final candidate validation is in progress; after the delivery merge it means only lifecycle archive/ownership release remains. DUR-01 becomes `ACCEPTED AND LIFECYCLE-CLOSED` only through its separate closeout.

No status row implies runtime implementation unless explicitly stated.

## 2. Foundation and Stage-B progression summary

| Gate | Current status | Canonical evidence / note |
|---|---|---|
| `FND-01` | `ACCEPTED AND APPLIED` | workspace/dependency contract + canonical 19-member destination cutover |
| `VSL-02` | `ACCEPTED AND COMPLETE` | Rust-client migration/cutover complete |
| `FND-ID-01` | `ACCEPTED` | `FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md` |
| `FND-02` | `ACCEPTED` | `protocol-oteryn` v1 architecture accepted; implementation remains separately gated |
| `FND-03` | `ACCEPTED` | authoritative runtime execution architecture accepted; implementation/capacity/OPS remain separately gated |
| `FND-04A` | `ACCEPTED AND LIFECYCLE-CLOSED` | fresh admission authority/profile |
| `FND-04B` | `ACCEPTED AND LIFECYCLE-CLOSED` | reconnect/recovery/continuity + recovery profile |
| `FND-04C` | `ACCEPTED AND LIFECYCLE-CLOSED` | errors/diagnostics/failure/compatibility integration |
| `FND-04` overall | `ACCEPTED AND CLOSED` | A/B/C semantic architecture accepted; programme #112 complete |
| `DUR-01` | `FINAL DELIVERY / CLOSEOUT_PENDING` | Issue #111; bounded analysis + candidate durable representation contract |
| `ANL-01` | `NEXT REQUIRED GATE` | must be accepted before DUR-02/DUR-03 finalize transactional outbox/critical audit evidence |
| `DUR-02` | `BLOCKED ON DUR-01 + ANL-01 INPUTS` | Persistence v1 |
| `DUR-03` | `BLOCKED ON DUR-01 + ANL-01 INPUTS` | item transaction/anti-duplication invariants |

## 3. Accepted foundation baseline

### FND-02

Canonical production protocol family is `oteryn`: TCP + TLS 1.3, ALPN `oteryn-game/1`, verified server identity, no plaintext/0-RTT/Canary fallback, bounded framing/protobuf, exact foundation UUID wire representation, authoritative post-admission GameSessionId, monotonic non-zero `connection_generation`, `(GameSessionId, CommandId)` identity, server sequencing/domain revisions, explicit snapshot/delta/resync and authenticated liveness primitives.

FND-02 `CommandId` is a GameSession-scoped monotonic non-zero `uint64`. FND-02 `schema_revision` is diagnostic/build evidence, not an opaque gameplay compatibility token.

### FND-03

GameNode runtime is multithreaded with many authoritative scopes and one logical ordered mutation owner per ChannelRuntime/InstanceRuntime. NodeId is process incarnation/placement rather than scope authority. Scope ownership generation, execution order, wall/monotonic clocks and CommandId remain distinct. Queues are bounded; stale generation/revision/handle work fails closed; capacity claims require evidence rather than guesses.

### FND-04

FND-04 is `ACCEPTED AND CLOSED`. Canonical index:

- `docs/architecture/FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md`.

Normative components are FND-04A fresh admission, FND-04B reconnect/recovery and FND-04C error/diagnostics/failure/compatibility integration.

Important frozen outcomes remain unchanged: ownership-before-world validation, separate AccountPresenceClaim/CharacterLease/GameSession/TransportBinding/runtime authority, purpose-separated Ed25519 grant profiles, independent protocol/transport/ruleset/content/map/world-policy dimensions, authenticated source-age `<=5s` with anti-rollback, PREPARE/COMMIT reconnect, healthy-binding non-preemption, `ControlLossEpoch`, exact 4-second eligible PvE re-entry protection, protection re-arm requiring stable-control evidence, no historical 2s/5s/15s timing authority, and fail-closed GameNode recovery without guessed continuity.

## 4. DUR-01 candidate decisions

DUR-01 consumes accepted identity meaning and freezes only durable representation.

Candidate final decisions are:

- native UUID-backed durable identities use PostgreSQL native `uuid`, preserving all 128 bits;
- canonical native `AccountId` is Platform-issued UUIDv7 per Platform ADR 0028; Platform local integer IDs and Canary account IDs are not native AccountId;
- canonical native `WorldId`/`ChannelId` are Platform-issued UUIDv7 per Platform ADR 0029 and ChannelId remains semantically WorldId-scoped;
- strong semantic types remain distinct even though PostgreSQL physical scalar is shared;
- nil UUID is invalid; semantic absence uses NULL/typed absence;
- scoped identities preserve `WorldId + component` and may not drop scope merely because UUID values are globally collision-resistant;
- UUIDv7 ordering is never authority, causality, fencing or business chronology;
- when FND-02 `CommandId` is persisted, preserve the full non-zero `uint64` as PostgreSQL `numeric(20,0)` and scope it by `GameSessionId`; signed `bigint` must not silently narrow the legal range;
- `ItemInstanceId` is introduced as game-owned global durable UUIDv7 identity for one concrete item-instance lifecycle;
- no generic catch-all DurableEntityId is introduced;
- EventId/OperationId/TransactionId/CorrelationId/CausationId/AnalyticsActorId remain ANL-01-owned semantic decisions;
- no cross-database Platform/game foreign keys; Platform-owned IDs remain contract references;
- legacy identity mapping uses a stable source namespace key; source revision/snapshot/import-run are provenance and cannot mint a second native identity for the same stable source entity;
- internal UUIDv7 values are not automatically public identifiers;
- representation changes require explicit lossless versioned migration and mixed-version evidence.

The candidate contract is:

- `docs/architecture/DUR-01_DURABLE_IDENTIFIER_REPRESENTATION_CONTRACT.md`.

Acceptance of this candidate does not create tables, migrations or runtime behavior.

## 5. Security/privacy boundary

- identity equality is not authentication/authorization;
- stored Platform-owned identity is not fresh Platform authority;
- AccountId/CharacterId/ItemInstanceId and other high-cardinality IDs are not ordinary Prometheus labels;
- UUIDv7 creation-time structure is considered before public exposure;
- migration/import conflicts fail closed rather than overwrite or silently re-key;
- no code may repair identity ambiguity by choosing the newest-looking UUID;
- CommandId persistence preserves its complete GameSession scope and unsigned range; persistence cannot redefine command identity.

## 6. Runtime/implementation status

Foundation architecture and DUR-01 representation architecture do **not** authorize:

- production protocol listener/client gameplay adapter;
- GameNode/session/admission/reconnect runtime implementation;
- PostgreSQL schema/table/migration implementation;
- CharacterLease persistence;
- item/currency mutation implementation;
- Platform migration/writes;
- production deployment/traffic;
- entitlement/Premium/VIP activation;
- broad gameplay/content production.

## 7. Next ordered architecture work

After DUR-01 delivery and lifecycle closeout:

1. `ANL-01 — Game Event and Audit Foundation` is the immediate missing prerequisite for durable audit/outbox semantics.
2. `DUR-02 — Persistence v1` may consume accepted DUR-01 identity/CommandId physical representation but must wait for required ANL-01 decisions before finalizing transactional outbox/critical audit evidence.
3. `DUR-03 — Item Transaction and Anti-Duplication Invariants` consumes `ItemInstanceId` and also requires ANL-01/DUR-02 integration for atomic provenance evidence.
4. `DUR-04 — Content, World Detail and Scripting` remains a separate durable-content gate.
5. `GAME-VISION-01` still blocks broad gameplay/content production.

`PROD-ENTITLEMENTS-01` remains separately blocked by open Platform security prerequisite `Oteryn-Platform#944`; it is not part of DUR-01.

## 8. Concise current rule

```text
foundation FND-01 .. FND-04
-> accepted / closed as applicable

DUR-01
-> FINAL DELIVERY / CLOSEOUT_PENDING
-> architecture representation only, no schema/runtime implementation

ANL-01
-> next required semantic gate

DUR-02 / DUR-03
-> cannot finalize durable mutation/audit architecture before required DUR-01 + ANL-01 inputs

PROD-ENTITLEMENTS-01
-> still blocked by Oteryn-Platform#944
```
