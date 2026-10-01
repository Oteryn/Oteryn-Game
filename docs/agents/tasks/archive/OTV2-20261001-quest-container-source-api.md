# OTV2-20261001-quest-container-source-api

```yaml
task_id: OTV2-20261001-quest-container-source-api
title: Preserve and compile source container rewards with exact provenance
mode: WORK
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/quest-container-source-api-20261001
pr: null
base_sha: e225b3f76e152d195f75cb757b3d639a3ff5577a
head_sha: null
owner: codex-quest-data-parent
created_at: 2026-10-01T19:41:00Z
updated_at: 2026-10-01T20:20:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/reward-claim-authoring/container_source.py
  - tools/content-schema/reward-claim-authoring/container_source_verify.py
  - tools/content-schema/reward-claim-authoring/container_source.schema.json
  - tools/content-schema/reward-claim-authoring/container_source_README.md
  - tools/content-schema/reward-claim-authoring/test_container_source.py
  - tools/content-schema/reward-claim-authoring/samples/container-source/
  - .github/workflows/quest-container-source.yml
  - docs/agents/tasks/archive/OTV2-20261001-quest-container-source-api.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
```

D284/5937496910 authorizes this first bounded SOURCE batch under D277;
5937958248 binds the root single writer. Executor restoration/5939106890
supersedes the conditional API recovery: normal local Git publication preserves
exact commit identity using the META3.1.0-bound integrity helper. The branch name
is historical; no raw Git Data publication or frozen-stack rewrite is performed.

Input is full immutable #1469@503bba9e evidence, explicitly unmerged and
OTS_HYPOTHESIS_ONLY:65 records/68 placements/227 child roles/150 Item definitions.
SOURCE recipes:65 AUTHORED placements,3 CONFLICT,0 WAITING_DATA. Two charge-conflict
recipes and Helheim remain explicit. Barbarian200 is corroborated by pinned wiki;
donor execution100 is separately retained. Fluid WATER subtype, empty nested
containers and capacities24/22 preserve exact facts and native constraints.
No canonical registry, closed native vocabulary or runtime admission changes.

Local qualification:9 regressions PASS, strict schema and byte-exact regeneration
PASS; full M selection/26 Item shard blobs, direct donor binary fluid flags,
helper/code/schema/XML blobs and wiki line witnesses verified. Repository
checks and final whole-diff review are completed before publication. Exact-head
CI, independent frozen-head review and protected integration remain pending and
are recorded in PR/#162 without modifying the frozen head.

High-risk authority/recovery qualification: NOT_APPLICABLE; this compiler only
produces SOURCE data and neither mutates production nor authorizes a live writer.
