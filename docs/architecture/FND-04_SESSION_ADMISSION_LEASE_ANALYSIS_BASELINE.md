# FND-04 — Identity, Game Session, Admission and Character Lease Analysis Baseline

- Status: Architecture analysis; recommendations only until a later accepted FND-04 contract freezes them
- Date: 2026-08-08
- Gate: `FND-04`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Repository: `blakinio/Oteryn-v2`
- Consumes: ADR-0003, ADR-0012, FND-ID-01, FND-02, accepted FND-03, reconnect/duplicate-login/disconnect owner baselines, foundation error vocabulary and foundation failure catalogue
- External reconciliation evidence: `blakinio/Oteryn-Platform@c0b8703d326a04b43ae8e06f6192b0cb91c859b7` remains read-only/reconciliation-only and cannot override later Oteryn-v2 semantics
- Does not authorize: Rust/runtime/protocol implementation, PostgreSQL schema, Platform writes, production key deployment, production traffic, production heartbeat/lease values or live account/session mutation

## 1. Purpose

`FND-04` must turn already accepted identity, protocol, runtime, reconnect and duplicate-login decisions into one coherent security/authority state model.

The minimum distinction is:

```text
Platform authentication
    != authorization to attempt gameplay admission
    != canonical GameSessionId
    != account-global presence exclusion
    != character writer/control lease
    != concrete transport binding
    != runtime scope ownership
```

Collapsing these concepts into one generic "session token" would make replay, reconnect, takeover, combat presence, channel handoff and crash recovery ambiguous.

This baseline is deliberately analysis-only. It recommends a direction, identifies mandatory final-contract decisions, records failure/race constraints and preserves implementation choices that do not need to be frozen yet.

## 2. Accepted constraints — not open for redesign here

### 2.1 Platform and identity

- Oteryn Platform Identity owns reusable account credentials, OAuth/PKCE, MFA, recovery and Game Login Ticket issuance.
- Game Gateway remains a Platform control-plane component responsible for bounded route/offer orchestration, not gameplay state.
- The game-domain Game Session / Admission authority establishes final gameplay admission and issues canonical `GameSessionId` only after successful authoritative admission.
- `GameSessionId` is identity, never a bearer credential.
- Character Authority owns canonical `CharacterId`, authoritative character lifecycle and current `AccountId <-> CharacterId` ownership relation.
- Final admission must revalidate the current game-owned account/character relation; browser/client/Platform projections are not final ownership proof.

### 2.2 Protocol

FND-02 already fixes:

- TCP + TLS 1.3 and ALPN `oteryn-game/1`;
- bounded opaque admission/reconnect material;
- no client-created canonical `GameSessionId` during bootstrap;
- game-issued `GameSessionId` after admission;
- `connection_generation` as a `uint64` fence scoped to one `GameSessionId`;
- generation zero only before admission/resume authority;
- a strictly newer non-zero generation on every accepted transport rebind;
- `(GameSessionId, CommandId)` as command identity/order;
- stale connection generations cannot command, advance liveness or alter reconciliation;
- authenticated `LivenessProbe` / `LivenessAck` primitives;
- bounded snapshot/replay reconciliation after admission/rebind.

FND-04 may define admission/reconnect credential semantics and later register typed messages under FND-02 rules. It may not silently redefine the protocol foundation.

### 2.3 Runtime and timing

Accepted FND-03 semantics remain binding:

- FND-04 decides what current-generation evidence counts as sufficient playable-control/liveness evidence;
- FND-03 timestamps accepted evidence using process-local monotonic time;
- FND-03 executes the accepted `2.0 s` disconnect-protection boundary, `5.0 s` stale concrete-transport cleanup and `4.0 s` defensive PvE re-entry effect;
- stale-generation evidence cannot restore liveness;
- GameNode/runtime-health evidence is considered so local server stalls are not mislabeled as player path loss.

### 2.4 Account, combat and takeover

- one `AccountId` may have at most one authoritative playable/mandatory-presence `CharacterId` at a time;
- a healthy combat/PZ/logout-locked incumbent cannot be kicked, fenced or revoked merely because a second client authenticates;
- a genuinely unavailable incumbent may be recovered through same-character reconnect/recovery;
- a different `CharacterId` remains blocked while the incumbent actor has mandatory world presence;
- reconnect/takeover cannot reset HP/resources/position/conditions/cooldowns/combat/PZ/logout/threat/encounter state or already committed effects;
- graceful logout/intentional takeover does not create disconnect re-entry protection.

### 2.5 Identifier discipline

FND-ID-01 intentionally did not add `AdmissionId` or `CharacterLeaseId`.

FND-04 must prefer existing identities plus scoped state/generation values unless a separately addressable durable semantic entity is proven necessary.

## 3. Decision timing

### 3.1 Must decide before FND-04 final acceptance — YES

The final contract must freeze:

- semantic ownership and state transitions for account presence, character lease/control, GameSession and transport binding;
- fresh-admission linearization point;
- signed/opaque/hybrid pre-admission credential class;
- credential purpose/type, issuer, audience and minimum route/revision bindings;
- one-time/replay-prevention semantics;
- reconnect credential semantics, rotation and replay/lost-response behavior;
- `connection_generation` commit rules;
- account-global exclusion and character lease fencing semantics;
- same-session reconnect, reauthenticated recovery and intentional takeover behavior;
- exact start/end semantics of the accepted 15-second same-GameSession reconnect grace;
- relationship among the accepted 2-second, 5-second, 15-second and 4-second behaviors;
- post-grace same-character recovery while an actor remains in mandatory world presence;
- reconnect routing/current-placement authority;
- Channel↔Instance and Channel↔Channel session-continuity rules;
- key-purpose, discovery, rotation and emergency-revocation invariants;
- failure/error mappings and credential redaction requirements.

### 3.2 Must be concrete before implementation, but need not be guessed in this analysis

The final FND-04 delivery must either freeze or explicitly gate a concrete, versioned **cross-language admission-grant security/interchange profile** before Platform/game implementation begins. That profile must define canonical credential encoding, signature/verification algorithm profile, required claims/fields and independent fixtures.

