# FND-04 Pre-Admission Grant Security / Interchange Profile v1

- Status: Candidate normative cross-repository security profile; canonical when the owning FND-04 delivery merges
- Profile ID: `oteryn-pre-admission-v1`
- Applies to: fresh native Oteryn-v2 gameplay entry authorization produced by Oteryn Platform and consumed by Oteryn-v2 final game admission
- Does not apply to: OAuth tokens, web sessions, Game Login Tickets, reconnect credentials, reauthenticated recovery grants, Channel/Instance handoff credentials, Canary compatibility admission or already-admitted GameSession control
- Cryptographic container: JWS Compact Serialization carrying a JWT claims set
- Signature profile: fully specified JOSE `alg = Ed25519`
- Standards baseline: RFC 7515, RFC 7519, RFC 8032, RFC 8037, RFC 8725 and RFC 9864
- Does not select: PHP/Rust JWT library, KMS/HSM/vendor, key-distribution transport, persistence/cache schema or production deployment

## 1. Purpose

This profile removes cross-language ambiguity from Platform -> Oteryn-v2 fresh-entry authorization while preserving the accepted authority split:

```text
Platform signs one bounded attempt capability
Oteryn-v2 verifies capability + current authoritative game facts
Oteryn-v2 consumes the grant at most once
Oteryn-v2 creates canonical GameSessionId only after final admission succeeds
```

A valid signature is necessary but never sufficient for game admission.

## 2. Exact v1 cryptographic profile

The v1 profile uses:

```text
JWS Compact Serialization
JWT Claims Set payload
alg = Ed25519
Ed25519 parameter set from RFC 8032
```

RFC 9864 registers the fully specified JOSE algorithm identifier `Ed25519` and deprecates the older polymorphic `EdDSA` JOSE identifier. Oteryn therefore does not introduce a new v1 contract using deprecated `alg = EdDSA`.

Only `alg = Ed25519` is accepted under profile v1.

Consumers MUST reject:

- `alg = none`;
- deprecated polymorphic `alg = EdDSA`;
- HMAC/RSA/ECDSA fallback;
- `Ed448` or another algorithm identifier under this profile;
- a key whose type/curve is incompatible with Ed25519;
- algorithm selection derived from untrusted token content beyond exact allowlist matching.

Changing the signature algorithm/container requires a new security-profile revision and independent cross-language fixtures. It is not a silent compatible change.

## 3. Protected JOSE header

The protected header MUST contain exactly:

```json
{
  "alg": "Ed25519",
  "kid": "<trusted-key-id>",
  "typ": "oteryn-admission+jwt"
}
```

Rules:

- `alg` MUST equal `Ed25519` exactly;
- `typ` MUST equal `oteryn-admission+jwt` exactly;
- `kid` MUST be a bounded ASCII identifier looked up only in the consumer's trusted admission-key set;
- header keys outside `alg`, `kid`, `typ` are rejected in v1;
- `kid` length MUST be 1..64 ASCII characters matching `[A-Za-z0-9._-]+`;
- token-controlled key discovery is forbidden.

v1 explicitly rejects:

- `jku`;
- `x5u`;
- `x5c`;
- embedded `jwk`;
- `crit`;
- `cty`;
- `zip`;
- detached/unencoded payload controls such as `b64=false`;
- any other protected-header member.

The verifier never fetches a key from a URI supplied by the token.

If JWK representation is used by trusted key distribution, the Ed25519 public key representation follows the accepted JOSE OKP/Ed25519 representation, while the token's `alg` remains the fully specified `Ed25519` value from RFC 9864.

## 4. Canonical issuer and audience

Profile v1 freezes:

```text
iss = urn:oteryn:platform:game-admission
aud = urn:oteryn:game:admission
```

Both are exact case-sensitive strings.

The signing key purpose is dedicated to `oteryn-pre-admission-v1`. A key trusted for OAuth, Game Login Tickets, recovery grants, service authentication or another credential type is not implicitly trusted here.

