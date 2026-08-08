# FND-04 — Identity, Game Session, Admission and Character Lease Contract

- Status: Candidate architecture contract; canonical when merged to `main`
- Date: 2026-08-08
- Gate: `FND-04`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Repository: `blakinio/Oteryn-v2`
- Consumes:
  - ADR-0003 Platform Identity / Game Gateway / admission boundary
  - ADR-0012 Character Authority / Platform lifecycle boundary
  - FND-ID-01 foundation identifier contract and owner baselines
  - FND-02 `protocol-oteryn` v1 foundation contract
  - accepted FND-03 runtime execution contract
  - `FND-04_SESSION_ADMISSION_LEASE_ANALYSIS_BASELINE.md`
  - `FND-04_PLATFORM_PRE_ADMISSION_RECONCILIATION_REFINEMENT.md`
  - disconnect/re-entry owner decisions
  - `FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md`
  - `FND-04_REAUTHENTICATED_RECOVERY_GRANT_PROFILE_V1.md`
  - Foundation Error Vocabulary / Failure Scenario Catalogue
  - read-only `Oteryn-Platform@216f5b2817e9d102337608609e344518512c2a0d` current native pre-admission/runtime-status contracts
- Does not authorize: Rust runtime/protocol implementation, PostgreSQL/Redis schema, Oteryn-Platform writes, production keys, production liveness/lease values, deployment or live traffic

## 1. Purpose

FND-04 freezes the final semantic authority and security contract for native gameplay admission, GameSession lifecycle, reconnect/recovery, account-global online-character exclusion and character lease fencing.

The central invariant is:

```text
Platform may authenticate and authorize an attempt.
Only the current Oteryn-v2 game authority may create or replace gameplay control.
```

A session, transport, account-presence claim, character lease and runtime scope owner are related but are never aliases.

## 2. Canonical authority layers

FND-04 freezes five distinct layers.

### 2.1 AccountPresenceClaim

Scope:

```text
AccountId
```

Meaning:

- identifies which CharacterId, if any, is the account's current playable or mandatory-presence actor;
- enforces the accepted one-online-character rule across worlds/channels/instances;
- remains held while the actor is `PRESENT_CONTROLLED` or `PRESENT_UNCONTROLLED`;
- is released only after authoritative actor lifecycle proves legal absence/removal or an accepted same-character transition preserves/replaces it atomically.

GameSession terminality, socket closure, reconnect-grace expiry or client-process death do not release it by themselves.

### 2.2 CharacterLease

Scope:

```text
CharacterId + character_lease_generation
```

Meaning:

- fences current authoritative character writer/control participation across session/runtime/durability boundaries;
- generation is a non-zero monotonic `uint64`-class fence or equivalent exact non-reused representation;
- stale generation cannot renew, commit durable character mutation or create/restore player control;
- may survive transport replacement and may survive GameSession replacement while the same authoritative actor remains current;
- advances only when ownership is actually replaced/recovered such that a former holder must be fenced.

Character Authority remains semantic owner of the CharacterId aggregate. FND-04 defines the lease/control protocol, not a new owner.

### 2.3 GameSession

Identity:

```text
GameSessionId
```

Meaning:

- one logical player-control lifecycle;
- created only by successful game-domain admission/recovery commit;
- never a bearer credential;
- independent of current NodeId, socket and ChannelRuntime thread/process placement;
- terminal GameSessionId never regains authority.

### 2.4 TransportBinding

Scope:

```text
GameSessionId + connection_generation
```

Meaning:

- exactly one current concrete transport binding may hold playable command/liveness/reconciliation authority for a GameSession;
- FND-02 generation semantics remain binding;
- generation `0` is pre-admission only;
- first admitted binding is `1`;
- every accepted rebind establishes exactly one strictly newer non-zero generation;
- stale generation cannot command, advance liveness, restore reconciliation or fence the winner.

### 2.5 RuntimeScopeAuthority

Scope:

```text
ChannelRuntime / InstanceRuntime semantic scope + FND-03 ownership generation
```

Meaning:

- current authoritative simulation owner;
- separate from CharacterLease and GameSession;
- NodeId is placement/process-incarnation evidence, not authority;
- current runtime ownership is revalidated at final admission/recovery/handoff.

## 3. Actor presence states

FND-04 freezes:

```text
ABSENT
PRESENT_CONTROLLED
PRESENT_UNCONTROLLED
```

### ABSENT

No authoritative actor remains in the world under mandatory gameplay presence. AccountPresenceClaim may be free/reassigned under normal admission rules.

### PRESENT_CONTROLLED

The actor exists and has one current playable GameSession/TransportBinding.

### PRESENT_UNCONTROLLED

The actor remains authoritative in world simulation, but no current playable controller exists.

This state is required when:

- same-session reconnect grace expires while combat/PZ/logout presence remains mandatory;
- a session becomes terminal before the actor is legally removable;
- a control-loss/recovery transition temporarily lacks player control while server-originated simulation must continue.

A different CharacterId for the same AccountId remains blocked while this state exists.

## 4. GameSession state machine

Canonical states:

```text
ACTIVE
CONTROL_SUSPECTED
RECONNECTABLE
TAKEOVER_DRAINING
TERMINATING
TERMINAL
```

### ACTIVE

