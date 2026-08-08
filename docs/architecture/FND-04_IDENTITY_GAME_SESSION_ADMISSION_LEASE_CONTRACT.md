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

FND-04 freezes the minimum semantic authority model required for native Oteryn-v2 gameplay admission without collapsing authentication, admission, session identity, account exclusion, character writer fencing, transport control and runtime scope ownership into one generic session object.

The canonical separation is:

```text
Platform authentication/security authority
    != Platform bounded admission-attempt authorization
    != AccountPresenceClaim
    != CharacterLease
    != GameSessionId / GameSession lifecycle
    != TransportBinding / connection_generation
    != ChannelRuntime / InstanceRuntime scope ownership generation
```

These layers may be co-located in one initial Rust server deployment. Co-location does not merge their semantics or owners.

## 2. Decision timing

### Must decide now? — YES

This contract blocks production implementation of:

- native fresh gameplay admission;
- canonical `GameSessionId` creation;
- account-global one-online-character enforcement;
- CharacterLease acquisition/fencing;
- reconnect/rebind and stale-transport rejection;
- duplicate-login/takeover behavior;
- post-grace same-character recovery;
- Channel/Instance session continuity;
- native admission/recovery failure mapping;
- first end-to-end Platform -> Gateway -> Oteryn-v2 gameplay entry.

### What becomes expensive if wrong

A wrong authority model could create dual playable characters, replayed grants, stale character writers, stale transport control, combat-escape or forced-disconnect abuse, route-owner resurrection after failover, ambiguous crash recovery or public success before authoritative session state exists.

Correcting those after deployed clients and durable data exist would require coordinated protocol, persistence, Platform and runtime migration.

### Evidence that may justify supersession

A later accepted contract may change a specific choice only with named evidence such as:

- penetration testing or credential-replay findings;
- cross-language security-profile interoperability failure;
- measured Platform/security-projection availability or latency evidence;
- lease/fencing fault injection exposing a split-writer window;
- reconnect-storm/liveness false-positive evidence;
- player fairness/abuse telemetry;
- a product requirement for sender-constrained credentials or stronger device/step-up authentication.

### Deliberately not decided here

This contract does not select:

- a Rust or Go cryptographic library;
- KMS/HSM/cloud-vendor products;
- PostgreSQL table/index layout or transaction isolation level;
- a Redis topology or any Redis authority role;
- exact grant TTL, wall-clock skew, verification-key overlap/cache values;
- exact reconnect-secret byte length/primitive;
- exact liveness probe cadence/hysteresis values;
- exact CharacterLease TTL/renewal/safety-margin values;
- exact admission/recovery rate limits;
- deployment/service extraction topology;
- final player-facing UX strings.

Those values or products may be chosen only through the explicit preimplementation gates in section 30.

## 3. Canonical authority layers

### 3.1 Platform Identity and security

Oteryn Platform remains authoritative for reusable account credentials, OAuth/PKCE, MFA, recovery, Platform sessions, Game Login Ticket issuance and Platform-owned account-security state.

The Rust game server does not become a second password/OAuth/MFA authority.

### 3.2 Platform bounded admission-attempt authorization

Platform/Gateway may authorize one bounded attempt to enter gameplay using current Platform policy plus fresh applicable Oteryn-v2 runtime evidence.

That authorization is not proof that gameplay admission succeeded and is not a `GameSessionId`, CharacterLease or current gameplay-owner claim.

### 3.3 `AccountPresenceClaim`

`AccountPresenceClaim` is the game-domain account-global exclusion state:

```text
scope: AccountId
value: CharacterId or none
purpose: at most one authoritative playable or mandatory-presence character per account
```

Required invariant:

```text
count(authoritative_playable_or_mandatory_presence_characters(AccountId)) <= 1
```

The claim remains held while the current actor is either `PRESENT_CONTROLLED` or `PRESENT_UNCONTROLLED`.

It is not released merely because:

- a socket closes;
- `connection_generation` changes;
- the 15-second same-session grace expires;
- a GameSession becomes terminal;
- the client process disappears;
- a lease renewal becomes uncertain while an actor may still exist.

Release requires an authoritative lifecycle transition proving that the actor is legally absent or that a legal same-account replacement transition has completed without dual authority.

### 3.4 `CharacterLease`

`CharacterLease` is the game-domain exclusive character writer/control fence:

```text
scope: CharacterId
binds: AccountId + character_lease_generation + current authoritative actor/runtime context
purpose: prevent stale session/runtime/durable writers from committing after authority changes
```

Character Authority remains the semantic owner of CharacterId, character lifecycle and authoritative AccountId<->CharacterId ownership.

Game Session / Admission coordinates player-session lease transitions. Runtime and persistence consumers must enforce the current lease fence. Physical storage/transaction details remain DUR-02.

### 3.5 `GameSession`

`GameSessionId` identifies one logical admitted player-control lifecycle.

The game-domain Game Session / Admission authority creates it only at the successful admission linearization point defined in section 8.

`GameSessionId` is never a credential.

### 3.6 `TransportBinding`

`TransportBinding` is the current concrete control binding for one `GameSessionId`:

```text
scope: GameSessionId
fence: connection_generation
purpose: exactly one current transport may command, advance liveness or mutate reconciliation authority
```

FND-02 remains authoritative for the visible `uint64 connection_generation` semantics.

### 3.7 `RuntimeScopeAuthority`

FND-03 remains authoritative for current ChannelRuntime/InstanceRuntime ownership generation.

A valid session or lease cannot authorize a stale runtime scope owner. A valid runtime owner cannot create a GameSession or account/character lease by itself.

## 4. Canonical actor-presence states

FND-04 requires the game-domain lifecycle to distinguish at least:

```text
ABSENT
PRESENT_CONTROLLED
PRESENT_UNCONTROLLED
```

`PRESENT_CONTROLLED` means one current GameSession/TransportBinding has playable control over the authoritative actor.

`PRESENT_UNCONTROLLED` means the authoritative actor remains in world simulation but no current playable controller is attached. Combat/PZ/logout rules may require this state after control loss or GameSession terminality.

`ABSENT` means the authoritative lifecycle has legally removed the actor from mandatory world presence.

These are semantic states; exact enum names/storage fields are implementation details.

## 5. Fencing values

### 5.1 Account-presence generation

Every current account-presence claim uses a stale-safe non-zero monotonic generation/revision or equivalent linearizable CAS fence scoped to `AccountId`.

Required semantics:

- stale admission/takeover work cannot replace a newer claim;
- changing/releasing/re-establishing account presence advances the fence where required to reject stale work;
- continuous presence of the same CharacterId may preserve the claim through transport reconnect, GameSession replacement or legal runtime handoff;
- no wrap/reuse; exhaustion fails closed.

This is an ordering/fencing value, not a foundation entity ID.

### 5.2 `character_lease_generation`

Every current CharacterLease uses a non-zero monotonic generation or equivalent stale-safe fence scoped to `CharacterId`.

Required semantics:

- older-generation player/runtime/durable writes fail closed;
- transport reconnect alone does not advance the character lease generation;
- replacing the authoritative character writer/runtime context advances the generation before the replacement can commit durable authority;
- a legal Channel/Instance ownership handoff that replaces the current character writer context advances or establishes an equivalent destination fence before source authority is released;
- no wrap/reuse; exhaustion fails closed.

The initial implementation representation should use a `uint64`-class monotonic fence unless DUR evidence proves a different equivalent representation is required.

### 5.3 `connection_generation`

FND-02 semantics are binding:

- pre-admission/resume bootstrap uses `0` only;
- fresh admission begins at `1`;
- every committed same-GameSession rebind establishes exactly one strictly newer non-zero generation;
- rejected/uncommitted attempts do not advance authority;
- old generations cannot command, advance liveness or mutate reconciliation state;
- no wrap/reuse; exhaustion makes the GameSession terminal.

## 6. No new `AdmissionId` or `CharacterLeaseId`

FND-04 does not add `AdmissionId` or `CharacterLeaseId` to the foundation identity catalogue.

Use instead:

- `AdmissionAttemptRef` — bounded Platform producer operation/correlation identity;
- `GrantNonce` — cryptographically random concrete PreAdmissionGrant one-time consume/replay key;
- `GameSessionId` — admitted logical gameplay session identity;
- account/character/connection/runtime generations — scoped fencing values;
- `HandoffId` — the already accepted cross-runtime ownership-transition identity where a handoff exists.

A later FND-ID amendment requires proof that another separately addressable durable semantic entity is necessary.

## 7. PreAdmissionGrant class — accepted

FND-04 accepts the following credential class for fresh native admission:

```text
short-lived signed Platform PreAdmissionGrant
+ explicit versioned security/interchange profile
+ authoritative game-domain one-time GrantNonce consumption
+ current game-domain revalidation before admission commit
```

This is a hybrid design: cryptographic issuer authenticity is self-contained, while one-successful-admission replay safety is game-domain authoritative state.

Normal grant verification must not require synchronous Platform introspection merely to validate every valid attempt. Security freshness is handled by section 11.

### 7.1 Credential purposes

Credential purpose is explicit and non-interchangeable.

At minimum the architecture recognizes distinct semantics for:

- `FRESH_ENTRY` — bounded attempt to enter one selected native World/Channel route;
- `RECOVER_EXISTING_CONTROL` — reauthenticated same-character recovery where game-domain state already contains the authoritative actor/session or post-grace actor presence.

A `FRESH_ENTRY` grant can never be reinterpreted as `RECOVER_EXISTING_CONTROL` and vice versa.

Unsupported/unknown mandatory purpose fails closed.

## 8. Fresh-admission linearization point

Fresh admission is one externally unambiguous authority transition.

Conceptually the game-domain admission authority performs:

```text
validate bounded material and security profile
validate signature/key purpose/issuer/audience/time
validate Platform security freshness applicability
validate WorldId/ChannelId/route/runtime observation/owner generation/revisions
revalidate current AccountId -> CharacterId ownership/lifecycle
validate account-global presence / duplicate-login state
validate target runtime is current and admission-capable
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

A candidate `GameSessionId` is not canonical until this transition commits. An uncommitted candidate is discarded and never reused.

No client-visible admission success may precede the authoritative commit.

If the required durable/fencing substrate cannot prove this transition safely, admission fails/holds closed. It does not downgrade to an unfenced in-memory success.

## 9. `AdmissionAttemptRef` and `GrantNonce`

### 9.1 `AdmissionAttemptRef`

Owner: Platform admission producer operation lifecycle.

Purpose:

- correlate one logical issuance attempt;
- distinguish retry from an independent attempt;
- make ambiguous issuer outcomes idempotent/reconcilable;
- provide bounded security/audit correlation.

It is not a credential, GameSessionId, GrantNonce or gameplay entity ID.

For one logical issuance attempt, Platform must either recover the same committed outcome or deterministically retire/reconcile the attempt. It must not mint multiple independently usable grants because a response was lost.

### 9.2 `GrantNonce`

`GrantNonce` identifies one concrete issued capability for authoritative game-domain consume/replay state.

Required properties:

- cryptographically random under the accepted security profile;
- unique enough for the profile's bounded issuance/retention domain;
- bound inside the authenticated grant;
- consumed at most once by the fresh-admission linearization transition;
- a consumed/expired/wrong-bound nonce can never become valid again;
- two distinct concrete grants must not intentionally share one GrantNonce.

A lost game-admission response does not make GrantNonce reusable. Recovery follows the admitted-session/recovery state machine.

## 10. PreAdmissionGrant mandatory semantic bindings

A `FRESH_ENTRY` grant must authenticate, directly or through the accepted immutable profile, equivalents of:

- credential type;
- credential purpose;
- security/profile revision;
- issuer identity;
- exact game-admission audience;
- `AdmissionAttemptRef`;
- `GrantNonce`;
- canonical `AccountId`;
- canonical `CharacterId`;
- canonical `WorldId`;
- canonical `ChannelId`;
- Platform account-security generation/revision or equivalent security-applicability evidence when used by the selected security profile;
- route/offer/topology revision or immutable admission-target revision;
- runtime observation/source revision;
- issuance-time runtime scope ownership generation where required for stale-owner prevention;
- protocol/transport profile;
- required content/ruleset/runtime compatibility revisions;
- server-authoritative issuance/not-before/expiry semantics;
- signing-key identifier/version.

The credential may carry additional bounded fields only when their owner, purpose and validation semantics are explicit.

The credential must not contain reusable Platform account credentials, passwords, OAuth refresh/access tokens, signing secrets, arbitrary verifier URLs or client-selected issuer/audience authority.

## 11. Post-issuance Platform security freshness — accepted model

FND-04 accepts this semantic model:

```text
short-lived signed PreAdmissionGrant
+ Platform-owned account-security generation/revision bound at issuance
+ authenticated game-side bounded-staleness security generation/revocation projection
+ fail closed for new admission when required security freshness cannot be proven
+ exceptional online introspection may exist only as an explicitly authorized fallback/reconciliation path
```

The exact projection transport and storage are not frozen.

### 11.1 Required behavior

If authoritative Platform account-security state has advanced in a way that invalidates the issued security revision before final admission, the unconsumed grant is rejected even when its signature and nominal expiry are otherwise valid.

If the selected security profile requires a sufficiently current security projection and that evidence is stale, unavailable, contradictory or invalid, new admission fails closed.

A bounded maximum projection staleness/risk window is mandatory before implementation and is frozen by the parameter/security profile in section 30.

### 11.2 Post-admission boundary

This pre-admission security rule does not grant Platform authority to asynchronously delete actors, release CharacterLease, terminate GameSession or force a combat-locked client offline.

Any emergency post-admission session revocation/control operation requires a separately accepted game-domain control/fencing contract.

## 12. Runtime observation and ownership-generation applicability

Platform may issue a `FRESH_ENTRY` grant only from fresh applicable current-owner Oteryn-v2 runtime evidence.

Final game admission still revalidates current authoritative runtime applicability.

Default rule:

```text
issuance-time target ownership generation != current authoritative target ownership generation
-> reject grant as STALE_GENERATION
-> require fresh Platform routing + fresh FRESH_ENTRY grant
```

The same rule applies to an obsolete route/topology/admission target revision when the change invalidates the original target applicability.

A grant is never silently retargeted to another Channel, GameNode, runtime owner or downgraded protocol route.

A future relaxation requires explicit proof that a generation-independent route fence preserves equivalent security and stale-owner guarantees.

## 13. Character ownership and lifecycle revalidation

Platform projection may prevent knowingly unsafe issuance, but final Oteryn-v2 admission revalidates current Character Authority facts.

At minimum:

- current `AccountId <-> CharacterId` ownership;
- deletion/transfer/sale/lock lifecycle state;
- current World applicability;
- game-domain sanctions/eligibility where owned by the game or explicit enforcement contract;
- current account-presence and CharacterLease state.

A stale Platform/browser/client claim never overrides newer game-domain authority.

## 14. GameSession lifecycle

The minimum logical session lifecycle is:

```text
NO_SESSION
    -> ADMISSION_COMMIT -> ACTIVE

ACTIVE
    -> current transport becomes insufficient/unavailable
    -> CONTROL_LOST_GRACE

CONTROL_LOST_GRACE
    -> eligible same-session rebind/recovery -> ACTIVE
    -> grace expires -> TERMINAL

ACTIVE
    -> legal logout/takeover/channel-session replacement/session revocation
    -> TERMINAL
