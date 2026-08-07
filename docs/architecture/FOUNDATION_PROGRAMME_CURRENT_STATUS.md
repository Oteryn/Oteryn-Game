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

## Current ordered foundation state

| Gate / programme step | Current status | Consequence |
|---|---|---|
| `FND-01` | `ACCEPTED AND APPLIED` | Workspace/dependency/migration contract is complete. |
| `VSL-02` | `COMPLETE` | Canonical migrated client/workspace exists; old source is historical/non-canonical. |
| `FND-ID-01` | `ACCEPTED AND MERGED` | Cross-boundary foundation identifier semantics are frozen. |
| `FND-02` | `ACCEPTED AND MERGED` | Native gameplay wire foundation is frozen; no runtime implementation claim is implied. |
| `FND-03` | `NEXT ORDERED GATE` | Define authoritative Rust runtime execution, scheduling, queueing, ownership/fencing and recovery contract. |
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

For current execution status, this file and live GitHub state are authoritative. For protocol semantics, `FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md` and its machine-readable companions are authoritative.

## Current next action

Create/continue one bounded **architecture-only `FND-03` Runtime Execution Contract** task in `blakinio/Oteryn-v2`.

`FND-03` must consume at minimum:

- ADR-0001 native Rust/multichannel authority rules;
- ADR-0009 GameNode execution/capacity/deployment/recovery baseline;
- `FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md`;
- `FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md`;
- `RESOURCE_LIMITS_REGISTRY.json`;
- `FOUNDATION_ERROR_VOCABULARY.md`;
- `FOUNDATION_FAILURE_SCENARIOS.md`;
- accepted instance/runtime, disconnect/liveness and reconnect-generation baselines.

It must decide runtime execution/ownership boundaries needed to implement the protocol safely without redefining FND-02 wire semantics.

`GAME-VISION-01` analysis may continue in parallel when it does not redefine accepted foundation identity, repository, protocol, Platform or persistence boundaries.

## Non-authorization

This closeout does **not** authorize implementation of the Rust gameplay server runtime, production `protocol-oteryn` listener/client adapter, Game Session admission/lease, persistence schemas, Platform changes, production deployment or live operations. Those remain behind their own explicit implementation gates.
