# OTV2-20261003-monster-d15-bestiary

```yaml
task_id: OTV2-20261003-monster-d15-bestiary
title: "MONSTER-D15-BESTIARY: Bestiary profiles from the reference-date TibiaWiki"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/monster-d15-bestiary
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
  - docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md
  - docs/architecture/reviews/OTERYN_GAME_MONSTER_D15_BESTIARY_DECISION_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-monster-d15-bestiary.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner decision 3b (#1622 comment 5968564302) extends D15 as schema row D48. Every Bestiary field
comes from the reference-date English TibiaWiki, pinned by revision. Class and taxonomy come from
`bestiaryclass`. Thresholds and charm points are derived by `Template:Bestiary Table` revision
1152628. Membership needs both the official client race table and the wiki. Conflicts are
reported and ruled in batches. Allocation: D286 (#1622 comment 5968568302).

No code or content change is made. The converter child MONSTER-D15B-1 needs its own allocation.

## Architecture and source of truth

- `PROVEN`: monster schema D15, D33, D44, D47, §9.1, §10.3; `imports/cipsoft-staticdata/creatures/`;
  TibiaWiki `Template:Bestiary Table` rev 1152628, `Bestiary/Difficulties` rev 1132172,
  `Bride of Night` rev 1192294, `Muglex Clan Footman` rev 1203216.
- `DERIVED`: the meaning of the client tiers `f4`/`f5`, used only as a cross-check.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. No persistence, fencing, protocol or authority change.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (content).
- [ ] Protected Merge Queue integration.

## Excluded scope

- Converter code, population re-conversion, Bosstiary, `locations`, Bestiary runtime and wire.

## Validation

- `python3 tools/agents/validate_governance.py`, `git diff --check`, and the architecture checks
  selected by the changed paths, run before the push.

## Self-review

- Method: whole-diff reread against the owner decision and the D15 family (D23, D25, D32, D43,
  D44, D47).
- Verdict: no open finding at freeze.

## Independent review

- required: YES (decision document, D245). The control plane triggers it on the frozen head.

## PR and closeout

- Record archived in the final authoring commit.
