# FND-04 Reauthenticated Recovery Grant Security / Interchange Profile v1

- Status: Candidate normative cross-repository security profile; canonical when the owning FND-04 delivery merges
- Profile ID: `oteryn-reauth-recovery-v1`
- Applies to: Platform-reauthenticated attempts to recover control of an already-existing authoritative CharacterId actor/GameSession state
- Does not apply to: fresh actor admission, OAuth/web session, Game Login Ticket, fast reconnect-secret proof, Channel/Instance handoff or Canary admission
- Cryptographic container: JWS Compact Serialization carrying a JWT claims set
- Signature profile: fully specified JOSE `alg = Ed25519`
- Standards baseline: RFC 7515, RFC 7519, RFC 8032, RFC 8037, RFC 8725 and RFC 9864
- Does not select: implementation library, KMS/HSM/vendor, recovery-locator transport, persistence/cache schema or deployment

## 1. Purpose and authority

This profile exists so loss of the game-domain reconnect secret does not force a fresh-entry credential to be misused as a recovery credential.

It proves only:

```text
Platform freshly authenticated AccountId
AND Platform currently permits AccountId to attempt recovery of CharacterId
```

It does not prove GameSession existence, reconnectability, current actor placement, CharacterLease/runtime ownership or permission to move/respawn/recreate the actor. Oteryn-v2 resolves those facts.

## 2. Mutually exclusive profile

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

Validators are mutually exclusive. Failure under one profile never triggers reinterpretation as the other.

A fresh-entry Channel-bound grant cannot move an existing actor. Recovery intentionally contains no ChannelId/InstanceId authority.

## 3. Exact cryptographic/header profile

Recovery v1 uses JWS Compact JWT with only the fully specified JOSE `alg = Ed25519` from RFC 9864.

The protected header MUST contain exactly:

```json
{
  "alg": "Ed25519",
  "kid": "<trusted-recovery-key-id>",
  "typ": "oteryn-recovery+jwt"
}
```

Rules:

- only `alg = Ed25519`;
- deprecated polymorphic `alg = EdDSA` is rejected;
- `kid` is 1..64 ASCII matching `[A-Za-z0-9._-]+` and selects only from the trusted recovery-key set;
- reject `none`, other algorithms/curves and algorithm fallback;
- reject `jku`, `x5u`, `x5c`, embedded `jwk`, `crit`, `cty`, `zip`, `b64=false` and all extra header members;
- never fetch verification keys from token-supplied URIs.

If trusted key distribution uses JWK, the public key follows the JOSE OKP/Ed25519 representation; the token `alg` remains `Ed25519`.

Recovery key purpose is distinct from fresh-entry, OAuth, Game Login Ticket and service-authentication purposes.

## 4. Issuer and audience

```text
iss = urn:oteryn:platform:game-recovery
aud = urn:oteryn:game:recovery
```

Exact case-sensitive matching is mandatory.

## 5. Required claims

Unknown/unregistered claims are rejected in v1.

### Standard claims

| Claim | Type | Rule |
|---|---|---|
| `iss` | string | exact Section 4 issuer |
| `aud` | string | exact single Section 4 audience; arrays rejected |
| `iat` | integer JSON number | whole-second NumericDate, authoritative Platform time |
| `nbf` | integer JSON number | `iat - 1 <= nbf <= iat + 1` |
| `exp` | integer JSON number | `exp > iat`, `exp - iat <= 30` seconds |
| `jti` | string | RecoveryGrantNonce: 32 random bytes, base64url unpadded, exactly 43 chars |

### Oteryn claims

