# OTV2-20260924-unknown-disposition-ledger

```yaml
task_id: OTV2-20260924-unknown-disposition-ledger
title: G3 documented blockers for UNKNOWN source pages
mode: AUDIT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/full-content-g3-unknown-disposition-ledger-20260924
issue: 162
pr: 831
base_sha: ac3d69f1c20e09ce006e3efb0777d4e4f32275d1
head_sha: a9bb2cd30268a9a04cab9ba1e506b84da4699466
final_head_sha: a9bb2cd30268a9a04cab9ba1e506b84da4699466
final_head_frozen_at: 2026-09-24T11:05:31Z
owner: delegated Luna writer
merge_commit_sha: a63678d828a8f2fdb23f5ca3ae74e97ccf1978fe
merge_group_run_id: 35991280898
merge_group_game_gate: SUCCESS
g3_substantive_completion: false
g4_promotion: false
ownership_released: true
owned_paths:
  - .github/workflows/unknown-disposition-ledger.yml
  - tools/content-census/unknown_disposition_ledger.py
  - tools/content-census/unknown_disposition_ledger_self_test.py
  - docs/agents/evidence/OTV2-20260924-unknown-disposition-ledger.json
  - docs/agents/tasks/active/OTV2-20260924-unknown-disposition-ledger.md
created_at: 2026-09-24
updated_at: 2026-09-24
```

## Outcome

Produce a deterministic, exact-page-ID ledger for the 5,512 pages left `UNKNOWN` by G3. Each row states `DOCUMENTED_BLOCKER`, keeps its source family `UNKNOWN`, carries the pinned source evidence and a specific next action, and records current revision status as unverified. The complete ledger is a 14-day workflow artifact only; Git retains this compact manifest and task record.

## Pinned input

- G3 artifact `10801778929`, run `35986883931`, exact source head `20e8e5d99b7b42b6fd008e67029623ea7a4a8e3a`.
- ZIP SHA-256: `a63635e4cdfcd237cf69b5e2fe0471c30723e45fe3ab2ccc9095730a9337d2e7`.
- Embedded classified universe SHA-256: `1562382c66ad471a46309eaa5ae2fe9e7b3b1e3c8c2d5fc047ee8faa65bd1666`.
- The ledger retains full G3/G2 per-lane provenance. Pinned revision IDs and timestamps are historical observations; current revisions remain `UNVERIFIED` because the live wiki API was unavailable (403). No title-only family or identity inference is allowed.

## Page-primary source-shape partition

| Primary shape | UNKNOWN pages |
| --- | ---: |
| `STRUCTURED_PRIMARY` | 3,803 |
| `NO_INFOBOX_ITEM` | 1,399 |
| `STRUCTURED_ALTERNATE` | 187 |
| `REDIRECT` | 104 |
| `SOURCE_CLASSIFICATION_UNRESOLVED` | 12 |
| `INFOBOX_ITEM_PARSE_ERROR` | 7 |
| **Total** | **5,512** |

The full lane-observation count has 1,436 `NO_INFOBOX_ITEM` observations. Thirty-seven of these are secondary observations on dual-lane page IDs whose primary observation is `STRUCTURED_PRIMARY`; they are preserved but not counted as extra pages.

Within the 3,803 primary structured rows, direct template evidence gives 3,122 pages with Infobox Object (3,121 Object-only plus the one Object × World Quest overlap), 521 Infobox Hunts, 12 Infobox World Change, 18 Infobox World Quest (17 World Quest-only), and 131 with none of those four signatures. Page `19087` (`Christmas`) is the single overlap. Six additional Infobox Object observations belong to already classified Creature rows and are excluded from the 3,122 UNKNOWN subset.

The residual 131 are subdivided by deterministic discovery-root precedence into evidence blockers and typed next actions. Their complete roots, source role, surfaces, templates, categories, and pinned revision evidence remain in each artifact row. A discovery root documents retrieval context only; it does not assign a family.

## Blocker limits

- Infobox Object does not resolve WorldObject, LocalObject, Terrain, or placement.
- Infobox Hunts is an editorial guide shape with Area, Creature, and Encounter references; it does not prove a blanket Area definition.
- Infobox World Change may describe runtime state or a relationship and does not automatically define a static Encounter.
- Alternate source, redirect, absent-infobox, classifier-unresolved, and parse-error cases remain distinct blocker classes.
- The ledger makes no target identity selection, candidate relation resolution, gameplay semantic promotion, or runtime authority claim. The existing G0 WorldProject/v2 and definition/relationship/placement/runtime boundaries remain in force.
- Existing hard exclusions `Kalkulatory`, `Narzędzie do nasycania`, and `Dostawca` remain excluded.

## Validation

The same-repository-only workflow job uses the existing protected downloader to verify the exact G3 run success, artifact ID, head, name, expiry, size, member list, and archive digest without forwarding the bearer token across the storage redirect; checks embedded schema, authority, and universe digest; builds the 5,512-row ledger; checks the compact manifest against Git; runs focused counterexamples for dual-lane pages, six classified Creature/Object rows, page 19087 overlap, UNKNOWN Mount-list `Winterlight Solstice`, redirects, alternates, parse errors, duplicate IDs, corrupt archives, and unverified current revisions; and uploads only the complete ledger and manifest with 14-day retention.

## Terminal closeout

- PROVEN: PR #831 merged from exact frozen head `a9bb2cd30268a9a04cab9ba1e506b84da4699466` at protected `main` commit `a63678d828a8f2fdb23f5ca3ae74e97ccf1978fe`.
- PROVEN: merge-group run [35991280898](https://github.com/Oteryn/Oteryn-Game/actions/runs/35991280898) completed with aggregate `game-gate` SUCCESS.
- PROVEN: ledger covers 5,512 UNKNOWN pages, each with a documented blocker; undispositioned rows: 0. Source families remain UNKNOWN; current revisions remain UNVERIFIED. No identity, gameplay, or G4 promotion is claimed.
- TERMINAL: this bounded ledger task is completed and its authoring ownership is released. Further G3/G4 classification requires a new live allocation.
