# ADR-0014: Dual gameplay transport — TCP default, QUIC opt-in target and safe fallback

- Status: Accepted strategy; QUIC admission activation blocked pending protocol/admission profile reconciliation
- Date: 2026-08-10
- Decision ID: `NET-TRANSPORT-01`
- FND-02 relationship: this ADR **does not supersede the current v1 TCP transport profile or FND-02's measured-benefit prerequisite**. It refines only the deferred product direction in FND-02 Section 5 (`Why TCP for v1`) and Section 24 (`QUIC v1: deferred pending measured benefit`) from “no decision yet whether QUIC should become a supported transport” to “QUIC is the intended future optional transport target”. TCP profile `1`, all current FND-02 wire/ordering/security rules, and the requirement for measured benefit before QUIC activation/default promotion remain binding.
- Does not authorize: QUIC admission, runtime adapter implementation, production listeners, Platform changes, final QUIC library choice, 0-RTT, QUIC DATAGRAM, or production rollout.

## Problem

`FND-02` correctly selected TCP protected by TLS 1.3 as the conservative first transport and explicitly deferred QUIC pending measured benefit. Oteryn now needs to decide whether its architecture should remain TCP-specific, remain entirely undecided, or preserve an explicit route to QUIC without pretending QUIC is currently admission-compatible.

The decision must not create a second gameplay protocol, weaken authentication/fallback, bypass the Platform Game Gateway, violate FND-02 visible ordering/snapshot barriers, or claim QUIC admission compatibility before the canonical transport/admission profiles support it.

## Constraints

The decision must preserve:

- one application protocol: `protocol-oteryn`;
- current FND-02 transport profile `1` as TCP + TLS 1.3;
- FND-02 server-sequence, state-revision and snapshot-barrier semantics;
- FND-04 purpose-separated fresh admission/recovery credentials and GameSession fencing;
- Platform Game Gateway ownership of Game Login Ticket redemption;
- no cross-profile credential reinterpretation or security downgrade;
- bounded resource usage and fail-closed behavior for untrusted network input;
- an evidence gate before QUIC becomes player-active or default.

## Options considered

### Option A — Keep TCP as the only planned transport indefinitely

Benefits:

- smallest implementation and operational surface;
- one transport stack to secure, observe and support;
- no UDP-path compatibility or QUIC stream-ordering concerns.

Costs:

- forecloses deliberate experimentation with QUIC connection migration and independent stream behavior;
- increases the chance that later gameplay/session code accidentally couples itself to TCP-specific assumptions;
- makes later introduction of QUIC more invasive if product measurements show a clear benefit.

Disposition: **rejected as the long-term architecture target**, while TCP remains the current authoritative transport.

### Option B — Leave the transport direction completely undecided

Benefits:

- avoids selecting a target before benchmarks exist;
- no immediate contract work beyond the current TCP baseline.

Costs:

- gives client/session/Gateway design no explicit requirement to remain transport-neutral;
- leaves player-settings and future endpoint-offer design ambiguous;
- postpones discovery of credential-binding and ordering constraints until implementation pressure is higher.

Disposition: **rejected** because transport neutrality and safe future extension are worth deciding now even though QUIC activation is not.

### Option C — TCP current/default; QUIC accepted as a blocked future optional target

Benefits:

- preserves the proven/conservative TCP v1 path;
- forces transport-neutral gameplay/session boundaries now;
- permits a bounded QUIC bake-off later without making QUIC production authority today;
- retains TCP as compatibility/operational fallback if QUIC is eventually activated;
- exposes the exact protocol/admission reconciliation work that must happen before player use.

Costs:

- creates a future dual-transport testing and operational burden;
- requires careful cross-transport credential issuance/recovery semantics;
- QUIC stream ordering, UDP path behavior, resource limits and library security must be proven before activation;
- no immediate player benefit is delivered by this architecture-only decision.

Disposition: **selected**.

### Option D — Register and ship QUIC now, or make QUIC the immediate default

Benefits:

- fastest route to real QUIC measurements;
- potential latency/roaming advantages become testable immediately.

Costs:

- conflicts with the current FND-02 registry and FND-04 grant profiles, which support transport profile `1` only;
- would require larger protocol/admission/security changes in this delivery;
- introduces unproven UDP/QUIC resource and operational attack surface;
- risks conflating architecture preference with production readiness.

Disposition: **rejected for the current stage**.

## Trade-offs and risks

The selected option deliberately buys future flexibility at the cost of maintaining two transport adapters once QUIC is activated. Material risks are:

- **ordering:** independent QUIC streams can reorder messages across streams and violate FND-02 `server_sequence` or snapshot publication barriers unless the lane contract is constrained;
- **security:** fallback can become a downgrade path if authenticated rejection is reinterpreted as a transport failure;
- **credential safety:** transport-bound pre-admission/recovery material cannot be replayed or silently rebound across transport profiles;
- **availability:** some networks block or impair UDP/QUIC, so TCP must remain usable;
- **resource exhaustion:** QUIC handshake, crypto, stream and reassembly state add externally triggerable memory/CPU pressure;
- **operations:** two transports increase telemetry, troubleshooting, DDoS and release/test matrices;
- **dependency risk:** the selected Rust QUIC implementation will require continuous security/advisory review;
- **player UX:** a “prefer QUIC” option must not imply that QUIC is always available or currently active.

These risks are why QUIC activation remains blocked rather than being inferred from this strategy decision.

## Recommendation

Choose **Option C**: preserve TCP profile `1` as the only current authoritative/default gameplay transport, make transport neutrality a binding architecture requirement, and accept QUIC v1 + TLS 1.3 as the intended future player-opt-in target only after profile reconciliation and evidence.

This is preferable to TCP-only because it avoids irreversible transport coupling, and preferable to immediate QUIC because it does not weaken the already-accepted admission, ordering or security contracts.

## Decision

`protocol-oteryn` remains the single gameplay application protocol.

```text
gameplay/domain semantics
        |
protocol-oteryn messages
        |
transport-neutral session boundary
        +-- TCP + TLS 1.3 adapter        (registered profile 1)
        +-- QUIC v1 + TLS 1.3 adapter    (target; profile not yet registered)
```

Current accepted product/runtime policy:

- **TCP + TLS 1.3** remains the initial default and the only currently registered gameplay transport profile.
- **QUIC v1 + TLS 1.3** is the accepted target for a future player-opt-in preferred transport, but the ordinary client must not expose a functional QUIC admission option until the required FND-02/FND-04 profile reconciliation is accepted.
- **TCP_ONLY** remains available for diagnostics, compatibility and a remote operational kill switch.
- **QUIC_ONLY** remains developer/diagnostic-only after QUIC is implemented; it is never the ordinary player preference.
- Promotion of QUIC from target/experimental to player-available, and later to default, requires explicit follow-up acceptance and evidence.

## Exact FND-02 refinement and preserved clauses

This ADR changes only the **future direction** of the deferred QUIC alternative:

- FND-02 Section 5 remains correct that TCP is the first native gameplay transport and that QUIC requires measured latency/head-of-line/roaming evidence. The refinement is that Oteryn now intends to pursue QUIC as a future optional transport if those gates are satisfied.
- FND-02 Section 24 bullet `QUIC v1: deferred pending measured benefit` remains operationally true. “Deferred” now means **activation/profile registration is deferred**, not that the project is undecided whether QUIC belongs in the intended architecture.

Explicitly **not superseded**:

- transport profile `1 = tcp_tls13_alpn_v1`;
- TLS/ALPN/service-identity requirements for profile `1`;
- no plaintext/Canary fallback/protocol sniffing;
- early-data prohibition;
- `CommandId`, `server_sequence`, state-revision and reconciliation semantics;
- snapshot transfer and snapshot sequencing barrier;
- bounded input/resource requirements;
- fresh Gateway authorization after a terminal TLS/service-identity route failure;
- the measured-benefit requirement before QUIC activation/default promotion.

A later QUIC profile contract must explicitly add its profile semantics without rewriting or reusing TCP profile `1`.

## Required profile reconciliation before QUIC admission

Before `PREFER_QUIC` can become an available player mode, one bounded follow-up delivery must at minimum:

1. add a stable QUIC transport profile ID to `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json` without reusing profile `1`;
2. reconcile `FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md` so fresh admission can bind to the registered QUIC profile without weakening its one-time/fail-closed semantics;
3. reconcile `FND-04_REAUTHENTICATED_RECOVERY_GRANT_PROFILE_V1.md` so recovery can bind to the registered QUIC profile;
4. add exact resource ceilings and failure/conformance evidence required for externally controlled QUIC ingress;
5. prove that profile mismatch, downgrade and cross-transport replay fail closed;
6. prove the QUIC lane-ordering rules below under packet loss, stream delay/reordering and snapshot/resync scenarios.

Until that delivery merges, any QUIC adapter is at most an isolated spike and cannot participate in authoritative admission or recovery.

## Platform/Game Gateway boundary

Game Login Ticket ownership does not move.

The accepted flow remains:

```text
client
-> Oteryn Platform Game Gateway
-> Gateway redeems Game Login Ticket
-> Gateway returns one transport-bound pre-admission grant/route for a currently supported transport profile
-> client connects to GameNode
-> GameNode consumes/verifies pre-admission material
-> final game admission creates canonical GameSessionId
```

