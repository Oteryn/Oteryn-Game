# Oteryn Game Production Gameplay Server Seam — Preparation Decision Packet

- Date: 2026-08-24
- Issue: Oteryn/Oteryn-Game#96
- Preparation task: `OTV2-20260824-prep-server-seam-96`
- Base: `main@22a3eb866dae19d048969edff1e1fa5012a429b6`
- Status: `BLOCKED_BEFORE_IMPLEMENTATION_ALLOCATION`
- Scope: architecture/preparation only; no runtime, Cargo, registry, deployment, secret or production-port mutation

## Decision summary

The smallest safe server seam is a TCP + TLS 1.3 profile-1 listener that delegates all protocol, admission, GameSession and reconnect authority to Foundation-owned APIs and remains fail-closed for unregistered gameplay commands/state.

The runtime lane is **not yet ready for implementation allocation**. Live code inspection found one Foundation wire gap that can be absorbed by the eventual serialized Server Seam allocation, plus two external readiness blockers:

1. **ALLOCATABLE SHARED GAP:** Foundation validates inbound bootstrap/resume payloads but does not expose typed parsed values or production outbound encoders for `ServerAccepted`, `ServerResumeAccepted` and `ProtocolError`. The conditional Server Seam allocation below explicitly leases the canonical Foundation protocol path to add only this already-registered bridge.
2. **BLOCKER:** Foundation exposes `ReconnectAttemptJournal` and `AdmissionAuthority`, but `main` has no non-test durable journal and no production consumer that verifies FND-04 admission/recovery material into trusted facts.
3. **BLOCKER:** the Resource Limits Registry has FND-02 message limits but no accepted NET/FND-03 ceilings for the new peer-controlled connection, TLS-handshake, outbound and pending-work boundaries exercised by a production listener.

Allocating a socket listener before blockers 2–3 close would either bypass FND-04 authority, invent safety limits, or ship a listener that cannot complete the required authoritative `admission -> GameSession` journey.

## Verified current state

- **PROVEN:** `apps/game-server/src/main.rs` still runs only `--smoke`; ordinary execution fails closed.
- **PROVEN:** `GameServerBootstrap` exposes only `GameplayAvailability::UnavailableBootstrap`.
- **PROVEN:** FND-02 profile 1 is TCP + TLS 1.3, ALPN `oteryn-game/1`, BE32 framing and one protobuf `WireEnvelope`; plaintext, 0-RTT and Canary fallback are forbidden.
- **PROVEN:** `FrameLength` enforces the 1,048,576-byte frame maximum before body use.
- **PROVEN:** `decode_framed_envelope` / `decode_wire_envelope` validate current inbound Foundation message shapes and resource limits.
- **PROVEN:** `ConnectionFence`, `ConnectionGeneration`, `AdmissionAuthority` and reconnect PREPARE/COMMIT semantics are merged.
- **PROVEN:** no production gameplay listener, TLS accept loop or client-entry transport adapter exists on `main`.
- **PROVEN:** QA Tier 1/2 gameplay journeys remain `NOT_EVALUATED` because the supported physical gameplay boundary is absent.

## Accepted invariants consumed unchanged

The server seam must not create a second owner for:

- protocol family `oteryn`, protocol major `1`, transport profile `1`;
- TLS 1.3 and exact ALPN `oteryn-game/1`;
- BE32 frame length and all FND-02 hard limits;
- accepted protobuf field/message IDs;
- `GameSessionId`, `CommandId`, server sequence, state revisions and snapshot barrier;
- `ConnectionGeneration` fencing across reconnect/rebind;
- `CharacterLease` and `ScopeOwnershipGeneration` authority;
- FND-04 fresh-admission and recovery security profiles;
- the single FND-03 authoritative runtime mutation owner;
- NET-TRANSPORT-01 transport neutrality; QUIC remains unavailable/unregistered.

## Required physical journey

