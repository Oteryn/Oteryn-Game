# Oteryn v2 Foundation Programme — Current Status

- Status: Canonical current execution-status overlay
- Date: 2026-08-08
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: current foundation gate progression and next-action interpretation
- Does not supersede: accepted ADR semantics, dedicated accepted contract semantics, product decisions or historical evidence except stale progress wording explicitly replaced here

## Purpose

This file is the canonical **current progression overlay** when older backlogs, registers, baselines or archived tasks contain progress sentences written before later merges.

Dedicated accepted ADRs/contracts remain semantic authority. `FOUNDATION_DECISION_BACKLOG.md` remains the stable gate-definition/backlog source. Historical records are not rewritten merely to look current.

Chat history is not execution authority.

## Current accepted foundation evidence

### `FND-01` — workspace/dependency/migration

- status: `ACCEPTED AND APPLIED`;
- canonical Rust workspace is `blakinio/Oteryn-v2`;
- client/workspace cutover PR #50 squash merge: `78988f72a80cc904aa9176ae850c50d4efa0b0f0`;
- archived `blakinio/otclient` is non-canonical historical source.

### `VSL-02` — client migration/cutover

- status: `COMPLETE`;
- canonical native Rust client path: `apps/client`;
- ADR-0011 `pre-native-protocol` fail-closed state remains binding until separately authorized native-protocol implementation.

### `FND-ID-01` — foundation identifiers

- status: `ACCEPTED AND MERGED`;
- canonical contract: `docs/architecture/FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md`;
- delivery PR #85 squash merge: `2c584543cd1e3758958755478a6cc6ed3d39a8a9`;
- lifecycle/status closeout PR #87 squash merge: `648aa10bb5b36d8826d82ed0f1ed94a47ca53a24`.

Canonical foundation entity identities remain:

```text
AccountId
CharacterId
WorldId
ChannelId
NodeId
InstanceId
PartyId
GameSessionId
conditional HandoffId
```

`AdmissionId` and `CharacterLeaseId` remain intentionally absent. FND-04 uses operation references/nonces and scoped generations without promoting them to new foundation entity IDs.

### `FND-02` — `protocol-oteryn` v1 foundation

- status: `ACCEPTED AND MERGED`;
- canonical contract: `docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md`;
- canonical schema: `docs/contracts/protocol-oteryn/v1/foundation.proto`;
- protocol registry: `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json`;
- resource limits: `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`;
- delivery PR #94 exact head: `91809204fcdf984a5a9d7b8c276ef9fb2f9cab9f`;
- squash merge: `769ecd2ce2dfe0a7644d8dc1d67c54d40da5d202`;
- runtime/component/E2E implementation remains `NOT IMPLEMENTED / NOT AUTHORIZED BY FND-02`.

Binding wire foundation includes TCP + TLS 1.3 / ALPN `oteryn-game/1`, bounded protobuf framing, game-issued GameSessionId after final admission, `connection_generation`, `(GameSessionId, CommandId)`, server sequencing, snapshot/delta/resync and authenticated liveness primitives.

## Accepted disconnect / re-entry direction

Owner clarification is accepted, merged and closed out.

Canonical sources include:

- `DISCONNECT_REENTRY_PVE_PROTECTION_OWNER_DECISION.md`;
- `DISCONNECT_CLIENT_OS_FORENSICS_OWNER_DIRECTION.md`;
- `DISCONNECT_CLIENT_OS_FORENSICS_PRIVACY_TIMING_REFINEMENT.md`;
- server-side forensic/privacy baselines named by those documents.

Delivery:

- PR #96 exact final head `ddf62ec48443fb4ce119eed0900662e222a99726`;
- PR #96 squash merge `496f0b9ad5231d24325e937a3f09ba221cf5c86b`;
- closeout #97 squash merge `b85bdd3f278d9de12284eab7c6352219325b3751`.

Binding player-facing semantics include:

- eligible unexpected-control-loss re-entry receives exactly 4 seconds of defensive PvE protection;
- no new incoming monster attack during the full protection interval, including from already-targeting monsters;
- no outgoing protected-player PvE offense and no buffering of prohibited offense;
- already-committed effects may resolve;
- movement/self-healing/potions remain legal under normal rules;
- protected character cannot heal another player but may receive legal healing;
- reconnect does not reset actor state;
- graceful logout/intentional takeover does not manufacture protection.

## `FND-03` — authoritative runtime execution

### Analysis

- status: `COMPLETE, MERGED AND ARCHIVED`;
- canonical inputs:
  - `FND-03_AUTHORITATIVE_RUNTIME_EXECUTION_ANALYSIS_BASELINE.md`;
  - `FND-03_RUNTIME_LIFECYCLE_FAILURE_AND_REPLAY_ANALYSIS_BASELINE.md`;
- analysis PR #98 squash merge: `86881713ac99877ae765f73bf2750867d450516b`;
- analysis closeout #100 squash merge: `a931c54e7e32b2cea370317ce88896a18eed8ccb`.

### Final contract

- status: `ACCEPTED, MERGED AND CLOSED OUT`;
- canonical contract: `docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md`;
- final delivery PR #102 exact head: `b1ccc90de58052f1bb61b705250b59507792f909`;
- final delivery squash merge: `e72f2514924e8bbf8d1a729721cce9e67d977544`;
- lifecycle closeout PR #103 squash merge: `3c32fb08ddf52939159c0ace5fe607ca4fb18332`.

FND-03 freezes one logical authoritative owner per ChannelRuntime/InstanceRuntime, ownership-generation fencing separate from NodeId, `RuntimeExecutionOrdinal`, monotonic timer/runtime semantics, bounded queues/backpressure, async revalidation, lifecycle/drain/checkpoint/recovery, deterministic gameplay RNG/replay and the runtime side of accepted disconnect/re-entry timing.

FND-03 merge did **not** authorize runtime implementation.

## `FND-04` analysis evidence

The FND-04 analysis phase is complete, repaired against current Oteryn Platform contracts and archived.

Canonical analysis inputs:

- `docs/architecture/FND-04_SESSION_ADMISSION_LEASE_ANALYSIS_BASELINE.md`;
- `docs/architecture/FND-04_PLATFORM_PRE_ADMISSION_RECONCILIATION_REFINEMENT.md`.

Lifecycle/evidence:

- initial analysis PR #104 exact head: `e14a386c8cc998f69075f99890e6fe68a930b396`;
- initial analysis squash merge: `c638ad524772f227dabc90e88a1381cc01e907ce`;
- delayed review identified a current-Platform reconciliation P1;
- premature closeout #105 was closed unmerged before releasing ownership;
- repair PR #107 exact head: `7ebb0818b771692de36c3b5323f68e7bb8d011fe`;
- repair squash merge: `bcf975f215e9aa86a544e158b9e3d42ece1bc642`;
- replacement lifecycle closeout #108 squash merge: `27f7f647f04e3b1a4151f9b124401986910f03d8`;
- duplicate reconciliation PR #106 was closed unmerged as superseded by canonical repair #107.

Current external reconciliation evidence is pinned read-only at:

```text
blakinio/Oteryn-Platform@216f5b2817e9d102337608609e344518512c2a0d
```

including current Platform native pre-admission handoff and runtime-status projection contracts.

Accepted analysis direction distinguishes:

```text
AccountPresenceClaim
CharacterLease
GameSession
TransportBinding
RuntimeScopeAuthority
```

and requires explicit handling of post-issuance Platform security changes, runtime observation/ownership-generation applicability, Platform AdmissionAttemptRef idempotency distinct from game GrantNonce consumption, reconnect lost-response ambiguity and current game-domain recovery placement.

Analysis alone did not complete FND-04 or authorize implementation.

## Current `FND-04` final contract delivery

Current bounded branch/task:

```text
docs/OTV2-20260808-fnd04-session-admission-final
OTV2-20260808-fnd04-session-admission-final
```

Candidate semantic contract:

- `docs/architecture/FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md`.

Required candidate normative refinement:

- `docs/architecture/FND-04_HEALTHY_BINDING_REBIND_SECURITY_REFINEMENT.md` — authoritative for healthy-binding non-preemption, PREPARE→COMMIT revalidation, the complete Decision Timing matrix, FND-04 failure progression and the PREPARE→COMMIT eligibility-change scenario disposition/evidence. The main FND-04 contract links this refinement reciprocally and is harmonized with it.

Candidate interoperable security profiles:

- `docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md`;
- `docs/contracts/FND-04_REAUTHENTICATED_RECOVERY_GRANT_PROFILE_V1.md`.

Candidate final delivery freezes, subject to exact-head review/merge:

- account-global presence exclusion separate from GameSession terminality;
- CharacterLease generation/fencing separate from runtime-scope ownership;
- fresh admission atomic linearization;
- Platform signed fresh-entry and recovery capability profiles using strict Ed25519/JOSE validation with mutually exclusive purposes;
- Platform security-generation/revocation freshness before new admission/recovery;
- authenticated signing-key/profile trust/revocation evidence accepted age `<=5 seconds` for both grant profiles at every authority-changing validation boundary; stale/unavailable/unprovable evidence uses the purpose-specific `*_GRANT_SECURITY_EVIDENCE_STALE`, while fresh explicit unknown/revoked/not-trusted key/profile uses `*_GRANT_AUTHENTICATION_FAILED`, with no nonce consumption or gameplay authority;
- fresh-entry route/runtime ownership-generation applicability;
- Platform `AdmissionAttemptRef` producer idempotency distinct from game GrantNonce one-time consume, including `ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED` for ambiguous fresh-entry issuance;
- recovery producer attempt idempotency with `RECOVERY_ATTEMPT_RECONCILIATION_REQUIRED` for ambiguous recovery-grant issuance: same recovery `attempt_ref` reconciliation only, no blind second grant/independent recovery attempt, deterministic retirement plus proof any possibly issued recovery grant is no longer acceptable before a new attempt, and no recovery-to-fresh-entry reinterpretation;
- game-domain 32-byte reconnect secret;
- two-phase reconnect PREPARE/COMMIT so lost responses/crashes cannot create ambiguous authority;
- atomic COMMIT-time current-authority/security/key-profile-trust-freshness/compatibility revalidation so PREPARE never escrows stale replacement authority;
- healthy current-binding non-preemption by reconnect/recovery bearer proof alone;
- recovery-specific malformed/authentication/not-yet-valid/expiry/replay/security/revision/no-target/healthy-controller/issuance-ambiguity failure progression distinct from fresh-entry Gateway actions;
- exact 15-second same-GameSession grace beginning at the accepted 2-second control-loss declaration boundary;
- actor-scoped ControlLossEpoch so routine rebind/session replacement cannot manufacture repeated 4-second protection;
- Platform-reauthenticated same-session recovery when the game proves safe current state;
- post-grace fresh GameSession attachment to the same `PRESENT_UNCONTROLLED` actor without reset/recreation;
- healthy combat/PZ/logout-locked incumbent protection;
- Channel/Instance session-continuity classes;
- explicit admission/recovery issuance ambiguity, admission/reconnect replay and PREPARE→COMMIT eligibility-change failure scenarios with stable internal errors and Foundation Error Vocabulary progression;
- measured preimplementation gates for liveness cadence, lease timing and runtime resource limits rather than guessed production defaults.

Delivery-state interpretation is transition-safe:

- while the final FND-04 PR remains open, its exact head is candidate architecture only;
- after squash merge, the FND-04 main contract, required rebind refinement and both grant profiles on `main` become semantic authority;
- a separate lifecycle closeout must archive/release final FND-04 task ownership;
- **no runtime, Platform, persistence, protocol-codec, key or production implementation is authorized merely by FND-04 architecture acceptance**.

