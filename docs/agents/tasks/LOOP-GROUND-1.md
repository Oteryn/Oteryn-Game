# LOOP-GROUND-1 — ground loop: dropped item decay and cell limit (skeleton task record)

```yaml
task_id: LOOP-GROUND-1
title: "LOOP-GROUND-1: ground loop: dropped item decay and cell limit"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row LOOP-GROUND-1"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_DEATH_COMPLETE0_PLAYER_DEATH_MODULE_DECISION_2026-10-13.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/loop-ground-1
needs_migration: true
needs_registry: true
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Items on the ground (drops, corpses' spill) decay under one durable policy and each Ground cell enforces a fixed item limit, closing the ground loop after death batch B.

Children (allocate from the brief's child graph; the plan row is only the family entry gate; no child of a CANDIDATE brief is allocated before the brief is ACCEPTED):

Single row; split only after the decay-policy decision lands.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| DEATH-DUR-1 (death module batch B) | BLOCKING (not merged) |
| decision on dropped-item decay policy and Ground cell limit | BLOCKING (explicitly not decided: DEATH-COMPLETE-0 §6, remaining_unknowns; architect/owner decision needed) |
| acceptance of DEATH-COMPLETE-0 | BLOCKING (CANDIDATE) |
| MAP-OVERLAY-1a Ground rebuild | MERGED (#1800) |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/durability/item_decay_retire.rs`
- `apps/game-server/src/durability/item_decay_retire_audit.rs`
- `apps/game-server/src/durability/ground_cell_limit.rs`
- `apps/game-server/src/durability/ground_cell_limit_tests.rs`
- `apps/game-server/tests/ground_decay_postgres.rs`
- `docs/agents/evidence/LOOP-GROUND-1-*.md`
- `docs/agents/tasks/LOOP-GROUND-1.md` (this record: status and notes only)

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

- map/overlay.rs and map/overlay/pickup.rs (Ground overlay, MAP-OVERLAY-1a)
- durability/item_decay_retire.rs existing retire arms
- docs/contracts/RESOURCE_LIMITS_REGISTRY.json (cell limit)

## Inseparable bundles (never split across PRs)

- decay schedule + retire writer + audit
- cell limit (RESOURCE_LIMITS) + drop refusal + tests

## Acceptance criteria

- [ ] Dropped items decay on the decided schedule exactly once, including across restart.
- [ ] A drop beyond the cell limit is refused (or redirected per decision) with a typed code.
- [ ] Decay never removes items under custody or in a corpse still owned per DEATH-COMPLETE-0.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server ground
cargo test -p oteryn-game-server corpse_decay
cargo test -p oteryn-game-server item_decay
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_DEATH_COMPLETE0_PLAYER_DEATH_MODULE_DECISION_2026-10-13.md §6 (remaining_unknowns)
- PR #1800 (MAP-OVERLAY-1a)
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 3), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Local bounded preparation — 2026-10-15 (not runtime acceptance)

Allocation: existing draft #2158, `skeleton/loop-ground-1`, authoring base
`efc7427095af92b969ca9341da5845d7bcca695b`. This allocation permits only
`durability/ground_cell_limit.rs`, its adjacent tests, and this record; the
historical broader owned-path list above is not an active write grant. No
commit, push, module registration, migration, registry, content or pin edit.

The independently retained protected-main decision at
`e2da9805d296a27b06b32335bf67394509d4910b`,
`OTERYN_GAME_ACCEPT_BATCH_2026-10-15_ACCEPTANCE_DECISION.md` §§2–3, releases the
old undecided policy gate: **64 items per tile**, overflow **"not enough room"**,
and **no periodic ground cleanup between resets**. It accepts DEATH-COMPLETE-0
and retains ADR-0021 D191 (crash survival and planned reset). Historical
decay/checklist/dependency text above is preserved as history, not an instruction
to introduce a timer or retire writer. No new timed cleanup is prepared here.

### Verified native gap and prepared source

