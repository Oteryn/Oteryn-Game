# FND-04A — Authority and Fresh Admission Contract

- Status: Candidate bounded architecture contract; canonical for FND-04A only when its owning PR merges
- Gate: `FND-04A`
- Replacement programme: Issue #112
- Owning delivery: Issue #113 / PR #114
- Repository: `blakinio/Oteryn-v2`
- Trusted base: FND-04 analysis accepted on `main@27f7f647f04e3b1a4151f9b124401986910f03d8`
- Historical reviewed evidence only: superseded PR #109, final head `bf82e392d6ef8b1e627849cdc7383af9a7c987ae`
- Normative companion: `docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md`
- Consumes: ADR-0003; ADR-0012; FND-ID-01; FND-02; accepted FND-03; accepted FND-04 analysis/reconciliation baselines; Foundation Error Vocabulary
- Does not authorize: reconnect/recovery finalization, Rust runtime/protocol implementation, persistence schema, Platform writes, KMS/HSM/vendor selection, deployment or production traffic

## 1. Purpose and bounded scope

FND-04A freezes only the authority and security semantics required for a **fresh native gameplay admission**.

```text
Platform authenticates and authorizes one bounded attempt.
Oteryn-v2 alone decides whether current game-domain facts permit gameplay authority.
No earlier validation escrows that authority.
```

FND-04A does **not** complete FND-04. Reconnect, reauthenticated recovery, same-session grace, ControlLossEpoch, GameNode continuity and handoff continuity belong to FND-04B. Final cross-programme error/failure/compatibility integration belongs to FND-04C.

### Decision timing

| Decision | Now? | Blocks | Risk if wrong later | Superseding evidence | Deliberately deferred |
|---|---|---|---|---|---|
| Platform attempt authority vs game final authority | `YES` | issuer/consumer, session creation | dual authorities and migration/security ambiguity | reviewed replacement model consistent with ADR-0003/ADR-0012 | service/API placement |
| Separate presence, lease, session, transport, runtime authority | `YES` | duplicate login, fencing, persistence | stale-writer/session aliasing | formal/fault proof of equivalent separation | physical tables/locks |
| One atomic final admission linearization | `YES` | replay, presence/lease transaction | TOCTOU and partial authority | concurrency proof of equivalent single-winner design | isolation/storage primitive |
| Current `CharacterId -> WorldId` / world eligibility | `YES` | world-transfer safety | stale valid grant can attach global character to wrong world | explicit future transfer protocol with equivalent fencing | transfer storage/workflow |
| Strict fresh-entry grant profile | `YES` | Platform issuer/game verifier | cross-purpose credential confusion | independently reviewed profile revision | JWT/KMS implementation |
| `<=5s` security/trust evidence ceiling | `YES` | revocation behavior | stale evidence creates new authority | measured threat-model supersession | projection transport/cadence inside ceiling |
| Production lease/liveness/capacity values | `NO` | implementation acceptance | guessed unsafe defaults | PERF/OPS/DUR evidence | exact numeric values |

## 2. Canonical fresh-admission authority layers

### 2.1 AccountPresenceClaim

Scope: `AccountId`. It identifies the account's current playable or mandatory-presence `CharacterId` and enforces account-global exclusion. It is not a GameSession and is not released merely because a transport closes.

Fresh admission may evaluate claim eligibility before commit, but no claim becomes authoritative until Section 7 commits.

### 2.2 CharacterLease

Scope:

```text
CharacterId + character_lease_generation
```

It fences current authoritative character writer/control participation. Generation is non-zero monotonic `uint64`-class state or an exact non-reused equivalent; stale generation cannot renew, commit durable mutation or create player control; exhaustion never wraps/reuses.

Fresh admission may evaluate acquisition eligibility before commit. Acquisition/advance becomes authoritative only inside the same final boundary that consumes the grant and creates GameSession authority.

### 2.3 GameSession

`GameSessionId` is created only by successful game-domain admission commit. It is identity, not a bearer credential. A candidate ID generated before commit is discarded and never reused if admission fails.

### 2.4 TransportBinding

First admitted binding uses:

```text
GameSessionId + connection_generation = 1
```

Generation `0` remains pre-admission only. Reconnect/rebind semantics are explicitly outside FND-04A and belong to FND-04B.

### 2.5 RuntimeScopeAuthority

