# ADR-0014: Dual gameplay transport — TCP default, QUIC opt-in target and safe fallback

- Status: Accepted strategy; QUIC admission activation blocked pending protocol/admission profile reconciliation
- Date: 2026-08-10
- Decision ID: `NET-TRANSPORT-01`
- Supersedes: the long-term TCP-only/defer-QUIC direction of `FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md` at strategy level only. The currently registered/accepted transport profile remains TCP profile `1` until a dedicated contract revision registers QUIC and reconciles FND-04 fresh/recovery grants.
- Does not authorize: QUIC admission, runtime adapter implementation, production listeners, Platform changes, final QUIC library choice, 0-RTT, QUIC DATAGRAM, or production rollout.

## Problem

`FND-02` selected TCP protected by TLS 1.3 as the conservative first transport. Oteryn should preserve that safe baseline while preparing a transport-neutral architecture that can later use QUIC where measurements prove a player benefit.

The strategy must not create a second gameplay protocol, weaken authentication/fallback, bypass the Platform Game Gateway, or claim QUIC admission compatibility before the canonical transport/admission profiles support it.

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

## Required profile reconciliation before QUIC admission

Before `PREFER_QUIC` can become an available player mode, one bounded follow-up delivery must at minimum:

1. add a stable QUIC transport profile ID to `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json` without reusing profile `1`;
2. reconcile `FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md` so fresh admission can bind to the registered QUIC profile without weakening its one-time/fail-closed semantics;
3. reconcile `FND-04_REAUTHENTICATED_RECOVERY_GRANT_PROFILE_V1.md` so recovery can bind to the registered QUIC profile;
4. add exact resource ceilings and failure/conformance evidence required for externally controlled QUIC ingress;
5. prove that profile mismatch, downgrade and cross-transport replay fail closed.

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

## QUIC stream baseline

A future QUIC implementation should use a small fixed bounded set of reliable logical lanes, not a stream per gameplay message. Latency-sensitive control/gameplay traffic should be isolated from large state transfer traffic where the selected QUIC API permits it.

Exact stream/lane count and priority API remain implementation-spike decisions.

TCP may model the same logical lanes with bounded application queues even though TCP retains transport-level head-of-line blocking.

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
- malformed-packet work budgets.

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
- large snapshot concurrent with control traffic;
- handshake CPU/rate and memory per connection;
- malformed/adversarial traffic;
- observability and operational maintainability;
- security-advisory response history.

## Rollout

1. **Now:** TCP profile `1` remains default and authoritative; QUIC is architecture target only.
2. **Development spike:** isolated QUIC implementation may be evaluated only after a task explicitly authorizes the spike; no admission authority.
3. **Player opt-in:** requires accepted transport-profile/grant reconciliation plus conformance, fault and resource evidence.
4. **Later default:** requires named telemetry/benchmark evidence and a separate accepted decision.

Required evidence includes connection-establishment success, fallback rate/reason, RTT/jitter/loss behavior, reconnect rate, CPU/memory, snapshot interference and network/region compatibility.

## Player UX

The intended ordinary UX after QUIC admission becomes accepted is a player-friendly preference such as **Prefer QUIC** or **Optimize connection stability**, with safe automatic TCP fallback. It must not imply that QUIC is guaranteed.

Before the profile-reconciliation gate is accepted, the production client must not expose this as a functional player option. Developer diagnostics may still describe future `AUTO_TCP_FIRST`, `PREFER_QUIC`, `TCP_ONLY` and `QUIC_ONLY` semantics without granting runtime authority.

## Decision timing

- **Must decide now?** YES for transport-neutral architecture and intended product direction; NO for activating QUIC admission before profile reconciliation/evidence.
- **Blocked downstream work:** QUIC admission adapter, functional client QUIC option, Gateway QUIC transport selection, recovery over QUIC and production rollout.
- **Harder later:** adding QUIC after transport-specific gameplay/session coupling.
- **Evidence to supersede:** representative measurements or security/operations evidence showing another default/fallback strategy is safer or materially better.
- **Deliberately not decided:** stable QUIC profile ID, exact library, numeric ceilings, lane count, fallback delay, continuation mechanism, load-balancer product and date/threshold for QUIC-default promotion.
