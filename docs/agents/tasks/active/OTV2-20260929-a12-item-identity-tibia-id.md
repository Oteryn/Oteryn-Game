# OTV2-20260929-a12-item-identity-tibia-id

```yaml
task_id: OTV2-20260929-a12-item-identity-tibia-id
title: "A12 Item identity equals the Tibia id (D146-D148)"
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: 1237
base_sha: b90f85c9
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01XdHJyZNPJMcmMnmSDgwQvZ (Sol Supervising Architect)
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_A12_ITEM_IDENTITY_TIBIA_ID_DECISION_2026-09-29.md
  - docs/architecture/reviews/OTERYN_GAME_A8_DONOR_ITEM_IDENTITY_EPOCH_DECISION_2026-09-28.md
  - docs/architecture/reviews/OTERYN_GAME_WO0_WORLD_OBJECT_AND_TERRAIN_AUTHORING_FORMAT_DECISION_2026-09-28.md
  - docs/architecture/reviews/OTERYN_GAME_STANCE0_CHARACTER_STANCE_PERSISTENCE_DECISION_2026-09-29.md
  - docs/architecture/OTERYN_G4_MULTI_SOURCE_IDENTITY_BINDING_DECISION.md
  - docs/agents/tasks/active/OTV2-20260929-a12-item-identity-tibia-id.md
  - docs/agents/tasks/active/OTV2-20260929-a11-stance-persistence-decision.md   # archive move after #1225
  - docs/agents/tasks/archive/OTV2-20260929-a11-stance-persistence-decision.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task records architect item A12 for the item-authoring escalation on #162 (5892865958). The
owner confirmed it directly as D146-D148:

- The canonical key of every Tibia item is its Tibia id.
- Semantic names become code constants.
- Oteryn-only items use their own namespace.

It supersedes A8 (D96, D97), amends G4 for CipSoft ids and amends the WO-0 D93 rule. It also
renumbers the A11 stance decision from D140 to D145, because D140 was already allocated.

No code, content, key or registry change is made. The ITEM-ID-1 migration lane follows.

## Architecture and source of truth

- `PROVEN`:
  - the item-authoring evidence in 5892865958;
  - the G4 decision;
  - A8 and WO-0 D93;
  - durable `definition_production_key` rows (`0010`, `0011`, `0014`);
  - D4 (#1218) holding D140-D144.
- `UNKNOWN`: which OT or donor ids lie outside the admitted CipSoft id set.

## High-risk authority/recovery qualification

Not applicable to this docs-only task. The decision changes canonical identity, so it needs an
independent exact-head identity review. ITEM-ID-1 needs its own review.

## Acceptance criteria

- [ ] The decision document is on an exact frozen head with passing validators.
- [ ] Independent exact-head review.
- [ ] Protected Merge Queue integration.

## Excluded scope

- Code, content, keys, bindings, migrations and WO-2.

## Finding dispositions

The Codex review of `225b350` raised four P1 findings. All were accepted and fixed in the repair
commit.

- 4135141605, removed ids: §4.1 defines the admitted CipSoft id set, which only grows. Current and
  retired keys follow from it, so 53161 keeps `oteryn:item.tibia.i53161` as a retired key.
- 4135141623, named-key ban: the ban covers authored content, code and new writes only. Historical
  durable rows keep named keys and resolve through the alias table.
- 4135141616, `EXACT` bindings: `EXACT` needs G4 identity evidence. Equal numbers only corroborate.
- 4135141634, alias totality: every retired key has exactly one entry. Each alias is derived from
  that key's own recorded Tibia-id evidence, and an unproven referenced key blocks ITEM-ID-1.

## Validation

- `python3 tools/agents/validate_governance.py`
- `python3 tools/repository/validate_repository_policy.py`
- `git diff --check`

## Context checkpoint

```yaml
last_progress: decision authored
status: validating
branch: claude/gifted-rubin-a0axzx
head_sha: null
pr: 1237
owner_action_required: null
blocker: null
next_action: "validate, open the PR, freeze, request review"
```
