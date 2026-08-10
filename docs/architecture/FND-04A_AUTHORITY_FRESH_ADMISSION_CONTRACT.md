# FND-04A — Authority and Fresh Admission Contract

- Status: Candidate bounded architecture contract; canonical for FND-04A when bounded successor delivery from Issue #120 merges
- Gate: `FND-04A`
- Replacement programme: Issue #112
- Owning successor repair: Issue #120 (`FND-04A-R1`)
- Reconstructed reviewed candidate: PR #114 exact head `79678485d009c22ece2736c822d6b75b6d235ad2`; #114 remains superseded/unmerged after exhausted repair budget
- Successor repairs: deterministic protected-header/binding classification, verifier-anchored pre-signature trust scope, and complete cryptographic/payload-schema classification precedence
- Repository: `blakinio/Oteryn-v2`
- Trusted reconstruction base: `main@43ca28f1f0f259c08a275c92946aa35f05d4d112`
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
| Authenticated freshness provenance + monotonic evidence fence | `YES` | revocation/security projection | cache re-age or rollback can resurrect stale allow | stronger atomic epoch/fence with equivalent proof | physical cache/storage transport |
| Strict fresh-entry Ed25519 profile | `YES` | issuer/verifier | cross-purpose credential confusion | reviewed profile revision | JWT/KMS implementation |
| Security/trust evidence age <=5s | `YES` | revocation behavior | unbounded stale trust | measured superseding threat model | transport/cadence within ceiling |
| Production lease/liveness/capacity values | `NO` | implementation acceptance | guessed unsafe values | PERF/OPS/DUR evidence | numeric values |

The <=5s policy is bounded-staleness, not instantaneous cross-repository revocation. That bounded window is valid only when freshness is measured from authenticated **source observation provenance** and older source decisions cannot roll back a newer accepted decision.

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

The profile uses JWS Compact JWT, fully specified `alg=Ed25519`, rejects algorithm negotiation/downgrade and deprecated polymorphic `EdDSA`, uses dedicated typ/issuer/audience/purpose/key purpose, lifetime <=30s, verifier skew <=5s, authenticated Platform-security evidence <=5s, authenticated signing-key/profile trust evidence <=5s, one-time 32-byte GrantNonce and distinct AdmissionAttemptRef.

Deterministic credential classification is part of the security contract:

- malformed protected/JWS/JSON structure -> `ADMISSION_GRANT_MALFORMED`;
- syntactically valid non-exact cryptographic `alg`, untrusted fixed-scope `kid`, incompatible key or signature failure -> `ADMISSION_GRANT_AUTHENTICATION_FAILED`;
- semantic payload exact-schema validation waits for signature success; authenticated malformed/missing/unknown/noncanonical payload schema -> `ADMISSION_GRANT_MALFORMED`;
- authenticated wrong exact `typ`/`iss`/`aud`/`purpose` -> `ADMISSION_GRANT_BINDING_MISMATCH`;
- authenticated structurally valid unsupported `profile` -> `ADMISSION_GRANT_REVISION_UNSUPPORTED`.

Before signature verification, the admission trust scope is selected exclusively from the verifier-configured expected fresh-entry v1 context: expected issuer, audience, profile, purpose and admission-signing key purpose. Unauthenticated token `iss`, `aud`, `profile`, `purpose` or `typ` never selects, broadens or retargets trust. `kid` may select only within that fixed trusted set. Semantic bindings, exact payload schema and unsupported-profile classification occur only after successful signature verification.

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

FND-02 `schema_revision` remains diagnostic/build evidence rather than an exact-equality admission gate and therefore is not added as a signed FND-04A v1 compatibility requirement.

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

Precommit checks are fail-fast eligibility only until the atomic authority boundary.

