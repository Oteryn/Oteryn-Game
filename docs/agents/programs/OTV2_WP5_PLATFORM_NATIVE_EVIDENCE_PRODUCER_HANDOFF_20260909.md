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
generation_representation_gap: UNRESOLVED_REQUIRES_PLATFORM_DECISION
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

The source is request/response HTTPS with TLS 1.3 mutual service authentication
and strict typed JSON. Endpoint paths are deployment mappings, not protocol IDs;
this packet therefore does not freeze a permanent URL. A future Platform owner
must select fixed versioned private route mappings and publish them through the
independently authenticated service-descriptor process.

For account observations, success returns the exact requested canonical
`AccountId`/purpose/scope plus `allowed` and **positive**
`minimum_valid_generation`. For signing-trust observations, success returns the
exact fixed issuer/profile/key-purpose/key-id binding plus `trusted` and exactly
32 Ed25519 public-key bytes in the accepted wire encoding. Every success also
carries authenticated `source_authority`, positive durable `source_revision`,
`decision_identity`, authoritative observation time and uncertainty.

## Fresh Platform evidence

Read-only Platform protected `main@13e207d5e826b2a24513664d7b8b6dc5d918f9b0`
contains useful current security state but not the required native producer.

### Canonical AccountId is accepted architecture, not runtime storage

Platform ADR 0028 requires:

```text
AccountId = strongly typed UUIDv7, full 128 bits
identities.id = local persistence surrogate only
conceptual target: identities.account_id = canonical immutable AccountId
```

The ADR leaves the additive column, backfill, uniqueness and rollout to a
separately authorized migration task. Current
`app/Identity/Models/Identity.php` still exposes integer `id` and
`game_auth_generation`; it has no canonical `account_id` property/cast. Default
branch search found no additive native `identities.account_id` UUIDv7 migration.
Platform therefore cannot satisfy native AccountId by relabeling `identities.id`
or `canary_account_id`.

### Existing private GameAuth surfaces remain Canary-bound

Current `routes/internal.php` exposes only:

```text
POST /internal/v1/game-auth/tickets/redeem
GET  /internal/v1/game-auth/accounts/{canaryAccountId}/login-context
```

`app/GameAuth/Context/GameLoginContextProvider.php` accepts an integer
`canaryAccountId` and reads Canary game data. Platform's own native pre-admission
review explicitly classifies the current v1 redeem/login-context payload as
Canary compatibility, not proof of a native AccountId-bearing producer.

No current default-branch implementation was found for any of the four accepted
source operations.

### Existing security mutation serialization is reusable

`app/Identity/Actions/RevokeIdentityGameAuthorizations.php` already executes
inside a DB transaction, locks the Identity row, increments
`game_auth_generation`, revokes native OAuth generation bindings and records the
security event. Current callers cover credential/password mutation, recovery-key
recovery, MFA reset/disable, email-change confirmation/recovery and termination
flows.

This is the strongest existing candidate serialization seam for account-security
source ordering. A future producer should extend/compose with this owning seam,
not create a separate revocation counter that can race it.

### Generation representation is an explicit unresolved compatibility gap

Current Platform storage initializes `game_auth_generation` to `0`. The protected
Game source wire rejects zero for `minimum_valid_generation`: accepted comparable
generation values are positive `u64`, and the FND-04 consumer rejects a grant when
`grant.account_security_generation < minimum_valid_generation`.

Therefore a future producer **must not** directly copy current zero-valued
`game_auth_generation` into the native response, and it must not apply a one-sided
`+1` translation only in the source response. The Platform native ticket/pre-
admission issuance path and the source producer must share one reviewed native
generation representation so comparison semantics remain sound across issuance,
revocation and recovery.

The separately authorized Platform design must resolve and qualify this before P2
can be ready. Acceptable implementation shapes may include an additive positive
native generation domain or a proven monotonic mapping of the existing fence, but
this Game handoff does not choose one. Any mapping must prove initialization,
revocation ordering, overflow, legacy coexistence and rollback behavior. Changing
the protected Game wire to admit zero requires a separate reviewed Game contract
decision and is not authorized here.

### Signing-trust source is genuinely missing

Default-branch search found no durable Game-admission/recovery signing-trust
registry carrying the accepted `(issuer, profile, key_purpose, key_id)` state,
32-byte Ed25519 public key, rotation/revocation ordering and retained source
high-water. Existing Laravel Passport keys serve OAuth and are not evidence that
the accepted Game signing-trust profile exists; they must not be promoted into WP5
trust authority merely because they are already deployed by Platform.