Current ChannelRuntime/InstanceRuntime semantic scope plus accepted FND-03 ownership generation is authoritative for simulation. NodeId is placement/incarnation evidence, never authority.

## 3. Platform and game-domain boundary

### Platform owns

- reusable account authentication/security policy;
- OAuth/PKCE/MFA/recovery and Game Login Ticket lifecycle;
- Platform account-security generation/revision;
- configured world/channel/login/maintenance/entitlement policy;
- Gateway route/offer orchestration;
- authorization and signing of one bounded fresh-entry attempt.

### Oteryn-v2 owns

- final `AccountId -> CharacterId` ownership/lifecycle validation;
- final `CharacterId -> WorldId` / world-eligibility validation;
- AccountPresenceClaim and CharacterLease;
- current runtime target/ownership/readiness;
- GrantNonce consume/replay state;
- GameSession creation and first TransportBinding generation;
- final fresh-admission outcome.

Platform never creates canonical GameSessionId. A valid Platform signature never bypasses current game facts.

## 4. Fresh-entry credential

Fresh entry uses only `docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md`.

It freezes JWS Compact JWT with fully specified JOSE `alg=Ed25519`, rejects deprecated polymorphic `EdDSA`, uses dedicated type/issuer/audience/profile/key purpose, lifetime `<=30s`, verifier skew `<=5s`, authenticated Platform-security evidence age `<=5s`, authenticated signing-key/profile trust/revocation evidence age `<=5s`, one-time 32-byte GrantNonce, distinct AdmissionAttemptRef, route/runtime/world bindings and fail-closed game revalidation.

OAuth credentials and Game Login Tickets are never accepted by the game server as this capability.

## 5. Current character-world binding

`CharacterId` is globally stable and may survive a legal world transfer. These facts are separate:

```text
AccountId owns CharacterId
CharacterId is currently eligible for WorldId
Gateway route targets WorldId/ChannelId
```

The current world relation MUST be evaluated only after the current `AccountId -> CharacterId` relationship has been proven for this admission candidate. This prevents a malformed/compromised producer input from turning world-state classification into an oracle for a character the account does not currently own.

### 5.1 v1 invariant

After current account-character ownership/lifecycle is proven, and again immediately before the atomic authority commit, Oteryn-v2 MUST prove:

```text
current_character_world_id == grant.world_id
AND current character lifecycle permits fresh admission to grant.world_id
```

If a future transfer protocol permits an intermediate relation, it must be explicitly represented by a later reviewed contract. v1 never infers transfer permission from a stale grant, Platform route or client route.

### 5.2 Mismatch semantics

If current ownership is valid but the grant names a different world, or the world relation changes after earlier validation but before commit:

- fail as `ADMISSION_GRANT_WORLD_STALE` before GrantNonce consumption;
- create/advance no candidate AccountPresenceClaim, CharacterLease, GameSession or TransportBinding authority;
- never retarget the old grant to the new/current world;
- preserve actual current transfer/world/presence/lease/session state;
- require current world resolution plus a newly authorized route/grant for any later attempt.

The bounded public class is `RETRY_LOGIN`; public presentation does not disclose whether a transfer, configuration change or other current-world change caused the stale binding.

## 6. Route/runtime/compatibility applicability

The signed grant binds at least `world_id`, `channel_id`, `route_revision`, `runtime_observation_revision`, `scope_ownership_generation`, `protocol_major`, `transport_profile` and `compatibility_revision`.

Reject when current target lifecycle is not open, route/runtime observation is superseded, scope ownership changed, runtime owner/placement/readiness is not current, protocol/transport/compatibility is unsupported, or—after ownership is proven—the character's current world relation does not permit the signed world.

No silent retarget to another World, Channel, owner, protocol family or Canary route.

## 7. Atomic fresh-admission linearization

All checks before the final boundary are fail-fast **eligibility only** and never authorization escrow.

Conceptual evaluation order:

1. FND-02 material bounds;
2. parser/header/profile bounds;
3. authenticated current signing-key/profile trust evidence age `<=5s` and trusted key lookup;
4. Ed25519 signature + issuer/audience/type/purpose/time;
5. canonical claims/UUID/generation encoding;
6. current Platform-security freshness/revocation/generation;
7. route/runtime/current scope/current target/protocol/transport/compatibility;
8. GrantNonce replay/consume eligibility;
9. current `AccountId -> CharacterId` ownership/lifecycle;
10. current authoritative `CharacterId -> WorldId` / world eligibility against signed `world_id`;
11. AccountPresenceClaim / duplicate-login eligibility;
12. CharacterLease + current runtime-scope acquisition/readiness eligibility;
13. one atomic final revalidation + authority commit;
14. publish success only after commit.