```

`ADMISSION_COMMIT` is a transition, not a long-lived authority state.

Implementation may use richer internal states, but they must preserve the same externally relevant authority boundaries.

`TERMINAL` is irreversible for that `GameSessionId`. A later logical session receives a fresh GameSessionId.

## 15. Server-authoritative liveness and control-loss timing

FND-04 decides what current-generation evidence is sufficient to prove playable control. FND-03 measures accepted evidence using the process-local monotonic clock.

The accepted composition is:

```text
last_sufficient_control_at = T0
control_loss_declared_at   = T0 + 2.0 s
stale_transport_cleanup    = T0 + 5.0 s
same_session_grace_expires = control_loss_declared_at + 15.0 s
```

Therefore the same logical GameSession receives a full 15-second recovery grace after server-authoritative control loss is declared.

The 5-second concrete transport cleanup is independent and does not release actor/account/lease authority.

Client timestamps, browser state, OS event logs or Launcher/Guardian evidence cannot move these authoritative timers.

### 15.1 Before `control_loss_declared_at`

A legitimate transport replacement may commit before the two-second loss boundary if the old binding is otherwise proven stale/lost.

That rebind may preserve the GameSession and advance `connection_generation`, but it does not receive the four-second defensive PvE re-entry effect merely because a transport changed.

### 15.2 Defensive PvE re-entry effect

The accepted four-second PvE defensive re-entry effect is granted at most once per eligible server-classified unexpected playable-control-loss episode.

Routine rebind, graceful reconnect, intentional takeover or connection-generation rotation does not manufacture eligibility.

FND-04 requires an actor/session-scoped control-loss episode state so multiple retries/rebinds inside the same loss episode cannot repeatedly re-arm the effect.

A new loss episode may begin only after the prior episode has reached the accepted recovered-stable boundary. The exact anti-flap/hysteresis numeric threshold is a preimplementation parameter gate.

## 16. Primary same-GameSession reconnect

Primary reconnect uses a game-domain-issued high-entropy opaque rotating reconnect proof, never GameSessionId and never a Platform reusable credential.

Required semantic properties:

- bound to exactly one current GameSession and reconnect-proof generation/state;
- sent only over the accepted TLS transport;
- server stores a verifier/digest or equivalent secret-safe representation where practical;
- raw proof is never logged, traced or exported to analytics;
- successful rebind rotates the proof;
- predecessor proof cannot authorize an unrelated/later rebind;
- replay fails closed without changing current authority;
- terminal GameSession invalidates reconnect authority.

### 16.1 Rebind linearization

A same-session rebind succeeds only when all required checks hold:

```text
GameSessionId still current/non-terminal
AccountId/CharacterId binding still valid
AccountPresenceClaim still compatible
CharacterLease/current writer fence still compatible
old transport proven stale/lost or otherwise replaceable
current runtime placement/revisions valid
reconnect proof valid
```

Then one atomic rebind transition:

```text
commits exactly one newer connection_generation
rotates reconnect proof state
fences every older transport generation
preserves GameSessionId
preserves actor/gameplay state
publishes rebind success only after commit
```

TLS establishment alone is not gameplay authority.

### 16.2 Concurrent reconnect race

Two reconnect contenders cannot both win.

The first linearized successful rebind becomes current. Later contenders observe stale proof/generation/current-control state and fail without fencing the winner.

## 17. Lost rebind response — accepted reconciliation rule

FND-04 rejects rotate-and-forget and also rejects making an old reconnect proof generally valid again after rotation.

If the server commits a newer connection generation and reconnect-proof rotation but the client does not receive the response, recovery uses the distinct Platform-reauthenticated `RECOVER_EXISTING_CONTROL` path in section 18.

The predecessor reconnect proof remains non-authoritative for creating another rebind after the commit.

This deliberately trades a rare fallback Platform dependency for simpler replay safety and avoids retaining a broadly reusable predecessor credential.

The game side must still reconcile current state rather than assume the previous rebind failed merely because the client reports a lost response.

## 18. Platform-reauthenticated same-character recovery — accepted

Oteryn-v2 accepts a distinct reauthenticated recovery path for cases such as lost reconnect material or lost rebind response.

The Platform-side producer implementation requires a separately coordinated future Platform task; this contract only freezes the Oteryn-v2 semantic consumer boundary.

### 18.1 Recovery credential purpose

The credential purpose is `RECOVER_EXISTING_CONTROL` or an exact later profile equivalent.

It proves bounded current Platform authorization for the AccountId/CharacterId/WorldId recovery intent.

It does not carry authority to choose the actor's current Channel/Instance placement.

The game domain resolves current authoritative placement from game/runtime state.

### 18.2 Healthy incumbent protection

A recovery credential cannot preempt a healthy current controller merely because the second client reauthenticated.

If the incumbent session is healthy/current, recovery fails with a conflict and the incumbent remains authoritative.

### 18.3 Inside same-session grace

If the existing GameSession is in `CONTROL_LOST_GRACE`, the actor is the same CharacterId, ownership/presence/lease/runtime state are current and no healthy controller exists, the recovery path may preserve the same GameSessionId and atomically establish a newer connection_generation plus fresh reconnect-proof state.

### 18.4 Route resolution

Recovery never trusts stale fresh-entry ChannelId data as actor placement authority.

A future implementation must provide a bounded authenticated game-domain resolver or read-only route projection capable of directing recovery to the current authoritative owner without making Platform the GameSession owner.

The exact API/service deployment is deferred, but implementation may not claim cross-placement recovery until this resolver path exists and is tested.

## 19. Post-grace same-character control recovery — accepted

When:

```text
same_session_grace_expires
AND actor remains PRESENT_UNCONTROLLED due to combat/PZ/logout/lifecycle rules
```

the old GameSession becomes terminal and cannot be resurrected.

`AccountPresenceClaim` remains held for the same CharacterId. The authoritative actor and compatible CharacterLease/runtime path remain present.

A fully reauthenticated `RECOVER_EXISTING_CONTROL` attempt may create a **fresh GameSessionId** and attach control to that exact existing actor when all of these are proven:

- old GameSession is terminal;
- no current playable controller exists;
- current AccountId still owns CharacterId;
- account presence still points to that CharacterId;
- current CharacterLease/current writer fence is valid;
- current authoritative actor placement is resolved from game-domain authority;
- required protocol/content/ruleset/runtime revisions are compatible;
- security freshness policy is satisfied.

The new GameSession begins `connection_generation = 1` and receives new reconnect-proof state.

This path must not respawn, teleport, duplicate, heal, refill, clear conditions/cooldowns/combat/PZ/logout state, reset threat/encounter state or roll back committed effects.

If the original unexpected-loss episode remains eligible and its defensive re-entry effect has not already been consumed, the one-shot four-second effect may apply to the restored control transition. A fresh GameSession does not create a second protection entitlement for the same episode.

A different CharacterId remains blocked while the incumbent actor has mandatory presence.

## 20. Duplicate login and intentional takeover

### 20.1 Account-global exclusion

Every fresh admission evaluates AccountPresenceClaim before granting gameplay authority.

Two different CharacterIds of one AccountId cannot both become authoritative, even across different worlds/channels/instances/nodes.

### 20.2 Healthy combat/PZ/logout-locked incumbent

A second authenticated client cannot fence, close, revoke or steal a healthy incumbent session solely by presenting valid credentials.

If the incumbent actor has a combat/PZ/logout blocker and current playable control remains healthy, the newcomer is denied/held as a conflict and the incumbent remains fully authoritative.

### 20.3 Logout-eligible incumbent

If the incumbent actor is legally logout-eligible, a fully authenticated newcomer may perform intentional takeover through a fenced transition:

1. authenticate/authorize newcomer;
2. prove takeover eligibility;
3. terminate/revoke old gameplay control at the legal boundary;
4. complete old actor logout/removal or accepted same-character transition;
5. only then establish the new authority.

No transition may expose two player-controlled authoritative characters.

A new logical GameSession after completed intentional takeover receives a new GameSessionId.

### 20.4 Incumbent genuinely unavailable

Genuine control loss uses same-character recovery rules. It is not treated as hostile newcomer preemption.

A different CharacterId remains blocked while the old actor is still mandatory-present.

## 21. CharacterLease owner and storage-authority contract

### 21.1 Logical owner

CharacterLease authority belongs to the Oteryn-v2 game domain. Platform/Gateway/client never owns or issues a CharacterLease.

Game Session / Admission coordinates acquisition/replacement for player-control transitions. Character Authority remains authoritative for character identity/lifecycle/ownership. Runtime and persistence enforce the current fence.

### 21.2 Storage authority

Authoritative lease/account-presence/session fencing state must live in game-owned authoritative durable/recoverable state under the accepted `oteryn_game` persistence boundary.

PostgreSQL is the accepted relational target from ADR-0004. Redis or any cache may accelerate reads but cannot become the sole authoritative lease/account-presence/session fence.

DUR-02 owns physical schema, indexes, isolation/locking mechanism, transaction layout, retention and crash-recovery encoding.

FND-04 implementation is blocked if the selected DUR-02 design cannot prove the linearizable/fenced semantics required here.

## 22. CharacterLease acquisition, renewal, uncertainty and release

### 22.1 Acquisition

A lease is acquired/established only inside an authoritative admission, recovery or handoff transition that also proves account presence, character ownership and current runtime writer applicability.

It is never granted merely because an old lease appears expired from one participant's local clock.

### 22.2 Renewal

The current authoritative writer/runtime may renew only the current lease generation and only while all required ownership/runtime/session conditions remain valid.

Stale lease generation, stale runtime owner generation or stale GameSession context cannot renew current authority.

Exact TTL/renewal/safety-margin values are implementation parameters, but the configured margins must ensure the old writer stops durable authority before a newer generation can commit.

### 22.3 Renewal uncertainty / expiry

Renewal timeout, dependency uncertainty or observed expiry does **not** self-grant replacement authority.

On uncertainty:

- new player-originated durable mutation fails closed at the appropriate boundary;
- stale-generation durable writes are prohibited;
- account presence is not released while an actor may still exist;
- no newer character writer generation is committed until the old writer/runtime is safely fenced or recovered;
- server-driven in-memory simulation may continue only under independently current FND-03 runtime-scope authority and only where it cannot race a replacement durable writer.

### 22.4 Replacement

A new character lease generation may commit only after the transition proves that the previous generation can no longer commit authoritative writes under the selected DUR/OPS fencing mechanism.

The replacement commit is the fence; waiting for cooperative shutdown/ack from the stale holder is not required for correctness once durable fencing is proven.

### 22.5 Release

CharacterLease release requires an authoritative lifecycle boundary proving that no actor/runtime path still requires that lease generation for the current character.

Socket close, GameSession terminality, reconnect grace expiry or client logout request alone are insufficient.

A legal handoff may replace rather than release the lease generation while continuous actor presence is preserved.

## 23. Same-GameSession recovery across GameNode replacement

Same-GameSession recovery across process/GameNode replacement is allowed only when the new authoritative path can preserve or safely reconstruct all state required to prevent replay/order contradiction, including at least:

- GameSessionId and lifecycle state;
- current `connection_generation` and reconnect-proof state needed for safe recovery;
- FND-02 `next_command_id` high-water state;
- pending command identities/outcomes required to prevent duplicate execution;
- server-sequence/snapshot reconciliation boundary;
- AccountPresenceClaim and its current fence;
- CharacterLease generation/current fence;
- authoritative actor state/current placement;
- current runtime scope ownership generation/revision applicability.

If any required state cannot be reconstructed safely, the old GameSession becomes terminal.

The actor/account presence may remain authoritative. Recovery then uses the fresh-GameSession same-character path from section 19 when its conditions are met.

Runtime convenience may never weaken no-double-execution, stale-generation or one-online-character guarantees.

## 24. Channel/Instance handoff continuity

FND-04 consumes the accepted `HandoffId` and FND-03 ownership-transition model.

For every handoff:

- one HandoffId identifies one transition lifecycle;
- AccountId, CharacterId, GameSessionId, account-presence fence, CharacterLease generation and source/destination runtime generations are explicit bindings;
- prepare work may overlap, but only one runtime owner may accept authoritative player mutation;
- destination writer fence must be established before source writer authority is released;
- stale/replayed HandoffId or generation fails closed;
- failure before commit preserves/reconciles source authority;
- failure after commit recovers from destination authority evidence.

### 24.1 Channel -> Instance and Instance -> Channel

When the transition is one continuous logical player-control session, the same GameSessionId is preserved.

AccountPresenceClaim remains continuously held for the same CharacterId.

CharacterLease generation advances or an equivalent destination writer fence is committed when writer/runtime authority changes.

If the concrete transport remains valid and bound to the same logical session, `connection_generation` need not change solely because simulation scope changed. If a transport rebind is required, normal section-16 generation rules apply.

### 24.2 Channel -> Channel

An accepted Channel -> Channel transition establishes a fresh logical GameSession on the destination and therefore a fresh GameSessionId.

AccountPresenceClaim remains continuously held for the same CharacterId across the committed transition so another CharacterId cannot enter a race window.

Destination CharacterLease/current writer fencing must be established before source authority is released.

A fresh destination Platform/Gateway route and fresh `FRESH_ENTRY` authorization are required; old admission material is never retargeted.

## 25. Failure and dependency behavior

### Platform unavailable

New Platform-dependent fresh admission and reauthenticated recovery cannot invent an alternate credential authority.

An already admitted healthy gameplay session or primary reconnect that requires no Platform security check beyond the accepted current projection does not automatically become invalid merely because Platform is unreachable.

### PostgreSQL/durable authority unavailable

If required admission/account-presence/lease/session state cannot be committed or fenced authoritatively, the transition cannot publish success.

Physical behavior remains DUR-02, but fail-open admission is forbidden.

### Runtime owner uncertain/superseded

Fresh admission/recovery/handoff fails or reconciles toward the current owner. A stale runtime owner cannot regain authority through a valid-looking session/grant.

### Ambiguous client outcome

Transport loss does not prove whether admission/rebind committed. The client must query/recover through the applicable game-domain session/recovery flow; it never treats a bearer credential as reusable merely because a response was lost.

## 26. Foundation failure-scenario dispositions

FND-04 classifies the shared catalogue as follows.

| Scenario | FND-04 status | Binding result |
|---|---|---|
| `FS-PLATFORM-UNAVAILABLE` | `PASS` | no alternate credential authority; new Platform-dependent admission/recovery fails boundedly; established unrelated gameplay does not fail merely from Platform outage |
| `FS-GATEWAY-AFTER-REDEEM` | `PASS` | no silent ticket reuse/downgrade; no GameSession without authoritative game admission commit |
| `FS-POSTGRES-UNAVAILABLE` | `DEFERRED_BY_ACCEPTED_GATE` | DUR-02 owns physical behavior; FND-04 forbids success when required durable/fenced transition cannot be proven |
| `FS-LEASE-RENEW-TIMEOUT` | `PASS` | old player/durable authority stops before newer writer may commit; expiry alone never grants replacement |
| `FS-DUPLICATE-LOGIN` | `PASS` | account-global exclusion; healthy protected incumbent cannot be kicked; at most one authority wins |
| `FS-STALE-GENERATION` | `PASS` | stale connection/account/lease/runtime generation cannot command/renew/reconnect/commit |
| `FS-DUPLICATE-COMMAND` | `NOT_APPLICABLE` | FND-02 remains authoritative once session exists |
| `FS-CHANNEL-SPLIT-OWNER` | `DEFERRED_BY_ACCEPTED_GATE` | FND-03/OPS own scope fencing; FND-04 validates current target owner |
| `FS-CHANNEL-DRAIN` | `DEFERRED_BY_ACCEPTED_GATE` | no fresh admission/handoff into non-admissible destination |
| `FS-QUEUE-SATURATION` | `DEFERRED_BY_ACCEPTED_GATE` | FND-03 limits; FND-04 must fail before partial authority transition |
| `FS-SLOW-CLIENT` | `DEFERRED_BY_ACCEPTED_GATE` | FND-02/FND-03 own bounded transport/resync; FND-04 owns resulting session eligibility |
| `FS-CLOCK-SKEW` | `PASS` | signed timestamps use bounded profile skew; liveness/grace durations use server monotonic time |
| `FS-KEY-ROTATION` | `PASS` | allowlisted current/retiring key policy; emergency revocation; no unknown/downgrade key acceptance |
| `FS-REVISION-MISMATCH` | `PASS` | fail closed; no implicit downgrade/mixed authority |
| `FS-SNAPSHOT-DELTA-MISMATCH` | `NOT_APPLICABLE` | FND-02/FND-03 reconciliation owns state replay after admission/rebind |
| `FS-DB-OUTBOX-BOUNDARY` | `DEFERRED_BY_ACCEPTED_GATE` | DUR/ANL own physical transaction/outbox; success cannot precede required authority commit |
| `FS-WORLD-BUNDLE-CORRUPT` | `NOT_APPLICABLE` | invalid world activation must already be unroutable |
| `FS-CLIENT-CUTOVER-ROLLBACK` | `NOT_APPLICABLE` | historical migration lifecycle |
| `FS-ADMISSION-GRANT-REPLAY` | `PASS` | consumed/expired/wrong-bound grant cannot create another admission/lease/GameSession |
| `FS-RECONNECT-CREDENTIAL-REPLAY` | `PASS` | stale/predecessor/stolen proof cannot produce parallel current binding or fence a valid winner |

Analytics-only failure scenarios remain `NOT_APPLICABLE` to session authority unless their owning ANL contract explicitly introduces a dependency; analytics cannot mutate FND-04 authority.

## 27. Stable internal failure codes

FND-04 freezes symbolic internal codes. Numeric wire/status registration belongs to the owning protocol/API profile.

| FND-04 code | Foundation category | Retry class | Meaning |
|---|---|---|---|
| `ADMISSION_INPUT_INVALID` | `INVALID_INPUT` | `TERMINAL` | malformed/non-canonical bounded admission input |
| `ADMISSION_CREDENTIAL_AUTH_FAILED` | `AUTHENTICATION_FAILED` | `SECURITY_TERMINAL` | bad signature/proof, untrusted issuer/key purpose or invalid recovery proof |
| `ADMISSION_GRANT_REJECTED` | `SESSION_REJECTED` | `TERMINAL` | expired, consumed, replayed, wrong-audience/purpose/binding grant |
| `ADMISSION_SECURITY_SUPERSEDED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | Platform account-security state invalidates issued grant |
| `ADMISSION_RUNTIME_STALE` | `STALE_GENERATION` | `TERMINAL` | route/runtime observation or ownership generation superseded |
| `ADMISSION_REVISION_UNSUPPORTED` | `UNSUPPORTED_REVISION` | `TERMINAL` | protocol/content/ruleset/runtime/security profile incompatible |
| `ADMISSION_OWNERSHIP_REJECTED` | `CONFLICT` | `TERMINAL` | authoritative AccountId/CharacterId lifecycle/ownership no longer permits attempt |
| `ADMISSION_ACCOUNT_PRESENCE_CONFLICT` | `CONFLICT` | `RETRYABLE` | another mandatory/current character presence blocks admission |
| `ADMISSION_DEPENDENCY_UNAVAILABLE` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | required security/runtime/durable dependency cannot establish safe current evidence |
| `RECONNECT_PROOF_REJECTED` | `AUTHENTICATION_FAILED` | `SECURITY_TERMINAL` | reconnect proof invalid/replayed/stale for requested rebind |
| `RECONNECT_SESSION_TERMINAL` | `SESSION_REJECTED` | `TERMINAL` | old GameSession cannot be resumed |
| `RECONNECT_BINDING_STALE` | `STALE_GENERATION` | `TERMINAL` | contender references an older connection/session/runtime fence |
| `RECOVERY_HEALTHY_INCUMBENT` | `CONFLICT` | `RETRYABLE` | healthy current controller cannot be preempted |
| `RECOVERY_CURRENT_PLACEMENT_UNAVAILABLE` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | game-domain current actor route/owner cannot be established safely |
| `TAKEOVER_BLOCKED_BY_LIFECYCLE` | `CONFLICT` | `RETRYABLE` | combat/PZ/logout/mandatory-presence state blocks intentional takeover |
| `LEASE_FENCE_UNCERTAIN` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | current/replacement lease authority cannot be proven safely |
| `SESSION_INTERNAL_UNAVAILABLE` | `INTERNAL_UNAVAILABLE` | `RETRYABLE` | safe fail-closed unexpected internal session/admission condition |

