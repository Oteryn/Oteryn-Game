# FND-04 Pre-Admission Grant Security / Interchange Profile v1

- Status: Candidate normative profile owned by FND-04A; canonical when the owning FND-04A delivery merges
- Profile ID: `oteryn-pre-admission-v1`
- Applies to: fresh native Oteryn-v2 gameplay entry authorization produced by Oteryn Platform and consumed by Oteryn-v2 final game admission
- Does not apply to: OAuth tokens, web sessions, Game Login Tickets, reconnect credentials, reauthenticated recovery grants, Channel/Instance handoff credentials, Canary compatibility admission or already-admitted GameSession control
- Cryptographic container: JWS Compact Serialization carrying a JWT claims set
- Signature profile: fully specified JOSE `alg = Ed25519`
- Standards baseline: RFC 7515, RFC 7519, RFC 8032, RFC 8037, RFC 8725 and RFC 9864
- Normative authority companion: `docs/architecture/FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md`
- Does not select: PHP/Rust JWT library, KMS/HSM/vendor, key-distribution transport, persistence/cache schema or production deployment

## 1. Purpose

```text
Platform signs one bounded fresh-entry capability.
Oteryn-v2 verifies the capability and current authoritative game facts.
Oteryn-v2 consumes the grant at most once.
Oteryn-v2 creates canonical GameSessionId only after final admission succeeds.
```

A valid signature is necessary but never sufficient. A signed `world_id` is a binding to be checked against current authoritative character/world state after current account-character ownership is proven; it is not evidence that the character still belongs to or is eligible for that world.

## 2. Exact v1 cryptographic profile

```text
JWS Compact Serialization
JWT Claims Set payload
alg = Ed25519
Ed25519 parameter set from RFC 8032
```

Only fully specified JOSE `alg = Ed25519` is accepted. RFC 9864 deprecates the older polymorphic `EdDSA` identifier for new fully specified use.

Reject `none`, `EdDSA`, HMAC/RSA/ECDSA fallback, `Ed448`, incompatible key type/curve and any algorithm selection not equal to the exact allowlisted profile.

Changing algorithm/container requires a new reviewed profile revision.

## 3. Protected JOSE header

Exact v1 header:

```json
{
  "alg": "Ed25519",
  "kid": "<trusted-key-id>",
  "typ": "oteryn-admission+jwt"
}
```

Rules:

- `alg` exactly `Ed25519`;
- `typ` exactly `oteryn-admission+jwt`;
- `kid` is 1..64 ASCII `[A-Za-z0-9._-]+` and resolves only inside the trusted admission-key set;
- any protected-header member outside `alg`, `kid`, `typ` is rejected.

Explicitly reject `jku`, `x5u`, `x5c`, embedded `jwk`, `crit`, `cty`, `zip`, `b64=false` and token-controlled key discovery.

If trusted key distribution uses JWK, its public key representation follows the accepted OKP/Ed25519 representation; token `alg` remains `Ed25519`.

## 4. Canonical issuer and audience

```text
iss = urn:oteryn:platform:game-admission
aud = urn:oteryn:game:admission
```

Both are exact case-sensitive strings. The signing-key purpose is dedicated to `oteryn-pre-admission-v1` and is not inherited from OAuth, Game Login Ticket, recovery-grant or service-authentication trust.

## 5. Required claims

The JWT payload MUST contain exactly the required claims below. Unknown claims are rejected by v1.

### 5.1 Standard claims

| Claim | Type | Rule |
|---|---|---|
| `iss` | string | exact Section 4 issuer |
| `aud` | string | exact single Section 4 audience; arrays rejected |
| `iat` | integer JSON number | whole-second NumericDate |
| `nbf` | integer JSON number | whole-second NumericDate; `iat - 1 <= nbf <= iat + 1` |
| `exp` | integer JSON number | `exp > iat` and `exp - iat <= 30` seconds |
| `jti` | string | GrantNonce: 32 random bytes, base64url without padding; exactly 43 chars |

### 5.2 Oteryn claims

