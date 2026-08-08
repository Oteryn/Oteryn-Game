# FND-04 — Rebind Security, Decision Timing and Failure Progression Refinement

- Status: Candidate normative FND-04 refinement; canonical when the owning FND-04 delivery merges
- Date: 2026-08-08
- Gate: `FND-04`
- Refines: `FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md`, especially Sections 4, 14, 15, 20, 27, 30 and 33
- Applies to: same-GameSession transport replacement, FND-04 decision timing and contract-owned cross-component failure progression
- Does not authorize: runtime/protocol implementation, transport migration feature, Platform writes or production traffic

Where this refinement is more specific, it is normative for the final FND-04 package and supersedes the less-specific candidate wording in the referenced sections of the main FND-04 contract.

## 1. Security problem

A reconnect secret is a high-entropy bearer proof. A reauthenticated recovery grant is a stronger Platform-authenticated attempt proof. Neither one, by itself, is permission to evict a healthy current playable transport.

Without explicit PREPARE and COMMIT eligibility rules, a stolen predecessor reconnect secret or separately authenticated second client could prepare while recovery is eligible and then commit after the incumbent becomes healthy again, after lease/runtime authority changes or after a recovery JWT/security projection is no longer valid.

FND-04 therefore freezes both the current-binding eligibility rule and the COMMIT-time revalidation rule below.

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

## 4. COMMIT revalidates authority; PREPARE is never authorization escrow

This section normatively tightens main-contract Section 14.3.

Possession of the prepared successor secret proves only that the caller received the prepared candidate. It does **not** freeze the authority facts that made PREPARE eligible.

Immediately before and atomically with any authority switch, COMMIT MUST revalidate all applicable current facts:

1. the prepared transition exists, is not expired and is bound to the exact GameSessionId, ReconnectAttemptRef, prepared TLS transport and candidate connection_generation;
2. the currently authoritative predecessor connection_generation is exactly the generation from which this transition was prepared; a later committed generation makes the candidate stale;
3. the GameSession remains in a state that permits this exact rebind and has not become TERMINATING/TERMINAL or entered an incompatible takeover/handoff transition;
4. the AccountPresenceClaim still denotes the same CharacterId and no newer account-presence revision superseded the prepared transition;
5. the CharacterLease generation/current authority is still the same compatible authority or has been revalidated under an explicitly accepted equivalent transition;
6. current RuntimeScopeAuthority, placement and FND-02 reconciliation state still permit the rebind;
7. the incumbent current generation has **not** regained sufficient playable-control authority after PREPARE; if it has, an unsolicited reconnect/recovery COMMIT is rejected unless a separately accepted current-generation-authorized healthy-migration transition applies;
8. same-session reconnect grace remains valid when that grace is required for the transition;
9. any proof-class-specific security state remains valid as specified below.

### 4.1 Fast reconnect-secret PREPARE

A reconnect-secret PREPARE remains a game-domain continuity path. COMMIT does not invent a new synchronous Platform dependency and does not treat a later Platform account-security change as implicit post-admission revocation, because FND-04 assigns such revocation to a separate fenced game-domain control contract.

It nevertheless revalidates every current game-domain condition in Section 4, including incumbent liveness, GameSession state, AccountPresenceClaim, CharacterLease, runtime ownership/placement, reconciliation state and grace.

### 4.2 Reauthenticated recovery-grant PREPARE

When PREPARE used `oteryn-reauth-recovery-v1`, COMMIT additionally MUST revalidate, in the same authority-changing boundary:

- the recovery JWT is still inside its accepted time/skew window;
- its RecoveryGrantNonce remains eligible for this exact idempotent transition and has not been consumed by another successful transition;
- current trusted Platform-security evidence is still within the accepted <=5-second freshness bound;
- the account remains admissible and the grant's account_security_generation remains current enough under that projection;
- key/profile revocation state still accepts the grant.

The RecoveryGrantNonce is consumed atomically with the successful authority transition. PREPARE alone does not turn an expiring/revoked recovery grant into durable replacement authority.

### 4.3 COMMIT failure and idempotency

If any required COMMIT-time condition fails before the authority switch:

```text
connection_generation does not advance
predecessor/current authoritative binding is not fenced by this failed attempt
successor reconnect secret never becomes current proof
prepared candidate becomes aborted/terminal or expires under one unambiguous state
no ControlLossEpoch/protection is manufactured
```

