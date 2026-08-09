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

FND-04A freezes only authority/security semantics required for **fresh native gameplay admission**.

```text
Platform authenticates and authorizes one bounded attempt.
Oteryn-v2 alone decides whether current game-domain facts permit gameplay authority.
No earlier validation escrows that authority.
```

FND-04A does not complete FND-04. Reconnect/recovery/continuity belongs to FND-04B; final error/failure/compatibility integration belongs to FND-04C.

### Decision timing

| Decision | Now? | Blocks | Risk if wrong later | Superseding evidence | Deferred |
|---|---|---|---|---|---|
| Platform attempt vs game final authority | `YES` | issuer/consumer/session creation | dual authority | reviewed replacement consistent with ADR-0003/0012 | service placement |
| Separate presence/lease/session/transport/runtime authority | `YES` | fencing/persistence/duplicate login | stale-writer aliasing | formal/fault proof of equivalent separation | tables/locks |
| Atomic final admission linearization | `YES` | replay/presence/lease integration | TOCTOU/partial authority | equivalent single-winner proof | transaction primitive |
| Current CharacterId->WorldId/world eligibility | `YES` | transfer safety | stale grant attaches character to wrong world | explicit fenced transfer contract | transfer implementation |
| Separate signed gameplay revision dimensions | `YES` | rollout compatibility | stale/mixed content/rules/policy admission | reviewed replacement compatibility scheme | physical revision registry |
| Strict fresh-entry Ed25519 profile | `YES` | issuer/verifier | cross-purpose credential confusion | reviewed profile revision | JWT/KMS implementation |
| Security/trust evidence age <=5s | `YES` | revocation behavior | unbounded stale trust | measured superseding threat model | transport/cadence within ceiling |
| Production lease/liveness/capacity values | `NO` | implementation acceptance | guessed unsafe values | PERF/OPS/DUR evidence | numeric values |

The <=5s trust-evidence policy intentionally accepts a bounded residual revocation-detection window: a revocation that occurs just after an authenticated evidence snapshot may remain undetectable until refreshed, but the verifier may never use that evidence after age 5s. FND-04A does not claim instantaneous globally atomic revocation.

## 2. Canonical fresh-admission authority layers

### 2.1 AccountPresenceClaim

Scope `AccountId`; identifies the current playable/mandatory-presence CharacterId and enforces account-global exclusion. It is not a GameSession. Eligibility may be evaluated before commit, but authority begins only in Section 7.

### 2.2 CharacterLease

Scope `CharacterId + character_lease_generation`. Generation is non-zero monotonic uint64-class state or exact non-reused equivalent; stale generation cannot renew, commit durable mutation or create control; exhaustion never wraps/reuses. Acquisition/advance becomes authoritative only at final admission commit.

### 2.3 GameSession

GameSessionId is created only by successful game-domain admission. It is identity, not bearer proof. A precommit candidate ID is discarded/never reused on failure.

### 2.4 TransportBinding

First admitted binding is `GameSessionId + connection_generation = 1`; generation 0 remains pre-admission. Reconnect/rebind is FND-04B.

### 2.5 RuntimeScopeAuthority

Current ChannelRuntime/InstanceRuntime semantic scope plus accepted FND-03 ownership generation. NodeId is placement evidence, not authority.

## 3. Platform and game-domain boundary

Platform owns reusable authentication/security, OAuth/PKCE/MFA/recovery, Game Login Ticket lifecycle, account-security generation, configured world/channel/login/maintenance/entitlement policy, Gateway offer/route orchestration and signing one bounded fresh-entry attempt.

Oteryn-v2 owns final AccountId->CharacterId, CharacterId->WorldId/world eligibility, presence/lease, current runtime target/ownership/readiness, GrantNonce replay state, GameSession/first TransportBinding and final admission outcome.

Platform never creates canonical GameSessionId. A valid Platform signature never bypasses current game facts.

## 4. Fresh-entry credential and independent revisions

Fresh entry uses only `docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md`.