### 7.1 Mandatory final revalidation

Immediately before and atomically with authority creation, revalidate every mutable predicate relevant to admission, including:

- JWT `nbf`/`exp`/lifetime/skew;
- exact key/profile trust and authenticated trust/revocation evidence age `<=5s`;
- authenticated Platform-security evidence age `<=5s`, account state and generation admissibility;
- route/runtime observation, target lifecycle, scope ownership, current runtime owner/placement/readiness;
- protocol/transport/compatibility;
- current `AccountId -> CharacterId` ownership/lifecycle **before evaluating character-world state**;
- current `CharacterId -> WorldId` / world eligibility still matches `grant.world_id`;
- GrantNonce still unconsumed/eligible;
- AccountPresenceClaim/duplicate-login state;
- CharacterLease legal acquisition/current fence state;
- absence of a newer world-transfer/handoff/fence/takeover/terminal transition superseding the candidate.

### 7.2 Atomic effects

Only if every predicate remains valid does the same boundary establish:

```text
consume GrantNonce
+ establish/advance AccountPresenceClaim as required
+ establish/acquire CharacterLease as required
+ create canonical GameSessionId
+ GameSession ACTIVE
+ connection_generation = 1
+ establish initial authoritative session/reconciliation boundary
```

FND-04A does not define reconnect secret/proof state; FND-04B must define any post-admission reconnect material without weakening this admission boundary.

No earlier presence/lease eligibility creates partial authority. If any final predicate changed, the candidate fails before its own authority mutation and cannot roll back/overwrite actual current world-transfer, presence, lease, runtime or session authority.

## 8. Account-global exclusion and duplicate login

Two different CharacterIds for one AccountId cannot both become playable or mandatory-presence actors.

A second fresh-entry grant cannot by possession alone fence incumbent control, close incumbent transport, release AccountPresenceClaim, replace CharacterLease or admit another CharacterId while current authority blocks it.

Concurrent candidates serialize through current account presence, lease, nonce and session/actor state; at most one final boundary wins. Intentional takeover/handoff continuity details belong to FND-04B/C.

## 9. Platform-security and signing-key/profile freshness

Fresh admission requires:

```text
short-lived signed grant
+ account_security_generation
+ authenticated Platform-security evidence age <=5s
+ authenticated signing-key/profile trust/revocation evidence age <=5s
```

Stale/unavailable/unauthenticated/contradictory/unprovable required evidence fails closed. Fresh authenticated evidence explicitly marking the exact admission key/profile unknown/revoked/not trusted is security-terminal for that credential. Nominal JWT expiry never overrides newer revocation.

This is a pre-admission veto only; it does not grant Platform post-admission gameplay authority.

## 10. AdmissionAttemptRef and GrantNonce

`attempt_ref` is Platform issuance/reconciliation identity; `jti` is the one-time game consume GrantNonce. They are never aliases.

Ambiguous issuance permits same-AdmissionAttemptRef reconciliation/status recovery only. Platform must not mint a blind second capability. A new independent attempt requires deterministic retirement of the old attempt plus proof any possibly issued capability is no longer acceptable.

One GrantNonce can participate in at most one successful admission. A losing replay cannot duplicate, revive or fence gameplay authority.

## 11. Fresh-admission error subset

FND-04A owns the complete Foundation Error Vocabulary shape for its bounded fresh-admission errors. FND-04C may integrate these accepted rows but may not silently alter them.

Common credential-free diagnostic envelope:

```text
error_code
request_trace_id
admission_attempt_ref only when safely parsed/authorized for diagnostics
profile_id when known
safe_kid only when known and policy permits
```

Never include raw JWT, GrantNonce, OAuth/Game Login Ticket, reconnect material, Platform security-generation values, private fencing data, SQL errors or unstable exception text.

