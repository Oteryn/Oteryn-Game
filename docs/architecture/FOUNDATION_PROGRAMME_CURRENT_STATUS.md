# Oteryn v2 Foundation Programme — Current Status

- Status: Canonical current execution-status overlay
- Date: 2026-08-10
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: current foundation gate progression and next-action interpretation
- Trusted main at FND-04C start: `3d07b3faaca683514fdfe6291e974f9195e2f763`
- Active final foundation delivery: Issue #130 (`FND-04C`)
- Current phase represented by this delivery candidate: `FINAL_DELIVERY / CLOSEOUT_PENDING`

## 1. Authority of this overlay

This document answers **what is accepted now and what may happen next**. Detailed historical review/CI/repair evidence lives in accepted contracts, archived task records and merged PRs.

If an older coordination/backlog/register paragraph still describes FND-02, FND-03 or FND-04 as the current live gate, that wording is historical execution narrative. Accepted contracts plus this overlay govern current progression. Stable decision IDs and future dependency requirements in the global register/backlog remain valid unless explicitly superseded.

No status row below implies runtime implementation unless explicitly stated.

## 2. Foundation progression summary

| Gate | Current status | Canonical evidence / note |
|---|---|---|
| `FND-01` | `ACCEPTED AND APPLIED` | workspace/dependency contract + canonical 19-member destination cutover; destination merge `78988f72a80cc904aa9176ae850c50d4efa0b0f0` |
| `VSL-02` | `ACCEPTED AND COMPLETE` | exact Rust-client migration/cutover contract; source-only historical/non-canonical closeout completed |
| `FND-ID-01` | `ACCEPTED` | `FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md`; merge `2c584543cd1e3758958755478a6cc6ed3d39a8a9` |
| `FND-02` | `ACCEPTED` | `protocol-oteryn` v1 contract; PR #94 merged as `769ecd2ce2dfe0a7644d8dc1d67c54d40da5d202`; runtime codec/listener implementation remains separately gated |
| `FND-03` | `ACCEPTED` | `FND-03_RUNTIME_EXECUTION_CONTRACT.md`; PR #102 merged as `e72f2514924e8bbf8d1a729721cce9e67d977544`; runtime implementation/capacity/OPS claims remain separately gated |
| `FND-04A` | `ACCEPTED AND LIFECYCLE-CLOSED` | fresh admission authority/profile; PR #125 merge `cae318b8891844891c734012eb2020e669ebaff4`; closeout #126 `2fd7bac4879f381d5b97230732076df2e9c61f95` |
| `FND-04B` | `ACCEPTED AND LIFECYCLE-CLOSED` | reconnect/recovery/continuity + recovery profile; PR #128 merge `e6282b9c48713b2a2980f2598a81775f78725cff`; closeout #129 `3d07b3faaca683514fdfe6291e974f9195e2f763` |
| `FND-04C` | `FINAL DELIVERY / CLOSEOUT PENDING` | Issue #130 owns final errors/diagnostics/failure/compatibility integration. Before its delivery merge this means final candidate validation; after delivery merge it means lifecycle archival/ownership release still remains. |
| `FND-04` overall | `INCOMPLETE UNTIL FND-04C DELIVERY + LIFECYCLE CLOSEOUT` | A/B accepted; C delivery and its closeout are the only remaining programme #112 steps |

The FND-04C lifecycle closeout MUST update this overlay again to `FND-04C = ACCEPTED AND LIFECYCLE-CLOSED` and `FND-04 = ACCEPTED AND CLOSED`; therefore the same text is correct both immediately before and immediately after the FND-04C delivery merge.

## 3. Accepted FND-02 baseline

Canonical production protocol family is `oteryn`.

Accepted foundation includes TCP + TLS 1.3; ALPN `oteryn-game/1`; verified server identity; no TLS 0-RTT/plaintext/Canary fallback; `uint32_be` bounded framing + protobuf/proto3; exact 16-byte wire form for exposed foundation UUIDs; post-admission GameSessionId; monotonic non-zero `connection_generation`; `(GameSessionId, CommandId)` command identity with monotonic uint64 CommandId order; server_sequence + typed state-domain revisions; explicit snapshot/delta/resync; authenticated liveness probe/ack; registered hard limits/stable protocol errors; and independent byte/malformed/property/fuzz/cross-version evidence before implementation claims.

FND-02 `schema_revision` is diagnostic/build evidence, not an opaque exact gameplay compatibility token.

## 4. Accepted FND-03 baseline

Canonical runtime semantics include a multithreaded GameNode with many authoritative scopes; one logical ordered mutation owner per ChannelRuntime/InstanceRuntime; NodeId as process-incarnation/placement rather than authority; separate scope ownership generation; execution order distinct from CommandId; separate wall/monotonic/execution clocks; mutation-capable async/timer work re-entering through the current owner; stale generation/revision/handle work failing closed; bounded queues; non-starvable fencing/control work; measured rather than guessed capacity; and deterministic replay based on normalized authoritative inputs/order rather than CPU/thread interleaving.

No async runtime product, worker count, CPU affinity, global tick numeric capacity, orchestrator or heartbeat cadence is selected merely by FND-03 acceptance.