Public client presentation may intentionally collapse security-sensitive distinctions. Internal evidence must retain the bounded code without exposing credentials or hidden account/character facts.

## 28. Logging, privacy and correlation

Permitted correlation includes, when operationally authorized:

- `AdmissionAttemptRef`;
- GameSessionId;
- HandoffId;
- WorldId/ChannelId;
- bounded runtime/security profile revisions;
- pseudonymous AccountId/CharacterId references;
- typed FND-04 failure code;
- timing/freshness measurements.

Never log or export:

- raw Game Login Ticket;
- raw PreAdmissionGrant;
- raw reconnect proof;
- OAuth access/refresh tokens;
- passwords/MFA/recovery material;
- private signing keys;
- secret lease/fencing material if any implementation introduces it.

Bearer material must be redacted in application logs, traces, proxy logs, crash reports and analytics.

## 29. Versioning, downgrade and rollout

Keep these independent:

- FND-04 semantic contract revision;
- admission security/interchange profile revision;
- Platform account-security revision;
- runtime observation/ownership generation;
- FND-02 protocol major/transport profile/schema revision;
- content/ruleset/runtime revisions;
- admitted-session state-machine implementation revision.

Unknown mandatory security/profile semantics fail closed.

A lower-version consumer cannot ignore a security-critical field and accept the credential as though it were optional.