## 5. Required claims

The JWT payload MUST be a JSON object containing exactly the required claims below. A later compatible extension must be registered by a new understood profile revision; an unknown claim is rejected by v1 so a security-critical producer change cannot be silently ignored.

### 5.1 Standard claims

| Claim | Type | Rule |
|---|---|---|
| `iss` | string | exact Section 4 issuer |
| `aud` | string | exact single Section 4 audience; arrays rejected in v1 |
| `iat` | integer JSON number | whole-second NumericDate, authoritative producer time |
| `nbf` | integer JSON number | whole-second NumericDate; `iat - 1 <= nbf <= iat + 1` |
| `exp` | integer JSON number | `exp > iat` and `exp - iat <= 30` seconds |
| `jti` | string | GrantNonce: 32 cryptographically random bytes encoded base64url without padding |

`jti` is exactly 43 base64url characters in v1.

### 5.2 Oteryn claims

| Claim | Type | Rule |
|---|---|---|
| `profile` | string | exact `oteryn-pre-admission-v1` |
| `purpose` | string | exact `fresh_entry` |
| `attempt_ref` | string | Platform AdmissionAttemptRef; canonical lowercase RFC UUIDv7 text |
| `account_id` | string | canonical lowercase non-nil UUID in the authoritative Platform representation accepted by FND-ID-01 |
| `character_id` | string | canonical lowercase non-nil RFC UUIDv7 text |
| `world_id` | string | canonical lowercase non-nil RFC UUIDv7 text |
| `channel_id` | string | canonical lowercase non-nil RFC UUIDv7 text |
| `account_security_generation` | string | decimal non-zero uint64 string |
| `route_revision` | string | bounded ASCII 1..64, `[A-Za-z0-9._:-]+` |
| `runtime_observation_revision` | string | bounded ASCII 1..64, `[A-Za-z0-9._:-]+` |
| `scope_ownership_generation` | string | decimal non-zero uint64 string |
| `protocol_major` | integer JSON number | exact `1` |
| `transport_profile` | integer JSON number | exact `1` |
| `compatibility_revision` | string | bounded ASCII 1..64, `[A-Za-z0-9._:-]+` |

All UUID claims MUST parse and round-trip to the exact canonical lowercase hyphenated form. Nil UUID is rejected.

`attempt_ref`, `character_id`, `world_id` and `channel_id` additionally MUST encode UUID version `7` and the RFC UUID variant; a syntactically canonical UUIDv1/v4/v6, Microsoft-reserved variant or other non-v7/non-RFC value is rejected. `account_id` remains Platform-owned and is validated against the authoritative Platform representation accepted by FND-ID-01 rather than being silently redefined as an Oteryn-issued UUIDv7.

Generation values are JSON strings so cross-language tooling cannot silently lose uint64 precision above `2^53`.

`attempt_ref` is producer operation/correlation identity only. It is not GameSessionId, GrantNonce or a foundation entity ID.

`jti` is the concrete capability's game consume/replay identity and is distinct from `attempt_ref`.

## 6. Size and parser limits

Before signature verification the consumer MUST enforce:

- compact token <= 4096 ASCII bytes;
- exactly 3 JWS segments;
- decoded protected header <= 512 bytes;
- decoded payload <= 3072 bytes;
- JSON nesting depth <= 2;
- duplicate JSON object member names reject;
- invalid UTF-8 reject;
- malformed/non-canonical base64url or padded compact segments reject;
- floating-point/exponent/fractional NumericDate reject;
- missing/null required claim reject;
- decompression unsupported.

The outer FND-02 admission-material bound also applies; the stricter bound wins.

## 7. Time policy

Security ceilings:

```text
maximum grant lifetime: 30 seconds from iat to exp
maximum verifier clock-skew allowance: 5 seconds
```

A producer MAY issue a shorter lifetime. A consumer MUST reject a declared lifetime above 30 seconds.

