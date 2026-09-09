# WP5 Platform native evidence producer handoff

Coordinator: #162. Source readiness: #319. Programme: #364.

## Status

Read-only-evidence-backed external handoff. **NOT_ACTIVE / PLATFORM WRITE AUTHORITY FORBIDDEN**.

This packet narrows the still-missing Oteryn Platform counterpart required by the
protected Game WP5 native source-ingestion allocation. It is a Game-side
coordination artifact only. It creates no Platform branch, Issue, PR, migration,
route, service, credential, certificate, deployment or production authority.

```yaml
handoff_id: OTV2-WP5-PLATFORM-NATIVE-EVIDENCE-PRODUCER-20260909
repository_written: Oteryn/Oteryn-Game
coordinator_issue: 162
source_issue: 319
programme_issue: 364
game_base_main_sha: 0ca0f6d257d6eb982c4ff9d05bc9a84e44ba48da
platform_evidence_main_sha: 13e207d5e826b2a24513664d7b8b6dc5d918f9b0
parent_game_allocation: "#452 protected"
source_reconciliation: "#461 protected"
platform_write_authority: FORBIDDEN
production_authority: FORBIDDEN
server_seam_authority: FORBIDDEN
implementation_state: MISSING_CROSS_REPOSITORY_PLATFORM_PRODUCER
```

## Accepted counterpart contract — do not reopen

A future separately authorized Platform worker must satisfy the already accepted
Game consumer contracts rather than inventing a new source model:

- `FND-NATIVE-SOURCE-EVIDENCE-V1`;
- `FND-RECOVERY-SOURCE-TRANSPORT-V2`;
- canonical Platform-owned UUIDv7 `AccountId` from Platform ADR 0028;
- fixed Game wire operations delivered by #346/#349:
  - `ReadAccountSecurityV1`;
  - `ReadFreshSigningTrustV1`;
  - `ReadRecoveryAccountSecurityV2`;
  - `ReadRecoverySigningTrustV2`.

The private source is request/response HTTPS with TLS 1.3 mutual service
authentication and strict typed JSON. Endpoint paths are deployment mappings, not
protocol identifiers, so this handoff deliberately does not freeze a permanent URL.
The future Platform owner must select fixed versioned private route mapping and
publish it through the independently authenticated service descriptor process.

For account observations, success returns the exact requested canonical
`AccountId`/purpose/scope plus `allowed` and positive
`minimum_valid_generation`. For signing-trust observations, success returns the
exact fixed issuer/profile/key-purpose/key-id binding plus `trusted` and exactly
32 Ed25519 public-key bytes in the accepted wire encoding. Every success also
carries authenticated `source_authority`, positive durable `source_revision`,
`decision_identity`, authoritative observation time and uncertainty.

## Fresh Platform evidence

Read-only Platform protected `main@13e207d5e826b2a24513664d7b8b6dc5d918f9b0`
shows useful security state but not the required native producer.

### Canonical AccountId is accepted architecture, not runtime storage

Platform ADR 0028 requires:

```text
AccountId = strongly typed UUIDv7, full 128 bits
identities.id = local persistence surrogate only
conceptual target: identities.account_id = canonical immutable AccountId
```

The same ADR explicitly leaves the additive column, backfill, uniqueness and
rollout to a separately authorized migration task.

Current `app/Identity/Models/Identity.php` still exposes integer `id` and
`game_auth_generation`; it has no canonical `account_id` property/cast. Default
branch search found no additive native `identities.account_id` UUIDv7 migration.
Therefore Platform cannot currently emit the accepted canonical AccountId by
relabeling `identities.id` or `canary_account_id`.

### Existing private GameAuth surfaces remain Canary-bound

Current `routes/internal.php` exposes only:

```text
POST /internal/v1/game-auth/tickets/redeem
GET  /internal/v1/game-auth/accounts/{canaryAccountId}/login-context
```

`app/GameAuth/Context/GameLoginContextProvider.php` accepts an integer
`canaryAccountId`, reads Canary game data and returns Canary/world compatibility
context. Platform's own native pre-admission review explicitly classifies the
current v1 redeem/login-context payload as Canary compatibility, not proof of a
native AccountId-bearing producer.

No current default-branch implementation was found for any of the four accepted
source operations.

### Existing security mutation serialisation is reusable

`app/Identity/Actions/RevokeIdentityGameAuthorizations.php` already executes
inside a DB transaction, locks the Identity row, increments
`game_auth_generation`, revokes native OAuth generation bindings and records the
security event.

