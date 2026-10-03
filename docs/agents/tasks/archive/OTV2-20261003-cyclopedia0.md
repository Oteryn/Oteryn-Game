# OTV2-20261003-cyclopedia0

```yaml
task_id: OTV2-20261003-cyclopedia0
title: "CYCLOPEDIA-0: Cyclopedia map discovery and area donations"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/cyclopedia0-discovery-donations
pr: "the PR named in the #1622 FREEZE_SHA entry"
base_sha: "origin/main at branch creation"
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-016c5MQoe5CoMk9fxmuuMcFJ (Sol Supervising Architect)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_CYCLOPEDIA0_MAP_DISCOVERY_AND_AREA_DONATIONS_DECISION_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-cyclopedia0.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner decision 1a (#1622 comment 5968564302) admits Cyclopedia map discovery and area donations
as in Tibia. The decision fixes per-character discovery (one active subarea, 7 POIs, premium),
its derived unlocks and grants, per-World donation pools burned under BANK-FEE-0 §3 with a new
`FeeBurnCause::AreaDonation` (amended into DUR-03 by the child CYC-DONATE-1), selection at
`WorldReset` (33% random area, donation area at 10,000,000 or more) and the 2x respawn factor in
the CREATURE-AI-0 §6.4 hook. Unproven values are `PARITY_PENDING`. Allocation: D286 (#1622
comment 5968568302).

No code, content or contract change is made. Each child needs its own allocation.

## Architecture and source of truth

- `PROVEN`: `docs/reference/tibia-manual/interface.md:35-44, 260`; `imports/cipsoft-staticdata/map/`;
  TibiaWiki `Cyclopedia` rev 1197515, `Measuring Tibia Quest` rev 935112,
  `Measuring Tibia Quest/Spoiler` rev 1198901, `Rapid Respawn Events` rev 1199249,
  `Point of Interest Effect` rev 848847, `Point of Interest Found Effect` rev 848848,
  `Discoverer Outfits` rev 1099576.
- `DERIVED`: tile-to-subarea membership from the client overlay images (decoded by CYC-CONTENT-1).
- `OTS_HYPOTHESIS_ONLY`: crystalserver#812 (unmerged), reference only.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. The persistence, economy and protocol shapes are reviewed again in
each child.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (persistence, economy, protocol).
- [ ] Protected Merge Queue integration.

## Excluded scope

- Runtime, migrations, content, wire numbers, the DUR-03 amendment text, the other Q1a systems,
  Bestiary, Bosstiary, Charms, Character and Houses tabs.

## Validation

- `python3 tools/agents/validate_governance.py`, `git diff --check`, and the architecture checks
  selected by the changed paths, run before the push.

## Self-review

- Method: whole-diff reread against the owner decision, the official manual, the multichannel
  scope matrix, BANK-FEE-0, DUR-03 and CREATURE-AI-0 §6.4-§6.5.
- Verdict: no open finding at freeze.

## Independent review

- required: YES (decision document, D245). The control plane triggers it on the frozen head.

## PR and closeout

- Record archived in the final authoring commit.
