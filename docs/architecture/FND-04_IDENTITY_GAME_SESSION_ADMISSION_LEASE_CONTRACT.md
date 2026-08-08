# FND-04 — Identity, Game Session, Admission and Character Lease Contract

- Status: Candidate architecture contract; canonical when merged to `main`
- Date: 2026-08-08
- Gate: `FND-04`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Repository: `blakinio/Oteryn-v2`
- Consumes:
  - `FND-04_SESSION_ADMISSION_LEASE_ANALYSIS_BASELINE.md`
  - `FND-04_PLATFORM_PRE_ADMISSION_RECONCILIATION_REFINEMENT.md`
  - `FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md`
  - `FND-ID-01_GAME_SESSION_ID_OWNER_ISSUER_BASELINE.md`
  - `FND-ID-01_GAME_SESSION_RECONNECT_GENERATION_OWNER_BASELINE.md`
  - `FND-ID-01_ACCOUNT_SINGLE_ONLINE_CHARACTER_OWNER_BASELINE.md`
  - `FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md`
  - `FND-03_RUNTIME_EXECUTION_CONTRACT.md`
  - `DISCONNECT_REENTRY_PVE_PROTECTION_OWNER_DECISION.md`
  - `FOUNDATION_ERROR_VOCABULARY.md`
  - `FOUNDATION_FAILURE_SCENARIOS.md`
- Current external reconciliation pin: `blakinio/Oteryn-Platform@8e2514b8721d385b626ead7ffa47fc74067b0a0b`
- External contract blobs consumed read-only:
  - pre-admission handoff: `a7a98b943c528b9f21c0cdc2ee90b308045706f8`
  - runtime-status projection: `5e45a4318716b62d53fd8bdf67b3b55676286ad1`
- Does not authorize: Rust/runtime/protocol-schema/persistence implementation, Platform writes, production keys, production routes/traffic, deployment or live account/session mutation

## 1. Purpose

FND-04 freezes the minimum semantic authority model required for native gameplay admission without collapsing authentication, admission, session identity, account exclusion, character writer fencing, concrete transport control and runtime scope ownership into one generic session object.

The canonical separation is:

```text
Platform authentication/security authority
    != Platform bounded gameplay authorization
    != AccountPresenceClaim
    != CharacterLease
    != GameSessionId / GameSession lifecycle
    != TransportBinding / connection_generation
    != ChannelRuntime / InstanceRuntime scope ownership generation
```

These layers may initially be co-located in one Rust deployment. Co-location does not merge their semantics or owners.

## 2. Decision timing

### Must decide now? — YES

This contract blocks production implementation of:

- native fresh gameplay admission and canonical `GameSessionId` creation;
- account-global one-online-character enforcement;
- CharacterLease acquisition/fencing;
- reconnect/rebind and stale-transport rejection;
- duplicate-login/takeover behavior;
- post-grace same-character recovery;
- Channel/Instance session continuity;
- native admission/recovery failure mapping;
- the first complete Platform -> Gateway -> Oteryn-v2 gameplay-entry slice.

### What becomes expensive if wrong

An incorrect boundary could create dual playable characters, replayed grants, stale character writers, stale transport control, combat-escape or forced-disconnect abuse, route-owner resurrection after failover, ambiguous crash recovery or user-visible success before authoritative session state exists.

Correcting those errors after deployed clients and durable data exist would require coordinated protocol, persistence, Platform and runtime migration.

### Evidence that may justify supersession

A later accepted contract may change a specific choice only with named evidence such as penetration testing, replay findings, cross-language interoperability failure, measured security-projection availability/latency, lease/fencing fault injection, reconnect-storm false-positive evidence, player abuse telemetry or a new requirement for sender-constrained/device-bound credentials.

### Deliberately not decided here

This contract does not select:

- Rust/Go cryptographic libraries;
- KMS/HSM/cloud-vendor products;
- PostgreSQL table/index layout or transaction isolation level;
- Redis topology or any Redis authority role;
- exact grant TTL, clock-skew, key-overlap/cache values;
- exact reconnect-proof primitive/length;
- exact liveness probe cadence/hysteresis;
- exact CharacterLease TTL/renewal/safety margin;
- exact admission/recovery/takeover rate limits;
- deployment/service extraction topology;
- final player-facing wording.

Those details require the explicit preimplementation gates in section 31.

## 3. Canonical authority layers

### 3.1 Platform Identity/security

Oteryn Platform remains authoritative for reusable account credentials, OAuth/PKCE, MFA, recovery, Platform sessions, Game Login Ticket issuance and Platform-owned account-security state.

The Rust game server never becomes a second password/OAuth/MFA authority.

### 3.2 Platform bounded gameplay authorization

Platform/Gateway may authorize a bounded gameplay attempt using current Platform policy and applicable Oteryn-v2 evidence.

That authorization is never proof that gameplay admission or recovery succeeded, and it is never a `GameSessionId`, CharacterLease or current gameplay-owner claim.

### 3.3 `AccountPresenceClaim`

`AccountPresenceClaim` is the game-domain account-global exclusion state:

```text
scope: AccountId
value: CharacterId or none
purpose: at most one authoritative playable or mandatory-presence character per account
```

Binding invariant:

```text
count(authoritative_playable_or_mandatory_presence_characters(AccountId)) <= 1
```

The claim remains held while the actor is `PRESENT_CONTROLLED` or `PRESENT_UNCONTROLLED`.

It is not released merely because a socket closes, `connection_generation` changes, reconnect grace expires, a GameSession becomes terminal, the client process disappears or a lease renewal becomes uncertain while an actor may still exist.

Release requires an authoritative lifecycle transition proving the actor is legally absent or that a legal replacement transition completed without dual authority.

### 3.4 `CharacterLease`

`CharacterLease` is the game-domain exclusive character writer fence:

```text
scope: CharacterId
binds: AccountId + character_lease_generation + current authoritative actor/runtime context
purpose: reject stale player/runtime/durable writers after authority changes
```

Character Authority remains owner of CharacterId, character lifecycle and authoritative AccountId<->CharacterId ownership.