The exact application libraries, KMS/HSM/vendor products and internal APIs do **not** need to be architecture constants. They remain implementation/security choices as long as they implement the accepted profile and pass the required evidence.

Likewise, the following numeric values require measured/security evidence before implementation acceptance and should not be invented in this analysis:

- pre-admission credential TTL and bounded wall-clock skew;
- verification-key refresh/cache-staleness windows;
- reconnect-secret length/primitive and verifier-retention window;
- liveness-probe cadence/hysteresis inside the accepted two-second behavioral boundary;
- character-lease TTL, renewal cadence and local safety margin;
- admission/reconnect/takeover rate limits.

### 3.3 Deliberately not decided here

- PostgreSQL table/index/locking/isolation representation;
- exact Rust/Go cryptographic/token library;
- KMS/HSM/cloud-vendor product;
- Redis or any other non-authoritative cache product;
- deployment/orchestrator topology;
- client proof-of-possession as a mandatory first-release feature;
- final player-facing UX wording.

### 3.4 Why the semantic model must be correct now

A wrong model could create:

- two playable characters under one account;
- two transports commanding one GameSession;
- stale session/runtime writers overwriting newer authority;
- replayable admission grants;
- second-client combat disconnect abuse;
- character/account release when an uncontrolled actor still exists;
- unsafe automatic writer replacement after lease uncertainty;
- stale Platform/client route data becoming gameplay authority;
- inability to recover an ambiguous admission/rebind safely.

### 3.5 Evidence that may justify supersession

A later contract may reopen a choice only with named evidence such as:

- penetration-test/security-review findings;
- cross-language interoperability or library-maintenance evidence;
- measured key-distribution/introspection latency or availability evidence;
- lease/fencing fault injection revealing a split-owner window;
- reconnect-storm/liveness false-positive evidence;
- player fairness/abuse telemetry;
- a new product requirement for sender-constrained credentials, device binding or stronger takeover authentication.

## 4. Recommended authority model

Use four distinct semantic authority layers plus runtime scope ownership.

```text
A. AccountPresenceClaim
   scope: AccountId
   value: CharacterId or none
   meaning: the account's one current playable/mandatory-presence character

B. CharacterLease
   scope: CharacterId
   binds: AccountId + character_lease_generation + current authoritative actor/runtime context
   meaning: exclusive current character writer/control fence consumed by session/runtime/persistence rules

C. GameSession
   identity: GameSessionId
   binds: AccountId + CharacterId + current CharacterLease + current world/placement/revisions
   meaning: one logical player-control lifecycle

D. TransportBinding
   scope: GameSessionId
   fence: connection_generation
   meaning: one current concrete transport may submit gameplay/liveness/reconciliation authority

E. RuntimeScopeAuthority
   scope: current ChannelRuntime/InstanceRuntime ownership generation
   meaning: the current simulation owner executes authoritative actor mutation
```

These may be co-located physically but must remain semantically distinct.

### 4.1 Why AccountPresenceClaim is separate

After the 15-second same-session reconnect grace expires, the old GameSession may become terminal while combat/PZ/logout rules still require the actor to remain in the world.

Therefore:

```text
GameSession terminal
!= actor absent
!= AccountId free for another CharacterId
```

`AccountPresenceClaim` remains held while the actor is `PRESENT_CONTROLLED` or `PRESENT_UNCONTROLLED`, and is released only after an authoritative lifecycle transition proves the actor is legally absent/replaced.

Socket close, GameSession terminality, client process death or reconnect-grace expiry do not release it by themselves.

### 4.2 Why CharacterLease is separate from GameSession

Server-driven simulation may continue to apply damage, death, conditions or other authoritative effects after player control is lost. A character writer fence therefore cannot be equivalent to “current TCP connection exists.”

The lease remains tied to the authoritative actor/runtime lifecycle and can outlive one transport or, where post-grace recovery is accepted, one GameSession.

Player-originated durable mutations must still be attributable to a current accepted GameSession/command context in addition to a current character/runtime fence; reusing the same character lease does not resurrect a terminal GameSession.

### 4.3 Character Authority remains authoritative

The lease protocol does not transfer ownership of the character aggregate away from Character Authority.

Recommended responsibility split:

- Character Authority remains semantic owner of CharacterId, character state and AccountId↔CharacterId ownership;
- Game Session / Admission authority coordinates player-session admission, account exclusion and lease transitions;
- ChannelRuntime/InstanceRuntime owns current simulation under FND-03 scope ownership;
- DUR later defines physical persistence/fencing/transaction enforcement.

## 5. Recommended scoped fencing values

### 5.1 Account presence revision/generation

A current account-presence claim requires a stale-safe revision/fence scoped to `AccountId`.

Recommended semantics:

- non-zero monotonic `uint64` generation or equivalent linearizable CAS revision;
- advances when a released/replaced account-presence claim is superseded;
- stale admission/takeover work cannot replace a newer claim;
- continuous presence of the same CharacterId may retain the claim across transport reconnect or a fresh GameSession attached to the same actor;
- no wrap/reuse; exhaustion fails safe.

This value does not need to become a public/wire foundation identifier.

### 5.2 `character_lease_generation`

Recommended scope:

```text
CharacterId + character_lease_generation
```

Recommended semantics:

- non-zero monotonic `uint64` class-4 fence;
- stale session/runtime/durable writes from an older lease generation fail closed;
- may survive transport reconnect and may survive GameSession replacement while the same authoritative actor/lease remains current;
- advances when authoritative character writer ownership is actually replaced/recovered such that a former holder must be fenced;
- no wrap/reuse; exhaustion fails safe.

### 5.3 `connection_generation`

FND-02 already fixes its type/scope.

Recommended FND-04 commit semantics:

- initial admitted transport uses generation `1`;
- each accepted same-GameSession rebind commits exactly one strictly newer value;
- simple `current + 1` is preferred unless implementation evidence proves a different allocator is required;
- a rejected/uncommitted rebind does not change current authority;
- no wrap/reuse; exhaustion makes that GameSession terminal.