1. FND-02 outer material limits;
2. JWS/parser/size/base64/JSON structural bounds;
3. protected-header shape: exact required member set and bounded syntactic `alg`/`kid`/`typ`; malformed/missing/extra/forbidden shape -> `ADMISSION_GRANT_MALFORMED`;
4. cryptographic algorithm policy: syntactically valid non-exact `alg` -> `ADMISSION_GRANT_AUTHENTICATION_FAILED`; exact `Ed25519` continues with no negotiation/fallback;
5. using the verifier-configured expected fresh-entry v1 issuer/profile/key-purpose scope only, authenticate current signing-key/profile trust/revocation evidence provenance/freshness/anti-rollback and resolve a well-formed `kid` only inside that fixed trusted set; unknown/untrusted `kid` -> `ADMISSION_GRANT_AUTHENTICATION_FAILED`; unauthenticated token semantic values cannot select or broaden the scope;
6. Ed25519 signature; failure -> `ADMISSION_GRANT_AUTHENTICATION_FAILED`;
7. after successful signature, exact payload claim membership/types/canonical encodings; missing/unknown/wrong-type/noncanonical schema -> `ADMISSION_GRANT_MALFORMED`;
8. after schema success, exact issuer/audience/type/purpose semantics -> `ADMISSION_GRANT_BINDING_MISMATCH` on mismatch; structurally valid unsupported profile -> `ADMISSION_GRANT_REVISION_UNSUPPORTED`;
9. JWT time/lifetime/skew;
10. current Platform-security evidence provenance/freshness/anti-rollback + account generation/state;
11. route/runtime/current target/ownership + independent protocol/transport/ruleset/content/map/world-policy/offer revisions;
12. GrantNonce eligibility;
13. current AccountId->CharacterId ownership/lifecycle;
14. current CharacterId->WorldId/world eligibility only after step 13;
15. AccountPresence/duplicate-login eligibility;
16. CharacterLease/current runtime-scope acquisition/readiness;
17. one atomic final revalidation + authority commit;
18. publish success after commit only.

This ordering is normative. A well-formed token cannot use algorithm, key, binding, profile or exact payload-schema differences as a pre-authentication oracle. An invalid signature wins over any otherwise well-formed semantic payload defect because semantic exact-schema/binding/profile classification occurs only after authentication.

### 7.1 Final revalidation

Immediately before/atomically with authority creation revalidate:

- JWT time/lifetime/skew;
- exact key/profile trust for the verifier-configured expected fresh-entry v1 context using authenticated source observation provenance, accepted upper-bound age <=5s and non-rollback source revision/fence;
- current Platform-security evidence using authenticated source observation provenance, accepted upper-bound age <=5s, non-rollback source revision/fence and account generation/state;
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

## 9. Security/trust evidence provenance, freshness and anti-rollback

Fresh admission requires both Platform-security evidence and admission signing-key/profile trust/revocation evidence to carry authenticated semantics sufficient to prove:

```text
source authority / purpose / scope
source_observed_at (or equivalently strong authenticated source-time provenance)
monotonic/comparable source_revision (or equivalently strong non-rollback decision fence)
current decision facts
```

For signing-key/profile trust before signature verification, the scope above is the **verifier-configured expected issuer/profile/key-purpose fresh-entry v1 scope**, not a scope named by unauthenticated token claims. Exact wire/storage names and transport remain implementation choices, but these semantics are mandatory.

### 9.1 Freshness is source age, never cache age

For each required evidence object, accepted age is the conservative upper bound on elapsed time from the authenticated **source observation** to current trusted game-server time, including known clock uncertainty:

```text
upper_bound_source_age <= 5 seconds
```

A consumer/cache receive time, database update time, cache refresh time, reserialization time or local re-read time MUST NOT reset or reduce evidence age.

If authenticated source observation provenance is absent, future/ambiguous, contradictory, or clock uncertainty prevents proving the upper-bound source age <=5s, the evidence is not fresh and admission fails as `ADMISSION_GRANT_SECURITY_EVIDENCE_STALE`.

This makes the 5s ceiling a real authority bound rather than a cache TTL.