```text
TCP connect -> TLS 1.3 + ALPN
-> bounded BE32 frame read -> Foundation decode/validate
-> ClientBootstrap or ClientResume authority verification
-> AdmissionAuthority commit/reconcile -> GameSession
-> current ConnectionGeneration response
-> liveness/resync -> explicit fail-closed gameplay entry
```

Transport success never implies admission, GameSession, CharacterLease, liveness or gameplay authority.

## Missing bridge A — wire extraction and outbound encoding

`protocol.rs` currently validates bootstrap/resume payload fields internally, then returns only raw payload bytes through `WireEnvelopeView`. The values needed by an admission consumer are not exposed as a typed parsed view.

Workspace search also found no production encoder for the server-side Foundation messages required to acknowledge admission/reconnect or emit a protocol error.

**Decision:** the later implementation allocation must explicitly lease `apps/game-server/src/foundation/protocol.rs` for a minimal Foundation-owned wire bridge. That bridge may expose typed parsed bootstrap/resume views and encoders for already-registered Foundation response messages. It may not add IDs, change field meanings, change limits or introduce a second schema.

Required Foundation bridge evidence:

- canonical/golden bytes against `foundation.proto`;
- malformed, duplicate, unknown and over-limit field tests;
- round-trip/cross-oracle evidence for new server encoders;
- exact direction/phase/generation validation;
- no allocation from unchecked peer lengths.

## Missing bridge B — trusted admission/reconnect authority consumer

`AdmissionAuthority<T, J>` requires a `ReconnectAttemptJournal<T>`. On current `main`, every concrete journal implementation is test-only. No production adapter consumes FND-04 JWS admission/recovery material and current authoritative evidence into trusted facts.

**Decision / tracked dependency:** Issue #115 owns the verifier preparation; #94 has the durable-journal cross-domain finding. #96 must not substitute an in-memory production journal, accept prevalidated facts from the socket peer, or treat signed material alone as sufficient. A concrete trusted authority consumer/journal must be allocated by its owning durability/security boundary, or #96 must consume an already merged implementation.

This is a hard readiness dependency because the required physical journey includes an authoritative `GameSession`, not merely a TLS socket and syntactically valid protobuf.

## Blocker C — transport/runtime resource ceilings

The accepted registry has FND-02 message/frame limits, but no FND-03/NET hard maxima for a production TCP/TLS accept path. FND-03 explicitly requires concrete maxima before runtime implementation acceptance for every externally influenced queue, pending set and amplification-prone boundary.

The first listener would exercise new peer-controlled work at least for:

- concurrent pre-admission TCP/TLS connections;
- concurrent TLS handshakes / authentication work;
- per-connection inbound retained bytes while assembling a bounded frame;
- per-session outbound queued entries and bytes;
- pending transport writes / slow-client backpressure;
- connection/task shutdown and drain work.

**Decision / tracked dependency:** Issue #116 owns this decision. These dimensions require an accepted NET/FND-03 resource-limit decision and registry entries (or explicit proof that an existing registered limit is the same resource) before implementation acceptance. #96 does not select numeric values and must not silently reuse unrelated FND-02 frame ceilings.

## Exact conditional implementation topology

After blockers B and C are closed, allocate one `OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM` worker with these exact runtime/test/shared paths:

- create `apps/game-server/src/gameplay_transport/mod.rs` — transport lifecycle/composition API only;
- create `apps/game-server/src/gameplay_transport/tcp_tls.rs` — TCP profile-1 accept/TLS/ALPN/framing I/O;
- create `apps/game-server/src/gameplay_transport/connection.rs` — one connection state machine binding Foundation decode/admission/session/reconnect/resync;
- modify `apps/game-server/src/foundation/protocol.rs` — Foundation-owned typed bootstrap/resume extraction plus outbound encoding of already-registered Foundation messages only;
- modify `apps/game-server/src/lib.rs` — compose the transport module without changing gameplay command/state authority;
- modify `apps/game-server/src/main.rs` — expose only configuration-driven startup; no hard-coded production address/port/certificate/secret;
- create `apps/game-server/tests/gameplay_server_seam.rs` — real socket/TLS production-path integration and negative tests;
- modify `apps/game-server/Cargo.toml`, root `Cargo.toml` and `Cargo.lock` only for the minimum direct TLS dependencies required by the accepted profile;
- create `docs/superpowers/plans/2026-08-24-oteryn-production-gameplay-server-seam.md` before runtime writes;
- create/update the implementation task record allocated by the coordinator.

