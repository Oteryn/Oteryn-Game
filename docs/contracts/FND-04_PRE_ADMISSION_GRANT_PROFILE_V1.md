# FND-04 Pre-Admission Grant Security / Interchange Profile v1

- Status: Candidate normative profile owned by bounded successor FND-04A-R1; canonical when the successor delivery from Issue #120 merges
- Profile ID: `oteryn-pre-admission-v1`
- Successor repair provenance: reconstructs reviewed PR #114 exact head `79678485d009c22ece2736c822d6b75b6d235ad2` and completes deterministic protected-header, verifier-anchored trust-scope and cryptographic/schema error precedence
- Applies to: fresh native Oteryn-v2 gameplay entry authorization produced by Oteryn Platform and consumed by Oteryn-v2 final game admission
- Does not apply to: OAuth tokens, web sessions, Game Login Tickets, reconnect/recovery credentials, handoff credentials, Canary compatibility admission or already-admitted GameSession control
- Cryptographic container: JWS Compact Serialization carrying a JWT claims set
- Signature profile: fully specified JOSE `alg = Ed25519`
- Standards baseline: RFC 7515, RFC 7519, RFC 8032, RFC 8037, RFC 8725 and RFC 9864
- Normative authority companion: `docs/architecture/FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md`
- Does not select: PHP/Rust JWT library, KMS/HSM/vendor, key-distribution transport, persistence/cache schema or production deployment

## 1. Purpose

```text
Platform signs one bounded fresh-entry capability.
Oteryn-v2 verifies capability + current authoritative game facts.
Oteryn-v2 consumes the grant at most once.
Oteryn-v2 creates canonical GameSessionId only after final admission succeeds.
```

A valid signature is necessary but never sufficient. Signed world/revision values are authorization bindings to current state, not proof that state remains current.

## 2. Exact cryptographic profile

v1 uses JWS Compact Serialization, JWT Claims Set and fully specified JOSE `alg=Ed25519` / RFC 8032 Ed25519.

There is no algorithm negotiation or downgrade path.

- missing, null, non-string, empty, non-ASCII or structurally out-of-bound `alg` is malformed input -> `ADMISSION_GRANT_MALFORMED`;
- a structurally valid algorithm identifier whose value is not exact `Ed25519` — including `none`, deprecated polymorphic `EdDSA`, HMAC/RSA/ECDSA, Ed448 or any other value — is a cryptographic authentication-policy failure -> `ADMISSION_GRANT_AUTHENTICATION_FAILED`;
- an incompatible key type/curve or signature verification failure is also `ADMISSION_GRANT_AUTHENTICATION_FAILED`;
- changing algorithm/container requires a new reviewed profile revision and cannot be negotiated by the token.

## 3. Protected JOSE header

Canonical successful header:

```json
{
  "alg": "Ed25519",
  "kid": "<trusted-key-id>",
  "typ": "oteryn-admission+jwt"
}
```

Pre-signature structural/security processing has deterministic precedence:

- protected members must be exactly `alg`, `kid`, `typ`; missing members, duplicate members, unknown/extra members or forbidden protected members are malformed -> `ADMISSION_GRANT_MALFORMED`;
- `alg` must first be a syntactically valid bounded ASCII string; malformed representation -> `ADMISSION_GRANT_MALFORMED`; a well-formed value other than exact `Ed25519` -> `ADMISSION_GRANT_AUTHENTICATION_FAILED` without key lookup or fallback;
- `kid` must be a string of 1..64 ASCII characters matching `[A-Za-z0-9._-]+`; malformed representation -> `ADMISSION_GRANT_MALFORMED`; a well-formed `kid` not present/trusted in the fixed verifier trust set -> `ADMISSION_GRANT_AUTHENTICATION_FAILED`;
- `typ` must be present as a non-empty JSON string of at most 64 visible ASCII bytes with no control or whitespace characters; malformed/missing/null/non-string/out-of-bound `typ` -> `ADMISSION_GRANT_MALFORMED`;
- `jku`, `x5u`, `x5c`, embedded `jwk`, `crit`, `cty`, `zip`, `b64=false` and token-controlled key discovery are forbidden protected-header shapes -> `ADMISSION_GRANT_MALFORMED`.