### 9.2 Monotonic anti-rollback fence

Freshness alone is insufficient because an older allow snapshot may still be younger than five seconds.

For each evidence authority/scope, Oteryn-v2 MUST reject authorization from a source revision/fence older than the highest accepted comparable revision already established for that scope. Equal revisions with contradictory authenticated content are invalid. Arrival order never overrides source ordering.

At minimum this applies independently to:

- Platform account-security evidence for the relevant AccountId/security purpose;
- admission signing-key/profile trust/revocation evidence for the verifier-configured expected issuer/profile/key-purpose fresh-entry v1 trust scope.

Consequences:

- after a newer Platform-security revision raises the minimum accepted generation, disables or revokes an account, an older allow revision can never re-authorize even if its source age remains <=5s;
- after a newer trust revision revokes/untrusts a key/profile in the configured expected trust scope, an older trusted revision can never restore trust even if its source age remains <=5s;
- a delayed/replayed cache record cannot move the accepted evidence floor backward;
- on process/storage recovery, the consumer must reconstruct a current non-rollback floor from authoritative evidence or preserved trusted state before authorizing; inability to prove the floor fails closed rather than assuming revision zero/latest-arrival.

Physical persistence/distribution of this floor is deferred; the safety property is not.

### 9.3 Bounded residual revocation window

The model is bounded-staleness, not an instantaneous globally atomic revocation fence.

If a revocation occurs **after** the observation point of the latest accepted authenticated evidence and before a newer source revision is observable, the verifier cannot infer that unseen source event. The prior evidence may remain usable only while:

- its conservative source age remains <=5s; and
- no newer comparable source revision/fence has been accepted.

The credential becomes unacceptable at the first of:

- a newer accepted Platform-security/trust revision establishing the restriction; or
- inability to prove the existing evidence source age remains <=5s.

Thus the accepted residual detection window is bounded by the five-second source-age ceiling and cannot be extended by cache refresh or rollback. Any zero-window requirement needs a separately reviewed cross-repository atomic epoch/fence.

### 9.4 Failure mapping

- stale/unavailable/unauthenticated/contradictory/unprovable provenance, source age or anti-rollback order -> `ADMISSION_GRANT_SECURITY_EVIDENCE_STALE`, no candidate nonce/authority mutation;
- current accepted Platform-security evidence explicitly denies/revokes -> `ADMISSION_GRANT_SECURITY_STATE_REVOKED`;
- current accepted signing-key/profile evidence for the verifier-configured expected trust scope explicitly marks exact key/profile unknown/revoked/not-trusted -> `ADMISSION_GRANT_AUTHENTICATION_FAILED`.

This is a pre-admission veto only; Platform gains no post-admission GameSession authority.

## 10. AdmissionAttemptRef and GrantNonce

`attempt_ref` is Platform issuance/reconciliation identity; `jti` is one-time game consume identity.

Ambiguous issuance permits same-AdmissionAttemptRef status/reconciliation only; no blind second capability. New independent attempt requires deterministic retirement plus proof any possibly issued old capability is no longer acceptable.

One GrantNonce -> at most one successful admission; losing replay cannot duplicate/revive/fence authority.

## 11. Fresh-admission error subset

FND-04A owns full Foundation Error Vocabulary shape for its fresh-admission errors. FND-04C may integrate but not silently alter accepted rows.

Common diagnostic envelope: `error_code`, `request_trace_id`, safe `admission_attempt_ref` when parsed/authorized, `profile_id` when known, `safe_kid` when known/policy-permitted. Never include raw JWT/GrantNonce, reusable credentials, Platform security-generation values, raw evidence fence/revision values when policy treats them as sensitive, private fencing generation, SQL errors or unstable exception strings.