Native route activation requires an explicit producer/consumer compatibility matrix and independent fixtures for the exact selected Platform producer and Oteryn-v2 consumer revisions.

No unsupported version may fall back to Canary, plaintext, another audience, another Channel or an alternate native route with the same credential.

## 30. Mandatory preimplementation companion gates

FND-04 architecture acceptance does not authorize runtime implementation by itself.

Before any production-capable admission/reconnect implementation claim, the implementation programme must freeze and prove two companion profiles.

### 30.1 Cross-language security/interchange profile

The profile must pin at minimum:

- exact PreAdmissionGrant container/encoding;
- exact signature algorithm/profile and algorithm allowlist;
- exact signed byte semantics/canonicalization rule;
- issuer and audience values;
- purpose values;
- key-id encoding and trusted discovery mechanism;
- AccountId and UUID field encodings;
- `AdmissionAttemptRef` and GrantNonce encoding/entropy requirements;
- exact mandatory claim set and rejection of unknown mandatory semantics;
- grant TTL, bounded wall-clock skew, key overlap/retirement/emergency revocation;
- security-generation/revocation projection freshness bound;
- exact credential-size/depth/count limits;
- independent Go/Rust golden positive and adversarial fixtures;
- cross-version/downgrade tests;
- secret redaction tests.