A retry of the same ReconnectAttemptRef may return the already-committed result when COMMIT previously succeeded, or the stable aborted/expired result when it did not. It must never reinterpret an aborted candidate as new authority.

This closes the PREPARE-to-COMMIT race: prepared proof cannot evict an incumbent that became healthy again and cannot outlive the authorization, lease/runtime or recovery-security state that the successful COMMIT requires.

## 5. Pre-loss / healthy transport migration is a distinct transition

FND-04 does not prohibit a future seamless migration of a healthy session to another transport/device/path, but it must not be implemented as an unsolicited bearer-secret reconnect.

A controlled healthy-binding migration requires an explicit authorization rooted in the **current authoritative connection_generation**, such as a server-issued one-time migration challenge/intent acknowledged by the current binding or another separately accepted equivalent proof.

Minimum invariants for any later migration contract:

- current binding participates in or explicitly authorizes the migration while still authoritative;
- authorization is bound to GameSessionId, current connection_generation, destination attempt and a short bounded lifetime;
- one migration attempt has at most one winner;
- PREPARE still grants no destination command/liveness authority;
- COMMIT atomically revalidates current-generation authorization and switches generation/current binding while fencing predecessor;
- a stale migration authorization cannot preempt a later generation;
- intentional healthy migration does not create ControlLossEpoch or four-second disconnect re-entry protection;
- a second device merely knowing the reconnect secret cannot manufacture current-binding authorization.

Exact protocol messages and migration UX remain later design and are not required for first-release reconnect.

## 6. Reauthenticated recovery grant interaction

`FND-04_REAUTHENTICATED_RECOVERY_GRANT_PROFILE_V1.md` requires rejection when a healthy controller exists and repeats the COMMIT-time revalidation required by Section 4.

Therefore a valid recovery JWT can authorize same-GameSession recovery only while the game-domain session state remains genuinely recovery-eligible through COMMIT. Platform reauthentication never converts a healthy current GameSession into a takeover target.

Intentional logout-eligible takeover remains the separate `TAKEOVER_DRAINING` path and results in a fresh GameSession where the main FND-04 contract requires it.

## 7. Reconnect secret theft consequence

Possession of a stolen reconnect secret may let an attacker race a legitimate reconnect **after** server-declared loss, but it cannot be used to kick a healthy current binding or to finish a prepared replacement after the incumbent has regained sufficient current-generation control.

The existing one-prepared-rebind / one-COMMIT-winner rules plus Section 4 COMMIT revalidation determine the post-loss race. A stale or losing proof cannot fence the winner.

Future sender-constrained/PoP reconnect credentials may further reduce stolen-bearer-secret risk, but are not required for v1 architecture acceptance.

## 8. Mandatory architecture decision timing

This section satisfies the repository architecture-decision discipline for the complete FND-04 package. `YES` means the semantic choice is required before FND-04 may become accepted architecture; `DEFERRED` means FND-04 intentionally leaves the mechanism/value to a named later evidence gate.