Game Session / Admission coordinates player-control transitions. Runtime and persistence enforce the current CharacterLease fence. DUR-02 owns the physical persistence/transaction mechanism.

### 3.5 `GameSession`

`GameSessionId` identifies one logical admitted player-control lifecycle. The game-domain Game Session / Admission authority creates it only at the successful admission linearization point in section 9.

`GameSessionId` is identity, never a credential.

### 3.6 `TransportBinding`

`TransportBinding` is the current concrete control binding for one GameSession:

```text
scope: GameSessionId
fence: connection_generation
purpose: exactly one current concrete transport may command, advance liveness or mutate reconciliation authority
```

FND-02 remains authoritative for the visible `uint64 connection_generation` contract.

### 3.7 `RuntimeScopeAuthority`

FND-03 remains authoritative for ChannelRuntime/InstanceRuntime scope ownership generation.

A valid session/lease cannot authorize a stale runtime owner. A valid runtime owner cannot create a GameSession/account presence/character lease by itself.

## 4. Actor-presence states

The game-domain lifecycle must distinguish at least:

```text
ABSENT
PRESENT_CONTROLLED
PRESENT_UNCONTROLLED
```

- `PRESENT_CONTROLLED`: an admitted current GameSession/TransportBinding has playable control of the authoritative actor.
- `PRESENT_UNCONTROLLED`: the actor remains in authoritative world simulation but no current playable controller is attached; combat/PZ/logout/lifecycle rules may require this state.
- `ABSENT`: authoritative lifecycle has legally removed the actor from mandatory world presence.

Exact enum/storage representation is deferred.

## 5. Fencing values

### 5.1 Account-presence generation

Every current AccountPresenceClaim uses a stale-safe non-zero monotonic generation/revision or equivalent linearizable CAS fence scoped to AccountId.

It must prevent stale admission/takeover work from replacing newer state, preserve continuous same-character presence across eligible reconnect/session replacement/handoff, never wrap/reuse, and fail closed on representational exhaustion.

It is a fence, not a foundation entity ID.

### 5.2 `character_lease_generation`

Every current CharacterLease uses a non-zero monotonic generation or equivalent stale-safe character-writer fence scoped to CharacterId.

Required semantics:

- older-generation player/runtime/durable writes fail closed;
- transport reconnect alone does not advance it;
- replacement/recovery of authoritative character-writer context advances or otherwise establishes a strictly newer equivalent fence before replacement authority may commit;
- a runtime handoff must make the source writer unable to commit before destination authority becomes externally final;
- no wrap/reuse; exhaustion fails closed.

A `uint64`-class monotonic representation is the initial direction unless DUR-02 evidence proves an equivalent representation is safer.

### 5.3 `connection_generation`

FND-02 semantics are binding:

- `0` is bootstrap/pre-admission or resume negotiation only;
- fresh admission starts at `1`;
- every committed same-GameSession rebind establishes exactly one strictly newer non-zero value;
- rejected/uncommitted attempts do not advance authority;
- old generations cannot command, advance liveness or mutate reconciliation state;
- no wrap/reuse; exhaustion makes that GameSession terminal.

## 6. No new `AdmissionId` or `CharacterLeaseId`

FND-04 does not add either identifier to the foundation catalogue.

Use:

- `AdmissionAttemptRef` — bounded Platform producer operation/correlation reference for one fresh-entry issuance lifecycle;
- `GrantNonce` — one concrete fresh-entry grant replay/consume key;
- `RecoveryAttemptRef` — bounded Platform producer operation/correlation reference for a reauthenticated recovery authorization lifecycle;
- `GameSessionId` — admitted logical gameplay-session identity;
- scoped account/lease/connection/runtime generations — fences;
- accepted `HandoffId` — ownership-transition identity.

`AdmissionAttemptRef` and `RecoveryAttemptRef` are operation-scoped references, not foundation entity IDs. A later FND-ID amendment requires evidence that a separately addressable durable entity is necessary.

## 7. Fresh-entry `PreAdmissionGrant` — accepted

FND-04 accepts this credential class for **fresh entry only**:

```text
short-lived signed Platform PreAdmissionGrant
+ explicit versioned cross-language security/interchange profile
+ authoritative game-domain one-time GrantNonce consumption
+ current game-domain revalidation before admission commit
```

The design is hybrid: issuer authenticity/bindings are signed, while one-successful-admission replay safety is authoritative game-domain consume state.

Normal valid fresh-entry verification should not require synchronous Platform introspection; post-issuance security freshness follows section 12.

### 7.1 `FRESH_ENTRY` is not recovery

The `PreAdmissionGrant` credential type/purpose is `FRESH_ENTRY` (or exact profile equivalent).

It authorizes one bounded attempt against one selected fresh-entry World/Channel route. It may not be reinterpreted as reconnect, same-character recovery, Channel retargeting, GameSession resurrection or takeover proof.

This preserves the current Platform pre-admission contract: ordinary native pre-admission material is not a post-admission reconnect credential.

## 8. Distinct reauthenticated `RecoveryAuthorizationGrant`

FND-04 also accepts a **separate future Platform authorization credential type** for reauthenticated same-character control recovery:

```text
RecoveryAuthorizationGrant
purpose: RECOVER_EXISTING_CONTROL
```

It is not a `PreAdmissionGrant`, Game Login Ticket, reconnect proof, GameSessionId or lease credential.

The exact signed container may reuse the same cross-language security-profile family only if the profile enforces non-confusable credential type, purpose, audience and validation rules. A validator for one credential type must never accept the other by ignoring or defaulting a type/purpose field.

Platform producer implementation for this recovery credential requires a separately authorized coordinated Platform task. Until that producer/profile exists, reauthenticated recovery is architecturally defined but not implementation-ready.

## 9. Fresh-admission linearization point

Fresh admission is one externally unambiguous authority transition.

Conceptually:

```text
validate bounded FRESH_ENTRY material/profile
validate signature/key purpose/issuer/audience/time
validate Platform account-security freshness applicability
validate WorldId/ChannelId/route/runtime observation/owner generation/revisions
revalidate authoritative AccountId -> CharacterId ownership/lifecycle
validate account-global presence / duplicate-login state
validate target runtime current and admission-capable
prepare AccountPresenceClaim transition
prepare CharacterLease/current writer fence
prepare candidate GameSessionId
prepare connection_generation = 1
prepare reconnect-proof verifier state
atomically commit:
    GrantNonce one-time consumption
    account-presence claim/fence
    character lease/fence
    canonical GameSession
    first TransportBinding
    reconnect-proof state
publish success only after commit
establish FND-02 initial snapshot/reconciliation boundary
```

A candidate GameSessionId is not canonical until commit. An uncommitted candidate is discarded and never reused.

No user-visible success may precede the authoritative commit. If required durable/fencing authority cannot be proven, admission fails/holds closed rather than publishing an unfenced in-memory success.

## 10. `AdmissionAttemptRef` and `GrantNonce`

### 10.1 `AdmissionAttemptRef`

Platform owns the bounded producer operation lifecycle.

It exists to correlate one logical fresh-entry issuance attempt, distinguish retry from independent login, reconcile ambiguous issuer outcomes and provide bounded audit correlation.

For one logical attempt, Platform must recover the same committed outcome or deterministically retire/reconcile the attempt. It must not mint multiple independently usable grants because a response was lost.

### 10.2 `GrantNonce`

`GrantNonce` identifies one concrete issued PreAdmissionGrant for game-domain consume/replay state.

It must be cryptographically random under the accepted profile, bound inside authenticated grant material, unique within the profile's required issuance/retention domain, consumed at most once at admission commit, and never become valid again after consumed/expired/wrong-bound rejection.

Two distinct concrete grants must not intentionally share one GrantNonce.

A lost game-admission response does not make GrantNonce reusable. Recovery uses game session/recovery state.

## 11. Fresh-entry grant mandatory bindings

A `FRESH_ENTRY` PreAdmissionGrant must authenticate equivalents of:

- explicit credential type and purpose;
- security/profile revision;
- issuer identity and exact admission audience;
- AdmissionAttemptRef;
- GrantNonce;
- canonical AccountId, CharacterId, WorldId and ChannelId;
- Platform account-security generation/revision or equivalent security-applicability evidence required by the selected profile;
- route/offer/topology/admission-target revision;
- runtime observation/source revision;
- issuance-time runtime scope ownership generation where required for stale-owner prevention;
- protocol/transport profile;
- required content/ruleset/runtime compatibility revisions;
- server-authoritative issuance/not-before/expiry semantics;
- signing-key identifier/version.

Additional fields require explicit owner/purpose/validation semantics.

The credential must not contain reusable Platform credentials, passwords, OAuth access/refresh tokens, signing secrets, arbitrary verifier/JWK URLs or client-selected issuer/audience authority.

## 12. Post-issuance Platform security freshness — accepted

FND-04 accepts:

```text
short-lived signed fresh-entry grant
+ Platform-owned account-security generation/revision bound at issuance
+ authenticated game-side bounded-staleness security generation/revocation projection
+ fail closed for new admission when required security freshness cannot be proven
+ exceptional online introspection only as an explicitly accepted fallback/reconciliation path
```

Exact projection transport/storage is deferred.

If authoritative Platform account-security state advances in a way that invalidates the issued revision before final admission, the unconsumed grant is rejected even if signature and nominal expiry remain valid.

If the selected profile requires a current-enough security projection and evidence is stale, unavailable, contradictory or invalid, new admission fails closed.

A concrete maximum freshness/risk window is mandatory before implementation through section 31.

### 12.1 Post-admission boundary

This rule does not grant Platform authority to delete actors, release CharacterLease, terminate GameSession or force a combat-locked client offline after admission.

Emergency post-admission session revocation/control requires a separately accepted game-domain control/fencing contract.

## 13. Runtime observation/ownership-generation applicability

Platform may issue fresh-entry material only from applicable current-owner Oteryn-v2 runtime evidence. Final game admission independently revalidates current authoritative runtime applicability.

Default rule:

```text
issuance-time target ownership generation != current authoritative target ownership generation
-> reject as stale generation
-> require fresh Platform routing + fresh PreAdmissionGrant
```

The same applies to an obsolete route/topology/admission-target revision when that change invalidates original applicability.

A grant is never silently retargeted to another Channel, GameNode, runtime owner or downgraded route. Relaxation requires explicit proof of an equivalent generation-independent fence.

## 14. Character ownership/lifecycle revalidation

Platform projection may prevent knowingly unsafe issuance, but final Oteryn-v2 admission revalidates current Character Authority facts, including current AccountId<->CharacterId ownership, deletion/transfer/sale/lock state, World applicability, applicable game-owned sanction/eligibility state, and current account-presence/lease state.

A stale Platform/browser/client claim never overrides newer game-domain authority.

## 15. GameSession lifecycle

The minimum logical lifecycle is:

```text
NO_SESSION
    -> ADMISSION_COMMIT -> ACTIVE

ACTIVE
    -> expected current-generation control evidence becomes insufficient
    -> CONTROL_SUSPECT

CONTROL_SUSPECT
    -> sufficient current-generation evidence resumes before loss declaration -> ACTIVE
    -> control_loss_declared_at reached -> CONTROL_LOST_GRACE
    -> an independently proven legitimate rebind commits before loss declaration -> ACTIVE

CONTROL_LOST_GRACE
    -> eligible same-session rebind/recovery before grace expiry -> ACTIVE
    -> grace expires -> TERMINAL

ACTIVE / CONTROL_SUSPECT / CONTROL_LOST_GRACE
    -> legal terminal transition (logout/replacement/revocation as allowed)
    -> TERMINAL
```

`CONTROL_SUSPECT` prevents the first missing/delayed evidence sample from being mislabeled as declared control loss. Ordinary gameplay behavior remains in effect until the accepted loss boundary is reached unless another authoritative rule independently changes it.

`ADMISSION_COMMIT` is a transition, not a durable authority state. `TERMINAL` is irreversible for that GameSessionId.

## 16. Server-authoritative liveness/timing

FND-04 determines what current-generation evidence is sufficient for playable control. FND-03 timestamps accepted evidence with the process-local monotonic clock.

