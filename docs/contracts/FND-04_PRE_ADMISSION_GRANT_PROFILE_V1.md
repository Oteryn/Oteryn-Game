# FND-04 Pre-Admission Grant Security / Interchange Profile v1

- Status: Candidate normative profile owned by FND-04A; canonical when the owning FND-04A delivery merges
- Profile ID: `oteryn-pre-admission-v1`
- Applies to: fresh native Oteryn-v2 gameplay entry authorization produced by Oteryn Platform and consumed by Oteryn-v2 final game admission
- Does not apply to: OAuth tokens, web sessions, Game Login Tickets, reconnect credentials, reauthenticated recovery grants, Channel/Instance handoff credentials, Canary compatibility admission or already-admitted GameSession control
- Cryptographic container: JWS Compact Serialization carrying a JWT claims set
- Signature profile: fully specified JOSE `alg = Ed25519`
- Standards baseline: RFC 7515, RFC 7519, RFC 8032, RFC 8037, RFC 8725 and RFC 9864
- Normative admission authority companion: `docs/architecture/FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md`
- Does not select: PHP/Rust JWT library, KMS/HSM/vendor, key-distribution transport, persistence/cache schema or production deployment

## 1. Purpose

This profile removes cross-language ambiguity from Platform -> Oteryn-v2 fresh-entry authorization while preserving the accepted authority split:

```text
Platform signs one bounded attempt capability
Oteryn-v2 verifies capability + current authoritative game facts
Oteryn-v2 consumes the grant at most once
Oteryn-v2 creates canonical GameSessionId only after final admission succeeds
```

A valid signature is necessary but never sufficient for game admission. In particular, a signed `world_id` is an authorization binding to be checked against current game-domain character/world state; it is not proof that the character still belongs to or is eligible for that world.

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

If the producer cannot prove whether the prior issuance succeeded, the exact attempt enters `ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED`. This is a bounded `DEPENDENCY_UNAVAILABLE` / `RETRYABLE` state: retry authority is limited to reconciliation/status recovery for the **same AdmissionAttemptRef**. The producer MUST NOT mint a second independently usable capability for that attempt and MUST NOT start a new independent attempt merely because the response was lost. If the prior outcome cannot be recovered within the registered attempt deadline, the old attempt must be deterministically retired and any possibly issued capability must be proven no longer acceptable before a new independently authorized attempt with a new AdmissionAttemptRef may proceed. Public presentation is `TEMPORARILY_UNAVAILABLE`; no gameplay authority is implied or created by this producer-side ambiguity.

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

## 11. Runtime, route and current character-world applicability

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

Default route/runtime rule:

```text
current target scope ownership generation
!= token.scope_ownership_generation
-> reject as stale grant
```

Default v1 character-world rule:

```text
current authoritative CharacterId -> WorldId relation
must equal token.world_id
AND current character lifecycle must permit fresh admission to token.world_id
```

`CharacterId` is global and may survive a legal world transfer. Therefore `AccountId -> CharacterId` ownership plus a valid route for `token.world_id` does not prove that the character is currently eligible for that world.

If current character world/eligibility does not match the signed `world_id`, or changes after earlier validation but before the final commit:

- reject as `ADMISSION_GRANT_WORLD_STALE`;
- do not consume GrantNonce for the losing candidate;
- do not create/advance AccountPresenceClaim, CharacterLease, GameSession or TransportBinding authority for it;
- never silently retarget the old grant to the character's current/new world;
- require current world resolution plus a newly authorized route/grant.

Also reject superseded/incompatible route, runtime observation, protocol/transport/compatibility revision or non-open target lifecycle.

v1 intentionally requires a fresh Platform route/grant after target owner-generation or character-world applicability change instead of allowing an old bearer capability to float across recovered/replaced owners or legal world transfers.

NodeId is not a grant claim and never substitutes for scope ownership generation.

No silent retarget to another World, Channel, owner, protocol family or Canary route.

## 12. Verification/admission order and final linearization

Steps 1–15 are fail-fast validation and eligibility evaluation. **They are not authorization escrow.** No mutable predicate checked before the final commit is trusted merely because it passed earlier.