The profile uses JWS Compact JWT, fully specified `alg=Ed25519`, rejects deprecated polymorphic `EdDSA`, uses dedicated typ/issuer/audience/purpose/key purpose, lifetime <=30s, verifier skew <=5s, authenticated Platform-security evidence <=5s, authenticated signing-key/profile trust evidence <=5s, one-time 32-byte GrantNonce and distinct AdmissionAttemptRef.

The grant MUST bind separate authoritative gameplay dimensions rather than one overloaded compatibility token:

```text
protocol_major
transport_profile
ruleset_revision
content_revision
map_revision
world_policy_revision
offer_revision
```

`route_revision` and `runtime_observation_revision` remain separate routing/runtime evidence. `scope_ownership_generation` remains a separate authority fence. These dimensions are not aliases and cannot be silently composed into a generic `compatibility_revision` under v1.

OAuth credentials and Game Login Tickets are never accepted by the game server as this grant.

## 5. Current character-world binding

CharacterId is globally stable and may survive legal world transfer. Distinguish:

```text
AccountId owns CharacterId
CharacterId is currently eligible for WorldId
Gateway route targets WorldId/ChannelId
```

World state is evaluated only after current AccountId->CharacterId ownership/lifecycle is proven, preventing world classification from becoming an oracle for a non-owned candidate.

After ownership is proven, and again at final commit:

```text
current_character_world_id == grant.world_id
AND current lifecycle permits fresh admission to grant.world_id
```

Valid ownership + mismatch/change before commit -> `ADMISSION_GRANT_WORLD_STALE`: no GrantNonce consumption, no candidate presence/lease/session/transport authority, no retarget to another world/channel, preserve current transfer/world authority, require newly authorized route/grant.

Invalid ownership fails as `ADMISSION_ACCOUNT_CHARACTER_CONFLICT` before any world-mismatch result.

## 6. Route/runtime/revision applicability

Grant binds `world_id`, `channel_id`, `route_revision`, `runtime_observation_revision`, `scope_ownership_generation`, `protocol_major`, `transport_profile`, `ruleset_revision`, `content_revision`, `map_revision`, `world_policy_revision`, `offer_revision`.

Reject non-open target, superseded route/runtime observation, changed scope ownership, non-current runtime owner/placement/readiness, unsupported/mismatched protocol/transport or any independent gameplay revision, and—after ownership is proven—stale character-world eligibility.

Every authoritative revision is compared independently against the current target. Updating any one dimension invalidates a grant carrying the older value even when all others remain unchanged.

No silent retarget/downgrade to another World, Channel, owner, content/ruleset/map/policy/offer generation, protocol family or Canary path.

## 7. Atomic fresh-admission linearization

Precommit checks are fail-fast eligibility only.

1. FND-02 material limits;
2. parser/header/profile bounds;
3. current authenticated key/profile trust evidence age <=5s + trusted key lookup;
4. signature;
5. exact issuer/audience/type/purpose/time/profile;
6. canonical claims/UUID/generations;
7. current Platform-security freshness/revocation/generation;
8. route/runtime/current target/ownership + independent protocol/transport/ruleset/content/map/world-policy/offer revisions;
9. GrantNonce eligibility;
10. current AccountId->CharacterId ownership/lifecycle;
11. current CharacterId->WorldId/world eligibility only after step 10;
12. AccountPresence/duplicate-login eligibility;
13. CharacterLease/current runtime-scope acquisition/readiness;
14. one atomic final revalidation + authority commit;
15. publish success after commit only.

### 7.1 Final revalidation

Immediately before/atomically with authority creation revalidate:

- JWT time/lifetime/skew;
- exact key/profile trust and trust/revocation evidence age <=5s;
- current Platform-security evidence age <=5s and account generation/state;
- route/runtime observation, target lifecycle, scope ownership, runtime owner/placement/readiness;
- protocol_major and transport_profile;
- each `ruleset_revision`, `content_revision`, `map_revision`, `world_policy_revision`, `offer_revision` independently;
- AccountId->CharacterId ownership/lifecycle first;
- CharacterId->WorldId/world eligibility second;
- GrantNonce;
- AccountPresence/incumbent state;
- CharacterLease/fence state;
- absence of newer transfer/handoff/fence/takeover/terminal authority.