At trusted server time `now`, require:

```text
now + 5s >= nbf
now - 5s < exp
exp > iat
exp - iat <= 30s
abs(iat - now) <= 35s as structural sanity bound
```

Client clocks never affect validity.

## 8. GrantNonce and one-time game consumption

`jti` is the GrantNonce.

Requirements:

- 32 cryptographically random bytes generated by the Platform admission issuer;
- base64url without padding;
- authoritative consume state keyed by at least `(trusted issuer, profile, jti)`;
- one GrantNonce may participate in at most one successful authoritative admission commit;
- concurrent use has at most one linearized winner;
- consumed grant never becomes reusable after a lost response;
- a losing replay cannot create/revive/fence a different current session.

Consume/replay evidence MUST remain authoritative at least until:

```text
exp + 5-second maximum clock skew
```

and longer when DUR/reconciliation requires it.

## 9. AdmissionAttemptRef producer idempotency

`attempt_ref` is a Platform producer operation/correlation reference represented as canonical RFC UUIDv7 text. This does not add `AdmissionId` to the foundation entity catalogue.

For one logical issuance attempt:

- retries/reconciliation use the same `attempt_ref`;
- the producer MUST NOT mint multiple independently usable capabilities because an issuance response was lost;
- producer behavior MUST either recover the exact prior issuance outcome or deterministically retire/fail that attempt and require a new authenticated attempt;
- a new independent login/admission attempt uses a new `attempt_ref`.

Oteryn-v2 may use an authorized redacted correlation of `attempt_ref`; it never treats it as authentication, GameSession identity or game consume authority.

## 10. Platform account-security freshness

The grant binds:

```text
account_id
account_security_generation
```

The producer only issues under current authoritative Platform security state.

The game admission boundary MUST additionally consume a trusted Platform-security validity projection able to establish, for new admissions:

- account fresh-admission disabled/revoked state;
- minimum/current accepted `account_security_generation` or equivalent invalidation floor;
- projection/source freshness.

Profile v1 freezes:

```text
maximum accepted age of required Platform-security evidence: 5 seconds
```

If required evidence is older than 5 seconds, unavailable, unauthenticated, contradictory or cannot prove the grant generation remains admissible, **new admission fails closed**.

Reject when:

```text
account disabled/revoked
OR grant.account_security_generation < minimum_valid_generation
```

Signature validity and nominal `exp` do not override newer Platform security invalidation.

The concrete projection transport/storage/cache is not defined here.

This fresh-admission mechanism does not give Platform authority to terminate an already-admitted GameSession. Post-admission emergency control requires a separate game-domain fenced control contract.

## 11. Runtime observation / ownership-generation applicability

Fresh-entry issuance binds:

- `world_id`;
- `channel_id`;
- `route_revision`;
- `runtime_observation_revision`;
- `scope_ownership_generation`;
- `protocol_major`;
- `transport_profile`;
- `compatibility_revision`.

At final admission Oteryn-v2 MUST revalidate current authoritative game state.

Default v1 rule:

```text
current target scope ownership generation
!= token.scope_ownership_generation
-> reject as stale grant
```

Also reject superseded/incompatible route, runtime observation, protocol/transport/compatibility revision or non-open target lifecycle.

v1 intentionally requires a fresh Platform route/grant after target owner-generation change instead of allowing an old bearer capability to float across recovered/replaced owners.

NodeId is not a grant claim and never substitutes for scope ownership generation.

No silent retarget to another Channel, owner, protocol family or Canary route.

## 12. Verification/admission order

