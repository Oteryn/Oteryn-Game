# OTV2-20260929-a12-item-identity-tibia-id

```yaml
task_id: OTV2-20260929-a12-item-identity-tibia-id
title: "A12 Item identity equals the Tibia id (D146-D148)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: 1237
base_sha: b90f85c9
head_sha: eed19673d77c90c22d4c457361f6a494ed378ab0
final_head_sha: eed19673d77c90c22d4c457361f6a494ed378ab0
final_head_frozen_at: "2026-09-29 (#162 5893275379)"
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
  - docs/agents/tasks/archive/OTV2-20260929-a12-item-identity-tibia-id.md
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

- [x] The decision document is on an exact frozen head with passing validators.
- [x] Independent exact-head review.
- [x] Protected Merge Queue integration.

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

The owner-requested re-review of `448b8f1` raised one P1 and one P2 finding. Both were accepted
and fixed in the second repair commit.

- 4135209759, current-id membership: every admitted CipSoft file needs a digest-bound membership
  manifest, derived from the exact file bytes. A difference-only census is not enough. ITEM-ID-1
  must prove the current and retired partition from these manifests.
- 4135209769, terminal aliases: entries are versioned. A `retired_without_successor` entry may be
  superseded by an alias that records new evidence. An alias entry is never superseded.

## Validation

- `python3 tools/agents/validate_governance.py`
- `python3 tools/repository/validate_repository_policy.py`
- `git diff --check`

## Terminal integration

- PR #1237 merged through the Merge Queue on 2026-09-29 as `8af7f88e`.
- Reviews:
  - Codex on `225b350` raised four P1 findings.
  - The owner-requested re-review on `448b8f1` raised one P1 and one P2.
  - All were fixed by `eed1967`, and the owner decided to merge after green CI.
- Protected-main readback: all seven owned files are byte-identical to `eed1967`.
- The coordinator's clarification request (5893830130) and owner decision D149 are recorded in the
  follow-up task `OTV2-20260929-a12-clarification-d149`. The Work coordinator archived this record
  in #1250, and the follow-up adds these review details.

## Context checkpoint

```yaml
last_progress: protected-integrated as 8af7f88e; archived
status: completed
branch: claude/gifted-rubin-a0axzx
head_sha: eed19673d77c90c22d4c457361f6a494ed378ab0
pr: 1237
owner_action_required: null
blocker: null
next_action: null
```

## Closeout

- PR #1237 merged via Merge Queue: final head `eed19673d77c90c22d4c457361f6a494ed378ab0`, merge commit `8af7f88e031333b4f243287c58be7a8c9d4578ad`.
- Protected-main readback: all 7 files the merge changed are byte-identical between the final head and the merge commit.
- Ownership released on merge. Archived by the Work coordinator in the 2026-09-29 batch.
