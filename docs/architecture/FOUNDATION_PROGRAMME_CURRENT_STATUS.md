# Oteryn v2 Foundation Programme — Current Status

- Status: Canonical current execution-status overlay
- Date: 2026-08-10
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: accepted foundation progression and next ordered architecture gates
- FND-04 lifecycle closeout merge: `adb0882a5ddbe42944fe955f5effb78fd5495422`
- DUR-01 delivery merge: `bc172d7244f823425feb84c46c7d04be1f942eed`
- Current phase represented by this closeout candidate: `DUR-01 ACCEPTED AND LIFECYCLE-CLOSED / NEXT ANL-01`

## 1. Authority of this overlay

This document answers **what is accepted now and what may happen next**. Detailed review/CI/repair evidence lives in accepted contracts, archived task records and merged PRs.

If an older coordination/backlog/register paragraph describes FND-02, FND-03, FND-04 or DUR-01 as a live gate, that wording is historical execution narrative. Accepted contracts plus this overlay govern current progression. Stable decision IDs and future dependency requirements remain valid unless explicitly superseded.

No status row implies runtime implementation unless explicitly stated.

## 2. Foundation and Stage-B progression summary

| Gate | Current status | Canonical evidence / note |
|---|---|---|
| `FND-01` | `ACCEPTED AND APPLIED` | workspace/dependency contract + canonical 19-member destination cutover |
| `VSL-02` | `ACCEPTED AND COMPLETE` | Rust-client migration/cutover complete |
| `FND-ID-01` | `ACCEPTED` | foundation semantic identity contract |
| `FND-02` | `ACCEPTED` | `protocol-oteryn` v1 semantic/wire architecture; implementation separately gated |
| `FND-03` | `ACCEPTED` | authoritative runtime execution architecture; implementation/capacity/OPS separately gated |
| `FND-04A/B/C` | `ACCEPTED AND LIFECYCLE-CLOSED` | fresh admission + reconnect/recovery + integration |
| `FND-04` overall | `ACCEPTED AND CLOSED` | programme #112 complete |
| `DUR-01` | `ACCEPTED AND LIFECYCLE-CLOSED` | PR #133 delivery merge `bc172d7244f823425feb84c46c7d04be1f942eed`; archived task carries exact CI/review evidence |
| `ANL-01` | `NEXT REQUIRED GATE` | required before DUR-02/DUR-03 finalize transactional outbox/critical audit evidence |
| `DUR-02` | `BLOCKED ON ANL-01 INPUT` | DUR-01 representation prerequisite is satisfied; Persistence v1 still needs ANL-01 audit/event semantics before final acceptance |
| `DUR-03` | `BLOCKED ON ANL-01 + DUR-02 INTEGRATION` | DUR-01 `ItemInstanceId` prerequisite is satisfied; anti-duplication design still needs audit/persistence integration |
| `DUR-04` | `QUEUED / INDEPENDENT DURABLE-CONTENT GATE` | content/world/scripting contract remains architecture work |
| `GAME-VISION-01` | `OPEN PRODUCT GATE` | still blocks broad gameplay/content production |

## 3. Accepted FND baseline

### FND-02

Canonical production protocol family is `oteryn`: TLS 1.3/TCP + ALPN, bounded framing/protobuf, exact foundation UUID wire representation, game-issued GameSessionId, `connection_generation`, `(GameSessionId, CommandId)` command identity, server sequence/domain revisions, snapshot/delta/resync and authenticated liveness primitives.

`CommandId` starts at 1, increases exactly by 1, is a full `uint64` and is scoped to one GameSessionId.

### FND-03

GameNode runtime uses one logical ordered mutation owner per ChannelRuntime/InstanceRuntime, separates NodeId from ownership generation, separates CommandId from runtime execution order, uses bounded queues/fail-closed stale work, and requires measured capacity rather than guessed limits.

### FND-04

FND-04 remains accepted/closed. Important frozen outcomes include ownership-before-world admission, separate presence/lease/session/transport/runtime authority, purpose-separated Ed25519 grant profiles, source-age `<=5s` + anti-rollback security evidence, PREPARE/COMMIT reconnect, healthy-binding non-preemption, ControlLossEpoch, exactly 4 seconds eligible PvE re-entry protection, stable-control protection re-arm and fail-closed GameNode recovery without guessed continuity.

## 4. Accepted DUR-01 durable representation

Canonical contract:

- `docs/architecture/DUR-01_DURABLE_IDENTIFIER_REPRESENTATION_CONTRACT.md`.

Supporting analysis:

- `docs/architecture/DUR-01_DURABLE_IDENTIFIER_REPRESENTATION_ANALYSIS.md`.

Accepted decisions:

- native UUIDv7-backed durable identities use PostgreSQL native `uuid`, preserving all 128 bits;
- Platform canonical `AccountId`, `WorldId` and `ChannelId` are consumed literally under Platform ADR 0028/0029; local integer/Canary identities never become native IDs by convenience;
- semantic strong types remain distinct although their physical PostgreSQL UUID scalar is common;
- nil UUID is invalid; optional semantic relations use NULL/typed absence;
- scoped identity retains `WorldId + component` semantics and cannot drop world scope;
- UUIDv7 ordering is never authority, fencing, causality or business chronology;
- persisted FND-02 `CommandId` uses PostgreSQL `numeric(20,0)` over `1..18446744073709551615` and remains logically `(GameSessionId, CommandId)`; signed bigint may not narrow the accepted range;
- `ItemInstanceId` is a game-owned, globally unique UUIDv7 identity for one concrete item-instance lifecycle and is never reused;
- generic catch-all EntityId/RowId domain identity is rejected;
- `EventId`, `OperationId`, `TransactionId`, `CorrelationId`, `CausationId`, `AnalyticsActorId` remain ANL-01-owned;
- no cross-database Platform/game foreign key is an authority proof;
- legacy import mapping uses stable `(source_system, source_namespace, source_entity_kind, legacy_identifier)` identity; snapshot/revision/import-run are provenance and cannot mint a second native identity for the same stable source entity;
- internal UUIDv7 values are not automatically public IDs because of correlation/creation-time leakage risk;
- representation changes require explicit lossless versioned migration, rollback and mixed-version evidence.

DUR-01 acceptance freezes representation, **not table/schema/runtime implementation**.

## 5. DUR-01 delivery evidence

- final PR #133 head: `1a431b4e0667a4a218ea15ed7152a7e979a1b072`;
- Agent Governance `31387230160`: PASS;
- Dependency Review `31387230532`: PASS;
- CodeQL `31387230664`: PASS;
- terminal architecture/security/data-integrity review `4896597698`: PASS, zero material findings;
- unresolved review threads: 0;
- repair budget used: `2/3`;
- squash delivery merge: `bc172d7244f823425feb84c46c7d04be1f942eed`;
- runtime/component/browser E2E: `NOT_APPLICABLE`.

## 6. Security/privacy boundary

- identity equality never equals authentication/authorization;
- persisted Platform-owned identity never becomes fresh Platform authority;
- AccountId/CharacterId/ItemInstanceId and other high-cardinality IDs are not ordinary metric labels;
- internal UUIDv7 public exposure requires product/privacy justification;
- legacy/import ambiguity fails closed rather than overwrite/re-key;
- persistence cannot reinterpret CommandId scope/range;
- analytics remains observational and consumes pseudonymized/audited identity under ANL contracts.

## 7. Runtime/implementation status

Accepted FND and DUR-01 architecture do **not** authorize:

- protocol listener/client gameplay adapter implementation;
- GameNode/admission/reconnect runtime implementation;
- PostgreSQL table/schema/migration/ORM implementation;
- CharacterLease persistence;
- item/currency transaction implementation;
- Platform migrations/writes;
- KMS/deployment/production traffic;
- entitlement/Premium/VIP activation;
- broad gameplay/content production.

## 8. Next ordered architecture work

The immediate dependency chain is now:

1. `ANL-01 — Game Event and Audit Foundation` — freeze event/audit identities, common envelope, durability classes, ordering/idempotency, outbox/publication checkpoints, privacy/retention and fixtures.
2. `DUR-02 — Persistence v1` — consume DUR-01 and ANL-01 to define schema/migrations/transactions/fencing/checkpoints/outbox/backup/restore/RPO/RTO.
3. `DUR-03 — Item Transaction and Anti-Duplication Invariants` — consume DUR-01 `ItemInstanceId`, DUR-02 transaction boundaries and ANL-01 critical evidence.
4. `DUR-04 — Content, World Detail and Scripting` — separate durable-content gate.
5. `GAME-VISION-01` — still required before broad gameplay/content production.

`PROD-ENTITLEMENTS-01` remains independently blocked by open P1 `Oteryn-Platform#944`; DUR-01 does not change that dependency.

## 9. Concise current rule

```text
FND-01 .. FND-04
-> accepted / closed as applicable

DUR-01
-> ACCEPTED AND LIFECYCLE-CLOSED
-> representation architecture only

ANL-01
-> NEXT REQUIRED GATE

DUR-02
-> DUR-01 satisfied; waits for ANL-01

DUR-03
-> ItemInstanceId satisfied; waits for ANL-01 + DUR-02 integration

PROD-ENTITLEMENTS-01
-> still blocked by Oteryn-Platform#944
```