| Internal code | Category | Progression | Retry / next authority | Mutation outcome | Public class | Redacted diagnostic | Extra credential-free correlation |
|---|---|---|---|---|---|---|---|
| `ADMISSION_GRANT_MALFORMED` | `INVALID_INPUT` | `TERMINAL` | new valid capability; never same malformed grant | no authority mutation | `RETRY_LOGIN` | `fresh admission grant malformed` | parser/schema stage; safe profile/header class only after authentication when semantic |
| `ADMISSION_GRANT_AUTHENTICATION_FAILED` | `AUTHENTICATION_FAILED` | `SECURITY_TERMINAL` | restart authenticated issuance; never same credential | no authority mutation | `AUTHENTICATION_REQUIRED` | `fresh admission credential authentication failed` | safe algorithm/key/trust decision class; never unauthenticated semantic binding/profile detail |
| `ADMISSION_GRANT_BINDING_MISMATCH` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | newly issued correct-bound grant; never reinterpret same credential | no authority mutation | `RETRY_LOGIN` | `fresh admission credential bound to a different context` | authenticated mismatch class only; never echo untrusted value |
| `ADMISSION_GRANT_NOT_YET_VALID` | `SESSION_REJECTED` | `RETRYABLE` | same unconsumed grant only after accepted nbf window while all bindings current | no nonce/authority mutation | `TEMPORARILY_UNAVAILABLE` | `fresh admission grant not yet active` | trusted-time boundary class |
| `ADMISSION_GRANT_EXPIRED` | `SESSION_REJECTED` | `TERMINAL` | fresh issuer/Gateway attempt | no authority mutation | `RETRY_LOGIN` | `fresh admission grant expired` | trusted-time boundary class |
| `ADMISSION_GRANT_REPLAYED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | reconcile prior admission; never reuse grant | prior success may exist; no duplicate | `SESSION_UNAVAILABLE` | `fresh admission grant already consumed or replayed` | replay receipt/correlation ref |
| `ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same AdmissionAttemptRef reconciliation until deterministic retirement/proof | ambiguity creates no gameplay authority | `TEMPORARILY_UNAVAILABLE` | `fresh admission issuance outcome requires reconciliation` | attempt_ref; operation-status revision |
| `ADMISSION_GRANT_SECURITY_STATE_REVOKED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | new authenticated attempt only after account security permits | no authority mutation | `AUTHENTICATION_REQUIRED` | `fresh admission denied by current account security state` | security decision class/source-order bucket only |
| `ADMISSION_GRANT_SECURITY_EVIDENCE_STALE` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same unconsumed grant only after fresh authenticated non-rollback evidence while all bindings valid | no nonce/authority mutation | `TEMPORARILY_UNAVAILABLE` | `fresh admission security evidence unavailable, stale or superseded` | evidence source class; source-age bucket; rollback/stale class; trust decision bucket |
| `ADMISSION_GRANT_ROUTE_STALE` | `STALE_GENERATION` | `TERMINAL` | fresh Gateway route + grant | no authority mutation | `RETRY_LOGIN` | `fresh admission route no longer current` | world_id; channel_id; route_revision |
| `ADMISSION_GRANT_RUNTIME_GENERATION_STALE` | `STALE_GENERATION` | `TERMINAL` | fresh current-owner evidence + grant | no authority mutation | `RETRY_LOGIN` | `fresh admission runtime ownership no longer current` | world_id; channel_id; runtime_observation_revision; match/stale class only |
| `ADMISSION_GRANT_WORLD_STALE` | `STALE_GENERATION` | `TERMINAL` | resolve current world then newly authorized route/grant; no retarget | no nonce/presence/lease/session/transport mutation | `RETRY_LOGIN` | `fresh admission character world binding no longer matches` | signed world_id; relation revision/class; no transfer details |
| `ADMISSION_GRANT_REVISION_UNSUPPORTED` | `UNSUPPORTED_REVISION` | `TERMINAL` | compatible producer/client/consumer revisions; no downgrade | no authority mutation | `CLIENT_UPDATE_REQUIRED` | `fresh admission authoritative revision unsupported` | authenticated mismatch dimension class plus non-secret revision IDs where policy permits |
| `ADMISSION_ACCOUNT_CHARACTER_CONFLICT` | `CONFLICT` | `TERMINAL` | new attempt only after ownership/lifecycle change | no partial admission | `SESSION_UNAVAILABLE` | `fresh admission account or character relationship conflicts with current authority` | ownership/lifecycle decision class; world only after ownership-safe evaluation |
| `ADMISSION_INCUMBENT_PROTECTED` | `CONFLICT` | `TERMINAL` | new attempt only after incumbent eligibility changes | incumbent unchanged; newcomer no authority | `CHARACTER_ALREADY_ACTIVE` | `fresh admission blocked by current character authority` | incumbent state class; world/channel where policy permits |
| `ADMISSION_CAPACITY_EXCEEDED` | `CAPACITY_EXCEEDED` | `RETRYABLE` | bounded backoff; same grant only on same current route while valid | no partial authority | `TEMPORARILY_UNAVAILABLE` | `fresh admission capacity unavailable` | capacity class; world/channel; route_revision |

Complete v1 credential precedence is:

```text
unparsable / malformed protected shape
-> ADMISSION_GRANT_MALFORMED

