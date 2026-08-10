# Oteryn v2 Foundation Programme — Current Status

- Status: Canonical current execution-status overlay
- Date: 2026-08-10
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: accepted foundation progression and next ordered architecture gates
- FND-04 final delivery merge: `cdca8f0ad2c8267c7533e52a4f9a48dc128b231d`
- Current phase represented by this closeout candidate: `FOUNDATION FND-04 COMPLETE / NEXT STAGE-B GATES`

## 1. Authority of this overlay

This document answers **what is accepted now and what may happen next**. Detailed review/CI/repair evidence lives in accepted contracts, archived task records and merged PRs.

If an older coordination/backlog/register paragraph describes FND-02, FND-03 or FND-04 as a live gate, that wording is historical execution narrative. Accepted contracts plus this overlay govern current progression. Stable decision IDs and future dependency requirements in the global register/backlog remain valid unless explicitly superseded.

No status row implies runtime implementation unless explicitly stated.

## 2. Foundation progression summary

| Gate | Current status | Canonical evidence / note |
|---|---|---|
| `FND-01` | `ACCEPTED AND APPLIED` | workspace/dependency contract + canonical 19-member destination cutover; destination merge `78988f72a80cc904aa9176ae850c50d4efa0b0f0` |
| `VSL-02` | `ACCEPTED AND COMPLETE` | exact Rust-client migration/cutover contract; source-only historical/non-canonical closeout completed |
| `FND-ID-01` | `ACCEPTED` | `FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md`; merge `2c584543cd1e3758958755478a6cc6ed3d39a8a9` |
| `FND-02` | `ACCEPTED` | `protocol-oteryn` v1 contract; PR #94 merge `769ecd2ce2dfe0a7644d8dc1d67c54d40da5d202`; runtime codec/listener implementation remains separately gated |
| `FND-03` | `ACCEPTED` | `FND-03_RUNTIME_EXECUTION_CONTRACT.md`; PR #102 merge `e72f2514924e8bbf8d1a729721cce9e67d977544`; runtime implementation/capacity/OPS claims remain separately gated |
| `FND-04A` | `ACCEPTED AND LIFECYCLE-CLOSED` | fresh admission authority/profile; PR #125 merge `cae318b8891844891c734012eb2020e669ebaff4`; closeout #126 `2fd7bac4879f381d5b97230732076df2e9c61f95` |
| `FND-04B` | `ACCEPTED AND LIFECYCLE-CLOSED` | reconnect/recovery/continuity + recovery profile; PR #128 merge `e6282b9c48713b2a2980f2598a81775f78725cff`; closeout #129 `3d07b3faaca683514fdfe6291e974f9195e2f763` |
| `FND-04C` | `ACCEPTED AND LIFECYCLE-CLOSED` | final errors/diagnostics/failure/compatibility integration; PR #131 delivery merge `cdca8f0ad2c8267c7533e52a4f9a48dc128b231d`; active ownership is released by the closeout carrying this overlay |
| `FND-04` overall | `ACCEPTED AND CLOSED` | A/B/C semantic architecture accepted; all component task ownership released by their lifecycle closeouts |
| programme `#112` | `COMPLETE` | replacement FND-04 programme has no remaining semantic architecture gate after this closeout merges |

## 3. Accepted FND-02 baseline

Canonical production protocol family is `oteryn`.

Accepted foundation includes TCP + TLS 1.3, ALPN `oteryn-game/1`, verified server identity, no TLS 0-RTT/plaintext/Canary fallback, bounded `uint32_be` framing + protobuf/proto3, exact 16-byte wire foundation UUID form, authoritative post-admission GameSessionId, monotonic non-zero `connection_generation`, `(GameSessionId, CommandId)` identity, server_sequence + typed state-domain revisions, explicit snapshot/delta/resync, authenticated liveness probe/ack, registered hard limits/stable errors and required independent byte/malformed/property/fuzz/cross-version evidence before implementation claims.

FND-02 `schema_revision` remains diagnostic/build evidence, not an opaque gameplay compatibility token.

## 4. Accepted FND-03 baseline