- one current connection_generation;
- current sufficient-control evidence is healthy;
- ordinary authorized commands may be accepted.

### CONTROL_SUSPECTED

- sufficient-control evidence is late but the accepted loss threshold is not yet crossed;
- existing GameSession remains the only session;
- no re-entry protection is created merely by suspicion;
- routine/proven-safe transport replacement may occur but does not manufacture a disconnect episode.

### RECONNECTABLE

- server has classified eligible unexpected loss of playable control;
- logical GameSession remains alive inside the accepted same-session reconnect grace;
- actor state is preserved;
- stale concrete transport may already have closed;
- reconnect proof or separately accepted reauthenticated recovery proof may establish a new binding.

### TAKEOVER_DRAINING

- intentional authenticated replacement is permitted because incumbent is logout-eligible;
- incumbent remains the only player-control authority until the committed fence/logout/handoff boundary;
- no disconnect protection is granted because this is intentional takeover.

### TERMINATING

- no new ordinary player-command admission;
- session reaches a bounded terminal boundary while required actor/world state remains authoritative.

### TERMINAL

- GameSessionId can never regain authority;
- no reconnect proof can revive it;
- later player control requires a new GameSessionId;
- actor may still be `PRESENT_UNCONTROLLED`.

## 5. Platform / game admission boundary

### Platform owns

- reusable account authentication and security policy;
- OAuth/PKCE/MFA/recovery;
- one-time Game Login Ticket lifecycle;
- Platform account-security generation/revision;
- configured World/Channel/login/maintenance/entitlement policy;
- Game Gateway route/offer orchestration;
- fresh-entry / reauthenticated-recovery attempt authorization;
- signing of the two accepted FND-04 Platform-to-game grant profiles.

### Game domain owns

- final authoritative AccountId->CharacterId revalidation;
- account-presence exclusion;
- CharacterLease state/generation;
- current runtime owner/readiness/revision check;
- grant consume/replay result;
- GameSessionId creation/terminality;
- connection_generation transition;
- reconnect proof state;
- actor control-loss episode/protection eligibility;
- final gameplay admission/recovery/takeover/handoff outcome.

Platform never creates canonical GameSessionId and never becomes gameplay authority by signing a grant.

## 6. Fresh-entry credential

Fresh entry uses only:

```text
docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md
```

Binding summary:

- JWS Compact JWT;
- Ed25519 / JOSE `EdDSA` only;
- explicit `typ = oteryn-admission+jwt`;
- dedicated issuer/audience/key purpose;
- max lifetime 30 seconds;
- max clock skew 5 seconds;
- 32-byte one-time GrantNonce/jti;
- Platform AdmissionAttemptRef separate from GrantNonce;
- account-security generation binding;
- fresh route/runtime observation/ownership-generation binding;
- no token-directed key discovery;
- game-side one-time consume and current-authority revalidation.

No OAuth credential or Game Login Ticket is accepted by the game server as a substitute.

## 7. Fresh admission linearization

Conceptual flow:

```text
Platform authentication/security
-> Game Login Ticket
-> Gateway redeem + current route/runtime evidence
-> signed one-time PreAdmissionGrant
-> Oteryn-v2 final admission validation
-> atomic authority commit
-> ServerAccepted / initial snapshot
```

Final admission validates in bounded fail-closed order:

1. FND-02 material/frame limits;
2. exact FND-04 grant profile and Ed25519 signature;
3. issuer/audience/type/purpose/time;
4. Platform security projection freshness/revocation/generation;
5. route/runtime observation/current scope ownership generation;
6. current protocol/content/ruleset/compatibility requirements;
7. GrantNonce consume eligibility;
8. current AccountId->CharacterId ownership/lifecycle;
9. AccountPresenceClaim / duplicate-login state;
10. CharacterLease compatibility/acquisition;
11. current runtime scope authority/readiness;
12. atomic admission commit.

The commit atomically establishes:

```text
consume GrantNonce
+ AccountPresenceClaim/current revision
+ CharacterLease/current generation
+ new canonical GameSessionId
+ GameSession ACTIVE
+ connection_generation = 1
+ current reconnect-secret verifier/state
+ initial authoritative session/reconciliation boundary
```

Client-visible success is forbidden before this commit.

A candidate GameSessionId generated before commit becomes canonical only if commit succeeds; otherwise it is discarded and never reused.

## 8. Account-global exclusion

Fresh admission across all WorldId/ChannelId/Instance placements must linearize account exclusion.

Two different CharacterIds for one AccountId cannot both become playable/mandatory-presence actors.

Semantic acquisition order:

```text
AccountPresenceClaim
-> CharacterLease
-> GameSession
-> TransportBinding
```

DUR may implement this using transactions/CAS/locks differently, but externally visible partial authority is forbidden.

A stale account-presence revision cannot revoke or overwrite a newer claim.

## 9. Duplicate login and intentional takeover

### 9.1 Healthy combat/PZ/logout-locked incumbent

A second authenticated attempt:

```text
MUST NOT fence incumbent
MUST NOT close incumbent transport
MUST NOT revoke incumbent GameSession
MUST NOT release AccountPresenceClaim
MUST NOT admit another CharacterId
```

Return a coarse conflict/session outcome.

### 9.2 Healthy logout-eligible incumbent