### 7.2 Atomic effects

Only if all remain valid:

```text
consume GrantNonce
+ establish/advance AccountPresenceClaim as required
+ establish/acquire CharacterLease as required
+ create canonical GameSessionId
+ GameSession ACTIVE
+ connection_generation = 1
+ establish initial authoritative session/reconciliation boundary
```

FND-04A defines no reconnect proof/secret; that belongs to FND-04B.

Failure before/during commit creates no candidate partial authority and never rolls back actual current world-transfer/presence/lease/runtime/session authority.

## 8. Account-global exclusion and duplicate login

Two different CharacterIds for one AccountId cannot both become playable/mandatory-presence actors. A fresh grant alone cannot fence a protected incumbent, close its transport, release presence, replace lease or admit another character. Concurrent candidates have at most one final-boundary winner.

Takeover/handoff continuity beyond this no-preemption invariant is FND-04B/C.

## 9. Security/trust freshness and bounded revocation detection

Fresh admission requires:

```text
short-lived signed grant
+ account_security_generation
+ authenticated Platform-security evidence age <=5s
+ authenticated signing-key/profile trust/revocation evidence age <=5s
```

Evidence older than 5s, unavailable, unauthenticated, contradictory or unprovable fails closed. Fresh accepted evidence explicitly recording exact key/profile unknown/revoked/not-trusted is security-terminal.

### 9.1 Residual revocation window

The freshness ceiling is a bounded-staleness model, not an atomic revocation fence across repositories.

If a revocation occurs **after** the observation point of trust evidence that is still authenticated and age <=5s, a verifier cannot infer that unseen event. The grant may remain acceptable to the trust check until either:

- newer authenticated evidence records the revocation -> `ADMISSION_GRANT_AUTHENTICATION_FAILED`; or
- the prior evidence exceeds 5s without a fresh provable replacement -> `ADMISSION_GRANT_SECURITY_EVIDENCE_STALE`.

Thus worst-case revocation detection attributable to this evidence contract is bounded by the five-second accepted age ceiling. Any future requirement for zero-window revocation requires a separately reviewed atomic epoch/fence design and cross-repository rollout.

This bounded pre-admission veto gives Platform no post-admission GameSession authority.

## 10. AdmissionAttemptRef and GrantNonce

`attempt_ref` is Platform issuance/reconciliation identity; `jti` is one-time game consume identity.

Ambiguous issuance permits same-AdmissionAttemptRef status/reconciliation only; no blind second capability. New independent attempt requires deterministic retirement plus proof any possibly issued old capability is no longer acceptable.

One GrantNonce -> at most one successful admission; losing replay cannot duplicate/revive/fence authority.

## 11. Fresh-admission error subset

FND-04A owns full Foundation Error Vocabulary shape for its fresh-admission errors. FND-04C may integrate but not silently alter accepted rows.

Common diagnostic envelope: `error_code`, `request_trace_id`, safe `admission_attempt_ref` when parsed/authorized, `profile_id` when known, `safe_kid` when known/policy-permitted. Never include raw JWT/GrantNonce, reusable credentials, Platform security-generation values, private fencing generation, SQL errors or unstable exception strings.