A Game Login Ticket is **never sent to or redeemed by the GameNode**.

The pre-admission grant is not the Game Login Ticket. It remains purpose-separated Platform-produced material consumed by final game admission.

For the current profile set, Gateway can issue profile `1` (TCP) only. After QUIC profile reconciliation, Gateway may issue a grant bound to the selected supported transport profile.

## Pre-admission fallback

Automatic player-visible fallback does not mean reusing one transport-bound grant on another transport profile.

If a future QUIC attempt fails at the transport layer **before admission succeeds**:

1. do not reinterpret the QUIC-bound pre-admission grant as TCP authority;
2. do not send the already-redeemed Game Login Ticket to the GameNode;
3. obtain a fresh Gateway-authorized fallback attempt and TCP-bound pre-admission grant through a fresh Game Login Ticket or a separately accepted Platform continuation mechanism;
4. preserve FND-04 issuance/retirement rules so an older possibly issued grant cannot remain independently usable alongside the fallback grant.

This baseline deliberately prefers correctness over shaving an extra control-plane round trip. A future optimized continuation mechanism requires its own reviewed contract.

## Safe fallback classification

Once QUIC admission is supported, `PREFER_QUIC -> TCP` fallback may be initiated only for transport-class failure, for example:

- UDP/QUIC path timeout without an authenticated application/security rejection;
- blocked or unusable UDP path;
- local QUIC stack initialization failure;
- authenticated rollout policy disabling QUIC.

Fallback must **not** reinterpret or bypass:

- certificate/service-identity rejection;
- ALPN/protocol rejection;
- invalid/expired/replayed grant or ticket at its owning boundary;
- unsupported client/protocol/transport profile;
- account/character restriction;
- entitlement denial;
- CharacterLease/session conflict;
- any other authenticated application or security decision.

TCP is never a lower-security recovery path.

## Invariants shared by both transports

When QUIC becomes registered, both transport adapters must preserve exactly the same:

- `protocol-oteryn` message semantics and versioning;
- service identity and certificate policy;
- ALPN/application identity;
- Platform/Game Gateway authentication boundary;
- pre-admission grant purpose separation;
- admission, CharacterLease and `GameSessionId` authority;
- `CommandId`, server sequencing, state revisions and reconciliation;
- `connection_generation` fencing;
- error/security meaning;
- bounded input/resource policy;
- reconnect and recovery semantics.

Gameplay/domain code must not branch on TCP versus QUIC.

## Active-session transport failure

After admission, a transport change uses accepted FND-04 reconnect/rebind semantics and an accepted credential/profile for the target transport:

```text
old transport fails
-> authorized reconnect/rebind
-> new connection_generation
-> stale binding fenced
-> snapshot/delta reconciliation
```

TCP and QUIC must never hold simultaneous command authority for the same Game Session generation.

QUIC connection migration may later preserve a QUIC connection across supported address/NAT changes, but it never migrates GameNode authority and does not replace FND-04 recovery.

## QUIC lane and ordering baseline

QUIC preserves ordering **within one stream, not across independent streams**. Therefore Oteryn cannot simply place FND-02 ordered state traffic on several streams and assume TCP-like total delivery order.

Baseline activation contract:

1. One reliable **server-authoritative ordered lane** carries every FND-02 `SERVER_SEQUENCED` message for a GameSession generation.
2. `SnapshotBegin`, all `SnapshotChunk` messages for that snapshot, and matching `SnapshotCommit` use that same server-authoritative ordered lane.
3. No `SERVER_SEQUENCED` message with `server_sequence > target_server_sequence` may be written to another QUIC stream to pass the snapshot. FND-02's snapshot publication barrier remains unchanged.
4. One reliable **client-command ordered lane** carries authoritative client command ingress whose `CommandId` ordering is interpreted by FND-02.
5. Semantically independent traffic such as bounded liveness/diagnostic control may use another lane only when its processing cannot advance or contradict FND-02 server-sequence, snapshot, command or state-revision authority.
6. Stream loss, reset or connection-generation change that affects an authoritative ordered lane follows existing FND-02/FND-04 failure/reconciliation semantics; the implementation must not silently reconstruct a different order from arrival time on other streams.

This baseline intentionally gives up some theoretical QUIC cross-stream parallelism in exchange for preserving already-accepted visible ordering.

A future design may split authoritative server state across multiple QUIC streams **only** if a separately reviewed bounded cross-lane resequencing/barrier mechanism proves:

- deterministic application order equal to FND-02 `server_sequence`;
- bounded memory/time under a stalled lane;
- correct snapshot begin/chunk/commit barrier behavior;
- no unbounded buffering or starvation;
- deterministic behavior under loss, stream reset, reordering and reconnect.

