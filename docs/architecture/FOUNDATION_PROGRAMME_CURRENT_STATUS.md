# Oteryn v2 Foundation Programme — Current Status

- Status: Canonical current execution-status overlay
- Date: 2026-08-08
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: current foundation gate progression and next-action interpretation
- Does not supersede: accepted ADR semantics, dedicated accepted contract semantics, product decisions or historical evidence except where this file explicitly identifies stale progress/coordination wording

## Purpose

Keep the live foundation programme state unambiguous when long-lived backlog, global-register, gap-register, baseline or coordinator documents contain progress sentences written before later accepted closeouts and contracts.

This file is authoritative for **current execution status and gate readiness**. Accepted architecture semantics remain authoritative in ADRs and dedicated contracts. Stable gate definitions remain in `FOUNDATION_DECISION_BACKLOG.md`; the wider decision horizon remains in `GLOBAL_ARCHITECTURE_DECISION_REGISTER.md`.

Chat history is not execution authority.

## Accepted foundation evidence

### `FND-01` — workspace/dependency/migration foundation

- status: `ACCEPTED AND APPLIED`;
- canonical Rust workspace exists in `blakinio/Oteryn-v2`;
- destination client/workspace cutover PR #50 squash merge: `78988f72a80cc904aa9176ae850c50d4efa0b0f0`;
- old `blakinio/otclient` Rust client source is historical/non-canonical after its dedicated source-marker closeout;
- canonical workspace remains the accepted 19-member Rust workspace until later consumer-backed crate changes supersede it.

### `VSL-02` — client migration/cutover

- status: `COMPLETE`;
- canonical native Rust client path: `apps/client`;
- ADR-0011 `pre-native-protocol` fail-closed state remains binding until separately authorized native-protocol implementation.

### `FND-ID-01` — foundation identifiers

- status: `ACCEPTED AND MERGED`;
- canonical contract: `docs/architecture/FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md`;
- delivery PR #85 exact validated head: `6686a4b62c1e6b518d38ab3c80326b8621abe5bb`;
- delivery squash merge: `2c584543cd1e3758958755478a6cc6ed3d39a8a9`;
- lifecycle/status closeout PR #87 squash merge: `648aa10bb5b36d8826d82ed0f1ed94a47ca53a24`;
- coordination reconciliation issue #86 is complete.

The accepted foundation identity catalogue remains `AccountId`, `CharacterId`, `WorldId`, `ChannelId`, `NodeId`, `InstanceId`, `PartyId`, `GameSessionId` plus conditional `HandoffId`. `AdmissionId` and `CharacterLeaseId` are not invented by the foundation contract.

### `FND-02` — `protocol-oteryn` v1 foundation contract

- status: `ACCEPTED AND MERGED`;
- canonical contract: `docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md`;
- canonical foundation schema: `docs/contracts/protocol-oteryn/v1/foundation.proto`;
- canonical protocol registry: `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json`;
- canonical resource limits: `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`;
- delivery PR: `blakinio/Oteryn-v2#94`;
- exact validated delivery head: `91809204fcdf984a5a9d7b8c276ef9fb2f9cab9f`;
- Agent governance run `31225287340`: `PASS`;
- Dependency review run `31225287332`: `PASS`;
- CodeQL run `31225287334`: `PASS`;
- exact-head architecture/security audit review `4887157361`: `PASS`, zero open material findings;
- unresolved review threads: `0`;
- squash merge: `769ecd2ce2dfe0a7644d8dc1d67c54d40da5d202`;
- schema revision: `1`;
- schema SHA-256: `6e1c614661e72daac529be9d0ec06317201b916cd47ae17ff1590da5c7205ebe`;
- runtime/component/E2E implementation: `NOT IMPLEMENTED / NOT AUTHORIZED BY FND-02`.

`CROSS_REPOSITORY_CONTRACT_LOCK.json` records the immutable accepted local FND-02 revision while preserving the older Platform protocol revision as `RECONCILIATION_INPUT_ONLY`.

## Accepted disconnect/re-entry clarification

The reconnect/disconnect owner clarification is accepted, merged and closed out.

Canonical sources:

