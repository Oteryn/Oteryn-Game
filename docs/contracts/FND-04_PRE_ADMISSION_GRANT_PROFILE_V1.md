# FND-04 Pre-Admission Grant Security / Interchange Profile v1

- Status: Candidate normative cross-repository security profile; canonical when the owning FND-04 delivery merges
- Profile ID: `oteryn-pre-admission-v1`
- Applies to: fresh native Oteryn-v2 gameplay entry authorization produced by Oteryn Platform and consumed by Oteryn-v2 final game admission
- Does not apply to: OAuth tokens, web sessions, Game Login Tickets, reconnect credentials, Channel/Instance handoff credentials, Canary compatibility admission or already-admitted GameSession control
- Cryptographic container: JWS Compact Serialization carrying a JWT claims set
- Signature profile: Ed25519 via JOSE `alg = EdDSA`
- Security guidance: RFC 7515, RFC 7519, RFC 8037 and RFC 8725 validation rules
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
alg = EdDSA
key subtype = Ed25519
```

Only this algorithm/key subtype is accepted under profile v1.

Consumers MUST reject:

- `alg = none`;
- HMAC/RSA/ECDSA fallback;
- Ed448 or another `OKP` curve under this profile;
- a token whose key type/curve is incompatible with Ed25519;
- an algorithm selected dynamically from untrusted token content beyond exact allowlist matching.

Changing the signature algorithm or container requires a new security-profile revision and cross-language fixtures. It is not a silent compatible change.

## 3. Protected JOSE header

The protected header MUST contain exactly these members:

```json
{
  "alg": "EdDSA",
  "kid": "<trusted-key-id>",
  "typ": "oteryn-admission+jwt"
}
```

Rules:

- `alg` MUST equal `EdDSA` exactly;
- `typ` MUST equal `oteryn-admission+jwt` exactly;
- `kid` MUST be a bounded ASCII identifier looked up only in the consumer's trusted admission-key set;
- header keys outside `alg`, `kid`, `typ` are rejected in v1;
- `kid` length MUST be 1..64 ASCII characters and match `[A-Za-z0-9._-]+`;
- token-controlled key discovery is forbidden.

Therefore v1 explicitly rejects headers such as:

- `jku`;
- `x5u`;
- `x5c`;
- embedded `jwk`;
- `crit`;
- `cty`;
- `zip`;
- detached/unencoded payload controls such as `b64=false`.

The verifier never fetches a key from a URI supplied by the token.

## 4. Canonical issuer and audience

Profile v1 freezes:

```text
iss = urn:oteryn:platform:game-admission
au d = urn:oteryn:game:admission
```

Normative value (without the spacing shown above for readability):

```text
aud = urn:oteryn:game:admission
```

The producer and consumer MUST compare both as exact case-sensitive strings.

The signing key purpose is dedicated to `oteryn-pre-admission-v1`. A key trusted for OAuth, Game Login Tickets, service authentication or another credential type is not implicitly trusted for this profile.

## 5. Required claims

The JWT payload MUST be a JSON object containing exactly the required claims below plus only extension claims registered by a later compatible profile revision.

Unknown unregistered claims are rejected by v1 so a security-critical producer change cannot be silently ignored by an old consumer.

### 5.1 Standard claims

| Claim | Type | Rule |
|---|---|---|
| `iss` | string | exact value from Section 4 |
| `aud` | string | exact single audience string from Section 4; arrays are rejected in v1 |
| `iat` | integer JSON number | NumericDate whole seconds, authoritative producer time |
| `nbf` | integer JSON number | NumericDate whole seconds; MUST be `>= iat - 1` and `<= iat + 1` |
| `exp` | integer JSON number | NumericDate whole seconds; MUST be `> iat` and `exp - iat <= 30` |
| `jti` | string | GrantNonce; 32 cryptographically random bytes encoded base64url without padding |

`jti` therefore has exactly 43 base64url characters in v1.

### 5.2 Oteryn private claims

| Claim | Type | Rule |
|---|---|---|
| `profile` | string | exact `oteryn-pre-admission-v1` |
| `purpose` | string | exact `fresh_entry` |
| `attempt_ref` | string | Platform AdmissionAttemptRef; canonical lowercase UUIDv7 text |
| `account_id` | string | canonical lowercase UUID text |
| `character_id` | string | canonical lowercase UUID text |
| `world_id` | string | canonical lowercase UUID text |
| `channel_id` | string | canonical lowercase UUID text |
| `account_security_generation` | string | decimal non-zero uint64, no leading `+`, no whitespace, no leading zero except impossible zero which is rejected |
| `route_revision` | string | bounded ASCII opaque revision, 1..64 chars, `[A-Za-z0-9._:-]+` |
| `runtime_observation_revision` | string | bounded ASCII opaque revision, 1..64 chars, `[A-Za-z0-9._:-]+` |
| `scope_ownership_generation` | string | decimal non-zero uint64 encoded as string |
| `protocol_major` | integer JSON number | exact `1` |
| `transport_profile` | integer JSON number | exact `1` |
| `compatibility_revision` | string | bounded ASCII opaque producer/consumer compatibility revision, 1..64 chars, `[A-Za-z0-9._:-]+` |

UUID claims MUST parse as standard UUIDs and MUST round-trip to the exact canonical lowercase hyphenated text supplied in the token. Nil UUID is rejected.

Generation values are strings rather than JSON numbers so PHP/JavaScript-adjacent tooling cannot silently lose `uint64` precision above `2^53`.

`attempt_ref` is an operation/correlation reference only. It is not a foundation entity identity and never becomes GameSessionId.

`jti` is the concrete grant's consume/replay identity. It is distinct from `attempt_ref`.

## 6. Size and parsing limits

Before cryptographic verification, the consumer MUST enforce:

- total compact token length: at most 4096 ASCII bytes;
- exactly three compact JWS segments;
- protected-header decoded JSON: at most 512 bytes;
- payload decoded JSON: at most 3072 bytes;
- JSON nesting depth: at most 2;
- duplicate JSON object member names: reject;
- invalid UTF-8, non-canonical base64url or base64 padding: reject;
- floating-point, exponent-form or fractional NumericDate values: reject;
- JSON `null` for required claims: reject.

No decompression is supported.

The outer FND-02 admission-material bound also applies; the stricter applicable bound wins.

## 7. Time policy

Profile v1 defines security ceilings, not target latency:

```text
maximum grant lifetime: 30 seconds from iat to exp
maximum verifier clock skew allowance: 5 seconds
```

A producer MAY issue a shorter lifetime. A consumer MUST NOT accept a grant whose declared lifetime exceeds 30 seconds.

Validation uses trusted server time only.

At verification time `now`, require:

```text
now + 5s >= nbf
now - 5s < exp
abs(iat - now) <= 35s only as a structural sanity bound
exp - iat <= 30s
```

The skew allowance does not extend replay-record retention below the latest possible acceptance instant.

Client clocks never affect these calculations.

## 8. GrantNonce and one-time game consumption

`jti` is the `GrantNonce`.

Requirements:

- 32 cryptographically random bytes generated by the Platform admission issuer;
- base64url without padding;
- unique with cryptographic probability under the issuer's key/profile scope;
- game-domain authoritative consume state is keyed by at least `(trusted issuer, profile, jti)`;
- one GrantNonce may participate in at most one successful authoritative admission commit;
- concurrent use has one linearized winner at most;
- a consumed grant never becomes reusable because a response was lost;
- replay fails closed without creating/fencing a different current session.

Consume/replay evidence MUST remain authoritative at least until:

```text
exp + maximum_clock_skew
```

and longer when a DUR/reconciliation contract requires it.

## 9. AdmissionAttemptRef and producer idempotency

`attempt_ref` is a Platform producer operation/correlation reference.

It is separate from GrantNonce.

Profile v1 represents it as canonical UUIDv7 text because it needs globally collision-resistant cross-system correlation while preserving approximate creation ordering for operations. This does not add `AdmissionId` to the foundation entity catalogue.

For one logical issuance attempt:

- retry/reconciliation uses the same `attempt_ref`;
- the Platform issuer MUST NOT mint multiple independently usable capabilities because an issuance response was lost;
- producer behavior MUST either recover the exact prior issuance outcome or deterministically retire/fail that attempt and require a new authenticated attempt;
- a new independent login/admission attempt uses a new `attempt_ref`.

Oteryn-v2 may log an authorized redacted/pseudonymous correlation of `attempt_ref`, but never treats it as authentication, GameSession identity or replay-consume authority.

## 10. Platform account-security freshness

Profile v1 binds:

```text
account_id
account_security_generation
```

The producer only issues under its current authoritative Platform security state.

The game admission boundary MUST additionally have a trusted Platform-security validity projection that can establish, for new admissions:

- whether the account is explicitly disabled/revoked for fresh game admission;
- the minimum/current accepted `account_security_generation` or equivalent invalidation floor;
- a global/source freshness watermark for the projection.

Profile v1 freezes this security ceiling:

```text
maximum accepted age of required Platform-security projection evidence: 5 seconds
```

If required evidence is older than 5 seconds, unavailable, unauthenticated, contradictory or cannot establish that the grant's security generation is still admissible, **new admission fails closed**.

If the projection states:

```text
account disabled/revoked
OR grant.account_security_generation < minimum_valid_generation
```

then the grant is rejected even when its signature and `exp` remain valid.

The concrete projection transport/storage/cache is not defined here.

This mechanism governs fresh admission. It does not by itself give Platform authority to asynchronously terminate an already-admitted GameSession; such post-admission emergency control requires a separate game-domain fenced control contract.

## 11. Runtime observation and ownership-generation binding

Fresh-entry issuance binds:

- `world_id`;
- `channel_id`;
- `route_revision`;
- `runtime_observation_revision`;
- `scope_ownership_generation`;
- `protocol_major`;
- `transport_profile`;
- `compatibility_revision`.

At final admission Oteryn-v2 MUST revalidate current authoritative game-domain state.

Default v1 rule:

```text
current target scope ownership generation
!= token.scope_ownership_generation
-> reject as stale grant
```

The consumer also rejects when route/revision compatibility is superseded or target scope is not currently eligible/open for fresh admission.

Profile v1 deliberately prefers a fresh Platform route/grant after GameNode/channel-owner recovery over allowing an old bearer capability to float across ownership generations.

NodeId is not a grant claim and never substitutes for current scope ownership generation.

No silent retarget to another Channel, owner, protocol family or Canary compatibility route is allowed.

## 12. Verification and admission order

The consumer performs, conceptually, this order:

1. outer FND-02 material bound;
2. compact-shape and size limits;
3. strict protected-header parsing and exact profile allowlist;
4. trusted `kid` lookup in the dedicated admission verification-key set;
5. Ed25519 signature verification;
6. exact `typ`, `iss`, `aud`, `profile`, `purpose` validation;
7. time/lifetime/skew validation;
8. claim schema/type/canonical encoding validation;
9. Platform account-security projection freshness/revocation/generation validation;
10. route/runtime-observation/ownership-generation/current-scope validation;
11. GrantNonce consume eligibility/replay check;
12. authoritative current AccountId -> CharacterId ownership validation;
13. AccountPresenceClaim / lifecycle / duplicate-login evaluation;
14. CharacterLease compatibility/acquisition;
15. one atomic final admission commit creates GameSessionId + connection_generation `1` + reconnect-proof state and consumes GrantNonce;
16. only after commit may admission success be published.

A failure before step 15 creates no partial player-control authority.

## 13. Key distribution and rotation

Game-side verification uses trusted public Ed25519 keys only.

Requirements:

- dedicated key purpose/profile;
- `kid` selects only from a locally/trusted-control-plane provisioned allowlist;
- private signing keys never leave the Platform signing authority/KMS boundary;
- multiple current/retiring verification keys may coexist during bounded rotation;
- a retiring key may validate only grants that are still otherwise within profile lifetime/revocation policy;
- emergency key revocation can invalidate an otherwise unexpired grant immediately once the game admission trust projection learns the revocation;
- unavailable/too-stale key-revocation state fails closed for new admission when the selected deployment contract requires that state to establish trust.

The exact KMS/HSM/vendor, publication transport and rotation cadence remain implementation/security-operations choices.

## 14. Compatibility and downgrade behavior

The following are independent version dimensions:

- PreAdmissionGrant profile (`oteryn-pre-admission-v1`);
- Platform producer revision;
- game consumer/FND-04 state-machine revision;
- protocol major/transport profile;
- route/runtime compatibility revision.

Production enablement requires an explicit producer/consumer compatibility matrix.

A consumer that does not understand a mandatory profile revision/claim MUST reject. It may not ignore a security-critical claim and accept the token as v1.

No profile downgrade, alternate algorithm or Canary fallback is attempted automatically.

## 15. Logging and privacy

MUST NOT log or export:

- raw compact JWT;
- raw GrantNonce/jti;
- signing private keys;
- OAuth/Game Login Ticket credentials;
- reconnect secret material;
- verifier digests.

Authorized diagnostics/audit MAY contain bounded non-secret correlation such as:

- `attempt_ref`;
- safe `kid`/profile revision;
- WorldId/ChannelId where policy permits;
- typed internal failure category;
- current/stale generation comparison result without private fencing material.

AccountId/CharacterId handling follows privacy/access policy and should not become ordinary high-cardinality metric labels.

## 16. Independent fixtures required before implementation acceptance

At minimum provide independent producer and consumer fixtures for:

### Positive

- one canonical Ed25519-signed v1 grant;
- key rotation with current and retiring key;
- maximum allowed lifetime/skew boundaries;
- exact canonical UUID/generation/string encodings.

### Negative

- `alg=none`;
- wrong JOSE algorithm/key type/curve;
- unknown `kid`;
- `jku`, `x5u`, embedded `jwk`, `crit`, extra protected header;
- wrong `typ`, `iss`, `aud`, `profile`, `purpose`;
- expired/not-yet-valid/over-30-second lifetime;
- malformed/duplicate claims;
- noncanonical UUID/base64url/generation encoding;
- oversized header/payload/token;
- stale/disabled Platform account security generation;
- Platform-security projection older than 5 seconds;
- stale route/runtime observation or changed scope ownership generation;
- consumed GrantNonce replay and concurrent consume race;
- ambiguous issuer retry with same AdmissionAttemptRef;
- mixed producer/consumer profile revision and downgrade attempt.

Fixtures MUST be generated/validated independently enough that producer and consumer cannot share one serialization/validation bug unnoticed.

## 17. Non-authorization

This profile does not implement or authorize:

- Platform issuer code;
- Oteryn-v2 verifier/consume store;
- HTTP/gRPC/event projection transport;
- database/cache schema;
- Rust/PHP library choice;
- KMS/HSM/vendor;
- production keys;
- production routing or traffic.

It defines the exact v1 interoperability/security contract to be implemented later under separate authority.
