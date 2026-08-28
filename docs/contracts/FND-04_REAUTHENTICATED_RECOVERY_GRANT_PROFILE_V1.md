# FND-04 Reauthenticated Recovery Grant Security / Interchange Profile v1

- Status: Candidate normative profile owned by FND-04B; canonical only when the owning delivery merges
- Profile ID: `oteryn-reauth-recovery-v1`
- Applies to: Platform-reauthenticated attempts to recover control of an already-existing authoritative CharacterId actor/session state
- Does not apply to: fresh admission, OAuth/web sessions, Game Login Tickets, fast reconnect proof, handoff credentials, Canary admission or healthy-session migration
- Cryptographic container: JWS Compact Serialization carrying a JWT claims set
- Signature profile: fully specified JOSE `alg = Ed25519`
- Standards baseline: RFC 7515, RFC 7519, RFC 8032, RFC 8037, RFC 8725 and RFC 9864
- Normative authority companion: `docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md`
- Does not select: implementation library, KMS/HSM/vendor, key-distribution transport, recovery-locator transport, persistence/cache schema or production deployment

## 1. Purpose and authority

This profile exists so loss of the game-domain reconnect proof does not force reuse of a fresh-entry credential.

It proves only:

```text
Platform freshly authenticated AccountId
AND Platform currently permits AccountId to attempt recovery of CharacterId in WorldId
```

It does not prove GameSession existence, reconnectability, actor presence, controller health, current Channel/Instance/Node placement, CharacterLease/runtime ownership or permission to recreate/reset/respawn the actor. Oteryn-v2 resolves those facts.

## 2. Purpose separation

Fresh admission and recovery are mutually exclusive credential profiles.

Fresh entry:

```text
typ     = oteryn-admission+jwt
profile = oteryn-pre-admission-v1
purpose = fresh_entry
aud     = urn:oteryn:game:admission
```

Reauthenticated recovery:

```text
typ     = oteryn-recovery+jwt
profile = oteryn-reauth-recovery-v1
purpose = existing_actor_recovery
aud     = urn:oteryn:game:recovery
```

Failure under one profile never causes reinterpretation as the other. Recovery contains no ChannelId/InstanceId/NodeId/runtime-owner authority.

## 3. Exact cryptographic profile

Recovery v1 uses JWS Compact JWT and exact JOSE `alg=Ed25519`.

There is no algorithm negotiation or downgrade:

- missing/null/non-string/empty/non-ASCII/out-of-bound `alg` -> `RECOVERY_GRANT_MALFORMED`;
- syntactically valid algorithm identifier other than exact `Ed25519`, including `none`, deprecated polymorphic `EdDSA`, HMAC/RSA/ECDSA/Ed448/other -> `RECOVERY_GRANT_AUTHENTICATION_FAILED`;
- incompatible key type/curve or Ed25519 verification failure -> `RECOVERY_GRANT_AUTHENTICATION_FAILED`;
- changing algorithm/container requires a new reviewed profile revision and cannot be token-negotiated.

## 4. Protected JOSE header

Canonical successful header:

```json
{
  "alg": "Ed25519",
  "kid": "<trusted-recovery-key-id>",
  "typ": "oteryn-recovery+jwt"
}
```

Pre-signature structural/security processing:

- protected members are exactly `alg`, `kid`, `typ`; duplicate/missing/unknown/extra members are malformed;
- malformed `alg` representation -> `RECOVERY_GRANT_MALFORMED`; well-formed non-exact `alg` -> `RECOVERY_GRANT_AUTHENTICATION_FAILED`;
- `kid` is a string of 1..64 ASCII matching `[A-Za-z0-9._-]+`; malformed representation -> `RECOVERY_GRANT_MALFORMED`; well-formed unknown/untrusted `kid` inside the fixed recovery trust set -> `RECOVERY_GRANT_AUTHENTICATION_FAILED`;
- `typ` is a non-empty string <=64 visible ASCII bytes with no control/whitespace; malformed/missing/null/non-string/out-of-bound -> `RECOVERY_GRANT_MALFORMED`;
- `jku`, `x5u`, `x5c`, embedded `jwk`, `crit`, `cty`, `zip`, `b64=false` and token-controlled key discovery are forbidden malformed protected-header shapes.

