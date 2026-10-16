# APP-1 — durable selected appearance and AccountUnlock set (skeleton task record)

```yaml
task_id: APP-1
title: "APP-1: durable selected appearance and AccountUnlock set"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 6 row APP-1 (rows 126, 127); §7 supporting briefs row APP-1"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_CHARACTER_APPEARANCE_OWNER_DECISION_2026-09-28.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/app-1
needs_migration: true
needs_registry: false
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Durable selected appearance and the AccountUnlock set, validation §4.3, character fence; selection survives relog.

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

APP-1 (hard, persistence).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| Character authority | MERGED (0005) |
| progression readiness migration numbering | CP lease |
| CHARACTER_APPEARANCE | ACCEPTED by protected #2209 (`e2da9805d296a27b06b32335bf67394509d4910b`) |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/durability/character_appearance.rs`
- `apps/game-server/src/durability/character_appearance_tests.rs`
- `apps/game-server/tests/character_appearance_postgres.rs`
- `docs/agents/tasks/APP-1.md` (this record: status and notes only)

To compile on the branch the builder may add only the module registration lines in shared `mod.rs`/`lib.rs` files; they are re-applied in the slot commit after merging main, together with every other slot-only change.

## slot_only_paths (ONLY in the final commit at the merge-queue turn, after merging main)

