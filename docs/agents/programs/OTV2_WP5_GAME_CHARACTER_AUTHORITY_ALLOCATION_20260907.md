# WP5 Game Character Authority owner/bootstrap allocation

Coordinator: #162. Source-readiness issue: #319. Remediation programme: #364.
Preparation authority: #319 comment `5575470388`.

## Status and boundary

This is a prospective Game-only allocation. It is **NOT_ACTIVE** and grants no
runtime, SQL, migration, production, Platform or external-repository write lease.
It removes future scope ambiguity while WP2/WP3/WP4 continue.

```yaml
allocation_id: OTV2-WP5-GAME-CHARACTER-AUTHORITY-20260907
repository: Oteryn/Oteryn-Game
coordinator_issue: 162
source_issue: 319
programme_issue: 364
allocation_base_main_sha: 494723b0d271e675eb44e63661c8575936ed6144
allocation_state: NOT_ACTIVE
worker_launch: NONE_UNTIL_WORK_APPLICATION
external_repositories: []
platform_write_authority: FORBIDDEN
production_authority: FORBIDDEN
migration_number: SELECT_AFTER_WP4_PROTECTED_READBACK
```

## Accepted authority consumed

This allocation consumes, without reopening:

- `docs/contracts/CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md`;
- `docs/architecture/ADR-0012-character-authority-and-platform-lifecycle-boundary.md`;
- `docs/architecture/FND-ID-01_CHARACTER_ID_ACCOUNT_LINK_OWNER_BASELINE.md`;
- `docs/architecture/FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md`;
- `docs/architecture/DUR-02_PROFILE_NEUTRAL_CHARACTER_PERSISTENCE_OWNER_BASELINE.md`;
- accepted FND-04 current ownership/lease/session fences.

Game owns canonical `CharacterId`, current AccountId ownership binding, current
WorldId membership, Character lifecycle and CharacterRevision. Platform owns and
issues canonical AccountId and its security/authentication state. Game must never
mint a competing AccountId; it losslessly retains only an authenticated
Platform-owned AccountId supplied through an accepted source boundary.

## Fresh protected facts

At the preparation base:

- `apps/game-server/src/domain/mod.rs::CharacterRecord` stores CharacterId,
  lifecycle, CharacterRevision, interpretation context and build but does not
  store authoritative AccountId ownership or current WorldId membership;
- DUR-02 already accepts normalized `character_root`, an account portfolio guard,
  one global CharacterRevision per Character semantic transaction, global name
  authority, durable operation receipts, explicit anomaly-closing locks and
  no-authority-resurrection after restore;
- protected `apps/game-server/migrations/` contains only immutable
  `0001_admission_reconnect_journal.sql`;
- Child B/#329/#335 still owns unreleased forward
  `0002_fresh_admission_authority.sql` plus current Durability schema/db/test
  surfaces;
- WP3/#351 currently owns the shared PostgreSQL qualification target until its
  terminal driver acceptance and explicit release;
- no current open Game Character Authority implementation lineage was found on
  the proposed domain/persistence surface; historical domain/DUR-02 ownership is
  archived/released.

Therefore the Character owner implementation can be prepared semantically now,
but any persistence/migration/PG implementation must serialize after protected
WP4 and the shared test-target release. This allocation intentionally does not
select `0002`, `0003` or any other migration number in advance.

## Prospective implementation surfaces

After a later explicit Work application and fresh ownership readback, the
smallest Game implementation is expected to use these semantic surfaces:

```text
apps/game-server/src/domain/mod.rs
apps/game-server/src/durability/character_authority.rs        # new
apps/game-server/src/durability/mod.rs                        # module/export only
apps/game-server/src/durability/db.rs                         # exact adapter hook only
apps/game-server/src/durability/schema.rs                     # exact typed codec/schema hook only
apps/game-server/migrations/<NEXT>_character_authority.sql    # new; number chosen after WP4
apps/game-server/tests/character_authority_postgres.rs        # new dedicated target
```

The existing broad Durability files and migration namespace are **conditional
future paths**, not current leases. They become writable only after Work freshly
verifies that WP4/B and WP3 have released them and records an exact path-level
application. `apps/game-server/tests/durability_postgres.rs` is deliberately not
included: the Character Authority uses its own dedicated PostgreSQL target to
avoid stealing B/driver custody.

If implementation can avoid one of `db.rs`, `schema.rs` or `mod.rs`, it must do
so. Any additional existing source path requires a new explicit amendment before
mutation. Root Cargo/lock, Foundation, workflows, registry and Platform are not
part of this allocation.

## Semantic implementation contract

### Canonical current Character state

The durable root must bind at least:

- canonical game-owned CharacterId;
- current authenticated Platform-owned AccountId;
- current WorldId;
- lifecycle state;
- positive non-reused CharacterRevision;
- exact interpretation/profile/ruleset/content context required to prevent
  silent reinterpretation;
- current operation/revision evidence needed for idempotent reconciliation.

A Platform list/cache/ticket is never the Character ownership source. Game must
revalidate its own current owner/world/lifecycle state at the authoritative
admission/mutation boundary.

The existing `CharacterRecord` semantic API may be extended to carry or consume
an explicit validated ownership/world binding, but it must not gain Platform
security authority or a constructor that treats caller-filled AccountId as
current authorization.

