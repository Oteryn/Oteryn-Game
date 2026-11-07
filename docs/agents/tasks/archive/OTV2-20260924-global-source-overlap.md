task_id: OTV2-20260924-global-source-overlap
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
branch: agent/full-content-global-source-overlap-20260924
pr: 825
base_sha: dbc7fe950fa2eb5188e51860bac02e8889463261
head_sha: 9c333fbba17f60cbf8eb107ecb623c5cda679d6c
final_head_sha: 9c333fbba17f60cbf8eb107ecb623c5cda679d6c
merge_commit_sha: 8a0af6e098ceda84a11494aacb4f92f553bdbb27
merge_group_run_id: 35982769140
merge_group_game_gate: SUCCESS
ownership_released: true
owned_paths:
  - tools/content-census/global_source_overlap.py
  - tools/content-census/global_source_overlap_self_test.py
  - tools/content-census/global_source_overlap_fetch_artifacts.py
  - .github/workflows/global-source-overlap.yml
  - docs/agents/evidence/OTV2-20260924-global-source-overlap.json
  - docs/agents/tasks/active/OTV2-20260924-global-source-overlap.md

# OTV2-20260924-global-source-overlap

> Lifecycle: TERMINAL / COMPLETED. PR #825 merged at `8a0af6e098ceda84a11494aacb4f92f553bdbb27` after qualification of exact PR head `9c333fbba17f60cbf8eb107ecb623c5cda679d6c`. Merge-group run [35982769140](https://github.com/Oteryn/Oteryn-Game/actions/runs/35982769140) completed with aggregate `game-gate` SUCCESS. The G2 task is complete and its authoring ownership is released.

## Objective

Build a deterministic union of G1 live non-Item and protected Item page IDs. Deduplicate only by exact MediaWiki `page_id`; retain each lane's title observation and provenance, including legitimate title divergence across snapshots. Same-title rows with different IDs stay separate.

## Pinned inputs and limits

- G1 artifact 10798668295 / run 35977349690 / head `f1d7dbd6577b033c53545d8650ffba05aafc9480`.
- #807 Item crosswalk artifact 10778892407 / run 35925860576 / head `61d051a13329c51ae04d8a011655e279664c334c`.
- Require exact run/artifact metadata, ZIP digests, schemas, embedded digests, and row counts. G1 hard exclusions must be absent as attested by its pinned manifest.
- Preserve lane-specific titles and provenance. Item revision remains `UNKNOWN`; do not consume crosswalk dispositions, selected keys, or mapping conclusions.
- Do not select canonical identities or promote classifications/semantics to gameplay truth.
- Keep the full corpus in a 14-day workflow artifact; do not commit it.

## Validation and acceptance

Focused synthetic tests cover same-ID title divergence, duplicate IDs within one lane (fail closed), and same-title distinct IDs. The pinned hosted job checks artifact metadata, ZIP and embedded digests, derives counts and union arithmetic from inputs, and uploads only the corpus plus compact manifest. Preserve exact hosted run/artifact digests in evidence. Keep this record below 12,000 characters. Draft PR only; coordinator owns review, integration order, and Merge Queue.

## Terminal closeout

- PROVEN: PR #825's frozen head is `9c333fbba17f60cbf8eb107ecb623c5cda679d6c`.
- PROVEN: protected `main` is at merge commit `8a0af6e098ceda84a11494aacb4f92f553bdbb27`.
- PROVEN: merge-group run 35982769140 has aggregate `game-gate` SUCCESS.
- TERMINAL: task ownership is released; further work requires a new live allocation.