The semantic exact `typ=oteryn-recovery+jwt` comparison occurs only **after successful signature verification and authenticated payload structural-schema validation**. A correctly signed but semantically different well-formed `typ` maps to `RECOVERY_GRANT_BINDING_MISMATCH`. Failed algorithm/key/trust/signature must not disclose whether untrusted `typ` would have matched.

## 5. Verifier-anchored trust context

Before token semantics can influence trust, the verifier fixes this expected endpoint/configuration context:

```text
expected issuer = urn:oteryn:platform:game-recovery
expected audience = urn:oteryn:game:recovery
expected profile = oteryn-reauth-recovery-v1
expected purpose = existing_actor_recovery
expected key purpose = dedicated recovery signing for that context
```

Unauthenticated `iss`, `aud`, `profile`, `purpose`, `typ` or other token text MUST NOT choose, broaden or retarget the trusted issuer/profile/key-purpose domain. A well-formed `kid` selects only a candidate key inside that already-fixed trusted set.

## 6. Canonical semantic bindings

```text
iss     = urn:oteryn:platform:game-recovery
aud     = urn:oteryn:game:recovery
typ     = oteryn-recovery+jwt
profile = oteryn-reauth-recovery-v1
purpose = existing_actor_recovery
```

`iss`, `aud`, `typ` and `purpose` are binding semantics. A correctly authenticated well-formed mismatch maps to `RECOVERY_GRANT_BINDING_MISMATCH`.

`profile` is a version/profile semantic. A correctly authenticated well-formed but unsupported profile maps to `RECOVERY_GRANT_REVISION_UNSUPPORTED`; it is never reinterpreted as another credential purpose.

Recovery signing-key purpose is distinct from fresh admission, OAuth, Game Login Ticket and service authentication.

## 7. Authenticated payload structural schema

After successful signature verification, the payload MUST be one JSON object containing exactly the registered claims below. This stage validates **membership, type and canonical lexical shape**, not semantic equality to the expected binding/profile/revision values handled later.

Unknown claims reject as `RECOVERY_GRANT_MALFORMED`.

### 7.1 Standard claims

| Claim | Structural type / shape |
|---|---|
| `iss` | non-empty string <=128 visible ASCII bytes |
| `aud` | non-empty string <=128 visible ASCII bytes; arrays rejected |
| `iat` | whole-second integer NumericDate |
| `nbf` | whole-second integer NumericDate |
| `exp` | whole-second integer NumericDate |
| `jti` | string, exactly 43 base64url-no-padding chars encoding 32 bytes |

### 7.2 Oteryn claims

| Claim | Structural type / shape |
|---|---|
| `profile` | non-empty string <=64 visible ASCII bytes |
| `purpose` | non-empty string <=64 visible ASCII bytes |
| `attempt_ref` | canonical lowercase non-nil RFC UUIDv7 |
| `account_id` | canonical lowercase non-nil authoritative Platform UUID representation accepted by FND-ID-01 |
| `character_id` | canonical lowercase non-nil RFC UUIDv7 |
| `world_id` | canonical lowercase non-nil RFC UUIDv7 |
| `account_security_generation` | decimal non-zero uint64 string |
| `protocol_major` | non-negative integer within registered protocol-major range |
| `transport_profile` | non-negative integer within registered transport-profile range |
| `ruleset_revision` | ASCII 1..64 `[A-Za-z0-9._:-]+` |
| `content_revision` | ASCII 1..64 `[A-Za-z0-9._:-]+` |
| `map_revision` | ASCII 1..64 `[A-Za-z0-9._:-]+` |
| `world_policy_revision` | ASCII 1..64 `[A-Za-z0-9._:-]+` |

The semantic current/supported values for `profile`, protocol, transport and gameplay revisions are evaluated only after this structural stage.