Binding composition:

```text
last_sufficient_control_at = T0
control_loss_declared_at   = T0 + 2.0 s
stale_transport_cleanup    = T0 + 5.0 s
same_session_grace_expires = control_loss_declared_at + 15.0 s
```

Thus a GameSession gets a full 15-second same-session recovery grace after server-authoritative control loss is declared.

The 5-second concrete-transport cleanup is independent and does not release GameSession-independent actor/account/lease authority.

Client timestamps, browser state, OS event logs or Launcher/Guardian evidence cannot move these authoritative timers.

### 16.1 Rebind before loss declaration

A legitimate replacement transport may commit while the session is `ACTIVE` or `CONTROL_SUSPECT` if the old binding is otherwise proven stale/lost. It may preserve GameSessionId and advance connection_generation.

It does not receive the four-second defensive PvE re-entry effect merely because transport/generation changed.

### 16.2 Defensive PvE re-entry effect

The accepted four-second PvE defensive effect is granted at most once per eligible server-classified unexpected playable-control-loss episode.

Routine rebind, graceful reconnect, intentional takeover or connection-generation rotation does not manufacture eligibility.

FND-04 therefore requires an actor/session-scoped loss-episode state so retries/rebinds in one episode cannot repeatedly re-arm protection. A new episode may begin only after the prior episode reaches an accepted recovered-stable boundary. The numeric anti-flap/hysteresis threshold is a section-31 parameter gate.

## 17. Primary same-GameSession reconnect

Primary reconnect uses a game-domain-issued high-entropy opaque rotating reconnect proof, never GameSessionId and never a Platform reusable credential.

Required properties:

- bound to exactly one current GameSession and reconnect-proof generation/state;
- transported only through accepted TLS;
- stored server-side as verifier/digest or equivalent secret-safe representation where practical;
- raw proof never logged/traced/exported to analytics;
- successful rebind rotates proof state;
- predecessor cannot authorize an unrelated/later rebind;
- replay fails closed without changing current authority;
- terminal GameSession invalidates reconnect authority.

### 17.1 Rebind linearization

A same-session rebind succeeds only when GameSession remains non-terminal, AccountId/CharacterId binding remains valid, AccountPresenceClaim and CharacterLease remain compatible, prior transport is replaceable under current liveness state, current runtime placement/revisions are valid and reconnect proof is valid.

One atomic rebind then:

```text
commits exactly one newer connection_generation
rotates reconnect-proof state
fences every older transport generation
preserves GameSessionId
preserves actor/gameplay state
publishes success only after commit
```

TLS establishment alone is not gameplay authority.

### 17.2 Concurrent reconnect race

Two contenders cannot both win. The first linearized successful rebind becomes current. Later contenders observe stale proof/generation/current-control state and fail without fencing the winner.

## 18. Lost rebind response — accepted recovery rule

FND-04 rejects rotate-and-forget and rejects making a predecessor reconnect proof generally valid again after a committed rotation.

If the server commits a newer connection generation/reconnect-proof state but the response is lost, the client cannot infer failure and cannot use the predecessor as a fresh rebind credential.

Recovery uses the distinct Platform-reauthenticated `RecoveryAuthorizationGrant` path in section 19. This deliberately trades a rare fallback Platform dependency for a simpler replay boundary without broadly reusable predecessor authority.

The game side reconciles current GameSession/actor state; it never assumes the prior rebind failed merely because the client did not receive its response.

## 19. Reauthenticated same-character recovery — accepted

A future `RecoveryAuthorizationGrant` supports cases such as lost reconnect material or lost rebind response.

### 19.1 Required recovery bindings

The profile must bind at least:

- credential type/purpose `RECOVER_EXISTING_CONTROL`;
- security/profile revision;
- issuer/audience;
- RecoveryAttemptRef;
- canonical AccountId, CharacterId and WorldId;
- current Platform account-security applicability evidence;
- issuance/not-before/expiry/key revision.

It must **not** make client/Platform ChannelId or InstanceId authoritative for current actor placement.

The game domain resolves current authoritative actor placement and current session/lease/runtime state.

### 19.2 Healthy incumbent protection

A recovery authorization cannot preempt a healthy current controller merely because a second client reauthenticated.

If a current healthy controller exists, recovery fails as conflict and incumbent authority remains unchanged.

### 19.3 Inside same-session grace

If the existing GameSession is in `CONTROL_LOST_GRACE`, the actor is the same CharacterId, ownership/presence/lease/runtime state are current and no healthy controller exists, the recovery path may preserve the same GameSessionId and atomically establish a newer connection_generation with new reconnect-proof state.

### 19.4 Current-placement route resolution

Recovery never trusts stale fresh-entry route data as actor-placement authority.

Implementation requires a bounded authenticated game-domain resolver or authorized read-only route projection that can direct recovery to the current authoritative owner without making Platform the GameSession owner.

Cross-placement recovery cannot be claimed implementation-ready until this resolver path exists and is tested.

## 20. Post-grace same-character control recovery — accepted

When same-session grace expires while combat/PZ/logout/lifecycle rules still require the actor in world:

```text
old GameSession          -> TERMINAL
actor                    -> PRESENT_UNCONTROLLED
AccountPresenceClaim     -> remains held for the same CharacterId
CharacterLease/runtime   -> remains current through the authoritative actor path
```

A valid RecoveryAuthorizationGrant may create a **fresh GameSessionId** and attach control to that exact existing actor only after proving:

- old GameSession terminality;
- no current playable controller;
- current AccountId still owns CharacterId;
- account presence still names the same CharacterId;
- current CharacterLease/current writer fence valid;
- current actor placement from game-domain authority;
- compatible protocol/content/ruleset/runtime revisions;
- current security-freshness policy.

The new GameSession starts `connection_generation = 1` and receives fresh reconnect-proof state.

This transition must not respawn, teleport, duplicate, heal/refill, clear conditions/cooldowns/combat/PZ/logout state, reset threat/encounter state or roll back committed effects.

If the original unexpected-loss episode is still eligible and its four-second defensive effect was not already consumed, the one-shot effect may apply to restored control. A new GameSession cannot create a second protection entitlement for the same loss episode.