### 5.4 No new AdmissionId / CharacterLeaseId

Current analysis finds no need for either durable entity identity.

Use:

- a credential-scoped cryptographically random grant nonce for admission replay/correlation;
- GameSessionId for admitted logical sessions;
- scoped account/character generations for exclusion/fencing.

A later narrow FND-ID amendment remains possible only if detailed crash/recovery design proves a separately addressable durable entity is necessary.

## 6. Fresh admission linearization

Recommended conceptual sequence:

```text
Platform Identity
    -> one-time Game Login Ticket
Game Gateway
    -> redeem ticket
    -> choose authorized entry route / protocol / revisions
    -> obtain dedicated PreAdmissionGrant
Game Session / Admission authority
    -> validate grant type/signature/issuer/audience/time/bindings
    -> validate current route/revisions/scope authority
    -> revalidate AccountId currently owns CharacterId
    -> evaluate account presence / duplicate-login state
    -> establish/transition AccountPresenceClaim
    -> establish current CharacterLease
    -> generate candidate GameSessionId
    -> atomically commit:
         one-time grant consumption
         account-presence fence/revision
         character lease generation/state
         logical GameSession
         connection_generation = 1
         initial reconnect credential verifier/state
    -> only then publish admission success
    -> establish FND-02 initial snapshot boundary
```

A generated candidate GameSessionId is not canonical until the admission commit succeeds. An uncommitted candidate is discarded and never reused.

### 6.1 Required atomic invariant

Fresh admission must behave as one linearizable authority transition even if DUR later maps it across multiple physical records:

```text
consume grant
+ prove account exclusion
+ prove character writer authority
+ create exactly one GameSession
+ create first current transport binding
= one externally unambiguous admission result
```

No client-visible success may precede this point.

## 7. Pre-admission credential alternatives

### 7.1 Opaque + online introspection/consume

Benefits:

- immediate central revocation;
- hidden claims;
- centralized one-time state.

Costs:

- every normal admission depends synchronously on a Platform-side service;
- higher coupling of game admission latency/availability to Platform;
- more ambiguous failure after Game Login Ticket redemption.

### 7.2 Pure self-contained signed credential

Benefits:

- local cryptographic verification by game nodes/admission service;
- easy horizontal verification.

Costs:

- signature validity alone is not one-time consumption;
- replay and emergency revocation need additional state/policy;
- token/key/audience confusion is security-sensitive.

### 7.3 Hybrid signed capability + game-domain one-time consumption

Benefits:

- no synchronous Platform introspection on the normal verification path;
- cryptographically authenticated issuer/audience/route/revision bindings;
- replay state is explicit and atomically tied to game admission;
- Platform remains authorization issuer while game remains final admission/session authority.

Costs:

- requires key-distribution/revocation plus bounded game-domain grant-consumption state;
- requires a versioned cross-language security/interchange profile and independent fixtures.

### 7.4 Recommendation

Use **hybrid signed PreAdmissionGrant + authoritative game-domain one-time consumption** as the final-contract direction unless a bounded security/availability evaluation disproves it.

This recommendation selects the semantic class, not a token library.

## 8. Grant issuer and key-purpose separation

Recommended trust model:

```text
Platform Identity credentials / Game Login Ticket
    -> Platform Identity purpose/keys

PreAdmissionGrant
    -> dedicated Platform-owned Game Admission Grant issuer/key purpose
    -> invoked by Gateway flow

Reconnect credential
    -> game-domain secret/verifier purpose
```

The Gateway should orchestrate grant issuance without obtaining broad reusable-account signing authority merely because it handles routes.

The game side receives verification capability, never Platform signing secrets.

Any later signed-container profile must enforce strict algorithm/profile allowlisting, explicit credential typing, trusted issuer/audience validation and mutually exclusive validation rules for distinct credential purposes. If JWT/JWS is selected, RFC 8725-style BCP controls are the minimum reference direction; the exact library remains an implementation choice.

## 9. PreAdmissionGrant minimum semantics

The grant must authenticate at minimum:

- explicit credential type and contract/security-profile revision;
- issuer;
- exact admission audience/purpose;
- AccountId;
- CharacterId;
- WorldId;
- fresh-entry ChannelId/route binding where the grant purpose is fresh entry;
- protocol major + transport profile;
- required ruleset/content/map/world-policy/offer compatibility revisions when those are authoritative gates;
- issue/not-before/expiry under bounded trusted wall-clock skew;
- cryptographically random one-time grant nonce;
- allowlisted signing-key identifier/version;
- route/offer revision where required to prevent stale offer substitution.

It must not contain or imply:

- canonical GameSessionId;
- connection_generation;
- character lease authority;
- NodeId as a substitute for current scope ownership;
- reusable account/OAuth credentials;
- arbitrary token-selected key/JWK URLs;
- client-selected issuer/audience/routing authority.

### 9.1 Credential purposes must not be interchangeable

If final FND-04 accepts a Platform-reauthenticated same-character recovery path, **fresh entry** and **same-character recovery** must use explicit, non-confusable validation purposes.

A fresh-entry grant bound to ChannelId must never be interpreted as permission to move an already-present actor to that channel.

A recovery-purpose credential may authenticate AccountId/CharacterId/WorldId intent, but the game domain must resolve the actor's **current authoritative placement** and session/lease state. Stale client/Platform placement is never authority.

The final contract must define how the client is routed to that current authority without making Platform the owner of gameplay/session state.

## 10. Validation order

Recommended fail-closed order:

1. FND-02 material/frame size and structural bounds;
2. explicit credential type/security-profile revision;
3. allowlisted issuer/key selection and cryptographic verification;
4. issuer/audience/time/skew validation;
5. protocol/route/revision binding validation appropriate to the credential purpose;
6. grant nonce replay/consumption eligibility;
7. authoritative AccountId -> CharacterId ownership revalidation;
8. current account-presence/duplicate-login eligibility;
9. current character lifecycle + current runtime placement/scope authority validation;
10. atomic admission/lease/GameSession transition.