All UUID values MUST parse and round-trip exact canonical lowercase hyphenated form; nil rejects. `attempt_ref`, `character_id` and `world_id` require UUIDv7 + RFC variant. `account_id` remains Platform-owned and is not silently redefined as Oteryn UUIDv7.

The profile MUST NOT contain `channel_id`, `instance_id`, `node_id`, runtime owner identity, scope ownership generation, `GameSessionId` as bearer authority, `HandoffId` or opaque `compatibility_revision`.

FND-02 `schema_revision` remains diagnostic/build evidence rather than exact recovery identity and is deliberately absent.

## 8. Safe pre-authentication parser/material bounds

Before signature verification enforce only bounded safe parsing/authentication prerequisites:

- token <=4096 ASCII bytes;
- exactly 3 JWS segments;
- decoded header <=512 bytes;
- decoded payload <=3072 bytes;
- header/payload decode as bounded JSON objects with nesting <=2;
- duplicate JSON members reject;
- invalid UTF-8 reject;
- malformed/noncanonical/padded base64url reject;
- decompression unsupported;
- protected-header rules from Sections 3–4.

Pre-authentication parsing MUST NOT use semantic payload membership/value differences to reveal binding/profile/revision state. Therefore an invalid-signature token with an otherwise well-formed missing/extra/wrong-bound/unsupported semantic payload remains an authentication failure when the token was structurally parseable enough to authenticate.

Stricter FND-02 outer bounds win.

## 9. Complete credential-classification precedence

Normative order:

1. outer/JWS/JSON/base64/protected-header malformed shape -> `RECOVERY_GRANT_MALFORMED`;
2. well-formed non-exact `alg` -> `RECOVERY_GRANT_AUTHENTICATION_FAILED`;
3. fixed-scope `kid`/key/trust failure or signature failure -> `RECOVERY_GRANT_AUTHENTICATION_FAILED`;
4. after successful signature, exact payload claim membership/type/canonical-shape failure -> `RECOVERY_GRANT_MALFORMED`;
5. after structural schema success, wrong exact `iss`, `aud`, `typ` or `purpose` -> `RECOVERY_GRANT_BINDING_MISMATCH`;
6. after structural schema success, well-formed unsupported `profile` -> `RECOVERY_GRANT_REVISION_UNSUPPORTED`;
7. then evaluate time policy;
8. then Platform-security/trust freshness, ownership/world/current-target state and independent authoritative revisions.

Consequences:

- invalid signature + wrong well-formed binding -> authentication failed;
- invalid signature + unsupported profile/revision -> authentication failed;
- invalid signature + otherwise well-formed semantic payload defect -> authentication failed;
- correctly signed structural schema defect -> malformed;
- correctly signed wrong binding -> binding mismatch;
- correctly signed unsupported profile -> revision unsupported.

## 10. Time policy

Security ceilings:

```text
maximum lifetime: 30s from iat to exp
maximum verifier skew: 5s
```

After authenticated structural schema validation, require:

```text
iat/nbf/exp are whole-second NumericDate values
exp > iat
exp - iat <=30s
iat - 1 <= nbf <= iat + 1
now + 5s >= nbf
now - 5s < exp
abs(iat - now) <=35s
```

Before the accepted `nbf` boundary -> `RECOVERY_GRANT_NOT_YET_VALID`. After expiry -> `RECOVERY_GRANT_EXPIRED`. Invalid intrinsic time structure/range such as `exp <= iat` or lifetime >30s is malformed/invalid input, not a retryable future-valid credential.

Client clocks never affect validity.

## 11. RecoveryGrantNonce and producer attempt

`jti` is the one-time RecoveryGrantNonce. `attempt_ref` is producer issuance/reconciliation identity. They are distinct and neither is GameSessionId.

Game consume state keyed by at least `(trusted issuer, profile, jti)` guarantees:

- at most one successful recovery authority transition;
- one winner under concurrent use;
- consumed nonce remains consumed after lost response;
- losing replay cannot create/revive/fence authority.

