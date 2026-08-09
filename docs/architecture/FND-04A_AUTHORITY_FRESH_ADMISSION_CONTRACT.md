# FND-04A — Authority and Fresh Admission Contract

- Status: Candidate bounded architecture contract; canonical for FND-04A only when its owning PR merges
- Gate: `FND-04A`
- Replacement programme: Issue #112
- Owning delivery: Issue #113
- Repository: `blakinio/Oteryn-v2`
- Trusted base: FND-04 analysis accepted on `main@27f7f647f04e3b1a4151f9b124401986910f03d8`
- Historical reviewed evidence only: superseded PR #109, final head `bf82e392d6ef8b1e627849cdc7383af9a7c987ae`
- Normative companion: `docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md`
- Consumes: ADR-0003 Platform Identity / Game Gateway boundary; ADR-0012 Character Authority boundary; FND-ID-01; FND-02; accepted FND-03; FND-04 analysis/reconciliation baselines; Foundation Error Vocabulary
- Does not authorize: reconnect/recovery finalization, Rust runtime/protocol implementation, persistence schema, Platform writes, KMS/HSM/vendor selection, deployment or production traffic

## 1. Purpose and bounded scope

FND-04A freezes only the authority and security semantics required to perform a **fresh native gameplay admission** without guessing.

Central invariant:

```text
Platform authenticates and authorizes one bounded attempt.
Oteryn-v2 alone decides whether current game-domain facts permit gameplay authority.
No earlier validation escrows that authority.
```

FND-04A deliberately does **not** complete FND-04. Reconnect, reauthenticated recovery, same-session grace, ControlLossEpoch, GameNode continuity and handoff continuity belong to FND-04B. Cross-programme error/failure/compatibility integration belongs to FND-04C.

### Decision timing

The following choices must be frozen now because fresh-admission implementation, persistence design and Platform producer integration depend on them:

| Decision | Now? | Blocks | If wrong later | Superseding evidence | Deliberately deferred |
|---|---|---|---|---|---|
| Platform attempt authority vs game final admission authority | `YES` | issuer/consumer and session creation | dual authorities, migration/security ambiguity | reviewed replacement authority model consistent with ADR-0003/ADR-0012 | concrete service/API placement |
| Separate presence, lease, session, transport and runtime-scope authority | `YES` | duplicate login, fencing, persistence | stale-writer/session aliasing | formal/fault evidence proving equivalent separation | physical tables/locks |
| One atomic final admission linearization | `YES` | admission transaction, replay, presence/lease integration | TOCTOU and partial authority | concurrency proof of equivalent single-winner design | transaction/isolation technology |
| Current `CharacterId -> WorldId` / world eligibility at final commit | `YES` | world transfer safety, routing, lease/session creation | valid old grant can attach global CharacterId to wrong world | explicit future transfer protocol with equivalent fencing | transfer storage/workflow mechanism |
| Strict fresh-entry grant profile | `YES` | Platform issuer and game verifier | cross-purpose credential confusion | independently reviewed profile revision | JWT library/KMS vendor |
| `<=5s` Platform-security and signing-key/profile evidence ceiling | `YES` | revocation behavior | stale security/trust can create new authority | measured threat-model supersession | projection transport/refresh cadence inside ceiling |
| Production lease/liveness/capacity numbers | `NO` | implementation acceptance only | guessed unsafe defaults | PERF/OPS/DUR evidence | exact numeric values |

## 2. Canonical fresh-admission authority layers

Fresh admission must keep these concepts distinct.

### 2.1 AccountPresenceClaim

Scope: `AccountId`.

It identifies the account's currently playable or mandatory-presence `CharacterId` and is the account-global exclusion boundary. It is not a GameSession and is not released merely because a transport closes.

For fresh admission, a candidate may evaluate whether the claim can be established or reused for the same legal character, but the claim does not become authoritative until the Section 7 atomic commit.