| Claim | Type | Rule |
|---|---|---|
| `profile` | string | exact `oteryn-reauth-recovery-v1` |
| `purpose` | string | exact `existing_actor_recovery` |
| `attempt_ref` | string | Platform recovery-attempt correlation reference; canonical lowercase RFC UUIDv7 text |
| `account_id` | string | canonical lowercase non-nil UUID in the authoritative Platform representation accepted by FND-ID-01 |
| `character_id` | string | canonical lowercase non-nil RFC UUIDv7 text |
| `world_id` | string | canonical lowercase non-nil RFC UUIDv7 text |
| `account_security_generation` | string | decimal non-zero uint64 string |
| `protocol_major` | integer JSON number | exact `1` |
| `compatibility_revision` | string | bounded ASCII 1..64, `[A-Za-z0-9._:-]+`; syntactic validity alone is insufficient and Section 9 requires current game-domain compatibility validation |

All UUID claims MUST parse and round-trip to the exact canonical lowercase hyphenated form. Nil UUID is rejected.

`attempt_ref`, `character_id` and `world_id` additionally MUST encode UUID version `7` and the RFC UUID variant; a syntactically canonical UUIDv1/v4/v6, Microsoft-reserved variant or other non-v7/non-RFC value is rejected. `account_id` remains Platform-owned and is validated against the authoritative Platform representation accepted by FND-ID-01 rather than being silently redefined as an Oteryn-issued UUIDv7.

This profile MUST NOT contain `channel_id`, `instance_id`, NodeId, runtime owner identity or scope ownership generation as placement authority. Oteryn-v2 resolves current actor/session placement after credential validation.

## 6. Parser and time policy

Parser/material ceilings:

- compact token <= 4096 ASCII bytes;
- exactly 3 JWS segments;
- decoded header <= 512 bytes;
- decoded payload <= 3072 bytes;
- JSON nesting depth <= 2;
- duplicate JSON members reject;
- invalid UTF-8/non-canonical base64url/padding reject;
- fractional/exponent NumericDate reject;
- required null/missing claim reject;
- no decompression.

Security time ceilings:

```text
maximum grant lifetime: 30 seconds from iat to exp
maximum verifier clock-skew allowance: 5 seconds
```

A producer may issue a shorter lifetime. A consumer MUST reject a declared lifetime above 30 seconds.

At trusted server time `now`, recovery v1 uses the same explicit skew equations as the fresh-entry profile:

```text
now + 5s >= nbf
now - 5s < exp
exp > iat
exp - iat <= 30s
abs(iat - now) <= 35s as structural sanity bound
```

Therefore the accepted `nbf` window begins when `now + 5s >= nbf`; validators do not wait for literal `now >= nbf`. Before that boundary the credential maps to `RECOVERY_GRANT_NOT_YET_VALID`. Client clocks never affect validity.

## 7. Platform security freshness

Recovery is higher-risk than ordinary reconnect-secret proof because fresh Platform authentication substitutes for missing game-domain proof.

Required Platform-security evidence age:

```text
<= 5 seconds
```

If evidence is stale/unavailable/unauthenticated/contradictory, recovery fails closed.

Reject if account is disabled/revoked or token `account_security_generation` is below current minimum-valid generation.

Platform may require MFA/step-up/risk checks before issuing this profile. That policy does not transfer final game authority to Platform.

## 8. RecoveryGrantNonce and producer attempt

`jti` is the one-time RecoveryGrantNonce, distinct from `attempt_ref`.

- game consume state keyed by trusted issuer/profile/jti;
- at most one successful recovery authority transition per jti;
- concurrent use has at most one winner;
- consumed jti stays consumed after lost response;
- replay evidence retained at least through `exp + 5 seconds` and longer when DUR/reconciliation requires.

`attempt_ref` is producer idempotency/correlation only. Ambiguous producer retry for the same logical attempt cannot mint multiple independently usable recovery grants.

## 9. Current game-domain recovery resolution and compatibility

After cryptographic/security validation, the game resolves state by AccountId + CharacterId + WorldId.

`compatibility_revision` is a signed compatibility requirement, not descriptive metadata. Before either recovery transition may become authoritative, the current Oteryn-v2 owner MUST verify that the token revision is supported by the current protocol-major/runtime/content/ruleset/session boundary needed to continue or recreate player control. A syntactically valid but unsupported, superseded or otherwise incompatible revision fails closed as `RECOVERY_GRANT_REVISION_UNSUPPORTED`, consumes no RecoveryGrantNonce and creates no authority mutation.