A different CharacterId remains blocked while incumbent mandatory presence exists.

## 21. Duplicate login and intentional takeover

### 21.1 Account-global exclusion

Every fresh admission evaluates AccountPresenceClaim. Two different CharacterIds of one AccountId cannot both become authoritative across any worlds/channels/instances/nodes.

### 21.2 Healthy combat/PZ/logout-locked incumbent

A second authenticated client cannot fence, close, revoke or steal a healthy incumbent session solely by presenting valid credentials.

If incumbent actor has a combat/PZ/logout blocker and playable control remains healthy, newcomer is denied/held as conflict and incumbent remains authoritative.

### 21.3 Logout-eligible incumbent

A fully authenticated newcomer may intentionally replace a logout-eligible incumbent through a fenced legal transition:

1. authenticate/authorize newcomer;
2. prove takeover eligibility;
3. end/revoke old gameplay control at the legal boundary;
4. complete old actor logout/removal or accepted same-character transition;
5. only then establish replacement gameplay authority.

There may never be two player-controlled authoritative characters. A new logical GameSession receives a new GameSessionId.

### 21.4 Genuinely unavailable incumbent

Genuine control loss uses same-character recovery rules, not hostile newcomer preemption. A different CharacterId remains blocked while old actor is mandatory-present.

## 22. CharacterLease ownership and durable authority

### 22.1 Logical owner

CharacterLease belongs to the Oteryn-v2 game domain. Platform/Gateway/client never owns or issues it.

Game Session / Admission coordinates acquisition/replacement for player-control transitions. Character Authority owns character identity/lifecycle/ownership. Runtime and persistence enforce the current fence.

### 22.2 Durable authority boundary

Authoritative account-presence/lease/session fencing state must be represented in game-owned authoritative durable/recoverable state under the accepted `oteryn_game` persistence boundary wherever correctness must survive process/node failure.

PostgreSQL is the accepted relational target. Redis/cache may accelerate non-authoritative access but cannot be the sole source of lease/account-presence/session fencing authority.

DUR-02 owns physical schema, indexes, locking/isolation, transaction layout, retention and recovery encoding. An FND-04 implementation is not acceptable if DUR-02 cannot prove the required linearizable/fenced behavior.

## 23. CharacterLease acquisition, renewal, uncertainty and release

### 23.1 Acquisition

A lease is established only inside an authoritative admission, recovery or handoff transition that also proves account presence, current character ownership and current runtime writer applicability.

It is never granted merely because an old lease appears expired on one participant's local clock.

### 23.2 Renewal

Lease renewal belongs to the current authoritative **actor/runtime writer lifecycle**, not to socket or GameSession existence alone.

A current runtime writer may renew only the current character lease generation while:

- AccountPresenceClaim still names this CharacterId;
- authoritative actor lifecycle still requires this character writer;
- current runtime scope ownership generation remains valid;
- current CharacterLease generation remains valid;
- required durable dependency/fencing evidence remains sufficient.

An ACTIVE GameSession is additionally required for **player-originated control/mutation authority**, but it is not required merely to keep the lease for a `PRESENT_UNCONTROLLED` actor that must continue server-driven simulation.

Thus a terminal GameSession does not automatically terminate CharacterLease while mandatory actor presence remains.

Exact TTL/renewal/safety-margin values are section-31 implementation parameters; margins must guarantee the old writer ceases durable authority before a newer generation can commit.

### 23.3 Renewal uncertainty / expiry

Renewal timeout, dependency uncertainty or observed expiry does not self-grant replacement authority.

On uncertainty:

- new player-originated durable mutation fails closed;
- stale-generation durable writes are prohibited;
- AccountPresenceClaim is not released while actor may exist;
- no newer character-writer authority commits until old writer/runtime is safely fenced/recovered;
- server-driven in-memory simulation may continue only under independently current FND-03 runtime-scope authority and only where it cannot race a replacement durable writer.

### 23.4 Replacement

A replacement character-writer fence may commit only after the transition proves the previous writer can no longer commit authoritative writes under the accepted DUR/OPS fencing mechanism.

Waiting for cooperative shutdown/ack from a stale holder is not required once durable fencing is proven.

### 23.5 Release

CharacterLease release requires an authoritative lifecycle boundary proving no actor/runtime path still requires that character writer fence.

Socket close, GameSession terminality, reconnect-grace expiry or client logout request alone is insufficient.

A legal handoff normally replaces/rebinds the character writer fence while continuous actor presence is preserved.

## 24. Same-GameSession recovery across GameNode replacement

Same-GameSession recovery across process/GameNode replacement is allowed only if the new authoritative path preserves/reconstructs all state needed to prevent replay/order contradiction, including:

- GameSessionId and lifecycle state;
- current connection_generation and recovery/reconnect-proof state;
- FND-02 `next_command_id` high-water;
- pending command identities/outcomes needed to prevent duplicate execution;
- server-sequence/snapshot reconciliation boundary;
- AccountPresenceClaim/current fence;
- CharacterLease/current fence;
- authoritative actor state/current placement;
- current runtime ownership generation/revision applicability.

If any required state cannot be reconstructed safely, old GameSession becomes terminal. Actor/account presence may remain, and recovery uses section 20 when safe.

Runtime convenience may not weaken no-double-execution, stale-generation or account-global exclusion guarantees.

## 25. Channel/Instance handoff continuity

FND-04 consumes accepted HandoffId and FND-03 ownership-transition semantics.

For each handoff:

- one HandoffId identifies one transition lifecycle;
- AccountId, CharacterId, GameSessionId, account-presence fence, character-writer fence and source/destination runtime generations remain explicit bindings;
- prepare work may overlap, but only one runtime owner may accept authoritative player mutation;
- destination writer fence is established before source writer authority is released;
- stale/replayed HandoffId/generation fails closed;
- failure before commit preserves/reconciles source authority;
- failure after commit recovers from destination authority evidence.

### 25.1 Channel -> Instance and Instance -> Channel

When the move is one continuous logical player-control session, GameSessionId is preserved and AccountPresenceClaim remains continuously held.