The **semantic exact value** of a structurally valid `typ` is deliberately not classified before cryptographic authentication. Only after fixed-scope trusted key lookup and successful Ed25519 signature verification is `typ` compared with exact `oteryn-admission+jwt`. A correctly signed but semantically different `typ` maps to `ADMISSION_GRANT_BINDING_MISMATCH`. A token that fails algorithm policy, key trust or signature verification must not disclose whether its otherwise well-formed `typ` would have matched.

The pre-signature trust context is **not token-selectable**. The verifier fixes the expected fresh-entry context from endpoint/configuration before reading semantic payload values:

```text
expected issuer = urn:oteryn:platform:game-admission
expected audience = urn:oteryn:game:admission
expected profile = oteryn-pre-admission-v1
expected purpose = fresh_entry
expected key purpose = admission signing for that context
```

Unauthenticated `iss`, `aud`, `profile`, `purpose`, `typ` or any other payload/header text MUST NOT choose, broaden or retarget the trusted issuer/profile/key-purpose scope. A syntactically valid `kid` may select only a candidate key within the already fixed trusted set. Semantic payload/header bindings are evaluated only after signature success.

## 4. Canonical issuer and audience

```text
iss = urn:oteryn:platform:game-admission
aud = urn:oteryn:game:admission
```

Both exact/case-sensitive. Signing-key purpose is dedicated to `oteryn-pre-admission-v1` and not inherited from OAuth, Game Login Ticket, recovery or service-auth trust. Before signature verification these are verifier-configured expected values for selecting the fixed trust context, not trusted claims from the token itself.

## 5. Required claims

A successfully authenticated v1 payload is a JSON object containing exactly these claims. Exact required-claim membership and unknown-claim rejection are semantic schema checks performed **after successful signature verification**. Pre-signature parsing may establish only the bounded syntactic JSON representation needed to authenticate safely; a well-formed unknown/missing semantic claim does not preempt an authentication failure.

### 5.1 Standard

| Claim | Type | Rule |
|---|---|---|
| `iss` | string | exact Section 4 issuer |
| `aud` | string | exact single audience; arrays rejected |
| `iat` | integer | whole-second NumericDate |
| `nbf` | integer | whole-second; `iat - 1 <= nbf <= iat + 1` |
| `exp` | integer | `exp > iat`; `exp - iat <=30s` |
| `jti` | string | 32 random bytes base64url-no-padding; exactly 43 chars |

### 5.2 Oteryn

| Claim | Type | Rule |
|---|---|---|
| `profile` | string | exact `oteryn-pre-admission-v1` |
| `purpose` | string | exact `fresh_entry` |
| `attempt_ref` | string | canonical lowercase RFC UUIDv7 |
| `account_id` | string | canonical lowercase non-nil authoritative Platform UUID representation accepted by FND-ID-01 |
| `character_id` | string | canonical lowercase non-nil RFC UUIDv7 |
| `world_id` | string | canonical lowercase non-nil RFC UUIDv7 |
| `channel_id` | string | canonical lowercase non-nil RFC UUIDv7 |
| `account_security_generation` | string | decimal non-zero uint64 string |
| `route_revision` | string | ASCII 1..64 `[A-Za-z0-9._:-]+` |
| `runtime_observation_revision` | string | ASCII 1..64 `[A-Za-z0-9._:-]+` |
| `scope_ownership_generation` | string | decimal non-zero uint64 string |
| `protocol_major` | integer | exact `1` |
| `transport_profile` | integer | exact `1` |
| `ruleset_revision` | string | ASCII 1..64 `[A-Za-z0-9._:-]+` |
| `content_revision` | string | ASCII 1..64 `[A-Za-z0-9._:-]+` |
| `map_revision` | string | ASCII 1..64 `[A-Za-z0-9._:-]+` |
| `world_policy_revision` | string | ASCII 1..64 `[A-Za-z0-9._:-]+` |
| `offer_revision` | string | ASCII 1..64 `[A-Za-z0-9._:-]+` |

`compatibility_revision` is deliberately absent. Protocol, transport, ruleset, content, map, world-policy and offer are independent authoritative dimensions and MUST NOT be overloaded into one opaque compatibility token.

FND-02 `schema_revision` is diagnostic/build evidence rather than exact-equality admission identity, so it is also deliberately absent from the signed FND-04A v1 gate.