## Shared-path and one-writer lease

The implementation lane requires a serialized one-writer lease for:

- `apps/game-server/src/foundation/protocol.rs`;
- `apps/game-server/src/lib.rs`;
- `apps/game-server/src/main.rs`;
- `apps/game-server/Cargo.toml`;
- root `Cargo.toml`;
- `Cargo.lock`.

No sibling Durability/Ability/Interaction/AI worker may mutate those exact paths while the Server Seam lease is active. `workspace-boundaries.toml` is not required by the proposed in-app module topology and stays excluded unless deterministic architecture-check evidence proves otherwise.

## Authority-before-mutation ordering

For every connection, processing order is fixed as:

1. accept TCP and complete exact profile-1 TLS 1.3 + ALPN;
2. read/check the four-byte frame length before peer-sized allocation;
3. Foundation decodes and validates exactly one `WireEnvelope` for direction/phase/limits;
4. bootstrap/resume material is verified by the FND-04 trusted consumer before construction of trusted admission facts;
5. durable admission/reconnect authority commits or reconciles through `AdmissionAuthority` + the owning journal;
6. only the resulting current `GameSession` / `ConnectionGeneration` may bind the transport;
7. stale transports/generations fail closed before command reservation or any authoritative mutation;
8. resync uses Foundation sequence/revision/snapshot semantics;
9. gameplay entry remains unavailable until a registered owning gameplay command/state slice exists.

A syntactically valid envelope, valid TLS connection, valid signature or successful socket write is never sufficient gameplay authority.

## Unsupported-capability behavior

The seam advertises/accepts only already registered Foundation protocol semantics. While no supported gameplay command/state slice is registered, gameplay-command dispatch stays disabled and any received `ClientCommand` must fail closed before command-specific/domain mutation. The implementation must use an already registered applicable Foundation error/result semantic if one exists; if none is applicable, it must close/reject the unsupported path rather than allocate a new stable gameplay ID inside this lane. It does not invent a capability, call domain mutation APIs or reinterpret transport success as gameplay readiness.

## Tier-1 QA boundary

After the production seam itself is merged, a separately allocated QA lane may attempt the ADR-0007 Tier-1 Foundation journey through the actual supported process/transport boundary:

```text
Platform/Gateway authorized route
-> TCP/TLS/ALPN production listener
-> bootstrap admission -> GameSession
-> unexpected transport loss
-> resume/reconnect with newer ConnectionGeneration
-> stale old-generation rejection
-> resync / snapshot boundary
-> explicit gameplay-unavailable result
```

The Server Seam delivery makes this physical journey executable; it does **not** mark Tier 1 `PROVEN`. QA owns the evidence envelope, disposable Platform/test authority, cleanup and accepted physical-attempt result.

## Mandatory negative/TDD evidence

Before positive authority tests, the implementation child plan must require RED then GREEN coverage for at least:

- zero-length, truncated and >1,048,576-byte frames rejected before body allocation;
- malformed protobuf, duplicate singular fields, invalid direction/phase and unknown message type;
- wrong protocol major, wrong transport profile and invalid ALPN;
- oversized bootstrap/reconnect material and invalid IDs/capability sets;
- invalid/expired/replayed/wrong-binding FND-04 material rejected before `GameSession` creation;
- concurrent/replayed fresh admission cannot create two sessions;
- stale `ConnectionGeneration` cannot reserve a command or mutate state after rebind;
- reconnect PREPARE/COMMIT ambiguity reconciles idempotently and stale transport is fenced;
- unsupported gameplay command/state fails closed after admission with zero domain mutation;
- slow client / outbound saturation and connection/handshake saturation stay inside accepted hard limits;
- shutdown/drain does not silently drop already-authoritative reserved work.