1. outer FND-02 material bound;
2. compact-shape/parser/size limits;
3. exact protected-header profile;
4. trusted `kid` lookup in dedicated admission verification-key set;
5. Ed25519 signature verification;
6. exact `typ`, `iss`, `aud`, `profile`, `purpose`;
7. time/lifetime/skew;
8. claim schema/canonical encoding, including UUID version/variant requirements;
9. current Platform-security projection/revocation/generation;
10. route/runtime-observation/current ownership-generation/current-scope validation;
11. GrantNonce consume eligibility/replay check;
12. authoritative AccountId -> CharacterId ownership/lifecycle;
13. AccountPresenceClaim / duplicate-login evaluation;
14. CharacterLease compatibility/acquisition;
15. one atomic final admission commit consumes GrantNonce and creates GameSessionId + connection_generation `1` + reconnect-proof state;
16. publish admission success only after commit.

No failure before step 15 creates partial player-control authority.

## 13. Key distribution / rotation

Game-side verification uses trusted Ed25519 public keys only.

Requirements:

- dedicated admission profile/key purpose;
- `kid` selects only from trusted provisioned/configured key set;
- private signing keys never leave Platform signing/KMS boundary;
- bounded current/retiring verification-key overlap may support still-valid grants;
- grant expiry remains binding even if a key remains trusted;
- emergency key revocation can invalidate otherwise unexpired grants once trusted revocation state reaches the consumer;
- unknown/revoked key/profile fails closed.

Exact KMS/HSM/vendor, publication transport and rotation cadence are implementation/security-operations choices.

## 14. Compatibility / downgrade

Independent version dimensions include:

- `oteryn-pre-admission-v1` profile;
- Platform producer revision;
- Oteryn-v2 FND-04 consumer/state-machine revision;
- protocol major/transport profile;
- route/runtime compatibility revision.

Production enablement requires an explicit producer/consumer compatibility matrix.

A consumer that does not understand a mandatory profile revision/claim MUST reject. It may not ignore a security-critical claim and accept as v1.

No profile downgrade, deprecated `EdDSA` fallback, alternate algorithm or Canary fallback is attempted automatically.

## 15. Logging / privacy

MUST NOT log/export:

- raw compact JWT;
- raw GrantNonce/jti;
- signing private key;
- OAuth/Game Login Ticket credentials;
- reconnect secret material;
- secret verifier digest.

Authorized diagnostics/audit MAY contain bounded non-secret correlation such as:

- `attempt_ref`;
- safe `kid`/profile revision;
- WorldId/ChannelId where policy permits;
- typed internal failure category;
- current/stale generation comparison result without private fencing material.

AccountId/CharacterId handling follows privacy/access policy and does not become ordinary high-cardinality metric labels.

## 16. Independent fixtures required before implementation acceptance

Positive fixtures include:

- canonical `alg=Ed25519` v1 grant;
- current/retiring key rotation;
- lifetime/skew boundaries;
- exact UUID/generation/string encoding.

Negative fixtures include:

- `alg=none`;
- deprecated `alg=EdDSA`;
- wrong algorithm/key type/curve;
- unknown/revoked `kid`;
- `jku`, `x5u`, embedded `jwk`, `crit`, extra protected header;
- wrong `typ`, `iss`, `aud`, `profile`, `purpose`;
- expired/not-yet-valid/over-30-second lifetime;
- malformed/duplicate/unknown claims;
- noncanonical UUID/base64url/generation encoding;
- canonical-looking wrong UUID version and wrong UUID variant for `attempt_ref`, `character_id`, `world_id` or `channel_id`;
- oversized header/payload/token;
- disabled/stale Platform account-security generation;
- Platform-security evidence older than 5 seconds;
- stale route/runtime observation or changed scope ownership generation;
- consumed GrantNonce replay/concurrent consume race;
- ambiguous producer retry with same AdmissionAttemptRef;
- mixed producer/consumer revision/downgrade attempt.

Fixtures MUST be independently produced/validated enough that producer and consumer cannot share one serialization/validation bug unnoticed.

## 17. Non-authorization

This profile does not implement or authorize Platform issuer code, Oteryn-v2 verifier/consume store, security-projection transport, database/cache schema, Rust/PHP library choice, KMS/HSM/vendor, production keys, production routing or live traffic.
