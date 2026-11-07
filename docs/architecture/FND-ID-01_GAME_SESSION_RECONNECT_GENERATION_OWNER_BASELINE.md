# FND-ID-01 Game Session Reconnect Generation Owner Baseline

- Status: Owner-accepted pre-contract baseline
- Date: 2026-08-07
- Decision owner: Oteryn project owner
- Gate: `FND-ID-01` input; mandatory consumer: `FND-04`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: `GameSessionId`, reconnect, transport rebinding, stale-packet fencing and duplicate-login recovery

## Purpose

Record the owner-accepted semantic distinction between a logical gameplay session and the concrete transport currently bound to that session.

This baseline freezes reconnect identity/fencing semantics and the initial reconnect grace-window policy value. It does not choose the final transport, heartbeat interval, reconnect token format, database schema, wire field names or cryptographic construction.

## Owner-accepted model

A bounded reconnect after a genuine transport/network interruption preserves the existing logical `GameSessionId` when the authoritative game-session lease/session remains current and reconnect eligibility is proven.

Conceptually:

```text
GameSessionId S1
    transport generation 5 lost
    -> eligible reconnect
GameSessionId S1
    transport generation 6 becomes authoritative
```

A transient transport loss is therefore **not by itself a new gameplay session**.

## Initial reconnect grace-window policy

The owner-accepted **initial default reconnect grace window is 15 seconds**.

Conceptually:

```text
reconnect_grace_window = 15 seconds
```

This is the starting production-policy target for later `FND-04` implementation design, not an immutable protocol constant. It may be tuned later through an explicit architecture/product decision informed by testing, gameplay safety and telemetry.

The exact server-side instant from which the 15-second window is measured remains `FND-04` work because it depends on the accepted liveness/transport-loss state machine. The value must not be measured from an untrusted client-supplied timestamp.

The same initial **15-second reconnect grace window applies while the character has an active combat/PZ/logout lock**. Combat/PZ state does not create a longer reconnect grace window and does not create a shorter one.

The grace window controls continuity of the logical `GameSessionId`; it does **not** override the separate combat/logout-presence rules. Expiry of the reconnect grace window must not by itself despawn, safe-log, teleport, protect or otherwise remove a combat/PZ/logout-locked actor from mandatory world presence. Exact post-expiry recovery/admission behavior remains `FND-04` work.

## Logical session versus transport binding

`GameSessionId` identifies the logical admitted gameplay session.

The currently authoritative network connection is a replaceable binding underneath that logical session.

The architecture must distinguish at least conceptually:

```text
GameSessionId
+ current transport/connection generation
```

The semantic term `connection_generation` may be used in architecture discussion. `FND-02`/`FND-04` may choose the final field name and physical representation while preserving the accepted semantics.

## Reconnect that preserves GameSessionId

The same `GameSessionId` may continue across reconnect only when all relevant authoritative checks succeed, including conceptually:

- the account/character binding still belongs to the same logical gameplay session;
- the prior session/lease has not been terminally revoked or replaced;
- reconnect occurs inside the accepted reconnect eligibility window;
- the server proves the previous transport is stale/lost or otherwise eligible for rebinding;
- the reconnecting client proves whatever reconnect credential/context `FND-04` later requires;
- the same in-world actor is resumed rather than recreated.

Successful reconnect establishes a **new transport/connection generation**.

## Stale transport fencing

Each accepted rebind must advance or replace the transport-level fencing generation so that only the newest binding may submit authoritative gameplay commands.

After generation `N+1` becomes authoritative:

- commands from generation `N` or older fail closed;
- delayed/reordered packets from the prior transport cannot regain authority;
- a stale connection reopening at the network layer does not restore gameplay authority;
- acknowledgment or close delivery to the stale transport is not required for correctness.

The exact counter width, monotonic representation, rollover rules and wire encoding remain later contract work.

## State continuity