After successful signature verification, exact schema validation rejects missing/null/wrong-type/unknown claims, noncanonical UUID/revision/generation encodings and invalid NumericDate representation as `ADMISSION_GRANT_MALFORMED`, except an otherwise structurally valid unsupported semantic `profile`, which has the explicit later outcome `ADMISSION_GRANT_REVISION_UNSUPPORTED` after the exact claim's string shape is established.

All UUIDs parse/round-trip exact canonical lowercase hyphenated form; nil rejects. `attempt_ref`, `character_id`, `world_id`, `channel_id` additionally require UUIDv7 + RFC variant. `account_id` remains Platform-owned and is not silently redefined as Oteryn UUIDv7.

Generation values are strings to avoid uint64 precision loss >2^53. `attempt_ref` is producer operation/correlation identity; `jti` is game consume identity; neither is GameSessionId.

## 6. Size/parser limits

Before signature verification enforce only bounded safe parsing/authentication prerequisites:

- token <=4096 ASCII bytes;
- exactly 3 JWS segments;
- decoded header <=512 bytes;
- decoded payload <=3072 bytes;
- payload/header decode as bounded JSON objects with nesting <=2;
- duplicate JSON members reject;
- invalid UTF-8 reject;
- malformed/noncanonical/padded base64url reject;
- decompression unsupported;
- protected-header structural/security rules from Sections 2–3.

Failures above that make the token structurally unparsable are `ADMISSION_GRANT_MALFORMED`; the explicit well-formed non-exact `alg` case remains `ADMISSION_GRANT_AUTHENTICATION_FAILED` as defined in Section 2.

Pre-signature parsing MUST NOT enforce semantic exact payload claim membership, unknown-claim rejection, UUID/revision values, issuer/audience/profile/purpose equality or time validity. Those are evaluated only after successful signature verification. Therefore:

- invalid signature + otherwise well-formed payload with a missing/unknown semantic claim -> `ADMISSION_GRANT_AUTHENTICATION_FAILED`;
- successful signature + exact-schema missing/unknown/wrong-type/noncanonical claim -> `ADMISSION_GRANT_MALFORMED`;
- successful signature + structurally valid unsupported `profile` -> `ADMISSION_GRANT_REVISION_UNSUPPORTED`;
- successful signature + wrong exact `iss`/`aud`/`purpose` or well-formed wrong `typ` -> `ADMISSION_GRANT_BINDING_MISMATCH`.

Stricter FND-02 outer material bound wins before all token work.

## 7. Time policy

```text
maximum lifetime: 30s from iat to exp
maximum verifier skew: 5s
```

After successful signature and exact claim-schema validation, at trusted server time `now`:

```text
now + 5s >= nbf
now - 5s < exp
exp > iat
exp - iat <=30s
abs(iat - now) <=35s
```

Client clocks never affect validity.

## 8. GrantNonce

`jti` is 32 cryptographically random producer bytes encoded base64url without padding.

Authoritative consume state keyed by at least `(trusted issuer, profile, jti)` guarantees one successful admission maximum, one linearized winner under concurrent use, no reuse after lost response and no authority creation/revival/fencing by losing replay. Here `trusted issuer` and `profile` mean the verifier-accepted authenticated semantic values, never pre-signature token-selected trust coordinates.

Replay evidence remains authoritative through at least `exp + 5s` and longer if DUR requires.

## 9. AdmissionAttemptRef producer idempotency

One logical issuance uses one `attempt_ref`. Lost response/crash does not permit a blind second independently usable capability.

Unknown exact issuance outcome -> `ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED`: `DEPENDENCY_UNAVAILABLE` + bounded `RETRYABLE`, public `TEMPORARILY_UNAVAILABLE`, same-ref status/reconciliation only. A new independent attempt requires deterministic retirement plus proof any possibly issued capability is no longer acceptable. Producer ambiguity creates no gameplay authority.

## 10. Required Platform-security evidence semantics

Grant binds `account_id` + `account_security_generation`. Final game admission consumes authenticated Platform-security evidence proving account enabled/revoked state, accepted generation floor and evidence freshness.

The evidence transport/schema is deferred, but the semantic envelope MUST provide enough authenticated information to establish:

```text
source authority + security purpose/scope
AccountId scope
source_observed_at (or equivalently strong authenticated source-time provenance)
monotonic/comparable source_revision (or equivalent non-rollback decision fence)
current account-security decision facts / accepted generation floor
```