| Internal code | Category | Progression | Retry / next authority | Mutation outcome | Public class | Redacted diagnostic message | Additional credential-free correlation fields |
|---|---|---|---|---|---|---|---|
| `ADMISSION_GRANT_MALFORMED` | `INVALID_INPUT` | `TERMINAL` | newly issued valid capability; never same malformed grant | no consume/presence/lease/session mutation | `RETRY_LOGIN` | `fresh admission grant malformed` | parser stage; safe profile/header class if parsed |
| `ADMISSION_GRANT_AUTHENTICATION_FAILED` | `AUTHENTICATION_FAILED` | `SECURITY_TERMINAL` | restart authenticated issuance; never same credential | no authoritative mutation | `AUTHENTICATION_REQUIRED` | `fresh admission credential authentication failed` | safe_kid; trust decision class/revision |
| `ADMISSION_GRANT_NOT_YET_VALID` | `SESSION_REJECTED` | `RETRYABLE` | same unconsumed grant only after accepted nbf window while all bindings remain current | no GrantNonce/authority mutation | `TEMPORARILY_UNAVAILABLE` | `fresh admission grant not yet active` | trusted-time boundary class; profile_id |
| `ADMISSION_GRANT_EXPIRED` | `SESSION_REJECTED` | `TERMINAL` | fresh issuer/Gateway attempt | no authoritative mutation | `RETRY_LOGIN` | `fresh admission grant expired` | trusted-time boundary class; profile_id |
| `ADMISSION_GRANT_REPLAYED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | reconcile prior admission; never reuse consumed grant | prior success may exist; no duplicate effect | `SESSION_UNAVAILABLE` | `fresh admission grant already consumed or replayed` | replay receipt/correlation ref; attempt_ref |
| `ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same AdmissionAttemptRef reconciliation until deterministic retirement/proof | producer ambiguity creates no gameplay authority | `TEMPORARILY_UNAVAILABLE` | `fresh admission issuance outcome requires reconciliation` | attempt_ref; issuer operation-status revision |
| `ADMISSION_GRANT_SECURITY_STATE_REVOKED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | new authenticated attempt only after Platform security permits | no authoritative mutation | `AUTHENTICATION_REQUIRED` | `fresh admission denied by current account security state` | security evidence decision class/revision; no generation value |
| `ADMISSION_GRANT_SECURITY_EVIDENCE_STALE` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same unconsumed grant only after fresh authenticated evidence while all bindings remain valid | no GrantNonce/authority mutation | `TEMPORARILY_UNAVAILABLE` | `fresh admission security evidence unavailable or stale` | evidence source class; freshness bucket; trust decision revision |
| `ADMISSION_GRANT_ROUTE_STALE` | `STALE_GENERATION` | `TERMINAL` | fresh Gateway route + new grant | no authoritative mutation | `RETRY_LOGIN` | `fresh admission route no longer current` | world_id; channel_id; route_revision |
| `ADMISSION_GRANT_RUNTIME_GENERATION_STALE` | `STALE_GENERATION` | `TERMINAL` | fresh current-owner evidence + new grant | no authoritative mutation | `RETRY_LOGIN` | `fresh admission runtime ownership no longer current` | world_id; channel_id; runtime_observation_revision; scope-ownership match/stale class only |
| `ADMISSION_GRANT_WORLD_STALE` | `STALE_GENERATION` | `TERMINAL` | resolve current character world then obtain newly authorized route/grant; never retarget old grant | no GrantNonce/presence/lease/session/transport mutation | `RETRY_LOGIN` | `fresh admission character world binding no longer matches` | signed world_id; character-world relation revision/class; route_revision; no transfer details |
| `ADMISSION_GRANT_REVISION_UNSUPPORTED` | `UNSUPPORTED_REVISION` | `TERMINAL` | compatible producer/client/consumer only; no downgrade | no authoritative mutation | `CLIENT_UPDATE_REQUIRED` | `fresh admission revision unsupported` | protocol_major; transport_profile; compatibility_revision |
| `ADMISSION_ACCOUNT_CHARACTER_CONFLICT` | `CONFLICT` | `TERMINAL` | new attempt only after authoritative ownership/lifecycle change | no partial admission | `SESSION_UNAVAILABLE` | `fresh admission account or character relationship conflicts with current authority` | lifecycle/ownership decision class/revision; world_id only after ownership-safe evaluation |
| `ADMISSION_INCUMBENT_PROTECTED` | `CONFLICT` | `TERMINAL` | new attempt only after incumbent eligibility changes | incumbent unchanged; newcomer no authority | `CHARACTER_ALREADY_ACTIVE` | `fresh admission blocked by current character authority` | incumbent state class; world_id/channel_id where policy permits |
| `ADMISSION_CAPACITY_EXCEEDED` | `CAPACITY_EXCEEDED` | `RETRYABLE` | bounded backoff; same grant only on same current route while valid, else new route/grant | no partial admission authority | `TEMPORARILY_UNAVAILABLE` | `fresh admission capacity unavailable` | capacity class; world_id/channel_id; route_revision |

## 12. Required fault/interoperability evidence

### Credential/profile

Require independent positive/negative fixtures for Ed25519, `alg=none`, deprecated `EdDSA`, wrong key type/curve, token-directed key discovery, parser/duplicate/unknown claims, issuer/audience/type/profile/purpose, UUIDv7/variant, `nbf`/expiry/skew/lifetime, key rotation and exact trust-evidence `5s`/`>5s`/unavailable/revoked cases, Platform-security stale/revoked, replay/concurrent consume and ambiguous issuance reconciliation.

### World binding — mandatory carried-P1 evidence

1. after current AccountId->CharacterId ownership is proven, grant `world_id` differs from current authoritative world -> `ADMISSION_GRANT_WORLD_STALE`, no candidate GrantNonce/authority mutation;
2. ownership/world validation initially passes, then legal world transfer/current relation changes before final commit -> final revalidation rejects `ADMISSION_GRANT_WORLD_STALE`;
3. stale grant is never retargeted to the new/current World or Channel;
4. concurrent transfer/admission ordering has one linearized authoritative outcome and loser preserves actual transfer/world/presence/lease/session state;
5. invalid AccountId->CharacterId relationship fails as account/character conflict **before** any world-mismatch classification is returned for that candidate.

### Change-before-commit matrix

After each earlier check succeeds, independently mutate JWT time, key/profile trust/freshness, Platform security, route/runtime/current target, protocol/transport/compatibility, AccountId->CharacterId ownership, CharacterId->WorldId/world eligibility, GrantNonce, AccountPresence/incumbent, CharacterLease/fence or superseding world-transfer/handoff/fence/takeover/terminal authority.

Each losing candidate must fail before its authority mutation and preserve actual current authority. Presence, lease, GameSession and TransportBinding become authoritative only together for the winner.

## 13. Security/privacy

Never log/export raw grant JWT, GrantNonce, reusable credentials or private key material. High-cardinality AccountId/CharacterId do not become ordinary metric labels. Diagnostic templates are stable redacted contract text, not raw dependency/exception strings.

## 14. Downstream integration

FND-04B consumes accepted authority identities and first-session/transport state to define reconnect/recovery/continuity; it must not weaken FND-04A fresh-admission authority.

FND-04C integrates accepted FND-04A/B errors, failure scenarios, compatibility/evidence and a thin final FND-04 index/status; it may not silently redefine accepted component semantics.

DUR/OPS/PERF own physical atomicity, replay/presence/lease persistence, numeric lease/liveness safety and production capacities.

## 15. Acceptance boundary

Merging FND-04A means only:

- fresh-admission authority and atomic linearization accepted;
- strict fresh-entry profile accepted;
- current ownership-safe `CharacterId -> WorldId` / world eligibility mandatory at final admission;
- fresh-admission error subset has complete Foundation Error Vocabulary shape;
- no runtime implementation authorized;
- overall FND-04 remains incomplete until FND-04B, FND-04C and lifecycle closeout.

## 16. Concise rule

```text
Platform authenticates + signs bounded fresh-entry attempt
-> never gameplay authority

Oteryn-v2 validates
-> security/trust freshness
-> route/runtime/compatibility
-> AccountId -> CharacterId FIRST
-> CharacterId -> current WorldId/world eligibility SECOND
-> nonce + presence + lease

precommit checks
-> eligibility only
-> never authorization escrow

one atomic final boundary
-> repeat ownership-safe world check and every other mutable predicate
-> stale world => ADMISSION_GRANT_WORLD_STALE, no retarget/no candidate mutation
-> all valid => consume GrantNonce + establish presence + lease + GameSession + connection_generation 1

reconnect/recovery
-> not defined by FND-04A
-> FND-04B
```
