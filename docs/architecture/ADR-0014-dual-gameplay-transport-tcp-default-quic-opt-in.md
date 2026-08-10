# ADR-0014: Dual gameplay transport — TCP default, QUIC opt-in and safe fallback

- Status: Accepted
- Date: 2026-08-10
- Decision ID: `NET-TRANSPORT-01`
- Supersedes: only the TCP-only/defer-QUIC transport-selection clauses of `FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md`; all application-protocol, sequencing, fencing, authentication and resource-safety requirements remain binding.
- Does not authorize: runtime adapter implementation, production listeners, Platform changes, final QUIC library choice, 0-RTT, QUIC DATAGRAM, or production rollout.

## Problem

`FND-02` selected TCP protected by TLS 1.3 as the conservative first transport. Oteryn now needs a transport strategy that can exploit QUIC where it improves latency/stability while preserving connectivity on networks that block or degrade UDP.

The decision must not create a second gameplay protocol, a weaker fallback path, duplicate active session authority, or a requirement to ship QUIC before evidence exists.

## Decision

`protocol-oteryn` remains the single gameplay application protocol.

```text
gameplay/domain semantics
        |
protocol-oteryn messages
        |
transport-neutral session boundary
        +-- TCP + TLS 1.3 adapter
        +-- QUIC v1 + TLS 1.3 adapter
```

Initial product policy:

- **TCP + TLS 1.3** is the default transport and mandatory safe fallback.
- **QUIC v1 + TLS 1.3** is an opt-in preferred transport exposed through the native client.
- A normal player preference means **prefer QUIC, then safely fall back to TCP after transport-class failure**.
- **TCP_ONLY** is supported for diagnostics, compatibility and a remote operational kill switch.
- **QUIC_ONLY** is developer/diagnostic only and is not the ordinary player setting.
- A later change making QUIC the default requires a new evidence-based decision.

## Invariants shared by both transports

Both adapters must preserve exactly the same:

- `protocol-oteryn` message semantics and versioning;
- service identity and TLS certificate verification;
- ALPN/application identity;
- Game Login Ticket semantics;
- admission, CharacterLease and `GameSessionId` authority;
- `CommandId`, server sequencing, state revisions and reconciliation;
- `connection_generation` fencing;
- error/security meaning;
- bounded input/resource policy;
- reconnect and recovery semantics.

Gameplay code must not branch on TCP versus QUIC.

## Transport selection and one-time credentials

The trusted Gateway/route authority advertises the transports allowed for the target GameNode. The client may establish transport candidates according to its setting, but the one-time gameplay credential is sent only on the selected, authenticated winning connection.

A ticket must not be redeemed concurrently over both transports as an ordinary connection strategy.

## Safe fallback

`PREFER_QUIC -> TCP` fallback is allowed only for transport-class failure, for example:

- UDP/QUIC path timeout without an authenticated policy response;
- blocked or unusable UDP path;
- local QUIC stack initialization failure;
- authenticated server policy temporarily disabling QUIC.

Fallback must **not** reinterpret or bypass:

- certificate/service-identity rejection;
- ALPN/protocol rejection;
- invalid/expired/replayed ticket;
- unsupported client/protocol version;
- account/character restriction;
- entitlement denial;
- CharacterLease/session conflict;
- any other authenticated application or security decision.

TCP is not a lower-security recovery path.

## Active-session transport failure

Transport switching after admission uses the accepted FND-04 reconnect/rebind model:

```text
old transport fails
-> reconnect/rebind
-> new connection_generation
-> stale binding fenced
-> snapshot/delta reconciliation
```

TCP and QUIC must never hold simultaneous command authority for the same Game Session generation.

QUIC connection migration may preserve a QUIC connection across supported client-address/NAT changes, but it does not migrate GameNode authority and does not replace FND-04 recovery.

## QUIC stream baseline

Baseline QUIC uses a small, fixed, bounded set of reliable logical lanes. It must not create a stream per gameplay message.

At minimum the implementation design must isolate latency-sensitive control/gameplay traffic from large state transfer traffic so that snapshot transfer does not unnecessarily block session/control messages. Exact lane count, direction and priority API remain implementation-spike decisions.

TCP fallback may model the same logical lanes with bounded application queues even though TCP retains transport-level head-of-line blocking.

## 0-RTT and DATAGRAM

- TLS/QUIC **0-RTT is forbidden** for the baseline. No admission credential or gameplay mutation may be sent as early data.
- QUIC **DATAGRAM is disabled** in the baseline. A future decision may enable it only for data whose loss/reordering is semantically safe and independently bounded.

## Security and resource requirements

Before a QUIC adapter can be accepted, the resource-limit registry must gain exact ceilings for at least:

- concurrent/incomplete handshakes;
- per-address/prefix connection pressure;
- connection and stream counts;
- crypto/reassembly memory;
- receive/send buffers;
- handshake, idle and admission timeouts;
- malformed-packet work budgets.

Ingress must be isolated from authoritative `ChannelRuntime`; malformed transport traffic or adapter failure must not terminate the authoritative GameNode process.

QUIC dependencies require normal dependency/security review and a defined advisory response path.

## Routing and load balancing

The first implementation may route a client directly to the selected GameNode and advertise both that node's TCP and QUIC endpoints. A shared QUIC-aware load-balancer/Connection-ID routing design is deferred until multi-node evidence requires it.

No routing design may weaken `NodeId`, channel ownership, `connection_generation`, CharacterLease or same-channel recovery semantics.

## Library selection

No Rust QUIC implementation is frozen by this ADR. A bounded bake-off should compare maintained candidates such as Quinn and s2n-quic using Windows client/Linux GameNode evidence including:

- UDP blackhole and restricted-network fallback;
- packet loss/reordering;
- NAT rebinding/path change;
- large snapshot concurrent with control traffic;
- handshake CPU/rate and memory per connection;
- malformed/adversarial traffic;
- observability and operational maintainability;
- security-advisory response history.

## Rollout

1. Development/closed alpha: TCP default; QUIC opt-in/experimental; safe fallback mandatory.
2. Beta: broaden QUIC eligibility under authenticated rollout policy; retain TCP default and `TCP_ONLY` kill switch.
3. Later: consider QUIC default only after named telemetry/benchmark evidence and a separate accepted decision.

Required evidence includes connection-establishment success, fallback rate/reason, RTT/jitter/loss behavior, reconnect rate, CPU/memory, snapshot interference and network/region compatibility.

## Player UX

The ordinary UI should expose a player-friendly preference such as **Prefer QUIC** or **Optimize connection stability**. It must not imply that QUIC is guaranteed when fallback is enabled.

Advanced diagnostics may expose `AUTO_TCP_FIRST`, `PREFER_QUIC`, `TCP_ONLY` and developer-only `QUIC_ONLY` semantics.

## Decision timing

- **Must decide now?** YES — transport-neutral session boundaries, client settings, endpoint offers and implementation spikes otherwise risk binding the product to TCP-only assumptions.
- **Blocked downstream work:** transport adapters, client networking settings, Gateway transport offers, conformance tests and rollout telemetry.
- **Harder later:** adding QUIC after transport-specific gameplay/session coupling or after one-time credentials are redeemed before transport selection.
- **Evidence to supersede:** representative production-like measurements showing another default/fallback strategy materially improves reliability, latency or operational safety.
- **Deliberately not decided:** exact library, numeric ceilings, lane count, fallback delay, load-balancer product and date/threshold for QUIC-default promotion.