| Material choice | Decide for FND-04 now? | Concrete downstream work blocked | Evidence that may justify later supersession |
|---|---|---|---|
| Platform attempt authorization versus Oteryn-v2 final gameplay authority | `YES` | all native admission/session implementation and Platform producer rollout | owner-approved cross-repository authority ADR with security and migration proof |
| Separate AccountPresenceClaim, CharacterLease, GameSession, TransportBinding and RuntimeScopeAuthority semantics | `YES` | DUR session/lease persistence, runtime admission/recovery and duplicate-login handling | fault-injection or formal/concurrency evidence showing an equivalent model preserves every fence and presence invariant |
| Atomic fresh-admission authority transition with no externally visible partial authority | `YES` | FND-02 admission messages, DUR persistence transaction design and E2E | durable transaction/reconciliation design proving an equally linearizable externally unambiguous transition |
| Mutually exclusive fresh-entry and reauthenticated-recovery credential purposes | `YES` | Platform producer profiles and game validators | security review/interoperability evidence proving a replacement profile cannot enable route/authority confusion |
| Fully specified JOSE `Ed25519` v1 profile and deprecated `EdDSA` rejection | `YES` | cross-language producer/consumer fixtures and key distribution | standards/security/interop evidence plus coordinated profile-version migration; never silent fallback |
| 30-second maximum grant lifetime, 5-second verifier skew and <=5-second Platform-security evidence age | `YES` for v1 security ceilings | producer/consumer acceptance and revocation/freshness behavior | measured clock/distribution/availability evidence plus threat-model review showing a changed bound preserves or improves accepted risk |
| Fresh-entry route/runtime observation and scope-ownership-generation binding; owner-generation change invalidates v1 grant | `YES` | Gateway routing, failover admission and runtime-status integration | failover/security proof showing safe carry-forward across owner generation without stale-owner admission |
| AdmissionAttemptRef producer idempotency distinct from GrantNonce/RecoveryGrantNonce consume identity | `YES` | Platform issuance reconciliation and game replay store | cross-system transaction model proving equivalent ambiguity/replay safety without identity collapse |
| Reconnect PREPARE/COMMIT with COMMIT-time authority revalidation | `YES` | reconnect/recovery protocol messages, session runtime and crash recovery | fault/concurrency proof of an alternative with no lost-response ambiguity or stale prepared-authority takeover |
| Healthy current binding non-preemption by reconnect/recovery bearer proof | `YES` | duplicate-login/reconnect/takeover implementation | explicit product/security decision plus current-generation-authorized migration proof that preserves incumbent safety |
| Accepted 2s loss / 5s transport cleanup / 15s same-session grace / 4s one-per-ControlLossEpoch protection composition | `YES` | session timers, reconnect UX and gameplay protection behavior | measured fairness/abuse/liveness telemetry and owner-approved gameplay policy revision |
| Post-grace same-character recovery attaches a fresh GameSession to the exact existing PRESENT_UNCONTROLLED actor | `YES` | actor lifecycle, recovery locator and account presence logic | gameplay/durability evidence proving another design preserves actor state and cannot manufacture logout/reset benefit |
| Exact liveness probe cadence/hysteresis | `DEFERRED` | runtime implementation acceptance, not FND-04 semantic acceptance | measured latency/load/packet-loss/scheduler/fault evidence required by main-contract Section 30 |
| CharacterLease TTL/renew interval/safety margin | `DEFERRED` | lease implementation acceptance, not FND-04 semantic acceptance | measured datastore/network/clock/failover uncertainty and split-owner fault injection |
| Prepared-state/replay/rate/resource hard limits | `DEFERRED` | implementation acceptance | resource/abuse/performance evidence and registry boundary tests |
| Physical PostgreSQL/Redis/other persistence representation and isolation primitive | `DEFERRED` to DUR | durable implementation | DUR transaction, rollback, migration and recovery evidence |
| Concrete Rust/Go/PHP crypto/token library, KMS/HSM vendor and key-distribution product | `DEFERRED` to implementation/security operations | implementation/deployment only | maintenance, interoperability, security-review and operations evidence |
| Healthy-session seamless migration UX/protocol | `DEFERRED` | optional future migration feature only | product need plus current-generation authorization, abuse and concurrency evidence |

A later contract may supersede one row only explicitly; historical FND-04 acceptance remains preserved as provenance.

## 9. Contract-owned failure progression

This section normatively completes main-contract Section 27 under `FOUNDATION_ERROR_VOCABULARY.md`.

Disposition vocabulary:

- `RETRYABLE` — a bounded retry is permitted only under the exact retry-authority rule in the table;
- `TERMINAL` — the rejected semantic attempt/proof cannot be retried as though it were still authoritative;
- `SECURITY_TERMINAL` — the credential/proof is rejected and must not be blindly retried or reinterpreted.

Mutation/idempotency vocabulary:

- `NO_AUTHORITY_MUTATION` — the rejection commits no new gameplay/session/lease authority;
- `COMMITTED_OR_RECONCILE_REQUIRED` — a prior success may already exist; reconcile the stable operation/session result before any independent retry;
- `BOUNDED_RENEWAL_ONLY` — retry may preserve an already-current lease/session only before its fail-safe deadline; it never grants replacement authority.