1. outer FND-02 material bound;
2. compact-shape/parser/size limits;
3. exact protected-header profile;
4. prove authenticated current admission signing-key/profile trust/revocation evidence with accepted age `<= 5 seconds`, then perform trusted `kid` lookup in the dedicated admission verification-key set;
5. Ed25519 signature verification;
6. exact `typ`, `iss`, `aud`, `profile`, `purpose`;
7. time/lifetime/skew;
8. claim schema/canonical encoding, including UUID version/variant requirements;
9. current Platform-security projection/revocation/generation;
10. route/runtime-observation/current ownership-generation/current-scope plus protocol/transport/compatibility validation;
11. current authoritative `CharacterId -> WorldId` / world-eligibility validation against `world_id`;
12. GrantNonce consume eligibility/replay check;
13. authoritative `AccountId -> CharacterId` ownership/lifecycle;
14. AccountPresenceClaim / duplicate-login eligibility evaluation;
15. CharacterLease acquisition eligibility plus current runtime-scope authority/readiness evaluation;
16. one atomic final admission linearization boundary revalidates **every mutable predicate relevant to authority creation** and, only if all remain valid, consumes GrantNonce and establishes the complete admission authority set;
17. publish admission success only after commit.

Immediately before and atomically with step 16 authority creation, the current game-domain owner MUST revalidate at minimum:

- the JWT still satisfies the accepted `nbf`/`exp`/lifetime/skew equations at trusted server time;
- authenticated admission signing-key/profile trust/revocation evidence still has accepted age `<= 5 seconds` and still accepts the exact key/profile/issuer/purpose;
- current authenticated Platform-security evidence still has accepted age `<= 5 seconds`, remains non-contradictory and still admits the token `account_security_generation` and account state;
- `route_revision`, `runtime_observation_revision`, target lifecycle, `scope_ownership_generation`, current runtime owner/placement and readiness still identify the exact permitted current target;
- `protocol_major`, `transport_profile` and `compatibility_revision` are still supported by the exact current admission/runtime/content/ruleset boundary;
- current authoritative `CharacterId -> WorldId` / world-eligibility still matches the signed `world_id`;
- GrantNonce is still eligible and has not been consumed by a competing successful admission;
- current `AccountId -> CharacterId` ownership/lifecycle still matches;
- AccountPresenceClaim / duplicate-login state still permits this exact CharacterId and no newer competing claim has won;
- CharacterLease is still legally acquirable/current for this exact character and no newer lease/fence generation conflicts;
- no newer world transfer, handoff, fence, takeover, terminal lifecycle or other current authority transition has superseded the candidate.

Step 16 atomically performs the first authoritative admission effects:

```text
consume GrantNonce
+ establish/advance AccountPresenceClaim as required
+ establish/acquire current CharacterLease as required
+ create canonical GameSessionId
+ GameSession ACTIVE
+ connection_generation = 1
+ initialize reconnect-proof state for later FND-04B continuity semantics
+ initial authoritative session/reconciliation boundary
```

No AccountPresenceClaim or CharacterLease acquisition becomes externally authoritative before this boundary merely because steps 14–15 evaluated eligibility.

If any mutable predicate changed after its earlier check, step 16 fails before any candidate authority mutation and uses the specific FND-04A outcome for the changed fact. A changed character-world relation uses `ADMISSION_GRANT_WORLD_STALE`. A losing candidate never rolls back or overwrites whatever world-transfer, account-presence, lease, runtime or session authority is actually current.

If required current key/profile trust/revocation evidence is older than 5 seconds, unavailable, unauthenticated, contradictory or otherwise cannot prove current trust, fail as `ADMISSION_GRANT_SECURITY_EVIDENCE_STALE`; GrantNonce is not consumed and no presence/lease/session/transport authority mutates. If authenticated evidence within the freshness bound explicitly says the exact key/profile is unknown, revoked or not trusted, fail as `ADMISSION_GRANT_AUTHENTICATION_FAILED` / `SECURITY_TERMINAL`. Validation earlier in the request is not trust escrow: a key/profile revoked before the authority-changing commit cannot succeed merely because signature verification previously passed.

No failure before or during step 16 creates partial player-control authority.

## 13. Key distribution / rotation

Game-side verification uses trusted Ed25519 public keys only.

Profile v1 freezes:

```text
maximum accepted age of required authenticated signing-key/profile trust/revocation evidence: 5 seconds
```

Age is evaluated using trusted server time against authenticated provenance/freshness evidence from the trusted key/profile distribution authority. `age <= 5s` is accepted; `age > 5s` is stale. If current freshness cannot be authenticated or proven, the consumer fails closed rather than extending trust indefinitely.

Requirements:

- dedicated admission profile/key purpose;
- `kid` selects only from trusted provisioned/configured key set;
- private signing keys never leave Platform signing/KMS boundary;
- bounded current/retiring verification-key overlap may support still-valid grants;
- grant expiry remains binding even if a key remains trusted;
- emergency key/profile revocation invalidates otherwise-unexpired grants once current authenticated trust evidence records the revocation; stale/unavailable trust evidence never counts as continued authorization;
- stale/unavailable/unauthenticated/contradictory trust/revocation evidence maps to `ADMISSION_GRANT_SECURITY_EVIDENCE_STALE`, with bounded retry only after fresh evidence while the same unconsumed grant and all other bindings remain valid;
- fresh authenticated evidence that explicitly marks the exact key/profile unknown/revoked/not trusted maps to `ADMISSION_GRANT_AUTHENTICATION_FAILED`;
- neither failure consumes GrantNonce or creates gameplay authority.