well-formed non-exact alg
OR fixed-scope kid/key/trust/signature failure
-> ADMISSION_GRANT_AUTHENTICATION_FAILED

authenticated exact-schema violation
-> ADMISSION_GRANT_MALFORMED

authenticated wrong typ/iss/aud/purpose
-> ADMISSION_GRANT_BINDING_MISMATCH

authenticated unsupported profile
-> ADMISSION_GRANT_REVISION_UNSUPPORTED
```

Unauthenticated semantic binding/profile/schema values never choose the trust scope or preempt an authentication failure.

## 12. Required evidence

### Credential/profile/revision

Independent fixtures cover:

- canonical Ed25519 positive;
- malformed/missing/non-string/out-of-bound `alg` -> malformed;
- well-formed `none`/`EdDSA`/RSA/ECDSA/HMAC/Ed448/other algorithm -> authentication failed, no fallback;
- malformed `kid` -> malformed; well-formed unknown/untrusted `kid` -> authentication failed;
- forbidden/extra protected member or token-directed key discovery -> malformed and never changes trust selection;
- malformed `typ` -> malformed;
- invalid signature + wrong well-formed `typ` -> authentication failed;
- invalid signature + unsupported/wrong well-formed `profile` -> authentication failed;
- invalid signature + otherwise well-formed missing/unknown semantic payload claim -> authentication failed;
- correctly signed missing/unknown/wrong-type/noncanonical payload claim -> malformed;
- correctly signed wrong exact `iss`/`aud`/`typ`/`purpose` -> binding mismatch;
- correctly signed unsupported profile -> revision unsupported;
- same-`kid` token-supplied issuer/profile/purpose trust-retarget attempts cannot escape the fixed verifier trust set;
- nbf/expiry/skew/lifetime, replay/concurrent consume, ambiguous issuance;
- independent mismatch for each ruleset/content/map/world-policy/offer dimension while all other dimensions remain unchanged.

### Security provenance/freshness/anti-rollback

Require independently:

1. evidence carries authenticated source observation provenance and comparable source revision/fence;
2. local cache insert/refresh/re-read does not reset source age;
3. evidence whose source-age upper bound including clock uncertainty cannot be proven <=5s -> `ADMISSION_GRANT_SECURITY_EVIDENCE_STALE`;
4. accept newer allow revision then replay an older still-<5s allow revision -> older revision rejected as superseded;
5. accept newer Platform-security deny/generation-floor revision then replay older still-<5s allow -> deny remains authoritative; no rollback;
6. accept newer key/profile revoke revision then replay older still-<5s trusted revision -> revoke remains authoritative; old trust cannot revive;
7. equal source revision with contradictory authenticated content -> invalid/fail closed;
8. restart/recovery without a provable current non-rollback floor -> no fresh admission until current authoritative evidence/floor is reconstructed;
9. final accepted evidence already records revocation -> purpose-specific terminal denial, no nonce/authority mutation;
10. revocation occurs after latest evidence observation point -> do not assert instant detection; prove authority expires at first newer restrictive revision or when prior source-age proof exceeds 5s, whichever comes first.

### World/ownership

- invalid AccountId->CharacterId -> account/character conflict before world classification;
- valid ownership + initial world mismatch -> `ADMISSION_GRANT_WORLD_STALE`;
- ownership/world initially valid then legal transfer/world change before final commit -> world stale;
- stale grant never retargeted;
- concurrent transfer/admission has one authoritative outcome and loser preserves current state.

### Change-before-commit

Independently mutate after earlier validation: JWT time; key/profile evidence source age/order/decision; Platform-security source age/order/account state; route/runtime/target; protocol or any independent gameplay revision; AccountId->CharacterId; CharacterId->WorldId/world eligibility; GrantNonce; AccountPresence/incumbent; CharacterLease/fence; superseding transfer/handoff/fence/takeover/terminal authority.

Each loser fails before candidate authority mutation; presence/lease/GameSession/TransportBinding become authoritative together only for the winner.

## 13. Security/privacy

Never log raw grant/nonce/reusable credential/private key. AccountId/CharacterId do not become ordinary metric labels. Diagnostic templates are stable redacted text and correlation fields avoid credentials/private fencing/security-generation values.

Before successful authentication, diagnostics must not expose whether token semantic issuer/audience/profile/purpose/typ or exact payload schema would have matched. Safe algorithm/key/trust-stage classes may be recorded only within the redaction policy and never include credentials or token-supplied key material.

## 14. Downstream integration

FND-04B consumes accepted authority/session starting state for reconnect/recovery without weakening A. FND-04C integrates A/B errors, failure scenarios, rollout/evidence and thin final FND-04 index without silently changing accepted component semantics. DUR/OPS/PERF own physical persistence/atomicity and measured production values.

## 15. Acceptance boundary

FND-04A merge accepts only fresh-admission authority, strict profile, deterministic cryptographic/schema/binding precedence, independent revision bindings, ownership-safe current-world validation, authenticated source-age + anti-rollback security evidence semantics and complete A-error shape. It authorizes no runtime implementation and does not complete FND-04.

## 16. Concise rule

```text
Platform bounded grant
-> no gameplay authority