## Current ordered programme state

| Gate / programme step | Current status | Consequence |
|---|---|---|
| `FND-01` | `ACCEPTED AND APPLIED` | Workspace/dependency/migration foundation complete. |
| `VSL-02` | `COMPLETE` | Canonical native client/workspace established. |
| `FND-ID-01` | `ACCEPTED AND MERGED` | Foundation identity semantics frozen. |
| `FND-02` | `ACCEPTED AND MERGED` | Native gameplay wire foundation frozen. |
| Disconnect/re-entry clarification | `ACCEPTED AND MERGED` | Player-facing recovery baseline frozen. |
| `FND-03` | `ACCEPTED, MERGED, CLOSED OUT` | Runtime execution semantics frozen; implementation still separate. |
| `FND-04` analysis | `COMPLETE, REPAIRED, CLOSED OUT` | Canonical analysis + current Platform reconciliation available. |
| `FND-04` final contract | `CURRENT DELIVERY / CANDIDATE UNTIL MERGED` | Final admission/GameSession/reconnect/lease semantics are being validated. |
| `DUR-01`…`DUR-04`, `PERF-01`, `OPS-CHANNEL-01`, `ANL-01`… | `LATER CONTRACT GATES` | Remain required under backlog/dependency ordering. |

## Downstream ordering after FND-04

After a successful FND-04 delivery **and lifecycle closeout**, select the next architecture gate from the canonical dependency backlog rather than beginning runtime implementation implicitly.

Important dependency facts already frozen in the backlog:

- `DUR-01` defines physical durable identifier representation without changing FND-ID semantics;
- `PERF-01` provides measured capacity/performance evidence;
- `OPS-CHANNEL-01` owns production placement/orchestration/fencing mechanics;
- `ANL-01` owns event/audit/outbox/privacy foundation and must be accepted before `DUR-02`/`DUR-03` finalize required audit/outbox transaction behavior;
- `DUR-02`/`DUR-03` remain required before durable character/item/currency mutation can be claimed.

The exact next package is chosen after final FND-04 closeout using live dependency/ownership state; no implementation package is implied automatically.

## External Platform boundary

Historical Platform protocol material remains reconciliation evidence only where superseded by Oteryn-v2 semantic authority.

The current Platform native pre-admission/runtime-status contracts at the pinned revision above are explicit cross-repository inputs to FND-04.

A future Platform producer rollout requires a **separately authorized** `blakinio/Oteryn-Platform` branch/task/PR implementing the accepted Oteryn-v2-compatible profiles. No Platform write is authorized by this status overlay or by the current FND-04 documentation delivery.

## Historical progress wording

Older ADRs, archived tasks, global registers and backlog prose may still say `FND-02 next`, `FND-03 next` or describe PR #102 as live. Those are historical progress statements and are not current execution authority.

Use this file plus live GitHub state for current progression and dedicated accepted contracts for semantics.

## Current next action

Complete the single bounded final architecture-only FND-04 delivery on its current branch:

1. finish exact contract/profile/refinement/failure-scenario/status reconciliation;
2. keep one PR with only declared owned paths;
3. require exact-head Agent governance, Dependency review and CodeQL;
4. require independent architecture/security audit with zero material findings;
5. resolve all review threads;
6. squash merge only if exact head remains unchanged and all gates pass;
7. archive/release ownership in a separate closeout PR;
8. only then select the next architecture gate from the dependency backlog.

Do **not** begin GameSession/runtime/Platform/persistence implementation from this architecture-only programme unless the owner separately authorizes implementation and the required FND-03/FND-04 numeric/evidence gates are satisfied.

## Non-authorization

This status update does **not** authorize implementation of the Rust gameplay server runtime, production `protocol-oteryn` listener/client adapter, GameSession/admission/reconnect/lease code, persistence schemas, Platform changes, key infrastructure, client diagnostics, Launcher/Guardian, Game Intelligence enforcement, production deployment or live operations.
