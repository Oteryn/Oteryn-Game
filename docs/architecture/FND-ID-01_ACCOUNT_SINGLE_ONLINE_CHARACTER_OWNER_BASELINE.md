# FND-ID-01 Account Single Online Character Owner Baseline

- Status: Owner-accepted pre-contract baseline
- Date: 2026-08-07
- Decision owner: Oteryn project owner
- Gate: `FND-ID-01` input; mandatory consumer: `FND-04`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: account/character concurrency, Game Session admission, duplicate login, reconnect, character lease, combat-aware takeover and stale-session fencing

## Purpose

Record the project owner's accepted gameplay-presence and duplicate-login invariants before `FND-04` freezes the exact Game Session, admission and character-lease mechanism.

This document fixes the concurrency and takeover behavior at the semantic level. It does not select token format, lease storage, timeout values, transport protocol, database schema or Platform implementation.

## Owner-accepted invariant

For one Platform-owned `AccountId`:

```text
count(authoritative_online_characters(AccountId)) <= 1
```

The rule is account-global. It is not reset by changing `WorldId`, `ChannelId`, `InstanceId`, GameNode, client device, network connection, geographic region or character name.

Therefore one account cannot legitimately have one character online on World A while another character from the same account is online on World B.

## Scope of "online"

For this invariant, **online** means holding current authoritative gameplay permission for a character under the accepted session/lease/fencing model.

The following do not by themselves count as a second online gameplay character:

- an authenticated Platform WWW/OAuth session;
- account-management browsing;
- a stale transport that has already lost authoritative gameplay rights;
- a reconnect/takeover candidate that has not yet acquired current gameplay authority;
- a login-screen or character-selection client without an admitted character session.

A transport may overlap briefly during a fenced handoff only if no more than one gameplay authority can mutate state.

## Relationship to CharacterId

`CharacterId` remains durable semantic character identity and is independent of online-session authority.

This rule does not alter CharacterId UUIDv7 representation, global uniqueness, rename semantics, world-transfer identity preservation, deletion/non-reuse rules or AccountId -> CharacterId ownership linkage.

## Required authority invariant

Later contracts must preserve an account-level exclusion rule conceptually equivalent to:

```text
AccountId A
    -> at most one current player-controlled authoritative
       (CharacterId, GameSessionId, session_generation, character lease/fence)
```

The exact tuple is illustrative. `FND-04` owns the physical representation.

Character-level fencing alone is insufficient if two different CharacterIds belonging to the same AccountId could independently hold player gameplay authority.

## Owner-accepted duplicate-login policy: conditional newcomer takeover

A fully authenticated new login may supersede the incumbent gameplay session **only when takeover is eligible**.

Takeover eligibility is server-authoritative. A second client does not gain the right to revoke the incumbent merely by presenting valid account credentials.

The accepted rules are:

- if the incumbent character is logout-eligible, the newcomer may replace the incumbent through a fenced legal handoff;
- if the incumbent client/session is still healthy and the incumbent character has an active combat/PZ/logout blocker, the newcomer must **not** fence, close, revoke or steal that gameplay session;
- if the incumbent transport/session is genuinely unavailable, recovery is handled as reconnect/failover and not as a hostile duplicate-login preemption;
- the newcomer must still pass normal authentication, account ownership, admission, ban/entitlement and other security checks.

The takeover attempt must be auditable. Exact audit storage, rate limits, risk-based authentication and user-facing warnings remain later security/product work.

## Combat-aware takeover: no escape and no forced combat disconnect

A duplicate login must never be usable either to escape combat consequences **or** to force a healthy fighting client to lose control.

The authoritative server decides whether the incumbent character has a combat/PZ/logout blocker and whether the incumbent gameplay session is still live. Client disconnect claims, reconnect requests or takeover requests cannot clear combat state or falsely declare the incumbent dead.

At minimum, active combat/PZ/logout lock is a mandatory takeover blocker while the incumbent session remains healthy. Later gameplay contracts may define additional blockers.

### Incumbent is logout-eligible

If the incumbent character has no mandatory logout blocker:

1. authenticate and authorize the newcomer;
2. establish that takeover is eligible;
3. fence/revoke the incumbent client authority;
4. complete the old character's legal logout/removal;
5. only then grant authoritative gameplay admission to the selected new character.

The transition may be atomic or staged, but it must never expose two player-controlled authoritative characters for the account.

### Incumbent is combat/PZ/logout locked and client is healthy

If the incumbent character has an active combat/PZ/logout blocker **and the incumbent gameplay session is still healthy/live**:

1. authenticate the newcomer at the account layer if credentials are valid;
2. do **not** fence, revoke, close or otherwise interrupt the incumbent gameplay client;
3. keep the incumbent client fully authoritative for its existing character;
4. deny or hold pending any gameplay takeover request;
5. do not admit a different `CharacterId` for the same `AccountId`;
6. retry eligibility only after the blocker clears or the incumbent session genuinely becomes unavailable.

This prevents both self-abuse and malicious use of valid/stolen credentials to make a fighting client disconnect at a dangerous moment.

### Incumbent is combat/PZ/logout locked but transport/session is genuinely unavailable

If the incumbent character must remain in world simulation because of combat/PZ/logout state but its client transport/session has genuinely been lost:

- the character remains in authoritative world simulation under normal disconnected/combat rules;
- a different `CharacterId` for that account stays blocked until the incumbent no longer has mandatory world presence;
- reconnecting the **same CharacterId** may recover control of the same in-world actor once `FND-04` proves the old transport/session is stale and establishes a new fenced session generation;
- reconnect must not respawn, teleport, duplicate, protect, heal, reset or otherwise alter the actor as a side effect;
- server-side lease/session liveness is authoritative; a second client cannot manufacture "old session lost" by simply requesting takeover.

