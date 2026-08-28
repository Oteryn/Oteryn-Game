# FND-02 — `protocol-oteryn` v1 Foundation Contract

- Status: Accepted architecture contract when merged to `main`
- Date: 2026-08-08
- Gate: `FND-02`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: native Rust client/server gameplay transport and wire foundation
- Canonical machine-readable companions:
  - `docs/contracts/protocol-oteryn/v1/foundation.proto`
  - `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json`
  - `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`
- Reconciliation input:
  - `docs/architecture/FND-02_PLATFORM_PROTOCOL_RECONCILIATION_OWNER_BASELINE.md`
  - `docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json`
- Does not authorize: runtime implementation, production listener/advertisement, Platform writes, persistence schema, FND-03 runtime, FND-04 admission/lease implementation or gameplay-specific payload implementation

## 1. Purpose

`FND-02` freezes the smallest complete native gameplay wire contract required before client/server implementation can proceed without guessing protocol identity, security, framing, command ordering, reconnect continuity, reconciliation or hard input limits.

It deliberately does **not** design every gameplay message. Movement, combat, items, chat, quests, content and later systems register their own typed command/state payloads under their owning gates.

The target remains one product-owned family:

```text
protocol-oteryn
```

It is not a Canary compatibility mode, translation layer, ruleset-specific protocol or client-profile fork. ADR-0008 remains binding: no Canary opcode, parser, fallback, listener sniffing or translation path belongs to production `protocol-oteryn`.

## 2. Decision timing

- **Must decide now?** `YES`.
- **Blocked downstream work:** `FND-03`, `FND-04` wire integration, the native protocol crate/client adapter, wire fixtures and the first vertical slice.
- **What becomes harder later:** changing framing, command identity/order or reconciliation after deployed clients exist creates upgrade and rollback cost.
- **Evidence that may justify supersession:** measured TCP head-of-line problems, parser/runtime security findings, protobuf ecosystem failure, measured wire overhead that materially harms gameplay, or a product requirement impossible to add through the registered extension model.
- **Deliberately undecided:** Rust protobuf/TLS library, socket/runtime implementation, heartbeat cadence, lease/reconnect credential construction, gameplay message schemas, compression, QUIC, persistence representation and runtime queue policy beyond protocol ingress limits.

## 3. Historical Platform reconciliation

The pinned Platform contract at `blakinio/Oteryn-Platform@c0b8703d326a04b43ae8e06f6192b0cb91c859b7` is reconciliation evidence, not Oteryn-v2 protocol authority.

### Preserved requirements

FND-02 preserves:

- Platform Identity as reusable-credential authority;
- Gateway/World Registry as route/offer authority;
- exact route/security validation by the gameplay server;
- no same-credential downgrade or fallback after route/session issuance;
- separation of protocol, transport, optional capability, content and ruleset versions;
- bounded fail-closed parsing;
- server-authoritative state/results;
- retry-safe commands;
- explicit snapshot/delta reconciliation;
- credential/secret redaction.

### Superseded historical assumptions

FND-02 does **not** carry forward as authority:

- Otheryn C++ as target runtime;
- production `protocol-canary` support;
- exact schema revision/hash equality as compatibility identity;
- a fixed capability digest;
- UUIDv4 `CommandId` plus a second client sequence;
- Platform issuance of canonical game-domain `GameSessionId`;
- historical schema revision `2` or its schema hash.

TCP, TLS 1.3, big-endian length framing and protobuf binary are selected below independently for Oteryn-v2, not inherited by inertia.

## 4. Protocol identity

| Dimension | FND-02 v1 |
|---|---|
| family | `oteryn` |
| protocol major | `1` |
| transport profile | `1 = tcp_tls13_alpn_v1` |
| ALPN | `oteryn-game/1` |
| framing | `uint32_be length` + exactly one protobuf `WireEnvelope` |
| serialization | Protocol Buffers binary |
| source IDL syntax | `proto3` |
| foundation schema revision | `1` |
| application compression | `none` |

These are separate from:

- schema source SHA-256;
- client/server build revision;
- content revision;
- ruleset revision;
- world policy revision;
- optional capabilities.

### Same-major schema evolution

