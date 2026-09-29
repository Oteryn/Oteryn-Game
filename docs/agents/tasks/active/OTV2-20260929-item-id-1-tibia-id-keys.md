# OTV2-20260929-item-id-1-tibia-id-keys

```yaml
task_id: OTV2-20260929-item-id-1-tibia-id-keys
title: ITEM-ID-1 - Item key equals the Tibia id (A12 §5, D146-D149)
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/item-id-1b-key-switch   # 1a: claude/item-id-1-tibia-id-keys (#1279, merged 47e865d4)
issue: 162
lane_id: Content/World item identity
pr: null   # each slice's PR is recorded in its FREEZE_SHA packet on #162
base_sha: c0f7e238   # 1b admission main
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "ITEM-ID-1b task session (control plane session_01MnSvpbKjAZEdzEaFrwiu7D)"
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:   # ITEM-ID-1b (1a's are on main)
  - apps/game-server/src/content/item_identity.rs
  - apps/game-server/src/content/mod.rs
  - apps/game-server/src/content/cw2_b1_import.rs   # tests only; the importer stays history
  - apps/game-server/src/combat/loot_plan.rs        # test keys
  - apps/game-server/src/combat/pickup.rs           # doc comment
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_item_identity.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - apps/game-server/tests/content_world_project_v2_{creature,encounter,npc}_admission.rs
  - apps/game-server/tests/support/{combat_pickup,item_transfer}_postgres_cases.rs   # comments
  - content/** (regenerated; content/items/aliases.json unchanged, content/assets/files untouched)
  - imports/crystalserver/bindings/items.json
  - imports/tibiawiki/{batches,sources}.json, imports/tibiawiki/bindings/items.json, imports/tibiawiki/facts/{items-family-fallback,items-wave1}.json
  - tools/content-census/{item_id_alias_table,item_id_alias_table_self_test,item_key_references,g4_item_crystal_binding_generator,g4_item_crystal_binding_generator_self_test,g4_item_wave1_stage,g4_item_wave1_stage_self_test}.py
  - tools/content-migration/{world_project_v2_to_tree,validate_world_project_v2_to_tree,test_world_project_v2_to_tree}.py
  - tools/content-schema/item-authoring/** (keys, fixtures, owner decisions, ITEM_CATEGORY_NAMES)
  - tools/content-schema/world-object-authoring/** (A12 §4.6 numbering)
  - tools/content-schema/store-authoring/{verify_formal_schema.py,synthetic-valid-store-offer.json,test_convert_store.py}
  - tools/content-schema/npc-authoring/promotion_candidates.py
  - .github/workflows/{item-authoring-schema,g4-item-crystal-bindings}.yml   # owner Q20a
  - docs/agents/evidence/OTV2-20260929-item-id-1b-{identity-screening,q13d-family-decisions}.json
  - docs/agents/tasks/active/OTV2-20260929-item-id-1-tibia-id-keys.md
public_contracts: []
depends_on: []
blocks: [WO-2]
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Plan (split announced on #162, comment 5898989570)

Hand-written logic is well over the ~500-line batch budget, so the task is split at real seams.
Every key change still lands in one atomic PR (1b).

| Part | Scope | State |
|---|---|---|
| 1a | Membership manifests for the 4 admitted CipSoft files; alias table (38,562 retired keys); D149 tombstones; generators with `--check` and self-tests. Additive: no key changes. | merged #1279 (`47e865d4`) |
| 1b | The key switch: the §4.1 rule at the materializer boundary, semantic constants, key translation of the pinned evidence packets through the alias table, tree and crystal generator, regenerated content and imports, D149 removals, pins, no-dangling-ref check, crosswalk requalification. | this branch |
| 1c | Durable readers resolve retired keys through the alias table; tombstone live-reader test. | `SHARED_LEASE_REQUIRED`: every `definition_production_key` reader is in `durability/**` (DEATH-1) |

## D149 gate (first gate, #162 comment 5898910567): PASS

- The 4,590 records are the Crystal `ff7ede59` binding rows whose id is in no admitted CipSoft file.
- The fluid kinds (ids 1-20) are OT pseudo-records without semantics in content. Fluid meaning
  lives in `ReferenceFluidType` on container items and in creature `death_residue.fluid_type`
  strings on pool item 2886, which is current.
- 201 of the 4,590 carry a name; 199 of those names exist on a current Tibia id. No content or
  apps ref outside the registry mirror names any of the 4,590 keys.

## 1a facts

- PROVEN: all 4 admitted files match their pins (Crystal `6adb790d`, Canary `aa44a154`, donor
  `17a72b30`, client 15.30 `2dfa943b`). Union 43,517 ids, current 43,516, retired `[53161]`.
- PROVEN: Crystal `items.xml` at `ff7ede59` has no `clientid`, so each row's item id is its
  appearance object id (the §4.2 source appearance reference).
- PROVEN: 33,972 retired keys alias to 33,971 Tibia keys; the one shared target is `tibia.i3031`
  (`registry.i00002921` and `currency.gold_coin`, the same R7-P04 source row). 0 continuity
  breaks. 4,590 `RETIRED_WITHOUT_SUCCESSOR`, each with a tombstone.
- DERIVED: the record digest differs for every id across files, because sprite ids are repacked
  between builds. The note is kept as `EVOLVED`; it never affects the disposition.

## Assumptions (reversible, ungoverned)

- The alias table lives at `content/items/aliases.json` ("with the Item registry", A12 §4.5); the
  tombstones live in `docs/agents/evidence/` (outside authored content, §4.1), next to the other
  protected historical packets. Neither adds a tree directory node.
- Admission order: Crystal `ff7ede59`, Canary `47dfd51f`, donor `00ce02a5`, client 15.30.
- 9,545 current 15.30 ids have no Item record today. The key rule covers them; minting records for
  them is not part of ITEM-ID-1 (WO-2 and the item-authoring lane).

## Findings for the control plane

- CI wiring: no workflow runs the new `--check` paths or self-tests yet. Adding them needs a
  `.github/workflows/` edit (outside owned paths; it also triggers the full Rust lanes). Proposed:
  `item-authoring-schema.yml` runs `test_appearance_membership.py` and
  `appearance_membership.py --check`; `g4-item-crystal-bindings.yml` runs
  `item_id_alias_table_self_test.py` and `item_id_alias_table.py --check`, with path triggers for
  `content/items/aliases.json` and `imports/official/appearance-membership/**`.

## Validation (1a, local)

- `appearance_membership.py --check`: PASS (15.30 regenerated byte-identically; all digests and
  the index verified). The 3 engine manifests were regenerated from the pinned files, fetched from
  the pinned revisions.
- `test_appearance_membership.py`: 8 tests pass.
- `item_id_alias_table.py --check`: PASS. `item_id_alias_table_self_test.py`: PASS (synthetic rule
  cases; byte-identical regeneration; tombstone digests; 38,163 old keys named in `content/` and
  `apps/` all resolve).
- `g4_item_crystal_binding_generator.py --check` and self-test: PASS (unchanged).
- `test_world_project_v2_to_tree.py`, `validate_world_project_v2_to_tree.py`,
  `validate_materialized_game_tree.py`: PASS.
- `ruff check` / `ruff format --check` on the new tools: PASS.
- No Rust changed in 1a.

## 1b design

- The protected packets (CW2-B1 catalogue, epoch 2, R7-P04, promotion lowering, wave 1, creature
  and NPC staging) stay byte-exact history. The importer and the materializer admit them in the
  historical key space unchanged; `content::item_identity::apply_tibia_id_key_rule` then runs once
  on the draft, before canonicalization: every Item record takes `tibia_item_key(source id)`, which
  must equal its alias entry; the 4,590 D149 records leave content; every other Item identity is
  rewritten through the alias table and must name a kept record. A reference to a D149 key, a
  dangling reference or an Item key outside an Item identity fails closed.
- `ItemKeyAliasTable` admits the exact committed table bytes (size and sha256 pinned) and is the
  reader 1c's durable readers reuse.
- 65 semantic constants in `item_identity::semantic` (64 CW2-B1 names + R7-P04 gold coin).
- The alias table and the tombstones are frozen history: `item_id_alias_table.py --check` verifies
  them (genesis digests, append-only vs `--base-ref`, §4.5 rules, per-target allowlist
  `tibia.i3031` only) and no longer re-derives them. The self-test derives the 65 named keys from
  the Rust literals, independently of the table.
- Crystal bindings: every historical row (38,157 epoch-1 + 404 epoch-2) is requalified through its
  own alias entry: `ALIAS` -> `EXACT` to `tibia.i<external_id>` (33,971 rows); D149 -> no binding
  (its crosswalk record is the alias entry). The historical epoch-1 bytes still match their digest.
- `item_key_references.py`: no retired key outside 6 named history files, no dangling Tibia key,
  across `content/`, `imports/` and `apps/`.

## 1b facts

- PROVEN: content has 33,567 Item records, all `oteryn:item.tibia.i<id>`; 0 retired keys in
  content, imports or non-history apps files; 185,421 key references checked.
- PROVEN: materializing twice is deterministic (`tree_sha256=fa64bbbc...`); the tree regenerates and
  `validate_world_project_v2_to_tree.py` passes on 33,567 items.
- D149 gate name source (review obligation): the 201 named records are the tombstones with a KNOWN
  presentation name. 199/201 names exist on a current id by authored (OT) names, 184/201 by CipSoft
  15.30 names (only 8,978 of 43,516 15.30 objects carry a name). 12699 magic gold converter has
  current successors 28525/28526; 15792 crystal bolt is OT-only (no TibiaWiki page); 16087 purse is
  deprecated in Tibia (removed 10.8, no itemid). No semantics lost (tombstones keep every
  definition); no escalation. Evidence: `OTV2-20260929-item-id-1b-identity-screening.json`.
- Name screening (review obligation): 293 ALIAS rows whose CipSoft 15.30 name differs from the OT
  name (superset of the review's 178): 258 OT range names, 26 state/variant names, 9 soul-core
  monster labels. No repurposed id; no escalation. The soul-core labels are a content data-quality
  finding for the item-authoring lane (authored names come from OT).
- 65 vs 64 named keys: A12 D147 counts the 64 CW2-B1 batch keys; R7-P04 `currency.gold_coin` is the
  65th. Both historical keys of Crystal 3031 alias to `tibia.i3031` (the one allowlisted pair).
- `ITEM_CATEGORY_NAMES[26]` = `soul_cores` (835 objects in 15.30); 28-30 occur in no admitted file.
- Q13d: 176 HIGH approved; of 90 MEDIUM/LOW, 49 re-verified APPROVE, 41 DISPUTED (owner). 14
  approved rows with an engine binding (lists B/C) are in `owner-item-family-decisions.json`; 211
  approved rows wait for their record (client-only list A, WorldObject routes). Register:
  `OTV2-20260929-item-id-1b-q13d-family-decisions.json`.
- Lease: no edit under `apps/game-server/src/durability/**`, `domain/progression.rs` or
  `apps/game-server/src/interaction/**`; a grep finds no Item key literal there, so no reconcile
  list is needed for 1b (1c covers durable readers).

## Assumptions (1b, reversible)

- The item-authoring, world-object and store-authoring lanes key off the retargeted bindings, so
  their fixtures, the world-object key numbering (A12 §4.6) and the fallback facts moved with 1b;
  otherwise their CI would fail on the binding change. The historical samples under `samples/`
  stay as history.
- Synthetic test keys in `apps/` (for example `oteryn:item.chance.never`) are not retired keys and
  stay.

## Size (1b, reported on #162)

Hand-written logic is about 850 lines (tests about 700; generated content, bindings and mechanical
key rewrites excluded), over the ~500 budget. The key switch itself is atomic (A12 §5). The seam
that could be split out is 1b-0: the frozen alias verifier, `item_key_references.py`, the CI wiring,
the screening evidence, Q13d and `ITEM_CATEGORY_NAMES` (about 450 of those lines).

## Validation (1b, local)

- `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`: PASS.
- `cargo test --workspace`: 9,669 passed; the one failure of that run (a promoted-count pin in
  `content_world_project_repository`) was fixed and that target re-run: 3/3 PASS.
- Materializer twice: deterministic; the tracked `content/world` package is exact.
- `world_project_v2_to_tree.py` regenerates the tree; `test_`/`validate_world_project_v2_to_tree.py`,
  `validate_materialized_game_tree.py`, item master census: PASS.
- `item_id_alias_table.py --check --base-ref origin/main` and self-test; `item_key_references.py`;
  crystal binding generator `--check` and self-test; wave-1 capture/stage self-tests and
  `g4_item_wave1_stage.py --check`: PASS.
- item-authoring workflow steps (schema build/verify, validator, engine/lowering/donor/census/
  membership tests, `appearance_membership.py --check` with the 3 fetched pinned files),
  world-object tests, store-authoring steps, NPC promotion tests: PASS.
- Historical chain: `export_reference_item_identity_map` + classification crosswalk reproduce
  `004948ee...` unchanged.
- `ruff check` / `ruff format --check` (ruff 0.16.1) on the changed tool trees: PASS.
- `validate_governance.py`, `validate_repository_policy.py`, `git diff --check`: PASS.