Application libraries/vendors remain implementation choices after the profile is frozen.

### 30.2 Liveness/lease/abuse parameter profile

This profile must freeze with measured/fault/security evidence:

- liveness probe cadence/hysteresis needed to implement the accepted two-second control-loss boundary without false-positive abuse;
- recovered-stable anti-flap threshold controlling creation of a new loss episode;
- CharacterLease TTL, renewal cadence and safety margin;
- durable fencing/dependency deadlines;
- reconnect-proof cryptographic size/primitive and verifier retention policy;
- admission/recovery/takeover rate limits;
- any maximum retry/reconciliation windows.

The profile may tune these values later through explicit versioned evidence without changing the FND-04 semantic model.

### 30.3 DUR-02 dependency

The first authoritative durable implementation must also consume accepted DUR-02 physical persistence/fencing semantics. FND-04 defines required behavior; DUR-02 defines how PostgreSQL transactions, locks/revisions and crash recovery enforce it.

## 31. Required implementation and E2E proof

A later implementation package must prove, on exact revisions, at least:

1. valid fresh Platform authorization creates exactly one canonical GameSession and one current transport binding;
2. one GrantNonce cannot produce two successful admissions under concurrency;
3. ambiguous Platform issuance with the same AdmissionAttemptRef cannot create multiple independent capabilities;
4. Platform security revision/revocation after issuance rejects the old unconsumed grant within the accepted bounded freshness policy;
5. stale runtime/ownership generation rejects the grant even before nominal expiry;
6. authoritative AccountId<->CharacterId transfer/sale/lifecycle race rejects stale owner admission;
7. two CharacterIds of one account cannot both become authoritative;
8. healthy combat/PZ/logout-locked incumbent cannot be kicked by a second authenticated client;
9. logout-eligible takeover produces one legal replacement authority with no overlap;
10. same-session reconnect preserves GameSessionId and advances exactly one connection_generation;
11. stale transport frames cannot command or advance liveness after rebind;
12. reconnect-proof replay cannot fence the valid winner;
13. lost rebind response recovers only through the accepted reauthenticated recovery path and cannot revive predecessor proof authority;
14. same-session grace expires exactly from `control_loss_declared_at + 15s` under server monotonic timing;
15. ordinary rebind does not manufacture four-second PvE protection;
16. one eligible unexpected-loss episode cannot generate multiple protection windows through reconnect flapping;
17. post-grace same-character recovery attaches a fresh GameSession to the exact existing actor without respawn/reset/duplication;
18. different-character login remains blocked while the old actor is mandatory-present;
19. lease renewal uncertainty never self-grants a replacement writer;
20. stale character lease/runtime generation cannot commit durable mutation;
21. GameNode replacement preserves same GameSession only when FND-02/FND-04 command/session state is reconstructable; otherwise safe fresh-session recovery occurs;
22. Channel->Instance->Channel continuous handoff preserves GameSessionId while maintaining one writer;
23. Channel->Channel transition uses fresh destination authorization and fresh GameSessionId while account presence remains continuous;
24. mixed producer/consumer revisions fail closed without downgrade;
25. logs/traces contain no bearer credential or reconnect secret;
26. Tier-1 native login/admission/reconnect failure scenarios pass before route activation, with Tier-2 native-client evidence for supported client-facing recovery behavior before claiming user-visible readiness.

