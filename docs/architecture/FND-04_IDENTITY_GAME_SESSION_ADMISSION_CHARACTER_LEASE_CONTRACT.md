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
  - read-only `Oteryn-Platform@216f5b2817e9d102337608609e344518512c2a0d` native pre-admission/runtime-status contracts
- Does not authorize: Rust runtime/protocol implementation, PostgreSQL/Redis schema, Oteryn-Platform writes, production keys, production liveness/lease values, deployment or live traffic

## 1. Purpose

FND-04 freezes the final semantic authority and security contract for native gameplay admission, GameSession lifecycle, reconnect/recovery, account-global online-character exclusion and character lease fencing.

Central invariant:

```text
Platform may authenticate and authorize an attempt.
Only the current Oteryn-v2 game authority may create or replace gameplay control.
```

A session, transport, account-presence claim, character lease and runtime scope owner are related but are never aliases.

## Decision timing

FND-04 applies the repository's mandatory timing test before freezing any material admission/session choice.

| Material decision | Must decide now? | Concrete downstream work blocked | What becomes harder if wrong | Evidence that may justify superseding | Deliberately undecided |
|---|---|---|---|---|---|
| Platform authorization vs game-domain final admission/GameSession authority | `YES` | native admission implementation, Platform producer integration, FND-02 admission messages, Character Authority integration | moving authority after deployment would require credential/session migration and risks two security authorities | concrete security architecture evidence proving a safer single authority without violating ADR-0003/ADR-0012 | implementation API/transport details |
| Separate AccountPresenceClaim, CharacterLease, GameSession, TransportBinding and RuntimeScopeAuthority | `YES` | persistence schema, duplicate-login handling, runtime fencing, reconnect/recovery | collapsing identities/generations would create ambiguous stale-writer/session ownership and anti-duplication risk | fault-injection/formal invariant evidence proving an equivalent simpler model preserves all fencing semantics | physical tables, lock primitives, service placement |
| Strict separate fresh-entry and reauthenticated-recovery grant profiles | `YES` | Platform issuer/consumer implementation and native recovery flow | one deployed bearer format could become an unintended cross-purpose credential/downgrade surface | independent cryptographic/security review and interoperability evidence for a replacement profile revision | concrete JWT libraries, KMS/HSM/vendor, key distribution transport |
| Two-phase reconnect PREPARE/COMMIT with atomic COMMIT-time authority/security revalidation | `YES` | reconnect wire implementation, same-session recovery, crash/lost-response handling | rotate-and-forget or PREPARE-time-only authorization can create split control, stale credential takeover and unrecoverable ambiguity | deterministic concurrency/fault evidence proving a different protocol retains one-current-binding and lost-response safety | prepared-state persistence encoding and storage technology |
| Healthy-current-binding non-preemption without a separately current-generation-authorized migration/takeover transition | `YES` | reconnect/recovery/takeover implementation | allowing secret/JWT possession alone to evict a healthy controller creates account-takeover and player-fairness risk | a separately reviewed healthy-session migration protocol with explicit current-generation consent/authorization and E2E race evidence | whether such healthy-session migration is ever added |
| Exact 2s control-loss, 5s stale-transport cleanup and 15s same-session grace composition already accepted by owner decisions | `YES` for semantic timing | liveness/session implementation and client reconnect UX | changing deployed behavioral windows affects combat fairness and compatibility | representative latency/fault/player-experience evidence plus an explicit superseding owner decision | probe cadence, hysteresis, scheduler margins and other measured implementation values |
| Post-grace existing-actor recovery creates a fresh GameSession without resetting actor state | `YES` | recovery state machine, persistence/reconciliation and anti-abuse tests | retrofitting after persistence/gameplay code could introduce respawn/reset/duplication escape paths | gameplay/security evidence showing a different transition preserves actor/economy/combat invariants | recovery-locator transport and physical state storage |
| CharacterLease numeric TTL/renew/safety values and concrete runtime capacities | `NO` | only implementation acceptance, not architecture analysis | guessing values now would encode unmeasured availability/performance assumptions | PERF/OPS/DUR benchmarks and fault injection are required to choose them | exact TTL, renew cadence, queue/resource values, RPO/RTO |

Therefore FND-04 freezes authority, ordering, security and failure semantics now, but intentionally does not freeze benchmark-sensitive numeric capacities, persistence technology details, deployment products or vendor/library choices without evidence.

## 2. Canonical authority layers

FND-04 freezes five distinct layers.

### 2.1 AccountPresenceClaim

Scope: `AccountId`.

It identifies the account's current playable or mandatory-presence CharacterId and enforces the accepted one-online-character invariant across worlds/channels/instances.

It remains held while the actor is `PRESENT_CONTROLLED` or `PRESENT_UNCONTROLLED` and is released only after authoritative actor lifecycle proves legal absence/removal or an accepted same-character transition preserves/replaces it atomically.

GameSession terminality, socket closure, reconnect-grace expiry or client-process death do not release it by themselves.

### 2.2 CharacterLease

Scope:

```text
CharacterId + character_lease_generation
```

It fences current authoritative character writer/control participation across session/runtime/durability boundaries.

Rules:

- generation is non-zero monotonic `uint64`-class state or an exact non-reused equivalent;
- stale generation cannot renew, commit durable character mutation or create/restore player control;
- the lease may survive transport replacement and may survive GameSession replacement while the same authoritative actor remains current;
- the generation advances only when authority is actually replaced/recovered such that a former holder must be fenced;
- exhaustion never wraps/reuses; fail safe.

Character Authority remains semantic owner of the CharacterId aggregate. FND-04 defines the lease/control protocol, not a new aggregate owner.

### 2.3 GameSession

Identity: `GameSessionId`.

It represents one logical player-control lifecycle.

Rules:

- created only by successful game-domain admission/recovery commit;
- never a bearer credential;
- independent of NodeId, concrete socket and OS thread/process placement;
- a terminal GameSessionId never regains authority.

### 2.4 TransportBinding

Scope:

```text
GameSessionId + connection_generation
```

Rules:

- exactly one current concrete transport binding may hold playable command/liveness/reconciliation authority for one GameSession;
- FND-02 generation semantics remain binding;
- generation `0` is pre-admission only;
- first admitted binding is `1`;
- every accepted rebind establishes one strictly newer non-zero generation;
- stale generation cannot command, advance liveness, restore reconciliation or fence the winner.

### 2.5 RuntimeScopeAuthority

Scope: current ChannelRuntime/InstanceRuntime semantic scope + FND-03 ownership generation.

It is the current authoritative simulation owner and remains separate from CharacterLease and GameSession.

NodeId is placement/process-incarnation evidence, not authority. Current runtime ownership is revalidated at final admission/recovery/handoff.

## 3. Actor presence states

Canonical states:

```text
ABSENT
PRESENT_CONTROLLED
PRESENT_UNCONTROLLED
```

### ABSENT

No authoritative actor remains under mandatory world presence. AccountPresenceClaim may be released/reassigned under normal admission rules.

### PRESENT_CONTROLLED

The actor exists and one current GameSession/TransportBinding provides playable control.

### PRESENT_UNCONTROLLED

The actor remains authoritative in world simulation while no current playable controller exists.

This is required when same-session grace expires while combat/PZ/logout presence remains mandatory, or when session control ends before actor removal is legal.

A different CharacterId for the same AccountId remains blocked while this state exists.

## 4. GameSession states

Canonical logical states:

```text
ACTIVE
CONTROL_SUSPECTED
RECONNECTABLE
TAKEOVER_DRAINING
TERMINATING
TERMINAL
```

### ACTIVE

One current connection_generation and current sufficient-control evidence; ordinary authorized commands may be accepted.

### CONTROL_SUSPECTED

Sufficient-control evidence is late but the accepted loss boundary is not yet crossed. No new GameSession and no protection merely from suspicion. A proven-safe routine transport replacement may occur without creating an unexpected-loss episode.

### RECONNECTABLE

Server classified eligible unexpected playable-control loss. Logical GameSession remains alive inside same-session grace, actor state is preserved and reconnect proof or accepted reauthenticated recovery proof may establish a new binding.

### TAKEOVER_DRAINING

Intentional authenticated replacement is permitted only because incumbent is logout-eligible. Incumbent remains the only current player-control authority until the committed fence/logout/handoff boundary. No disconnect protection is granted.

### TERMINATING

No new ordinary player commands; the session progresses to one bounded terminal outcome while required actor/world state remains authoritative.

### TERMINAL

GameSessionId never regains authority. Later player control requires a fresh GameSessionId. The actor may remain `PRESENT_UNCONTROLLED`.

## 5. Platform / game authority boundary

### Platform owns

- reusable account authentication/security policy;
- OAuth/PKCE/MFA/recovery;
- Game Login Ticket lifecycle;
- Platform account-security generation/revision;
- configured world/channel/login/maintenance/entitlement policy;
- Game Gateway route/offer orchestration;
- fresh-entry and reauthenticated-recovery attempt authorization;
- signing of accepted Platform-to-game grant profiles.

### Game domain owns

- final authoritative AccountId->CharacterId revalidation;
- AccountPresenceClaim;
- CharacterLease state/generation;
- current runtime owner/readiness/revision validation;
- grant consume/replay state;
- GameSessionId creation/terminality;
- connection_generation transitions;
- reconnect proof state;
- ControlLossEpoch/protection eligibility;
- final gameplay admission/recovery/takeover/handoff outcome.

Platform never creates canonical GameSessionId and never becomes gameplay authority by signing a grant.

## 6. Fresh-entry credential

Fresh entry uses only `docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md`.

Binding summary:

- JWS Compact JWT;
- fully specified JOSE `alg = Ed25519` only, per the accepted RFC 9864 direction;
- deprecated polymorphic `alg = EdDSA` is rejected;
- explicit `typ = oteryn-admission+jwt`;
- dedicated issuer/audience/key purpose;
- maximum grant lifetime 30 seconds;
- maximum verifier clock skew 5 seconds;
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

Fail-closed order:

1. FND-02 material/frame limits;
2. exact FND-04 fresh-entry profile and Ed25519 signature;
3. issuer/audience/type/purpose/time;
4. current Platform security projection freshness/revocation/generation;
5. route/runtime observation/current scope ownership generation;
6. protocol/content/ruleset/compatibility requirements;
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

Client-visible success is forbidden before commit.

A candidate GameSessionId generated before commit becomes canonical only if commit succeeds; otherwise it is discarded and never reused.

## 8. Account-global exclusion

Fresh admission across all worlds/channels/instances must linearize account exclusion.

Two different CharacterIds for one AccountId cannot both become playable/mandatory-presence actors.

Semantic acquisition order:

```text
AccountPresenceClaim
-> CharacterLease
-> GameSession
-> TransportBinding
```

DUR may realize the physical transaction differently, but no partial authority may become externally visible. Stale account-presence revision cannot overwrite a newer claim.

## 9. Duplicate login / takeover

### 9.1 Healthy combat/PZ/logout-locked incumbent

A second authenticated attempt MUST NOT:

- fence incumbent;
- close incumbent transport;
- revoke incumbent GameSession;
- release AccountPresenceClaim;
- admit another CharacterId.

Return a coarse conflict/session outcome.

### 9.2 Healthy logout-eligible incumbent

Intentional takeover:

1. authenticate newcomer;
2. prove takeover eligibility;
3. transition incumbent to `TAKEOVER_DRAINING`;
4. stop new incumbent ordinary commands at an explicit committed fence boundary;
5. complete legal logout/removal or accepted same-character handoff boundary;
6. release/advance account/character authority atomically;
7. create fresh GameSessionId;
8. create fresh connection_generation namespace at `1`;
9. grant no disconnect/re-entry protection.

No interval may contain two current player-control authorities.

### 9.3 Concurrent contenders

Concurrent fresh admission/takeover/recovery requests serialize through AccountPresenceClaim, CharacterLease and GameSession state. A stale loser cannot fence a newer successful winner.

## 10. Platform account-security freshness after issuance

FND-04 freezes the normal-path security model:

```text
short-lived signed grant
+ account_security_generation in the grant
+ trusted bounded-staleness Platform-security validity projection at game admission/recovery
```