### 2.2 CharacterLease

Scope:

```text
CharacterId + character_lease_generation
```

It fences current authoritative character writer/control participation. Generation is non-zero monotonic `uint64`-class state or an exact non-reused equivalent; stale generation cannot renew, commit durable character mutation or create player control; exhaustion never wraps/reuses.

Fresh admission may evaluate acquisition eligibility before commit. Acquisition/advance becomes authoritative only inside the same final boundary that consumes the grant and creates GameSession authority.

### 2.3 GameSession

`GameSessionId` is created only by a successful game-domain admission commit. It is identity, not a bearer credential. A candidate identifier generated before commit is discarded and never reused if admission fails.

### 2.4 TransportBinding

First admitted transport uses:

```text
GameSessionId + connection_generation = 1
```

Generation `0` remains pre-admission only. Reconnect/rebind transitions after admission belong to FND-04B.

### 2.5 RuntimeScopeAuthority

Current ChannelRuntime/InstanceRuntime semantic scope plus the accepted FND-03 ownership generation is authoritative for the target simulation. NodeId is placement/incarnation evidence, never a substitute for ownership generation.

## 3. Platform and game-domain boundary

### Platform owns

- reusable account authentication/security policy;
- OAuth/PKCE/MFA/recovery and Game Login Ticket lifecycle;
- Platform account-security generation/revision;
- configured world/channel/login/maintenance/entitlement policy;
- Gateway route/offer orchestration;
- authorization and signing of a bounded fresh-entry attempt.

### Oteryn-v2 game domain owns

- final `AccountId -> CharacterId` ownership/lifecycle validation;
- final `CharacterId -> WorldId` / world-eligibility validation;
- AccountPresenceClaim and CharacterLease;
- current runtime target, ownership generation and readiness;
- grant consume/replay state;
- GameSession creation;
- first TransportBinding generation;
- final fresh-admission outcome.

Platform never creates canonical GameSessionId and a valid Platform signature never bypasses current game-domain facts.

## 4. Fresh-entry credential

Fresh entry uses only `docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md`.

Required security properties include:

- JWS Compact JWT;
- fully specified JOSE `alg = Ed25519` only;
- deprecated polymorphic `EdDSA` rejected;
- dedicated `typ`, issuer, audience, profile and key purpose;
- maximum lifetime 30 seconds and maximum verifier skew 5 seconds;
- authenticated signing-key/profile trust/revocation evidence age `<=5s` at authority-changing validation;
- 32-byte one-time GrantNonce;
- Platform AdmissionAttemptRef distinct from GrantNonce;
- account-security generation;
- world/channel/route/runtime observation/current ownership-generation bindings;
- protocol/transport/compatibility revision;
- no token-directed key discovery;
- fail-closed final game revalidation.

OAuth credentials and Game Login Tickets are never accepted by the game server as this capability.

## 5. Current character-world binding

`CharacterId` is globally stable and may survive a legal world transfer. Therefore these facts are distinct:

```text
AccountId owns CharacterId
CharacterId is currently eligible for WorldId
Gateway route targets WorldId/ChannelId
```

Passing the first and third checks does not prove the second.

### 5.1 v1 invariant

At initial eligibility evaluation and again immediately before the atomic authority commit, the current game-domain authority MUST prove:

```text
current_character_world_id == grant.world_id
AND current character lifecycle permits fresh admission to grant.world_id
```

If a future transfer protocol allows an intermediate world relation, that state must be explicitly represented by a later reviewed contract. v1 never infers transfer permission from a stale grant, Platform route, client route or ownership alone.

### 5.2 Mismatch semantics

If the grant names a world different from the current authoritative character-world relation, or if that relation changes after earlier validation but before commit:

- fail before GrantNonce consumption;
- create/advance no AccountPresenceClaim, CharacterLease, GameSession or TransportBinding authority for the candidate;
- do not retarget the existing grant to the new/current world;
- preserve whatever authority/world-transfer state is actually current;
- return `ADMISSION_GRANT_WORLD_STALE` internally;
- a later attempt requires current world resolution plus a newly authorized route/grant.

This closes the final review P1 carried from superseded PR #109.

## 6. Route/runtime/compatibility applicability

The signed fresh grant binds at least:

- `world_id`;
- `channel_id`;
- `route_revision`;
- `runtime_observation_revision`;
- `scope_ownership_generation`;
- `protocol_major`;
- `transport_profile`;
- `compatibility_revision`.

Final admission rejects when the current target lifecycle is not open, route/runtime observation is superseded, ownership generation changed, runtime owner/placement/readiness is not current, protocol/transport/compatibility is unsupported, or the character's current world relation does not permit the grant world.

No silent retarget to another world, Channel, owner, protocol family or Canary route is allowed.

## 7. Atomic fresh-admission linearization

All checks before the final boundary are fail-fast **eligibility evaluation only**. They are not authorization escrow.

Conceptual order:

1. FND-02 admission material bounds;
2. exact protected header/profile/parser bounds;
3. authenticated current admission signing-key/profile trust evidence age `<=5s` and trusted key lookup;
4. Ed25519 signature, issuer/audience/type/purpose/time;
5. canonical claim/UUID/generation validation;
6. current Platform-security freshness/revocation/generation;
7. route/runtime/current scope/current target/protocol/transport/compatibility;
8. current authoritative `CharacterId -> WorldId` / world eligibility;
9. GrantNonce replay/consume eligibility;
10. current `AccountId -> CharacterId` ownership/lifecycle;
11. AccountPresenceClaim / duplicate-login eligibility;
12. CharacterLease and current runtime-scope acquisition/readiness eligibility;
13. one atomic final revalidation + authority commit;
14. publish success only after commit.

### 7.1 Mandatory final revalidation

Immediately before and atomically with authority creation, the current game-domain owner MUST revalidate every mutable predicate that can affect admission, including at minimum:

- JWT `nbf`/`exp`/lifetime/skew under trusted server time;
- exact admission signing-key/profile trust and authenticated trust/revocation evidence age `<=5s`;
- current authenticated Platform-security evidence age `<=5s`, account state and `account_security_generation` admissibility;
- `route_revision`, `runtime_observation_revision`, target lifecycle, `scope_ownership_generation`, current runtime owner/placement/readiness;
- `protocol_major`, `transport_profile`, `compatibility_revision`;
- current authoritative `CharacterId -> WorldId` / world eligibility still matches `grant.world_id`;
- GrantNonce still unconsumed/eligible;
- current `AccountId -> CharacterId` ownership/lifecycle;
- AccountPresenceClaim / duplicate-login state and whether a newer incumbent/claim won;
- CharacterLease legal acquisition/current fence generation;
- absence of a newer handoff/fence/takeover/terminal/transfer transition superseding the candidate.

### 7.2 Atomic effects

Only if every predicate remains valid does one boundary establish:

```text
consume GrantNonce
+ establish/advance AccountPresenceClaim as required
+ establish/acquire CharacterLease as required
+ create canonical GameSessionId
+ GameSession ACTIVE
+ connection_generation = 1
+ initialize reconnect-proof state for later FND-04B semantics
+ establish initial authoritative session/reconciliation boundary
```

No AccountPresenceClaim or CharacterLease becomes externally authoritative merely because its earlier eligibility check passed.

If any final predicate changed, the candidate fails before its own authority mutation. A losing candidate never rolls back or overwrites whatever world-transfer, presence, lease, runtime or session authority is actually current.

## 8. Account-global exclusion and duplicate login

Fresh admissions serialize through AccountPresenceClaim, CharacterLease and current session/actor state.

Two different CharacterIds for one AccountId cannot both become playable or mandatory-presence actors.

### Healthy/current protected incumbent