### 10.1 Source-age freshness

The maximum accepted age is 5 seconds measured from authenticated source observation, not cache receipt:

```text
conservative upper_bound_source_age <= 5s
```

The upper bound includes known clock uncertainty. Local cache insertion, refresh, DB write, reserialization or re-read MUST NOT reset/reduce the age.

If source-time provenance is missing/future-ambiguous/contradictory or timing uncertainty prevents proving the upper-bound source age <=5s, fail as `ADMISSION_GRANT_SECURITY_EVIDENCE_STALE`.

### 10.2 Platform-security anti-rollback

For the relevant AccountId/security purpose, a source revision/fence older than the highest accepted comparable revision already established for that scope cannot authorize, even while its source age remains <=5s. Equal revisions with contradictory authenticated decision content are invalid.

After a newer revision disables/revokes the account or raises its minimum accepted generation, an older allow revision can never restore authorization. On recovery/restart, admission requires reconstruction of a current non-rollback floor from authoritative evidence or preserved trusted state; inability to prove that floor fails closed.

Reject current accepted evidence when:

```text
account disabled/revoked
OR grant.account_security_generation < minimum_valid_generation
```

Current explicit account/security denial maps to `ADMISSION_GRANT_SECURITY_STATE_REVOKED`; stale/superseded/unprovable evidence maps to `ADMISSION_GRANT_SECURITY_EVIDENCE_STALE`.

Signature validity/exp never overrides newer Platform-security state. Platform gains no post-admission GameSession authority.

## 11. Route/runtime, independent revisions and ownership-safe character-world applicability

Grant binds independently:

```text
world_id
channel_id
route_revision
runtime_observation_revision
scope_ownership_generation
protocol_major
transport_profile
ruleset_revision
content_revision
map_revision
world_policy_revision
offer_revision
```

Each dimension is compared with current authoritative target state separately. A change to any one invalidates an older grant even when all others remain unchanged.

Default runtime rule:

```text
current scope ownership generation != token.scope_ownership_generation
-> stale runtime grant
```

Character-world state is checked only after current `AccountId -> CharacterId` ownership/lifecycle is proven:

```text
current_character_world_id == token.world_id
AND current lifecycle permits fresh admission to token.world_id
```

Global CharacterId may survive legal world transfer; route validity plus CharacterId alone is insufficient.

Valid ownership + current world mismatch/change before commit -> `ADMISSION_GRANT_WORLD_STALE`, no candidate nonce/presence/lease/session/transport mutation, no retarget, require current world resolution + newly authorized route/grant.

Reject non-open target, stale route/runtime observation, changed scope ownership, unsupported protocol/transport, mismatched ruleset/content/map/world-policy/offer revision. No silent retarget/downgrade. NodeId never substitutes for scope ownership generation.

## 12. Verification/admission order and final linearization

Steps are fail-fast eligibility only until the atomic authority boundary:

1. FND-02 outer material bound;
2. JWS/parser/size/base64/JSON structural bounds;
3. protected-header shape: required exact member set and bounded syntactic `alg`/`kid`/`typ`; malformed shape -> `ADMISSION_GRANT_MALFORMED`;
4. cryptographic algorithm policy: syntactically valid non-exact `alg` -> `ADMISSION_GRANT_AUTHENTICATION_FAILED`; exact `Ed25519` continues;
5. using verifier-configured expected fresh-entry v1 issuer/profile/key-purpose scope only, obtain authenticated admission trust/revocation evidence with source provenance, source-age upper bound <=5s and anti-rollback revision/fence; resolve well-formed `kid` only inside that fixed trusted set; unknown/untrusted `kid` -> `ADMISSION_GRANT_AUTHENTICATION_FAILED`; unauthenticated token semantics cannot select or broaden this scope;
6. Ed25519 signature; failure -> `ADMISSION_GRANT_AUTHENTICATION_FAILED`;
7. after successful signature, exact payload claim membership/types/canonical encodings; malformed/missing/unknown/noncanonical schema -> `ADMISSION_GRANT_MALFORMED`;
8. after schema success, exact semantic `iss`, `aud`, `typ`, `purpose`; wrong value -> `ADMISSION_GRANT_BINDING_MISMATCH`; structurally valid unsupported `profile` -> `ADMISSION_GRANT_REVISION_UNSUPPORTED`;
9. time/lifetime/skew;
10. Platform-security evidence: source provenance, source-age upper bound <=5s, anti-rollback revision/fence and current account generation/state;
11. route/runtime/current target/ownership + protocol/transport + ruleset/content/map/world-policy/offer revisions independently;
12. GrantNonce eligibility;
13. current AccountId->CharacterId ownership/lifecycle;
14. current CharacterId->WorldId/world eligibility only after step 13;
15. AccountPresence/duplicate-login eligibility;
16. CharacterLease/current runtime-scope acquisition/readiness;
17. one atomic final boundary revalidates every mutable predicate and only then commits complete FND-04A admission authority;
18. publish success only after commit.