Fresh admission and reauthenticated recovery require projection evidence age <= 5 seconds.

Fail closed if evidence is older, unavailable, unauthenticated, contradictory or unable to prove that the token generation remains admissible.

Reject if the account is disabled/revoked or token generation is below the current minimum-valid generation.

Online Platform introspection is not required on every normal path. A later implementation may add exceptional introspection in addition to the accepted projection.

This mechanism does not automatically terminate an already-admitted session. Post-admission emergency account/session revocation requires a separate game-domain fenced control contract.

## 11. AdmissionAttemptRef versus GrantNonce

FND-04 freezes:

```text
AdmissionAttemptRef
-> Platform producer operation/correlation/idempotency

GrantNonce
-> concrete signed capability game consume/replay identity
```

They are never aliases.

AdmissionAttemptRef:

- canonical UUIDv7 text in grant profiles;
- not a foundation entity ID;
- same logical producer retry uses same ref;
- ambiguous issuance cannot mint multiple independently usable grants for that attempt;
- independent new attempt uses new ref.

GrantNonce:

- 32 random bytes under the signed profile;
- at most one successful game authority transition;
- replay evidence retained through latest token acceptance time at minimum;
- consumed state never becomes reusable because a response was lost.

## 12. Fresh-entry route/runtime applicability

A fresh-entry grant is valid only for exact issuance-time target evidence accepted by the profile.

Game final admission always checks current authoritative state.

Default v1 rule:

```text
token.scope_ownership_generation != current scope ownership generation
-> reject stale grant
-> require fresh Gateway routing + fresh grant
```

Also reject stale/incompatible route revision, runtime observation revision, protocol/transport/compatibility revision or non-open current target lifecycle.

No silent retarget to another Channel, GameNode owner, protocol family or Canary route. NodeId is not a grant authority claim.

## 13. Reconnect secret

Each admitted GameSession receives game-domain reconnect proof material.

v1:

- exactly 32 cryptographically random bytes;
- only inside accepted TLS;
- stored server-side as a one-way verifier or equivalent secret-safe representation;
- never logged/traced/exported to analytics/rendered to users;
- scoped to one current GameSession/reconnect-proof state;
- rotated only through the PREPARE/COMMIT rebind state machine;
- GameSessionId alone never substitutes for it.

Exact verifier primitive is an implementation-security choice and may not reduce effective secret resistance below the accepted 256-bit random secret strength.

## 14. Reconnect PREPARE / COMMIT

FND-04 rejects rotate-and-forget reconnect and rejects PREPARE-time-only authorization.

### 14.1 ReconnectAttemptRef

Client creates a fresh cryptographically random 16-byte operation reference for one rebind attempt. It is not a foundation entity ID.

### 14.2 PREPARE

On a new TLS transport client presents:

```text
GameSessionId
current reconnect secret OR accepted reauthenticated recovery grant
ReconnectAttemptRef
```

Server validates session eligibility, current/old transport replacement status, CharacterLease/runtime/session reconciliation safety, proof and reconnect grace when relevant.

If accepted, reserve exactly one prepared transition:

```text
candidate connection_generation = current + 1
successor reconnect secret = new 32 random bytes
ReconnectAttemptRef
prepared_from_connection_generation = current
proof_class = reconnect_secret OR reauthenticated_recovery_grant
prepared expiry <= remaining same-session grace when grace applies
prepared expiry <= source recovery-grant acceptance window when recovery proof is used
```

Prepared state retains only the bounded non-secret/replay/security references required to revalidate the exact transition at COMMIT; it never logs or turns the raw recovery JWT/reconnect secret into analytics evidence.

PREPARE does **not** make the new transport authoritative, does not consume a recovery grant as a successful authority transition and does not advance current generation.

Retry with the same proof + same ReconnectAttemptRef obtains the same prepared outcome while valid. Competing different attempt cannot create a second simultaneous prepared winner.

Client retains predecessor proof until COMMIT is acknowledged or authoritative evidence shows successor committed.

### 14.3 COMMIT

Client proves possession of the prepared successor secret on the prepared TLS transport.

Possession of the successor secret is necessary but never sufficient. Immediately before any authority change, the game-domain owner MUST atomically revalidate the prepared transition against current authoritative state.

For every proof class COMMIT requires:

- prepared state exists, is unexpired and is still the unique current prepared transition for this GameSession;
- current GameSession is non-terminal and still eligible for the same rebind class;
- `prepared_from_connection_generation` still equals the current predecessor generation expected by PREPARE;
- AccountPresenceClaim still names the same CharacterId/account relationship;
- current CharacterLease generation/state remains valid for this actor;
- current runtime scope ownership generation/placement remains current and unambiguous;
- FND-02 command/session/server-sequence/snapshot reconciliation state remains safe for same-GameSession continuation;
- no newer fencing, takeover, handoff, terminality or ownership transition has superseded PREPARE;
- the incumbent has not regained sufficient healthy current-generation playable-control authority. A reconnect secret, recovery grant, prepared successor secret or PREPARE alone cannot evict a healthy current binding. Any future healthy-session migration requires a separately current-generation-authorized transition not defined by v1.

When PREPARE used a reauthenticated recovery grant, COMMIT additionally requires:

- the JWT remains within its accepted time/skew window;
- the RecoveryGrantNonce remains unconsumed/eligible and is consumed exactly once only by the successful COMMIT;
- trusted Platform-security evidence is still authenticated and <=5 seconds old;
- account disabled/revoked/minimum-security-generation policy still admits the grant;
- current AccountId->CharacterId ownership still matches;
- same-session grace remains unexpired.

If any COMMIT revalidation fails:

```text
current TransportBinding remains authoritative
current connection_generation does not advance
predecessor reconnect proof remains current
prepared candidate is cancelled/terminalized
prepared successor secret never becomes current proof
recovery nonce is not consumed as a successful transition
no partial gameplay/session/lease/runtime authority mutation is committed
```

The client must reconcile current authority and, where the cause requires it, obtain a fresh proof/grant/route. A stale prepared transition can never be revived merely by replaying the successor secret.

Only after all required revalidation succeeds does the server atomically commit:

```text
candidate connection_generation becomes current
prepared transport becomes current TransportBinding
successor reconnect secret becomes current proof
predecessor proof becomes stale
old transport/generation loses command/liveness/reconciliation authority
RecoveryGrantNonce is consumed when recovery proof was the authorization source
prepared state becomes committed/terminal
```

Only then is the new transport authoritative.

### 14.4 Lost PREPARE response

Predecessor remains current before COMMIT. Retry same ReconnectAttemptRef obtains the same prepared transition if still valid. No duplicate candidate is minted.

### 14.5 Crash after PREPARE before COMMIT

PREPARE alone creates no new authority. If prepared state is lost, predecessor remains current and can be retried subject to eligibility. If prepared state is durable, recovery still preserves one candidate and no generation change before COMMIT.

A recovered prepared transition is never trusted solely because it was durable; all Section 14.3 COMMIT-time current-authority/security checks still apply.

### 14.6 Lost COMMIT response / crash around COMMIT

Client already knows successor secret from PREPARE.

If COMMIT succeeded, recoverable state must show the new connection_generation, successor verifier and, where applicable, consumed RecoveryGrantNonce; predecessor can never regain authority.

If COMMIT did not succeed, predecessor remains current and any partially observed candidate has no authority.

Recovery must prove one of those states; never accept both or guess.

If exact state cannot be reconstructed after GameNode replacement, same-GameSession continuity is not claimed and the session follows safe fresh-session recovery rules.

## 15. Reconnect replay/concurrency

- same current reconnect proof raced by two attempts -> one PREPARE owner/winner at most;
- stale predecessor after COMMIT -> reject, cannot fence successor;
- stale connection_generation -> FND-02 `STALE_GENERATION` behavior;
- replayed consumed recovery grant -> no second rebind/session;
- prepared transition whose eligibility/security changed before COMMIT -> terminalize/cancel without authority change;
- incumbent that regains sufficient current-generation control before COMMIT remains authoritative; prepared contender cannot fence it;
- one current prepared rebind per GameSession is the v1 semantic maximum.

Concrete prepared-state bytes/retention/resource limits must be registered before implementation acceptance.

## 16. Sufficient playable-control/liveness evidence

Primary sufficient evidence is a valid current-generation response to a recent server-issued authenticated liveness probe, bound to:

- GameSessionId;
- current connection_generation;
- probe identity;
- server-observed round-trip progress;
- runtime-health context.

Not sufficient:

- socket-open state;
- client wall-clock timestamp;
- stale-generation acknowledgement;
- arbitrary one-way bytes;
- gameplay-command silence/presence by itself.

Other bidirectional current-generation control exchanges may count only when a later exact contract proves equivalent evidence semantics.

### Numeric liveness evidence gate

FND-04 does not invent production probe cadence.

Before implementation acceptance a registered liveness profile must provide concrete interval/hysteresis validated by latency/load/packet-loss/fault tests and satisfy at least:

```text
probe_interval < 0.5 * 2.0-second loss threshold
```

with measured margin for network/scheduler jitter.

The 2.0-second behavioral loss boundary remains accepted.

## 17. Exact same-session grace

FND-04 freezes:

```text
T0 = last accepted sufficient current-generation control evidence
control_loss_declared_at = T0 + 2.0 seconds
stale_concrete_transport_cleanup = T0 + 5.0 seconds
same_session_grace_expires = control_loss_declared_at + 15.0 seconds
```

Thus same GameSession receives a full 15-second reconnect window after server-authoritative loss classification.

The 5-second concrete transport cleanup does not terminate the logical GameSession.

If control is restored/rebound before `control_loss_declared_at`, no unexpected-loss episode is created and no 4-second re-entry protection is granted merely because connection_generation changed.

At grace expiry an unrecovered GameSession progresses to TERMINAL.

## 18. ControlLossEpoch and re-entry protection

FND-04 freezes an internal actor/session `ControlLossEpoch` revision/state; it is not a foundation entity ID.

Rules:

- created only when server classifies eligible unexpected playable-control loss;
- one protection activation maximum per epoch;
- routine rebind, graceful logout, intentional takeover or grant issuance does not create it;
- stale/replayed reconnect attempt does not create/restart it;
- replacing GameSession during the same loss episode does not reset it;
- FND-03 executes accepted 4-second PvE effect after FND-04 marks one eligible re-entry;
- once consumed, later rebinds in the same epoch cannot restart protection.

A later loss becomes a new epoch only after the actor/session reaches registered `STABLE_ACTIVE` liveness state.

Exact anti-flap hysteresis for `STABLE_ACTIVE` is a measured liveness/security-policy value that must be concrete before implementation acceptance; it is not guessed here.

## 19. Reauthenticated recovery credential

Loss of reconnect secret may use only `docs/contracts/FND-04_REAUTHENTICATED_RECOVERY_GRANT_PROFILE_V1.md`.

It is cryptographically/profile-separated from fresh entry:

- fully specified `alg = Ed25519`;
- distinct `typ`, issuer, audience, purpose and key purpose;
- no ChannelId/InstanceId authority;
- current actor/session placement resolved by Oteryn-v2.

Platform may require MFA/step-up/risk policy before issuance.

## 20. Same-GameSession reauthenticated recovery inside grace

A valid recovery grant may substitute for missing reconnect secret only if game state proves:

- current GameSession is `RECONNECTABLE`;
- grace not expired;
- no healthy current playable controller;
- current AccountId owns CharacterId;
- AccountPresenceClaim still same CharacterId;
- CharacterLease/runtime authority is safe;
- current actor placement resolved by game domain;
- FND-02 command/session reconciliation state remains reconstructable.

PREPARE records a candidate only. The recovery grant remains subject to all Section 14.3 COMMIT-time token, Platform-security, incumbent-health, lease/runtime/session/reconciliation and grace revalidation.

Success:

- consumes RecoveryGrantNonce exactly once at successful COMMIT;
- uses reconnect PREPARE/COMMIT;
- preserves GameSessionId;
- commits one newer connection_generation;
- creates/rotates reconnect proof;
- may consume the current ControlLossEpoch's one protection activation if eligible.