Intentional takeover uses:

1. authenticate newcomer;
2. prove takeover eligibility;
3. transition incumbent to `TAKEOVER_DRAINING`;
4. stop new incumbent ordinary commands at an explicit committed fence boundary;
5. complete legal logout/removal or accepted same-character handoff boundary;
6. release/advance account/character authority atomically;
7. create a fresh GameSessionId for the newcomer;
8. create fresh connection_generation namespace at `1`;
9. grant no disconnect/re-entry protection.

No interval may contain two current player-control authorities.

### 9.3 Concurrent contender rule

Concurrent fresh admission/takeover/recovery attempts serialize through current AccountPresenceClaim, CharacterLease and GameSession state. A stale loser cannot fence a newer successful winner.

## 10. Platform account-security freshness after grant issuance

FND-04 adopts the normal-path security model from the reconciliation refinement:

```text
short-lived signed grant
+ account_security_generation in the grant
+ trusted bounded-staleness Platform-security validity projection at game admission
```

Fresh admission and reauthenticated recovery require projection evidence age <= 5 seconds.

Fail closed when required projection evidence is:

- older than 5 seconds;
- unavailable;
- unauthenticated;
- contradictory;
- unable to prove the token generation remains admissible.

Reject when account is disabled/revoked or token generation is below the current minimum-valid generation.

Online Platform introspection is **not** required on every normal admission in v1. A later security implementation may use exceptional introspection in addition to the accepted projection.

This mechanism does not automatically terminate already-admitted gameplay. A post-admission emergency account/session revocation channel is a separate fenced control contract and remains unimplemented/unapproved here.

## 11. Platform admission-attempt idempotency versus game consume replay

FND-04 freezes two concepts:

```text
AdmissionAttemptRef
-> Platform producer operation/correlation/idempotency

GrantNonce
-> concrete signed capability game consume/replay identity
```

They are never aliases.

### AdmissionAttemptRef

- canonical UUIDv7 text in grant profiles;
- not a foundation entity identity;
- same logical producer retry uses same attempt ref;
- ambiguous issuance cannot mint multiple independently usable grants for that one attempt;
- new independent attempt uses new ref.

### GrantNonce

- 32 random bytes, encoded according to the signed profile;
- at most one successful game authority transition;
- consumed/replay state remains valid through latest token acceptance time at minimum;
- consumed grant never becomes reusable because a response was lost.

## 12. Fresh-entry route/runtime-owner applicability

A fresh-entry grant is valid only for the exact issuance-time target evidence accepted by its profile.

Game final admission always validates current authoritative state.

Default v1 rule:

```text
token.scope_ownership_generation != current scope ownership generation
-> STALE_GRANT / reject
-> fresh Gateway routing + fresh grant required
```

Also reject stale/incompatible route revision, runtime observation revision, protocol/transport/compatibility revision or non-open current target lifecycle.

No silent retarget to another Channel, GameNode owner, protocol family or Canary route.

NodeId is not a grant authority claim.

## 13. Game-domain reconnect secret

Each admitted GameSession receives game-domain reconnect proof material.

v1 security properties:

- exactly 32 cryptographically random bytes;
- transmitted only inside accepted TLS;
- stored server-side as a one-way verifier or equivalent secret-safe representation;
- never logged, traced, exported to analytics or rendered to users;
- scoped to one current GameSession/reconnect-proof state;
- rotated through the rebind state machine below;
- GameSessionId alone never substitutes for it.

The exact verifier primitive is an implementation-security choice. Given 256 bits of random secret entropy, storage MUST not reduce effective online/offline resistance below the accepted secret strength.

## 14. Reconnect PREPARE / COMMIT state machine

FND-04 rejects rotate-and-forget reconnect.

A same-GameSession transport replacement uses two authority phases.

### 14.1 ReconnectAttemptRef

The client creates a fresh cryptographically random 16-byte operation reference for one rebind attempt.

It is not a foundation entity identity.

### 14.2 PREPARE

Client presents on a new TLS transport:

```text
GameSessionId
current reconnect secret OR accepted reauthenticated recovery grant
ReconnectAttemptRef
```

Server validates:

- GameSession/current state eligibility;
- old/current transport loss/replacement eligibility;
- current CharacterLease/runtime/session reconciliation safety;
- presented proof;
- reconnect grace when same-session loss recovery is being requested.

If accepted, server reserves exactly one bounded prepared rebind:

```text
candidate connection_generation = current + 1
successor reconnect secret = new 32 random bytes
ReconnectAttemptRef
prepared-state expiry <= remaining same-session grace
```

PREPARE does **not** make the new transport authoritative.

The current/old generation is not restored/advanced by PREPARE.

A retry using the same proof + same ReconnectAttemptRef obtains the same prepared outcome while it remains valid. A competing different attempt cannot create a second simultaneous candidate winner once a current prepared transition owns the rebind slot.

The client MUST retain the predecessor reconnect secret until COMMIT is acknowledged or it has authoritative evidence that successor state committed.

### 14.3 COMMIT

Client proves possession of the prepared successor secret on the prepared TLS transport.

The server atomically commits:

```text
candidate connection_generation becomes current
prepared transport becomes current TransportBinding
successor reconnect secret becomes current proof
predecessor proof becomes stale
old transport/generation loses command/liveness/reconciliation authority
prepared state becomes committed/terminal
```