- Existing native Ground MINT locations in `item_mint.rs`, `map_item_mint.rs`,
  `native_map_items.rs` and `spell_item_transaction.rs` have no per-tile 64-root
  admission check. The spell transaction's 500-row snapshot bound is a read-work
  limit. `map/overlay.rs::add_ground` intentionally preserves durable truth on
  rebuild; neither that overlay nor custody writers were duplicated or edited.
- `ground_cell_limit.rs` checks actual native `GroundItemInstance` data against
  independently supplied typed World/Channel, ownership generation and native
  `SpellGroundTarget`. It reuses the native signed 10-byte Ground codec and
  rejects mismatched cell/revisions/placement context or duplicate item IDs.
- It counts physical top-level Ground roots, not stack quantity. The caller
  supplies complete remaining live roots after actual removals and actual new
  roots after the native stack planner has decided merges. Contained descendants
  belong to the separate ITEM-MOVE §5/BAGS per-channel counter, not this root
  count. Static base-map decoration has its separate ADR-0021 base-item bound;
  materialized map items in Ground custody are ordinary supplied roots.
- The helper only checks a plan. It does not establish current authority,
  completeness, ordering, an allowed merge, a destination or reserved capacity.
  It has no mutable owner, expiry, cleanup, writer, grants or persistence.

### Remaining CP/architect integration obligations

Runtime acceptance remains unchecked. CP must register and integrate the check
at every actual root-insertion boundary, including player drop, death spill,
loot/map/spell MINT and inventory overflow, with complete live reads under the
accepted tile/custody locks and the same physical transaction as the inserts.
Current ownership, session-generation, recovery, admission and content fences
remain each existing writer's independently resolved obligations. Caller data,
prepared rows and a successful helper result are not current authority.

Preserve native top-stack ordering/merge rules and per-channel counter updates;
do not guess an overflow destination or redirect items on refusal. A retry or
reload must independently reread occupancy and recheck. Removal-only and rebuild
paths must preserve durable rows, including preexisting overfull tiles, without
using admission refusal as cleanup. Death writer/schema integration, resource
registry/error mapping and slot-only wiring remain later CP work. No PostgreSQL
concurrency, rollback, restart durability, full-server, wire/E2E, architect
acceptance or independent exact-head review is claimed by pure helper tests.

### Actual local validation

- **8/8 passed** in an exact-source standalone Rust harness: 64/65 admission,
  batched insertion, physical stack/root counting and zero-new-root merge,
  actual removal/replacement, duplicate IDs, independently varied World/Channel/
  generation, each tile metadata field, opaque/truncated addresses, and repeated
  reload evaluation without mutable state. The last case is pure determinism,
  not proof of database restart durability. These new tests have no executed
  behavioral RED predecessor; no setup failure is reported as a RED test.
- Standalone `clippy-driver` passed with workspace Rust/Clippy deny settings
  and `-D warnings`; owned `rustfmt --check` and `cargo fmt --all --check` passed.
  Rust commands ran under `/tmp/oteryn-prep-rust.lock`, shared target/jobs 2,
  `CARGO_INCREMENTAL=0`. No server/Cargo test binary or migrate link was built.
- Governance validation and **124/124** agent unit tests passed. Whole owned diff
  reviewed; no new cleanup, authority source, writer or guessed stack merge.
  Scope and whitespace checks passed for exactly the three allocated paths.
- Durable evidence: `/workspace/oteryn-prep/loop-ground-1-evidence/`, including
  real raw logs, reproduction scripts and before/after bindings of five actual
  source files. The harness imports both new files directly and uses nine
  hashed, verbatim native type/ID/codec excerpts; no writer/authority stub.
- Source SHA-256: helper
  `54cae6c36bd6bef274673e3c5bb5d68fe6fa7e414d1f34bc51da5afec701c984`;
  tests `d3b3f1fa32acce8ca5909966c6f42fffe20825dd33c60b7c8491c5e08ac48c5c`;
  harness `f25b85af35bc9ac04e4194d103964ae111232e97c6dc163977b292e73f5fb08b`.

The detached owned subset is prepared, not accepted as the full LOOP-GROUND-1
implementation. The original runtime acceptance boxes remain unchecked.