- tools/content-migration/regenerate_content.py outputs
- content/project.json
- content/manifest.json, content/world/manifest.json
- content/content.lock.json, content/world/content.lock.json
- content/interactions/index.json
- content/world/pins/*
- Rust pins (pinned registries / fixtures regenerated from content)
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring
- crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants
- apps/game-server/src/durability/mod.rs, domain/mod.rs, lib.rs module registration

## Wiring points (verified on origin/main 5742f30f3)

- apps/game-server/src/domain/appearance.rs (pure rules, existing)
- apps/game-server/src/gameplay_transport/appearance_atomic.rs
- character fence (character_recovery_fence.rs)

## Inseparable bundles (never split across PRs)

- migration + appearance writer + durability tests

## Acceptance criteria

- [ ] Locked outfit/addon/mount rejected; out-of-palette colour rejected.
- [ ] Stale character fence writes nothing.
- [ ] Selection survives relog.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server app
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_CHARACTER_APPEARANCE_OWNER_DECISION_2026-09-28.md §4.2-§4.4, §5, §8
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Local preparation — 2026-10-15 (not an acceptance or runtime-ready claim)

Owning `CHARACTER_APPEARANCE` is now ACCEPTED through merged #2209, read at protected
`e2da9805d296a27b06b32335bf67394509d4910b`; its §4.2 delegates physical schema to this child.
The earlier CANDIDATE row is historical. The separate account-progress/#707 decision §4.3 and
§4.6 governs earned facts. The schema, migration content, fenced writer and qualification are APP-1 mechanic work. Only
migration numbering and shared integration require the CP slot; they are not a missing owner
schema decision. This preparation leaves the migration/writer/test bundle incomplete.

Implemented only private read shapes and PostgreSQL readers: a per-Character selected appearance
and the account's earned unlock facts with immutable first provenance. Decoding uses native
`domain::{AccountId, CharacterId, CharacterRevision}` and `AppearanceSelection<String>`; malformed
widths/ranges/revisions fail closed. Unknown keys and historical content provenance are retained.
Native `domain::appearance::{validate_selection, displayed_look}` is reused unchanged. No
production mutation, current-authority proof, new entitlement, fallback write or Store grant is
implemented. Reads must be called within an independently authorized admission/read transaction;
returning these historical records grants no gameplay authority.

### Private physical schema packet (implementing-child design; CP slot required)

Names below are private Game storage, not Platform/Store or protocol contracts. The owned PG test
installs four private schema fixture tables in a throwaway database, including the selection
receipt and its deferred projection/appearance-chain guards. It is not a production migration
or a production schema compatibility/role qualification.

| Relation | Keys and columns / final constraints |
|---|---|
| `game_character_appearance` | `character_id UUID PRIMARY KEY`, `outfit_key TEXT`, `colours INTEGER[4]` (one dimension, lower bound 1, exactly four non-null unsigned-16 values), `addons SMALLINT` (0..255), `mount_key TEXT NULL`, `content_revision TEXT` (1..128 UTF-8 bytes), `committed_character_revision NUMERIC(20,0)` (native u64, at least 2), final `last_appearance_occurrence_id UUID`. The fixture projection references its exact immutable last receipt; the final projection also references the Character root; no key/palette interpretation occurs on load. |
| `game_character_appearance_receipts` (created by the owned private fixture) | UUIDv7 `appearance_occurrence_id PRIMARY KEY`; complete versioned command binding; Character identity; original and committed CharacterRevision (exact +1, independently unique per Character); complete optional before/after selections and their provenance; the existing progression-chain columns (level/experience before/after, profile/ruleset/content/simulation/evidence/declaration/policy/reward revisions), unchanged for a cosmetic transition; committed time. Before/after cannot be identical. Immutable UPDATE/DELETE/TRUNCATE protection. Current full gameplay fence is checked independently before first insertion; a replay only returns historical disposition and must never reactivate an actor. |
| `game_account_appearance_unlock_receipts` | `grant_occurrence_id UUID PRIMARY KEY`; `account_id UUID`, `unlock_key TEXT`, `content_revision TEXT` (1..128 bytes), `granting_character_id UUID`, `source_kind TEXT` (nonempty), `source_event_id BYTEA` (nonempty opaque owning-event identity). Unique `(source_kind, source_event_id, unlock_key)` prevents rebinding a granting event; unique `(grant_occurrence_id, account_id, unlock_key)` supports the fact FK. The production consumer must compare a replay's full binding (account/Character/key/revision/source), not treat `ON CONFLICT` alone as replay authorization. All rows immutable; retain first and subsequent duplicate grant provenance. |
| `game_account_appearance_unlocks` | Primary key `(account_id, unlock_key)` with `grant_occurrence_id UUID`; composite FK to exactly that receipt/account/key. `ON CONFLICT(account_id,unlock_key) DO NOTHING` preserves the **first** fact and provenance. Immutable UPDATE/DELETE/TRUNCATE protection. No cascade or FK to the granting Character: Character deletion must not revoke account facts. Final account identity is derived from the event's locked Character root, never supplied as grant authority. |

All non-optional fields are NOT NULL. Outfit/mount/unlock keys stay opaque content keys; no
invented key concatenation, addon-key derivation, collation for gameplay, SQL points rule or
provenance-equality compatibility rule is introduced. Active definitions decide key mapping and
provenance compatibility. Each addon is authorized separately through the existing domain trait.
No World/Channel equivalence or account-wide selected look is introduced. Final permissions are
least privilege, REVOKE PUBLIC, immutable grants/facts and receipt reads/inserts only; the
existing runtime role and event owner must be qualified in the slot, not granted here.

### Concrete remaining native integration seams

1. Final migration and event writer must extend
   `migrations/0073_character_progression_guard_wheel_union.sql`'s latest
   `game_character_progression_consistency_guard()` family union, plus
   `durability/character_authority.rs::verify_character_integrity` (the receipt union and
   appearance-specific projection/receipt replay verification). The root revision guard in
   `0022_character_name.sql` only enforces exact +1; it is the deferred chain guard, not that
   root guard, that currently lacks an appearance receipt family. Preserve every existing arm.
   The complete guarded native writer is retained as a proposal outside the author checkout;
   no substitute receipt or uncommittable production writer is installed here.
2. First selection writes must use the actual `begin_semantic_transaction` / semantic deadline,
   `assert_recovery_fence`, `lock_admission_relations`, and
   `character_progression::assert_gameplay_fence` (including current node incarnation,
   session/connection/lease/scope generations, guards, locked root/revision and independent
   interpretation). Lock occurrence before the root as in the existing stance writer; compare
   exact replay binding before returning historical disposition. Lock progression state, verify
   context/root and expected prior selection, then atomically advance root/progression +1 and
   insert receipt/update selection. Concurrent losing transitions receive typed revision/state
   rejection; do not retry a conflicting occurrence with new facts. Pending respawn handling and
   semantic commit must match native Character writers. No completion is consumed as authority.
3. Earned unlock insertion must be **inside the actual quest/NPC granting event transaction**.
   Existing `account_achievement::FencedGrantingCharacter` is transaction-stamped with private
   Character/transaction fields; its only production constructor is
   `reward_claim_mint::admit` via `RewardClaimFenceChecked`. It exposes no cosmetic grant consumer
   accessor. Its owner must provide the exact shared transaction-bound consumer seam (including
   independently derived account, current event identity and earned source) before APP-1 can
   insert facts. A request source label or retained committed quest receipt is insufficient.
   No public token constructor, new grant authority or separate account-grant revision is added.
4. APP-2/current content must supply a real `AppearanceContent` implementation, addon unlock-key
   mapping and compatible-provenance resolver. Character creation/sex still needs its owning
   input; only the accepted native default test Character may use a content-declared default.
   Temporary spell `gameplay_transport/appearance_atomic.rs` is not the durable selection commit
   owner. Store remains unavailable until gap §32; it never becomes an earned AccountUnlock.
5. Module registration, migration numbering, shared integrity/guard integration, audit/event family
   registration, runtime admission/command publication and protocol wiring require the CP slot.
   APP-1 owns the production DDL/permissions content and native writer; their prepared proposals
   are not classified as completed or delegated slot mechanics.
   Native independent exact-head review is required before the eventual persistence integration.

### Qualification and limits

The owned standalone test wrapper loads the actual new source and uses the existing native
server rlib for all domain IDs, revisions and the unmodified appearance validator. No duplicate
native types or validator are compiled. An isolated PostgreSQL **17.6** database qualifies
private projection reads, per-Character/per-account isolation, reconnection/reload, write-once
first provenance, source-event collision, UPDATE/DELETE/TRUNCATE refusal, and SQL shape bounds.
The new selection receipt cases additionally cover exact +1, unchanged XP/level, complete prior state and provenance,
no-op refusal, deferred projection/chain consistency, transaction rollback and storage reload.
The private recovery helper rejects individually corrupted projection, before selection and
before provenance even when fixture triggers are bypassed by the isolated test administrator.
It still needs the CP-owned call from `verify_character_integrity` and its global receipt-union arm.
Fixture-only earned mapping/compatibility is explicit; revision equality there is not a product
policy. No production granting authority, runtime session, real relog, complete fenced writer,
Character deletion flow, role/grant compatibility, complete event chain or E2E is qualified.
The acceptance checkboxes above intentionally remain unchecked. In particular “stale fence writes
nothing” awaits an actual writer and independent missing/stale/mismatched/expired/replayed
boundary cases; it cannot be claimed from this read-only subset.

Evidence, exact commands, checks and source hashes are retained outside the repository in
`/workspace/oteryn-prep/app-1-evidence/` (historical index, validation, scope and source manifest).
That preparation made no worker commit/push/ready/merge/review trigger, shared source edit,
production DDL or content change.

### Current bounded packet — 2026-10-16

The CI reader wrapper remains `#[path]` and compiles actual owned reader/recovery code. A native
writer cannot share that standalone wrapper: its real fence and semantic-transaction helpers are
private to the existing native durability owner. No cfg shim, duplicate fence or public token
constructor is introduced. The guarded writer plus complete unnumbered migration and recovery
integration patch are preserved for the exact CP registration/integration slot. The writer is
not installed, tested or claimed complete by this schema-only candidate.

Resume triggers: CP authorizes native module registration and shared guard/recovery integration;
the granting owner exposes the accepted transaction-bound cosmetic consumer on
`FencedGrantingCharacter`; APP-2 supplies the real content key/provenance resolver; the Character
creation owner supplies sex. Stale-fence-no-write and actual relog remain unqualified until the
native writer and runtime admission are integrated. No acceptance checkbox is advanced.

Current evidence and the uninstalled native integration packet are retained outside the
repository at `/workspace/scratch/app1-packet/` (exact source manifest, SQL and source patches,
validation logs). The owned PostgreSQL 17.6 target passes 4 tests; focused target clippy,
workspace format, governance validation and 124 governance tests pass. These checks do not
qualify production integration or the uninstalled native writer.
