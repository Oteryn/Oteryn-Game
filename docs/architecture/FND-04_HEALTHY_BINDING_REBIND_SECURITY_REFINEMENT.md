# FND-04 — Healthy Transport Binding Rebind Security Refinement

- Status: Candidate normative FND-04 refinement; canonical when the owning FND-04 delivery merges
- Date: 2026-08-08
- Gate: `FND-04`
- Refines: `FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md`, especially Sections 4, 14, 15 and 20
- Applies to: same-GameSession transport replacement using a reconnect secret or reauthenticated recovery proof
- Does not authorize: runtime/protocol implementation, transport migration feature, Platform writes or production traffic

## 1. Security problem

A reconnect secret is a high-entropy bearer proof. A reauthenticated recovery grant is a stronger Platform-authenticated attempt proof. Neither one, by itself, is permission to evict a healthy current playable transport.

Without an explicit incumbent-binding rule, a stolen predecessor reconnect secret or separately authenticated second client could attempt PREPARE while the current generation is still healthy and use a successful COMMIT to displace the legitimate player.

FND-04 therefore freezes the current-binding eligibility rule below.

## 2. Healthy current binding is non-preemptible by recovery proof alone

When a GameSession has a current TransportBinding whose current connection_generation still has accepted sufficient playable-control evidence, an unsolicited PREPARE from another transport MUST be rejected.

This applies even when the newcomer presents:

- the current reconnect secret;
- a valid reauthenticated recovery grant;
- the correct GameSessionId;
- the correct AccountId/CharacterId;
- a newer local TLS connection;
- a fresh ReconnectAttemptRef.

None of those facts independently authorize replacement of healthy current control.

A rejected contender:

- does not create prepared rebind state;
- does not mint/disclose a successor reconnect secret;
- does not advance connection_generation;
- does not invalidate the incumbent reconnect proof;
- does not close/fence the incumbent transport;
- does not create a ControlLossEpoch or defensive re-entry protection;
- maps to the existing coarse healthy-controller/conflict outcome.

## 3. Unsolicited recovery eligibility

A new transport may enter normal reconnect PREPARE without current-binding cooperation only after server-authoritative state establishes that the incumbent binding is eligible for replacement, for example:

```text
eligible unexpected playable-control loss has been declared
AND GameSession is RECONNECTABLE
AND old/current binding is stale/lost under current FND-04/FND-03 state
```

The concrete socket may already have been closed by the accepted stale-transport cleanup path; socket closure itself is not the authority proof.

Current GameSession, CharacterLease, runtime placement, reconnect grace and reconciliation checks remain mandatory.

## 4. Pre-loss / healthy transport migration is a distinct transition

FND-04 does not prohibit a future seamless migration of a healthy session to another transport/device/path, but it must not be implemented as an unsolicited bearer-secret reconnect.

A controlled healthy-binding migration requires an explicit authorization rooted in the **current authoritative connection_generation**, such as a server-issued one-time migration challenge/intent acknowledged by the current binding or another separately accepted equivalent proof.

Minimum invariants for any later migration contract:

- current binding participates in or explicitly authorizes the migration while still authoritative;
- authorization is bound to GameSessionId, current connection_generation, destination attempt and a short bounded lifetime;
- one migration attempt has at most one winner;
- PREPARE still grants no destination command/liveness authority;
- COMMIT atomically switches generation/current binding and fences predecessor;
- a stale migration authorization cannot preempt a later generation;
- intentional healthy migration does not create ControlLossEpoch or four-second disconnect re-entry protection;
- a second device merely knowing the reconnect secret cannot manufacture current-binding authorization.

Exact protocol messages and migration UX remain later design and are not required for first-release reconnect.

## 5. Reauthenticated recovery grant interaction

`FND-04_REAUTHENTICATED_RECOVERY_GRANT_PROFILE_V1.md` already requires rejection when a healthy controller exists. This refinement makes the same non-preemption invariant apply to the shared PREPARE/COMMIT machinery itself.

Therefore a valid recovery JWT can authorize same-GameSession recovery only when the game-domain session state is genuinely recovery-eligible. Platform reauthentication never converts a healthy current GameSession into a takeover target.

Intentional logout-eligible takeover remains the separate `TAKEOVER_DRAINING` path and results in a fresh GameSession where the main FND-04 contract requires it.

## 6. Reconnect secret theft consequence

Possession of a stolen reconnect secret may let an attacker race a legitimate reconnect **after** server-declared loss, but it cannot be used to kick a healthy current binding.

The existing one-prepared-rebind / one-COMMIT-winner rules determine the post-loss race. A stale or losing proof cannot fence the winner.

Future sender-constrained/PoP reconnect credentials may further reduce stolen-bearer-secret risk, but are not required for v1 architecture acceptance.

## 7. Required implementation evidence

Before reconnect implementation acceptance, tests MUST demonstrate:

1. current generation healthy + correct reconnect secret from second transport -> PREPARE rejected, incumbent unaffected;
2. current generation healthy + valid reauthenticated recovery grant -> PREPARE/recovery rejected, incumbent unaffected;
3. current generation healthy + multiple concurrent contenders -> none can create prepared state without current-binding migration authorization;
4. server-declared eligible loss -> one valid reconnect contender may PREPARE and exactly one may COMMIT;
5. pre-loss current-binding-authorized migration, if implemented, switches authority atomically without creating ControlLossEpoch/protection;
6. stale migration authorization from generation N cannot affect generation N+1;
7. stolen predecessor reconnect secret after successful COMMIT cannot regain authority or fence successor.

## 8. Concise rule

```text
healthy current binding
+ reconnect secret / recovery JWT on another transport
-> NOT replacement authority
-> reject unsolicited PREPARE

server-proven eligible loss
-> reconnect PREPARE may proceed
-> one candidate / one COMMIT winner

healthy intentional migration
-> separate current-generation-authorized transition
-> never bearer-secret-only takeover
-> no disconnect protection
```