A second fresh-entry grant cannot by possession alone:

- fence incumbent control;
- close incumbent transport;
- release AccountPresenceClaim;
- replace CharacterLease;
- admit another CharacterId.

Intentional takeover/handoff details beyond the admission-side no-preemption invariant are finalized by FND-04B/C where they intersect continuity/error integration.

### Concurrent candidates

At most one candidate can win the final presence/lease/session authority boundary. A stale loser cannot consume its GrantNonce as success, fence the winner or restore older authority.

## 9. Platform-security and signing-key/profile freshness

Fresh admission requires:

```text
short-lived signed grant
+ account_security_generation
+ authenticated Platform-security evidence age <=5s
+ authenticated signing-key/profile trust/revocation evidence age <=5s
```

Stale, unavailable, unauthenticated, contradictory or unprovable required security/trust evidence fails closed before candidate authority mutation.

Fresh authenticated evidence explicitly showing the exact admission key/profile unknown, revoked or not trusted is security-terminal for that credential. Nominal JWT expiry never overrides newer security revocation.

This pre-admission veto does not grant Platform authority over an already-admitted GameSession.

## 10. AdmissionAttemptRef and GrantNonce

`attempt_ref` is Platform producer operation/correlation identity; `jti` is the game consume/replay GrantNonce. They are never aliases.

If Platform cannot prove whether an issuance attempt succeeded, the same AdmissionAttemptRef may be used only for bounded reconciliation/status recovery. Platform must not mint a blind second independently usable capability. Before a new independent attempt, the old attempt must be deterministically retired and any possibly issued capability proven no longer acceptable.

One GrantNonce can participate in at most one successful authoritative admission commit. Replay/concurrent use has at most one winner and cannot duplicate or revive authority.

## 11. Fresh-admission error subset

FND-04A owns the complete error-vocabulary shape for errors originating in its bounded fresh-admission scope. FND-04C may integrate these accepted rows into the final FND-04 catalogue but may not silently alter their semantics.

Every row uses the common credential-free diagnostic envelope:

```text
error_code
request_trace_id
admission_attempt_ref (only when parsed/authorized for diagnostics)
profile_id = oteryn-pre-admission-v1 when known
safe_kid when known and policy permits
```

Never include raw JWT, GrantNonce, OAuth/Game Login Ticket, reconnect secret, private fencing data, Platform security-generation internals, SQL errors or unstable exception text.