- `docs/architecture/DISCONNECT_REENTRY_PVE_PROTECTION_OWNER_DECISION.md`;
- `docs/architecture/DISCONNECT_CLIENT_OS_FORENSICS_OWNER_DIRECTION.md`;
- `docs/architecture/DISCONNECT_CLIENT_OS_FORENSICS_PRIVACY_TIMING_REFINEMENT.md`;
- `docs/architecture/DISCONNECT_FORENSIC_EVIDENCE_OWNER_BASELINE.md` remains the authoritative server-side forensic baseline;
- `docs/architecture/CLIENT_CRASH_DIAGNOSTICS_PRIVACY_OWNER_BASELINE.md` remains the privacy authority for automatic client-originated diagnostics.

Delivery evidence:

- PR #96 final exact head: `ddf62ec48443fb4ce119eed0900662e222a99726`;
- PR #96 squash merge: `496f0b9ad5231d24325e937a3f09ba221cf5c86b`;
- lifecycle closeout PR #97 squash merge: `b85bdd3f278d9de12284eab7c6352219325b3751`.

Binding semantics include:

- valid re-entry after **unexpected loss of playable control** receives exactly four seconds of defensive PvE protection;
- accepted graceful logout/login does not manufacture that protection window;
- movement, self-healing and health/mana/resource potion use remain legal under ordinary costs/cooldowns/exhaustion;
- the protected character cannot execute offensive actions against PvE monsters and prohibited outgoing actions are not buffered;
- the protected character cannot heal another player but may receive legal healing from another player;
- reconnect does not reset HP/resources/position/conditions/cooldowns/combat/PZ/logout state, threat/aggro, encounter state or committed effects;
- client/OS/Launcher/Guardian evidence is corroborating only; server-generated gameplay/liveness/runtime evidence remains authoritative;
- full Event Viewer/Event Log ingestion is rejected in favor of bounded, allowlisted, normalized incident evidence;
- automatic client-originated incident-capsule uploads remain governed by the existing global client-diagnostics opt-out;
- diagnostics opt-out or missing client evidence is not adverse evidence and cannot weaken server-side incident visibility;
- an independent client-side observer/Launcher/Guardian is an extension point, not a mandatory first implementation;
- a direct Guardian heartbeat remains separately gated by measured purpose/privacy/security/resource evidence;
- Game Intelligence may investigate longitudinal suspicious disconnect patterns but cannot autonomously sanction or mutate gameplay.

No kernel driver, invasive anti-cheat, production client diagnostics or enforcement implementation was authorized by that package.

## `FND-03` analysis evidence

The bounded FND-03 analysis gate is complete, merged and archived.

Canonical analysis inputs:

- `docs/architecture/FND-03_AUTHORITATIVE_RUNTIME_EXECUTION_ANALYSIS_BASELINE.md`;
- `docs/architecture/FND-03_RUNTIME_LIFECYCLE_FAILURE_AND_REPLAY_ANALYSIS_BASELINE.md`.

Delivery evidence:

- analysis PR #98 exact final head: `d46be7cda497de02ef671f7297a75d88f004cbbe`;
- Agent governance run `31269178770`: `PASS`;
- Dependency review run `31269178707`: `PASS`;
- CodeQL run `31269178709`: `PASS`;
- exact-head architecture audit review `4889306827`: `PASS`, zero open material findings;
- PR #98 squash merge: `86881713ac99877ae765f73bf2750867d450516b`;
- lifecycle closeout PR #100 squash merge: `a931c54e7e32b2cea370317ce88896a18eed8ccb`;
- duplicate/premature PR #99 and duplicate closeout PR #101 were closed unmerged and are non-authoritative.

The accepted analysis direction is:

```text
multithreaded GameNode
-> many independent authoritative scopes may progress concurrently
-> each ChannelRuntime/InstanceRuntime has one logical ordered mutation owner
-> logical owner is not a dedicated OS-thread contract
-> NodeId is process-incarnation identity, not scope authority
-> scope ownership generation is separate from NodeId and semantic ChannelId/InstanceId
-> FND-02 CommandId order remains separate from runtime execution order
-> wall clock, process monotonic time and authoritative execution order remain distinct
-> mutation-capable timers and async results re-enter through the current owner
-> stale generation/revision/local-handle work fails closed
-> control/fencing cannot starve behind ordinary gameplay backlog
-> all queues/executors are bounded
-> benchmark-sensitive numeric capacities require measured/safety evidence rather than architectural guessing
-> deterministic replay records normalized authoritative inputs/clocks/randomness/order, not original CPU/thread interleaving
```

