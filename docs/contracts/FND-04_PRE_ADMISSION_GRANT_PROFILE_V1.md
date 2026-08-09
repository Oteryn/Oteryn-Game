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

`compatibility_revision` is deliberately absent from v1. Protocol, transport, ruleset, content, map, world-policy and offer are independent authoritative dimensions and MUST NOT be overloaded into one opaque compatibility token.

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

## 10. Platform account-security freshness

Grant binds `account_id` + `account_security_generation`. Final game admission consumes authenticated Platform-security evidence proving account enabled/revoked state, accepted generation floor and evidence freshness.

```text
maximum accepted Platform-security evidence age: 5s
```

Older/unavailable/unauthenticated/contradictory/unprovable evidence fails closed. Reject disabled/revoked account or grant generation below current minimum. Signature validity/exp never overrides newer account-security invalidation. Platform does not gain post-admission GameSession authority.

## 11. Route/runtime, independent revisions and character-world applicability

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
4. authenticated admission key/profile trust/revocation evidence age <=5s, then trusted `kid` lookup;
5. Ed25519 signature;
6. exact `iss`, `aud`, `typ`, `purpose`; unsupported `profile` is a revision failure;
7. time/lifetime/skew;
8. exact claim schema/canonical UUID/revision encoding;
9. current Platform-security projection;
10. route/runtime/current target/ownership + protocol/transport + ruleset/content/map/world-policy/offer revisions independently;
11. GrantNonce eligibility;
12. current AccountId->CharacterId ownership/lifecycle;
13. current CharacterId->WorldId/world eligibility only after step 12;
14. AccountPresence/duplicate-login eligibility;
15. CharacterLease/current runtime-scope acquisition/readiness;
16. one atomic final boundary revalidates every mutable predicate and only then commits complete admission authority;
17. publish success only after commit.

### 12.1 Wrong-bound credential classification

A syntactically valid and correctly signed credential whose exact `iss`, `aud`, `typ` or `purpose` is wrong returns `ADMISSION_GRANT_BINDING_MISMATCH` (`SESSION_REJECTED`, `SECURITY_TERMINAL`) and is never reinterpreted as the required fresh-entry credential. Unsupported `profile` returns `ADMISSION_GRANT_REVISION_UNSUPPORTED`; malformed/missing/noncanonical structure returns `ADMISSION_GRANT_MALFORMED`; cryptographic/key trust failure returns `ADMISSION_GRANT_AUTHENTICATION_FAILED`.

### 12.2 Final atomic revalidation

Immediately before/atomically with authority creation revalidate:

- JWT time/lifetime/skew;
- exact key/profile trust + authenticated evidence age <=5s;
- Platform-security evidence age <=5s + account state/generation;
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

## 13. Key distribution, rotation and bounded revocation detection

Verification uses trusted Ed25519 public keys only. Dedicated admission key purpose, trusted configured set, bounded current/retiring overlap. Token-controlled key fetch forbidden. Private signing key never leaves Platform signing/KMS boundary.

```text
maximum accepted authenticated key/profile trust/revocation evidence age: 5s
```

- evidence age >5s/unavailable/unauthenticated/contradictory/unprovable -> `ADMISSION_GRANT_SECURITY_EVIDENCE_STALE`;
- fresh accepted evidence explicitly marking exact key/profile unknown/revoked/not-trusted -> `ADMISSION_GRANT_AUTHENTICATION_FAILED`.

### 13.1 Residual revocation window

The <=5s model is bounded stale evidence, not atomic global revocation.

If revocation occurs after the observation point of evidence that is still authenticated and <=5s old, the verifier cannot know that unseen event. The old evidence may remain acceptable only until:

- newer authenticated evidence records the revocation, or
- its age exceeds 5s, at which point failure is mandatory without a fresh provable replacement.

Therefore FND-04A explicitly accepts at most the five-second residual detection window attributable to this projection. It does not require an impossible instantaneous-revocation fixture. Any zero-window design would require a separately reviewed cross-repository epoch/fence.

## 14. Compatibility/downgrade

Version dimensions remain separate: profile, producer/consumer contract, protocol major, transport profile, ruleset, content, map, world policy and offer. Unknown mandatory revision rejects; no profile/algorithm/Canary downgrade.

FND-04C later integrates rollout compatibility matrix; it cannot collapse accepted independent dimensions into one opaque revision.

## 15. Logging/privacy

Never log/export raw JWT, GrantNonce, private key, OAuth/Game Login Ticket or future reconnect material.

Authorized diagnostics may include attempt_ref, safe kid/profile, WorldId/ChannelId where policy permits, route/runtime revision and typed outcome. Never export Platform security-generation values, raw scope-ownership fence generation or transfer details; use match/stale/relation classes. AccountId/CharacterId remain privacy-controlled, not ordinary metric labels.

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

For each of `ruleset_revision`, `content_revision`, `map_revision`, `world_policy_revision`, `offer_revision`, mutate only that current dimension after issuance while keeping all others unchanged. Final admission must reject as revision unsupported/stale according to FND-04A and never accept because an opaque compatibility token happened not to change.

### Security/revocation timing

- trust evidence exactly 5s accepted if otherwise valid;
- >5s/unavailable/unauthenticated/contradictory -> stale evidence;
- final accepted evidence already contains revocation -> authentication failed/no mutation;
- revocation occurs after the evidence observation point while evidence remains <=5s -> do **not** assert instant detection; prove acceptance cannot extend beyond first newer revocation evidence or expiry of the 5s evidence window.

### Ownership/world

- non-owned CharacterId -> account/character conflict before any world classification;
- valid ownership + initial world mismatch -> `ADMISSION_GRANT_WORLD_STALE`;
- valid ownership/world then legal transfer/world change before final commit -> world stale;
- stale grant never retargeted;
- concurrent transfer/admission has one authoritative outcome.

### Change-before-commit matrix

Independently mutate JWT time, key/profile trust, Platform security, route/runtime/target/ownership, protocol/transport, each independent gameplay revision, AccountId->CharacterId, CharacterId->WorldId/world eligibility, GrantNonce, AccountPresence/incumbent, CharacterLease/fence or superseding transfer/handoff/fence/takeover/terminal authority. Every loser fails before candidate authority mutation and preserves actual current authority.

Fixtures must be independently produced/validated enough to avoid shared producer/consumer bugs.

## 17. Error integration

FND-04A authority contract fully defines its symbolic outcomes with Foundation category, disposition, retry authority, mutation outcome, public class, redacted diagnostic and credential-free correlation fields. FND-04C may integrate, not weaken.

## 18. Non-authorization

This profile implements nothing and authorizes no Platform/Rust verifier, consume store, security projection, persistence schema, library, KMS/HSM, production key, routing or traffic. Overall FND-04 remains incomplete until FND-04B/FND-04C/closeout.