### 12.1 Complete classification precedence

The following precedence is normative and exhaustive for the v1 credential-validation surface:

1. outer/material/parser/JWS/JSON/base64 failures, malformed/missing/non-string/out-of-bound protected-header members, and forbidden/extra protected members -> `ADMISSION_GRANT_MALFORMED`;
2. syntactically valid but non-exact `alg` -> `ADMISSION_GRANT_AUTHENTICATION_FAILED`; there is no algorithm negotiation, downgrade or revision fallback;
3. exact algorithm but well-formed `kid` absent/untrusted in the verifier-configured expected trust set, trust decision denial, incompatible key, or Ed25519 signature failure -> `ADMISSION_GRANT_AUTHENTICATION_FAILED`;
4. only after successful signature, exact payload schema membership/types/canonical encodings are authoritative for classification; missing/unknown/wrong-type/noncanonical claims -> `ADMISSION_GRANT_MALFORMED`;
5. after successful signature and schema validation, wrong exact `iss`, `aud`, `typ` or `purpose` -> `ADMISSION_GRANT_BINDING_MISMATCH` (`SESSION_REJECTED`, `SECURITY_TERMINAL`);
6. after successful signature and schema validation, structurally valid unsupported semantic `profile` -> `ADMISSION_GRANT_REVISION_UNSUPPORTED`;
7. only then evaluate time policy and the mutable game-domain predicates.

Consequences:

- invalid signature + well-formed wrong `typ`/binding values -> `ADMISSION_GRANT_AUTHENTICATION_FAILED`;
- invalid signature + well-formed unsupported profile -> `ADMISSION_GRANT_AUTHENTICATION_FAILED`;
- invalid signature + otherwise well-formed missing/unknown semantic payload claim -> `ADMISSION_GRANT_AUTHENTICATION_FAILED`;
- successfully signed unknown/missing/wrong-type/noncanonical payload claim -> `ADMISSION_GRANT_MALFORMED`;
- successfully signed wrong binding -> `ADMISSION_GRANT_BINDING_MISMATCH`;
- successfully signed unsupported profile -> `ADMISSION_GRANT_REVISION_UNSUPPORTED`.

This prevents algorithm, key, binding, profile and exact-schema oracles before authentication and guarantees deterministic producer/consumer classification.

### 12.2 Final atomic revalidation

Immediately before/atomically with authority creation revalidate:

- JWT time/lifetime/skew;
- exact key/profile trust for the verifier-configured expected fresh-entry context using authenticated source provenance, source-age upper bound <=5s and non-rollback source revision/fence;
- Platform-security using authenticated source provenance, source-age upper bound <=5s, non-rollback source revision/fence + account state/generation;
- route/runtime observation, target lifecycle, scope ownership, runtime owner/placement/readiness;
- protocol_major and transport_profile;
- each ruleset/content/map/world-policy/offer revision independently;
- AccountId->CharacterId ownership/lifecycle first;
- CharacterId->WorldId/world eligibility second;
- GrantNonce;
- AccountPresence/incumbent state;
- CharacterLease/fence state;
- no newer world-transfer/handoff/fence/takeover/terminal authority.

Only then atomically:

```text
consume GrantNonce
+ establish/advance AccountPresenceClaim as required
+ establish/acquire CharacterLease as required
+ create canonical GameSessionId
+ GameSession ACTIVE
+ connection_generation = 1
+ establish initial authoritative session/reconciliation boundary
```

FND-04A defines no reconnect secret/proof state.

Any failed final check leaves actual current authority unchanged. Ownership failure precedes world classification; owned-character stale world uses `ADMISSION_GRANT_WORLD_STALE`.