The analysis intentionally did not itself complete FND-03 or authorize runtime implementation.

## Current `FND-03` final contract delivery

PR #102 (`docs/OTV2-20260808-fnd03-runtime-execution-final`) is the single bounded architecture-only final FND-03 delivery package.

Candidate contract:

- `docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md`.

The final contract freezes, subject to exact-head validation and merge:

- `NodeRuntime`, `WorldServices`, `ChannelRuntime` and `InstanceRuntime` responsibilities;
- semantic scope identity versus scope ownership generation versus NodeId placement;
- one owner-scoped `RuntimeExecutionOrdinal` as the runtime input linearization/evidence value, while domain/state revisions remain the committed-state authority;
- non-interleaved authoritative input resolution and FND-02 per-session CommandId ordering;
- wall-clock versus process-local monotonic versus runtime execution-order semantics;
- durable/recoverable timer boundary and bounded catch-up taxonomy;
- auxiliary worker/service pending/revalidation behavior;
- bounded queue/backpressure, control-lane, scope-fairness and slow-client semantics;
- scope activation, drain, checkpoint cut, fencing, failure containment and recovery;
- strict FND-03/FND-04 liveness/reconnect timing ownership split;
- deterministic authoritative gameplay randomness and simulation replay evidence;
- foundation error mapping and complete failure-scenario disposition;
- downstream ownership split with FND-04, DUR-*, ANL-*, PERF-01, OPS-CHANNEL-01 and gameplay contracts.

The contract deliberately does **not** select an async runtime, worker count, CPU affinity, fixed global tick/quantum, benchmark-sensitive numeric capacities, checkpoint storage/RPO/RTO, orchestrator product or heartbeat cadence.

Implementation acceptance will require concrete numeric runtime hard limits in `RESOURCE_LIMITS_REGISTRY.json` (or accepted successor) with boundary tests and safety/benchmark/spike evidence. This architecture package does not invent those numbers.

Delivery-state interpretation is transition-safe:

- while PR #102 remains open, its exact head is candidate delivery state and must not be represented as accepted architecture;
- after PR #102 squash-merges, `FND-03_RUNTIME_EXECUTION_CONTRACT.md` on `main` becomes the FND-03 semantic authority;
- after delivery merge, a separate task-lifecycle closeout archives/releases the FND-03 contract task without changing its semantics;
- runtime code still requires a separately explicit implementation task; FND-03 contract acceptance alone does not authorize implementation in this architecture-only programme.

## Current ordered foundation state

| Gate / programme step | Current status | Consequence |
|---|---|---|
| `FND-01` | `ACCEPTED AND APPLIED` | Workspace/dependency/migration contract is complete. |
| `VSL-02` | `COMPLETE` | Canonical migrated client/workspace exists; old source is historical/non-canonical. |
| `FND-ID-01` | `ACCEPTED AND MERGED` | Cross-boundary foundation identifier semantics are frozen. |
| `FND-02` | `ACCEPTED AND MERGED` | Native gameplay wire foundation is frozen; no runtime implementation claim is implied. |
| Disconnect/re-entry clarification | `ACCEPTED AND MERGED` | Mandatory input to runtime/session design; no diagnostics/enforcement implementation claim. |
| FND-03 analysis | `COMPLETE AND MERGED` | Two reviewed analysis baselines are canonical inputs to the final contract. |
| `FND-03` final contract | `DELIVERY = LIVE PR #102 / MAIN AFTER MERGE` | Complete runtime execution semantics are being validated; no runtime implementation is authorized by delivery alone. |
| `FND-04` | `BLOCKED ON OWN CONTRACT` | Admission/Game Session/lease/reconnect implementation remains unclaimed; final contract follows accepted FND-03. |
| `DUR-01`…`DUR-04`, `ANL-01`… | `LATER GATES` | Existing ordering and architecture requirements remain unchanged. |

## `FND-02` completion boundary

The accepted v1 foundation freezes the minimum wire mechanics required by downstream runtime/admission work:

- one native production family: `oteryn`;
- transport profile 1: TCP + TLS 1.3, ALPN `oteryn-game/1`, verified server identity, no TLS 0-RTT/plaintext/Canary fallback;
- framing: `uint32_be` payload length + one bounded protobuf `WireEnvelope`;
- Protocol Buffers binary with `proto3` source IDL for v1;
- schema revision/hash as immutable evidence metadata, **not** exact-equality runtime compatibility identity;
- stable numeric registries for foundation messages/errors and future additive capabilities;
- initially zero speculative optional capabilities; core v1 semantics are mandatory under protocol major 1;
- foundation UUID identities use exact 16-byte standard UUID wire representation when exposed;
- canonical game-domain `GameSessionId` is issued only after authoritative admission;
- `connection_generation` is a monotonic non-zero post-admission transport fence carried in both directions;
- `(GameSessionId, CommandId)` is the command identity, where `CommandId` is monotonic `uint64` and also defines total client-command order;
- bounded ordered command ingress permits safe pipelining while preventing duplicate execution;
- server-authoritative `server_sequence` and typed state-domain revisions continue across eligible reconnect of the same GameSession;
- snapshot/delta/resync is explicit, atomic and never guesses through gaps;
- replacement snapshots use a bounded sequencing barrier so later server-sequenced output cannot overtake `SnapshotCommit`;
- liveness uses authenticated probe/ack primitives and is not inferred from gameplay-command silence or socket-open state;
- hard peer-controlled wire/count/depth limits are concrete in `RESOURCE_LIMITS_REGISTRY.json`;
- future implementation acceptance requires independent byte fixtures, malformed corpus, property tests, fuzzing, cross-version fixtures, reconnect-fencing tests, pipelining tests and snapshot-barrier tests.

FND-02 deliberately does **not** select a concrete Rust protobuf/TLS library, implement listeners/codecs, define heartbeat cadence, define admission/reconnect credentials, define gameplay-specific movement/combat/content payloads, introduce QUIC/compression, or enable production traffic.

## External Platform reconciliation boundary

The merged `blakinio/Oteryn-Platform` protocol contract at `c0b8703d326a04b43ae8e06f6192b0cb91c859b7` remains immutable historical/reconciliation evidence. It is **not** final Oteryn-v2 protocol authority and remains classified `RECONCILIATION_INPUT_ONLY`.

The final local FND-02 authority is `blakinio/Oteryn-v2@769ecd2ce2dfe0a7644d8dc1d67c54d40da5d202` plus its canonical contract/schema/registry/limits.

A complete cross-repository rollout still requires a separately authorized `blakinio/Oteryn-Platform` task/branch/PR to reconcile Gateway/World Registry/session-offer structures with accepted FND-02. No Platform write or rollout is authorized by this status overlay.

## Historical progress wording

Historical ADRs, archived tasks, evidence snapshots and older coordination documents may retain progress-only sentences from before later foundation completion. Do not mass-rewrite historical evidence merely to make old timestamps read like current status.

`GLOBAL_ARCHITECTURE_DECISION_REGISTER.md`, `FOUNDATION_DECISION_BACKLOG.md` and other long-lived coordination files may contain stale progress sentences such as `FND-02 is next` or `FND-03 is next`. Those sentences are not current execution authority. This file and live GitHub state govern current progression; dedicated accepted contracts govern semantics.

## Current next action

Use live PR #102 state to select exactly one transition:

- **if PR #102 is open** — complete full diff/accepted-input review, exact-head repository checks and independent architecture/security audit; squash merge only with zero open material findings;
- **if PR #102 is merged** — perform its separate task-lifecycle archive/ownership release, then begin one bounded architecture-only `FND-04 Identity, Game Session, Admission and Character Lease Contract` package from current `main`.

Do **not** start authoritative runtime implementation from this architecture-only programme merely because FND-03 merges. Runtime implementation requires separately explicit implementation authority and the concrete resource-limit evidence required by FND-03.

`GAME-VISION-01` analysis may continue in parallel when it does not redefine accepted foundation identity, repository, protocol, runtime, Platform or persistence boundaries.

## Non-authorization

This status update does **not** authorize implementation of the Rust gameplay server runtime, production `protocol-oteryn` listener/client adapter, Game Session admission/lease, persistence schemas, Platform changes, client/Windows diagnostics, Launcher/Guardian, Game Intelligence detectors, sanctions, production deployment or live operations. Those remain behind their own explicit implementation gates and/or owner authorization.