A healthy incumbent cannot be preempted by a recovery JWT, reconnect secret or prepared successor secret.

## 21. Post-grace same-character existing-actor recovery

FND-04 accepts this native behavior.

At grace expiry:

```text
old GameSession -> TERMINAL
```

If gameplay rules keep actor present:

```text
actor -> PRESENT_UNCONTROLLED
AccountPresenceClaim -> still same CharacterId
CharacterLease/runtime actor -> remains current under existing fencing
```

A fresh valid reauthenticated recovery grant may create a **new GameSessionId** attached to the exact existing actor only when:

- old GameSession terminal;
- no current playable controller;
- current AccountId->CharacterId ownership matches;
- AccountPresenceClaim remains same CharacterId;
- CharacterLease/runtime scope is current/safe;
- current game-domain placement is unambiguous;
- current compatibility/reconciliation supports a fresh authoritative snapshot/session boundary.

Commit creates:

```text
new GameSessionId
connection_generation = 1
new reconnect secret
new session command/reconciliation namespace
control attached to same existing actor
```

It MUST NOT respawn/recreate/heal/reset/teleport actor, clear conditions/cooldowns/combat/PZ/threat/encounter state, open an AccountPresenceClaim race window or duplicate inventory/state.

If current placement/lease/actor state cannot be proven, fail closed.

A different CharacterId remains blocked until original actor is legally absent.

## 22. Recovery locator/current placement

Reauthenticated recovery requires a game-domain locator/dispatcher.

Input concept:

```text
AccountId + CharacterId + WorldId + recovery-attempt correlation
```

It resolves current actor presence, GameSession state, CharacterLease, ChannelRuntime/InstanceRuntime placement and current ownership generation.

Rules:

- Platform configured ChannelId is not actor placement authority;
- stale client route never moves actor;
- actor in InstanceRuntime remains there unless accepted handoff changes ownership;
- ambiguous/suspected/no-current-owner evidence fails closed;
- private topology/fencing need not be exposed to client;
- exact API/transport/deployment remains implementation/OPS work.

## 23. Channel/Instance handoff

### 23.1 Continuous Channel<->Instance activity

When transition preserves one logical control session:

- AccountPresenceClaim remains continuously held;
- CharacterLease may remain when writer transfer is properly fenced;
- GameSessionId may remain;
- connection_generation may remain if concrete TransportBinding does not rebind;
- HandoffId identifies ownership transition;
- source loses mutation authority at committed handoff barrier;
- destination becomes current before normal destination deltas continue.

### 23.2 Channel->Channel fresh logical session

When accepted transition establishes a fresh logical GameSession:

- fresh destination authorization required;
- fresh GameSessionId;
- source session drains/terminates under handoff rules;
- AccountPresenceClaim remains atomically continuous for same CharacterId;
- CharacterLease generation advances only when writer-fence transition requires it;
- no old grant/route is silently retargeted.

These session-continuity classes are frozen; later gameplay routing defines which activity belongs to each class.

## 24. CharacterLease renewal/fail-safe

Binding rules:

- only current generation may renew;
- sent renewal is not proof of renewal;
- authoritative lease-store/ownership state defines accepted expiry;
- local holder must fail safe before another generation can legitimately become authoritative;
- renewal uncertainty does not release AccountPresenceClaim;
- renewal uncertainty does not self-grant a replacement writer;
- stale generation cannot commit durable character/player mutation;
- server-originated in-memory effects may continue only when FND-03/DUR prove no competing writer can exist;
- replacement generation requires explicit fence/recovery evidence.

### Numeric lease evidence gate

Before implementation acceptance DUR/OPS/PERF fault injection must freeze:

- authoritative lease TTL;
- renew interval;
- local safety margin;
- maximum assumed dependency/network/clock uncertainty;
- fail-safe deadline;
- replacement/fencing timing.

Minimum relationships:

```text
renew_interval < lease_TTL / 3
local_fail_safe_deadline < authoritative_expiry
safety_margin > measured worst-case uncertainty used by the safety proof
```

No production library/default/infinite values absent this evidence.

## 25. GameNode replacement / same-session continuity

New NodeId does not automatically end GameSession, but same-session continuity is permitted only if current authority can safely reconstruct:

- GameSession state/terminality;
- current connection_generation;
- reconnect current/prepared/committed proof state;
- CommandId high-water/pending/result state;
- server-sequence/snapshot reconciliation boundary;
- AccountPresenceClaim;
- CharacterLease current generation/state;
- current ControlLossEpoch/protection-consumed state;
- current runtime ownership generation/placement.

If any required state cannot be proven, do not guess same-session continuity. Safely terminate old GameSession and use accepted fresh-session existing-actor recovery when actor state is valid, otherwise remain fail closed.

## 26. Key rotation/revocation

Both grant profiles require:

- dedicated asymmetric verification key purpose;
- fully specified `Ed25519` algorithm;
- trusted allowlisted key source;
- exact `kid` lookup only in trusted provisioned/configured set;
- bounded current/retiring overlap for still-valid grants;
- emergency key revocation;
- no acceptance merely because a key is mathematically valid outside trusted policy;
- mixed profile/producer/consumer revisions fail closed.

Production KMS/HSM, key generation, rollout and cadence remain later security-operations implementation work.

## 27. Stable internal error codes / public classes

Stable symbolic internal codes are frozen; numeric wire allocation follows later FND-02 registry work if/when exposed.

Every FND-04 cross-component failure obeys `FOUNDATION_ERROR_VOCABULARY.md`. `TERMINAL` means terminal for the current operation/credential/transition, not necessarily terminal for the account or actor. `SECURITY_TERMINAL` forbids retrying the same suspect credential/proof. `RETRYABLE` is always bounded by the named current authority/expiry/deadline; it never permits silent authority replacement.