No failure before step 10 may leave player-control authority or a partially active GameSession.

## 11. One-time grant replay and retention

### 11.1 Consumed means consumed

One PreAdmissionGrant may create at most one successful admission transition.

Replay of an already consumed grant must not create another GameSession, lease or transport binding and maps to `SESSION_REJECTED` with a stable narrower final code.

### 11.2 Consumption key and retention

The replay key should be semantically at least:

```text
trusted issuer/security-profile + grant nonce
```

Consumption evidence must remain authoritative for at least the full period during which the grant could still pass time/skew validation. Evicting replay evidence while the credential can still be accepted is forbidden.

Exact retention/storage is later DUR/security implementation work.

### 11.3 Lost admission response

If the admission commit succeeded but `ServerAccepted` was lost, the original consumed grant does not become reusable.

Recommended recovery:

1. client obtains a fresh Platform/Gateway authorization/grant;
2. game-domain admission re-evaluates current state;
3. if an existing same-character GameSession is genuinely eligible for the accepted reauthenticated-recovery path, recover it through that distinct path;
4. otherwise return the bounded current state or a fresh-session path as permitted;
5. never blindly create a second actor/session.

## 12. Reconnect credential recommendation

Use a **game-domain-issued high-entropy opaque rotating reconnect secret** rather than GameSessionId or a Platform token.

Required semantic properties:

- bound to exactly one current GameSession and reconnect-credential state/generation;
- sent only over accepted TLS;
- stored server-side as a verifier/digest or equivalent secret-safe representation where practical;
- never logged or exported to analytics;
- rotated when a new transport binding successfully commits;
- predecessor cannot authorize an unrelated/later rebind;
- replay fails closed without changing current authority;
- terminal GameSession invalidates reconnect authority.

### 12.1 Lost rebind response is a mandatory final-contract decision

A simple rotate-and-forget design is insufficient if the server commits generation `N+1` and a new reconnect secret but the response is lost: the client still knows only the predecessor.

The final contract must choose a bounded idempotent reconciliation mechanism that never restores generation `N` and never makes the old reconnect secret generally valid again. Possible implementation shapes include a narrowly scoped predecessor verifier tied to the exact committed rebind attempt or a reauthenticated recovery path. This baseline does not freeze the storage shape.

### 12.2 Proof-of-possession extension

Sender-constrained reconnect/admission credentials could later reduce bearer-secret replay risk. The first native vertical slice does not require a client-held PoP key by architecture because that introduces key storage/recovery complexity; the final profile should remain extensible if threat evidence later justifies it.

## 13. GameSession state model

Recommended logical states:

```text
ACTIVE
CONTROL_SUSPECTED
RECONNECTABLE
TAKEOVER_DRAINING
TERMINATING
TERMINAL
```

### ACTIVE

- one current non-zero connection_generation;
- accepted sufficient-control evidence is current;
- player commands may be accepted subject to protocol/runtime/gameplay rules.

### CONTROL_SUSPECTED

- sufficient-control evidence is late but the session has not crossed the accepted loss boundary;
- no new GameSession is created;
- player-visible protection follows FND-03 timing, not the state name alone.

### RECONNECTABLE

- unexpected playable-control loss is server-authoritatively classified;
- old concrete transport may be closed independently;
- logical GameSession is eligible for same-session recovery within its grace;
- actor state is preserved.

### TAKEOVER_DRAINING

- an intentional authenticated newcomer transition is allowed only because incumbent state is logout-eligible;
- incumbent remains the only current authority until the explicit fence/logout boundary;
- no second player-control session becomes authoritative before that boundary.

### TERMINATING

- no new ordinary player commands;
- the session progresses to one bounded terminal outcome while preserving required world-presence semantics.

### TERMINAL

- GameSessionId can never regain authority;
- later player control requires a fresh GameSessionId;
- terminality does not imply actor absence.

## 14. Actor/account-presence state

Keep actor presence distinct from GameSession state:

```text
ABSENT
PRESENT_CONTROLLED
PRESENT_UNCONTROLLED
```

`PRESENT_UNCONTROLLED` means the actor remains authoritative in world simulation while no playable controller/GameSession is currently usable.

`AccountPresenceClaim` remains held in `PRESENT_CONTROLLED` and `PRESENT_UNCONTROLLED`.

This is the required model for combat-X-log and post-grace actor continuity.

## 15. Sufficient-control/liveness evidence

### 15.1 Primary evidence

Recommend a valid **current-generation response to a recent server-issued authenticated liveness probe** as the primary evidence.

It must be tied to:

- current GameSessionId;
- current connection_generation;
- current probe ID;
- server-observed receipt/progress;
- server-side runtime-health context.

Client wall-clock timestamps, self-declared lag/disconnect flags, socket-open state and stale-generation acknowledgements never count.

### 15.2 Other traffic

Gameplay-command silence is never disconnect evidence.

The final contract may enumerate other current-generation **bidirectional** control exchanges as equivalent sufficient-control evidence only when they prove the same property as a probe round-trip. Arbitrary inbound bytes or one-way commands must not reset the loss timer.

### 15.3 Cadence

Healthy idle evidence must be refreshable comfortably within the accepted 2-second behavior boundary. Exact cadence/hysteresis must be selected from bounded latency/load/fault evidence before implementation acceptance; this analysis does not guess a production number.

## 16. Reconnect grace timing recommendation

The accepted initial same-GameSession reconnect grace is 15 seconds.

Recommend:

```text
last_sufficient_control_at = T0
control_loss_declared_at   = T0 + 2.0 s
stale_transport_cleanup    = T0 + 5.0 s
same_session_grace_expires = control_loss_declared_at + 15.0 s
```

This gives a full 15-second logical grace after server-authoritative loss classification while keeping the 5-second concrete-transport cleanup independent.

The final contract must explicitly accept/reject this composition.

### 16.1 Rebind before the 2-second loss boundary

A legitimate transport replacement may complete before `control_loss_declared_at` when the old transport is otherwise proven stale/lost.