The character-writer fence must advance/rebind so the source runtime cannot commit after destination authority becomes current. The implementation may realize this through `character_lease_generation` advancement or an equivalent composite durable fence proven by DUR-02; semantic stale-source rejection is mandatory.

If concrete transport remains valid, connection_generation need not change solely because simulation scope changed. If transport rebind is required, section 17 applies.

### 25.2 Channel -> Channel

An accepted Channel -> Channel transition establishes a fresh destination logical GameSession and therefore a fresh GameSessionId.

AccountPresenceClaim remains continuously held for the same CharacterId across the transition. Destination character-writer fencing must be established before source authority is released.

A fresh destination Platform/Gateway route and fresh `FRESH_ENTRY` PreAdmissionGrant are required. Old admission material is never retargeted.

## 26. Dependency/failure behavior

### 26.1 Platform unavailable

New Platform-dependent fresh entry and reauthenticated recovery cannot invent alternate credential authority.

An already admitted healthy session and primary reconnect that can validate current game-domain state do not automatically fail merely because Platform is unreachable.

A lost-rebind-response fallback may be temporarily unable to reauthenticate while Platform is unavailable; this does not restore predecessor proof authority or release mandatory actor/account/lease state.

### 26.2 Durable authority unavailable

If required admission/account-presence/lease/session state cannot be committed/fenced authoritatively, transition success cannot be published.

Physical failure handling belongs to DUR-02; fail-open authority is forbidden.

### 26.3 Runtime owner uncertain/superseded

Admission/recovery/handoff fails or reconciles toward current authoritative owner. A stale runtime owner cannot regain authority through otherwise valid-looking credential/session state.

### 26.4 Ambiguous client outcome

Transport loss never proves admission/rebind failure. Client follows current game-domain session/recovery state rather than reusing bearer material because a response was lost.

## 27. Foundation failure-scenario dispositions

| Scenario | FND-04 status | Binding result |
|---|---|---|
| `FS-PLATFORM-UNAVAILABLE` | `PASS` | no alternate credential authority; new Platform-dependent admission/recovery fails boundedly; established gameplay is not invalidated merely by Platform outage |
| `FS-GATEWAY-AFTER-REDEEM` | `PASS` | no silent ticket reuse/downgrade; no GameSession without game-domain admission commit |
| `FS-POSTGRES-UNAVAILABLE` | `DEFERRED_BY_ACCEPTED_GATE` | DUR-02 owns physical handling; FND-04 forbids success when required durable/fenced transition cannot be proven |
| `FS-LEASE-RENEW-TIMEOUT` | `PASS` | old durable authority stops before newer writer may commit; expiry alone never grants replacement |
| `FS-DUPLICATE-LOGIN` | `PASS` | account-global exclusion; healthy protected incumbent cannot be kicked; at most one authority wins |
| `FS-ADMISSION-GRANT-REPLAY` | `PASS` | consumed/expired/replayed/wrong-bound fresh-entry grant cannot create another admission/lease/GameSession |
| `FS-RECONNECT-CREDENTIAL-REPLAY` | `PASS` | stale/predecessor/stolen reconnect proof cannot create parallel current binding or fence winner |
| `FS-STALE-GENERATION` | `PASS` | stale connection/account/lease/runtime generation cannot command/renew/reconnect/commit |
| `FS-DUPLICATE-COMMAND` | `NOT_APPLICABLE` | FND-02 remains authority once GameSession exists |
| `FS-CHANNEL-SPLIT-OWNER` | `DEFERRED_BY_ACCEPTED_GATE` | FND-03/OPS own scope fencing; FND-04 validates current target owner |
| `FS-CHANNEL-DRAIN` | `DEFERRED_BY_ACCEPTED_GATE` | no fresh admission/handoff into non-admissible destination |
| `FS-QUEUE-SATURATION` | `DEFERRED_BY_ACCEPTED_GATE` | FND-03/resource limits own runtime queues; FND-04 cannot partially grant authority |
| `FS-SLOW-CLIENT` | `DEFERRED_BY_ACCEPTED_GATE` | FND-02/FND-03 own bounded transport/resync; FND-04 owns resulting session eligibility |
| `FS-CLOCK-SKEW` | `PASS` | signed timestamps use bounded profile skew; liveness/grace uses server monotonic time |
| `FS-KEY-ROTATION` | `PASS` | allowlisted current/retiring key policy plus emergency revocation; no unknown/downgrade key acceptance |
| `FS-REVISION-MISMATCH` | `PASS` | fail closed; no implicit downgrade/mixed authority |
| `FS-SNAPSHOT-DELTA-MISMATCH` | `NOT_APPLICABLE` | FND-02/FND-03 own state replay after admission/rebind |
| `FS-DB-OUTBOX-BOUNDARY` | `DEFERRED_BY_ACCEPTED_GATE` | DUR/ANL own physical transaction/outbox; success cannot precede required authority commit |
| `FS-WORLD-BUNDLE-CORRUPT` | `NOT_APPLICABLE` | invalid world activation must already make target unroutable |
| `FS-CLIENT-CUTOVER-ROLLBACK` | `NOT_APPLICABLE` | historical migration lifecycle |

Analytics-only failure scenarios are `NOT_APPLICABLE` to FND-04 authority unless a future accepted ANL contract introduces a dependency. Analytics never mutates session/lease authority.

## 28. Stable internal failure codes

FND-04 freezes symbolic internal codes; numeric protocol/API registration belongs to the owning profile.