Only after this commit is the new transport authoritative.

### 14.4 Lost PREPARE response

Because old proof remains current before COMMIT, client retries the same ReconnectAttemptRef and receives the same prepared transition if still valid.

No duplicate candidate is minted for the same logical attempt.

### 14.5 Crash after PREPARE before COMMIT

PREPARE alone cannot make a new generation authoritative.

If prepared state is lost, the old committed reconnect proof/session generation remains the authority state and the client can retry with retained predecessor proof, subject to current reconnect eligibility.

If an implementation durably persists prepared state, replay/recovery must still preserve exactly one candidate and no generation change before COMMIT.

### 14.6 Lost COMMIT response / crash around COMMIT

The client already knows the successor secret from PREPARE.

If COMMIT succeeded, durable/recoverable session state must show the new current connection_generation and successor verifier. The predecessor cannot regain authority.

If COMMIT did not succeed, predecessor remains current.

Recovery must determine one of those two authoritative states; it may never guess or accept both.

If this exact state cannot be reconstructed after GameNode replacement, same-GameSession continuity is not claimed and the session follows the safe fresh-session recovery path.

## 15. Reconnect concurrency and replay

Only one prepared/committed current rebind wins.

Scenarios:

- two attempts using same current reconnect proof -> one PREPARE owner/winner at most;
- stale predecessor after successful COMMIT -> reject, cannot fence successor;
- replayed successor outside its current lifecycle -> reject;
- stale connection_generation traffic -> FND-02 `STALE_GENERATION` semantics;
- replayed consumed reauthenticated recovery grant -> no second rebind/session.

Bounded prepared-rebind state is mandatory. Exact count/resource limit is registered before implementation acceptance; one current prepared rebind per GameSession is the v1 semantic maximum.

## 16. Sufficient playable-control/liveness evidence

Primary sufficient evidence is a valid current-generation response to a recent server-issued authenticated liveness probe.

It binds:

- GameSessionId;
- current connection_generation;
- current probe identity;
- server-observed round-trip progress;
- current runtime-health context.

Not sufficient:

- socket-open state;
- client wall-clock timestamp;
- stale-generation ack;
- one-way bytes;
- arbitrary gameplay-command silence/presence.

Other bidirectional current-generation control exchanges may count only when a later exact contract proves they establish the same property as a liveness round trip.

### Numeric cadence evidence gate

FND-04 does **not** invent the production probe cadence.

Before implementation acceptance, a registered liveness profile MUST provide a concrete interval/hysteresis validated by latency/load/packet-loss/fault tests and satisfy at least:

```text
probe_interval < 0.5 * 2.0-second loss threshold
```

with additional measured margin for scheduler/network jitter.

The 2.0-second behavioral loss threshold itself remains the accepted owner decision.

## 17. Exact same-session reconnect grace

FND-04 freezes:

```text
T0 = last accepted sufficient current-generation control evidence
control_loss_declared_at = T0 + 2.0 seconds
stale_concrete_transport_cleanup = T0 + 5.0 seconds
same_session_grace_expires = control_loss_declared_at + 15.0 seconds
```

Thus the GameSession receives a full 15-second same-session reconnect window after authoritative loss classification.

The 5-second transport cleanup does not terminate the GameSession.

If current sufficient control is restored/rebound before `control_loss_declared_at`, no unexpected-loss episode is created and no 4-second defensive re-entry effect is granted merely because connection_generation changed.

At grace expiry, GameSession transitions toward TERMINAL if not recovered.

## 18. ControlLossEpoch and re-entry protection

FND-04 freezes an internal actor/session semantic `ControlLossEpoch` (revision/state, not foundation entity ID).

Rules:

- one epoch is created only when server classifies eligible unexpected loss of playable control;
- exactly one re-entry-protection activation may be consumed for one epoch;
- routine rebind, graceful logout, intentional takeover or JWT issuance does not create an epoch;
- stale/replayed reconnect attempts do not create/restart an epoch;
- GameSession replacement during the same episode does not reset the epoch;
- FND-03 executes the accepted 4-second PvE effect after FND-04 marks one eligible re-entry;
- once the epoch's protection eligibility is consumed, further rebinds in that same epoch cannot restart it.

A later new loss becomes a new epoch only after the session/actor has returned to a registered `STABLE_ACTIVE` liveness state.

The exact anti-flap hysteresis required for `STABLE_ACTIVE` is a measured liveness/security-policy value and MUST be concrete before implementation acceptance; it is not guessed here.

## 19. Reauthenticated recovery grant

Loss of reconnect secret may use:

```text
docs/contracts/FND-04_REAUTHENTICATED_RECOVERY_GRANT_PROFILE_V1.md
```

This profile has mutually exclusive validation from fresh entry and contains no ChannelId/InstanceId authority.

It proves a fresh Platform-authenticated recovery attempt for AccountId + CharacterId + WorldId, not current actor/session placement.

Platform may require MFA/step-up/risk checks before issuing it.

## 20. Same-GameSession reauthenticated recovery inside grace

A valid recovery grant may substitute for missing reconnect secret only when game-domain state proves:

- current GameSession is `RECONNECTABLE`;
- same-session grace has not expired;
- no healthy current playable controller exists;
- AccountId currently owns CharacterId;
- AccountPresenceClaim still binds CharacterId;
- current CharacterLease/runtime authority is safe;
- current actor placement is resolved by game domain;
- FND-02 command/session reconciliation state remains reconstructable.

Successful path:

- consumes RecoveryGrantNonce once;
- uses reconnect PREPARE/COMMIT;
- preserves GameSessionId;
- commits one newer connection_generation;
- creates/rotates current reconnect proof;
- may consume the current ControlLossEpoch's one 4-second protection activation if still eligible.

A healthy incumbent cannot be preempted by a valid recovery JWT.

## 21. Post-grace same-character existing-actor recovery

FND-04 accepts this direction as part of the native contract.

When grace expires:

```text
old GameSession -> TERMINAL
```

If gameplay rules keep the actor present:

```text
actor -> PRESENT_UNCONTROLLED
AccountPresenceClaim -> still same CharacterId
CharacterLease/runtime actor -> remains current under existing authority/fencing
```

A fresh valid reauthenticated recovery grant for the same AccountId/CharacterId may create a **new GameSessionId** and attach control to that exact existing actor when:

- old GameSession is terminal;
- no current playable controller exists;
- current AccountId->CharacterId ownership still matches;
- AccountPresenceClaim remains same CharacterId;
- CharacterLease/runtime scope is current/safe;
- current game-domain placement is unambiguous;
- current compatibility/reconciliation state supports a fresh authoritative snapshot/session boundary.

The commit creates:

```text
new GameSessionId
connection_generation = 1
new reconnect secret
new session command/reconciliation namespace
control attached to same existing actor
```

It MUST NOT:

- respawn/recreate actor;
- heal/reset resources;
- clear conditions/cooldowns/combat/PZ/threat/encounter state;
- teleport merely for recovery;
- release/reacquire AccountPresenceClaim through a race window;
- duplicate inventory/state.

If current placement/lease/actor state cannot be proven, fail closed rather than recreating the character.

A different CharacterId remains blocked until the original actor is legally absent.

## 22. Recovery locator/current placement

Reauthenticated recovery requires a game-domain recovery locator/dispatcher.

Semantic input:

```text
AccountId + CharacterId + WorldId + recovery-attempt correlation
```

It resolves:

- current actor presence;
- current GameSession state if any;
- current CharacterLease generation/state;
- current ChannelRuntime/InstanceRuntime placement and ownership generation;
- current routable recovery endpoint/owner.

Rules:

- Platform configured ChannelId is not actor placement authority;
- stale client route does not move actor;
- actor in InstanceRuntime remains in that instance unless an accepted handoff changes ownership;
- ambiguous/suspected/no-current-owner evidence fails closed;
- private NodeId/fencing/topology detail need not be exposed to client;
- exact API/transport/deployment is later implementation/OPS work.

## 23. Channel and Instance handoff

### 23.1 Channel -> Instance / Instance -> Channel continuous activity

When accepted gameplay transition preserves one logical control session:

- same AccountPresenceClaim remains continuously held;
- same CharacterLease may remain when writer ownership transfer is properly fenced;
- same GameSessionId may be preserved;
- same current connection_generation may remain if the concrete transport/session binding does not rebind;
- HandoffId identifies the ownership transition;
- source stops mutation authority only at committed handoff barrier;
- destination becomes current before normal destination deltas continue.

### 23.2 Channel -> Channel fresh logical session

When the accepted transition establishes a fresh logical GameSession:

- fresh destination admission authorization is required;
- fresh GameSessionId is created;
- source session becomes terminal/drained under handoff rules;
- AccountPresenceClaim for the same CharacterId remains atomically continuous, not released to another character;
- CharacterLease generation advances only if the writer-fence transition requires it;
- no old grant/route is silently retargeted.

The exact transition catalogue is later gameplay/instance routing contract data, but these two session-continuity classes are frozen.

## 24. CharacterLease renewal and fail-safe semantics

CharacterLease physical TTL/cadence/storage is not guessed by FND-04.

Binding semantic rules:

- only current generation may renew;
- sent renewal request is not proof of renewal;
- authoritative lease-store/ownership state defines accepted expiry;
- local holder has a fail-safe deadline strictly earlier than the point another generation could legitimately become authoritative;
- renewal uncertainty does not release AccountPresenceClaim;
- renewal uncertainty does not self-grant a replacement writer;
- stale generation cannot commit durable player/character mutation;
- current runtime may continue only those server-originated in-memory effects for which FND-03/DUR prove there cannot be a competing writer;
- replacement generation requires explicit fence/recovery evidence.

### Numeric lease evidence gate

Before lease implementation acceptance, DUR/OPS/PERF fault injection MUST freeze a concrete policy containing:

- authoritative lease TTL;
- renewal interval;
- local safety margin;
- maximum dependency/network uncertainty assumed;
- fail-safe deadline;
- replacement/fencing timing.

Minimum relationship:

```text
renew_interval < lease_TTL / 3
local_fail_safe_deadline < authoritative_expiry
safety_margin > measured worst-case renewal/clock/transport uncertainty used by the proof
```

No production implementation may use library/default/infinite values absent that evidence.

## 25. GameNode replacement and same-session continuity