Canonical runtime semantics include multithreaded GameNode operation with many authoritative scopes; one logical ordered mutation owner per ChannelRuntime/InstanceRuntime; NodeId as process-incarnation/placement rather than authority; separate scope ownership generation; execution order distinct from CommandId; separate wall/monotonic/execution clocks; mutation-capable async/timer work re-entering through current owner; stale generation/revision/handle work failing closed; bounded queues; non-starvable fencing/control work; measured rather than guessed capacity; and deterministic replay from normalized authoritative inputs/order rather than CPU/thread interleaving.

No async runtime product, worker count, CPU affinity, global tick numeric capacity, orchestrator or heartbeat cadence is selected merely by FND-03 acceptance.

## 5. Accepted FND-04 architecture

The thin canonical integration index is:

- `docs/architecture/FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md`.

Its normative component contracts are:

- FND-04A: `docs/architecture/FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md` + `docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md`;
- FND-04B: `docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md` + `docs/contracts/FND-04_REAUTHENTICATED_RECOVERY_GRANT_PROFILE_V1.md`;
- FND-04C: `docs/architecture/FND-04C_ERROR_DIAGNOSTICS_FAILURE_COMPATIBILITY_CONTRACT.md` + `docs/contracts/FOUNDATION_FAILURE_SCENARIOS.md`.

### Fresh admission

- Platform authenticates/authorizes one bounded attempt; Oteryn-v2 alone decides current game-domain admission.
- AccountId->CharacterId ownership is proven before CharacterId->WorldId/world eligibility and both are revalidated at the atomic authority boundary.
- stale world never retargets an old grant and creates no candidate nonce/presence/lease/session/transport authority.
- AccountPresenceClaim, CharacterLease, GameSession, TransportBinding and RuntimeScopeAuthority remain distinct.
- fresh grant uses purpose-separated fully specified Ed25519, verifier-anchored trust and deterministic crypto/schema/binding/profile precedence.
- protocol/transport/ruleset/content/map/world-policy/offer revisions remain independent; no opaque `compatibility_revision`.
- Platform-security and key/profile trust require authenticated source observation age `<=5s`, anti-rollback ordering, no cache re-aging and fail-closed restart-floor reconstruction.

### Reconnect/recovery/continuity

- one current playable `connection_generation` per GameSession;
- healthy current binding is non-preemptible by reconnect bearer proof or recovery JWT;
- PREPARE has no gameplay/liveness/fencing authority; COMMIT revalidates current authority and atomically switches one winner;
- inactive candidate successor reconnect proof may be delivered before COMMIT, becomes current only at COMMIT and predecessor proof never revives;
- same-session grace begins at server-authoritative `ControlLossEpoch`, not socket close/first missed probe/cleanup/reconnect attempt;
- original grace/protection/re-arm state and deadlines survive GameNode/runtime-owner replacement without restart;
- valid eligible re-entry activates exactly 4 seconds defensive PvE protection at most once per eligible entitlement;
- successful control restoration does not automatically re-arm protection; server-authoritative stable-control evidence is required before a later unexpected loss may receive another entitlement;
- historical candidate reconnect/liveness timings `2s/5s/15s` are non-canonical;
- recovery uses its own purpose-separated Ed25519 profile and independent protocol/transport/ruleset/content/map/world-policy revisions;
- ownership precedes recovery world/actor/controller classification;
- same-session recovery preserves GameSessionId/CommandId/server_sequence/domain revisions and actor state;
- post-grace existing-actor recovery creates a new GameSessionId and never respawns/resets/teleports/heals the actor;
- GameNode replacement may preserve same-session continuity only from complete fenced recoverable evidence; NodeId/restart never guesses authority or restarts deadlines;
- ordinary reconnect does not manufacture HandoffId.

### Error/failure/compatibility integration

- one canonical FND-04 error catalogue defines category, progression, exact retry authority, mutation/idempotency outcome, bounded public class, redacted diagnostic and credential-free correlation;
- superseded aliases are explicitly non-canonical where A/B/C own the condition;
- every Foundation Failure Scenario registered at FND-04C acceptance has an explicit FND-04 disposition using only `PASS`, `NOT_APPLICABLE`, `BLOCKED`, `DEFERRED_BY_ACCEPTED_GATE`;
- producer/consumer rollout is purpose/profile/key-purpose exact, fully specified Ed25519, verifier-anchored, source-age `<=5s` + anti-rollback and independently revisioned; no fresh/recovery/Canary reinterpretation or downgrade;
- rollback cannot restore older allow/trust/revision authority or revive terminal GameSession, consumed nonce, stale reconnect proof or old runtime owner;
- implementation evidence is required for credential interoperability, replay/races, fencing, lost responses, exact 4-second protection/re-arm, failover, actor preservation, diagnostics and privacy.