| Claim | Type | Rule |
|---|---|---|
| `profile` | string | exact `oteryn-pre-admission-v1` |
| `purpose` | string | exact `fresh_entry` |
| `attempt_ref` | string | Platform AdmissionAttemptRef; canonical lowercase RFC UUIDv7 |
| `account_id` | string | canonical lowercase non-nil UUID in authoritative Platform representation accepted by FND-ID-01 |
| `character_id` | string | canonical lowercase non-nil RFC UUIDv7 |
| `world_id` | string | canonical lowercase non-nil RFC UUIDv7 |
| `channel_id` | string | canonical lowercase non-nil RFC UUIDv7 |
| `account_security_generation` | string | decimal non-zero uint64 string |
| `route_revision` | string | ASCII 1..64 `[A-Za-z0-9._:-]+` |
| `runtime_observation_revision` | string | ASCII 1..64 `[A-Za-z0-9._:-]+` |
| `scope_ownership_generation` | string | decimal non-zero uint64 string |
| `protocol_major` | integer JSON number | exact `1` |
| `transport_profile` | integer JSON number | exact `1` |
| `compatibility_revision` | string | ASCII 1..64 `[A-Za-z0-9._:-]+` |

All UUIDs must parse and round-trip to exact lowercase hyphenated canonical text; nil UUID rejects. `attempt_ref`, `character_id`, `world_id`, `channel_id` additionally require UUID version 7 and RFC variant. `account_id` remains Platform-owned and is not silently redefined as Oteryn UUIDv7.

Generation values are strings to avoid uint64 precision loss above `2^53`.

`attempt_ref` is producer operation/correlation identity only. `jti` is the concrete game consume/replay identity. Neither is GameSessionId.

## 6. Size and parser limits

Before signature verification enforce:

- compact token <= 4096 ASCII bytes;
- exactly 3 JWS segments;
- decoded protected header <= 512 bytes;
- decoded payload <= 3072 bytes;
- JSON nesting depth <= 2;
- duplicate JSON members reject;
- invalid UTF-8 reject;
- malformed/noncanonical/padded base64url reject;
- floating/exponent/fractional NumericDate reject;
- missing/null required claim reject;
- decompression unsupported.

The stricter FND-02 outer admission-material bound also applies.

## 7. Time policy

```text
maximum grant lifetime: 30 seconds from iat to exp
maximum verifier clock-skew allowance: 5 seconds
```

At trusted server time `now` require:

```text
now + 5s >= nbf
now - 5s < exp
exp > iat
exp - iat <= 30s
abs(iat - now) <= 35s
```

Client clocks never affect validity.

## 8. GrantNonce and one-time game consumption

`jti` is 32 cryptographically random producer bytes encoded base64url without padding.

Authoritative consume state is keyed by at least `(trusted issuer, profile, jti)` and guarantees:

- one GrantNonce participates in at most one successful admission commit;
- concurrent use has at most one linearized winner;
- consumed nonce never becomes reusable after lost response;
- losing replay cannot create/revive/fence a different current session.

Consume/replay evidence remains authoritative at least through `exp + 5s` skew and longer when DUR/reconciliation requires.

## 9. AdmissionAttemptRef producer idempotency

One logical issuance attempt uses one `attempt_ref`. Retry/reconciliation uses the same ref; a lost response may not mint a blind second independently usable capability.

If exact prior issuance cannot be proven, use `ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED`: bounded `DEPENDENCY_UNAVAILABLE` / `RETRYABLE`, public `TEMPORARILY_UNAVAILABLE`, same-AdmissionAttemptRef status/reconciliation only. A new independent attempt requires deterministic retirement of the old attempt and proof any possibly issued capability is no longer acceptable.

Producer ambiguity creates no gameplay authority. `attempt_ref` may be used only as authorized redacted correlation, never authentication or game consume authority.

## 10. Platform account-security freshness

The grant binds `account_id` and `account_security_generation`.

Final game admission consumes authenticated Platform-security evidence proving current fresh-admission disabled/revoked state, accepted minimum/current generation and evidence freshness.

```text
maximum accepted Platform-security evidence age: 5 seconds
```

Older, unavailable, unauthenticated, contradictory or unprovable evidence fails closed.

Reject when:

```text
account disabled/revoked
OR grant.account_security_generation < minimum_valid_generation
```

Signature validity/expiry never overrides newer account-security invalidation. This admission veto does not give Platform post-admission GameSession authority.

## 11. Route/runtime and ownership-safe character-world applicability

The grant binds `world_id`, `channel_id`, `route_revision`, `runtime_observation_revision`, `scope_ownership_generation`, `protocol_major`, `transport_profile` and `compatibility_revision`.

Default runtime rule:

```text
current target scope ownership generation != token.scope_ownership_generation
-> stale grant
```

Default character-world rule is evaluated only after current `AccountId -> CharacterId` ownership/lifecycle is proven:

```text
current_character_world_id == token.world_id
AND current character lifecycle permits fresh admission to token.world_id
```

`CharacterId` is global and may survive a legal world transfer. Therefore route validity plus a global CharacterId is insufficient.

When current ownership is valid but character-world applicability differs or changes before commit:

- `ADMISSION_GRANT_WORLD_STALE`;
- no candidate GrantNonce consumption;
- no candidate AccountPresenceClaim/CharacterLease/GameSession/TransportBinding authority;
- no silent retarget to current/new world or Channel;
- current world must be resolved and a new route/grant authorized.

Also reject superseded route/runtime observation, changed scope owner, non-open target lifecycle and unsupported protocol/transport/compatibility.

NodeId never substitutes for scope ownership generation.

## 12. Verification/admission order and final linearization

Steps 1–15 are fail-fast **eligibility**, never authorization escrow.

1. outer FND-02 material bound;
2. compact/parser/size limits;
3. exact header/profile;
4. authenticated current admission key/profile trust/revocation evidence age `<=5s`, then trusted `kid` lookup;
5. Ed25519 signature;
6. exact `typ`, `iss`, `aud`, `profile`, `purpose`;
7. time/lifetime/skew;
8. claim schema/canonical encoding/UUID rules;
9. current Platform-security projection/revocation/generation;
10. route/runtime/current target/ownership generation + protocol/transport/compatibility;
11. GrantNonce consume eligibility/replay check;
12. authoritative current `AccountId -> CharacterId` ownership/lifecycle;
13. only after step 12 succeeds, authoritative `CharacterId -> WorldId` / world eligibility against signed `world_id`;
14. AccountPresenceClaim / duplicate-login eligibility;
15. CharacterLease acquisition eligibility + current runtime-scope readiness;
16. one atomic final boundary revalidates **every mutable authority predicate** and, only if all remain valid, consumes GrantNonce and establishes complete FND-04A admission authority;
17. publish success only after commit.

### 12.1 Mandatory final revalidation

Immediately before and atomically with authority creation revalidate:

- JWT time/skew/lifetime;
- exact key/profile trust + authenticated trust evidence age `<=5s`;
- authenticated Platform-security evidence age `<=5s`, account state/generation;
- route/runtime observation, target lifecycle, scope ownership, current runtime owner/placement/readiness;
- protocol/transport/compatibility;
- current `AccountId -> CharacterId` ownership/lifecycle **first**;
- current `CharacterId -> WorldId` / world eligibility **second**, only for that proven account-owned character;
- GrantNonce eligibility;
- AccountPresence/duplicate-login state;
- CharacterLease current/acquirable fence state;
- absence of newer world-transfer/handoff/fence/takeover/terminal authority superseding the candidate.

### 12.2 Atomic effects

Only after all revalidation succeeds:

```text
consume GrantNonce
+ establish/advance AccountPresenceClaim as required
+ establish/acquire CharacterLease as required
+ create canonical GameSessionId
+ GameSession ACTIVE
+ connection_generation = 1
+ establish initial authoritative session/reconciliation boundary
```

FND-04A intentionally defines no reconnect secret/proof state. FND-04B must later define reconnect/recovery without weakening this boundary.

No earlier presence/lease eligibility creates partial authority. Any failed final revalidation leaves the actual current world-transfer/presence/lease/runtime/session authority unchanged.

Specific examples:

- stale/unprovable key/profile or Platform-security evidence -> `ADMISSION_GRANT_SECURITY_EVIDENCE_STALE`;
- fresh explicit key/profile unknown/revoked/not-trusted -> `ADMISSION_GRANT_AUTHENTICATION_FAILED`;
- ownership conflict -> `ADMISSION_ACCOUNT_CHARACTER_CONFLICT`, without world-state classification for a non-owned candidate;
- current owned character world mismatch/change -> `ADMISSION_GRANT_WORLD_STALE`;
- changed route/runtime generation -> route/runtime stale outcomes;
- consumed nonce -> replay outcome.

## 13. Key distribution / rotation

Game-side verification uses trusted Ed25519 public keys only.

```text
maximum accepted authenticated signing-key/profile trust/revocation evidence age: 5 seconds
```

`age <=5s` is accepted; `>5s`, unavailable, unauthenticated, contradictory or unprovable state fails closed as `ADMISSION_GRANT_SECURITY_EVIDENCE_STALE`. Fresh authenticated evidence explicitly marking the exact key/profile unknown/revoked/not trusted maps to `ADMISSION_GRANT_AUTHENTICATION_FAILED`.