Exact KMS/HSM/vendor, publication transport, refresh mechanism and rotation cadence inside this security ceiling are implementation/security-operations choices.

## 14. Compatibility / downgrade

Independent version dimensions include:

- `oteryn-pre-admission-v1` profile;
- Platform producer revision;
- Oteryn-v2 FND-04A consumer/state-machine revision;
- protocol major/transport profile;
- route/runtime compatibility revision.

Production enablement requires an explicit producer/consumer compatibility matrix finalized by FND-04C before production authorization.

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
- route/runtime observation revision;
- typed internal outcome;
- current/stale relation class without private fencing material.

The complete fresh-admission diagnostic templates/correlation fields are owned by FND-04A Section 11. AccountId/CharacterId handling follows privacy/access policy and does not become ordinary high-cardinality metric labels.

## 16. Independent fixtures required before implementation acceptance

Positive fixtures include:

- canonical `alg=Ed25519` v1 grant;
- current/retiring key rotation;
- authenticated admission key/profile trust/revocation evidence at exact accepted age `5s`;
- lifetime/skew boundaries;
- exact UUID/generation/string encoding;
- grant `world_id` equal to the character's current authoritative world with current admission eligibility.

Negative/fault fixtures include:

- `alg=none`;
- deprecated `alg=EdDSA`;
- wrong algorithm/key type/curve;
- unknown/revoked `kid` under fresh current trust evidence -> `ADMISSION_GRANT_AUTHENTICATION_FAILED`;
- signing-key/profile trust/revocation evidence older than `5s`, unavailable, unauthenticated or contradictory -> `ADMISSION_GRANT_SECURITY_EVIDENCE_STALE`, no GrantNonce consumption and no authority mutation;
- key/profile trusted at initial verification but emergency-revoked before atomic final admission commit -> `ADMISSION_GRANT_AUTHENTICATION_FAILED`, no GrantNonce consumption and no authority mutation;
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
- **initial world mismatch:** signed grant `world_id` differs from current authoritative CharacterId->WorldId/world eligibility -> `ADMISSION_GRANT_WORLD_STALE`, no GrantNonce/authority mutation;
- consumed GrantNonce replay/concurrent consume race;
- ambiguous producer response maps to `ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED`; reconciliation uses the same AdmissionAttemptRef and cannot mint a blind second capability or begin an independent new attempt until the prior attempt is deterministically retired and any possibly issued capability is no longer acceptable;
- mixed producer/consumer revision/downgrade attempt;
- **change-before-commit matrix:** after the corresponding earlier validation succeeds but before step 16, independently expire/not-yet-validate the grant, revoke/stale Platform-security evidence, stale/untrust the signing-key/profile evidence, change route/runtime observation or scope ownership generation, make protocol/transport/compatibility unsupported, **change the current authoritative CharacterId->WorldId/world-eligibility relation (including legal world transfer)**, consume GrantNonce concurrently, change AccountId->CharacterId ownership/lifecycle, let another AccountPresenceClaim/incumbent win, make CharacterLease conflicting/stale, change runtime owner/readiness, or supersede the candidate with a newer world-transfer/handoff/fence/takeover/terminal authority transition; every case must fail before candidate authority mutation with the specific FND-04A outcome, consume no GrantNonce unless another already-successful transition consumed it, and preserve the authority state actually current at the final linearization boundary.

World mismatch/transfer fixtures MUST also prove an old grant is never silently retargeted to the character's new/current World or Channel.

Fixtures MUST be independently produced/validated enough that producer and consumer cannot share one serialization/validation bug unnoticed.

## 17. Error integration

The symbolic outcomes used by this profile are fully defined for the fresh-admission subset in `FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md`, including Foundation category, progression, retry authority, mutation outcome, public class, redacted diagnostic message and credential-free correlation fields.

FND-04C may integrate those accepted rows into the final FND-04 catalogue but does not get authority to weaken the profile or fresh-admission boundary.

## 18. Non-authorization

This profile does not implement or authorize Platform issuer code, Oteryn-v2 verifier/consume store, security-projection transport, database/cache schema, Rust/PHP library choice, KMS/HSM/vendor, production keys, production routing or live traffic. FND-04 overall remains incomplete until FND-04B/FND-04C and lifecycle closeout complete.