## Minimal future Platform work packets

These are bounded **requested allocations**, not active Platform leases. Exact
Platform ownership must be refreshed and granted inside that repository before any
mutation.

### P1 — canonical native AccountId substrate

Purpose: make ADR 0028 runtime-real without re-keying Platform foreign keys.

Prospective bounded surfaces:

```text
database/migrations/<NEXT>_add_native_account_id_to_identities.php   # new
app/Identity/Models/Identity.php                                     # existing
app/Identity/AccountId/CanonicalAccountId.php                        # new default handoff path
app/Identity/AccountId/BackfillCanonicalAccountIds.php               # new if backfill is not migration-local
tests/Feature/Identity/CanonicalAccountIdTest.php                    # new
```

Required semantics:

- UUIDv7 full 128-bit canonical lower-case representation;
- additive/backward-compatible rollout while integer `identities.id` remains the
  local surrogate;
- eventually non-null, unique and immutable canonical AccountId for every native
  eligible Identity;
- idempotent one-time backfill with collision failure closed and no derivation from
  local/Canary IDs;
- mixed-version reads fail closed rather than minting/reconstructing AccountId;
- rollback does not silently drop or remint already issued canonical identities.

`<NEXT>` and any alternative new helper filename require fresh Platform migration
and path-ownership readback; those naming choices do not broaden the semantic P1
scope.

### P2 — durable ordered account-security observation source

Purpose: implement `ReadAccountSecurityV1` and
`ReadRecoveryAccountSecurityV2` from Platform-owned security truth.

Prospective new owner namespace:

```text
app/GameAuth/NativeEvidence/**                                      # new bounded producer domain
```

Existing integration surfaces requiring explicit serialized ownership if mutated:

```text
app/Identity/Actions/RevokeIdentityGameAuthorizations.php
app/Identity/Models/Identity.php
```

Prospective durable schema responsibility:

```text
database/migrations/<NEXT>_create_native_game_evidence_state.php
```

The physical schema may be normalized differently, but it must durably preserve
account-security source revision/high-water and immutable decision replay. P2 must
prove:

- monotonic positive source revision per AccountId/security-purpose namespace;
- exact equal-revision replay with original observed time/uncertainty;
- no local re-aging;
- one reviewed **positive native security-generation representation** shared with
  native ticket/pre-admission issuance; raw Platform zero cannot be emitted and a
  source-only offset is forbidden;
- disable/termination/revoke and every security mutation that invalidates game
  authorization serialized into source ordering;
- overflow/rollback/unknown restored high-water fails closed;
- not-found/unavailable/unauthorized/unsupported never manufactures allow state.

A new observation is one Platform-owned transaction: protect current subject state,
read current security truth and authoritative time, allocate the next durable
revision, persist the immutable decision, then return success. Success must never
precede durable decision commit.

### P3 — durable signing-trust observation source

Purpose: implement `ReadFreshSigningTrustV1` and
`ReadRecoverySigningTrustV2` without conflating OAuth/Passport trust.

Prospective bounded surfaces:

```text
app/GameAuth/NativeEvidence/**
database/migrations/<NEXT>_create_native_game_signing_trust.php    # if not coherently included in P2 schema
```

Required state/ordering:

- exact fixed issuer/profile/key-purpose sets from the accepted Game request;
- bounded key IDs and exactly 32-byte Ed25519 public keys;
- explicit trusted/untrusted state;
- durable profile/set-wide monotonic revision shared across key IDs so changing a
  key ID cannot evade a newer profile revocation;
- key/profile rotation, revoke and trust mutations serialized with observations;
- retained high-water/bootstrap evidence sufficient to detect producer restore or
  namespace rollback;
- no private signing key exposed through or required inside the evidence response.

Actual signing-key generation/custody may have a separate owner. P3 needs the
authoritative public trust state and ordered mutation history, not possession of
private signing material.

### P4 — private authenticated producer API

Purpose: expose the four closed operations after the required P1-P3 state exists.

Conditional existing surfaces:

```text
routes/internal.php
app/Http/Middleware/GameAuth/**
```

Prospective new surfaces:

```text
app/Http/Controllers/GameAuth/NativeEvidenceController.php          # or closed split controllers
tests/Feature/GameAuth/NativeEvidenceSourceApiTest.php              # new
```