`schema_revision` is build/evidence/diagnostic metadata, not an exact-equality handshake gate.

Peers under `protocol_major = 1` may interoperate across schema revisions only when:

- used fields/types remain wire-compatible;
- core v1 semantics are unchanged;
- additive semantics are capability-gated when an older peer cannot safely ignore them;
- cross-version/golden fixtures prove compatibility.

Breaking wire or core semantic changes require a new protocol major.

The SHA-256 of `foundation.proto` is an immutable source-artifact fingerprint. It is not a credential, not a compatibility token and not semantic message identity.

## 5. Transport and TLS

Native v1 uses one separately advertised **TCP** endpoint protected by **TLS 1.3**.

TLS 1.3 follows current RFC 9846. Server service identity verification follows RFC 9525.

Required behavior:

- TLS 1.3 only for transport profile 1;
- ALPN exactly `oteryn-game/1`;
- trusted certificate-chain and reference-identifier verification are mandatory on the client;
- client certificates/mTLS are **not required by FND-02 v1**; application admission/reconnect proof belongs to FND-04;
- no plaintext mode;
- no protocol-family sniffing;
- no Canary fallback on the same listener;
- no application encryption/MAC layered over TLS;
- TLS 0-RTT/early data is forbidden;
- TLS resumption, if enabled later, is only a transport optimization and never restores gameplay authority.

TLS/ALPN/service-identity failure is terminal for the connection. A different route requires a fresh authorized Gateway flow; the same gameplay credential is never silently rebound to a downgraded endpoint.

### Why TCP for v1

The first native gameplay path is predominantly ordered, Tokio networking support already exists in the workspace, and TCP minimizes new operational/protocol complexity for the first vertical slice.

QUIC is deferred, not permanently rejected. It requires measured latency/head-of-line/roaming evidence and must preserve the same authority semantics.

An open TCP socket is not gameplay liveness truth. Authenticated application liveness primitives exist below; FND-03/FND-04 own cadence, timeout/hysteresis and server-stall discrimination.

## 6. Framing

Every post-TLS application frame is:

```text
uint32_be payload_length
payload_length bytes containing exactly one protobuf WireEnvelope
```

Rules:

1. the four-byte prefix is excluded from `payload_length`;
2. zero-length frames are invalid;
3. frame hard maximum is `1,048,576` bytes;
4. length is checked **before** peer-sized allocation/read reservation;
5. truncated, overlong, trailing/multiple-envelope or malformed input fails closed;
6. one frame contains one envelope and one registered typed payload;
7. application compression is forbidden in v1.

Any future compression contract must define compressed size, decompressed size, expansion ratio and CPU/work limits before implementation acceptance.

## 7. Serialization and IDL

FND-02 selects **Protocol Buffers binary**.

Reasons:

- mature binary evolution rules;
- stable numeric field tags;
- cross-language tooling;
- strong golden/property/fuzz testing support;
- additive unknown fields can be ignored when semantics permit;
- less bespoke parser/evolution surface than a custom Oteryn binary format.

The canonical v1 source uses `syntax = "proto3"` for current Rust-toolchain compatibility. FND-02 does not select a concrete Rust protobuf runtime. A later source-IDL migration to Protobuf Editions may stay under protocol major 1 only when generated wire behavior and semantics remain compatible and fixtures prove it.

Schema rules:

- field numbers are never reused;
- deleted numbers/names are reserved;
- existing field meaning is not silently changed;
- unknown fields never activate behavior;
- semantic presence is validated after decode rather than inferred from default zero values;
- zero/`UNSPECIFIED` values are rejected where a concrete value is required;
- peer-controlled strings/bytes/repeated fields obey named limits;
- parsing limits apply before expensive domain work.

Serialized protobuf bytes are **not canonical semantic identity** and must not define command equality, state identity or authorization.

## 8. Envelope and registries

`WireEnvelope` is intentionally small:

```text
message_type
connection_generation
server_sequence
payload
```

`PROTOCOL_OTERYN_V1_REGISTRY.json` maps every foundation `message_type` to canonical name, direction, phase and sequencing class.

Rules:

- `message_type = 0` is invalid;
- merged numeric IDs are never reused;
- unregistered message types never activate behavior;
- the foundation envelope does not contain a high-churn giant gameplay `oneof`;
- later contracts register typed command/state payload schemas carried by `ClientCommand`, `CommandResult`, `StateDelta` and `StateDomainSnapshot`.

## 9. Capability model

Core protocol-v1 semantics are **not capabilities**. Protocol major 1 always implies:

- server-authoritative command results;
- CommandId ordering/idempotency;
- server sequencing;
- state revisions;
- snapshot/delta/resync;
- transport-generation fencing;
- liveness probe/ack support;
- no Canary fallback.

The v1 registry initially contains **zero optional capabilities**. Speculative capabilities are not invented.

Future additive capabilities use stable numeric IDs only when an older same-major peer can safely continue without the feature and the feature cannot weaken a core invariant.

Negotiation rules:

- a peer's **supported** capability list is sorted, unique and bounded;
- an unknown advertised supported capability from a newer peer may be ignored during intersection;
- a capability becomes active only if selected by the authoritative negotiation path;
- an unknown **selected/required** capability fails closed;
- duplicate or unsorted selected/required capability IDs fail closed;
- there is no capability digest and no exact full-list equality requirement.

## 10. Wire identifiers

Canonical FND-ID UUID identities exposed directly by this foundation use exactly **16 bytes** in standard UUID network byte order. Nil/all-zero is invalid.

This currently covers:

- `CharacterId`;
- `WorldId`;
- `ChannelId`;
- `GameSessionId`.

Other foundation IDs enter later payloads only when their owning contract proves client-visible need.

Wire bytes do not replace strong Rust semantic wrappers. Identity never grants authority.

## 11. Bootstrap and admission boundary

After TLS/ALPN, `ClientBootstrap` carries:

- protocol major;
- transport profile ID;
- diagnostic schema revision;
- supported optional capabilities;
- bounded opaque admission material;
- selected `CharacterId`;
- bounded client build ID.

It does **not** carry a client-created canonical `GameSessionId`.

FND-04 owns admission-material construction and validation. Before authoritative admission succeeds:

- no playable map/entity state is exposed;
- envelope `connection_generation = 0` is allowed only for bootstrap/resume traffic;
- gameplay commands are rejected.

After admission, `ServerAccepted` returns the game-issued `GameSessionId`, authoritative `WorldId`/`ChannelId`, non-zero `connection_generation`, current server-sequence boundary and `next_command_id`.

### Resume

`ClientResume` carries the existing `GameSessionId`, opaque reconnect material, last fully applied server sequence and protocol/transport/schema/capability support evidence.

FND-04 decides eligibility. If accepted:

- the same `GameSessionId` survives;
- a strictly newer non-zero `connection_generation` is established;
- the server returns current `server_sequence` and `next_command_id`;
- replay or replacement-snapshot reconciliation follows.

A terminally ended GameSession cannot be resurrected by transport reconnect.

## 12. Connection-generation fencing

`connection_generation` is a `uint64` fence scoped to one `GameSessionId`.

Rules:

- zero is bootstrap/pre-admission only;
- initial admission establishes the first non-zero generation;
- every accepted transport rebind uses a strictly larger generation;
- generation never wraps or reuses; exhaustion is session-terminal;
- every post-admission envelope in **both directions** carries the current generation;
- the server rejects/ignores stale-generation client frames before command/liveness/reconciliation authority;
- the client discards stale-generation server frames and never applies them to the newer binding;
- stale generation cannot submit commands, recover liveness or mutate reconciliation state;
- `STALE_CONNECTION_GENERATION` is transport-fatal for the stale transport but does not kill a newer authoritative binding.

FND-04 owns the admission/reconnect state machine; FND-02 owns the visible fence.

## 13. `CommandId`: identity, order and bounded pipelining

FND-02 rejects the historical `UUIDv4 CommandId + client_sequence` pair.

The accepted identity is:

```text
CommandRef = (GameSessionId, CommandId)

CommandId:
    uint64
    starts at 1
    increases by exactly 1
    scoped to one logical GameSessionId
    survives eligible reconnect of that same GameSessionId
```