| Internal code | Category | Progression | Permitted retry / next authority | Idempotency / partial-mutation outcome | Coarse public class |
|---|---|---|---|---|---|
| `ADMISSION_GRANT_MALFORMED` | `INVALID_INPUT` | `TERMINAL` | corrected/new admission material only | no grant consume, session, lease or presence mutation | `RETRY_LOGIN` |
| `ADMISSION_GRANT_AUTHENTICATION_FAILED` | `AUTHENTICATION_FAILED` | `SECURITY_TERMINAL` | never same grant; obtain new authenticated authorization | no authoritative mutation | `AUTHENTICATION_REQUIRED` |
| `ADMISSION_GRANT_EXPIRED` | `SESSION_REJECTED` | `TERMINAL` | fresh Gateway/grant | expired grant remains unusable; no authoritative mutation | `RETRY_LOGIN` |
| `ADMISSION_GRANT_REPLAYED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` for that grant | never retry same grant; reconcile prior result or obtain fresh authorization | no second consume/effect; prior committed outcome, if any, remains authoritative | `RETRY_LOGIN` |
| `ADMISSION_GRANT_SECURITY_STATE_REVOKED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | fresh authentication/security resolution required | no admission mutation | `AUTHENTICATION_REQUIRED` |
| `ADMISSION_GRANT_SECURITY_EVIDENCE_STALE` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same still-valid/unconsumed grant only after fresh trusted evidence; otherwise fresh grant | no authoritative mutation | `TEMPORARILY_UNAVAILABLE` |
| `ADMISSION_GRANT_ROUTE_STALE` | `STALE_GENERATION` | `TERMINAL` for grant | fresh Gateway route + fresh grant | no mutation; stale route cannot retarget | `RETRY_LOGIN` |
| `ADMISSION_GRANT_RUNTIME_GENERATION_STALE` | `STALE_GENERATION` | `TERMINAL` for grant | fresh current-owner route + fresh grant | no mutation; stale owner generation cannot admit | `RETRY_LOGIN` |
| `ADMISSION_GRANT_REVISION_UNSUPPORTED` | `UNSUPPORTED_REVISION` | `TERMINAL` | compatible client/route/profile then new attempt | no downgrade and no mutation | `CLIENT_UPDATE_REQUIRED` |
| `ADMISSION_ACCOUNT_CHARACTER_CONFLICT` | `CONFLICT` | `TERMINAL` for attempt | new attempt only after authoritative ownership/lifecycle state changes | no partial admission | `SESSION_UNAVAILABLE` |
| `ADMISSION_INCUMBENT_PROTECTED` | `CONFLICT` | `TERMINAL` for attempt | later new attempt after incumbent becomes legally replaceable/absent | incumbent unchanged; newcomer gets no authority | `CHARACTER_ALREADY_ACTIVE` |
| `ADMISSION_CAPACITY_EXCEEDED` | `CAPACITY_EXCEEDED` | `RETRYABLE` | bounded retry while same grant remains valid/unconsumed and target remains current; otherwise fresh route/grant | no partial admission/reservation authority | `TEMPORARILY_UNAVAILABLE` |
| `RECONNECT_PROOF_INVALID` | `AUTHENTICATION_FAILED` | `SECURITY_TERMINAL` | never retry same invalid proof; use valid current proof or reauthenticated recovery | current binding unchanged | `AUTHENTICATION_REQUIRED` |
| `RECONNECT_PROOF_REPLAYED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` for stale proof | reconcile current binding; never re-execute stale proof | no generation rollback/second transition | `SESSION_UNAVAILABLE` |
| `RECONNECT_SESSION_TERMINAL` | `SESSION_REJECTED` | `TERMINAL` | fresh-session existing-actor recovery or fresh admission only if independently eligible | terminal GameSession never revives | `RETRY_LOGIN` |
| `RECONNECT_GENERATION_STALE` | `STALE_GENERATION` | `TERMINAL` for stale transport/attempt | reconcile to current generation; new proof path if eligible | no current-generation mutation | `SESSION_UNAVAILABLE` |
| `RECONNECT_ATTEMPT_CONFLICT` | `CONFLICT` | `RETRYABLE` only through reconciliation | same winning ReconnectAttemptRef may be queried/retried; competing attempt cannot create another candidate | at most one prepared winner; no authority change before successful COMMIT | `SESSION_UNAVAILABLE` |
| `RECONNECT_GRACE_EXPIRED` | `SESSION_REJECTED` | `TERMINAL` for same-session reconnect | reauthenticated post-grace existing-actor recovery if actor remains eligible | old GameSession progresses terminal; no rebind | `RETRY_LOGIN` |
| `RECOVERY_GRANT_REPLAYED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` for that grant | never retry same grant; reconcile committed state or obtain fresh reauthentication | no second consume/rebind/session/protection | `AUTHENTICATION_REQUIRED` |
| `RECOVERY_HEALTHY_CONTROLLER_PRESENT` | `CONFLICT` | `TERMINAL` for recovery attempt | no secret/JWT preemption; later explicit takeover/migration or new recovery after genuine loss | incumbent remains current; prepared candidate cancelled | `CHARACTER_ALREADY_ACTIVE` |
| `RECOVERY_PLACEMENT_UNAVAILABLE` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same still-valid/unconsumed grant may retry after authoritative locator recovers; otherwise fresh recovery grant | no placement guess and no authority mutation | `TEMPORARILY_UNAVAILABLE` |
| `RECOVERY_STATE_UNSAFE` | `INTERNAL_UNAVAILABLE` | `TERMINAL` for current transition | authoritative reconciliation/recovery first; then a new eligible attempt | fail closed; no partial control mutation | `SESSION_UNAVAILABLE` |
| `CHARACTER_LEASE_STALE` | `STALE_GENERATION` | `TERMINAL` for stale holder/transition | reread current authority; only current generation may proceed | stale generation commits nothing | `SESSION_UNAVAILABLE` |
| `CHARACTER_LEASE_RENEW_TIMEOUT` | `TIMEOUT` | `RETRYABLE` only before current holder's fail-safe deadline | bounded renewal retry under same current generation; after fail-safe deadline holder must stop authority and use recovery/fencing path | timeout never grants replacement writer or releases presence | `TEMPORARILY_UNAVAILABLE` |
| `CHARACTER_LEASE_DEPENDENCY_UNAVAILABLE` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` only inside accepted safety window | bounded same-generation retry; no replacement until explicit fencing proof | no authority promotion; fail safe at deadline | `TEMPORARILY_UNAVAILABLE` |
| `SESSION_TAKEOVER_NOT_ALLOWED` | `CONFLICT` | `TERMINAL` for attempt | later new takeover only after current authoritative eligibility changes | incumbent remains current; no partial takeover | `CHARACTER_ALREADY_ACTIVE` |