| FND-04 code | Foundation category | Retry class | Meaning |
|---|---|---|---|
| `ADMISSION_INPUT_INVALID` | `INVALID_INPUT` | `TERMINAL` | malformed/non-canonical bounded fresh-entry input |
| `ADMISSION_CREDENTIAL_AUTH_FAILED` | `AUTHENTICATION_FAILED` | `SECURITY_TERMINAL` | bad fresh-entry signature/proof or untrusted issuer/key purpose |
| `ADMISSION_GRANT_REJECTED` | `SESSION_REJECTED` | `TERMINAL` | expired/consumed/replayed/wrong-audience/purpose/binding fresh-entry grant |
| `ADMISSION_SECURITY_SUPERSEDED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | Platform account-security state invalidates issued grant |
| `ADMISSION_RUNTIME_STALE` | `STALE_GENERATION` | `TERMINAL` | route/runtime observation or ownership generation superseded |
| `ADMISSION_REVISION_UNSUPPORTED` | `UNSUPPORTED_REVISION` | `TERMINAL` | protocol/content/ruleset/runtime/security profile incompatible |
| `ADMISSION_OWNERSHIP_REJECTED` | `CONFLICT` | `TERMINAL` | authoritative AccountId/CharacterId lifecycle/ownership no longer permits attempt |
| `ADMISSION_ACCOUNT_PRESENCE_CONFLICT` | `CONFLICT` | `RETRYABLE` | current mandatory character presence blocks fresh admission |
| `ADMISSION_DEPENDENCY_UNAVAILABLE` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | required security/runtime/durable evidence cannot be proven safely |
| `RECONNECT_PROOF_REJECTED` | `AUTHENTICATION_FAILED` | `SECURITY_TERMINAL` | opaque reconnect proof invalid/replayed/stale for requested rebind |
| `RECONNECT_SESSION_TERMINAL` | `SESSION_REJECTED` | `TERMINAL` | old GameSession cannot be resumed |
| `RECONNECT_BINDING_STALE` | `STALE_GENERATION` | `TERMINAL` | contender references older connection/session/runtime fence |
| `RECOVERY_CREDENTIAL_AUTH_FAILED` | `AUTHENTICATION_FAILED` | `SECURITY_TERMINAL` | RecoveryAuthorizationGrant invalid/untrusted/wrong-purpose |
| `RECOVERY_HEALTHY_INCUMBENT` | `CONFLICT` | `RETRYABLE` | healthy current controller cannot be preempted |
| `RECOVERY_CURRENT_PLACEMENT_UNAVAILABLE` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | game-domain current actor route/owner cannot be established safely |
| `TAKEOVER_BLOCKED_BY_LIFECYCLE` | `CONFLICT` | `RETRYABLE` | combat/PZ/logout/mandatory-presence state blocks intentional takeover |
| `LEASE_FENCE_UNCERTAIN` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | current/replacement lease authority cannot be proven safely |
| `SESSION_INTERNAL_UNAVAILABLE` | `INTERNAL_UNAVAILABLE` | `RETRYABLE` | safe fail-closed unexpected internal admission/session condition |

`RETRYABLE` never means immediate unbounded retry. Owning operation/profile defines whether retry requires changed state, same attempt identity, a fresh authorization or bounded backoff.

Public presentation may collapse sensitive distinctions. Internal evidence retains bounded code without exposing account/character facts or credentials.

## 29. Logging, privacy and correlation

Permitted bounded correlation, when authorized, includes AdmissionAttemptRef, RecoveryAttemptRef, GameSessionId, HandoffId, WorldId/ChannelId, relevant revisions, pseudonymous AccountId/CharacterId references, typed FND-04 failure code and timing/freshness measurements.

Never log/export raw Game Login Ticket, PreAdmissionGrant, RecoveryAuthorizationGrant, reconnect proof, OAuth tokens, passwords/MFA/recovery material, private signing keys or secret fencing material.

Bearer material must be redacted in application logs, traces, proxy logs, crash reports and analytics.

## 30. Versioning, downgrade and rollout

Keep independent:

- FND-04 semantic contract revision;
- fresh-entry credential security/interchange profile revision;
- recovery credential security/interchange profile revision;
- Platform account-security revision;
- runtime observation/ownership generation;
- FND-02 protocol major/transport/schema revisions;
- content/ruleset/runtime revisions;
- admitted-session state-machine implementation revision.

Unknown mandatory security/profile semantics fail closed. A lower-version consumer cannot ignore a critical field and accept as optional.

Native route activation requires an explicit producer/consumer compatibility matrix and independent fixtures for the exact Platform producer and Oteryn-v2 consumer revisions.

No unsupported version may fall back to Canary, plaintext, another audience, another Channel or alternate native route with the same credential.

## 31. Mandatory preimplementation companion gates

FND-04 architecture acceptance does not authorize production-capable runtime implementation.

### 31.1 Cross-language authorization/security profile

Before fresh-entry implementation, the profile must pin:

- exact PreAdmissionGrant container/encoding and credential type/purpose;
- exact signature algorithm/profile/allowlist and signed-byte canonicalization;
- issuer/audience/key-id/discovery semantics;
- AccountId/UUID encodings;
- AdmissionAttemptRef and GrantNonce encoding/entropy;
- exact mandatory claims and unknown-critical rejection;
- grant TTL, bounded wall-clock skew, key overlap/retirement/emergency revocation;
- security-generation/revocation-projection freshness bound;
- exact credential size/depth/count limits;
- independent Go/Rust golden positive/adversarial fixtures;
- cross-version/downgrade and secret-redaction tests.

Before reauthenticated recovery implementation, the coordinated profile must additionally pin:

- distinct RecoveryAuthorizationGrant credential type/purpose;
- RecoveryAttemptRef semantics;
- exact recovery audience/bindings;
- mutually exclusive fresh-entry vs recovery validation rules;
- Platform producer compatibility and rollback behavior.

Application libraries/vendors remain implementation choices after profile acceptance.

### 31.2 Liveness/lease/abuse parameter profile

Measured/fault/security evidence must freeze:

- liveness probe cadence/hysteresis sufficient to implement the accepted two-second loss boundary without unsafe false positives;
- recovered-stable anti-flap threshold controlling a new loss episode;
- CharacterLease TTL, renewal cadence and safety margin;
- durable fencing/dependency deadlines;
- reconnect-proof primitive/size and verifier-retention policy;
- admission/recovery/takeover rate limits;
- maximum retry/reconciliation windows.

Values may later be tuned through explicit versioned evidence without changing FND-04 semantics.

### 31.3 DUR-02 dependency

The first authoritative durable implementation must consume accepted DUR-02 physical persistence/fencing semantics. FND-04 defines required behavior; DUR-02 defines how PostgreSQL transactions/locks/revisions/recovery enforce it.

## 32. Required implementation/E2E proof

A later implementation package must prove on exact revisions at least:

1. valid fresh Platform authorization creates exactly one GameSession and one current transport binding;
2. one GrantNonce cannot produce two successful admissions under concurrency;
3. ambiguous Platform fresh-entry issuance with same AdmissionAttemptRef cannot create multiple independent capabilities;
4. post-issuance Platform security revocation/generation change rejects old unconsumed grant within accepted freshness bound;
5. stale runtime ownership generation rejects fresh-entry grant before nominal expiry;
6. AccountId<->CharacterId transfer/sale/lifecycle race rejects stale owner;
7. two CharacterIds of one account cannot both become authoritative;
8. healthy combat/PZ/logout-locked incumbent cannot be kicked by second authenticated client;
9. logout-eligible takeover produces one replacement authority without overlap;
10. CONTROL_SUSPECT returns to ACTIVE when sufficient current-generation evidence resumes before declared loss;
11. same-session reconnect preserves GameSessionId and advances exactly one connection_generation;
12. stale transport frames cannot command or advance liveness after rebind;
13. reconnect-proof replay cannot fence valid winner;
14. lost rebind response cannot revive predecessor proof and recovers only through accepted recovery authorization/current game state;
15. recovery credential cannot be validated as fresh-entry credential or vice versa;
16. same-session grace expires at `control_loss_declared_at + 15s` under server monotonic timing;
17. ordinary rebind does not manufacture four-second PvE protection;
18. one loss episode cannot produce repeated protection windows through flapping;
19. post-grace recovery attaches fresh GameSession to exact existing actor without respawn/reset/duplication;
20. different-character login remains blocked while old actor mandatory-present;
21. terminal GameSession with PRESENT_UNCONTROLLED actor does not release AccountPresenceClaim or CharacterLease merely due to session terminality;
22. current runtime may renew the existing CharacterLease for mandatory uncontrolled actor without granting player command authority;
23. lease renewal uncertainty never self-grants replacement writer;
24. stale lease/runtime generation cannot commit durable mutation;
25. GameNode replacement preserves same GameSession only when required command/session state reconstructable; otherwise safe fresh-session recovery;
26. Channel->Instance->Channel continuous handoff preserves GameSessionId while maintaining one writer;
27. Channel->Channel transition uses fresh destination authorization/fresh GameSessionId while AccountPresenceClaim remains continuous;
28. mixed producer/consumer revisions fail closed without downgrade;
29. logs/traces contain no bearer/reconnect/recovery secret;
30. Tier-1 native login/admission/reconnect/recovery failure scenarios pass before route activation, with Tier-2 native-client evidence for supported client-facing recovery before user-visible readiness is claimed.

## 33. Decision summary

```text
Platform reusable credentials/security policy
-> Platform-owned

