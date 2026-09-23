# CHARACTER_AUTHENTICATED_BOOTSTRAP_INTENT_V1

Status: architecture decision candidate for Issue #791. No implementation authority follows until this document is independently reviewed, protected-integrated and read back from `main`.

## Scope

This decision closes only the missing authenticated creation intent required by the bounded WP5 Character Authority first slice. It does not enable ordinary player-facing Character creation, naming or quota policy, Character transfer, World transfer, rename, retirement, Platform writes or a second Character system.

- Parent control plane: #162
- Source-readiness programme: #319
- Blocked material implementation: #790 / WP5 #414

## Decision

Platform Account authority owns and authenticates the bootstrap intent. Game consumes that intent and remains the sole Character mutation authority.

For the current G0 milestone, only one intent variant is enabled:

`OPERATOR_CONTROL_PLANE_BOOTSTRAP`

An authorized operator/control-plane request causes the Platform Account authority to authenticate the canonical AccountId and issue one immutable, bounded Character bootstrap intent. The operator does not directly attest or fill AccountId as authority. Game validates current Game-owned context, allocates CharacterId, decides current owner/world/lifecycle state and commits the authoritative result.

A later `PLATFORM_USER_CREATE` variant is reserved as a separate explicit contract and remains disabled. It cannot be treated as an alias of the operator variant and is not required for G0.

## Authority separation

The bootstrap intent is not interchangeable with:

- #415 `NodeIncarnationProof`, which proves only current Game process incarnation;
- S1/S2 account-security evidence, which proves only current Platform account-security state;
- an FND-04 admission or recovery grant;
- a caller-filled AccountId, WorldId or interpretation revision;
- an account portfolio guard;
- a receipt, audit event, cache, projection or current GameSession.

Current protected S1/S2 evidence may be consumed as an independent prerequisite for the exact AccountId. An allowed account-security observation never becomes create authorization by itself.

## Authenticated intent

The minimum immutable intent is logically equivalent to:

```text
CharacterAuthenticatedBootstrapIntentV1 {
    contract_version = 1
    variant = OPERATOR_CONTROL_PLANE_BOOTSTRAP
    issuer_authority
    issuer_decision_id
    source_revision
    operation_id
    operation = INITIAL_CHARACTER_BOOTSTRAP
    account_id
    target_world_id
    interpretation_context {
        profile_revision
        ruleset_revision
        content_revision
        starter_template_revision
    }
    issued_at_source
    expires_at_source
    audience = OTERYN_GAME_CHARACTER_AUTHORITY
}
```

Semantics:

- `issuer_authority` is a configured authenticated Platform authority namespace; request payload cannot select or broaden trust.
- `issuer_decision_id` identifies one immutable Platform authorization decision.
- `source_revision` is positive and comparable inside the issuer/variant scope.
- `operation_id` is the stable cross-boundary semantic operation identity. Its uniqueness scope is `(issuer_authority, variant, operation_id)`.
- `account_id` is the canonical Platform-issued subject authenticated for this operation.
- `target_world_id` is requested context, not proof that Game accepts that world.
- interpretation revisions bind retry to the exact already-versioned first-slice semantics and do not authorize new product values.
- source issue/expiry times are authenticated source-time provenance. The implementation allocation may choose only a bounded technical expiry compatible with the issuing control-plane operation; this decision chooses no product-facing duration.
- `audience` prevents intent reuse by another consumer.

Game stores the complete canonical semantic binding with the operation receipt. Equal scoped operation identity with changed intent semantics is conflicting reuse.

## Freshness and anti-rollback

Game validates source time against conservative trusted Game time. Database insert time, cache time, message arrival or retry time cannot refresh the intent.

For each authenticated issuer/variant scope, Game retains the highest accepted comparable intent source revision:

- lower revision rejects;
- equal revision is allowed only for byte-equivalent authenticated intent;
- equal revision with changed decision/content fails closed as contradiction;
- missing or regressed retained floor after restart/restore is not initialized to zero from absence.