COMMIT-time revalidation failures use the most specific code above for the changed fact. For example, recovered incumbent health uses `RECOVERY_HEALTHY_CONTROLLER_PRESENT`, expired recovery authorization follows the relevant terminal recovery/admission rejection, stale runtime/lease generations use their stale-generation code, and unavailable current placement uses `RECOVERY_PLACEMENT_UNAVAILABLE`. A failed COMMIT terminalizes/cancels the prepared candidate and never changes current generation, proof, lease or player authority.

Redacted diagnostics may include safe correlation IDs, profile/revision identifiers and cause classes required for operators, but never credentials, raw JWTs/nonces/secrets, Platform security-generation internals, private fencing data, SQL errors or unstable implementation strings.

## 28. Foundation failure scenarios

FND-04 contract-level disposition:

| Scenario | Status | FND-04 requirement / owner |
|---|---|---|
| `FS-PLATFORM-UNAVAILABLE` | `PASS` | new Platform-dependent grant issuance fails/holds boundedly; no alternate credential authority; active gameplay/game-domain fast reconnect not invalidated merely by Platform outage |
| `FS-GATEWAY-AFTER-REDEEM` | `PASS` | AdmissionAttemptRef idempotency; no blind second capability; no GameSession absent game commit |
| `FS-POSTGRES-UNAVAILABLE` | `DEFERRED_BY_ACCEPTED_GATE` | DUR physical persistence; no success when required atomic/fenced state cannot be proven |
| `FS-LEASE-RENEW-TIMEOUT` | `PASS` | old writer fails closed before replacement; timeout never self-grants new writer |
| `FS-DUPLICATE-LOGIN` | `PASS` | account-global exclusion + healthy incumbent protection + one winner |
| `FS-STALE-GENERATION` | `PASS` | stale connection/lease/runtime generation cannot command/recover/commit |
| `FS-DUPLICATE-COMMAND` | `NOT_APPLICABLE` | FND-02 remains authority |
| `FS-CHANNEL-SPLIT-OWNER` | `PASS` | current runtime generation + FND-04 route/lease checks prevent stale admission/control; physical fencing continues under OPS/DUR |
| `FS-CHANNEL-DRAIN` | `PASS` | no new admission to non-open/draining target; current session/handoff follows FND-03 drain barrier |
| `FS-QUEUE-SATURATION` | `DEFERRED_BY_ACCEPTED_GATE` | runtime/resource limits; authority transition fails before partial commit |
| `FS-SLOW-CLIENT` | `PASS` | transport may close while logical GameSession follows FND-04 reconnect semantics |
| `FS-CLOCK-SKEW` | `PASS` | grant max 5s skew; liveness/grace uses server monotonic time |
| `FS-KEY-ROTATION` | `PASS` | dedicated purpose, bounded overlap, emergency revocation, fail-closed unknown/deprecated algorithm/profile |
| `FS-REVISION-MISMATCH` | `PASS` | no profile/protocol/route/runtime downgrade |
| `FS-SNAPSHOT-DELTA-MISMATCH` | `NOT_APPLICABLE` | FND-02/FND-03 reconciliation |
| `FS-DB-OUTBOX-BOUNDARY` | `DEFERRED_BY_ACCEPTED_GATE` | DUR/ANL physical atomic evidence; admission success cannot precede required durable commit |
| `FS-WORLD-BUNDLE-CORRUPT` | `NOT_APPLICABLE` | invalid target must already be unroutable |
| `FS-CLIENT-CUTOVER-ROLLBACK` | `NOT_APPLICABLE` | historical migration lifecycle |
| `FS-ANALYTICS-TELEMETRY-OVERFLOW` | `NOT_APPLICABLE` | telemetry never session authority |
| `FS-AUDIT-OUTBOX-BACKLOG` | `DEFERRED_BY_ACCEPTED_GATE` | ANL/DUR required security audit cannot silently degrade |
| `FS-EVENT-DUPLICATE-DELIVERY` | `NOT_APPLICABLE` | analytics replay cannot alter session state |
| `FS-EVENT-OUT-OF-ORDER` | `NOT_APPLICABLE` | analytics order cannot alter session state |
| `FS-AUDIT-MUTATION-MISMATCH` | `DEFERRED_BY_ACCEPTED_GATE` | ANL/DUR atomic evidence |
| `FS-ANALYTICS-PRIVACY-POLICY` | `NOT_APPLICABLE` | credentials excluded from analytics |
| `FS-DETECTOR-FALSE-POSITIVE` | `NOT_APPLICABLE` | analytics cannot sanction/revoke autonomously |
| `FS-INVESTIGATION-MUTATION-ATTEMPT` | `NOT_APPLICABLE` | investigation cannot mutate session/runtime authority |
| `FS-ADMISSION-GRANT-REPLAY` | `PASS` | one GrantNonce <= one successful admission; losing replay cannot create/revive/fence another GameSession |
| `FS-RECONNECT-CREDENTIAL-REPLAY` | `PASS` | PREPARE/COMMIT + COMMIT-time current-authority/security revalidation gives one winner; stale/prepared proof cannot regain authority/fence a recovered healthy successor/incumbent |

`PASS` means architecture invariant exists, not executable proof.

## 29. Cross-repository compatibility

Production implementation requires an explicit compatibility lock/matrix across:

- Oteryn-v2 FND-04 contract revision;
- fresh-entry grant profile v1;
- recovery grant profile v1;
- Oteryn Platform producer revisions;
- Platform account-security projection revision;
- runtime-status observation revision;
- FND-02 protocol major/transport profile;
- current runtime/content compatibility revision.

Independent fixtures cover positive and negative profile cases plus session/reconnect/fencing faults.

Producer may not emit a mandatory new security field before target consumers understand/reject it according to rollout plan. No silent downgrade.

## 30. Required implementation evidence gates

Architecture is complete at semantic/security level. Implementation claims remain blocked until concrete evidence exists.