The token revision does not select stale content/runtime state and cannot downgrade the current actor/session. Current game-domain state remains authority for whether a compatible recovery boundary exists.

The grant can authorize only one of two game-domain transitions.

### 9.1 Same-GameSession recovery

Require:

- existing session in accepted unexpected-loss `RECONNECTABLE` state;
- still inside same-session 15-second grace;
- no healthy current controller;
- current AccountId->CharacterId ownership;
- AccountPresenceClaim still same CharacterId;
- current CharacterLease/runtime authority;
- current game-domain placement;
- token `compatibility_revision` remains supported for the current GameSession/runtime/content/ruleset and the exact FND-02 reconciliation/snapshot boundary;
- safe FND-02 command/session reconciliation state.

Success preserves GameSessionId and uses the FND-04 reconnect PREPARE/COMMIT state machine; this recovery grant substitutes only for the missing current reconnect-secret authentication proof.

PREPARE is not authorization escrow. If this grant is used to create a prepared rebind, COMMIT MUST atomically revalidate before any authority change that:

- the prepared transition is unexpired and still belongs to the current GameSession/current predecessor generation;
- the recovery JWT is still inside its accepted time window and its one-time nonce remains eligible;
- the current trusted recovery signing key/profile policy still accepts the exact `kid`, issuer, purpose and profile used by this grant; emergency key or profile revocation after PREPARE invalidates COMMIT;
- current trusted Platform-security evidence is fresh and still admits the grant's `account_security_generation`;
- the token `compatibility_revision` is still supported by the current GameSession/runtime/content/ruleset/reconciliation boundary;
- the account/character ownership, AccountPresenceClaim, CharacterLease, runtime ownership/placement and reconciliation state are still current;
- no healthy current controller has regained sufficient current-generation authority;
- the same-session grace remains valid.

If the prepared transition itself has expired while the same-session grace may still be valid, that candidate fails as `RECONNECT_PREPARED_EXPIRED`. It performs no authority mutation and does not consume RecoveryGrantNonce as a successful recovery. A caller may start a new PREPARE only after re-evaluating the still-current session/loss/grace/authority state and using currently valid proof; `RECONNECT_GRACE_EXPIRED` remains distinct and forbids same-session retry.

If the recovery signing key/profile trust policy no longer accepts the grant, this candidate COMMIT fails before authority mutation as `RECOVERY_GRANT_AUTHENTICATION_FAILED`; RecoveryGrantNonce is not consumed and the current authority state remains unchanged.

If compatibility or any other required condition changed, this candidate COMMIT fails before performing any authority mutation. The prepared candidate is cancelled/terminalized, its successor secret never becomes current proof and its candidate connection generation never becomes current. The failure leaves whatever GameSession/TransportBinding/lease/runtime authority state is actually current at revalidation unchanged; it never revives a PREPARE predecessor that was already fenced, handed off, superseded or made terminal. Compatibility failure maps to `RECOVERY_GRANT_REVISION_UNSUPPORTED`. A caller must reconcile current authority and, when required, obtain a compatible fresh recovery grant; possession of a prepared successor secret never overrides changed authorization.

### 9.2 Post-grace existing-actor attachment

Require:

- prior GameSession terminal;
- same authoritative actor still `PRESENT_UNCONTROLLED`;
- no current playable controller;
- AccountPresenceClaim remains same CharacterId;
- current AccountId->CharacterId ownership;
- current CharacterLease/runtime actor;
- current placement resolved by game-domain authority;
- token `compatibility_revision` is supported by the current actor/runtime/content/ruleset state and by the fresh authoritative snapshot/new-GameSession boundary.

Success creates a **new GameSessionId**, new connection_generation namespace beginning at `1`, new reconnect proof and control attachment to the existing actor without respawn/reset/teleport/heal.