FRESH_ENTRY PreAdmissionGrant
-> distinct signed short-lived Platform capability
-> route/runtime/security applicability bound
-> GrantNonce consumed once by game admission

RecoveryAuthorizationGrant
-> distinct future Platform authorization type
-> RECOVER_EXISTING_CONTROL only
-> does not choose actor placement

Oteryn-v2 final admission
-> current ownership/runtime/account-presence/lease revalidation
-> one atomic authority commit
-> canonical GameSessionId created only on success

one AccountId
-> at most one playable/mandatory-presence CharacterId

AccountPresenceClaim
-> survives transport/GameSession loss while actor remains present

CharacterLease + character-writer fence
-> survives GameSession terminality when mandatory actor remains
-> can be renewed by current runtime for PRESENT_UNCONTROLLED actor
-> never grants player command authority by itself

TransportBinding + connection_generation
-> exactly one current controller per GameSession

control evidence
-> suspect before 2s boundary
-> declared lost at T0+2s
-> stale concrete transport cleanup at T0+5s
-> same-GameSession grace expires 15s after declared loss
-> at most one 4s PvE defensive effect per eligible loss episode

post-grace mandatory actor
-> old GameSession terminal
-> AccountPresenceClaim + CharacterLease remain
-> same CharacterId may attach fresh GameSession after full recovery revalidation
-> different CharacterId remains blocked

Channel<->Instance continuous handoff
-> may preserve GameSessionId

Channel->Channel session transition
-> fresh route + fresh FRESH_ENTRY grant + fresh GameSessionId

lease expiry/uncertainty
-> never self-grants replacement authority
```

## 34. Consequences

### Positive

- authentication, authorization, account presence, character writer, GameSession, transport and runtime ownership are explicit and independently fenced;
- fresh-entry and recovery credentials cannot be silently confused;
- account-global duplicate-session behavior is combat-safe;
- stale runtime generations cannot resurrect old routes;
- reconnect preserves logical continuity without making GameSessionId a bearer token;
- lost rebind response has a deterministic security-safe fallback;
- the 2s control-loss declaration is distinct from first evidence suspicion;
- mandatory actor/lease presence is independent from GameSession lifetime;
- Channel/Instance handoffs retain one-writer fencing;
- numeric/security technology choices remain evidence-driven.

### Costs

- implementation needs authoritative durable account/lease/session fencing state;
- rare lost-proof/rebind cases require Platform reauthentication;
- recovery needs a separate coordinated Platform producer credential and current game-domain route resolver;
- cross-language security fixtures are mandatory;
- DUR-02 must prove physical transaction/fencing substrate before durable production admission;
- model is intentionally richer than one generic session token/row.

## 35. Non-authorization and gate effect

When merged, this document completes the **semantic architecture contract** for FND-04.

It does not authorize production-capable implementation by itself. Implementation remains blocked until:

- section-31 authorization/security profile is accepted, including the coordinated Platform recovery profile before reauthenticated recovery is implemented;
- section-31 liveness/lease/abuse parameter profile is accepted;
- required DUR-02 persistence/fencing semantics exist for the implementation slice;
- a dedicated task explicitly authorizes Rust/protocol/persistence implementation;
- exact-head tests/audits/E2E for that implementation pass.

After semantic FND-04 acceptance, later architecture packages may proceed where they do not bypass these implementation prerequisites.