This preserves all three requirements:

```text
one account has at most one playable character
combat state cannot be escaped by switching/logging
second client cannot kick a healthy combat-locked first client
```

## Anti-abuse invariants

Takeover, disconnect or reconnect must not by itself reset or clear any gameplay state that would give a combat advantage, including conceptually:

- combat/PZ/logout lock;
- HP, mana or other resources;
- position or facing;
- conditions, damage-over-time or debuffs;
- cooldowns and exhaustion;
- aggro/threat or combat attribution where applicable;
- death risk and already committed combat consequences;
- instance or encounter state;
- server-scheduled effects already committed before the authority transition.

Exact combat-engine semantics remain owned by the combat/gameplay contracts, but the session layer may not use reconnect/takeover as a reset primitive.

Commands from an incumbent session are rejected only after a legitimate fencing boundary has been established. Merely opening or authenticating a second client must not create that boundary while a protected incumbent session is healthy.

Already committed authoritative actions are not rolled back merely because a later takeover or reconnect occurs.

## Character switching

Switching from Character A to Character B under the same account must not create an interval in which both are player-controlled authoritative gameplay writers.

If Character A is combat/PZ/logout locked, Character B cannot receive gameplay admission while Character A has mandatory world presence.

If Character A's client is still healthy, the second client cannot force Character A into disconnected/X-log behavior merely by attempting to log Character B or Character A.

The safe direction under ambiguity or partial failure is to preserve the current healthy incumbent authority and delay/reject the newcomer rather than allow dual authority, combat escape or forced loss of control.

## Same-character reconnect/takeover distinction

A second client selecting the **same CharacterId** is not automatically entitled to preempt a healthy incumbent client during combat/PZ/logout lock.

If the incumbent session is healthy, the existing client remains in control.

If the incumbent session is genuinely unavailable, `FND-04` may allow immediate same-character reconnect to the exact same in-world actor using a fresh fenced session generation. This must preserve the actor's complete state and must not create a second copy, respawn, teleport, clear combat state or create an invulnerability window.

Exact liveness timeout, lease expiry, reconnect grace period and fencing transaction remain `FND-04` work.

## Failure and race requirements

`FND-04` and persistence/runtime contracts must prove the invariant under at least:

- simultaneous login attempts from two devices;
- two different CharacterIds selected nearly simultaneously;
- takeover while the incumbent is actively fighting;
- takeover immediately before/after combat lock acquisition or expiry;
- repeated second-client login attempts against a healthy combat-locked incumbent;
- reconnect racing with a fresh duplicate login;
- real transport loss racing with a healthy-session heartbeat;
- channel or instance switch racing with duplicate login;
- character death while a takeover/reconnect is pending;
- Game Gateway retry/replay;
- GameNode crash during handoff;
- lease timeout and delayed packets;
- stale session-generation messages;
- Platform/Gateway/game-database partial failure.

The safe failure direction is toward preserving the healthy incumbent session or, after proven loss, toward zero or one gameplay authority. The system must never create two authoritative playable characters, instant combat escape, or a forced combat disconnect triggered only by another client login.

## Cross-repository ownership consequence

`AccountId` remains owned by Oteryn Platform Identity and CharacterId remains game-domain identity.

The single-online-character and takeover rules cross the Platform/Gateway/game boundary and therefore require an explicit contract. They must not be implemented through unrestricted cross-database coupling or by making Platform the owner of gameplay character state.

Exact responsibility for admission artifacts, account-level exclusion lease, character lease, fencing generations, liveness proof and pending-takeover reconciliation remains `FND-04` work.

No write to `blakinio/Oteryn-Platform` is authorized by this baseline.

## Security consequence

A valid AccountId, CharacterId or GameSessionId is not sufficient authority by itself.

Production admission must validate current account ownership and current session/lease/fencing state so that stale sessions cannot keep playing, a second CharacterId cannot bypass the account exclusion rule, world/channel/instance changes cannot bypass it, replayed identifiers cannot acquire gameplay authority, and a second authenticated client cannot weaponize takeover to disconnect a healthy combat-locked incumbent.

## Required application to later contracts

This baseline is mandatory input to:

- completion of the `FND-ID-01` ownership/concurrency catalogue;
- `FND-04` Game Session/admission/character-lease contract;
- `FND-02` fields that carry session/admission identity after semantics are fixed;
- `DUR-02` persistence/fencing model where durable lease/session state is required;
- combat/logout lifecycle rules;
- QA/E2E duplicate-login, combat-X-log, forced-disconnect and reconnect scenarios;
- Platform/Game Gateway reconciliation where AccountId participates in admission.

## Programme effect

Accepted now:

```text
one AccountId = maximum one authoritative online CharacterId
logout-eligible incumbent -> valid newcomer may replace it through fenced handoff
healthy combat/PZ/logout-locked incumbent -> newcomer cannot kick or steal the session
proven incumbent connection/session loss -> same-character reconnect may recover the existing actor without resetting combat state
different character remains blocked while incumbent has mandatory world presence
```

This section supersedes the unconditional "newcomer always wins control" wording introduced by PR #71 for the case of a healthy combat/PZ/logout-locked incumbent.

Still unresolved:

- exact GameSessionId representation and issuer;
- AdmissionId representation and issuer;
- CharacterLeaseId representation and issuer;
- account-level versus character-level lease physical layout;
- lease location, TTL, renewal and revocation;
- session-generation increment rules;
- exact healthy/stale transport liveness proof;
- reconnect grace period;
- exact disconnected-character combat behavior;
- exact Gateway/game transaction and failure state machine;
- exact UX while takeover is blocked or pending.

No runtime, protocol, persistence, database or Platform implementation is authorized by this decision alone.
