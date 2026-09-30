# OTV2-20260929-tibiawiki-achievement-facts

```yaml
task_id: OTV2-20260929-tibiawiki-achievement-facts
title: ACHIEVEMENTS-2 - stage TibiaWiki achievement facts and join them to 15.30 staticdata
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/eloquent-goldberg-ie1mqf
issue: 162
lane_id: content population (reference source staging)
pr: null   # recorded in the FREEZE_SHA packet on #162
base_sha: d4cb72ee
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
final_head_sha: null
final_head_frozen_at: null
owner: "achievement facts worker (claude-code-session-01L687XUJAozgAPkNZ7B8GMG)"
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - imports/tibiawiki/achievements/**
  - tools/content-census/stage_tibiawiki_achievements.py
  - tools/content-census/stage_tibiawiki_achievements_self_test.py
  - docs/agents/tasks/archive/OTV2-20260929-tibiawiki-achievement-facts.md
public_contracts: []
depends_on: [OTV2-20260929-staticdata-houses-achievements-staging]
blocks: []
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

Owner-requested first step towards an Achievement schema (step 1 of 3: facts, then the Achievement owner
contract (D126) with the catalogue schema, then persistence). The 572 TibiaWiki achievements are staged as
reference facts under `imports/tibiawiki/achievements/2026-09-29/` and joined to the 368 staticdata observations.
The official tibia.com library list (owner-pasted; the site refuses the container) carries the same 368
non-secret achievements, 275/84/9 by grade, and names 204 secret ones without details; the wiki is the only
source for those and for points, secret and premium flags. `content/achievements/`, schemas and runtime are
untouched.

## Source verification

- PROVEN: every one of the 368 staticdata `source_id`s joins to exactly one wiki `achievementid`; 203 wiki-only
  achievements are all `secret = yes`; one wiki page has an inferred id (`563?`) and is left out of the join.
- PROVEN: a second fetch of the pinned revisions produced a byte-identical raw snapshot, and `check RAW`
  regenerates both committed files.
- REPORTED, not corrected: 57 joined records differ in name (2) or description (55); 10 anomalies (missing
  premium or points, grade 0, `Yes`, `?`).

## Validation (local)

- `stage_tibiawiki_achievements.py check` and `check RAW` (refetched pinned revisions): ok (2 files).
- `stage_tibiawiki_achievements_self_test.py`: 4 tests pass (join, uncertain id, prose dropped, gallery pipes,
  offline and raw tamper detection, unknown parameter, duplicate or invalid id, unterminated infobox).
- `ruff check` on the new tools: pass.
- Review: none required (reference-only staging data).