Replay evidence remains authoritative through at least `exp + 5s` and longer when durability/reconciliation requires.

Ambiguous producer issuance uses same-`attempt_ref` status/reconciliation only. A blind second independently usable recovery grant is forbidden. A new independent attempt requires deterministic retirement plus proof any possibly issued old capability is no longer acceptable.

Unknown exact issuance outcome maps to `RECOVERY_ATTEMPT_RECONCILIATION_REQUIRED` (`DEPENDENCY_UNAVAILABLE`, bounded `RETRYABLE`, public `TEMPORARILY_UNAVAILABLE`).

## 12. Platform-security evidence

Grant binds `account_id + account_security_generation`.

Current recovery authorization requires authenticated Platform-security evidence proving current enabled/revoked state and accepted generation floor with:

```text
source authority / security purpose / scope
AccountId scope
source_observed_at or equivalently strong authenticated source-time provenance
monotonic/comparable source_revision or equivalent non-rollback fence
current decision facts / accepted generation floor
```

Accepted conservative source age:

```text
upper_bound_source_age <= 5 seconds
```

Cache receive/refresh/store/re-read time never resets age. Missing/future-ambiguous/contradictory provenance or inability to prove the upper bound -> `RECOVERY_GRANT_SECURITY_EVIDENCE_STALE`.

An older comparable revision cannot authorize after a newer accepted revision. Equal revision with contradictory authenticated content is invalid. Newer disable/revoke/higher generation floor cannot be rolled back by older allow even when still younger than five seconds.

On consumer restart, current non-rollback floor must be reconstructed from authoritative evidence or preserved trusted state before recovery authorization; inability fails closed.

Current explicit account deny/revoke -> `RECOVERY_GRANT_SECURITY_STATE_REVOKED`.

## 13. Recovery signing-key/profile trust evidence

Verification uses trusted Ed25519 public keys only in the verifier-configured expected recovery trust scope.

Authenticated trust/revocation evidence must provide:

```text
trusted source authority
expected recovery issuer/profile/key-purpose trust scope
source_observed_at or equivalent authenticated source-time provenance
monotonic/comparable source_revision or equivalent non-rollback fence
current key/profile trust/revocation decision
```

Accepted source-age upper bound is also `<=5s`; cache time never re-ages evidence.

Older trusted evidence cannot roll back a newer accepted revoke/untrust. Equal revision contradiction fails closed. Restart requires reconstruction of a current non-rollback floor before authorization.

- stale/unavailable/unprovable trust evidence -> `RECOVERY_GRANT_SECURITY_EVIDENCE_STALE`;
- fresh current explicit unknown/revoked/not-trusted exact key/profile -> `RECOVERY_GRANT_AUTHENTICATION_FAILED`.

Earlier successful signature/PREPARE is not trust escrow; current trust is revalidated at the authority-changing boundary.

## 14. Bounded residual revocation model

The `<=5s` evidence policy is bounded-staleness, not an instantaneous cross-repository revocation fence.

If a source revocation occurs after the latest accepted authenticated observation and before a newer revision is observable, prior evidence may remain usable only while:

- conservative source age remains <=5s; and
- no newer comparable restrictive revision has been accepted.

Authority ends at the earlier of a newer accepted restriction or inability to prove the prior evidence remains within the source-age ceiling. Cache refresh cannot extend the window.

## 15. Ownership-safe game-domain resolution

After successful credential/security validation, recovery resolves current state in this order:

1. current `AccountId -> CharacterId` ownership/lifecycle;
2. only if ownership is valid, current `CharacterId -> WorldId` / world eligibility against signed `world_id`;
3. current actor/session/controller placement and runtime authority.

Invalid ownership fails without using world/actor/controller state as an oracle.

Valid ownership + stale/mismatched world -> `RECOVERY_GRANT_WORLD_STALE`; no nonce/authority mutation, no retarget to another world, require newly authorized recovery attempt for current state.

## 16. Independent authoritative revisions

Recovery binds and validates independently:

```text
protocol_major
transport_profile
ruleset_revision
content_revision
map_revision
world_policy_revision
```