Such a rebind may preserve the same GameSession and advance connection_generation, but **does not receive the 4-second defensive PvE re-entry effect merely because a connection generation changed**.

Re-entry protection requires a server-classified eligible unexpected loss-of-playable-control episode under the accepted owner policy.

## 17. Fast same-GameSession reconnect

Inside the accepted same-session window, recommend:

```text
existing GameSessionId S
+ same AccountId/CharacterId ownership still valid
+ session not terminal/revoked
+ current CharacterLease compatible
+ accepted reconnect proof valid
+ old transport proven stale/lost
+ current runtime placement/revisions valid
    -> atomically commit newer connection_generation
    -> rotate/reconcile reconnect credential
    -> old generation immediately loses authority
    -> same GameSessionId continues
```

The rebind commit is the authority linearization point. TLS establishment alone is not authority.

If and only if this rebind belongs to an eligible **classified unexpected control-loss episode**, FND-03 activates the accepted 4-second defensive PvE re-entry effect. A routine transport replacement or pre-loss-boundary rebind does not manufacture protection.

### 17.1 Concurrent reconnect race

Two reconnect attempts using the same current proof/secret must not both become successful current bindings.

The first accepted linearized rebind replaces current authority and rotates/reconciles the reconnect proof; later contenders observe stale/consumed state and fail without fencing the winner.

### 17.2 GameNode replacement

Same-GameSession recovery across process/GameNode replacement may be claimed only when the system can preserve or safely reconstruct all state required by FND-02/FND-03/FND-04, including:

- GameSession terminal/current state;
- current connection_generation;
- reconnect-proof state;
- CommandId high-water/pending/result reconciliation state;
- required server-sequence/snapshot boundary;
- account/character lease fences.

If that cannot be proven, the old GameSession terminates safely and recovery uses a fresh-session path. Runtime convenience may not weaken no-double-execution or stale-generation guarantees.

## 18. Platform-reauthenticated same-character recovery

A fresh Platform/Gateway credential can prove current account/character authorization but cannot by itself preempt a healthy session.

### 18.1 Healthy incumbent

- no forced reconnect/takeover shortcut;
- healthy combat/PZ/logout-locked incumbent remains fully authoritative;
- logout-eligible intentional takeover follows the explicit takeover flow and produces a fresh GameSession after old authority ends.

### 18.2 Server-proven control loss inside same-session grace

A reauthenticated same-character recovery path is useful when the client lost/never received reconnect material, but it introduces routing and account-compromise trade-offs.

Recommendation: allow this path in the final contract **only if** it uses an explicit recovery credential purpose, current server-side session/actor state, current authoritative placement resolution and any Platform-owned step-up/risk policy required by security review.

It must not reinterpret an ordinary fresh-entry ChannelId grant as permission to move the actor.

A successful reauthenticated recovery still commits a single newer connection_generation on the existing GameSession; it does not create a parallel GameSession.

After one recovery succeeds and the session returns to healthy `ACTIVE`, concurrent/late recovery contenders fail rather than repeatedly steal control.

## 19. Post-grace same-character recovery while actor remains present

When same-session grace expires:

```text
old GameSessionId -> TERMINAL
```

If combat/PZ/logout rules still require the actor in world:

```text
actor                  -> PRESENT_UNCONTROLLED
AccountPresenceClaim   -> remains held for the same CharacterId
CharacterLease/runtime -> remains under the current authoritative actor path
```

A later fully authenticated admission for the **same CharacterId** may create a fresh GameSessionId and attach control to that exact existing actor if the final contract accepts and proves:

- old GameSession terminality;
- no current playable controller;
- current AccountId still owns CharacterId;
- account presence still points to that CharacterId;
- current CharacterLease/runtime scope is valid;
- current actor placement is resolved from game-domain authority, not stale client/Platform route data;
- protocol/ruleset/content/current-placement revisions are compatible.

This path must not respawn, teleport, duplicate, heal or reset the actor. The new GameSession starts its own connection_generation namespace at `1`.

### 19.1 Re-entry protection across GameSession replacement

Protection eligibility must be actor/control-loss-episode state, not a side effect of GameSession identity.

If this fresh session is the first valid re-entry for the same eligible unexpected loss episode, FND-03 may apply the accepted 4-second protection. Replacing GameSessionId must not erase, restart or duplicate an already-consumed protection decision.

A small actor-scoped control-loss episode revision/state may be used internally; no new foundation entity ID is justified by this analysis.

### 19.2 Different CharacterId

A different character remains blocked while the incumbent actor has mandatory world presence. AccountPresenceClaim is released only after the authoritative actor lifecycle reaches legal absence/removal.

## 20. Intentional duplicate-login takeover

### 20.1 Healthy combat/PZ/logout-locked incumbent

Binding accepted behavior:

```text
new login authenticated
-> do not fence incumbent
-> do not close incumbent transport
-> do not revoke incumbent session
-> do not admit another CharacterId
-> incumbent remains authoritative
```

Return a bounded `CONFLICT`/session outcome without leaking unnecessary combat/session detail.

### 20.2 Healthy logout-eligible incumbent

Recommended transition:

1. authenticate/authorize newcomer;
2. prove takeover eligibility;
3. move incumbent GameSession to `TAKEOVER_DRAINING`;
4. establish a committed fence boundary for new incumbent commands;
5. complete legal logout/removal or an accepted same-character/channel handoff boundary;
6. update/release account/character authority as appropriate;
7. create a fresh GameSessionId for the newcomer only after old player authority is gone;
8. do not grant disconnect/re-entry protection for this intentional transition.

No interval may contain two player-controlled authoritative characters.

### 20.3 Races

Concurrent takeover/fresh-admission requests serialize through the account-presence claim/revision and character/session state. A stale contender cannot revoke a newer healthy winner.

## 21. Account-global exclusion and character lease atomicity

Fresh admission must atomically prove:

```text
AccountId may hold this CharacterId as its one playable/mandatory-presence character
AND
CharacterId may hold the current accepted character writer/control lease
```

