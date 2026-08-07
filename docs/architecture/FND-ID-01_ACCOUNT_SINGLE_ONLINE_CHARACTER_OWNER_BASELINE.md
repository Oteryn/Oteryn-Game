# FND-ID-01 Account Single Online Character Owner Baseline

- Status: Owner-accepted pre-contract baseline
- Date: 2026-08-07
- Decision owner: Oteryn project owner
- Gate: `FND-ID-01` input; mandatory consumer: `FND-04`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: account/character concurrency, Game Session admission, duplicate login, reconnect, character lease and stale-session fencing

## Purpose

Record the project owner's accepted gameplay-presence invariant before `FND-04` freezes the exact Game Session, admission and character-lease mechanism.

This document fixes the concurrency rule only. It does not select token format, lease storage, timeout values, duplicate-login UX, transport behavior, database schema or Platform implementation.

## Owner-accepted invariant

For one Platform-owned `AccountId`:

```text
count(authoritative_online_characters(AccountId)) <= 1
```

Equivalently:

```text
AccountId A
    -> CharacterId C1 online authoritatively
    -> CharacterId C2 must not also be online authoritatively
```

The rule is global to the account. It is not reset by changing:

- `WorldId`;
- `ChannelId`;
- `InstanceId`;
- GameNode process;
- client device;
- network connection;
- geographic region;
- character name.

Therefore an account cannot legitimately have one character online on World A while another character from the same account is online on World B.

## Scope of "online"

For this invariant, **online** means holding current authoritative gameplay permission for a character under the accepted session/lease/fencing model.

The following do not by themselves count as a second online gameplay character:

- an authenticated Platform WWW/OAuth session;
- account-management browsing;
- a stale TCP/QUIC transport that has already lost authoritative gameplay rights;
- a reconnect candidate that has not yet acquired current authority;
- a login-screen or character-selection client without an admitted authoritative character session.

A temporary transport overlap during reconnect or handoff is acceptable only if `FND-04` proves that no more than one character session/lease for the `AccountId` can mutate authoritative gameplay state.

## Relationship to CharacterId

`CharacterId` remains durable semantic character identity and is independent of online-session authority.

This rule does not alter:

- CharacterId UUIDv7 representation;
- global CharacterId uniqueness;
- rename semantics;
- world-transfer identity preservation;
- deletion/non-reuse rules;
- AccountId -> CharacterId ownership linkage.

It constrains only concurrent authoritative gameplay presence.

## Required authority invariant

Later contracts must preserve an account-level exclusion rule conceptually equivalent to:

```text
AccountId A
    -> at most one current authoritative
       (CharacterId, GameSessionId, session_generation, character lease/fence)
```

The exact tuple is illustrative. `FND-04` owns the final `GameSessionId`, admission, lease and generation contract and may choose a different physical representation while preserving the invariant.

Character-level fencing alone is insufficient if it allows two different CharacterIds belonging to the same AccountId to hold independent current gameplay authority simultaneously.

## Character switching

Switching from Character A to Character B under the same account must not create an interval in which both are authoritative gameplay writers.

The final transition may be implemented as, for example:

- release old authority then admit the new character;
- an atomic fenced handoff;
- another mechanism proven equivalent.

No mechanism is accepted yet. The only fixed property is non-overlap of authoritative gameplay character presence.

## Duplicate login and reconnect policy remains unresolved

This decision does **not** choose what the player sees when a second login attempt occurs.

`FND-04` must decide whether the canonical behavior is:

- reject the newcomer while the existing character remains authoritative;
- revoke/fence the existing character and transfer authority to the newcomer;
- offer an explicit user-confirmed takeover;
- use another bounded policy.

Whichever policy is selected, stale sessions must fail closed and two authoritative characters for the same AccountId must never coexist.

## Failure and race requirements

`FND-04` and persistence/runtime contracts must later prove the invariant under at least:

- simultaneous login attempts from two devices;
- two different CharacterIds selected nearly simultaneously;
- reconnect racing with a fresh login;
- channel switch racing with duplicate login;
- world transfer/account state transition racing with admission;
- Game Gateway retry/replay;
- GameNode crash during handoff;
- lease timeout and delayed network packets;
- stale session-generation messages;
- Platform/Gateway/game-database partial failure.

The safe failure direction is toward **zero or one** authoritative gameplay character, never two.

## Cross-repository ownership consequence

`AccountId` remains owned by Oteryn Platform Identity and CharacterId remains game-domain identity.

The single-online-character rule therefore crosses the Platform/Gateway/game boundary and must be represented by an explicit contract. It must not be implemented through unrestricted cross-database coupling or by allowing Platform to become the owner of gameplay character state.

Exact responsibility for issuing admission artifacts, acquiring/releasing the account-level exclusion lease and reconciling stale ownership remains `FND-04` work.

No write to `blakinio/Oteryn-Platform` is authorized by this baseline.

## Security consequence

A valid AccountId, CharacterId or GameSessionId is not sufficient authority by itself.

Production admission must validate current account ownership and current session/lease/fencing state so that:

- stale sessions cannot keep playing after authority moves;
- a second CharacterId cannot bypass the account-level exclusion rule;
- changing world/channel/instance cannot bypass the invariant;
- guessing or replaying identifiers cannot acquire gameplay authority.

## Required application to later contracts

This baseline is mandatory input to:

- completion of the `FND-ID-01` ownership/concurrency catalogue;
- `FND-04` Game Session/admission/character-lease contract;
- `FND-02` fields that carry session/admission identity after semantics are fixed;
- `DUR-02` persistence/fencing model where durable lease or session state is required;
- QA/E2E duplicate-login and reconnect scenarios;
- Platform/Game Gateway reconciliation where account identity participates in admission.

## Programme effect

Accepted now:

```text
one AccountId = maximum one authoritative online CharacterId
```

Still unresolved:

- exact duplicate-login UX/policy;
- newcomer-wins versus incumbent-wins behavior;
- takeover confirmation;
- GameSessionId representation and issuer;
- AdmissionId representation and issuer;
- CharacterLeaseId representation and issuer;
- lease location, TTL, renewal and revocation;
- session-generation increment rules;
- reconnect grace period;
- exact Gateway/game transaction and failure state machine.

No runtime, protocol, persistence, database or Platform implementation is authorized by this decision alone.
