# OTV2-20260926-monster-import-readiness

```yaml
task_id: OTV2-20260926-monster-import-readiness
title: Raise Canary monster import readiness and record familiar and corpse decisions
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: 951
jira: KAN-16
base_sha: d6c6c18eb1690208213b8871a30c992681ee4f2d
head_sha: 204f6dcbbb9bfe24701a4f7cd194c38ab235d82d
final_head_sha: 204f6dcbbb9bfe24701a4f7cd194c38ab235d82d
final_head_frozen_at: null
owner: claude-code-session-01UiMEDawVAZ3kxLCLepWZnG
created_at: 2026-09-26
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260926-monster-import-readiness.md
  - docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md
  - tools/content-schema/monster-authoring/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner request after PR #947: do the monster import work that nothing blocks. Map addons and
mounts, make the converter follow the engine on edge cases found by converting all 1,656 Canary
monster files, classify the creature events they name, and record owner decisions D16 (familiar
split) and D17 (any Item as corpse). A monster runtime in the server is out of scope: the server
has no creature spawn/AI/combat/death path and GAME-AI-01 (#275) is still a proposal.
Authority: direct owner request in this session; product/runtime implementation stays unallocated.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline authoring evidence and Python tooling only; no production mutation,
fence, session, authority or persisted-recovery evidence is touched.

## Acceptance and evidence

- `population_census.py` over 1,656 Canary monster files: 1,103 fully resolved (870 before),
  547 blocked, 6 not converted, 0 structure-invalid.
- `samples/events-canary-47dfd51f.json`: 193 creature events classified with evidence lines;
  only high-confidence quest, encounter-bookkeeping and no-effect events are omitted.
- Schema: `change_target.interval_ms` may be 0, `total_damage_range.minimum` may be 0,
  `appearance.selection=owner_familiar_look` (D16); validator drops the corpse-flag requirement
  (D17). `verify_formal_schema.py` 180/180.
- Batch 1 10/10 and batch 2 7/10 manifests resolve; `knight_familiar` look and thorn knight addons
  are mapped.

## Postmerge closeout

`PROVEN`: [PR #951](https://github.com/Oteryn/Oteryn-Game/pull/951) merged
`204f6dcbbb9bfe24701a4f7cd194c38ab235d82d` as
`150928e65c8a83f91ceb5b238ac1dda9a3b067eb` at `2026-09-27T06:07:19Z`.
[Reconciliation allocation](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5855464134)
confirms the prior lease is terminal/released and allocates this separate archive move.
The bounded delivery is completed; earlier acceptance results and owned paths remain
historical evidence. The monster runtime and GAME-AI-01 product gap remain outside scope.
`final_head_sha` identifies the merged PR head; its original freeze timestamp is
`UNKNOWN` and stays null. No new review, MQ, E2E or runtime qualification is claimed.
