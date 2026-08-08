# FND-04 Reauthenticated Recovery Grant Security / Interchange Profile v1

- Status: Candidate normative cross-repository security profile; canonical when the owning FND-04 delivery merges
- Profile ID: `oteryn-reauth-recovery-v1`
- Applies to: Platform-reauthenticated attempts to recover control of an already-existing authoritative CharacterId actor/GameSession state
- Does not apply to: fresh actor admission, OAuth/web session, Game Login Ticket, fast reconnect-secret proof, Channel/Instance handoff or Canary admission
- Cryptographic container: JWS Compact Serialization carrying a JWT claims set
- Signature profile: Ed25519 via JOSE `alg = EdDSA`
- Validation guidance: RFC 7515, RFC 7519, RFC 8037 and RFC 8725
- Does not select: implementation library, KMS/HSM/vendor, recovery-locator transport, persistence/cache schema or deployment

## 1. Purpose and authority

This profile exists so loss of the game-domain reconnect secret does not force a fresh-entry credential to be misused as a recovery credential.

The capability proves only:

```text
Platform has freshly authenticated AccountId
AND Platform currently permits AccountId to attempt recovery of CharacterId
```

It does **not** prove:

- a GameSession exists;
- the actor is reconnectable;
- the actor is currently uncontrolled;
- current ChannelId/InstanceId placement;
- current CharacterLease/runtime ownership;
- permission to move/respawn/recreate the actor.

Oteryn-v2 resolves all current game-domain facts and may reject the recovery attempt.

## 2. Separation from fresh-entry profile

Fresh entry uses:

```text
typ     = oteryn-admission+jwt
profile = oteryn-pre-admission-v1
purpose = fresh_entry
aud     = urn:oteryn:game:admission
```

Reauthenticated recovery uses:

```text
typ     = oteryn-recovery+jwt
profile = oteryn-reauth-recovery-v1
purpose = existing_actor_recovery
aud     = urn:oteryn:game:recovery
```

Validation rules are mutually exclusive. A consumer MUST NOT reinterpret one profile as the other when validation fails.

A fresh-entry grant bound to a Channel/route cannot be used to move an existing actor. A recovery grant intentionally contains no ChannelId or runtime placement claim.

## 3. Cryptographic profile

Recovery v1 uses JWS Compact JWT with exactly:

```text
alg = EdDSA
key subtype = Ed25519
```

Only Ed25519 is accepted.

The protected header MUST contain exactly:

```json
{
  "alg": "EdDSA",
  "kid": "<trusted-recovery-key-id>",
  "typ": "oteryn-recovery+jwt"
}
```

Rules match the fresh-entry v1 strictness:

- `kid` 1..64 ASCII characters matching `[A-Za-z0-9._-]+`;
- key selected only from the trusted recovery-key purpose/set;
- reject `none`, alternate algorithms/curves and algorithm fallback;
- reject `jku`, `x5u`, `x5c`, embedded `jwk`, `crit`, `cty`, `zip`, `b64=false` and any additional header member;
- never fetch a verifier key from a URI supplied by the token.

Recovery signing/verification key purpose is distinct from fresh-entry admission, OAuth, Game Login Ticket and service-authentication purposes. Physical key reuse across purposes is not assumed or required; production security operations choose key material under the accepted purpose separation.

## 4. Issuer and audience

Profile v1 freezes:

```text
iss = urn:oteryn:platform:game-recovery
aud = urn:oteryn:game:recovery
```

Both are exact case-sensitive strings.

## 5. Required claims

### 5.1 Standard claims

| Claim | Type | Rule |
|---|---|---|
| `iss` | string | exact Section 4 issuer |
| `aud` | string | exact single Section 4 audience; arrays rejected in v1 |
| `iat` | integer JSON number | whole-second NumericDate, authoritative Platform time |
| `nbf` | integer JSON number | whole-second NumericDate, within one second of `iat` |
| `exp` | integer JSON number | `> iat`, with `exp - iat <= 30` seconds |
| `jti` | string | RecoveryGrantNonce: 32 random bytes, base64url unpadded, exactly 43 characters |