Until such evidence exists, cross-lane authoritative resequencing is not part of the baseline.

TCP may model the logical lane priorities with bounded application queues, but its bytes remain one ordered transport stream and retain TCP head-of-line behavior.

## 0-RTT and DATAGRAM

- TLS/QUIC **0-RTT is forbidden** for the baseline. No admission/recovery credential or gameplay mutation may be sent as early data.
- QUIC **DATAGRAM is disabled** in the baseline. A future decision may enable it only for independently bounded data whose loss/reordering is semantically safe.

## Security and resource requirements

Before a QUIC adapter can be accepted for admission, the resource-limit registry must gain exact ceilings for at least:

- concurrent/incomplete handshakes;
- per-address/prefix connection pressure;
- connection and stream counts;
- crypto/reassembly memory;
- receive/send buffers;
- handshake, idle and admission timeouts;
- malformed-packet work budgets;
- authoritative-lane pending bytes/messages and any permitted cross-lane control queues.

Ingress must remain isolated from authoritative `ChannelRuntime`; malformed transport traffic or adapter failure must not terminate the authoritative GameNode process.

QUIC dependencies require normal dependency/security review and a defined advisory-response path.

## Routing and load balancing

The first future QUIC implementation may route a client directly to the selected GameNode. A shared QUIC-aware load-balancer/Connection-ID routing design is deferred until multi-node evidence requires it.

No routing design may weaken `NodeId`, channel ownership, `connection_generation`, CharacterLease or same-channel recovery semantics.

## Library selection

No Rust QUIC implementation is frozen by this ADR. A bounded bake-off should compare maintained candidates such as Quinn and s2n-quic using Windows client/Linux GameNode evidence including:

- UDP blackhole and restricted-network behavior;
- packet loss/reordering;
- NAT rebinding/path change;
- large snapshot concurrent with semantically independent control traffic;
- authoritative ordered-lane behavior under loss/stall/reset;
- handshake CPU/rate and memory per connection;
- malformed/adversarial traffic;
- observability and operational maintainability;
- security-advisory response history.

## Rollout

1. **Now:** TCP profile `1` remains default and authoritative; QUIC is architecture target only.
2. **Development spike:** isolated QUIC implementation may be evaluated only after a task explicitly authorizes the spike; no admission authority.
3. **Player opt-in:** requires accepted transport-profile/grant reconciliation plus ordering, conformance, fault and resource evidence.
4. **Later default:** requires named telemetry/benchmark evidence and a separate accepted decision.

Required evidence includes connection-establishment success, fallback rate/reason, RTT/jitter/loss behavior, reconnect rate, CPU/memory, authoritative-lane ordering, snapshot/barrier correctness, network/region compatibility and whether QUIC delivers a measured player/operational benefit over TCP.

## Player UX

The intended ordinary UX after QUIC admission becomes accepted is a player-friendly preference such as **Prefer QUIC** or **Optimize connection stability**, with safe automatic TCP fallback. It must not imply that QUIC is guaranteed.

Before the profile-reconciliation gate is accepted, the production client must not expose this as a functional player option. Developer diagnostics may still describe future `AUTO_TCP_FIRST`, `PREFER_QUIC`, `TCP_ONLY` and `QUIC_ONLY` semantics without granting runtime authority.

## Future impact

If QUIC is later activated, Oteryn accepts the cost of maintaining two transport adapters and a broader compatibility/failure matrix while retaining one gameplay protocol and one authority model. Because profile IDs, grants and ordering are explicit, QUIC can be removed or disabled without changing gameplay semantics or the persisted game model.

If evidence fails to show sufficient benefit, TCP remains viable indefinitely and this ADR does not force production QUIC rollout.

## Decision timing

- **Must decide now?** YES for transport-neutral architecture and intended product direction; NO for activating QUIC admission before profile reconciliation/evidence.
- **Blocked downstream work:** clean transport-neutral adapter boundary and bounded QUIC research can proceed; authoritative QUIC admission, functional client QUIC option, Gateway QUIC profile selection, recovery over QUIC and production rollout remain blocked.
- **Harder later:** adding QUIC after transport-specific gameplay/session coupling or after assuming all ordered traffic necessarily shares TCP's single byte stream.
- **Evidence to supersede:** representative measurements or security/operations evidence showing QUIC adds insufficient benefit, or a different transport strategy is materially safer/better.
- **Deliberately not decided:** stable QUIC profile ID, exact library, numeric ceilings, optional future cross-lane resequencer, fallback delay, continuation mechanism, load-balancer product and date/threshold for QUIC-default promotion.