## 13. Signing-key/profile trust evidence: provenance, freshness and anti-rollback

Verification uses trusted Ed25519 public keys only. Dedicated admission key purpose, trusted configured set, bounded current/retiring overlap. Token-controlled key fetch forbidden. Private signing key never leaves Platform signing/KMS boundary.

For pre-signature verification, the trust scope is selected exclusively from the verifier-configured expected fresh-entry v1 context. The unverified token does not choose issuer/profile/key-purpose trust. `kid` is only an index into that fixed set; it is never authority to fetch or switch to another trust domain. After signature success, the authenticated token's `iss`, `profile`, `purpose` and related semantic bindings are compared to the expected context and rejected by their contract-specific outcomes when mismatched/unsupported.

The authenticated trust/revocation evidence transport/schema is deferred, but it MUST provide enough semantics to establish:

```text
trusted source authority
verifier-configured expected issuer/profile/key-purpose trust scope
source_observed_at (or equivalent authenticated source-time provenance)
monotonic/comparable source_revision (or equivalent non-rollback decision fence)
current key/profile trust/revocation decision
```

### 13.1 Source-age freshness

```text
conservative upper_bound_source_age <= 5s
```

Age is from authenticated source observation and includes known clock uncertainty. Local cache receive/refresh/storage time never re-ages evidence.

Missing/ambiguous/contradictory provenance or inability to prove the upper bound <=5s -> `ADMISSION_GRANT_SECURITY_EVIDENCE_STALE`.

### 13.2 Trust anti-rollback

For a comparable verifier-configured expected issuer/profile/key-purpose trust scope, an evidence revision/fence lower than the highest accepted comparable revision cannot authorize even if its source age is <=5s. Equal revision with contradictory authenticated content is invalid.

Once a newer accepted trust revision revokes/untrusts a key/profile, an older trusted revision cannot restore it. On recovery/restart, consumer must reconstruct a current non-rollback floor from authoritative evidence or preserved trusted state before fresh admission; inability to prove the floor fails closed.

- current accepted explicit unknown/revoked/not-trusted decision -> `ADMISSION_GRANT_AUTHENTICATION_FAILED`;
- stale/superseded/unprovable evidence -> `ADMISSION_GRANT_SECURITY_EVIDENCE_STALE`.

### 13.3 Residual revocation window

The <=5s model is bounded-staleness, not instantaneous global revocation.

If revocation occurs after the observation point of the latest accepted evidence and before a newer source revision is observable, the verifier cannot infer that unseen event. The prior evidence may remain usable only while its conservative source age stays <=5s **and** no newer comparable revision has been accepted.

Authorization ends at the first of:

- newer accepted evidence recording the restriction; or
- inability to prove the existing evidence source-age upper bound remains <=5s.

Cache refresh cannot extend this window; an older revision cannot roll back a newer revoke. A future zero-window design requires a separately reviewed cross-repository atomic epoch/fence.

## 14. Compatibility/downgrade

Version dimensions remain separate: profile, producer/consumer contract, protocol major, transport profile, ruleset, content, map, world policy and offer. FND-02 `schema_revision` remains diagnostic/build metadata rather than exact admission equality. Unknown mandatory revision rejects; no profile/algorithm/Canary downgrade.

FND-04C later integrates rollout compatibility matrix; it cannot collapse accepted independent dimensions into one opaque revision.

## 15. Logging/privacy

Never log/export raw JWT, GrantNonce, private key, OAuth/Game Login Ticket or future reconnect material.

Authorized diagnostics may include attempt_ref, safe kid/profile, WorldId/ChannelId where policy permits, route/runtime revision and typed outcome. Never export Platform security-generation values, raw source evidence fences when sensitive, raw scope-ownership fence generation or transfer details; use safe source-age/relation/order classes. AccountId/CharacterId remain privacy-controlled, not ordinary metric labels.

Complete FND-04A diagnostic rows live in the authority companion contract.

## 16. Independent implementation fixtures

### Profile/crypto/binding/schema precedence