Reconnect to the same `GameSessionId` and same in-world actor must not by itself reset or recreate gameplay state.

At minimum reconnect must not implicitly:

- respawn or duplicate the character;
- teleport or reposition the actor;
- heal or refill resources;
- clear combat/PZ/logout locks;
- clear conditions, damage-over-time, cooldowns or exhaustion;
- reset encounter/instance state;
- create invulnerability or protection windows;
- roll back already committed authoritative actions.

The session layer reconnects control to existing authoritative state; it is not a gameplay reset primitive.

## When a new GameSessionId is required

A new `GameSessionId` is required after the previous logical gameplay session has ended rather than merely losing transport.

Terminal boundaries include conceptually:

- authoritative session termination/logout completion;
- lease/session revocation;
- expiry beyond the accepted reconnect eligibility window;
- a completed legal takeover that ends/replaces the old logical gameplay session;
- another `FND-04` terminal state explicitly classified as session end.

Exact terminal-state enumeration remains `FND-04` work, but transport interruption alone is not sufficient.

## Relationship to duplicate-login protection

This baseline consumes the accepted account single-online-character and combat-aware duplicate-login rules.

A second client must not create a new authoritative gameplay session merely because it can authenticate while a healthy protected incumbent session is still authoritative.

When the incumbent transport is genuinely unavailable and same-character reconnect is eligible, the reconnect may preserve the existing `GameSessionId` while advancing the transport generation.

A different `CharacterId` does not use this reconnect path while the incumbent character still has mandatory world presence.

## Failure and race requirements

`FND-04`, `FND-02` and later runtime/persistence contracts must preserve the model under at least:

- delayed packets from the previous transport after successful reconnect;
- old and new transports briefly existing simultaneously;
- rapid disconnect/reconnect loops;
- real transport loss racing with heartbeat/liveness evidence;
- duplicate-login attempts racing with reconnect;
- GameNode/channel routing changes during reconnect;
- reconnect while combat/PZ/logout lock remains active;
- reconnect racing with death or session termination;
- reconnect grace expiry while combat/PZ/logout lock remains active;
- retry/replay of admission or reconnect requests;
- process crash while establishing the new transport generation.

The safe direction is to keep at most one current authoritative transport binding for the logical gameplay session.

## Security consequence

Possession of `GameSessionId` alone is never sufficient to reconnect or regain authority.

Later contracts must require current reconnect/session credentials and authoritative lease/fencing validation. Replayed identifiers or stale reconnect material must not acquire control.

## Required application to later contracts

This baseline is mandatory input to:

- completion of the `FND-ID-01` identifier/session catalogue;
- `FND-04` Game Session, reconnect, lease, liveness and takeover state machine;
- `FND-02` session/reconnect/fencing fields once semantics are consumed by the protocol contract;
- `DUR-02` if session or fencing state must survive process/node failure;
- QA/E2E reconnect, stale-packet, duplicate-login and combat-X-log scenarios.

## Programme effect

Accepted now:

```text
short eligible reconnect != new logical GameSession
initial reconnect grace window = 15 seconds
combat/PZ/logout-locked reconnect grace window = same 15 seconds
same GameSessionId may survive transport loss
accepted rebind creates a newer transport/connection generation
older transport generations lose command authority
terminal session end -> next admission receives a new GameSessionId
```

Still unresolved:

- exact `GameSessionId` representation and issuer;
- exact transport-generation field name/representation;
- exact server-authoritative start/measurement semantics for the 15-second window;
- heartbeat/liveness thresholds;
- reconnect credential/token construction;
- transport cryptographic re-key behavior;
- exact terminal-state enumeration;
- exact post-grace behavior while a combat/PZ/logout-locked actor remains in mandatory world presence;
- persistence requirements for reconnect/session fencing;
- final protocol/wire encoding.

No runtime, protocol, persistence, database or Platform implementation is authorized by this baseline.