## 6. Exact timing/security values accepted by FND-04

Accepted exact/profile ceilings:

- fresh/recovery grant maximum lifetime: 30 seconds;
- fresh/recovery verifier skew: 5 seconds;
- authenticated Platform-security/signing-key/profile source-age ceiling: `<=5 seconds`;
- defensive PvE protection after eligible valid re-entry: exactly 4 seconds.

FND-04 intentionally does **not** freeze numeric values for liveness probe cadence/hysteresis/control-loss threshold, stale transport cleanup, same-session grace, stable-control protection re-arm threshold, CharacterLease TTL/renew/safety deadlines, prepared reconnect/resource/rate limits, recovery locator/cache/resource limits or relevant queue caps.

Those values must be finite and measured in their owning DUR/OPS/PERF/future registry gates before runtime activation. Historical defaults are not implementation authority.

## 7. Security/privacy boundary

- server game/liveness/runtime evidence decides gameplay control/loss/protection;
- client/OS/Launcher/Guardian evidence is corroborative only;
- diagnostics opt-out remains respected and missing client evidence is not adverse;
- no broad Windows Event Log ingestion, kernel driver, invasive anti-cheat or mandatory fingerprint is required;
- Game Intelligence may investigate bounded/audited patterns but cannot autonomously sanction, mutate, fence, reconnect or recover gameplay;
- raw credentials/proofs/nonces/private keys/private fence values never enter ordinary telemetry;
- AccountId/CharacterId are not ordinary high-cardinality metric labels.

## 8. Runtime/implementation status

`FND-04 = ACCEPTED AND CLOSED` means **semantic architecture complete**, not runtime implemented.

This programme does not authorize:

- production protocol listener/client gameplay adapter;
- GameNode/session/admission/reconnect/recovery runtime implementation;
- CharacterLease persistence/schema;
- Platform issuer/security-projection writes;
- production signing keys/KMS/HSM;
- deployment/traffic cutover;
- entitlement/Premium/VIP implementation;
- broad gameplay/content production.

Any implementation must prove the FND-04C evidence matrix plus downstream DUR/OPS/PERF decisions for deferred physical/numeric values.

## 9. Next ordered architecture work

With the identity/protocol/runtime/admission foundation chain closed, next dependency work is:

1. `ANL-01 — Game Event and Audit Foundation` — required before DUR-02/DUR-03 finalize transactional outbox and critical audit evidence.
2. `DUR-01 — Durable Identifier Representation Contract` — required before authoritative durable gameplay representation.
3. `DUR-02 — Persistence v1` and `DUR-03 — Item Transaction and Anti-Duplication Invariants` — hard gates before authoritative durable character/item/currency mutation; consume required ANL-01/DUR-01 decisions.
4. `DUR-04 — Content, World Detail and Scripting` — required before broad durable/scripted content behavior.
5. `GAME-VISION-01` — still blocks broad gameplay/content production and must define measurable reference-vs-evolved launch/parity strategy.

ANL consumers remain observational and never replace authoritative invariants; DUR-03 prevents duplication while later ANL-03 detects/investigates economy integrity anomalies.

## 10. Concise current rule

```text
FND-01 / VSL-02 / FND-ID-01 / FND-02 / FND-03
-> accepted

FND-04A / FND-04B / FND-04C
-> accepted + lifecycle-closed

FND-04 overall
-> ACCEPTED AND CLOSED

programme #112
-> COMPLETE

runtime implementation
-> still separately gated and not authorized by this architecture closeout

next
-> ANL-01 + DUR-01
-> DUR-02 / DUR-03 after required inputs
-> DUR-04 for durable content/scripting
-> no broad gameplay/content production before GAME-VISION-01
```