signed dimensions stay separate
-> protocol + transport + ruleset + content + map + world-policy + offer
-> schema_revision remains diagnostic FND-02 metadata, not exact admission gate

security evidence
-> authenticated source observation provenance
-> conservative source age <=5s; cache time never re-ages
-> monotonic/comparable source revision; newer accepted decision fences older
-> no rollback from newer deny/revoke to older allow/trust
-> bounded residual unseen-revocation window <= source-age ceiling

pre-signature trust
-> verifier-configured expected issuer/profile/key-purpose context only
-> token semantics never select/broaden trust
-> kid selects only within fixed trusted set

credential classification
-> malformed JWS/header shape => MALFORMED
-> well-formed non-exact alg => AUTHENTICATION_FAILED
-> fixed-scope kid/key/trust/signature failure => AUTHENTICATION_FAILED
-> authenticated exact-schema violation => MALFORMED
-> authenticated wrong typ/iss/aud/purpose => BINDING_MISMATCH
-> authenticated unsupported profile => REVISION_UNSUPPORTED

Oteryn-v2
-> ownership FIRST, current world SECOND
-> route/runtime/revisions + nonce + presence + lease

atomic final boundary repeats every mutable fact
-> valid ownership + stale world => ADMISSION_GRANT_WORLD_STALE
-> stale/rollback/unprovable evidence => ADMISSION_GRANT_SECURITY_EVIDENCE_STALE
-> all valid => one admission authority commit

reconnect/recovery
-> FND-04B
```