| Internal code | Category | Disposition | Retry authority | Mutation / idempotency outcome | Public class |
|---|---|---|---|---|---|
| `ADMISSION_GRANT_MALFORMED` | `INVALID_INPUT` | `TERMINAL` | never same malformed grant; obtain a newly issued valid attempt capability | `NO_AUTHORITY_MUTATION` | `RETRY_LOGIN` |
| `ADMISSION_GRANT_AUTHENTICATION_FAILED` | `AUTHENTICATION_FAILED` | `SECURITY_TERMINAL` | never same credential; restart authenticated issuance | `NO_AUTHORITY_MUTATION` | `AUTHENTICATION_REQUIRED` |
| `ADMISSION_GRANT_EXPIRED` | `SESSION_REJECTED` | `TERMINAL` | fresh Gateway/issuer attempt and new grant | `NO_AUTHORITY_MUTATION` | `RETRY_LOGIN` |
| `ADMISSION_GRANT_REPLAYED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | never reuse grant; reconcile prior admission outcome first, then new authenticated attempt only if no current authority exists | `COMMITTED_OR_RECONCILE_REQUIRED` | `SESSION_UNAVAILABLE` |
| `ADMISSION_GRANT_SECURITY_STATE_REVOKED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | no retry until Platform security authority again permits a newly authenticated attempt | `NO_AUTHORITY_MUTATION` | `AUTHENTICATION_REQUIRED` |
| `ADMISSION_GRANT_SECURITY_EVIDENCE_STALE` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same unconsumed grant only if still time-valid and all other bindings remain current after fresh trusted evidence; otherwise new grant | `NO_AUTHORITY_MUTATION` | `TEMPORARILY_UNAVAILABLE` |
| `ADMISSION_GRANT_ROUTE_STALE` | `STALE_GENERATION` | `TERMINAL` | fresh Gateway routing and new grant; never retarget old grant | `NO_AUTHORITY_MUTATION` | `RETRY_LOGIN` |
| `ADMISSION_GRANT_RUNTIME_GENERATION_STALE` | `STALE_GENERATION` | `TERMINAL` | fresh current-owner runtime evidence plus new grant | `NO_AUTHORITY_MUTATION` | `RETRY_LOGIN` |
| `ADMISSION_GRANT_REVISION_UNSUPPORTED` | `UNSUPPORTED_REVISION` | `TERMINAL` | only a producer/client/consumer revision accepted by the compatibility matrix; no downgrade | `NO_AUTHORITY_MUTATION` | `CLIENT_UPDATE_REQUIRED` |
| `ADMISSION_ACCOUNT_CHARACTER_CONFLICT` | `CONFLICT` | `TERMINAL` | new attempt only after authoritative ownership/lifecycle changes and is revalidated | `NO_AUTHORITY_MUTATION` | `SESSION_UNAVAILABLE` |
| `ADMISSION_INCUMBENT_PROTECTED` | `CONFLICT` | `TERMINAL` | do not retry same grant as takeover; new attempt only after incumbent is authoritatively eligible/absent | `NO_AUTHORITY_MUTATION` | `CHARACTER_ALREADY_ACTIVE` |
| `ADMISSION_CAPACITY_EXCEEDED` | `CAPACITY_EXCEEDED` | `RETRYABLE` | bounded backoff; same unconsumed grant only on the same still-current route while valid, otherwise fresh routing/grant | `NO_AUTHORITY_MUTATION` | `TEMPORARILY_UNAVAILABLE` |
| `RECONNECT_PROOF_INVALID` | `AUTHENTICATION_FAILED` | `SECURITY_TERMINAL` | never blind-retry invalid proof; use valid current proof or reauthenticated recovery path | `NO_AUTHORITY_MUTATION` | `AUTHENTICATION_REQUIRED` |
| `RECONNECT_PROOF_REPLAYED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | reconcile current GameSession/binding; stale proof is never reusable | `COMMITTED_OR_RECONCILE_REQUIRED` | `SESSION_UNAVAILABLE` |
| `RECONNECT_SESSION_TERMINAL` | `SESSION_REJECTED` | `TERMINAL` | same GameSession never retries; use eligible fresh-session existing-actor recovery/new login path | `NO_AUTHORITY_MUTATION` | `SESSION_UNAVAILABLE` |
| `RECONNECT_GENERATION_STALE` | `STALE_GENERATION` | `TERMINAL` | reconcile current generation; stale generation/proof cannot retry as authority | `NO_AUTHORITY_MUTATION` | `SESSION_UNAVAILABLE` |
| `RECONNECT_ATTEMPT_CONFLICT` | `CONFLICT` | `RETRYABLE` | reconcile the current prepared/committed attempt; same ReconnectAttemptRef may obtain its stable result; a different attempt waits until prior state is terminal | `NO_AUTHORITY_MUTATION` or stable prior committed result | `TEMPORARILY_UNAVAILABLE` |
| `RECONNECT_GRACE_EXPIRED` | `SESSION_REJECTED` | `TERMINAL` | same-session retry forbidden; use reauthenticated post-grace recovery if actor is eligible | `NO_AUTHORITY_MUTATION` | `SESSION_UNAVAILABLE` |
| `RECOVERY_GRANT_REPLAYED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | never reuse grant; reconcile prior recovery result before any newly authenticated recovery attempt | `COMMITTED_OR_RECONCILE_REQUIRED` | `SESSION_UNAVAILABLE` |
| `RECOVERY_HEALTHY_CONTROLLER_PRESENT` | `CONFLICT` | `TERMINAL` | no bearer-proof retry/takeover; new recovery only after server-authoritative loss or separately authorized healthy migration | `NO_AUTHORITY_MUTATION` | `CHARACTER_ALREADY_ACTIVE` |
| `RECOVERY_PLACEMENT_UNAVAILABLE` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same unconsumed grant only while time/security-valid; otherwise fresh recovery grant | `NO_AUTHORITY_MUTATION` | `TEMPORARILY_UNAVAILABLE` |
| `RECOVERY_STATE_UNSAFE` | `INTERNAL_UNAVAILABLE` | `TERMINAL` | client does not retry the same authority transition until server reconciliation/recovery establishes safe state | `NO_AUTHORITY_MUTATION` | `SESSION_UNAVAILABLE` |
| `CHARACTER_LEASE_STALE` | `STALE_GENERATION` | `TERMINAL` | stale holder never renews/replaces authority; reconcile current owner/session | `NO_AUTHORITY_MUTATION` | `SESSION_UNAVAILABLE` |
| `CHARACTER_LEASE_RENEW_TIMEOUT` | `TIMEOUT` | `RETRYABLE` | bounded same-current-lease renewal only before local fail-safe deadline; after deadline fail safe and do not self-grant replacement | `BOUNDED_RENEWAL_ONLY` | `TEMPORARILY_UNAVAILABLE` |
| `CHARACTER_LEASE_DEPENDENCY_UNAVAILABLE` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | bounded same-current-lease renewal/reconciliation only while safe deadline remains | `BOUNDED_RENEWAL_ONLY` | `TEMPORARILY_UNAVAILABLE` |
| `SESSION_TAKEOVER_NOT_ALLOWED` | `CONFLICT` | `TERMINAL` | new takeover attempt only after authoritative eligibility changes and fresh authorization | `NO_AUTHORITY_MUTATION` | `CHARACTER_ALREADY_ACTIVE` |