Two different CharacterIds for the same AccountId racing across worlds/channels cannot both succeed.

Recommended semantic acquisition order:

```text
account presence/exclusion
-> character lease/fence
-> GameSession
-> transport binding
```

DUR may optimize the physical transaction, but no partial authority may become externally visible.

## 22. Character lease lifecycle

### 22.1 Ownership

Character Authority remains owner of the character aggregate. FND-04 defines the lease/exclusion protocol by which Game Session/Admission and runtime obtain current player-control/writer authority. DUR/OPS later provide physical persistence/renewal/recovery mechanisms.

### 22.2 Acquire

- after credential/route/current-ownership checks;
- only with compatible current AccountPresenceClaim;
- generation/state transition is atomic with the authoritative admission/handoff path;
- stale former generation cannot commit after replacement.

### 22.3 Renew

- only current authorized lease/runtime/session path may request renewal;
- a sent renewal request is not proof of renewal;
- authoritative lease-store time/state, not client time, determines acceptance;
- current generation remains explicit.

### 22.4 Safety margin

The holder must stop relying on the lease before another holder could legitimately acquire a newer generation.

A local safety deadline shorter than authoritative expiry is recommended. Exact TTL/cadence/margin require bounded fault-injection/security evidence before implementation acceptance.

### 22.5 Renewal uncertainty / expiry

Lease expiry or renewal uncertainty is **not automatic replacement authority**.

When current lease authority cannot be proven safely:

- new player commands fail closed at the appropriate session/runtime boundary;
- old-generation durable writes are prohibited;
- the affected state moves toward suspected/fenced/recovery semantics;
- AccountPresenceClaim is not released merely because renewal is uncertain while an actor may still exist;
- no newer CharacterLease generation is granted until the old writer/runtime authority is safely fenced/recovered under the accepted DUR/OPS contract;
- server-driven in-memory actor simulation may continue only under an independently current runtime-scope authority and only in a form that cannot race a newer character writer; durable reconciliation remains a DUR decision.

This avoids a split writer while also preventing lease failure from becoming a combat-escape primitive.

### 22.6 Release

Clean lifecycle transitions explicitly release where possible, but correctness cannot depend on a crashed holder sending release. Fencing/recovery must handle omission safely.

## 23. Key discovery, rotation and emergency revocation

The final contract/security profile should require:

- asymmetric verification on the game side;
- dedicated admission-grant key purpose;
- allowlisted issuer/key-set source;
- no arbitrary key URL from untrusted token headers;
- stable key identifier/version;
- bounded current/retiring-key overlap only for still-valid grants;
- grant expiry remains binding even if verification key is retained;
- emergency revocation can invalidate a compromised key before normal overlap ends;
- cached-key staleness is bounded and has explicit fail behavior;
- exact current/retiring/revoked-key fixtures and rollout/rollback tests before production.

Exact KMS/HSM/provider/library remains outside architecture unless later evidence makes it contract-relevant.

## 24. Route and current-placement binding

Fresh entry and recovery have different routing semantics and must not be confused.

### 24.1 Fresh entry

A fresh-entry grant may bind exact `WorldId + ChannelId + route/offer revision`. The selected GameNode must additionally prove that it currently holds accepted scope authority; possession of a grant does not create that ownership.

Wrong/stale route, wrong world/channel, incompatible protocol/revisions or non-current scope ownership fail closed. The same credential is never silently downgraded or moved to Canary/another channel.

### 24.2 Existing actor/session recovery

An existing actor's **current game-domain placement** is authority.

A stale client/Platform ChannelId cannot move that actor to another channel or pull it out of an InstanceRuntime.

The final FND-04 contract must define a bounded session/recovery route-resolution mechanism that can direct an authenticated reconnect/recovery attempt to the current authoritative owner without making Platform the owner of GameSession/actor state. A read-only/authorized route projection or game-domain resolver is acceptable conceptually; its exact API/deployment is later design.

Until this routing is defined, reauthenticated recovery across changed runtime placement remains a conditional recommendation rather than an implementation-ready claim.

## 25. Channel/instance handoff interaction

FND-04 consumes FND-03 handoff execution and FND-ID HandoffId semantics.

Recommended session rules:

- one `HandoffId` identifies one ownership transition lifecycle;
- account presence, CharacterId, GameSessionId and current source/destination generations remain explicit bindings;
- source and destination may prepare concurrently, but only one current gameplay owner may accept player mutation;
- Channel -> Instance and Instance -> Channel may preserve the same GameSessionId when the accepted gameplay transition is one continuous logical player-control session;
- an accepted Channel -> Channel transition that establishes a fresh logical GameSession uses a fresh GameSessionId;
- account presence for the same CharacterId remains continuously held through a legal same-character handoff, avoiding a race window for another CharacterId;
- source player authority is not released until the destination commit/fence is established;
- stale/replayed HandoffId/generation cannot commit;
- failure before commit preserves/recovers source-safe authority; failure after commit recovers from destination authority evidence.

The final contract must state exactly which transition families preserve versus replace GameSessionId.

## 26. Disconnect/re-entry abuse boundary

Re-entry protection is not a reward for changing sockets or connection_generation.

The final contract must ensure:

- eligibility belongs to a server-classified unexpected playable-control-loss episode;
- graceful logout/intentional takeover never creates it;
- rebind before loss classification does not manufacture it;
- stale/replayed reconnect attempts cannot restart it;
- GameSession replacement during the same loss episode cannot duplicate/restart it;
- concurrent recovery contenders cannot each create protection windows;
- repeated later genuine loss episodes remain observable/auditable under Game Intelligence policy.

This analysis introduces no punishment, disconnect cooldown or automatic sanction.

Whether a minimum healthy-control interval/hysteresis is required before a new loss is treated as a distinct protection-eligible episode remains a final FND-04/product decision backed by abuse/fault evidence.

## 27. Candidate foundation failure dispositions

These are analysis recommendations. The final FND-04 contract must use the exact normative status vocabulary from `FOUNDATION_FAILURE_SCENARIOS.md`.