The post-grace transition MUST use one atomic authoritative commit boundary. Immediately before creating/publishing the new GameSession or any replacement control authority, that commit MUST revalidate at minimum:

- the recovery JWT is still inside its accepted time window and RecoveryGrantNonce remains eligible;
- the current trusted recovery signing key/profile policy still accepts the exact `kid`, issuer, purpose and profile used by this grant;
- current trusted Platform-security evidence is fresh and still admits the grant's `account_security_generation`;
- the token `compatibility_revision` is still supported by the current actor/runtime/content/ruleset/snapshot/new-GameSession boundary;
- the account/character ownership, AccountPresenceClaim, CharacterLease, runtime ownership/placement and existing actor state are still current;
- the actor is still `PRESENT_UNCONTROLLED`, no playable controller has become current and no newer fence/handoff/takeover/terminal transition has superseded the candidate.

Validation performed earlier in routing, lookup or recovery resolution is not trust escrow. Emergency recovery-key/profile revocation after earlier validation but before this post-grace commit fails before authority/session mutation as `RECOVERY_GRANT_AUTHENTICATION_FAILED`; RecoveryGrantNonce is not consumed, no new GameSession/lease/runtime/transport authority is committed and whatever authority state is actually current at commit-time revalidation remains unchanged.

If current compatibility cannot be proven, reject as `RECOVERY_GRANT_REVISION_UNSUPPORTED` with no RecoveryGrantNonce consumption or authority mutation. If authoritative state matches neither Section 9.1 nor Section 9.2 — including when the actor has legally become `ABSENT` — reject as `RECOVERY_TARGET_NOT_ELIGIBLE`. That outcome is terminal for this recovery transition, consumes no RecoveryGrantNonce, commits no authority mutation and never reinterprets this recovery grant as fresh-entry authority. A later fresh login, if permitted, is a separate newly authorized fresh-entry flow.

## 10. Healthy incumbent safety

A valid recovery JWT, a reconnect secret, a prepared successor secret or a completed PREPARE alone cannot preempt a healthy current controller.

Healthy combat/PZ/logout-locked incumbent remains authoritative. Intentional logout-eligible takeover uses the separate takeover state machine, not an unconditional recovery-grant/reconnect-secret fence. Any future healthy-session migration requires a separately current-generation-authorized transition and is not implied by this profile.

## 11. Current-placement routing

The client/Gateway does not choose actor placement from stale route memory.

Implementation must provide a bounded authenticated game-domain recovery locator/dispatcher that:

- resolves current actor/session owner and scope ownership generation;
- routes/proxies to the current owner without exposing unnecessary private topology;
- fails closed on ambiguous/suspected/unavailable current ownership;
- never treats Platform configured route as proof of actor placement.

Exact API/transport/deployment remains later design.

## 12. Re-entry protection

Consuming a recovery grant never creates protection by itself.

Protection remains keyed to one server-owned ControlLossEpoch:

- same-session or post-grace first eligible re-entry may consume that epoch's one protection activation;
- healthy/routine takeover does not;
- new JWT, new GameSessionId or new connection_generation cannot restart/duplicate consumed protection.

## 13. Verification order

1. outer protocol/material bound;
2. compact/parser limits;
3. exact protected-header profile;
4. trusted recovery-key lookup + Ed25519 verification;
5. exact typ/iss/aud/profile/purpose;
6. time/lifetime/skew;
7. claim canonical encoding, including UUID version/variant requirements;
8. current Platform-security evidence;
9. current `protocol_major` / `compatibility_revision` support for the current recovery target and required snapshot/reconciliation boundary;
10. one-time RecoveryGrantNonce eligibility;
11. current AccountId->CharacterId ownership;
12. current actor/session/presence/lease/runtime placement;
13. healthy-controller/reconnectable/post-grace decision, rejecting authoritative no-target state as `RECOVERY_TARGET_NOT_ELIGIBLE`;
14. atomic game-domain recovery/rebind/new-session commit, including COMMIT-time current recovery-key/profile trust, token time/nonce, compatibility and authority/security revalidation for both Section 9.1 same-session rebind and Section 9.2 post-grace new-GameSession attachment;
15. publish success only after commit.

