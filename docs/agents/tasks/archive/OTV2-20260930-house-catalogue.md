# OTV2-20260930-house-catalogue

```yaml
task_id: OTV2-20260930-house-catalogue
title: HOUSES-5 - populate content/houses/ (contract §5 step 2)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/tender-mendel-06tjg2
issue: 162
lane_id: HOUSES
pr: null   # recorded in the FREEZE_SHA packet
base_sha: a795d5fe
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "house schema worker (claude-code-session-01AGaDHCuMKQ95cqTJBs5XRQ)"
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - content/houses/**
  - tools/content-schema/house-authoring/build_catalogue.py
  - tools/content-schema/house-authoring/README.md
  - .github/workflows/house-authoring-schema.yml
  - docs/agents/tasks/archive/OTV2-20260930-house-catalogue.md
public_contracts: []
depends_on: [OTV2-20260930-house-catalogue-owner-contract]
blocks: []
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

Owner direction 2026-09-30 (2a, D202-D205 on #162): populate `content/houses/` right after the House catalogue
owner contract is accepted. `content/houses/` holds 995 records in two shards, built by the new
`build_catalogue.py` from `convert_houses.convert` (client 15.30 staging, pinned CrystalServer sample, door item
ids) under contract §3. The directory marker is `POPULATED`. No runtime, migration, protocol or Area change; city
`Area` records are contract §5 step 3.

## Evidence

- 995 records: 878 `private_house`, 66 `guildhall`, 51 `shop`; 19 towns; 117,226 tiles; 5,372 doors.
- Every record validates as one catalogue (unique key, source id, engine id, name and door across shards).
- Keys are derived once; a rebuild keeps every committed key and revision by `provenance.source_id`
  (tested locally by editing a committed key and revision and rebuilding: both kept).
- Divergences for review (contract §5 step 2), unchanged from `samples/conversion-report.json`: engine `size`
  812, engine name 1, entrance not next to a door 27, entrance not next to a House tile 10. Official values win;
  `entrance` stays CrystalServer until derived (contract §3, §6).

## Validation (local)

- `build_catalogue.py --check`: ok (995 houses, 2 files); `convert_houses.py convert --check`,
  `verify_formal_schema.py`, `wiki_br_houses.py self-test`: ok.
- ruff check and format; `validate_materialized_game_tree.py`, `validate_governance.py`,
  `validate_repository_policy.py`, `git diff --check`: see the FREEZE_SHA packet.
- The workflow change extends the HOUSES-2 workflow's path filter to `content/houses/**` and adds the
  `build_catalogue.py --check` step.