## 32. Decision summary

FND-04 accepts:

```text
Platform authenticates and authorizes one bounded attempt.
Oteryn-v2 decides final admission.

one AccountId
-> at most one playable/mandatory-presence CharacterId

AccountPresenceClaim
-> survives transport/GameSession loss while actor remains present

CharacterLease + character_lease_generation
-> fences stale character writers

GameSessionId
-> game-owned UUIDv7 created only at successful admission commit

TransportBinding + connection_generation
-> exactly one current concrete controller

FRESH_ENTRY PreAdmissionGrant
-> signed short-lived Platform capability
-> dedicated purpose/issuer/audience
-> account security + route/runtime applicability bound
-> game-domain GrantNonce consumed once

RECOVER_EXISTING_CONTROL
-> distinct reauthenticated recovery purpose
-> current actor placement resolved by game authority

unexpected control loss
-> declared at last sufficient control + 2s
-> stale concrete transport cleanup at +5s from last sufficient control
-> same-GameSession grace expires 15s after declared loss
-> eligible re-entry gets one 4s PvE defensive effect per loss episode

post-grace actor still present
-> old GameSession terminal
-> AccountPresenceClaim remains
-> same CharacterId may attach a fresh GameSession to the exact actor after full revalidation
-> different CharacterId remains blocked

Channel<->Instance continuous handoff
-> may preserve GameSessionId

Channel->Channel session transition
-> fresh destination authorization + fresh GameSessionId

lease expiry/uncertainty
-> never self-grants replacement authority
```

## 33. Consequences

### Positive

- account-global duplicate-session behavior is explicit and combat-safe;
- Platform security authority remains separate from game gameplay authority;
- replay/issuer idempotency/session identity are distinct;
- stale runtime generations cannot resurrect old routes;
- reconnect continuity is preserved without making GameSessionId a bearer token;
- lost reconnect response has a deterministic security-safe recovery path;
- mandatory actor presence is independent from GameSession lifetime;
- Channel/Instance handoffs retain explicit one-writer fencing;
- implementation-sensitive numeric and crypto-library choices remain evidence-driven.

### Costs

- the implementation requires authoritative durable account/lease/session fencing state;
- rare lost-proof/rebind cases require Platform reauthentication;
- cross-placement recovery requires a game-domain route-resolution mechanism;
- Platform and game implementations need an exact cross-language security profile and fixtures;
- DUR-02 must prove the physical transaction/fencing substrate before durable production admission;
- more explicit state is required than a single generic session token/row.

## 34. Non-authorization and next gate effect

When this contract merges, FND-04 semantic architecture is accepted.

It does **not** by itself authorize production-capable runtime implementation. Implementation remains blocked until:

- the section-30 security/interchange profile is accepted;
- the section-30 liveness/lease/abuse parameter profile is accepted;
- required DUR-02 persistence/fencing semantics exist for the implementation slice;
- the implementation task explicitly authorizes Rust/protocol/persistence work;
- exact-head tests/audits/E2E required by the implementation package pass.

After FND-04 architecture acceptance, the foundation programme may advance to the next ordered architecture packages that do not violate these implementation prerequisites.
