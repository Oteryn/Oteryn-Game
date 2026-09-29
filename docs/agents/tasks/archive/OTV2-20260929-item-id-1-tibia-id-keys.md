# OTV2-20260929-item-id-1-tibia-id-keys

```yaml
task_id: OTV2-20260929-item-id-1-tibia-id-keys
title: ITEM-ID-1 - Item key equals the Tibia id (A12 §5, D146-D149)
mode: IMPLEMENT
status: done   # 1a and 1b; 1c is a separate SHARED_LEASE_REQUIRED task
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/compassionate-albattani-s29syw   # 1b; 1a was claude/item-id-1-tibia-id-keys
issue: 162
lane_id: Content/World item identity
pr: null   # 1a and 1b PRs recorded in their FREEZE_SHA packets on #162
base_sha: 3eac57cd
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "ITEM-ID-1b writer (claude-code-session-01MFaQBaeFJYGSE3vwS1GDRU; claim #162 comment 5900305459); 1a by claude-code-session-01SrxQsHqQZW5LJEJcMT6fbD"
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:   # ITEM-ID-1a
  - imports/official/appearance-membership/**
  - imports/official/index.json
  - tools/content-schema/item-authoring/appearance_membership.py
  - tools/content-schema/item-authoring/test_appearance_membership.py
  - tools/content-census/item_id_alias_table.py
  - tools/content-census/item_id_alias_table_self_test.py
  - content/items/aliases.json
  - docs/agents/evidence/OTV2-20260929-item-id-1-d149-tombstones.json
  - docs/agents/tasks/active/OTV2-20260929-item-id-1-tibia-id-keys.md
owned_paths_1b:   # the key switch; every retired Item key reader and writer
  - apps/game-server/{src,tests,examples}/** (Item key constants, allocation, fixtures)
  - content/**/* (regenerated; content/assets excluded)
  - imports/{crystalserver,tibiawiki}/** (Item bindings, wave1 facts, family fallback, pins)
  - tools/content-census/{g4_item_*,item_id_alias_table*}.py
  - tools/content-migration/*world_project_v2_to_tree.py
  - tools/content-schema/{item,world-object,store,npc}-authoring/** (Item key literals, samples)
  - .github/workflows/{g4-item-*,item-content-*,item-authoring-schema}.yml
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json (DUR04-REFERENCE-ITEM-PROFILE-V1 rows)
  - docs/architecture/OTERYN_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE_V1.md
  - docs/agents/tasks/archive/OTV2-20260929-item-id-1-tibia-id-keys.md
public_contracts:
  - DUR04-REFERENCE-ITEM-PROFILE-V1 (count and byte maxima re-derived from D149; see 1b)
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
| 1a | Membership manifests for the 4 admitted CipSoft files; alias table (38,562 retired keys); D149 tombstones; generators with `--check` and self-tests. Additive: no key changes. | merged |
| 1b | The key switch: the §4.1 rule in `cw2_b1_import.rs`, semantic constants, key translation of the pinned evidence packets through the alias table, materializer, tree and crystal generator, regenerated content and imports, D149 removals, pins, no-dangling-ref check, crosswalk requalification. | this record, branch above |
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

## 1b outcome

- DONE: every Item key is `oteryn:item.tibia.i<id>`. The 64 named batch keys and the R7-P04 gold
  coin are code constants (`ITEM_*` in `cw2_b1_import.rs`, 65 in all = 64 + gold coin). The
  retired allocations (epoch 1, epoch 2, R7 rename, promotion lowering) stay in code as history
  verifiers; each public importer re-derives them, verifies their pins, then translates through
  the frozen alias table and records `evidence.retired-key`.
- DONE (D149): 4,590 Crystal rows leave the family. Items 38,157 -> 33,567 (68 shards); admitted
  promotion 14,927/12,301 -> 14,723/12,100 fields/items; wiki fallback 1,490 -> 1,471 records;
  bindings 33,971 (33,567 + 404 epoch 2).
- DONE: alias table and tombstones frozen by sha256 (`FROZEN_*` in `item_id_alias_table.py`); the
  one shared target (`tibia.i3031`) is an explicit allowlist entry.
- DONE: no-dangling-ref check. `item_id_alias_table_self_test.py` fails if any retired key appears
  in `content/` or `apps/` outside the two history-verifier files.
- DONE (Q20a): CI runs the alias self-test and `--check` (g4-item-crystal-bindings, triggered by
  `content/**` and `apps/**`), and the appearance membership tests and `--check`
  (item-authoring-schema).
- DERIVED: DUR04-REFERENCE-ITEM-PROFILE-V1 maxima follow the record count (33,567); index, body,
  artifact and pair byte bounds were recomputed from the regenerated artifact. No other limit moved.

## 1b assumptions (reversible)

- The 404 epoch-2 donor ids get Crystal bindings but no definitions (unchanged from B1b).
- The four history pipelines keyed by retired keys (crosswalk and its successors) call the
  exporter with `--retired`, which reproduces the pinned crosswalk byte-identically; the default
  export is the Tibia-key map. Re-running the creature and NPC staging tools also needs `--retired`.

## 1b review obligations (#162 comment 5900124035)

| Obligation | Status |
|---|---|
| Collision allowlist (`tibia.i3031`) | done |
| Alias table freeze | done |
| 65 vs 64 named keys | 64 batch + R7-P04 gold coin; both map to `tibia.i3031` via the allowlist |
| D149 name source (184/201 vs 199/201) | the 1a gate measured 199/201 named rows with their name on a current id; the review packet's 184 is not reconciled here. Reviewer to recheck against the tombstones |
| Name screening of the 178 renamed ALIAS rows | open for the independent identity reviewer; the alias table records both names |
| `ITEM_CATEGORY_NAMES` has 31 entries, not 26 | question for the control plane; no change made |
| Q13d table (#1282) | merged after 1b froze its scope; applied in follow-up 1d (owner-item-family-decisions, 90 rows to re-verify) |

## Findings for the control plane

- CI wiring (1a finding): resolved in 1b (Q20a).
- 1c (durable readers resolve retired keys) and WO-2 are unblocked once 1b merges.

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