- canonical Ed25519 positive;
- missing/null/non-string/out-of-bound `alg` -> `ADMISSION_GRANT_MALFORMED`;
- syntactically valid `none`, deprecated `EdDSA`, HMAC/RSA/ECDSA, Ed448 or other non-exact algorithm -> `ADMISSION_GRANT_AUTHENTICATION_FAILED`, with no fallback;
- malformed `kid` -> `ADMISSION_GRANT_MALFORMED`;
- well-formed unknown/untrusted `kid` in fixed trust scope -> `ADMISSION_GRANT_AUTHENTICATION_FAILED`;
- forbidden/extra protected member, embedded key or token-directed key discovery -> `ADMISSION_GRANT_MALFORMED` and never changes trust selection;
- malformed/missing/non-string/out-of-bound `typ` -> `ADMISSION_GRANT_MALFORMED`;
- structurally valid wrong `typ` + invalid/untrusted signature -> `ADMISSION_GRANT_AUTHENTICATION_FAILED` without binding-oracle detail;
- structurally valid unsupported/wrong `profile` + invalid/untrusted signature -> `ADMISSION_GRANT_AUTHENTICATION_FAILED`; untrusted profile cannot retarget trust or force a revision verdict;
- invalid signature + otherwise well-formed unknown/missing semantic payload claim -> `ADMISSION_GRANT_AUTHENTICATION_FAILED`;
- correctly signed unknown/missing/wrong-type/noncanonical payload claim -> `ADMISSION_GRANT_MALFORMED`;
- correctly signed, structurally valid wrong exact `iss`, `aud`, `typ` or `purpose` -> `ADMISSION_GRANT_BINDING_MISMATCH`;
- correctly signed unsupported `profile` -> `ADMISSION_GRANT_REVISION_UNSUPPORTED`;
- same `kid` under a token-supplied different issuer/profile/purpose cannot escape the verifier-configured expected trust set;
- nbf/expiry/lifetime/skew boundaries;
- replay/concurrent consume;
- ambiguous issuance reconciliation.

### Independent authoritative revisions

For each `ruleset_revision`, `content_revision`, `map_revision`, `world_policy_revision`, `offer_revision`, mutate only that current dimension after issuance while keeping all others unchanged. Final admission must reject; no opaque compatibility token may hide the mismatch.

### Security evidence provenance/rollback

Apply independently to Platform-security evidence and signing-key/profile trust evidence:

- authenticated source observation provenance + comparable source revision/fence required;
- local cache refresh/reinsert/re-read does not reset source age;
- source-age upper bound including clock uncertainty must be provably <=5s;
- older still-<5s allow/trust revision arriving after a newer accepted revision is rejected as superseded;
- newer Platform-security deny/generation floor cannot be rolled back by older allow;
- newer trust revoke cannot be rolled back by older trusted evidence;
- equal revision with contradictory authenticated content fails closed;
- restart/recovery without provable current non-rollback floor cannot authorize until floor/current evidence is reconstructed;
- latest accepted evidence already records revoke -> purpose-specific terminal denial/no mutation;
- revocation after latest evidence observation point -> do not assert instant detection; prove authority ends at first newer restrictive evidence or source-age proof >5s, whichever comes first.

### Ownership/world

- non-owned CharacterId -> account/character conflict before any world classification;
- valid ownership + initial world mismatch -> `ADMISSION_GRANT_WORLD_STALE`;
- valid ownership/world then legal transfer/world change before final commit -> world stale;
- stale grant never retargeted;
- concurrent transfer/admission has one authoritative outcome.

### Change-before-commit matrix

Independently mutate JWT time; key/profile evidence source age/order/decision; Platform-security evidence source age/order/account state; route/runtime/target/ownership; protocol/transport; each independent gameplay revision; AccountId->CharacterId; CharacterId->WorldId/world eligibility; GrantNonce; AccountPresence/incumbent; CharacterLease/fence; or newer transfer/handoff/fence/takeover/terminal authority.

Every loser fails before candidate authority mutation and preserves actual current authority. Fixtures must be independently produced/validated enough to avoid shared producer/consumer bugs.

## 17. Error integration

FND-04A authority contract fully defines its symbolic outcomes with Foundation category, disposition, retry authority, mutation outcome, public class, redacted diagnostic and credential-free correlation fields. FND-04C may integrate, not weaken.

## 18. Non-authorization

This profile implements nothing and authorizes no Platform/Rust verifier, consume store, security projection, persistence schema, library, KMS/HSM, production key, routing or traffic. Overall FND-04 remains incomplete until FND-04B/FND-04C/closeout.