`CommandId` is both operation identity and total client-command order. It is not a credential or durable entity ID.

### 13.1 Server-authoritative ingress high-water mark

`next_command_id` means the **next CommandId not yet reserved by authoritative session ingress**.

A command is reserved exactly once when:

1. the envelope/generation is valid;
2. its `command_id == next_command_id`;
3. the bounded outstanding-command window has capacity; and
4. the server atomically places that command identity into the ordered authoritative session-ingress stream.

At that point `next_command_id` advances exactly once, even though domain execution/result may occur later.

This allows bounded pipelining without waiting one network RTT for every command result.

Commands reserved in the ingress stream commit authoritative effects/results in CommandId order. FND-03 may parallelize non-authoritative auxiliary work but may not allow later CommandIds to commit authoritative mutation ahead of earlier reserved CommandIds.

The hard count of reserved-but-not-terminal commands is 64 per GameSession.

### 13.2 Receive behavior

- `command_id == next_command_id` and capacity is available:
  - reserve it once in ordered ingress;
  - advance `next_command_id` once;
  - later produce exactly one terminal authoritative outcome.
- `command_id == next_command_id` but the bounded ingress window is full:
  - reject with `TOO_MANY_OUTSTANDING_COMMANDS`;
  - do **not** reserve or advance the ID;
  - the client may retry that same ID later.
- `command_id < next_command_id`:
  - it is already reserved, pending or terminal;
  - it is **never enqueued/executed again**;
  - if terminal result is retained, replay it as duplicate replay;
  - if the original command is still pending, no second execution or second terminal outcome is created; the original eventual outcome remains authoritative;
  - if neither pending state nor terminal result is retained while the same GameSession is still claimed resumable, return `COMMAND_OUTCOME_EXPIRED` and require reconciliation.
- `command_id > next_command_id`:
  - reject without reservation/domain mutation as `COMMAND_SEQUENCE_GAP`;
  - return the expected ID and require bounded reconciliation.

A structurally valid `ClientCommand` whose registered command type/payload is semantically rejected may still consume its reserved CommandId and produce one terminal `REJECTED` result. Malformed outer frame/envelope traffic is handled before command-stream reservation and follows protocol-fatal rules instead.

### 13.3 Reconnect and crash consequence

Eligible reconnect preserving the same `GameSessionId` also preserves:

- `next_command_id`;
- every still-authoritative pending command identity;
- retained terminal outcomes needed for duplicate/result reconciliation.

A runtime/recovery design may not claim same-GameSession resume if it has lost enough of this state that a lower reserved CommandId could execute again or an already-committed ordering boundary could be contradicted. FND-03/FND-04/DUR contracts must either preserve/reconstruct the necessary session command state or terminate the old logical GameSession safely.

### 13.4 Why no unbounded UUID cache is required

Safety follows from the monotonic ingress high-water mark: every `command_id < next_command_id` has already been reserved once and can never become a new operation again.

Old result payloads may be evicted under a bounded retention policy; that can remove exact-result replay convenience but cannot re-enable execution.

A duplicate's replacement bytes are never reinterpreted as a new operation, so protobuf's non-canonical byte serialization cannot become a command-equality bug.

A new `GameSessionId` gets a fresh command namespace beginning at 1.

## 14. Server-authoritative sequence

The server maintains one `uint64 server_sequence` per logical GameSession.

- first authoritative post-admission sequenced message uses 1;
- every `SERVER_SEQUENCED` message increments exactly once;
- `CommandResult` and `StateDelta` are server-sequenced;
- bootstrap, liveness and snapshot transfer-control frames are unsequenced;
- the client applies only the next expected authoritative sequence;
- old/duplicate sequence is not applied again;
- a gap suspends affected application and triggers bounded resync;
- sequence never wraps/reuses; exhaustion is session-terminal;
- sequence continues across eligible reconnect of the same GameSession.

FND-03 owns queue/publication mechanics; FND-02 owns visible ordering.

## 15. State revisions

Client-visible state synchronization uses typed domain revisions:

```text
StateRevision {
    domain_id
    revision
}
```

Rules:

- `domain_id = 0` is invalid;
- each registered domain owns a monotonic `uint64` revision stream;
- revision never wraps/reuses in scope;
- delta carries `base_revision` and `new_revision`;
- client applies only when current revision equals `base_revision`;
- mismatch causes resync, never speculative application.

Movement/combat/inventory/etc. gates register actual domains and typed payloads later.

## 16. Snapshot and resync

### Initial state

After admission, an initial authoritative snapshot boundary is established before gameplay commands are accepted for that new session. The actual state-domain set belongs to vertical-slice/gameplay contracts.

### Snapshot transfer

A logical `SnapshotBody` is protobuf-encoded once and transferred through:

```text
SnapshotBegin
SnapshotChunk[0..N-1]
SnapshotCommit
```

`SnapshotBegin` declares non-zero session-scoped monotonic `snapshot_id`, exact chunk count, exact encoded byte length and `target_server_sequence`.

Rules:

- all chunks use the same snapshot ID;
- chunk indexes are zero-based and unique;
- declared/cumulative size uses checked arithmetic and hard limits before allocation;
- conflicting duplicates, mixed IDs, gaps, overflow or excess size discard the assembly;
- **any connection-generation change discards every partial snapshot assembly from the old generation**; chunks from different generations are never combined;
- protobuf decode occurs only after a full bounded body is assembled;
- apply is atomic only after all chunks and matching `SnapshotCommit` validate;
- after apply, `last_applied_server_sequence = target_server_sequence`;
- newer sequenced messages apply afterward.

### Snapshot sequencing barrier

A replacement snapshot creates a bounded publication barrier at its declared `target_server_sequence`.

While `SnapshotBegin` through matching `SnapshotCommit` are in flight for one current connection generation:

- the server does not transmit any `SERVER_SEQUENCED` message whose `server_sequence` is greater than `target_server_sequence` on that transport before `SnapshotCommit`;
- authoritative runtime may continue progressing, but later sequenced outputs are retained in the server-owned bounded egress/replay path and become eligible for transmission only after the snapshot commits on the wire;
- the client therefore never needs an unbounded buffer of post-snapshot sequenced messages while assembling a replacement snapshot;
- if the bounded server-side retention/egress policy cannot preserve the required continuation, the transport/session follows the explicit FND-03/FND-04 slow-client/recovery policy rather than violating ordering or dropping authoritative state silently;
- a connection-generation change aborts the barrier together with the partial snapshot; the new generation establishes a fresh replay-or-snapshot reconciliation boundary.

The barrier is a protocol ordering invariant. FND-03 owns the concrete egress queue, backpressure and retention implementation and must keep it bounded.

TLS already supplies transport integrity; v1 adds no application checksum/MAC.

### Resync

`ResyncRequest` carries the client's last applied server sequence and bounded current domain revisions.

The server chooses:

1. contiguous retained authoritative replay; or
2. replacement snapshot.

If replay is unavailable, snapshot is mandatory. The protocol never fabricates missing deltas or instructs a client to skip unknown authoritative state.

Resync is idempotent control traffic and consumes no CommandId.

## 17. Liveness primitives

`LivenessProbe`/`LivenessAck` are authenticated application-protocol control messages.

- probe ID is non-zero `uint64` monotonic within one `connection_generation`;
- probe IDs never wrap/reuse; exhaustion makes that transport generation terminal and requires an authorized rebind rather than reset-to-1 reuse;
- liveness traffic consumes neither CommandId nor server_sequence;
- an idle healthy client still responds to probes;
- gameplay-command silence is not disconnect evidence;
- receipt time at the authoritative server, current generation and server-local health evidence are authoritative;
- client timestamps or self-declared lag/disconnect are not authority;
- stale-generation acknowledgements cannot restore authority.

Probe cadence, grace/hysteresis and mapping to the accepted 2.0-second PvE protection boundary remain FND-03/FND-04 because ordinary jitter and server stalls must be distinguished safely.

## 18. Error model

Stable numeric errors are registered in `PROTOCOL_OTERYN_V1_REGISTRY.json`. Each maps to an existing foundation error category and default disposition:

- `OPERATION_TERMINAL`;
- `RESYNC_REQUIRED`;
- `SESSION_FATAL`;
- `TRANSPORT_FATAL`.