A replacement NodeId does not automatically end GameSession, but same-session recovery is permitted only if current authoritative state can safely reconstruct:

- GameSession state/terminality;
- current connection_generation;
- reconnect current/prepared/committed proof state;
- CommandId high-water/pending/result state;
- server-sequence/snapshot reconciliation boundary;
- AccountPresenceClaim;
- CharacterLease current generation/state;
- current ControlLossEpoch/protection-consumed state;
- current runtime ownership generation/placement.

If any required state cannot be proven, the system does not guess same-session continuity.

It safely terminates the old GameSession and uses the accepted fresh-session existing-actor recovery path when actor state is valid, or remains fail-closed when it is not.

## 26. Key rotation and revocation

Both Platform grant profiles require:

- dedicated asymmetric verification key purpose;
- allowlisted trusted key-set source;
- exact `kid` lookup only inside trusted configured/provisioned set;
- current/retiring overlap for still-valid grants;
- emergency key revocation;
- no acceptance merely because a signature key is cryptographically valid outside the trusted policy;
- mixed producer/consumer profile revisions fail closed.

Production key creation/rotation cadence/KMS is a later security-operations implementation task.

## 27. Stable internal error codes and public classes

FND-04 freezes symbolic stable internal codes. Numeric wire allocation, if later exposed, follows the FND-02 registry process.

### Credential/admission

- `ADMISSION_GRANT_MALFORMED` -> `INVALID_INPUT`
- `ADMISSION_GRANT_AUTHENTICATION_FAILED` -> `AUTHENTICATION_FAILED`
- `ADMISSION_GRANT_EXPIRED` -> `SESSION_REJECTED`
- `ADMISSION_GRANT_REPLAYED` -> `SESSION_REJECTED`
- `ADMISSION_GRANT_SECURITY_STATE_REVOKED` -> `SESSION_REJECTED`
- `ADMISSION_GRANT_SECURITY_EVIDENCE_STALE` -> `DEPENDENCY_UNAVAILABLE`
- `ADMISSION_GRANT_ROUTE_STALE` -> `STALE_GENERATION`
- `ADMISSION_GRANT_RUNTIME_GENERATION_STALE` -> `STALE_GENERATION`
- `ADMISSION_GRANT_REVISION_UNSUPPORTED` -> `UNSUPPORTED_REVISION`
- `ADMISSION_ACCOUNT_CHARACTER_CONFLICT` -> `CONFLICT`
- `ADMISSION_INCUMBENT_PROTECTED` -> `CONFLICT`
- `ADMISSION_CAPACITY_EXCEEDED` -> `CAPACITY_EXCEEDED`

### Reconnect/recovery

- `RECONNECT_PROOF_INVALID` -> `AUTHENTICATION_FAILED`
- `RECONNECT_PROOF_REPLAYED` -> `SESSION_REJECTED`
- `RECONNECT_SESSION_TERMINAL` -> `SESSION_REJECTED`
- `RECONNECT_GENERATION_STALE` -> `STALE_GENERATION`
- `RECONNECT_ATTEMPT_CONFLICT` -> `CONFLICT`
- `RECONNECT_GRACE_EXPIRED` -> `SESSION_REJECTED`
- `RECOVERY_GRANT_REPLAYED` -> `SESSION_REJECTED`
- `RECOVERY_HEALTHY_CONTROLLER_PRESENT` -> `CONFLICT`
- `RECOVERY_PLACEMENT_UNAVAILABLE` -> `DEPENDENCY_UNAVAILABLE`
- `RECOVERY_STATE_UNSAFE` -> `INTERNAL_UNAVAILABLE`

### Lease/session

- `CHARACTER_LEASE_STALE` -> `STALE_GENERATION`
- `CHARACTER_LEASE_RENEW_TIMEOUT` -> `TIMEOUT`
- `CHARACTER_LEASE_DEPENDENCY_UNAVAILABLE` -> `DEPENDENCY_UNAVAILABLE`
- `SESSION_TAKEOVER_NOT_ALLOWED` -> `CONFLICT`

Public client responses may intentionally collapse sensitive distinctions into safe presentation classes such as:

```text
AUTHENTICATION_REQUIRED
SESSION_UNAVAILABLE
CHARACTER_ALREADY_ACTIVE
RETRY_LOGIN
TEMPORARILY_UNAVAILABLE
CLIENT_UPDATE_REQUIRED
```

They MUST NOT expose raw grants/secrets, account-security generation, private fencing state, SQL/internal errors or combat-sensitive details beyond accepted UX policy.

## 28. Foundation failure scenario disposition

FND-04 classifies the shared catalogue at architecture-contract level.