No failure creates partial player-control authority.

## 14. Independent fixtures / fault cases

Before implementation acceptance prove:

- canonical `alg=Ed25519` recovery JWT across independent producer/consumer implementations;
- deprecated `alg=EdDSA` rejection;
- fresh-entry token rejected by recovery validator and vice versa;
- wrong key purpose/alg/typ/issuer/audience/purpose/profile;
- forbidden/extra header and unknown claim;
- explicit trusted-time boundaries: `now + 5s < nbf` rejects as `RECOVERY_GRANT_NOT_YET_VALID`, while the first accepted boundary `now + 5s >= nbf` may proceed only if every other recovery/security/session condition remains valid;
- expiry boundary `now - 5s < exp` remains accepted and `now - 5s >= exp` rejects as expired;
- lifetime/skew/stale Platform-security rejection;
- canonical-looking wrong UUID version and wrong UUID variant rejection for `attempt_ref`, `character_id` and `world_id`;
- syntactically valid but unsupported/superseded `compatibility_revision` rejects as `RECOVERY_GRANT_REVISION_UNSUPPORTED`, consumes no RecoveryGrantNonce and creates no authority mutation for both same-session and post-grace recovery;
- compatibility revision/current runtime-content-ruleset-session support changes after PREPARE -> COMMIT rejects before candidate authority switch and maps to `RECOVERY_GRANT_REVISION_UNSUPPORTED`;
- recovery signing key/profile is trusted at PREPARE, then emergency-revoked before same-session COMMIT -> COMMIT rejects as `RECOVERY_GRANT_AUTHENTICATION_FAILED`, consumes no RecoveryGrantNonce, commits no authority mutation and preserves whatever authority state is current at revalidation;
- post-grace lookup/validation accepts a recovery key/profile, then that key/profile is emergency-revoked before the atomic new-GameSession attachment commit -> reject as `RECOVERY_GRANT_AUTHENTICATION_FAILED`, consume no RecoveryGrantNonce, create no new GameSession/control authority and preserve whatever authority state is current at revalidation;
- authoritative actor becomes legally `ABSENT`, or otherwise matches neither same-session nor post-grace recovery target, before recovery commit -> `RECOVERY_TARGET_NOT_ELIGIBLE`, no RecoveryGrantNonce consumption, no authority mutation and no recovery-to-fresh-entry reinterpretation;
- same-session PREPARE's own bounded expiry is reached while grace may still be valid -> `RECONNECT_PREPARED_EXPIRED`, no authority mutation/no successful nonce consumption; a new PREPARE is allowed only after fresh current-state/proof evaluation, while actual grace expiry remains `RECONNECT_GRACE_EXPIRED`;
- another valid fencing/handoff/takeover/terminality transition supersedes the PREPARE predecessor -> stale candidate COMMIT cannot revive predecessor authority or overwrite the authority/no-current-transport state that is current at revalidation;
- concurrent one-time jti consume;
- healthy incumbent cannot be preempted;
- PREPARE followed by incumbent liveness recovery cannot COMMIT/fence that incumbent;
- PREPARE followed by recovery-grant expiry/revocation/security-generation change cannot COMMIT;
- PREPARE followed by lease/runtime/session/reconciliation change cannot COMMIT under stale prepared authority;
- same-session recovery preserves GameSessionId and advances connection generation only after successful revalidation/COMMIT;
- post-grace recovery creates fresh GameSessionId without actor reset;
- stale Platform/client ChannelId does not move actor;
- InstanceRuntime actor is resolved through current game-domain placement;
- unreconstructable same-session state after GameNode replacement falls back safely rather than guessing continuity;
- consumed grant/lost response cannot create a second controller or duplicate protection.

## 15. Non-authorization

This profile does not implement or authorize Platform recovery issuer, recovery locator, Rust session runtime, database/cache schema, protocol message registration, key deployment or production traffic.