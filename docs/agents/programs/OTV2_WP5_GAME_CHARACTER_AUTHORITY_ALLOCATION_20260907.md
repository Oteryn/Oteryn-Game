# WP5 Game Character Authority owner/bootstrap allocation

Coordinator: #162. Source-readiness issue: #319. Remediation programme: #364.
Preparation authority: #319 comment `5575470388`.

## Status and boundary

Prospective Game-only allocation. **NOT_ACTIVE**. It grants no current runtime,
SQL, migration, registry, workflow, production, Platform or external write lease.
It removes scope ambiguity while WP2/WP3/WP4 continue.

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
audit_outbox_prerequisite: OTV2-WP5-CHARACTER-AUDIT-OUTBOX-20260907
canonical_pg_ci_prerequisite: OTV2-WP5-DEDICATED-POSTGRES-CI-ROUTING-20260907
ownership_sensitive_mutation_without_audit_outbox: FORBIDDEN
```

## Accepted authority consumed

This allocation consumes, without reopening:

- `docs/contracts/CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md`;
- `docs/architecture/ADR-0012-character-authority-and-platform-lifecycle-boundary.md`;
- `docs/architecture/FND-ID-01_CHARACTER_ID_ACCOUNT_LINK_OWNER_BASELINE.md`;
- `docs/architecture/FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md`;
- `docs/architecture/DUR-02_PROFILE_NEUTRAL_CHARACTER_PERSISTENCE_OWNER_BASELINE.md`;
- `docs/architecture/ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md`;
- `docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json` rules;
- accepted FND-04 current ownership/lease/session fences.

Game owns canonical `CharacterId`, current AccountId ownership binding, current
WorldId membership, Character lifecycle and CharacterRevision. Platform owns and
issues canonical AccountId and its security/authentication state. Game must never
mint a competing AccountId; it losslessly retains only an authenticated
Platform-owned AccountId supplied through an accepted source boundary.

ANL-01 is mandatory for every enabled ownership-/security-sensitive Character
mutation. The current event registry intentionally contains no domain event types,
so no create/bootstrap/owner transfer/world transfer/lifecycle mutation may be
called complete until the companion audit/outbox allocation has registered exact
event types/retention and atomically integrated their durable outbox.

## Fresh protected facts

At the preparation base:

- `apps/game-server/src/domain/mod.rs::CharacterRecord` stores CharacterId,
  lifecycle, CharacterRevision, interpretation context and build but not
  authoritative AccountId ownership/current WorldId membership;
- DUR-02 accepts normalized `character_root`, account portfolio guard, one global
  CharacterRevision per semantic transaction, global name authority, durable
  receipts, anomaly-closing locks and no-authority-resurrection after restore;
- ANL-01 requires mutation/audit atomicity, immutable EventId/payload semantics,
  durable audit and at-least-once post-commit publication;
- `GAME_EVENT_FOUNDATION_REGISTRY.json` has an intentionally empty `event_types`
  list; runtime code cannot invent Character event IDs;
- protected migrations contain only immutable `0001_admission_reconnect_journal.sql`;
- Child B/#329/#335 owns unreleased `0002_fresh_admission_authority.sql` and
  current Durability schema/db/test surfaces;
- WP3/#351 owns the shared `durability_postgres.rs` qualification target until
  terminal driver acceptance/release;
- current canonical PostgreSQL workflows execute only `durability_postgres`, so
  `character_authority_postgres` cannot count as canonical PG evidence until the
  companion CI routing allocation is protected and exercised;
- no current open Game Character Authority implementation lineage was found on
  the proposed domain/persistence surface; historical domain/DUR-02 ownership is
  archived/released.

Therefore semantic preparation is legal now, while persistence/migration/audit/CI
activation must serialize after protected WP3/WP4 release and the two companion
allocations. No `0002`, `0003` or other future migration number is selected here.

## Prospective implementation surfaces

After a later explicit Work application and fresh ownership readback:

```text
apps/game-server/src/domain/mod.rs
apps/game-server/src/durability/character_authority.rs        # new owner
apps/game-server/src/durability/character_authority_audit.rs  # new durable audit/outbox owner
apps/game-server/src/durability/mod.rs                        # module/export only
apps/game-server/src/durability/db.rs                         # exact adapter hook only
apps/game-server/src/durability/schema.rs                     # exact codec/schema hook only
apps/game-server/migrations/<NEXT>_character_authority.sql    # new; number selected after WP4
apps/game-server/tests/character_authority_postgres.rs        # new dedicated target
```

The companion audit allocation separately governs conditional mutation of:

```text
docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json
docs/contracts/game-events/v1/character_authority.proto
```

The companion CI allocation separately governs any workflow/policy/pin source
needed to make `character_authority_postgres` execute inside canonical PR/MQ PG
layers. Those are not Character Authority runtime leases.

Existing Durability files, event registry, migration namespace and workflows are
conditional future paths only. `apps/game-server/tests/durability_postgres.rs` is
not part of this allocation. If implementation can avoid `db.rs`, `schema.rs` or
`mod.rs`, it must do so. Any additional existing path requires an explicit
amendment before mutation.

## Semantic implementation contract

### Canonical current Character state

The durable root binds at least:

- canonical game-owned CharacterId;
- current authenticated Platform-owned AccountId;
- current WorldId;
- lifecycle state;
- positive non-reused CharacterRevision;
- exact interpretation/profile/ruleset/content context needed to prevent silent
  reinterpretation;
- current operation/revision evidence required for idempotent reconciliation.

A Platform list/cache/ticket never becomes Character ownership authority. Game
revalidates its own current owner/world/lifecycle state at admission/mutation.
The `CharacterRecord` API may consume an explicit validated ownership/world
binding but must not gain Platform security authority or a constructor that turns
caller-filled AccountId into current authorization.

### Portfolio and bootstrap/create

Use the game-owned account portfolio guard keyed by the losslessly wrapped
AccountId. It serializes create/restore/ownership-transfer/quota-sensitive work
but is not Account authority.

Any initial authoritative owner/world bootstrap or enabled create operation is an
ownership-sensitive mutation and therefore requires the companion ANL-01 durable
audit/outbox in the **same PostgreSQL transaction** as Character root,
CharacterRevision and operation receipt. Without registered event type/retention
and outbox capacity, bootstrap/create remains disabled/fail-closed.

For an enabled create: independently authenticated creation intent is consumed;
Game allocates fresh UUIDv7 CharacterId, resolves authoritative name ownership,
persists owner/world/lifecycle/current revision, immutable operation receipt and
mandatory durable audit events atomically. Exact retry reconciles one semantic
Character and the same audit identities. Conflicting operation reuse rejects.

This allocation selects no product quota, starter value, naming normalization or
Platform transport.

### Ownership/world/lifecycle changes

Rename, legal world transfer and legal AccountId ownership transfer preserve
CharacterId and advance CharacterRevision exactly once per committed semantic
transaction. Terminal retirement never permits CharacterId reuse.

Account transfer locks source/destination portfolio guards in canonical complete
AccountId order and the Character root. A stale owner/revision rejects. Every
enabled ownership/world/lifecycle transition also stages all mandatory registered
`DURABLE_AUDIT` events and durable publication state atomically; if audit staging
or required event registration fails, the Character mutation does not commit.

Until the audit companion is protected/applied, **AccountId ownership transfer,
create/bootstrap, world transfer and ownership-sensitive lifecycle transitions
are explicitly disabled**. This document cannot be used to accept them alone.

Admission and ownership-sensitive mutations serialize with current FND-04
lease/session authority so transfer/lifecycle cannot create a stale playable
owner. FND-04-only control transitions do not advance CharacterRevision.

### Audit/outbox atomicity

For one authoritative Character transaction, Character root mutation,
CharacterRevision, immutable operation receipt, mandatory ANL-01 event records and
durable outbox/publication state commit or roll back together. Same EventId retry
reuses the exact immutable semantic envelope/payload. Publication is at-least-once
after commit; publisher outage leaves durable pending outbox state and never
downgrades audit to best-effort or replays the Character mutation.

Exact event IDs/payload schema/finite retention profile are assigned only through
the companion registry allocation, never ad hoc in runtime code.

### Restart and restore

Normal restart reloads exact current Character ownership/world/lifecycle/revision,
operation receipts and pending audit outbox without regression. Database
restore/PITR does not resurrect rolled-back ownership/session authority; the
accepted non-rollback recovery fence or a proven later OPS/DUR mechanism is
required before new authoritative mutation.

Missing, contradictory or regressed owner/world/revision/audit state fails closed.
Platform caches, current sessions, UUID ordering or audit history cannot be used
to fabricate current Character authority.

## Persistence requirements after activation

The first forward migration after WP4 uses the DUR-02 normalized model and, for
any enabled owner-sensitive mutation, the companion ANL-01 outbox model. It
contains typed equivalents of:

- Character root/current owner/current world/lifecycle/CharacterRevision;
- account-character portfolio guard;
- immutable operation receipts;
- durable Character audit outbox/publication checkpoints;
- global name authority only when selected create/rename semantics are enabled.

No generic JSON/EAV Character authority source. `0001` and then-released B
migration remain immutable. Runtime/migration roles retain least privilege.

## Required RED/GREEN evidence

RED before material implementation proves at least:

1. current `CharacterRecord`/persistence cannot prove AccountId ownership/current
   WorldId from an independently current Game-owned source;
2. restart/bootstrap lacks durable canonical owner + operation reconciliation;
3. current repository has no registered Character durable-audit event/outbox;
4. current required PG workflows do not execute `character_authority_postgres`.

GREEN requires actual PostgreSQL17.6 through the **protected companion CI routing**
and includes:

- authenticated valid AccountId -> CharacterId/world bootstrap positive only with
  mandatory audit/outbox in same commit;
- wrong/stale owner, world or CharacterRevision rejects;
- duplicate exact create/transfer reconciles one result, one revision and same
  EventIds; conflicting operation reuse rejects;
- concurrent owner/world mutations produce one lawful winner;
- portfolio serialization under concurrent create/restore/transfer;
- active-session/lease conflict rejects under FND-04;
- injected event registration/encoding/outbox failure rolls back Character root,
  revision and receipt;
- DB rollback after staged audit leaves neither mutation nor durable event;
- publication outage survives restart as pending durable outbox and does not
  reapply domain mutation;
- same EventId with changed payload rejects; duplicate delivery deduplicates;
- restart preserves exact owner/world/revision/receipt/outbox;
- rollback/regressed revision/bootstrap fails closed;
- terminal CharacterId never reissues;
- public projection never discloses AccountId/alternate-character relation;
- dedicated PG target missing/deleted/renamed cannot pass canonical required gate.

A fixture/in-memory repository, workspace test without configured PG, or
`--no-run` does not close WP5.

## Composition with other WP5 producers

This allocation does not implement or authorize Platform native security/trust
producers. Character Authority consumes authenticated native AccountId/security
once separately authorized; it cannot fabricate them.

Game runtime-scope assignment/readiness is a separate WP5 owner. Character
ownership cannot stand in for assignment, AccountPresenceClaim or CharacterLease.
WP5 readiness requires composed evidence from all required real owners.

## Activation sequence

```text
main Character allocation + audit companion + dedicated-PG-CI allocation protected
-> WP2 protected semantic/API
-> WP3 terminal protected + shared PG target released
-> WP4 protected/read back + Durability/migration paths released
-> exact Character event registration/retention + next migration/path set applied
-> dedicated character_authority_postgres target exists
-> protected CI routing activated/exercised for that target
-> one sole Character Authority/audit writer TDD implementation
-> real PostgreSQL/restart/concurrency/audit qualification
-> independent high-risk whole-diff review
-> canonical CI + full Merge Queue + protected readback
-> compose Platform security/trust + Game assignment producers
-> only then WP5 / Server Seam reevaluation
```

Protection of these docs alone never activates Character mutation.

## Excluded scope

No Platform/external repo, credentials/KMS, production DB/deployment/live data,
root Cargo/lock, Foundation, B/#335, WP3/#356, Server Seam/#247, item/economy/
profile-specific persistence, product naming/quota values, Character Bazaar
commercial saga, analytics warehouse/broker/detectors or production retention
policy invention. Registry/workflow source remain separately governed by their
companion allocations.

Runtime E2E is NOT_APPLICABLE to this allocation-only document.
