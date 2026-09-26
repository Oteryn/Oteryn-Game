# CONTENT-QUEST-01 r7 — Annihilator physical map binding witness

Status: **OTS_HYPOTHESIS_ONLY / PHYSICAL_SOURCE_BINDING / PROPOSED_NONCANONICAL**
Date: 2026-09-21
Tracking: #707 / PR #709
Evidence head: `0e3d83bce0ed5358b6ac9ad8f5b108105b740d66`
Workflow run: `35585892692`
Workflow job: `106289013612`

## Purpose

Close the binary-map precondition retained by r4/r6 for the selected Crystal Annihilator source without creating another OTBM parser.

Existing parser pin: `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce`.
Selected source: `zimbadev/crystalserver@ac447fef0935e6df52dc6b6376ae4c1534ecd73f`.
Map path: `data-global/world/world.otbm`.
Map Git blob: `e95e8f7c7a95d1b634b49a5dea5a5dc76021406b`.
Map bytes: `52267895`.
Map SHA-256: `09cce62af6c86644b5579fba460c674585261eb987ca5aa1f52baef9e91f8bbb`.

The workflow checked out the exact Game PR head, exact existing parser and exact Crystal source, verified the identities above, and streamed the OTBM with `tools.otbm_atlas.semantic.iter_map_records(..., strict=True)`. The job completed **SUCCESS** with `problems=[]`.

## Exact physical findings

| Source role | Legacy position | Static OTBM record |
|---|---|---|
| reward access door | `33216,31671,13` | ground `410`, item `5113`, flags `0` |
| spawn 1 | `33219,31657,13` | ground `410`, no top-level item, flags `0` |
| spawn 5 | `33220,31661,13` | ground `410`, no top-level item, flags `0` |
| spawn 2 | `33221,31657,13` | ground `410`, no top-level item, flags `0` |
| spawn 6 | `33222,31661,13` | ground `410`, no top-level item, flags `0` |
| spawn 3 | `33223,31659,13` | ground `10145`, no top-level item, flags `8` |
| spawn 4 | `33224,31659,13` | ground `10145`, no top-level item, flags `8` |
| lever | `33226,31671,13` | ground `410`, item `2772`, flags `0` |
| Demon Armor selector | `33227,31656,13` | ground `410`, item `2472`, flags `9` |
| Magic Sword selector | `33229,31656,13` | ground `410`, item `2472`, flags `9` |
| Stonecutter Axe selector | `33231,31656,13` | ground `410`, item `2472`, flags `9` |
| Present selector | `33233,31656,13` | ground `410`, item `2472`, flags `9` |
| room exit | `33236,31655,13` | ground `410`, item `1949`, flags `9` |

For every inspected target tile `house_id = null`. No stored `action_id` or `unique_id` was observed on the inspected OTBM records.

## Closed source-composition gap

The earlier fail-closed edge was that startup tables and Lua scripts did not prove the physical object existed in the binary map. For this exact source that edge is now closed for the listed lever, door, four reward selectors, room exit and six spawn coordinates.

The qualified Crystal source binding is a join, not one legacy number:

```text
exact world.otbm tile/object
+ exact data-global startup-table entry
+ exact registered Lua handler
= qualified source binding
```

### Lever

The static map contains item `2772` at `(33226,31671,13)`. r3 already established that the selected startup table stamps UID `30025` there and the selected Annihilator script registers `uid(30025)`. The UID is dispatch/provenance, not native Oteryn identity.

### Reward selectors

The static map contains four item-`2472` chests at the exact positions used by `ChestUnique[6085..6088]`. The inspected OTBM records carry no unique IDs at those positions; startup stamping plus the generic reward handler binds the selectors. This strengthens r2: the four physical selectors must not become four independent native entitlements.

### Door and exit

The static map contains item `5113` at the source reward-access door position and item `1949` at the room-exit position. Gameplay meaning still comes from qualified startup/script/teleport composition; physical presence alone is not canonical gameplay semantics.

## Spawn-tile consequence

All six authored spawn coordinates exist, but they are not physically homogeneous. Four have ground `410` with flags `0`; two have ground `10145` with flags `8`. No semantic meaning is inferred here from those numeric values. The importer must preserve the exact map projection and must not flatten all six coordinates into one guessed arena-floor state merely because one Lua table groups them together.

The static map still does not prove runtime spawn success, kill-all qualification, Quest completion, or Global target geometry.

## Provenance refinement

The protected fresh-source profile and selected `ac447f...` revision share the exact same map blob/digest. Physical map evidence may therefore be reused only when exact map-byte identity is proven. Scripts, startup tables, NPCs and reward behavior remain repository/revision/path qualified and cannot be inherited from map digest alone.

Conceptually:

```text
MapSourceRef = exact map blob/digest + path
BehaviorSourceRef = repository + revision + path/blob
BindingEvidence = explicit join of qualified refs
```

## Architecture consequence

No owner change. Quest does not gain map/collision authority. Encounter still composes through existing runtime world-overlay or InstanceRuntime owners. Startup UID/action numbers remain importer provenance. Global Reference parity is unchanged: these coordinates and item IDs remain Crystal/OTS evidence, not target truth.

## New required tests

- **T72 — exact physical source binding:** required static object missing or different at the qualified coordinate fails linking rather than being synthesized.
- **T73 — map/startup/script composition:** map object without expected startup/handler, or startup binding without expected map object/handler, remains unresolved; numeric ID equality cannot replace the qualified join.
- **T74 — spawn-tile heterogeneity preservation:** grouped spawn coordinates retain exact source world projection; importer/Encounter cannot normalize them to one guessed tile state.
- **T75 — same map blob across source revisions:** exact map-byte identity may reuse physical-map evidence while behavior remains revision-qualified.

Required CONTENT-QUEST-01 corpus is now **75 cases**. This workflow proves the selected source precondition represented by T72; it is not a production gameplay test.

## Evidence classification

```yaml
physical_source_binding:
  classification: OTS_HYPOTHESIS_ONLY
  map_identity: PROVEN_FOR_PINNED_SOURCE_BYTES
  parser_identity: PROVEN_FOR_PINNED_PARSER
  required_static_objects: PROVEN_FOR_PINNED_SOURCE_BYTES
  stored_action_unique_ids_at_targets: ABSENT_IN_OBSERVED_OTBM_RECORDS
  startup_dispatch_composition: SOURCE_DERIVED_FROM_R3_PLUS_R7
  runtime_spawn_success: NOT_PROVEN
  quest_completion_condition: EVIDENCE_REQUIRED
  global_target_geometry: EVIDENCE_REQUIRED
  global_target_reward_semantics: EVIDENCE_REQUIRED
```

This witness composes with r1-r6 and closes the exact binary-map precondition for the selected Crystal source without promoting it to Reference truth.
