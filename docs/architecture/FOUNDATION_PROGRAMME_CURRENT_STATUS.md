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

## Current owner clarification package — disconnect/re-entry

PR #96 (`docs/OTV2-20260808-reentry-pve-protection`) is the bounded architecture-only delivery vehicle for the owner-accepted reconnect-protection clarification that `FND-03`/`FND-04` must consume before freezing dependent runtime/session semantics.

Package sources are:

- `docs/architecture/DISCONNECT_REENTRY_PVE_PROTECTION_OWNER_DECISION.md`;
- `docs/architecture/DISCONNECT_CLIENT_OS_FORENSICS_OWNER_DIRECTION.md`;
- `docs/architecture/DISCONNECT_CLIENT_OS_FORENSICS_PRIVACY_TIMING_REFINEMENT.md`;
- existing `docs/architecture/DISCONNECT_FORENSIC_EVIDENCE_OWNER_BASELINE.md` remains the server-side forensic authority refined by the client-side direction;
- existing `docs/architecture/CLIENT_CRASH_DIAGNOSTICS_PRIVACY_OWNER_BASELINE.md` remains the privacy authority for automatic client-originated diagnostics.

Owner-accepted semantics and binding consistency rules captured by the package:

- valid re-entry after **unexpected loss of playable control** receives exactly four seconds of defensive PvE protection;
- accepted graceful logout/login does not manufacture that protection window;
- movement, self-healing and health/mana/resource potion use remain legal under ordinary costs/cooldowns/exhaustion;
- the protected character cannot execute offensive actions against PvE monsters and prohibited outgoing actions are not buffered;
- the protected character cannot heal another player but may receive legal healing from another player;
- reconnect does not reset HP/resources/position/conditions/cooldowns/combat/PZ/logout state, threat/aggro, encounter state or committed effects;
- client/OS/Launcher/Guardian evidence is corroborating only; server-generated gameplay/liveness/runtime evidence remains authoritative;
- full Event Viewer/Event Log ingestion is rejected in favor of bounded, allowlisted, normalized incident evidence;
- separate investigative evidence classes are preserved for graceful exit, client crash, abrupt process loss, NIC/interface loss, administrative interface change, path loss, system crash/power interruption, Oteryn-side infrastructure failure and unknown cases;
- automatic client-originated incident-capsule uploads remain governed by the existing global client-diagnostics opt-out regardless of whether evidence is assembled by the game client or a later Launcher/Guardian/helper;
- diagnostics opt-out or missing client evidence is not adverse evidence and cannot weaken server-side incident visibility;
- an independent client-side observer/Launcher/Guardian is an extension point, not a mandatory first implementation or required separate process;
- a separate Guardian diagnostic heartbeat remains a later candidate requiring its own measured purpose/privacy/security/resource contract and cannot be introduced as a silent opt-out bypass;
- longitudinal Game Intelligence may correlate combat risk, HP/resources, incoming pressure, reconnect timing, protection use, healing/potions, escape outcome, client/OS/Guardian evidence and infrastructure correlation to detect suspicious or unusually deterministic patterns;
- the analytical target is abuse of disconnect protection, not proof of one exact physical/software disconnect mechanism;
- mechanical protection and retrospective abuse analysis are intentionally separate: an episode may receive protection and still be investigated later;
- no single disconnect, client event or analytics score authorizes an automatic sanction; enforcement remains separately governed and human reviewed;
- no kernel driver or invasive anti-cheat is authorized by this clarification.

Delivery-state interpretation is intentionally transition-safe:

- while PR #96 remains open, the PR head and its dedicated documents are the delivery/validation authority;
- after PR #96 is squash-merged, the resulting documents on `main` become canonical without requiring this file to retain a stale `PR #96 VALIDATING` statement;
- in both states `FND-03` remains the next ordered foundation gate, but its final contract must consume the delivered clarification and must not infer a mandatory Launcher/Guardian implementation.

## Current ordered foundation state