## 5. Accepted FND-04A baseline

Fresh admission canonically requires Platform bounded-attempt authority with Oteryn-v2 final game-domain admission authority; AccountId->CharacterId before CharacterId->WorldId eligibility including final revalidation; stale-world no-retarget/no partial authority; separate presence/lease/session/transport/runtime authority; one atomic final admission boundary; strict purpose-separated Ed25519 fresh profile with verifier-anchored trust and deterministic crypto/schema/binding/profile precedence; independent protocol/transport/ruleset/content/map/world-policy/offer revisions; and authenticated Platform-security + signing-key/profile source-age <=5s with monotonic anti-rollback, no cache re-aging and fail-closed restart-floor reconstruction.

## 6. Accepted FND-04B baseline

Reconnect/recovery canonically requires one current playable connection generation; healthy-binding non-preemption; PREPARE without authority and atomic COMMIT revalidation/switch; inactive candidate successor proof activation with predecessor fencing and lost-response reconciliation; same-session grace originating at server-authoritative ControlLossEpoch; grace/protection/re-arm state surviving failover without timer restart; exact 4-second defensive PvE protection as the only frozen reconnect/gameplay timing; stable-control re-arm before any later protection entitlement; historical `2s/5s/15s` candidate timings non-canonical; purpose-separated Ed25519 recovery profile; independent protocol/transport/ruleset/content/map/world-policy revisions; ownership before recovery world/actor/controller classification; same-session state continuity; post-grace new GameSession attached to the same unreset actor; GameNode continuity only from complete fenced evidence; and no ordinary reconnect HandoffId invention.

## 7. FND-04C acceptance target

FND-04C delivery must provide, on one accepted exact head:

- complete FND-04 cross-component error catalogue conforming Foundation Error Vocabulary;
- stable redacted diagnostics + credential-free correlation per error;
- historical superseded error aliases explicitly non-canonical where A/B already own the condition;
- Foundation failure scenarios for admission/recovery/reconnect replay/races, lost responses, security/key rotation, ownership/world changes, GameNode continuity ambiguity and protection re-arm/failover;
- independent producer/consumer compatibility and rollout/rollback rules;
- implementation evidence gates;
- security/privacy integration;
- thin final `FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md` linking A/B/C without duplication;
- this transition-safe current-status overlay.

After delivery merges, only lifecycle archival/ownership release remains. Closeout then updates this overlay to final accepted/closed state and closes Issue #130 + programme #112.

## 8. Deliberately deferred implementation-sensitive decisions

FND-04 acceptance does not freeze unmeasured implementation values. Still deferred: CharacterLease TTL/renew/safety/fail-safe deadlines; liveness cadence/hysteresis/control-loss threshold; same-session grace duration; stale-transport cleanup; stable-control protection re-arm threshold; prepared reconnect/resource/rate/retention limits; async runtime/worker/affinity choices; production capacity/scaling; persistence schema/isolation/outbox/RPO/RTO; KMS/HSM vendor; healthy-session migration protocol; and handoff implementation details beyond accepted fencing extension points.

These are not permission to use defaults. Owning DUR/OPS/PERF/future contracts must accept measured evidence before implementation can claim correctness.

## 9. Runtime/implementation status

Foundation architecture acceptance is **not runtime implementation**. Current architecture work does not authorize production protocol listener/client gameplay adapter, GameNode/session/admission/reconnect runtime, CharacterLease persistence, Platform issuer/security-projection writes, production keys/KMS, deployment/traffic cutover, entitlement/Premium/VIP implementation or broad gameplay/content production.

The migrated Rust client remains governed by ADR-0011 pre-native-protocol fail-closed requirements until separately authorized implementation proves FND-02/FND-03/FND-04 conformance.

## 10. Next ordered work after FND-04 closes

After FND-04C lifecycle closeout:

1. `ANL-01 — Game Event and Audit Foundation` must be accepted before `DUR-02`/`DUR-03` finalize transactional outbox and critical audit evidence.
2. `DUR-01 — Durable Identifier Representation Contract` remains required before authoritative durable gameplay representation.
3. `DUR-02 — Persistence v1` and `DUR-03 — Item Transaction and Anti-Duplication Invariants` remain hard gates before authoritative durable character/item/currency mutation and consume required DUR-01/ANL-01 decisions.
4. `DUR-04 — Content, World Detail and Scripting` remains required before broad import/scripted durable behavior.
5. `GAME-VISION-01` still blocks broad gameplay/content production and must define measurable reference-vs-evolved launch/parity strategy.

ANL consumers remain observational and never replace authoritative invariants; DUR-03 prevents duplication while later ANL-03 detects/investigates economy integrity anomalies.

## 11. Concise rule

```text
FND-01 / VSL-02 / FND-ID-01 / FND-02 / FND-03
-> accepted

FND-04A
-> accepted + closed

FND-04B
-> accepted + closed

FND-04C
-> final delivery / closeout pending

FND-04C delivery + lifecycle closeout
-> FND-04 ACCEPTED AND CLOSED
-> programme #112 closes

then
-> ANL-01 + DUR-01 dependency work
-> DUR-02 / DUR-03 only after required inputs
-> no broad gameplay/content production before GAME-VISION-01
```