### Portfolio and creation bootstrap

Use the accepted game-owned account portfolio guard keyed by the losslessly
wrapped AccountId. It serializes create/restore/ownership-transfer/quota-sensitive
operations but is not Account authority.

Character creation requires an independently authenticated creation intent from
the accepted source boundary, then Game allocates the fresh UUIDv7 CharacterId,
resolves authoritative name ownership, persists owner/world/lifecycle/current
revision and a durable operation receipt atomically. Same operation identity +
same request reconciles one logical Character; conflicting reuse rejects.

This allocation does not choose product quota counts, starter values, naming
normalization or a Platform transport. Unresolved product/profile choices stay
fail closed or disabled.

### Ownership/world/lifecycle changes

Rename, legal world transfer and legal AccountId ownership transfer preserve
CharacterId and advance CharacterRevision once per committed semantic
transaction. Terminal retirement never permits CharacterId reuse.

Account transfer locks source/destination portfolio guards in canonical complete
AccountId order and the Character root in the accepted deterministic ordering.
A stale owner or stale CharacterRevision rejects before mutation.

Admission and ownership-sensitive mutations must serialize with current FND-04
lease/session authority so a transfer/lifecycle change cannot create a stale
playable owner. FND-04-only control transitions do not advance CharacterRevision.

### Restart and restore

Normal restart reloads exact current Character ownership/world/lifecycle/revision
without regression. Database restore/PITR does not automatically resurrect
rolled-back ownership/session authority: the accepted non-rollback recovery fence
or an equivalently proven later OPS/DUR mechanism is required before serving new
authoritative mutations.

Missing, contradictory or regressed owner/world/revision state fails closed. No
best-effort reconstruction from Platform caches, current sessions, UUID order or
audit history may become current Character authority.

## Persistence requirements after activation

The first forward migration after WP4 must use the accepted DUR-02 normalized
model, including typed equivalents of:

- Character root/current owner/current world/lifecycle/CharacterRevision;
- account-character portfolio guard;
- operation receipts necessary for ambiguous-result reconciliation;
- global name authority only if the selected first implementation exercises
  create/rename and its naming policy is already accepted.

Do not create a generic JSON/EAV Character state source. `0001` and the then
released B migration remain immutable. The migration tool/role and normal runtime
roles retain existing least-privilege separation.

## Required RED/GREEN evidence

Before implementation source:

1. RED current `CharacterRecord`/persistence cannot prove AccountId ownership and
   current WorldId from an independently current Game-owned source.
2. RED restart/bootstrap lacks a durable canonical owner row and operation
   reconciliation source.

GREEN must include, at minimum:

- authenticated valid AccountId -> CharacterId/world bootstrap positive;
- wrong/stale owner rejects;
- wrong/stale world rejects;
- stale CharacterRevision rejects;
- duplicate exact create/transfer operation reconciles one result;
- conflicting operation identity reuse rejects;
- concurrent same-character owner/world mutations produce one lawful winner;
- portfolio guard serialization under concurrent create/restore/transfer;
- active-session/lease conflict rejects according to accepted FND-04 policy;
- restart preserves exact owner/world/revision;
- injected rollback/regressed revision/bootstrap fails closed;
- terminal CharacterId never reissues;
- no public/read projection discloses AccountId relation outside authorized
  boundary.

Actual PostgreSQL 17.6 is mandatory for persistence/lock/restart evidence. A
fixture/in-memory repository does not close WP5.

## Composition with Platform and other WP5 producers

This Game allocation does **not** implement or authorize the still-missing
Platform native security/trust producers identified by #319. Character Authority
may consume an authenticated native AccountId/security source once separately
implemented/authorized, but cannot fabricate it.

Likewise Game runtime-scope assignment/readiness is a separate WP5 Game owner
under the protected scope-assignment decision. Character ownership cannot stand
in for runtime assignment, AccountPresenceClaim or CharacterLease.

WP5 readiness requires composed evidence from all required real owners, not only
this Character Authority.

## Activation sequence

```text
prospective allocation protected
-> WP2 protected semantic/API
-> WP3 terminal protected + shared PG target released
-> WP4 protected implementation/readback + Durability/migration paths released
-> Work fresh readback selects next migration number and exact conditional paths
-> explicit Character Authority allocation application
-> TDD implementation on one sole writer
-> real PostgreSQL/restart/concurrency qualification
-> independent high-risk whole-diff review
-> canonical CI + full Merge Queue + protected readback
-> compose with separately authorized Platform security/trust and Game assignment producers
-> only then WP5 readiness / Server Seam reevaluation
```

The prospective document may integrate while predecessors run because it changes
no runtime bytes and grants no active writer. It must not be interpreted as
permission to mutate conditional paths before the explicit later Work
application.

## Excluded scope

No Platform/external repository, credential/key/KMS, production DB, deployment,
live data, root Cargo/lock, Foundation, B/#335, WP3/#356, Server Seam/#247,
workflow/ruleset/MQ, resource-registry, item/economy/profile-specific persistence,
product naming/quota values or Character Bazaar commercial saga mutation.

Runtime E2E is NOT_APPLICABLE to this allocation-only document. Later material
implementation requires actual database/source/restart evidence.