The future Platform owner may split controllers/routes by operation after fresh
ownership readback. The API must still preserve:

- fixed version/operation/request binding;
- exact AccountId or trust-scope equality in the response;
- strict unknown/duplicate/wrong-type rejection;
- body/identifier/work/concurrency limits compatible with accepted Game `NSRC-*`;
- independent service/client authentication suitable for the Game mTLS descriptor;
- authorization of the Game client for the exact observation purpose;
- no redirects/discovery/bearer-grant trust substitution;
- sensitive-response no-cache behavior;
- closed failure bodies only.

Existing `RequireGatewayServiceCredential` proves a private-service auth pattern,
not accepted mutual-TLS peer/descriptor compatibility. Reuse requires explicit
qualification; the Game source contract must not be weakened to fit the existing
middleware.

### P5 — producer qualification and cross-repository compatibility

The Platform candidate must independently prove:

- UUIDv7 AccountId lossless cross-language encoding and rejection of integer/
  Canary substitutes;
- the resolved positive native generation representation across issuance,
  observation and revocation, including legacy coexistence/overflow;
- current allow/deny behavior under concurrent revoke/security mutation;
- trust/key rotation and revocation ordering across key IDs;
- equal-revision immutable replay and non-reaging;
- source restart/restored-backup high-water behavior;
- wrong operation/account/key/purpose/profile/issuer negatives;
- malformed/duplicate/oversized input and numeric overflow negatives;
- exact golden request/response compatibility with Game #346/#349 wire semantics;
- canonical Platform security review, CI, protected integration and readback.

Actual Platform↔Game authenticated interoperability is a later composition proof
requiring **both** compatible protected endpoints. P5 may use a controlled
compatible counterpart before Game S1 exists, and Game S1 may independently
qualify against its own controlled producer before the real Platform producer
exists. Neither side is ordered behind the other merely for local qualification.

Production endpoints, roots, certificates, private keys and deployment remain a
separate authority gate after code compatibility is protected.

## Dependency graph

```text
                 [separate Platform authority]
                           |
                           v
                P1 -> P2 -> P3 -> P4
                           |
                           v
             Platform protected qualification
                           |
                           +-----------------------+
                                                   |
[WP3 protected/released] -> Game S1 qualification |
                           |                       |
                           v                       |
                     Game S1 protected            |
                           |                       |
[WP4 + S1 gates] -------> Game S2 protected       |
                           |                       |
                           +-----------+-----------+
                                       v
                       fresh authenticated cross-repo
                         interoperability/golden proof
                                       |
                                       v
                       S3 fresh composition/readiness
```

Platform P1-P5 and Game S1 can advance independently when their own authority and
dependency gates are true. Real cross-repository interoperability is required
before S3/final source composition, not as an artificial prerequisite for local S1
qualification against a controlled producer.

P2/P3 may share one transactional source-state schema only if the future Platform
design proves the accepted independent namespaces and rollback semantics. They
must not share a revision namespace merely to reduce schema work when the Game
contract scopes ordering differently.

## Explicit exclusions

This handoff does **not** authorize:

- any write to `Oteryn/Oteryn-Platform`;
- changes to #351/#356, #335 or Server Seam #247;
- Game runtime/Cargo/Foundation/composition mutation;
- production endpoint, PKI root, certificate or key selection;
- direct Platform DB reads by Game;
- promotion of Canary `accounts.id`, Platform `identities.id`, OAuth tokens,
  Passport keys, Game Login Tickets, directory/catalogue data or fixtures into
  native source authority;
- source-only remapping of `game_auth_generation` that would desynchronize native
  issuance and evidence comparison;
- weakening the accepted five-second freshness rule or any `NSRC-*` ceiling;
- claiming producer availability before separately authorized Platform code is
  protected and interoperability is proven.

## Current disposition

`MISSING_CROSS_REPOSITORY_PLATFORM_PRODUCER` remains true.

The missing work is now bounded to Platform-owned AccountId storage, one reviewed
positive native security-generation representation, ordered account-security
source, ordered signing-trust source, private authenticated API and producer
qualification. Game WP5 may use this packet to avoid rediscovering legacy
Canary/OAuth surfaces, but Platform implementation may begin only under separate
explicit repository authority and fresh live ownership/custody.

`PLATFORM_WRITE_AUTHORITY: FORBIDDEN`
`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