### 5.2 Private claims

| Claim | Type | Rule |
|---|---|---|
| `profile` | string | exact `oteryn-reauth-recovery-v1` |
| `purpose` | string | exact `existing_actor_recovery` |
| `attempt_ref` | string | Platform recovery-attempt correlation reference; canonical lowercase UUIDv7 text |
| `account_id` | string | canonical lowercase UUID text, non-nil |
| `character_id` | string | canonical lowercase UUID text, non-nil |
| `world_id` | string | canonical lowercase UUID text, non-nil |
| `account_security_generation` | string | decimal non-zero uint64 string |
| `protocol_major` | integer JSON number | exact `1` |
| `compatibility_revision` | string | bounded ASCII 1..64, `[A-Za-z0-9._:-]+` |

Profile v1 MUST NOT contain `channel_id`, `instance_id`, NodeId, runtime owner identity or scope ownership generation as an authority claim. Current actor/session placement is resolved by Oteryn-v2 after credential validation.

Unknown unregistered claims are rejected by v1.

## 6. Size, parsing and time limits

Security/parser ceilings are the same class as fresh admission:

- compact token <= 4096 ASCII bytes;
- exactly 3 JWS segments;
- decoded header <= 512 bytes;
- decoded payload <= 3072 bytes;
- JSON nesting depth <= 2;
- duplicate keys reject;
- invalid UTF-8/non-canonical base64url/padded segments reject;
- no floating/exponent/fractional NumericDate;
- required null/missing claim rejects;
- maximum lifetime 30 seconds;
- maximum verifier clock skew 5 seconds.

A producer may issue a shorter token.

## 7. Platform account-security freshness

Recovery is a higher-risk action than ordinary fast reconnect because a fresh Platform authentication may substitute for missing game-domain reconnect proof.

Therefore the same required current Platform-security projection/revocation boundary applies as fresh admission:

```text
required Platform-security projection age <= 5 seconds
```

If the projection is stale/unavailable/unauthenticated/contradictory, recovery fails closed.

If the account is disabled/revoked or the token's `account_security_generation` is below the minimum/current accepted generation, recovery is rejected even while the JWT is otherwise valid.

Platform may additionally require MFA/step-up/risk policy before issuing this recovery profile. That producer policy does not transfer final game authority to Platform.

## 8. RecoveryGrantNonce and producer attempt

`jti` is a one-time RecoveryGrantNonce, distinct from `attempt_ref`.

Requirements:

- game-domain consume keyed by trusted issuer/profile/jti;
- at most one successful recovery authority transition from one jti;
- concurrent use has at most one winner;
- consumed jti remains non-reusable after a lost response;
- consume evidence retained at least through `exp + 5 seconds` and longer when DUR/reconciliation requires it.

`attempt_ref` provides Platform producer idempotency/correlation only and is not GameSessionId, connection_generation or replay authority.

An ambiguous Platform recovery-grant issuance follows the same producer rule as fresh admission: same logical attempt cannot mint multiple independently usable recovery grants.

## 9. Game-domain recovery resolution

After cryptographic/profile/security validation, the recovery boundary resolves current game-domain state by `AccountId + CharacterId + WorldId`.

The token is eligible only when Oteryn-v2 proves one of two states:

### 9.1 Same-GameSession recovery

- an existing GameSession for the CharacterId is in an accepted unexpected-loss `RECONNECTABLE` state;
- the session remains inside its 15-second same-session grace;
- no current healthy playable controller has authority;
- AccountId still owns CharacterId;
- AccountPresenceClaim still binds the same CharacterId;
- CharacterLease/runtime authority is current and compatible;
- current placement is resolved by game-domain authority;
- FND-02 command/session reconciliation state is safe.

Successful recovery preserves GameSessionId and uses the same rebind prepare/commit mechanism as reconnect-secret recovery, except this consumed recovery grant is the authentication proof that authorizes preparing the replacement transport.

### 9.2 Post-grace existing-actor attachment

