# FND-04 Pre-Admission Grant Security / Interchange Profile v1

- Status: Candidate normative profile owned by FND-04A; canonical when the owning FND-04A delivery merges
- Profile ID: `oteryn-pre-admission-v1`
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

Reject `alg=none`, deprecated polymorphic `EdDSA`, HMAC/RSA/ECDSA fallback, Ed448, incompatible key type/curve and any non-exact algorithm selection. Changing algorithm/container requires a new reviewed profile revision.

## 3. Protected JOSE header

Exact header:

```json
{
  "alg": "Ed25519",
  "kid": "<trusted-key-id>",
  "typ": "oteryn-admission+jwt"
}
```

- `alg` exactly `Ed25519`;
- `typ` exactly `oteryn-admission+jwt`;
- `kid` 1..64 ASCII `[A-Za-z0-9._-]+`, looked up only in trusted admission-key set;
- no other protected member.

Reject `jku`, `x5u`, `x5c`, embedded `jwk`, `crit`, `cty`, `zip`, `b64=false` and token-controlled key discovery.

## 4. Canonical issuer and audience

```text
iss = urn:oteryn:platform:game-admission
aud = urn:oteryn:game:admission
```

Both exact/case-sensitive. Signing-key purpose is dedicated to `oteryn-pre-admission-v1` and not inherited from OAuth, Game Login Ticket, recovery or service-auth trust.

## 5. Required claims

Payload is a JSON object containing exactly these claims; unknown claims reject in v1.

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

All UUIDs parse/round-trip exact canonical lowercase hyphenated form; nil rejects. `attempt_ref`, `character_id`, `world_id`, `channel_id` additionally require UUIDv7 + RFC variant. `account_id` remains Platform-owned and is not silently redefined as Oteryn UUIDv7.

Generation values are strings to avoid uint64 precision loss >2^53. `attempt_ref` is producer operation/correlation identity; `jti` is game consume identity; neither is GameSessionId.

## 6. Size/parser limits

Before signature verification enforce: token <=4096 ASCII bytes; exactly 3 JWS segments; decoded header <=512; payload <=3072; nesting <=2; duplicate JSON members reject; invalid UTF-8 reject; malformed/noncanonical/padded base64url reject; fractional/exponent NumericDate reject; missing/null required claim reject; decompression unsupported. Stricter FND-02 outer bound wins.

## 7. Time policy

```text
maximum lifetime: 30s from iat to exp
maximum verifier skew: 5s
```

At trusted server time `now`:

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

Authoritative consume state keyed by at least `(trusted issuer, profile, jti)` guarantees one successful admission maximum, one linearized winner under concurrent use, no reuse after lost response and no authority creation/revival/fencing by losing replay.

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

Steps 1–15 are fail-fast eligibility only:

1. FND-02 material bound;
2. parser/size bounds;
3. exact protected header/profile;
4. authenticated admission key/profile trust/revocation evidence: source provenance, source-age upper bound <=5s and anti-rollback revision/fence; then trusted `kid` lookup;
5. Ed25519 signature;
6. exact `iss`, `aud`, `typ`, `purpose`; unsupported `profile` is a revision failure;
7. time/lifetime/skew;
8. exact claim schema/canonical UUID/revision encoding;
9. Platform-security evidence: source provenance, source-age upper bound <=5s, anti-rollback revision/fence and current account generation/state;
10. route/runtime/current target/ownership + protocol/transport + ruleset/content/map/world-policy/offer revisions independently;
11. GrantNonce eligibility;
12. current AccountId->CharacterId ownership/lifecycle;
13. current CharacterId->WorldId/world eligibility only after step 12;
14. AccountPresence/duplicate-login eligibility;
15. CharacterLease/current runtime-scope acquisition/readiness;
16. one atomic final boundary revalidates every mutable predicate and only then commits complete FND-04A admission authority;
17. publish success only after commit.

### 12.1 Wrong-bound credential classification

A syntactically valid and correctly signed credential whose exact `iss`, `aud`, `typ` or `purpose` is wrong returns `ADMISSION_GRANT_BINDING_MISMATCH` (`SESSION_REJECTED`, `SECURITY_TERMINAL`) and is never reinterpreted as the required fresh-entry credential. Unsupported `profile` returns `ADMISSION_GRANT_REVISION_UNSUPPORTED`; malformed/missing/noncanonical structure returns `ADMISSION_GRANT_MALFORMED`; cryptographic/key trust failure returns `ADMISSION_GRANT_AUTHENTICATION_FAILED`.

### 12.2 Final atomic revalidation

Immediately before/atomically with authority creation revalidate:

- JWT time/lifetime/skew;
- exact key/profile trust using authenticated source provenance, source-age upper bound <=5s and non-rollback source revision/fence;
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

The authenticated trust/revocation evidence transport/schema is deferred, but it MUST provide enough semantics to establish:

```text
trusted source authority
issuer/profile/key-purpose trust scope
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

For a comparable issuer/profile/key-purpose trust scope, an evidence revision/fence lower than the highest accepted comparable revision cannot authorize even if its source age is <=5s. Equal revision with contradictory authenticated content is invalid.

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

### Profile/crypto/binding

- canonical Ed25519 positive;
- `none`, deprecated `EdDSA`, wrong algorithm/key type/curve;
- token-directed key discovery;
- malformed/duplicate/unknown claims, UUIDv7/variant/canonical failures, size limits;
- wrong exact `iss`, `aud`, `typ`, `purpose` -> `ADMISSION_GRANT_BINDING_MISMATCH`;
- unsupported `profile` -> `ADMISSION_GRANT_REVISION_UNSUPPORTED`;
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
