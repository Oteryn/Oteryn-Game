# OTV2-20260930-house-catalogue-closeout

```yaml
task_id: OTV2-20260930-house-catalogue-closeout
title: HOUSES-6 - close the static House catalogue (entrance and beds final, reports for the base map)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/tender-mendel-06tjg2
issue: 162
lane_id: HOUSES
pr: 1366
base_sha: ff09445f
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "house schema worker (claude-code-session-01AGaDHCuMKQ95cqTJBs5XRQ)"
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/OTERYN_HOUSE_CATALOGUE_OWNER_CONTRACT_V1.md
  - tools/content-schema/house-authoring/convert_houses.py
  - tools/content-schema/house-authoring/otbm_tile_check.py
  - tools/content-schema/house-authoring/samples/**
  - tools/content-schema/house-authoring/README.md
  - docs/agents/tasks/archive/OTV2-20260930-house-catalogue-closeout.md
public_contracts: [docs/architecture/OTERYN_HOUSE_CATALOGUE_OWNER_CONTRACT_V1.md]
depends_on: [OTV2-20260930-house-catalogue, OTV2-20260930-area-catalogue]
blocks: []
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

The static House catalogue is closed. Contract §3 makes `entrance` (engine entry tile) and `beds` (official count)
final, §5 records steps 1-3 as done, and §6 moves bed and door item placements to the base-map lane. The
conversion report lists the 46 entrances that are not in front of an outer door, for the walkability check when
the base map is compiled. The local engine-map check lists the 84 houses whose engine bed items are not two per
official bed, for the base-map owner. No `content/houses/` record changes. Housing runtime stays contract §5 step 4.

## Owner direction (2026-09-30)

Verbatim: "mozemy te domy finalnie domknac?", then `1tak, 2 tak, 3 trak` to:
1a engine `entrance` final, the 46 to the base-map walkability check;
2a official `beds`, the 84 bed divergences to the base-map owner;
3a open this closing PR.

## Evidence

- PROVEN: the client layout carries no bed items (0 in every house), so bed items are base-map placements.
- PROVEN: the client ships no tiles outside House layouts, so entrance walkability cannot be derived from it.
- DERIVED: engine entrance is the tile in front of an outer door (a door neighbour outside every House layout) for
  949 of 995 houses; engine bed items are two per official bed for 911 of 995.

## Validation (local)

- `convert_houses.py convert --check`, `extract-door-items --check`, `extract-bed-items --check`,
  `otbm_tile_check.py --check` (pinned map), `build_catalogue.py --check` (unchanged catalogue),
  `verify_formal_schema.py`, `wiki_br_houses.py self-test`: ok.
- ruff 0.16.1, governance, repository policy: pass; architecture semantic audit: NOT_APPLICABLE.
- CI green on `416e45ba`. Review: independent review of `416e45ba` FIX (issuecomment-5913729459): contract §5
  counted step 1 as done while the contract is still a candidate, and this record lacked the PR number and review
  state; both fixed in the next head, which is re-frozen on #162.
