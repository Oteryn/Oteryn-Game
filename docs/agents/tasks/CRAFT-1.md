# CRAFT-1 — item conversion crafting (skeleton task record)

```yaml
task_id: CRAFT-1
title: "CRAFT-1: item conversion crafting"
mode: IMPLEMENT
status: basic_server_validated
pr: null
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row CRAFT-1"
briefs:
  - (none on main; CRAFT brief required)
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: claude/happy-davinci-ijz3wy-srv-craft
needs_migration: true
needs_registry: false
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **PARTIAL**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: [#2298](https://github.com/Oteryn/Oteryn-Game/pull/2298). CRAFT-CONTENT-1 supplies 125 validated authored conversion recipes and lowering. Evidence: `crates/gameplay/content/src/content/project/v2/item_conversion.rs`.

Remaining: Atomic consume/mint/audit execution, NPC/tool runtime and unresolved digging activation remain; the content compiler is not a crafting writer. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

## Goal

Use-with conversions (e.g. tools on items) produce output items through a data-driven conversion table, atomically and deterministically.

## Server wiring gap list

Audit of the allocated HEAD, 2026-10-23 (local AUTHORING; owner forbids publication):

| Layer | Present | Missing / action |
|---|---|---|
| L3 | 125 validated `item_conversion` declarations, pinned cooking provenance | Execute the basic fixed cooking shapes; refuse unsupported shapes |
| L4 | Current item fence, root serialization, existing item and container rows | Atomic consume/mint, immutable command receipt and audit; migration 0165 for proofs |
| Wire / registry | Bounded USE command 2, map use-with under capabilities 11 + 15, existing typed results | Reuse codec and IDs; no new command |
| Dispatch | Food/potion USE and map-handle resolution | Mechanic-owned conversion handler and production authority forwarding |
| Capability | Bundle item/map views | Offer existing 11 + 15 for bundle cooking |
| Runtime | Native bundle map, qualified item facts, admitted character/session | Resolve current reachable cooking station and invoke fenced writer |
| Validation | Registered dispatch library PG lane | Filtered admitted-session bytes → database → result/reload E2E |

Children (allocate from the brief's child graph; the plan row is only the family entry gate; no child of a CANDIDATE brief is allocated before the brief is ACCEPTED):

Single row; children allocated after the architect writes the CRAFT brief.

## Basic server implementation (owner direction 2026-10-23)

The production `FreshAdmissionAuthority` now routes existing command 2 map use-with to
`gameplay_transport/item_craft.rs`. The item owner re-resolves the authored fixed cooking
row against the admitted native item policies inside the fenced transaction. The server
binary compiles the existing `content/craft/recipes.json` catalogue; no client-selected
recipe, output, quantity or definition is trusted. Supported sources are plain direct
entries of the main backpack. Each output is a fresh stack (C3a v1: no implicit merge).
Migration 0165 adds only the immutable command/audit receipt and narrow proof arms;
existing ItemInstance/container rows hold all value. The recipe-owned native profile
overlay supplies 22 previously absent cooking inputs/stations/outputs using the existing
pinned importer and normalized facts; existing native policies retain precedence. Existing audit operation tags are
preserved; native type-2 operation 15 records the cooking proof.

Bundle worlds offer existing capabilities 11 and 15. Under the unchanged mandatory
admission-floor rule, bundle policies declare `[11, 18]` and clients support the closure
`[4, 6, 11, 15, 18]`. The local-login bundle configuration is updated accordingly.
Fixture worlds without a bundle retain their previous offer.

### Reference parity table

| Visible behavior | Reference evidence | Oteryn now |
|---|---|---|
| Dough on an oven produces bread and other authored baked foods | Pinned Canary `baking.lua` lines 35–48, `content/craft/recipes.json` provenance | Fixed single-USED replacement cooking rows execute through USE/use-with |
| Wheat on a millstone produces flour | Same pinned script, lines 52–59 | Same content-driven executor; no hard-coded recipe pairs |
| Cookie baking returns the tray and twelve cookies | Same pinned script, lines 42–44 | Fixed secondary outputs are minted in the same transaction when their native facts are admitted |
| No room or insufficient carrying capacity changes nothing | Accepted CRAFT1 C3/C3a | Main-container capacity, the 500-item tree bound and locked carried-weight projection checked before writes |
| Unknown pair or absent/stale input refuses | Accepted CRAFT1 C1/C3 | Existing typed USE refusals; no receipt or audit for a refused attempt |
| Retry/restart cannot consume twice | Accepted CRAFT1 C2/C3; existing item-use receipt pattern | Immutable `(GameSessionId, CommandId)` receipt compares original bounded USE bytes before current handle/custody resolution |

### Unfinished

- Carried-target/fluid ingredient combinations, fishing/random outcomes, digging, NPC
  exchanges, timed outputs, nested/equipped sources and definition-hotkey crafting remain
  outside this basic server execution. Those shapes fail closed. No client UI changes.
- No commit, push, PR, paid external review, integration or task archive is performed in
  this worker workspace. Full CRAFT-1 acceptance remains broader than this basic form.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| #2039 (ITEM-USE-1a+1b), ITEM-USE-1c | MERGED (#2039, #2082) |
| ITEM-LOOP-1 (family entry gate) | BLOCKING (Wave 1, not merged) |
| acceptance of the CRAFT1 brief (`OTERYN_GAME_CRAFT1_DECISION_2026-10-15.md`) | ACCEPTED by the owner 2026-10-15 ("akceptuje hire i craft"), recorded in ACCEPT-BATCH-2026-10-15 §8; effective when this PR is integrated. The DUR-03 §39.3 pointer and the RESOURCE_LIMITS admission rows named by its C3a and C8 are added by this PR (see §8), so the CRAFT-RT-1 and CRAFT-NPC-1 admission precondition is met on its integration |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `content/craft/** (sources only)`
- `crates/gameplay/durability/src/durability/item_craft.rs`
- `crates/gameplay/durability/src/durability/item_craft_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/item_craft.rs`
- `apps/game-server/tests/item_craft_postgres.rs`
- `docs/agents/evidence/CRAFT-1-*.md`
- `docs/agents/tasks/CRAFT-1.md` (this record: status and notes only)

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
- crates/gameplay/dispatch/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring
- crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants
- crates/gameplay/durability/src/durability/core_modules.rs, domain/mod.rs, lib.rs module registration

## Wiring points (verified on origin/main 5742f30f3)

- ITEM-USE-1 use-with path (#2039/#2082) as the entry point
- durability/mod.rs registration

## Inseparable bundles (never split across PRs)

- migration + conversion writer (consume inputs + mint outputs + audit) in one transaction

## Acceptance criteria

- [ ] A conversion consumes inputs and mints outputs atomically or does nothing.
- [ ] Random outcomes are deterministic per the brief's RNG rule and reproducible in tests.
- [ ] Conversion table comes from content; unknown pairs refused with typed code.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation evidence (2026-10-23, PROVEN unless noted)

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --locked -p oteryn-gameplay-items -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch --all-targets --quiet -- -D warnings`: passed. The vendored upstream deprecation warnings are unchanged.
- `cargo test --locked -p oteryn-gameplay-dispatch --lib item_craft --quiet`: **1 passed on PostgreSQL**, 20.67 s. Real admitted-session wire snapshot/USE/result, partial-stack consume/mint, durable receipt/outbox and reload, owner restart replay, unknown pair, full-container refusal, final-unit retirement and slot reuse, stale handle; also admitted milling/cookie plans. Mounted by dispatch's existing library PG test tree.
- `cargo test --locked -p oteryn-gameplay-dispatch --lib world_required_capabilities --quiet`: **17 passed**.
- `cargo test --locked -p oteryn-gameplay-dispatch --lib gameplay_transport::capabilities::tests --quiet`: **44 passed**.
- `cargo test --locked -p oteryn-protocol-oteryn --lib world_object --quiet`: **21 passed**, including the existing bounded use-with codecs.
- `cargo test --locked -p oteryn-game-server --test content_world_project_repository --quiet`: **5 passed** (content-only target, no PostgreSQL suite).
- `python3 tools/content-migration/test_craft_runtime_profiles.py`: **3 passed**.
- `python3 tools/content-migration/register_spell_families.py --check`: passed; all 432 existing native policies remain identical, 22 cooking policies added, provider digest verified.
- `python3 tools/content-census/g4_item_crystal_binding_generator.py --check`: passed.
- `python3 tools/agents/validate_governance.py`, `python3 tools/repository/validate_repository_policy.py`, `python3 -m unittest discover -s tools/agents/tests` (**129 tests**), `bash -n tools/qualification/login_local/run.sh`, `git diff --check`: passed.
- Full `python3 tools/content-migration/regenerate_content.py --check`: registration, import, World/tree, materialized content, craft/imbuement/charm/proficiency/reward/starter validators passed; stopped at unchanged `tools/content-schema/item-authoring/test_engine_items.py` because the environment lacks `rfc3339-validator`. No dependency downloaded and no validator weakened. The remaining binding and content-repository checks were run separately and passed.
- D1462 self-review: complete diff/checklist reread; workspace-wide constructor/implementation/match searches for `CookingConversion`, `ItemCraftRequest`, `OneItemCraftV1`, `OneItemOperationV1` and `FreshAdmissionAuthority::use_craft`. Both new public Rust functions have production callers and coverage through the real dispatch. No request/result variant, command ID or capability ID added. No client or protected CI/policy paths changed.

### Unfinished validation

The full content regeneration check needs the existing `rfc3339-validator` dependency
available in its Python environment before `test_engine_items.py` can run. No unrelated
PostgreSQL suite was run. Integration/review and the larger mechanic scope remain as
listed above.

## Reference evidence pointers

- PRs #2039, #2082
- plan §4 row CRAFT-1
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 3), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Changed files in this worker

- `apps/game-server/migrations/0165_item_craft.sql`
- `content/content.lock.json`
- `content/craft/runtime-item-profiles.json`
- `content/items/definitions/spell-native-profiles.json`
- `content/items/index.json`
- `content/manifest.json`
- `content/spells.manifest.json`
- `crates/gameplay/dispatch/src/gameplay_transport/capabilities.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/capabilities_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/connection.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/item_craft.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/map_bundle_domain_postgres_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/mod.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/item_craft.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/world_required_capabilities_tests.rs`
- `crates/gameplay/durability/src/durability/core_modules.rs`
- `crates/gameplay/durability/src/durability/item_craft.rs`
- `crates/gameplay/durability/src/durability/item_craft/audit.rs`
- `crates/gameplay/durability/src/durability/item_mint_audit.rs`
- `crates/gameplay/items/src/item_craft.rs`
- `crates/gameplay/items/src/lib.rs`
- `docs/agents/tasks/CRAFT-1.md`
- `docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json`
- `docs/contracts/game-events/v1/native_one_item_transaction.proto`
- `tools/content-migration/craft_runtime_profiles.py`
- `tools/content-migration/register_spell_families.py`
- `tools/content-migration/test_craft_runtime_profiles.py`
- `tools/content-schema/craft-authoring/build_runtime_profiles.py`
- `tools/qualification/login_local/run.sh`