Current callers include password/credential mutation, recovery-key recovery,
MFA reset/disable, email-change confirmation/recovery and termination flows.
This is the strongest existing candidate serialization seam for account-security
source ordering. A future producer should extend/compose with this owning seam,
not create an independent revocation counter that can race it.

### Signing-trust source is genuinely missing

Default-branch search found no durable Game-admission/recovery signing-trust
registry carrying the accepted `(issuer, profile, key_purpose, key_id)` state,
32-byte Ed25519 public key, rotation/revocation ordering and retained source
high-water. Existing Laravel Passport public/private keys serve OAuth and are not
evidence that the accepted Game signing-trust profile exists; they must not be
silently reused as the WP5 trust authority.

## Minimal future Platform work packets

These are bounded **requested allocations**, not active Platform leases. Exact
Platform ownership must be re-read and granted in that repository before any
mutation.

### P1 — canonical native AccountId substrate

Purpose: make ADR 0028 runtime-real without re-keying Platform foreign keys.

Minimum required surfaces:

```text
database/migrations/<NEXT>_add_native_account_id_to_identities.php   # new
app/Identity/Models/Identity.php                                     # existing
app/Identity/AccountId/CanonicalAccountId.php                        # new, or equivalently bounded owning value/service
app/Identity/AccountId/BackfillCanonicalAccountIds.php               # new, if backfill is not migration-local
tests/Feature/Identity/CanonicalAccountIdTest.php                    # new
```

Required semantics:

- UUIDv7 full 128-bit canonical lower-case representation;
- non-null, unique and immutable after migration completion;
- additive/backward-compatible rollout while integer `identities.id` remains the
  local surrogate;
- deterministic one-time backfill for every existing Identity, with collision
  failure closed and no derivation from local/Canary IDs;
- mixed-version reads fail closed rather than minting or reconstructing AccountId;
- rollback does not silently drop or remint already issued canonical identities.

If current Platform migration conventions require a different exact new filename,
the future Platform coordinator may select it only after fresh migration-number and
path-ownership readback. The semantic owned surface above remains bounded to
Identity AccountId introduction.

### P2 — durable ordered account-security observation source

Purpose: implement `ReadAccountSecurityV1` and
`ReadRecoveryAccountSecurityV2` from current Platform-owned security truth.

Prospective new owner namespace:

```text
app/GameAuth/NativeEvidence/**                                      # new bounded producer domain
```

Necessary existing integration surfaces:

```text
app/Identity/Actions/RevokeIdentityGameAuthorizations.php          # serialized mutation hook
app/Identity/Models/Identity.php                                   # read-only/current truth except P1 change
```

Necessary durable schema responsibility:

```text
database/migrations/<NEXT>_create_native_game_evidence_state.php   # new, exact name selected after fresh readback
```

The physical schema may be normalized differently, but must durably preserve at
least the account-security source revision/high-water and immutable decision replay
needed to prove:

- monotonic positive source revision per AccountId/security-purpose namespace;
- exact equal-revision replay with original observed time/uncertainty;
- no local re-aging;
- current `game_auth_generation` reflected as the minimum valid generation;
- disable/termination/revoke and every security mutation that invalidates game
  authorization serialized into source ordering;
- overflow/rollback/unknown restored high-water fail closed;
- not-found/unavailable/unauthorized/unsupported remain closed failures and do not
  manufacture allow state.

A new observation is one Platform-owned transaction: protect current subject state,
read current security truth and authoritative time, allocate the next durable
revision, persist the immutable decision, then return it. Success must never be
sent before the durable decision exists.

### P3 — durable signing-trust observation source

Purpose: implement `ReadFreshSigningTrustV1` and
`ReadRecoverySigningTrustV2` without conflating OAuth/Passport trust.

Prospective bounded owner namespace:

```text
app/GameAuth/NativeEvidence/**                                      # same producer domain as P2
database/migrations/<NEXT>_create_native_game_signing_trust.php    # new, if not included in P2 state migration
```

Required state/ordering:

- exact fixed issuer/profile/key-purpose sets from the accepted Game request;
- bounded key IDs and exactly 32-byte Ed25519 public keys;
- explicit trusted/untrusted state;
- durable profile/set-wide monotonic revision shared across key IDs so rotation to
  another key ID cannot evade a newer profile revocation;
- key/profile rotation, revoke and trust mutations serialized with observation
  creation;
- retained high-water/bootstrap evidence sufficient to detect producer restore or
  namespace rollback;
