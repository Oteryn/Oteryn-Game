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

## Owner-accepted duplicate-login policy: newcomer wins

After a second login/takeover request for the same `AccountId` has been fully authenticated and accepted by admission policy, the **new login supersedes the incumbent client session**.

The incumbent transport/session must be fenced so it cannot continue submitting authoritative player commands.

This is not an authorization shortcut: the newcomer must still pass normal authentication, account ownership, admission, ban/entitlement and other security checks.

The takeover itself must be auditable. Exact audit storage, rate limits, risk-based authentication and user-facing warnings remain later security/product work.

## Combat-aware takeover: no escape-by-login

A duplicate login must never be usable as a way to erase combat consequences or force an instant safe logout.

The authoritative server decides whether the incumbent character has a combat/logout blocker. Client disconnect, reconnect or takeover cannot clear that state.

At minimum, active combat lock is a mandatory blocker. Later gameplay contracts may define additional logout blockers.

### Incumbent is logout-eligible

If the incumbent character has no mandatory logout blocker:

1. authenticate and authorize the newcomer;
2. fence/revoke the incumbent client authority;
3. complete the old character's legal logout/removal;
4. only then grant authoritative gameplay admission to the selected new character.

The transition may be atomic or staged, but it must never expose two player-controlled authoritative characters for the account.

### Incumbent is combat/logout locked

If the incumbent character has an active combat lock or another mandatory logout blocker:

1. authenticate and authorize the newcomer;
2. fence/revoke the incumbent **client control** immediately;
3. do **not** remove, teleport, protect or otherwise safe-log the incumbent character because of the takeover;
4. keep that character in authoritative world simulation under the normal disconnected/combat-presence rules until the blocker resolves, the character legally logs out, or death resolution completes;
5. keep gameplay admission of a **different CharacterId** for that AccountId pending/blocked until the incumbent character no longer has mandatory world presence.

The newcomer may remain authenticated at the account/login layer while gameplay admission is pending. Authentication is not gameplay authority.

This preserves both requirements:

```text
newcomer session wins control
AND
combat state cannot be escaped by switching characters
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
- server-scheduled effects already committed before the fence.

Exact combat-engine semantics remain owned by the combat/gameplay contracts, but the session layer may not use reconnect/takeover as a reset primitive.

Commands arriving from the fenced incumbent session after the authority boundary must fail closed. Already committed authoritative actions are not rolled back merely because takeover occurs.

## Character switching

Switching from Character A to Character B under the same account must not create an interval in which both are player-controlled authoritative gameplay writers.

If Character A is combat/logout locked, Character B cannot receive gameplay admission merely because a fresh login displaced Character A's client connection.

The safe direction under ambiguity or partial failure is to delay/reject the new gameplay admission rather than allow dual authority or combat escape.

## Same-character takeover remains a separate detail

Whether a newcomer selecting the **same CharacterId** may immediately reattach to the existing in-world actor during combat, without resetting any state, remains to be frozen explicitly in `FND-04`.

Whatever policy is chosen must preserve the same actor state and must not create a second copy, respawn, teleport, clear combat state or create an invulnerability window.

## Failure and race requirements

`FND-04` and persistence/runtime contracts must prove the invariant under at least:

- simultaneous login attempts from two devices;
- two different CharacterIds selected nearly simultaneously;
- takeover while the incumbent is actively fighting;
- takeover immediately before/after combat lock acquisition or expiry;
- reconnect racing with a fresh login;
- channel or instance switch racing with duplicate login;
- character death while a takeover is pending;
- Game Gateway retry/replay;
- GameNode crash during handoff;
- lease timeout and delayed packets;
- stale session-generation messages;
- Platform/Gateway/game-database partial failure.

The safe failure direction is toward **zero or one** player gameplay authority and preservation of incumbent combat consequences, never two authoritative playable characters and never instant escape from combat.

## Cross-repository ownership consequence

`AccountId` remains owned by Oteryn Platform Identity and CharacterId remains game-domain identity.

The single-online-character and takeover rules cross the Platform/Gateway/game boundary and therefore require an explicit contract. They must not be implemented through unrestricted cross-database coupling or by making Platform the owner of gameplay character state.

Exact responsibility for admission artifacts, account-level exclusion lease, character lease, fencing generations and pending-takeover reconciliation remains `FND-04` work.

No write to `blakinio/Oteryn-Platform` is authorized by this baseline.

## Security consequence

A valid AccountId, CharacterId or GameSessionId is not sufficient authority by itself.

Production admission must validate current account ownership and current session/lease/fencing state so that stale sessions cannot keep playing, a second CharacterId cannot bypass the account exclusion rule, world/channel/instance changes cannot bypass it, and replayed identifiers cannot acquire gameplay authority.

## Required application to later contracts

This baseline is mandatory input to:

- completion of the `FND-ID-01` ownership/concurrency catalogue;
- `FND-04` Game Session/admission/character-lease contract;
- `FND-02` fields that carry session/admission identity after semantics are fixed;
- `DUR-02` persistence/fencing model where durable lease/session state is required;
- combat/logout lifecycle rules;
- QA/E2E duplicate-login, combat-X-log and reconnect scenarios;
- Platform/Game Gateway reconciliation where AccountId participates in admission.

## Programme effect

Accepted now:

```text
one AccountId = maximum one authoritative online CharacterId
new valid login supersedes the old client session
combat/logout lock prevents instant removal and prevents admission of a different character until legal resolution
```

Still unresolved:

- same-CharacterId takeover/reattach behavior during combat;
- exact GameSessionId representation and issuer;
- AdmissionId representation and issuer;
- CharacterLeaseId representation and issuer;
- account-level versus character-level lease physical layout;
- lease location, TTL, renewal and revocation;
- session-generation increment rules;
- reconnect grace period;
- exact disconnected-character combat behavior;
- exact Gateway/game transaction and failure state machine;
- exact UX while a different-character takeover is pending.

No runtime, protocol, persistence, database or Platform implementation is authorized by this decision alone.