For v1, `protocol_major=1` and `transport_profile=1` are currently supported exact values; a correctly authenticated structurally valid unsupported value is `RECOVERY_GRANT_REVISION_UNSUPPORTED`, not malformed input.

Every gameplay revision is compared independently against the current actor/session/runtime transition boundary. A change to any one dimension invalidates the older grant even when the others remain unchanged.

No silent downgrade, retarget or mixed authoritative state. Unsupported/mismatched dimension -> `RECOVERY_GRANT_REVISION_UNSUPPORTED`, no RecoveryGrantNonce consumption or authority mutation.

## 17. Ordered recovery dispatch

After ownership-safe state resolution:

1. healthy current playable controller -> `RECOVERY_HEALTHY_CONTROLLER_PRESENT`;
2. existing same GameSession is `RECONNECTABLE` and same-session grace valid -> same-session recovery through FND-04B PREPARE/COMMIT;
3. prior GameSession terminal and same actor remains `PRESENT_UNCONTROLLED` -> post-grace existing-actor attachment with a new GameSession;
4. otherwise -> `RECOVERY_TARGET_NOT_ELIGIBLE`.

Healthy-controller conflict is not hidden behind generic no-target fallback.

## 18. Same-GameSession recovery COMMIT

When the grant substitutes for a missing reconnect proof, PREPARE is not authorization escrow.

COMMIT atomically revalidates before any authority change:

- prepared transition current/unexpired and exact transport/session/attempt binding;
- current predecessor generation/session/grace/healthy-controller state;
- recovery JWT time;
- fixed-scope key/profile trust evidence source age/order/current decision;
- Platform-security evidence source age/order/current decision/generation;
- RecoveryGrantNonce eligibility;
- each independent recovery revision;
- AccountId->CharacterId ownership first;
- CharacterId->WorldId/world eligibility second;
- AccountPresenceClaim, CharacterLease, runtime ownership/placement and FND-02 reconciliation state;
- no newer handoff/takeover/fence/terminal authority.

Only successful COMMIT consumes RecoveryGrantNonce and establishes the new current transport generation/reconnect proof.

A failed candidate preserves whatever authority is actually current and never revives PREPARE-time state.

## 19. Post-grace existing-actor commit

A terminal GameSession cannot revive. Post-grace recovery may create a new GameSession only for the same still-authoritative `PRESENT_UNCONTROLLED` actor.

Immediately before and atomically with new authority creation revalidate all Section 18 credential/security/ownership/revision facts plus:

- prior GameSession remains terminal;
- actor remains `PRESENT_UNCONTROLLED`;
- no current playable controller exists;
- AccountPresenceClaim/CharacterLease/runtime owner still own that actor;
- current actor snapshot/new-session reconciliation boundary is safe.

Success creates a fresh GameSession/connection-generation namespace and new reconnect proof without respawn/teleport/heal/resource refill/state reset. RecoveryGrantNonce is consumed in the same atomic boundary.

## 20. Error outcomes owned by profile