| Scenario | Candidate classification | FND-04 requirement |
|---|---|---|
| `FS-PLATFORM-UNAVAILABLE` | `PASS` | no alternate credential authority; new Platform-dependent admission fails/holds boundedly; already valid gameplay/fast reconnect does not become invalid merely because Platform is unavailable |
| `FS-GATEWAY-AFTER-REDEEM` | `PASS` | redeemed ticket is not silently reused/downgraded; fresh bounded authorization flow; no GameSession without game admission commit |
| `FS-POSTGRES-UNAVAILABLE` | `DEFERRED_BY_ACCEPTED_GATE` | DUR owns physical dependency; FND-04 forbids admission/lease transition when required atomic/fenced authority cannot be proven |
| `FS-LEASE-RENEW-TIMEOUT` | `PASS` | old player/durable authority stops before newer lease writer may commit; expiry alone never self-grants replacement |
| `FS-DUPLICATE-LOGIN` | `PASS` | account-global exclusion + lease/session fencing; healthy combat-locked incumbent cannot be kicked |
| `FS-STALE-GENERATION` | `PASS` | stale connection/account/lease/runtime generation cannot command/renew/reconnect/commit |
| `FS-DUPLICATE-COMMAND` | `NOT_APPLICABLE` | FND-02 remains authority once a GameSession exists |
| `FS-CHANNEL-SPLIT-OWNER` | `DEFERRED_BY_ACCEPTED_GATE` | FND-03/OPS own scope fencing; FND-04 validates current target scope before admission/handoff |
| `FS-CHANNEL-DRAIN` | `DEFERRED_BY_ACCEPTED_GATE` | FND-03/OPS own drain; no admission/handoff into non-open destination |
| `FS-QUEUE-SATURATION` | `DEFERRED_BY_ACCEPTED_GATE` | FND-03/resource limits own runtime queues; FND-04 transitions remain bounded and fail before partial authority |
| `FS-SLOW-CLIENT` | `DEFERRED_BY_ACCEPTED_GATE` | FND-02/FND-03 own bounded resync/transport behavior; FND-04 owns resulting logical reconnect eligibility |
| `FS-CLOCK-SKEW` | `PASS` | credentials use bounded trusted wall-clock skew; liveness/reconnect durations use server-monotonic state, never client time |
| `FS-KEY-ROTATION` | `PASS` | bounded current/retiring overlap + emergency revocation; no acceptance outside credential/key policy |
| `FS-REVISION-MISMATCH` | `PASS` | fail closed; no implicit downgrade/mixed authoritative state |
| `FS-SNAPSHOT-DELTA-MISMATCH` | `NOT_APPLICABLE` | FND-02/FND-03 reconciliation after admission/rebind |
| `FS-DB-OUTBOX-BOUNDARY` | `DEFERRED_BY_ACCEPTED_GATE` | DUR/ANL own physical atomic evidence; FND-04 cannot announce success before required durable admission/session commit |
| `FS-WORLD-BUNDLE-CORRUPT` | `NOT_APPLICABLE` | invalid world activation must already make target unroutable |
| `FS-CLIENT-CUTOVER-ROLLBACK` | `NOT_APPLICABLE` | historical migration lifecycle |
| `FS-ANALYTICS-TELEMETRY-OVERFLOW` | `NOT_APPLICABLE` | telemetry never becomes admission/session authority |
| `FS-AUDIT-OUTBOX-BACKLOG` | `DEFERRED_BY_ACCEPTED_GATE` | ANL/DUR own required audit backlog semantics; security-required takeover/lease audit cannot silently downgrade |
| `FS-EVENT-DUPLICATE-DELIVERY` | `NOT_APPLICABLE` | analytics replay cannot alter session authority |
| `FS-EVENT-OUT-OF-ORDER` | `NOT_APPLICABLE` | analytics order cannot alter session authority |
| `FS-AUDIT-MUTATION-MISMATCH` | `DEFERRED_BY_ACCEPTED_GATE` | ANL/DUR own atomic audit/mutation boundary |
| `FS-ANALYTICS-PRIVACY-POLICY` | `NOT_APPLICABLE` | credentials/secrets are never analytics payloads |
| `FS-DETECTOR-FALSE-POSITIVE` | `NOT_APPLICABLE` | analytics cannot revoke/sanction autonomously |
| `FS-INVESTIGATION-MUTATION-ATTEMPT` | `NOT_APPLICABLE` | investigation cannot mutate session/runtime authority |

### 27.1 Recommended new scenario IDs for final FND-04 delivery

Two FND-04-specific races are important enough to deserve explicit stable E2E/audit names rather than hiding under broader categories:

- `FS-ADMISSION-GRANT-REPLAY` — consumed/expired/wrong-bound PreAdmissionGrant is replayed; no second admission/lease/GameSession results.
- `FS-RECONNECT-CREDENTIAL-REPLAY` — predecessor/stolen/stale reconnect proof races a valid rebind; only one current generation wins and loser cannot fence winner.

The analysis does not edit the shared catalogue; the final FND-04 package should add them if independent review agrees.

## 28. Error mapping direction

| Condition | Foundation category |
|---|---|
| malformed admission/reconnect input | `INVALID_INPUT` |
| bad cryptographic proof or untrusted issuer/key purpose | `AUTHENTICATION_FAILED` |
| expired/consumed/replayed/wrong-audience/wrong-purpose credential | `SESSION_REJECTED` |
| incompatible protocol/ruleset/content/route/security-profile revision | `UNSUPPORTED_REVISION` |
| stale connection/account/character/runtime generation | `STALE_GENERATION` |
| healthy incumbent/current lifecycle blocks takeover | `CONFLICT` |
| registered admission/session bound reached | `CAPACITY_EXCEEDED` |
| required Platform/key/lease/persistence authority unavailable | `DEPENDENCY_UNAVAILABLE` |
| named bounded admission/reconnect/takeover deadline expires | `TIMEOUT` |
| explicit safe cancellation before commit | `CANCELLED` |
| unexpected state where current authority cannot be proven | `INTERNAL_UNAVAILABLE` |