### Liveness profile

Must freeze exact probe interval, hysteresis/STABLE_ACTIVE rule, latency/load/packet-loss/fault evidence, scheduler margin and false-positive/negative expectations.

### Character lease profile

Must freeze TTL, renew cadence, safety margin, maximum uncertainty, fail-safe deadline and replacement/fencing timing with fault injection.

### Session/reconnect resource limits

Register hard maxima for:

- prepared rebinds per GameSession (v1 semantic max one current prepared rebind);
- prepared-state bytes/time retention;
- grant/recovery replay records;
- admission/recovery attempts per relevant abuse scope;
- Platform-security projection state;
- recovery-locator outstanding work;
- terminal/reconciliation receipt retention.

### Crypto/interoperability evidence

Require independent PHP producer/Rust consumer fixtures, malformed/algorithm-confusion corpus including deprecated `EdDSA`, UUIDv7/version/variant rejection cases, key rotation/revocation, mixed-version rejection, replay/concurrency and credential-redaction tests.

### Reconnect authority race evidence

Before implementation acceptance, deterministic/fault tests must prove at minimum:

- PREPARE then incumbent liveness recovery before COMMIT cannot fence the incumbent;
- PREPARE then recovery-grant expiry, revocation or account-security-generation advancement cannot COMMIT;
- PREPARE then CharacterLease/runtime ownership/session/reconciliation change cannot COMMIT stale authority;
- COMMIT revalidation and authority switch are one atomic linearization boundary;
- failed COMMIT leaves predecessor generation/proof/current authority unchanged and the candidate non-revivable;
- lost COMMIT response/crash resolves to exactly predecessor-current or successor-current, never both.

No production defaults are inferred from application-library defaults.

## 31. Security/privacy summary

Never log/expose reusable credentials, Game Login Ticket, raw grant JWT, GrantNonce/RecoveryGrantNonce, reconnect secret, private key or verifier digest.

GameSessionId/AccountId/CharacterId are identifiers, not credentials.

High-cardinality IDs do not become ordinary metric labels.

Client/OS diagnostics never authorize admission/reconnect or advance liveness.

## 32. Rejected alternatives

- Platform creates GameSessionId — rejected; violates game-domain final admission authority.
- Game server accepts OAuth/password — rejected; duplicates reusable-credential authority.
- GameSessionId as reconnect secret — rejected; identity is not proof.
- One generic JWT for fresh entry and recovery — rejected; routing/authority semantics differ; mutually exclusive validation is safer.
- Deprecated polymorphic `alg=EdDSA` in a new v1 profile — rejected; RFC 9864 provides fully specified `Ed25519` and deprecates the polymorphic JOSE identifier.
- Reuse fresh-entry ChannelId grant for existing actor recovery — rejected; stale route could move actor/bypass Instance placement.
- Pure self-contained JWT with expiry only — rejected; insufficient post-issuance Platform-security invalidation semantics.
- Platform introspection required for every fast reconnect — rejected; unnecessary Platform dependency for admitted game-domain continuity.
- Rotate reconnect secret and forget predecessor before client obtains successor — rejected; lost-response ambiguity.
- PREPARE makes new transport authoritative — rejected; ambiguity before successor proof/commit.
- PREPARE-time-only validation — rejected; eligibility/security may change before COMMIT and stale prepared authority cannot fence a current controller.
- Reconnect/recovery secret alone preempts a healthy current binding — rejected; possession is not current-generation migration authorization.
- Lease expiry automatically grants replacement writer — rejected; split-brain/stale-writer/combat-abuse risk.
- GameSession terminality releases account presence immediately — rejected; actor may remain mandatory in world.
- Duplicate login kicks healthy combat-locked incumbent — rejected.

## 33. Downstream ownership

### DUR

Physical AccountPresence/CharacterLease/GameSession persistence, atomicity/isolation/recovery, prepared-rebind durability choice, replay-store implementation and durable gameplay safety.

### OPS/PERF

Measured lease/liveness/placement capacities, failure detection, production rollout/drain/recovery objectives and runtime hard limits.

### Oteryn Platform

Later separately authorized producer implementation for both accepted grant profiles, Platform-security validity projection and runtime-status consumer integration.

### FND-02 / protocol implementation

Later registers exact admission/reconnect/recovery message types and numeric error codes without changing FND-04 semantics.

### ANL / Game Intelligence

May consume bounded security/audit evidence, never raw credentials and never automatic gameplay/session mutation authority.

## 34. Acceptance boundary

When this contract and both grant profiles merge:

- FND-04 architecture gate is complete;
- Identity/GameSession/admission/reconnect/account-presence/CharacterLease semantics are frozen;
- native implementation is still **not authorized** by this merge alone;
- implementation requires separate tasks plus Section 30 evidence gates;
- Platform producer rollout requires separate authorized Platform task/PR;
- DUR/OPS/PERF/ANL gates remain independently required.

## 35. Canonical concise rule

```text
Platform authenticates
-> strict signed bounded attempt capability
-> never GameSession authority

fresh entry
-> Ed25519 fresh-entry grant
-> current Platform-security evidence
-> current route/runtime owner evidence
-> one-time GrantNonce
-> AccountId->CharacterId revalidation
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
-> reconnect secret OR strict Ed25519 recovery grant
-> current placement resolved by Oteryn-v2
-> PREPARE candidate generation + successor secret
-> COMMIT successor proof + atomic current-authority/security revalidation
-> healthy incumbent / expired-revoked grant / stale lease-runtime-session state cancels candidate without fencing
-> exactly one current generation
-> same GameSessionId
-> one protection activation per eligible ControlLossEpoch

post-grace actor mandatory
-> old GameSession terminal
-> actor PRESENT_UNCONTROLLED
-> AccountPresenceClaim remains same CharacterId
-> recovery grant may create fresh GameSessionId attached to same actor
-> no reset/respawn/teleport/heal
-> different CharacterId blocked

lease uncertainty
-> no automatic replacement
-> old writer fails closed when authority cannot be proven
-> replacement only after explicit fence/recovery

performance-sensitive liveness/lease/capacity values
-> measured registered evidence before implementation acceptance
-> never guessed defaults
```