The wire never exposes stack traces, SQL details, tokens, arbitrary exception text or arbitrary server diagnostic strings.

`ProtocolError` is server-to-client control output and may carry only stable safe correlation values such as related/expected CommandId or expected server sequence.

Important distinctions:

- a stale old transport may be closed without killing a newer GameSession binding;
- rejected command does not imply invalid session;
- malformed/oversize frame is transport-fatal;
- sequence/revision gaps request resync instead of guessed continuation;
- FND-04 owns admission/session credential error semantics and maps them through the shared vocabulary.

## 19. Hard resource limits

Exact externally controlled protocol limits are canonical in `RESOURCE_LIMITS_REGISTRY.json`.

Current parent maxima include:

| Resource | Hard maximum |
|---|---:|
| encoded WireEnvelope frame | 1 MiB |
| bootstrap/resume payload | 64 KiB |
| admission material | 16 KiB |
| reconnect material | 16 KiB |
| client build ID | 128 UTF-8 bytes |
| capability IDs | 128 |
| ordinary repeated collection | 4096 |
| protobuf nesting | 32 levels |
| command expected revisions | 64 |
| command payload | 64 KiB |
| command-result payload | 64 KiB |
| outstanding commands / GameSession | 64 |
| state domains / sync unit | 256 |
| state-delta payload | 256 KiB |
| snapshot chunks | 256 |
| snapshot chunk bytes | 512 KiB |
| assembled SnapshotBody | 16 MiB |

Later message types may define smaller limits but may not silently exceed these parent wire bounds.

All receive paths validate size/count/depth before peer-sized allocation or expensive decode/domain work.

## 20. Privacy and observability

Never log at ordinary levels:

- raw admission/reconnect material;
- bearer/ticket/OAuth/session credentials;
- private chat payloads;
- unrestricted serialized frames;
- secrets/authentication headers.

Authorized correlation may include protocol/transport/schema/build revision, GameSessionId under applicable privacy policy, generation, CommandId, server sequence, state revision, stable error code and World/Channel/Node context.

High-cardinality IDs are not ordinary Prometheus labels.

## 21. Foundation failure scenarios

FND-02 defines these wire obligations:

| Scenario | Requirement |
|---|---|
| `FS-DUPLICATE-COMMAND` | lower CommandId is never reserved/executed twice; pending duplicate is not re-enqueued; retained terminal outcome may be replayed |
| `FS-REVISION-MISMATCH` | no guessed mutation; resync |
| `FS-SNAPSHOT-DELTA-MISMATCH` | invalid chain/assembly discarded; replay or replacement snapshot |
| `FS-SLOW-CLIENT` | hard wire/input limits exist; FND-03 owns egress queue/drop/disconnect policy |
| `FS-QUEUE-SATURATION` | 64 reserved-but-not-terminal commands is protocol ingress bound; 65th expected ID is rejected without reservation |
| `FS-STALE-GENERATION` | stale transport can neither command nor restore/apply liveness/state |
| `FS-DUPLICATE-LOGIN` | protocol preserves GameSession/generation fencing; FND-04 decides admission/takeover |
| `FS-GATEWAY-AFTER-REDEEM` | no fallback/reuse assumption; FND-04 owns credential recovery |
| `FS-KEY-ROTATION` | transport/admission credentials are not protocol IDs; FND-04 owns key-roll behavior |
| `FS-CLOCK-SKEW` | ordering/revision correctness uses counters, not client wall clock |

This contract defines assertions; it does not claim those runtime scenarios are implemented.

## 22. Independent wire evidence

Implementation acceptance requires more than “client and server sharing the same codec can talk.”

Required evidence:

1. canonical byte-level golden fixtures for every foundation message and framing boundary;
2. a small independent raw-byte fixture verifier sufficient to validate framing and selected canonical fields without reusing the production codec path;
3. malformed/adversarial corpus for length, truncation, unknown IDs, illegal phases/directions, invalid UUID length, zero sentinels, count/depth overflow and sequence/revision gaps;
4. property tests for semantic encode/decode round trips and bounds;
5. fuzzing of framing and protobuf ingress decoders;
6. cross-version fixtures for permitted same-major additive evolution;
7. tests proving removed protobuf tags are reserved and never reused;
8. tests proving schema-source-hash differences do not themselves force incompatibility;
9. tests proving raw protobuf byte differences do not define command identity/equality;
10. reconnect tests proving old generation traffic in both directions cannot regain/apply authority;
11. pipelining tests proving contiguous IDs reserve once, full-window rejection does not consume the next ID, pending duplicates are not re-enqueued and later IDs cannot commit mutation ahead of earlier reserved IDs;
12. snapshot-barrier tests proving post-target sequenced messages are not emitted before `SnapshotCommit`, old-generation partial snapshots are discarded and bounded server-side retention/backpressure handles continuation without silent loss.

Shared generated schemas/codecs are useful, but not the only oracle.

## 23. Cross-repository rollout

FND-02 acceptance in Oteryn-v2 does not mutate Platform.

After the local contract merges:

- `CROSS_REPOSITORY_CONTRACT_LOCK.json` may record the exact merged Oteryn-v2 FND-02 commit/schema revision/hash as local accepted evidence;
- the old Platform rev2 contract remains `RECONCILIATION_INPUT_ONLY`;
- a separately authorized Platform task must reconcile Gateway/World Registry/session-offer structures with accepted FND-02 dimensions;
- Platform advertisement stays disabled until its own implementation/validation/rollout gates pass;
- native client/server production implementation remains separately gated by FND-03/FND-04 and implementation tasks.

Mutable PR heads are never written to the lock as canonical merged evidence.

## 24. Rejected or deferred alternatives

- **Custom Oteryn binary wire:** rejected for v1; bespoke parser/evolution/security burden has no demonstrated benefit.
- **FlatBuffers / Cap'n Proto:** deferred; reconsider only with measured CPU/allocation/latency evidence.
- **Mandatory Protobuf Editions 2024 source today:** deferred; current Rust support is not uniform enough to make source-language novelty a foundation blocker.
- **Exact schema SHA equality:** rejected; artifact identity is not semantic compatibility.
- **Fixed capability digest:** rejected; core invariants belong to protocol major, optional additions to explicit IDs.
- **UUIDv4 CommandId + independent sequence:** rejected; one GameSession-scoped monotonic uint64 gives deterministic identity/order and bounded duplicate proof.
- **Single outstanding command / stop-and-wait:** rejected; it would unnecessarily couple command throughput and responsiveness to network RTT. The accepted bounded ordered-ingress model permits safe pipelining.
- **QUIC v1:** deferred pending measured benefit.
- **Application compression v1:** rejected until bandwidth evidence justifies decompression/ratio/CPU attack surface.

## 25. Deliberately unresolved after FND-02

- runtime tick/scheduler/threading/queue mechanics — `FND-03`;
- heartbeat cadence, liveness state machine, reconnect credential, lease/takeover rules — `FND-04`;
- persistence/recovery of session command/sequence fences across crashes when required — later runtime/durability contracts;
- movement payloads — `VSL-MOVE-01`;
- combat/death/loot payloads — `VSL-COMBAT-01`;
- map/content snapshot payloads — `VSL-CONTENT-01`;
- item transaction payloads — `GAME-ITEM-01` / `DUR-03`;
- future optional capability IDs until a real additive feature needs one;
- QUIC/compression until evidence requires them.

## 26. Acceptance invariant

A future implementation may claim `protocol-oteryn` v1 compatibility only when it proves that it:

> speaks only the registered Oteryn v1 transport/framing/schema foundation; authenticates TLS server identity correctly with no 0-RTT or Canary downgrade; enforces hard limits before unsafe allocation; treats `(GameSessionId, CommandId)` as the one ordered command identity with bounded ordered ingress and never reserves/executes a lower CommandId twice; fences stale connection generations in both directions; applies server sequence/state revisions without guessing; reconciles through bounded replay or atomic replacement snapshot with a bounded snapshot sequencing barrier; and passes independent byte, malformed, property, fuzz, cross-version, pipelining and snapshot-barrier evidence.

Until those proofs exist, this is architecture authority, **not** an implementation-complete claim.