| Internal code | Category | Progression | Retry / next authority | Mutation outcome | Public class | Redacted diagnostic | Extra credential-free correlation |
|---|---|---|---|---|---|---|---|
| `ADMISSION_GRANT_MALFORMED` | `INVALID_INPUT` | `TERMINAL` | new valid capability; never same malformed grant | no authority mutation | `RETRY_LOGIN` | `fresh admission grant malformed` | parser stage; safe profile/header class |
| `ADMISSION_GRANT_AUTHENTICATION_FAILED` | `AUTHENTICATION_FAILED` | `SECURITY_TERMINAL` | restart authenticated issuance; never same credential | no authority mutation | `AUTHENTICATION_REQUIRED` | `fresh admission credential authentication failed` | safe_kid; trust decision class/revision |
| `ADMISSION_GRANT_BINDING_MISMATCH` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | obtain a newly issued grant with correct issuer/audience/type/purpose; never reinterpret same credential | no authority mutation | `RETRY_LOGIN` | `fresh admission credential bound to a different context` | mismatch class only: issuer/audience/type/purpose; do not echo untrusted value |
| `ADMISSION_GRANT_NOT_YET_VALID` | `SESSION_REJECTED` | `RETRYABLE` | same unconsumed grant only after accepted nbf window while all bindings current | no nonce/authority mutation | `TEMPORARILY_UNAVAILABLE` | `fresh admission grant not yet active` | trusted-time boundary class |
| `ADMISSION_GRANT_EXPIRED` | `SESSION_REJECTED` | `TERMINAL` | fresh issuer/Gateway attempt | no authority mutation | `RETRY_LOGIN` | `fresh admission grant expired` | trusted-time boundary class |
| `ADMISSION_GRANT_REPLAYED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | reconcile prior admission; never reuse grant | prior success may exist; no duplicate | `SESSION_UNAVAILABLE` | `fresh admission grant already consumed or replayed` | replay receipt/correlation ref |
| `ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same AdmissionAttemptRef reconciliation until deterministic retirement/proof | ambiguity creates no gameplay authority | `TEMPORARILY_UNAVAILABLE` | `fresh admission issuance outcome requires reconciliation` | attempt_ref; operation-status revision |
| `ADMISSION_GRANT_SECURITY_STATE_REVOKED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | new authenticated attempt only after account security permits | no authority mutation | `AUTHENTICATION_REQUIRED` | `fresh admission denied by current account security state` | security decision class/revision only |
| `ADMISSION_GRANT_SECURITY_EVIDENCE_STALE` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same unconsumed grant only after fresh authenticated evidence while all bindings valid | no nonce/authority mutation | `TEMPORARILY_UNAVAILABLE` | `fresh admission security evidence unavailable or stale` | evidence source class; freshness bucket; trust decision revision |
| `ADMISSION_GRANT_ROUTE_STALE` | `STALE_GENERATION` | `TERMINAL` | fresh Gateway route + grant | no authority mutation | `RETRY_LOGIN` | `fresh admission route no longer current` | world_id; channel_id; route_revision |
| `ADMISSION_GRANT_RUNTIME_GENERATION_STALE` | `STALE_GENERATION` | `TERMINAL` | fresh current-owner evidence + grant | no authority mutation | `RETRY_LOGIN` | `fresh admission runtime ownership no longer current` | world_id; channel_id; runtime_observation_revision; match/stale class only |
| `ADMISSION_GRANT_WORLD_STALE` | `STALE_GENERATION` | `TERMINAL` | resolve current world then newly authorized route/grant; no retarget | no nonce/presence/lease/session/transport mutation | `RETRY_LOGIN` | `fresh admission character world binding no longer matches` | signed world_id; relation revision/class; no transfer details |
| `ADMISSION_GRANT_REVISION_UNSUPPORTED` | `UNSUPPORTED_REVISION` | `TERMINAL` | compatible producer/client/consumer revisions; no downgrade | no authority mutation | `CLIENT_UPDATE_REQUIRED` | `fresh admission authoritative revision unsupported` | mismatch dimension class plus accepted/non-secret revision IDs for protocol/transport/ruleset/content/map/world-policy/offer where policy permits |
| `ADMISSION_ACCOUNT_CHARACTER_CONFLICT` | `CONFLICT` | `TERMINAL` | new attempt only after ownership/lifecycle change | no partial admission | `SESSION_UNAVAILABLE` | `fresh admission account or character relationship conflicts with current authority` | ownership/lifecycle decision class; world only after ownership-safe evaluation |
| `ADMISSION_INCUMBENT_PROTECTED` | `CONFLICT` | `TERMINAL` | new attempt only after incumbent eligibility changes | incumbent unchanged; newcomer no authority | `CHARACTER_ALREADY_ACTIVE` | `fresh admission blocked by current character authority` | incumbent state class; world/channel where policy permits |
| `ADMISSION_CAPACITY_EXCEEDED` | `CAPACITY_EXCEEDED` | `RETRYABLE` | bounded backoff; same grant only on same current route while valid | no partial authority | `TEMPORARILY_UNAVAILABLE` | `fresh admission capacity unavailable` | capacity class; world/channel; route_revision |

Syntactically valid, correctly signed credentials with wrong exact `iss`, `aud`, `typ` or `purpose` use `ADMISSION_GRANT_BINDING_MISMATCH`; unsupported profile revision uses `ADMISSION_GRANT_REVISION_UNSUPPORTED`; malformed/missing/noncanonical structure remains `ADMISSION_GRANT_MALFORMED`.

## 12. Required evidence

### Credential/profile/revision

Independent fixtures cover Ed25519 positive/negative/algorithm confusion, token-directed key discovery, parser/claim/UUID failures, exact binding mismatch (`iss/aud/typ/purpose`), unsupported profile, nbf/expiry/skew/lifetime, key rotation, trust evidence exactly 5s vs >5s/unavailable, account-security stale/revoked, replay/concurrent consume, ambiguous issuance, and independent revision mismatch for **each** ruleset/content/map/world-policy/offer dimension while the other dimensions remain unchanged.

### Revocation timing

Require two distinct cases:

1. final accepted authenticated trust evidence already records revocation -> `ADMISSION_GRANT_AUTHENTICATION_FAILED`, no nonce/authority mutation;
2. revocation occurs after the observation point of still-valid <=5s evidence -> test must not pretend instantaneous detection; prove the credential becomes unacceptable at the first newer evidence recording revocation or no later than expiry of the previous 5s evidence window, with no extension by stale cached evidence.

### World/ownership

- invalid AccountId->CharacterId -> account/character conflict before world classification;
- valid ownership + initial world mismatch -> `ADMISSION_GRANT_WORLD_STALE`;
- ownership/world initially valid then legal transfer/world change before final commit -> final `ADMISSION_GRANT_WORLD_STALE`;
- stale grant never retargeted;
- concurrent transfer/admission has one authoritative outcome and loser preserves current state.

### Change-before-commit

Independently mutate after earlier validation: JWT time; trust/security evidence; route/runtime/target; protocol or any independent ruleset/content/map/world-policy/offer revision; AccountId->CharacterId; CharacterId->WorldId/world eligibility; GrantNonce; AccountPresence/incumbent; CharacterLease/fence; superseding transfer/handoff/fence/takeover/terminal authority.

Each loser fails before candidate authority mutation; presence/lease/GameSession/TransportBinding become authoritative together only for the winner.

## 13. Security/privacy

Never log raw grant/nonce/reusable credential/private key. AccountId/CharacterId do not become ordinary metric labels. Diagnostic templates are stable redacted text and correlation fields avoid private fencing/security generations.

## 14. Downstream integration

FND-04B consumes accepted authority/session starting state for reconnect/recovery without weakening A. FND-04C integrates A/B errors, failure scenarios, rollout/evidence and thin final FND-04 index without silently changing accepted component semantics. DUR/OPS/PERF own physical persistence/atomicity and measured production values.

## 15. Acceptance boundary

FND-04A merge accepts only fresh-admission authority, strict profile, independent revision bindings, ownership-safe current-world validation and complete A-error shape. It authorizes no runtime implementation and does not complete FND-04.

## 16. Concise rule

```text
Platform bounded grant
-> no gameplay authority

signed dimensions
-> protocol + transport + ruleset + content + map + world-policy + offer separately
-> no opaque compatibility overload

Oteryn-v2
-> trust/security evidence <=5s
-> bounded residual revocation-detection window explicitly accepted
-> ownership FIRST, current world SECOND
-> route/runtime/revisions + nonce + presence + lease

atomic final boundary repeats all mutable facts
-> valid ownership + stale world => ADMISSION_GRANT_WORLD_STALE
-> wrong aud/typ/purpose => ADMISSION_GRANT_BINDING_MISMATCH
-> all valid => one admission authority commit

reconnect/recovery
-> FND-04B, not FND-04A
```