The Character restore fence selected by `CHARACTER_RESTORE_NONROLLBACK_FENCE_V1` is a mandatory prerequisite before the retained intent floor or Character state may be treated as current after restore.

## Consumption and atomicity

Before the authoritative first commit, Game proves independently:

1. authenticated intent source/issuer and supported operator variant;
2. exact unexpired intent and exact operation binding;
3. current allowed S1/S2 Platform account-security evidence for the same AccountId under its existing freshness/non-rollback rules;
4. current #415 Game process incarnation;
5. current valid Character recovery fence;
6. valid current Game World/context interpretation;
7. account portfolio and Character transaction serialization.

In one PostgreSQL transaction, or not at all, Game commits:

- intent/high-water advancement required by this operation;
- fresh canonical CharacterId and Character root/current state;
- positive CharacterRevision;
- immutable operation receipt containing the complete authenticated intent binding;
- required registered durable-audit event with stable EventId and exact payload bytes;
- pending durable outbox/publication state.

Platform never writes Character tables and never becomes the authoritative AccountId-to-CharacterId relation owner.

## Replay and ambiguity

- exact retry before or after ambiguous response reconciles by the same scoped operation identity;
- a committed exact retry returns the same Character, revision, EventIds, transaction identity and payload bytes;
- exact retry cannot allocate a second Character or advance revision;
- same operation identity with changed AccountId, WorldId, interpretation, variant, issuer decision or intent bytes is a conflict;
- uncertain commit outcome is reconciled before any new attempt;
- source or dependency unavailability yields bounded unavailable/fail-closed behavior, never a fallback authorization.

## Minimum implementation handoff

After this decision is protected, Work may amend the SAME #790 lineage only as narrowly as needed to:

- replace caller-authoritative `BootstrapCommand` semantics with `CharacterAuthenticatedBootstrapIntentV1`;
- authenticate/verify the Platform-owned intent through a dedicated narrow consumer seam;
- retain the intent source high-water and complete intent binding with the existing Character operation receipt;
- continue requiring independent S1/S2 account-security, #415 current-incarnation and Character recovery-fence evidence;
- extend the existing `character_authority_postgres` target.

Use existing AccountId, WorldId, interpretation revision, semantic transaction, receipt and audit/outbox types when their meanings match. Do not reuse #415 bootstrap secret/types as Character intent authority and do not add a generic broker/authority framework.

If concrete compiler/API evidence requires `db.rs`, `schema.rs` or another existing shared path, Work must allocate that path explicitly. This architecture document grants no write lease.

## Qualification matrix

Real PostgreSQL 17.6 and final composition must independently prove:

- valid current operator Platform intent + current account-security + process proof + recovery fence -> exactly one Character/revision/receipt/audit/outbox commit;
- caller-filled AccountId without intent -> reject with zero authoritative writes;
- S2 allow without intent -> reject;
- valid NodeIncarnationProof without intent -> reject;
- unsupported/future user-create variant -> reject;
- exact retry before/after lost response -> same result/EventIds;
- same operation with changed AccountId/world/context/variant -> conflict;
- lower intent source revision -> stale reject;
- equal revision with changed authenticated intent -> contradiction/fail closed;
- expired, future/ambiguous or unauthenticated intent -> reject/unavailable;
- source unavailable -> bounded unavailable, no fallback;
- wrong/denied/stale current Platform account-security -> reject;
- wrong/revoked/replaced #415 process proof -> reject independently;
- missing/stale Character recovery fence -> reject independently;
- concurrent exact retries -> one semantic result;
- concurrent distinct operations serialize through accepted account/Character locks without inventing quota policy;
- event registration/encoding/outbox failure -> entire transaction rolls back;
- restart retains exact receipt and source floor without re-authorizing;
- restored database without valid recovery fence -> bootstrap remains closed;
- raw intent secrets/credentials and private AccountId linkage are not logged or publicly projected.

## Explicitly deferred

Ordinary player-facing Character creation remains disabled. A later `PLATFORM_USER_CREATE` contract must independently define its user authorization and product-specific naming/quota/starter semantics. This deferral is intentional under the protected playable-first/minimum-sufficient policy.