- no private signing key need be exposed to or stored by the evidence endpoint.

The owner of actual signing-key generation/custody, if separate, remains outside
this handoff. The evidence producer needs only the authoritative public trust state
and its ordered mutation history.

### P4 — private authenticated producer API

Purpose: expose the four closed operations after P1-P3 are durable.

Conditional existing surfaces:

```text
routes/internal.php
app/Http/Middleware/GameAuth/**
```

Prospective new surfaces:

```text
app/Http/Controllers/GameAuth/NativeEvidenceController.php          # new or split into closed operation controllers
tests/Feature/GameAuth/NativeEvidenceSourceApiTest.php              # new
```

A future Platform owner may split controllers/routes by operation if current
routing/security ownership makes that safer. The API must nevertheless preserve:

- fixed version/operation/request binding;
- exact AccountId or trust-scope equality in the response;
- strict unknown/duplicate/wrong-type rejection;
- bounded body/identifier/work/concurrency behavior compatible with the accepted
  Game `NSRC-*` envelope;
- independent service/client authentication suitable for the Game mTLS descriptor;
- authorization of the Game client for the exact observation purpose;
- no redirects/discovery/bearer-grant trust substitution;
- sensitive response no-cache behavior;
- failure bodies limited to the closed accepted failure family.

Existing `RequireGatewayServiceCredential` is evidence of a private-service auth
pattern, not proof that it satisfies the accepted mutual-TLS peer/descriptor
contract. Reuse requires explicit qualification; do not downgrade the Game source
contract to fit the existing bearer/service-credential middleware.

### P5 — producer qualification and cross-repository compatibility

Before the Platform counterpart may satisfy WP5 S3, its exact candidate must prove:

- AccountId UUIDv7 lossless cross-language encoding and rejection of integer/
  Canary substitutes;
- current allow/deny and generation behavior under concurrent revoke/security
  mutations;
- trust/key rotation and revocation ordering across key IDs;
- equal-revision immutable replay and non-reaging;
- source restart and restored-backup high-water behavior;
- wrong operation/account/key/purpose/profile/issuer negatives;
- malformed/duplicate/oversized input and numeric overflow negatives;
- authenticated mTLS interoperability with the separately qualified Game S1
  client or a controlled compatible counterpart;
- exact golden request/response compatibility with Game #346/#349 wire semantics;
- canonical Platform review/CI/protection/readback under that repository's rules.

Production endpoints, roots, certificates, private keys and deployment remain a
separate authority gate after code compatibility is protected.

## Ordering and dependency graph

```text
separate Platform authority + fresh Platform ownership readback
-> P1 canonical AccountId substrate
-> P2 account-security ordered source + mutation serialization
-> P3 signing-trust ordered source
-> P4 private authenticated API and closed operation binding
-> Platform exact-head security review + canonical CI/protected integration
-> authenticated producer-side qualification
-> Game S1 protected
-> Game S2 protected
-> fresh cross-repository golden + authenticated interoperability evidence
-> only then S3 composition/readiness may consume the producer
```

P2/P3 implementation may share one transactional source-state schema if the future
Platform design proves the required independent namespaces and rollback semantics.
They must not share a revision namespace merely to reduce schema work when the
accepted Game contract scopes ordering differently.

## Explicit exclusions

This handoff does **not** authorize:

- any write to `Oteryn/Oteryn-Platform`;
- changes to #351/#356, #335 or Server Seam #247;
- Game runtime/Cargo/Foundation/composition mutation;
- production endpoint, PKI root, certificate or key selection;
- direct Platform DB reads by Game;
- promotion of Canary `accounts.id`, Platform `identities.id`, OAuth tokens,
  Passport keys, Game Login Tickets, directory/catalogue data or test fixtures into
  native source authority;
- weakening the accepted five-second freshness rule or any `NSRC-*` resource
  ceiling;
- claiming producer availability before separately authorized Platform code is
  protected and interoperability is proven.

## Current disposition

`MISSING_CROSS_REPOSITORY_PLATFORM_PRODUCER` remains true.

The missing work is now bounded to a Platform-owned additive AccountId substrate,
ordered account-security source, ordered signing-trust source, private authenticated
API and producer qualification. Game WP5 may use this packet to avoid rediscovering
legacy Canary/OAuth surfaces, but material Platform implementation must begin only
under separate explicit repository authority and fresh live ownership/custody.

`PLATFORM_WRITE_AUTHORITY: FORBIDDEN`
`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