| Scenario | FND-04 status | Requirement / owner |
|---|---|---|
| `FS-PLATFORM-UNAVAILABLE` | `PASS` | new Platform-dependent fresh/re-auth grants fail/hold boundedly; no alternate credential authority; already-active gameplay and game-domain fast reconnect are not invalidated merely by Platform outage |
| `FS-GATEWAY-AFTER-REDEEM` | `PASS` | no blind second issuance; AdmissionAttemptRef idempotency/reconciliation; no GameSession absent final game commit |
| `FS-POSTGRES-UNAVAILABLE` | `DEFERRED_BY_ACCEPTED_GATE` | DUR owns physical persistence; no session/lease commit may claim success without required atomic authority evidence |
| `FS-LEASE-RENEW-TIMEOUT` | `PASS` | old writer fails closed before replacement; timeout never self-grants new writer |
| `FS-DUPLICATE-LOGIN` | `PASS` | account-global exclusion + healthy incumbent protection + one winner |
| `FS-STALE-GENERATION` | `PASS` | stale connection/lease/runtime generation cannot command/recover/commit |
| `FS-DUPLICATE-COMMAND` | `NOT_APPLICABLE` | FND-02 command contract remains authority |
| `FS-CHANNEL-SPLIT-OWNER` | `PASS` | FND-03 current ownership + FND-04 route/lease checks prevent stale admission/control; physical fencing proof continues under OPS/DUR |
| `FS-CHANNEL-DRAIN` | `PASS` | no new admission to non-open/draining target; current session/handoff follows FND-03 drain barrier |
| `FS-QUEUE-SATURATION` | `DEFERRED_BY_ACCEPTED_GATE` | resource limits/runtime implementation; authority transition fails before partial commit |
| `FS-SLOW-CLIENT` | `PASS` | transport may close under FND-02/FND-03 while logical session follows FND-04 reconnect semantics |
| `FS-CLOCK-SKEW` | `PASS` | signed grants max 5s skew; liveness/grace uses server monotonic time |
| `FS-KEY-ROTATION` | `PASS` | dedicated key purposes, bounded overlap, emergency revocation, fail-closed unknown revision/key |
| `FS-REVISION-MISMATCH` | `PASS` | no profile/protocol/route/runtime downgrade |
| `FS-SNAPSHOT-DELTA-MISMATCH` | `NOT_APPLICABLE` | FND-02/FND-03 reconciliation after admitted/rebound state |
| `FS-DB-OUTBOX-BOUNDARY` | `DEFERRED_BY_ACCEPTED_GATE` | DUR/ANL owns atomic durable evidence; admission success cannot precede required durable commit |
| `FS-WORLD-BUNDLE-CORRUPT` | `NOT_APPLICABLE` | invalid target must not become routable/admissible under upstream activation rules |
| `FS-CLIENT-CUTOVER-ROLLBACK` | `NOT_APPLICABLE` | historical migration lifecycle |
| `FS-ANALYTICS-TELEMETRY-OVERFLOW` | `NOT_APPLICABLE` | telemetry never session authority |
| `FS-AUDIT-OUTBOX-BACKLOG` | `DEFERRED_BY_ACCEPTED_GATE` | ANL/DUR required security audit must not silently degrade |
| `FS-EVENT-DUPLICATE-DELIVERY` | `NOT_APPLICABLE` | analytics replay cannot alter session state |
| `FS-EVENT-OUT-OF-ORDER` | `NOT_APPLICABLE` | analytics order cannot alter session state |
| `FS-AUDIT-MUTATION-MISMATCH` | `DEFERRED_BY_ACCEPTED_GATE` | ANL/DUR atomic evidence boundary |
| `FS-ANALYTICS-PRIVACY-POLICY` | `NOT_APPLICABLE` | credentials excluded from analytics; privacy remains ANL policy |
| `FS-DETECTOR-FALSE-POSITIVE` | `NOT_APPLICABLE` | analytics cannot sanction/revoke autonomously |
| `FS-INVESTIGATION-MUTATION-ATTEMPT` | `NOT_APPLICABLE` | investigation cannot mutate session/runtime authority |
| `FS-ADMISSION-GRANT-REPLAY` | `PASS` | one GrantNonce at most one successful admission; concurrent replay has one winner; consumed stays consumed |
| `FS-RECONNECT-CREDENTIAL-REPLAY` | `PASS` | PREPARE/COMMIT + current proof/generation gives one winner; stale predecessor cannot fence successor |

`PASS` means the architecture invariant is present, not that executable implementation proof exists.

## 29. Cross-repository compatibility

Production implementation requires an explicit compatibility lock/matrix across:

- Oteryn-v2 FND-04 contract revision;
- fresh-entry grant profile v1;
- reauthenticated recovery grant profile v1;
- Oteryn Platform producer revisions;
- Platform account-security projection contract revision;
- runtime-status observation contract revision;
- FND-02 protocol major/transport profile;
- current runtime/content compatibility revision.

Independent fixtures MUST cover both positive and negative cases from the two grant profiles plus session/reconnect/fencing fault cases.

A producer may not start emitting a mandatory new security field before all target consumers reject/understand it according to rollout plan.

No silent downgrade.

## 30. Resource and evidence gates before implementation acceptance

Architecture is complete only at semantic/security level. Implementation claims remain blocked until concrete evidence exists for:

### Liveness profile

- exact probe interval;
- hysteresis / STABLE_ACTIVE rule;
- latency/load/packet-loss/fault evidence;
- scheduler jitter margin;
- false-positive/false-negative expectations.

### Character lease profile

- TTL;
- renew cadence;
- local safety margin;
- maximum uncertainty assumption;
- replacement/fence timing;
- database/lease-store fault injection.

### Session/reconnect resource limits

Register concrete hard maxima for:

- prepared rebinds per GameSession (semantic v1 maximum one current prepared rebind);
- prepared-state bytes/time retention;
- GrantNonce/recovery-nonce replay records;
- admission/recovery attempts per account/IP/session as appropriate;
- account-security projection cache/state;
- recovery locator outstanding work;
- terminal/reconciliation receipt retention.

### Cryptographic/interoperability evidence

- independent PHP producer / Rust consumer fixtures;
- malformed/algorithm-confusion corpus;
- key rotation/revocation fixtures;
- mixed-version rejection fixtures;
- replay/concurrency tests;
- credential redaction tests.

No production defaults are inferred from library defaults.

## 31. Security/privacy summary

Never log or expose:

- reusable account credentials;
- Game Login Ticket;
- raw fresh-entry/recovery JWT;
- GrantNonce/RecoveryGrantNonce;
- raw reconnect secret;
- signing private keys;
- secret verifier digests.

GameSessionId/AccountId/CharacterId are identifiers, not credentials.

High-cardinality IDs do not become ordinary metric labels.

Client/OS diagnostics never authorize admission/reconnect or advance liveness.

## 32. Rejected alternatives

### Platform creates GameSessionId

Rejected: violates accepted final game-domain admission authority.

### Game server accepts OAuth/password directly

Rejected: duplicates Platform reusable-credential authority.

### GameSessionId as reconnect secret

Rejected: identity is not authentication proof.

### One generic signed JWT for fresh entry and recovery

Rejected: route/authority semantics differ and RFC-8725-style mutually exclusive validation is safer.

### Reuse fresh-entry ChannelId grant to recover actor

Rejected: stale Platform/client route could move an existing actor or bypass current InstanceRuntime placement.

### Pure self-contained JWT with expiry only

Rejected: does not fully disposition post-issuance Platform security revocation/generation changes.

### Online Platform introspection required for every fast reconnect

Rejected: creates unnecessary Platform dependency for already-admitted game-domain continuity.

### Rotate reconnect secret immediately and forget predecessor before client receives successor

Rejected: lost response can strand the client and creates ambiguous authority.

### Reconnect PREPARE makes new transport authoritative

Rejected: creates ambiguity before successor proof/commit.

### Lease expiry automatically grants replacement writer

Rejected: split-brain/stale-writer risk and potential combat-escape abuse.

### GameSession terminality releases account presence immediately

Rejected: actor may remain mandatory in world.

### Duplicate login kicks healthy combat-locked incumbent

Rejected: accepted owner direction forbids it and creates abuse/security risk.

## 33. Downstream ownership

### DUR

Owns physical AccountPresence/CharacterLease/GameSession persistence, atomicity, isolation, recovery, prepared-rebind durability decisions, replay-store implementation and item/currency durable safety.

### OPS/PERF

Owns measured lease/liveness/placement capacities, failure detection, production rollout/drain/recovery objectives and hard numeric runtime limits.

### Oteryn Platform

Must later implement the two accepted producer grant profiles, Platform-security validity projection and current runtime-status consumer integration under a separately authorized cross-repository rollout.

### FND-02 / protocol implementation

Later implementation registers exact admission/reconnect/recovery messages and numeric error codes without changing FND-04 semantics.

### ANL / Game Intelligence

May consume bounded security/audit evidence but never raw credentials and never becomes automatic session/gameplay mutation authority.

## 34. Acceptance boundary

When this contract and its two grant profiles merge:

- FND-04 architecture gate is complete;
- Identity/GameSession/admission/reconnect/account-presence/CharacterLease semantics are frozen;
- native admission/reconnect implementation is still **not authorized** by this merge alone;
- implementation requires separate tasks plus the numeric/evidence gates in Section 30;
- Platform producer rollout requires a separate authorized Platform task/PR;
- DUR/OPS/PERF/ANL gates remain independently required.

## 35. Canonical concise rule

```text
Platform authenticates account
-> signed bounded attempt capability
-> never GameSession authority

fresh entry
-> strict Ed25519 fresh-entry grant
-> current Platform-security evidence
-> current route/runtime owner evidence
-> one-time GrantNonce
-> revalidate AccountId->CharacterId
-> AccountPresenceClaim + CharacterLease
-> atomic new GameSessionId + connection_generation 1

active control
-> one GameSession
-> one current transport generation
-> one current runtime owner

unexpected loss
-> T0 last sufficient control
-> T0+2s loss declared
-> T0+5s concrete transport cleanup may close
-> loss+15s same-session grace ends

same-session recovery
-> reconnect secret OR strict reauthenticated recovery grant
-> current game placement resolved by Oteryn-v2
-> PREPARE candidate generation + successor secret
-> COMMIT successor proof
-> exactly one new current generation
-> same GameSessionId
-> one protection activation per eligible ControlLossEpoch

post-grace actor still mandatory
-> old GameSession terminal
-> actor PRESENT_UNCONTROLLED
-> AccountPresenceClaim remains same CharacterId
-> recovery grant may create fresh GameSessionId attached to same actor
-> no reset/respawn/teleport/heal
-> different CharacterId remains blocked

lease uncertainty
-> no automatic replacement
-> old writer fails closed when authority cannot be proven
-> replacement only after explicit fence/recovery

all implementation-sensitive cadence/lease/capacity values
-> concrete measured registry/profile before implementation acceptance
-> never library defaults or guesses
```