No public mapping exposes raw grant/proof validity, account-security generation, fencing values, lease details or combat-sensitive state. Numeric wire allocation remains later FND-02 registry work and cannot weaken this progression.

## 10. Required implementation evidence

Before reconnect implementation acceptance, tests MUST demonstrate:

1. current generation healthy + correct reconnect secret from second transport -> PREPARE rejected, incumbent unaffected;
2. current generation healthy + valid reauthenticated recovery grant -> PREPARE/recovery rejected, incumbent unaffected;
3. current generation healthy + multiple concurrent contenders -> none can create prepared state without current-binding migration authorization;
4. server-declared eligible loss -> one valid reconnect contender may PREPARE and exactly one eligible contender may COMMIT;
5. incumbent regains sufficient current-generation control after PREPARE -> unsolicited COMMIT rejected, incumbent remains authoritative;
6. recovery JWT expires/is revoked or Platform-security generation/freshness invalidates after PREPARE -> recovery COMMIT rejected with no authority switch;
7. CharacterLease/runtime/session/reconciliation authority changes after PREPARE -> stale candidate cannot COMMIT;
8. pre-loss current-binding-authorized migration, if implemented, switches authority atomically without creating ControlLossEpoch/protection;
9. stale migration authorization from generation N cannot affect generation N+1;
10. stolen predecessor reconnect secret after successful COMMIT cannot regain authority or fence successor;
11. every Section 9 failure code follows the frozen disposition/retry/idempotency/public mapping under positive, negative and ambiguous-outcome fixtures.

## 11. Concise rule

```text
healthy current binding
+ reconnect secret / recovery JWT on another transport
-> NOT replacement authority
-> reject unsolicited PREPARE

server-proven eligible loss
-> reconnect PREPARE may proceed
-> PREPARE grants no authority escrow
-> COMMIT atomically revalidates current eligibility
-> incumbent recovered / lease-runtime-state changed / recovery proof expired-revoked
   => no authority switch
-> exactly one current generation winner

healthy intentional migration
-> separate current-generation-authorized transition
-> never bearer-secret-only takeover
-> no disconnect protection

cross-component failure
-> stable code + foundation category
-> explicit RETRYABLE / TERMINAL / SECURITY_TERMINAL
-> explicit retry authority + mutation/idempotency outcome + bounded public class
```