- prior GameSession is terminal;
- the same authoritative CharacterId actor remains `PRESENT_UNCONTROLLED` because gameplay rules require world presence;
- no current playable controller exists;
- AccountPresenceClaim remains the same CharacterId;
- AccountId still owns CharacterId;
- current CharacterLease/runtime actor is valid and current placement is resolved by game-domain authority.

Successful recovery creates a **new GameSessionId**, starts its own `connection_generation = 1`, creates new reconnect-proof state and attaches to the existing actor without respawn/reset/teleport/heal.

If neither state exists, the recovery grant is rejected. It does not turn into fresh-entry authority automatically.

## 10. Healthy incumbent and takeover safety

A valid recovery JWT does not preempt a healthy current controller.

If the incumbent session/transport has current sufficient-control evidence, recovery is rejected with a coarse conflict/session response.

Intentional takeover of a healthy logout-eligible incumbent follows the separately accepted takeover state machine; it does not use this recovery grant as an unconditional fence.

A healthy combat/PZ/logout-locked incumbent cannot be kicked by presenting this grant.

## 11. Current-placement routing

Because this profile intentionally contains no ChannelId/InstanceId authority, the client/Gateway cannot choose the current actor owner from stale client memory.

The final implementation must provide a bounded authenticated game-domain recovery locator/dispatcher that:

- resolves the current authoritative actor/session owner;
- follows current scope ownership generations;
- does not expose private topology/fencing detail to the client unnecessarily;
- routes or proxies the recovery attempt to the current owner;
- fails closed on ambiguous/suspected/unavailable ownership;
- never uses Platform configured route state as proof of current actor placement.

Exact API/transport/topology is deferred.

## 12. Re-entry protection relationship

Consuming a recovery grant never grants PvE protection by itself.

Protection is keyed to the server-owned control-loss episode:

- inside grace, if this is the first valid re-entry for an eligible classified unexpected-loss episode, FND-03 may activate the accepted 4-second defensive effect;
- routine/healthy takeover does not create it;
- post-grace new GameSession attachment to the same actor can consume the same episode's still-eligible protection decision only once;
- GameSessionId replacement, new JWT or new connection_generation cannot restart/duplicate a consumed protection window.

## 13. Verification order

1. outer protocol/material bound;
2. compact/token parser limits;
3. exact protected-header profile;
4. trusted recovery-key lookup and Ed25519 verification;
5. exact `typ/iss/aud/profile/purpose`;
6. time/lifetime/skew;
7. claim canonical encoding;
8. current Platform account-security projection/revocation/generation;
9. one-time RecoveryGrantNonce eligibility;
10. current AccountId->CharacterId ownership;
11. current actor/session/presence/lease/runtime-placement resolution;
12. healthy-controller/reconnectable/post-grace state decision;
13. atomic game-domain recovery/rebind or new-session attachment commit;
14. consume jti as part of the successful authority transition; publish success only after commit.

No failure creates partial player-control authority.

## 14. Required independent fixtures / fault cases

Before implementation acceptance, prove at least:

- positive canonical recovery JWT across independent Platform/Rust implementations;
- all fresh-entry profile tokens are rejected by recovery validator and vice versa;
- wrong key purpose, alg, typ, issuer, audience, purpose, profile;
- extra/forbidden header and unknown claim;
- over-lifetime/skew/stale Platform-security evidence;
- concurrent one-time jti consumption;
- healthy incumbent cannot be preempted;
- same-session recovery preserves GameSessionId and advances connection generation;
- post-grace actor attachment creates fresh GameSessionId without actor reset;
- stale Platform/client ChannelId does not move the actor;
- actor in InstanceRuntime is located through current game-domain placement;
- GameNode replacement with unreconstructable session state falls back to post-grace/fresh-session rules rather than guessing same-session continuity;
- consumed recovery grant/lost response cannot create a second controller or duplicate protection.

## 15. Non-authorization

This profile does not implement or authorize Platform recovery issuer, recovery locator, Rust session runtime, database/cache schema, protocol message registration, key deployment or production traffic. Those require separate implementation and cross-repository rollout tasks.