| Gate / programme step | Current status | Consequence |
|---|---|---|
| `FND-01` | `ACCEPTED AND APPLIED` | Workspace/dependency/migration contract is complete. |
| `VSL-02` | `COMPLETE` | Canonical migrated client/workspace exists; old source is historical/non-canonical. |
| `FND-ID-01` | `ACCEPTED AND MERGED` | Cross-boundary foundation identifier semantics are frozen. |
| `FND-02` | `ACCEPTED AND MERGED` | Native gameplay wire foundation is frozen; no runtime implementation claim is implied. |
| Disconnect/re-entry owner clarification | `OWNER-ACCEPTED; DELIVERY STATE = LIVE PR #96 / MAIN AFTER MERGE` | Must be consumed by `FND-03`/`FND-04`; no runtime implementation is authorized. |
| `FND-03` | `NEXT ORDERED GATE` | Define authoritative Rust runtime execution, scheduling, queueing, ownership/fencing and recovery after consuming the disconnect clarification. |
| `FND-04` | `BLOCKED ON OWN CONTRACT` | Admission/Game Session/lease/reconnect implementation remains unclaimed. |
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
- hard peer-controlled wire/count/depth limits are now concrete in `RESOURCE_LIMITS_REGISTRY.json`;
- future implementation acceptance requires independent byte fixtures, malformed corpus, property tests, fuzzing, cross-version fixtures, reconnect-fencing tests, pipelining tests and snapshot-barrier tests.

FND-02 deliberately does **not** select a concrete Rust protobuf/TLS library, implement listeners/codecs, define heartbeat cadence, define admission/reconnect credentials, define gameplay-specific movement/combat/content payloads, introduce QUIC/compression, or enable production traffic.

## External Platform reconciliation boundary

The merged `blakinio/Oteryn-Platform` protocol contract at `c0b8703d326a04b43ae8e06f6192b0cb91c859b7` remains immutable historical/reconciliation evidence. It is **not** final Oteryn-v2 protocol authority and remains classified `RECONCILIATION_INPUT_ONLY`.

The final local FND-02 authority is now `blakinio/Oteryn-v2@769ecd2ce2dfe0a7644d8dc1d67c54d40da5d202` plus its canonical contract/schema/registry/limits.

A complete cross-repository rollout still requires a separately authorized `blakinio/Oteryn-Platform` task/branch/PR to reconcile Gateway/World Registry/session-offer structures with accepted FND-02. No Platform write or rollout is authorized by this closeout.

## Historical progress wording

Historical ADRs, archived tasks, evidence snapshots and older coordination documents may retain progress-only sentences from before FND-ID/FND-02 completion. Do not mass-rewrite historical evidence merely to make old timestamps read like current status.

`GLOBAL_ARCHITECTURE_DECISION_REGISTER.md` and other long-lived coordination files may still contain stale progress sentences such as `FND-02 is next`. Those sentences are not current execution authority. For current execution status, this file and live GitHub state are authoritative. For protocol semantics, `FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md` and its machine-readable companions are authoritative. For disconnect/re-entry semantics, use the dedicated delivered package above; live PR state determines whether its delivery authority is PR #96 or the merged `main` result.

## Current next action

Use live PR #96 state to select exactly one transition:

- **if PR #96 is still open** — complete its exact-head validation/audit and squash merge when every gate passes;
- **if PR #96 is merged** — create/continue one bounded **architecture-only `FND-03` Runtime Execution Contract** task in `blakinio/Oteryn-v2`.

`FND-03` must consume at minimum:

- ADR-0001 native Rust/multichannel authority rules;
- ADR-0009 GameNode execution/capacity/deployment/recovery baseline;
- `FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md`;
- `FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md`;
- `RESOURCE_LIMITS_REGISTRY.json`;
- `FOUNDATION_ERROR_VOCABULARY.md`;
- `FOUNDATION_FAILURE_SCENARIOS.md`;
- accepted instance/runtime, disconnect/liveness and reconnect-generation baselines;
- `DISCONNECT_REENTRY_PVE_PROTECTION_OWNER_DECISION.md` after delivery;
- `DISCONNECT_CLIENT_OS_FORENSICS_OWNER_DIRECTION.md` together with `DISCONNECT_CLIENT_OS_FORENSICS_PRIVACY_TIMING_REFINEMENT.md` for client-side forensic authority/privacy/timing boundaries;
- `DISCONNECT_FORENSIC_EVIDENCE_OWNER_BASELINE.md` for authoritative server-side evidence semantics.

It must decide runtime execution/ownership boundaries needed to implement the protocol safely without redefining FND-02 wire semantics. Exact client/OS diagnostic APIs, existence/topology of a Launcher/Guardian, Guardian heartbeat cadence/transport, enforcement thresholds and production telemetry remain outside `FND-03` unless a separately accepted owning contract proves they are required there.

`GAME-VISION-01` analysis may continue in parallel when it does not redefine accepted foundation identity, repository, protocol, Platform or persistence boundaries.

## Non-authorization

This closeout does **not** authorize implementation of the Rust gameplay server runtime, production `protocol-oteryn` listener/client adapter, Game Session admission/lease, persistence schemas, Platform changes, client/Windows diagnostics, Launcher/Guardian, Game Intelligence detectors, sanctions, production deployment or live operations. Those remain behind their own explicit implementation gates.