Use dedicated admission key purpose, trusted configured key set and bounded current/retiring overlap for still-valid grants. Token-controlled key fetching is forbidden. Private keys never leave Platform signing/KMS boundary.

KMS/HSM/vendor/publication transport/refresh cadence remain implementation choices inside this ceiling.

## 14. Compatibility / downgrade

Independent dimensions include this profile revision, Platform producer revision, Oteryn-v2 FND-04A consumer revision, FND-02 protocol major/transport profile and route/runtime compatibility revision.

Unknown mandatory revision/claim rejects. No deprecated `EdDSA`, algorithm, profile or Canary downgrade is attempted automatically.

FND-04C must later integrate the production compatibility matrix before any implementation/rollout claim.

## 15. Logging / privacy

MUST NOT log/export raw JWT, GrantNonce/jti, signing private key, OAuth/Game Login Ticket credentials or future reconnect secret/verifier material.

Authorized bounded diagnostic correlation MAY use `attempt_ref`, safe `kid`/profile, WorldId/ChannelId where policy permits, route/runtime observation revision and typed outcome. It MUST NOT export raw `scope_ownership_generation`, Platform security-generation values, private fencing data or transfer details. Use match/stale/relation classes where a fence-sensitive comparison must be diagnosed.

The complete fresh-admission diagnostic templates/correlation fields are owned by FND-04A Section 11. AccountId/CharacterId remain privacy-controlled and are not ordinary high-cardinality metric labels.

## 16. Independent fixtures required before implementation acceptance

Positive fixtures include canonical Ed25519 grant, key overlap/rotation, trust evidence exactly `5s`, time/skew bounds, canonical UUID/generation encoding, current ownership and current world match.

Negative/fault fixtures include:

- `alg=none`, deprecated `EdDSA`, wrong algorithm/key type/curve;
- `jku`, `x5u`, embedded `jwk`, `crit`, extra protected header;
- wrong type/issuer/audience/profile/purpose;
- malformed/duplicate/unknown claims and noncanonical UUID/base64url/generation;
- wrong UUID version/variant for `attempt_ref`, `character_id`, `world_id`, `channel_id`;
- oversized token/header/payload;
- not-yet-valid, expired or >30s lifetime;
- unknown/revoked key under fresh trust evidence -> authentication failed;
- trust evidence `>5s`, unavailable, unauthenticated, contradictory -> stale security evidence;
- key/profile revocation after early verification but before final commit;
- disabled/stale Platform-security evidence;
- stale route/runtime observation or changed scope ownership;
- replay/concurrent GrantNonce;
- ambiguous producer issuance reconciliation;
- mixed revision/downgrade attempt;
- **ownership-before-world negative:** grant references a CharacterId not currently owned by account -> account/character conflict; do not return world-mismatch classification for that candidate;
- **initial owned-character world mismatch:** ownership valid, but signed `world_id` differs from current authoritative world -> `ADMISSION_GRANT_WORLD_STALE`, no candidate nonce/authority mutation;
- **world transfer/change-before-commit:** ownership/world initially valid, then current authoritative CharacterId->WorldId/world eligibility changes before step 16 -> `ADMISSION_GRANT_WORLD_STALE`, no candidate mutation;
- stale grant is never retargeted to new/current world or Channel.

### Change-before-commit matrix

Independently change after earlier validation and before step 16: JWT time; key/profile trust/freshness; Platform security; route/runtime/current target; protocol/transport/compatibility; AccountId->CharacterId ownership; CharacterId->WorldId/world eligibility; GrantNonce; AccountPresence/incumbent; CharacterLease/fence; or a newer world-transfer/handoff/fence/takeover/terminal transition.

Every losing candidate fails before its own authority mutation and preserves actual current authority. Fixtures must be independently produced/validated enough that producer and consumer cannot share one bug unnoticed.

## 17. Error integration

Every FND-04A fresh-admission symbolic outcome is fully defined in `FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md`, including code/category, progression, retry authority, mutation outcome, public class, redacted diagnostic message and credential-free correlation fields.

FND-04C may integrate accepted rows but cannot weaken them.

## 18. Non-authorization

This profile implements nothing. It does not authorize Platform issuer code, Rust verifier/consume store, security projection transport, persistence/cache schema, JWT library, KMS/HSM/vendor, production keys, production routing or live traffic. Overall FND-04 remains incomplete until FND-04B, FND-04C and lifecycle closeout complete.