| Code | Category | Progression | Retry authority | Mutation | Public class | Redacted diagnostic |
|---|---|---|---|---|---|---|
| `RECOVERY_GRANT_MALFORMED` | `INVALID_INPUT` | `TERMINAL` | new valid grant | none | `RETRY_LOGIN` | `recovery grant malformed` |
| `RECOVERY_GRANT_AUTHENTICATION_FAILED` | `AUTHENTICATION_FAILED` | `SECURITY_TERMINAL` | fresh authenticated recovery | none | `AUTHENTICATION_REQUIRED` | `recovery credential authentication failed` |
| `RECOVERY_GRANT_BINDING_MISMATCH` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | new correct-bound grant | none | `RETRY_LOGIN` | `recovery credential bound to a different context` |
| `RECOVERY_GRANT_NOT_YET_VALID` | `SESSION_REJECTED` | `RETRYABLE` while same unconsumed credential/current state remains valid | wait only to accepted nbf boundary | none | `TEMPORARILY_UNAVAILABLE` | `recovery grant not yet active` |
| `RECOVERY_GRANT_EXPIRED` | `SESSION_REJECTED` | `TERMINAL` | fresh recovery attempt | none | `RETRY_LOGIN` | `recovery grant expired` |
| `RECOVERY_GRANT_REPLAYED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | reconcile prior result; never reuse | prior success may exist; no duplicate | `SESSION_UNAVAILABLE` | `recovery grant already consumed or replayed` |
| `RECOVERY_ATTEMPT_RECONCILIATION_REQUIRED` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same attempt_ref reconciliation | issuance ambiguity only | `TEMPORARILY_UNAVAILABLE` | `recovery issuance outcome requires reconciliation` |
| `RECOVERY_GRANT_SECURITY_STATE_REVOKED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | new recovery after security permits | none | `AUTHENTICATION_REQUIRED` | `recovery denied by current account security state` |
| `RECOVERY_GRANT_SECURITY_EVIDENCE_STALE` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` while token/target valid | fresh authenticated non-rollback evidence | no nonce/authority mutation | `TEMPORARILY_UNAVAILABLE` | `recovery security evidence unavailable, stale or superseded` |
| `RECOVERY_GRANT_WORLD_STALE` | `STALE_GENERATION` | `TERMINAL` | current world resolution + new recovery grant | no nonce/authority mutation | `RETRY_LOGIN` | `recovery character world binding no longer matches` |
| `RECOVERY_GRANT_REVISION_UNSUPPORTED` | `UNSUPPORTED_REVISION` | `TERMINAL` | compatible producer/consumer revision; no downgrade | none | `CLIENT_UPDATE_REQUIRED` | `recovery authoritative revision unsupported` |
| `RECOVERY_HEALTHY_CONTROLLER_PRESENT` | `CONFLICT` | `TERMINAL` | incumbent remains authority | none | `CHARACTER_ALREADY_ACTIVE` | `recovery blocked by current playable controller` |
| `RECOVERY_TARGET_NOT_ELIGIBLE` | `SESSION_REJECTED` | `TERMINAL` | resolve current actor/session flow | none | `SESSION_UNAVAILABLE` | `recovery target is not eligible` |

Correlation is credential-free. Before authentication, diagnostics never reveal token semantic binding/profile/schema/world/actor/controller match state.

## 21. Required implementation fixtures

### Crypto/schema/binding/revision

- canonical Ed25519 positive;
- malformed vs well-formed non-exact alg precedence;
- malformed vs unknown/untrusted kid precedence;
- forbidden/extra protected members and token-directed key discovery;
- invalid signature + wrong binding/profile/schema/revision -> authentication failure;
- authenticated structural schema defect -> malformed;
- authenticated wrong iss/aud/typ/purpose -> binding mismatch;
- authenticated unsupported profile/protocol/transport -> revision unsupported;
- each ruleset/content/map/world-policy revision changed independently -> revision unsupported;
- nbf/expiry/lifetime/skew boundaries.

### Security evidence

Independently for Platform-security and recovery-key/profile trust:

- authenticated source provenance + comparable revision required;
- cache refresh/re-read does not reset age;
- source-age upper bound including clock uncertainty <=5s;
- older still-<5s allow/trust cannot roll back newer deny/revoke;
- equal revision contradictory content fails closed;
- restart without provable current non-rollback floor cannot authorize;
- revocation after earlier validation but before authority commit fails before nonce/authority mutation.

### Ownership/transition

- invalid ownership before world/actor/controller classification;
- valid ownership + stale world -> world stale/no retarget;
- healthy controller -> dedicated conflict;
- same-session recovery PREPARE then current facts change -> COMMIT rejects;
- post-grace actor becomes ABSENT or controlled before commit -> reject;
- concurrent nonce use -> one success maximum;
- lost successful response -> reconciliation never duplicates transition.

## 22. Non-authorization

This profile implements nothing. It authorizes no Platform issuer/verifier code, Rust runtime, consume store, recovery locator, persistence schema, KMS/HSM, production key, deployment or traffic.