Final numeric/narrow codes must not expose raw SQL errors, secret/token contents, verifier hashes, stack traces or sensitive combat/session details.

## 29. Security and privacy requirements

The final contract must require:

- no reusable account password/OAuth bearer credential sent to gameplay server;
- strict credential type/key-purpose/audience separation;
- no arbitrary token-supplied verification endpoint;
- admission/reconnect credentials redacted from logs/traces/crash reports/analytics;
- GameSessionId alone never authorizes control;
- authoritative AccountId/CharacterId ownership revalidation;
- no client timestamp as expiry/liveness authority;
- bounded replay records and credential material size;
- bounded admission/reconnect/takeover attempts before production;
- auditable takeover/lease-replacement/security-terminal outcomes;
- no high-cardinality session/player IDs as ordinary metrics labels;
- diagnostic correlation may include IDs/generations only under privacy/access policy and never include credentials.

## 30. Player-facing consequences

The recommended model aims for:

- fast same-session recovery after genuine brief loss;
- no session death just because the concrete socket closes at five seconds;
- no combat escape by changing characters;
- no malicious second-client kick of a healthy combat-locked player;
- same-actor recovery even after old GameSession terminality when mandatory presence continues;
- no hidden heal/teleport/reset on reconnect;
- one clear current controller rather than ambiguous dual login;
- no protection farming from routine socket/generation churn.

Main player-risk areas requiring later evidence are false-positive loss detection, reconnect storms and repeated-loss protection abuse.

## 31. Producer and operations consequences

The model allows:

- Platform/Gateway outage to block new authorization without automatically killing valid active gameplay;
- fast reconnect using game-domain proof without requiring Platform availability;
- signed fresh-admission grants to be verified without synchronous Platform introspection on every normal admission;
- Platform key rotation independent from game deployments when compatible trust sets are staged;
- game-domain account/character fencing to remain authoritative;
- GameSession identity to remain independent of NodeId/current process placement.

Costs are explicit replay state, key-distribution/revocation operations, account-global exclusion, lease recovery and significantly more race/fault tests than a legacy password login server.

## 32. Required final FND-04 decisions

Before FND-04 can be called complete, the final architecture package must explicitly resolve:

1. accept/reject the hybrid signed + one-time-consume PreAdmissionGrant direction;
2. freeze or reference a concrete versioned cross-language grant security/interchange profile before implementation; do **not** freeze a library/vendor without evidence;
3. freeze concrete grant TTL/skew/key-overlap/cache-staleness values from security/rollout evidence;
4. freeze reconnect-secret security profile and the lost-rebind-response reconciliation rule;
5. accept/reject Platform-reauthenticated same-GameSession recovery and its anti-account-takeover/step-up policy boundary;
6. define reconnect/current-placement route resolution without making Platform session authority;
7. accept/reject fresh-GameSession attachment to `PRESENT_UNCONTROLLED` same-character actor after grace expiry;
8. accept/reject same-session grace expiry at `control_loss_declared_at + 15 s`;
9. define liveness-probe cadence/hysteresis through a bounded preimplementation evidence gate if not frozen directly;
10. define character-lease TTL/renewal/safety-margin evidence gate and exact replacement/fencing semantics;
11. decide actor-scoped control-loss anti-flap/hysteresis required to prevent duplicate 4-second protection windows;
12. add or reject the two proposed stable replay failure-scenario IDs;
13. freeze exact Channel↔Instance versus Channel↔Channel GameSession continuity and account-presence continuity;
14. freeze stable admission/reconnect/takeover error codes/presentation classes;
15. define same-GameSession recovery requirements across GameNode replacement, with mandatory fresh-session fallback when required FND-02/FND-04 state cannot be reconstructed.

## 33. Recommended concise model

```text
Platform reusable credentials
-> Platform Identity only

Game Login Ticket
-> one-time Platform authorization
-> Gateway redemption/routing

PreAdmissionGrant
-> dedicated Platform-owned admission issuer/key purpose
-> signed short-lived capability
-> explicit purpose + AccountId + CharacterId + World/route/revisions + audience
-> random one-time nonce
-> game-domain authoritative consume
-> never GameSessionId

fresh admission commit
-> validate grant + current scope/revisions
-> revalidate AccountId -> CharacterId
-> atomically establish account presence
-> establish current CharacterLease/fence
-> consume grant
-> create canonical GameSessionId
-> connection_generation = 1
-> create reconnect-proof state
-> only then publish success

unexpected loss
-> current-generation sufficient-control evidence stops at T0
-> T0 + 2 s: server classifies accepted loss boundary / FND-03 disconnect protection
-> T0 + 5 s: stale concrete transport cleanup may close socket
-> loss boundary + 15 s: same-GameSession grace expires

same-session rebind
-> same GameSessionId
-> one newer connection_generation wins atomically
-> reconnect proof rotates/reconciles
-> old generation fenced
-> 4 s PvE re-entry protection only if this is an eligible classified unexpected-loss episode

post-grace actor still mandatory
-> old GameSession terminal
-> actor PRESENT_UNCONTROLLED
-> AccountPresenceClaim remains same CharacterId
-> different CharacterId remains blocked
-> fresh same-character GameSession may attach only through current game-domain actor placement if final contract accepts

intentional takeover
-> healthy combat-locked incumbent: no kick/fence
-> logout-eligible incumbent: fenced legal drain/logout/handoff
-> fresh GameSession only after old player authority is safely gone

lease uncertainty
-> never automatic replacement authority
-> old player/durable writes fail closed when current lease cannot be proven
-> new lease generation waits for explicit fence/recovery
```

## 34. Gate result

If this analysis baseline passes exact-head review/CI/audit and merges, it becomes canonical **analysis input only**.

It does not complete FND-04 and does not authorize runtime, protocol, persistence, Platform or production implementation.

The next safe package is one final architecture-only `FND-04 Identity, Game Session, Admission and Character Lease Contract` that consumes this analysis and freezes only the decisions genuinely required before implementation.