Golden/cross-oracle wire evidence is mandatory for any new Foundation outbound encoder. Tests must traverse the production listener/composition path; a direct-domain or test-only listener does not satisfy the physical seam claim.

## Risk and review classification

The eventual runtime change crosses protocol, TLS, admission, GameSession and reconnect fencing boundaries. It therefore requires genuinely independent exact-head semantic/security review in addition to whole-diff self-review, protected CI, dependency/supply-chain checks and `game-gate`.

This preparation packet itself changes no runtime/public contract/registry/security policy and requires deterministic documentation/governance validation plus exact-diff self-review. Runtime/E2E for this preparation delivery is `NOT_APPLICABLE`.

## Conditional implementation allocation proposal

```yaml
task_id: OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM
repository: Oteryn/Oteryn-Game
status: BLOCKED_PENDING_PREREQUISITES
branch: agent/otv2-gameplay-server-seam-01
child_plan: docs/superpowers/plans/2026-08-24-oteryn-production-gameplay-server-seam.md
owned_primary_paths:
  - apps/game-server/src/gameplay_transport/**
  - apps/game-server/tests/gameplay_server_seam.rs
shared_one_writer_paths:
  - apps/game-server/src/foundation/protocol.rs
  - apps/game-server/src/lib.rs
  - apps/game-server/src/main.rs
  - apps/game-server/Cargo.toml
  - Cargo.toml
  - Cargo.lock
forbidden:
  - gameplay command/state/event ID allocation
  - Movement/Combat/Ability/Interaction/AI implementation
  - production port/address/certificate/secret selection
  - QUIC registration/activation
  - Platform or external-repository mutation
```

### Prerequisites that must be merged before allocation release

1. **Issue #115:** a production FND-04 fresh-admission/recovery material verifier/consumer exists and yields trusted facts without peer-controlled authority.
2. **Issue #94 cross-domain dependency:** the Durability topology supplies or explicitly allocates a production durable `ReconnectAttemptJournal` with required fresh-admission/reconnect idempotency and recovery semantics; an in-memory test journal is insufficient.
3. **Issue #116:** NET/FND-03 resource ceilings for the TCP/TLS listener's newly exercised connection/handshake/outbound/pending work are accepted and registered with boundary tests.
4. The coordinator confirms the listed shared paths are free and grants their serialized one-writer lease.

The Foundation wire extraction/outbound encoder gap is **not** a separate external prerequisite if the coordinator grants `apps/game-server/src/foundation/protocol.rs` in the Server Seam allocation exactly as listed above.

## Decision timing

- **Must decide now? YES.** Client allocation and real Tier-1 server E2E are blocked on a concrete production seam boundary.
- **Blocked downstream work:** `OTV2-IMPL-CLIENT`, real ADR-0007 Tier 1 Foundation journeys, then Movement integration readiness.
- **Harder later if chosen incorrectly:** a listener-local parser/session owner would duplicate protocol authority and make reconnect/security correction invasive.
- **Evidence that could supersede this topology:** a merged Foundation/Durability provider seam that changes the concrete consumer boundary without changing FND-02/FND-04 semantics, or measured evidence requiring a different in-app decomposition.
- **Deliberately not decided:** numeric transport limits, production addresses/ports, certificate/key storage, deployment/orchestrator configuration, QUIC, gameplay command/state schemas and gameplay availability.

## Handoff verdict

`BLOCKED_BEFORE_SERVER_SEAM_ALLOCATION`

Preparation is complete enough to identify the exact conditional implementation topology, tests, shared leases and blockers. Do **not** invoke `Oteryn: impl server seam` until prerequisites 1–4 above are merged/accepted and the coordinator converts the conditional proposal into an exact implementation allocation bound to current `main`.
