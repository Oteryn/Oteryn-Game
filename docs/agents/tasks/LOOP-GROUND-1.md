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

## Native capacity owner, generation 2 (authoring checkpoint; 2026-10-15)

Existing draft #2158 at `8c2e00b6dd3f74fe89cf72acb9980ad1d14061c7`
has a new exact four-path local allocation: the Ground helper, its tests, the
original PG wrapper and this card. Accepted Batch §§2–3 remains the rule owner:
64 physical Ground roots, “not enough room”, no periodic cleanup. The original
decay-policy/candidate gate is historical; no new decay implementation is made.

`check_ground_cell_root_admission_in_transaction` consumes the existing sealed
native `SpellItemScopeAuthority` and an actual borrowed physical transaction.
It reuses `locked_tile_rows_for_scope`, whose genuine producer independently
checks Root recovery, Node incarnation, current scope assignment/readiness and
native admission, and holds the existing Channel key-33 lock. The reader checks
the physical transaction, actual live row placement/generation/order and its
existing complete 500-visited-item bound. Unknown or incompatible rows are never
filtered into an empty/cached/truncated fallback. Actual current rows become the
native remaining inputs to the unchanged count/identity policy; the caller
cannot supply a remaining/removal DTO.

The caller must read after its actual removals and before its exact prospective
insertions in that same transaction. The result neither writes nor commits,
reserves room beyond the transaction, authorizes an insertion, nor qualifies
current Character/item/content/map/frame facts. In particular, a target matching
an empty tile is still target DATA; only the eventual actual writer can prove
the current admitted target. No new authority wrapper, getter, guard predicate,
cause, schema, clock, role, wire code or migration is introduced.

Zero new roots incur no capacity charge, even for preexisting overfull rows;
native transaction/source checks remain. Removal-only and rebuild paths preserve
truth rather than using a capacity refusal to hide/retire it. The actual writer
still owns stack merging and overflow destinations. Every player drop, death
spill, map/loot/spell MINT and overflow insertion must later compose this guard
with its own real source/current authority and exact inserts. No claim of global
enforcement or complete atomic drop integration follows from this callable part.

### Source qualification plan (not executed)

The original eight pure test bodies remain byte-identical. Eight native PG
bodies are authored for 63+1/64+1 and 65-root zero admission; an actual native
Ground-to-container-slot transfer observed without supplied remaining rows;
foreign physical transaction; stored placement/prospective identity mismatch;
actual stale scope generation and independently registered nonholder Node;
native row locks through rollback; opaque stored source/empty-target authority
distinction; and the native 501-row complete-read refusal. The removal case
qualifies a real committed source removal followed by a fresh guarded read,
not an unimplemented removal-plus-insertion compositor.

PG fixtures use installed native migrations, genuine bestiary Root/recovery/
Node/scope/readiness fixtures, actual MINT/TRANSFER writers and unchanged native
guards. MINT causes and definition facts are explicitly native test fixtures,
not gameplay-produced kills or admitted Content. The existing test-only fresh
spell lane is fixture setup; no fabricated sealed DB authority constructor is
added. The private-root standalone target is disabled until CP registers the
module; zero tests there is not a runtime pass. Outside-tree literal native
registration/selected-body qualification is planned, with its transforms and
source bindings recorded.

No new Rust or PG result is claimed at this checkpoint. Heavy qualification
awaits the root's existing Trade/VIP/Guild queue and storage budget. Evidence
is retained in `/workspace/oteryn-prep/loop-ground-1-native-2-evidence`.
Early independent review corrected a fixture assertion to the actual native
`game_item_container_slots` Character/World/item binding; it was caught before
compilation and is not runtime RED evidence. A wrapper-context concern was
explicitly retracted after verifying this exact draft's native source, with no
source expansion. Original full-family acceptance and CP no-ready/merge/shared
slot rules stay open and unchanged. The worker performs no commit or push.

The source-only phase-3 review removes a newly added module-level
`allow(dead_code)` rather than hiding the missing production insertion callers.
The real strict-production result remains pending and any actual unused-chain
failure will be retained as RED/unqualified. Root permits a separately recorded
ordinary-Rust-warning metadata context, retaining the configured Clippy denials,
to qualify the original private-root consumer and selected native bodies after
that result. This scoped evidence cannot complete the strict/global CI gate or
substitute for the real future insertion callers. No fake caller or public
visibility expansion is used.


## Native generation 4: actual focused qualification (2026-10-16)

The bounded same-transaction Ground admission component now has an actual
**16/16 PASS**, comprising the original eight pure cases and eight mandatory
native PostgreSQL cases. All eight PG fixtures independently reported
`server_version_num=170006`; zero tests were ignored. The selected native rlib
and driver compiled and linked successfully. Actual current Root/recovery/Node/
scope constructors, native MINT/TRANSFER writers, row locks, complete-reader
bounds and negative authority assertions remained enabled.

The first genuine native run was **10 PASS / 6 FAIL**: six MINT-based fixtures
supplied an empty `corpse_ref`, which the unchanged native technical-byte guard
rejects. The shared PG fixture now supplies bounded canonical native test DATA
`id(3)`. This changes neither authority nor admission rules. All sixteen test
function bodies and the original eight-case pure prefix remain unchanged.
The actual failing run and successful successor are both retained.

Production strict metadata remains **RED/unqualified** for six unused private
Ground-chain items. The enabled original consumer strict check likewise remains
**RED/unqualified** for two unused existing native harness helpers. Separately
named ordinary-Rust-warning production and original-consumer metadata contexts,
and fresh selected-native metadata, passed with every configured Clippy denial
preserved. No allowance, fake caller, visibility widening or global strict PASS
was introduced. Production metadata reuse proves the exact retained 1,964-file
context, all 61 extern/proc-macro artifacts, actual captured argv/environment,
toolchain and restored artifact hash; the repaired leaf is excluded by native
`cfg(test)` in the unchanged production graph. Fresh consumer/selected metadata
and the native sixteen cases were executed after the fixture repair.

The root-approved measured Ground-only operating budget retained the 512 MB
reserve and all product/test gates. Earlier setup and resource-check failures
remain evidence. Runtime code hashes before and after qualification are equal;
this card update is a literal append after that completed execution. Evidence,
commands, original-body mappings and logs are in
`/workspace/oteryn-prep/loop-ground-1-native-2-evidence/`, especially
`attempt-4-tests.log`, `attempt-4-status.json` and `overlay-manifest-attempt-4.json`.

This qualifies the bounded native component and its original fixtures. It does
not complete every insertion writer, shared module registration, destination/
stack planning, current Content/map authority, full-server/wire/E2E, global strict
CI or the complete mechanic. Those CP and runtime acceptance gates remain open.
Final independent execution review and root-owned candidate publication follow
this checkpoint; the worker has made no commit or push.