| Internal code | Category | Progression | Retry / next authority | Mutation outcome | Public class | Redacted diagnostic message | Additional credential-free correlation fields |
|---|---|---|---|---|---|---|---|
| `ADMISSION_GRANT_MALFORMED` | `INVALID_INPUT` | `TERMINAL` | never same malformed grant; newly issued valid capability | no consume/presence/lease/session mutation | `RETRY_LOGIN` | `fresh admission grant malformed` | parser stage, bounded profile/header classification if safely parsed |
| `ADMISSION_GRANT_AUTHENTICATION_FAILED` | `AUTHENTICATION_FAILED` | `SECURITY_TERMINAL` | never same credential; restart authenticated issuance | no authoritative mutation | `AUTHENTICATION_REQUIRED` | `fresh admission credential authentication failed` | safe_kid, trust decision revision/class |
| `ADMISSION_GRANT_NOT_YET_VALID` | `SESSION_REJECTED` | `RETRYABLE` | same unconsumed grant only after accepted nbf window and while every other binding remains current | no GrantNonce or authority mutation | `TEMPORARILY_UNAVAILABLE` | `fresh admission grant not yet active` | trusted-time boundary class, profile_id |
| `ADMISSION_GRANT_EXPIRED` | `SESSION_REJECTED` | `TERMINAL` | fresh issuer/Gateway attempt | no authoritative mutation | `RETRY_LOGIN` | `fresh admission grant expired` | trusted-time boundary class, profile_id |
| `ADMISSION_GRANT_REPLAYED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | reconcile prior admission; never reuse consumed grant | prior success may exist; no duplicate effect | `SESSION_UNAVAILABLE` | `fresh admission grant already consumed or replayed` | replay receipt/correlation reference, attempt_ref |
| `ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same AdmissionAttemptRef reconciliation only until deterministic retirement/proof | producer ambiguity creates no gameplay authority | `TEMPORARILY_UNAVAILABLE` | `fresh admission issuance outcome requires reconciliation` | attempt_ref, issuer operation status revision |
| `ADMISSION_GRANT_SECURITY_STATE_REVOKED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | new authenticated attempt only after Platform security permits | no authoritative mutation | `AUTHENTICATION_REQUIRED` | `fresh admission denied by current account security state` | security evidence revision/class, no generation value |
| `ADMISSION_GRANT_SECURITY_EVIDENCE_STALE` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same unconsumed grant only after fresh authenticated evidence and while all bindings remain valid | no GrantNonce or authority mutation | `TEMPORARILY_UNAVAILABLE` | `fresh admission security evidence unavailable or stale` | evidence source class, freshness bucket, trust decision revision |
| `ADMISSION_GRANT_ROUTE_STALE` | `STALE_GENERATION` | `TERMINAL` | fresh Gateway route + new grant | no authoritative mutation | `RETRY_LOGIN` | `fresh admission route no longer current` | world_id, channel_id, route_revision |
| `ADMISSION_GRANT_RUNTIME_GENERATION_STALE` | `STALE_GENERATION` | `TERMINAL` | fresh current-owner evidence + new grant | no authoritative mutation | `RETRY_LOGIN` | `fresh admission runtime ownership no longer current` | world_id, channel_id, runtime_observation_revision, scope_ownership_generation |
| `ADMISSION_GRANT_WORLD_STALE` | `STALE_GENERATION` | `TERMINAL` | resolve current character world then obtain newly authorized route/grant; never retarget old grant | no GrantNonce or presence/lease/session/transport mutation | `RETRY_LOGIN` | `fresh admission character world binding no longer matches` | grant world_id, current character-world relation revision/class, route_revision |
| `ADMISSION_GRANT_REVISION_UNSUPPORTED` | `UNSUPPORTED_REVISION` | `TERMINAL` | compatible producer/client/consumer only; no downgrade | no authoritative mutation | `CLIENT_UPDATE_REQUIRED` | `fresh admission revision unsupported` | protocol_major, transport_profile, compatibility_revision |
| `ADMISSION_ACCOUNT_CHARACTER_CONFLICT` | `CONFLICT` | `TERMINAL` | new attempt only after authoritative ownership/lifecycle change | no partial admission | `SESSION_UNAVAILABLE` | `fresh admission account or character relationship conflicts with current authority` | lifecycle/ownership decision revision, world_id |
| `ADMISSION_INCUMBENT_PROTECTED` | `CONFLICT` | `TERMINAL` | new attempt only after authoritative incumbent eligibility changes | incumbent unchanged; newcomer no authority | `CHARACTER_ALREADY_ACTIVE` | `fresh admission blocked by current character authority` | incumbent state class, world_id/channel_id where policy permits |
| `ADMISSION_CAPACITY_EXCEEDED` | `CAPACITY_EXCEEDED` | `RETRYABLE` | bounded backoff; same grant only on same current route while valid, else new route/grant | no partial admission authority | `TEMPORARILY_UNAVAILABLE` | `fresh admission capacity unavailable` | capacity class, world_id/channel_id, route_revision |

## 12. Required fault/interoperability evidence

Before implementation acceptance, independent fixtures must cover at least:

### Credential/profile

- canonical Ed25519 grant;
- `alg=none`, deprecated `EdDSA`, wrong key type/curve;
- token-directed key discovery attempts;
- malformed/duplicate/unknown claims;
- wrong issuer/audience/type/profile/purpose;
- UUIDv7/variant/canonical encoding failures;
- exact `nbf`, expiry and lifetime/skew boundaries;
- key rotation, exact 5s trust evidence, >5s/unavailable/contradictory evidence and emergency revocation;
- Platform-security stale/revoked cases;
- replay/concurrent consume;
- ambiguous producer issuance/reconciliation.

### World binding — mandatory carried-P1 evidence

1. grant `world_id` differs from the character's current authoritative WorldId before admission evaluation -> `ADMISSION_GRANT_WORLD_STALE`, no GrantNonce/authority mutation;
2. initial current-world validation passes, then legal world transfer/current world relation changes before final commit -> final atomic revalidation rejects as `ADMISSION_GRANT_WORLD_STALE`;
3. stale grant is never silently retargeted to the new/current world or Channel;
4. concurrent transfer/admission ordering has one linearized authoritative outcome and the losing candidate preserves actual current transfer/world/presence/lease/session state.

### Change-before-commit matrix

After each earlier check succeeds, independently change before the final boundary:

- JWT time window;
- signing-key/profile trust/freshness;
- Platform-security evidence/account state;
- route/runtime observation/current target lifecycle/ownership/readiness;
- protocol/transport/compatibility;
- current CharacterId->WorldId/world eligibility;
- GrantNonce;
- AccountId->CharacterId ownership/lifecycle;
- AccountPresence/incumbent state;
- CharacterLease/fence generation;
- superseding handoff/fence/takeover/terminal/transfer authority.

Each case must fail before candidate authority mutation under its specific FND-04A outcome, and AccountPresenceClaim, CharacterLease, GameSession and TransportBinding must become authoritative only together for the winner.

## 13. Security/privacy

Never log/export raw grant JWT, GrantNonce, reusable credentials, signing private key, reconnect secret material or verifier digest.

High-cardinality AccountId/CharacterId do not become ordinary metric labels. Diagnostic access to identifiers follows privacy/access policy. Diagnostic templates are stable redacted contract text, not raw dependency/exception strings.

## 14. Downstream integration

### FND-04B

Consumes the accepted authority identities and session/transport starting state to define reconnect, recovery and continuity. FND-04B must not weaken FND-04A's fresh-admission final authority boundary.

### FND-04C

Integrates accepted FND-04A/FND-04B error rows, failure scenarios, compatibility/evidence and final thin FND-04 index/status. It may allocate/compose final diagnostics/error surfaces but cannot silently change FND-04A semantics.

### DUR / OPS / PERF

Physical atomicity, replay/presence/lease persistence, lease numeric safety, capacities and operational placement remain separately evidence-gated.

## 15. Acceptance boundary

Merging FND-04A means only:

- fresh-admission authority layers and final linearization are accepted;
- the strict fresh-entry profile is accepted;
- current CharacterId->WorldId/world eligibility is mandatory at final admission;
- the fresh-admission error subset has complete Foundation Error Vocabulary shape;
- no runtime implementation is authorized;
- overall FND-04 remains incomplete until FND-04B and FND-04C merge and lifecycle closeout completes.

## 16. Concise rule

```text
Platform authenticates and signs a bounded fresh-entry attempt
-> never gameplay authority

Oteryn-v2 validates
-> account security + key/profile trust <=5s
-> route/runtime/current target
-> AccountId -> CharacterId
-> CharacterId -> current WorldId/world eligibility
-> presence + lease + nonce + compatibility

all precommit checks
-> eligibility only
-> never authorization escrow

one atomic final boundary
-> revalidate every mutable predicate including current character world
-> if any changed: fail, no candidate authority, no retarget, preserve current state
-> if all valid: consume GrantNonce + establish presence + lease + GameSession + connection_generation 1

FND-04A merge
-> fresh admission architecture accepted
-> reconnect/recovery and final integration still pending
-> no runtime implementation authorization
```
