# OTV2-20260930-boss-raid0

```yaml
task_id: OTV2-20260930-boss-raid0
title: "BOSS-RAID-0 bosses, raids and the Bosstiary"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-boss-raid-0
pr: "the PR named in the #162 FREEZE_SHA entry"
base_sha: "origin/main at branch creation"
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-01KbqAgmFfAYDSKHKkFmWKWW (Sol Supervising Architect)
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_BOSS_RAID0_BOSSES_RAIDS_AND_BOSSTIARY_DECISION_2026-09-30.md
  - docs/agents/tasks/archive/OTV2-20260930-boss-raid0.md
  - docs/architecture/MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md
  - docs/architecture/reviews/OTERYN_GAME_VSL_COMBAT_RESOURCE_ROWS_DECISION_2026-09-28.md
  - docs/architecture/OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md
  - docs/architecture/reviews/OTERYN_GAME_CHARACTER_REVISION_ITEM_TRANSACTION_COMPOSITION_DECISION_2026-09-27.md
  - docs/architecture/reviews/OTERYN_GAME_CHARM0_BESTIARY_CHARM_PROGRESSION_DECISION_PACKET_2026-09-29.md
public_contracts:
  - docs/architecture/MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

BOSS-RAID-0 decides bosses, raids and the Bosstiary under the owner's direction (2026-09-30:
build now, full Tibia Global parity).

- **Raids:** content schedule (random with a minimum gap, or fixed), one durable World firing per
  draw, one claimed run per channel, announcements per channel, no catch-up and no re-run after a
  restart.
- **Open-world bosses:** a durable spawn clock per (World, channel, spawn), advanced by the death.
- **Boss rooms:** lever admission of the whole group into one activity instance (the instance
  baseline, D26, SCOPE-HANDOFF-1), durable per-character cooldowns, compensation if nobody arrives.
- **Rewards:** contribution-based personal draws for reward bosses into a `CharacterRewardChest`
  with a 7-day expiry; anti-hopping eligibility per raid firing.
- **Bosstiary:** kill receipts on the Bestiary pattern, derived levels and boss points, two boss
  slots, a daily Boosted Archfoe per reset.
- **Architect rulings** R1a, R2a, R3b (raid channels, open-world hopping, rewards after a crash);
  **owner answer** R4a (2026-09-30, #162): the slot swap fee is a gold sink as in Tibia, the first
  change after a server save free.

No code, migration or content change is made.

## Architecture and source of truth

- `PROVEN`: the scope matrix; ADR-0001; the instance baseline; the encounter format; DUR-03 A4;
  D77, D109, D3; CHARM-0 and `0019`; QUEST-STATE-0 §5.2; ADR-0021; content counts.
- `DERIVED`: HOUSE-RUNTIME-0 and PARTY-PVP-0 (candidates on branches); the Tibia manual.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. RAID-1, BOSS-1, BOSS-ROOM-1, BOSS-REWARD-1 and BOSSTIARY-1 need
persistence review; BOSS-REWARD-1 economy and security review; BOSS-WIRE-1 protocol review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (persistence, economy, security, concurrency, protocol).
- [ ] Protected Merge Queue integration, after HOUSE-RUNTIME-0 for SCOPE-HANDOFF-1.

## Excluded scope

- Code, migrations and content; the encounter runtime (ENCOUNTER-RT-0); World Changes; the
  Tibiadrome; the Party Finder; the Boosted Creature; Hazard; spectators.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.
- `git diff --cached --check`: clean.
- Codex round-1 repair: ALIVE clock owner generation (§5), death-record bonus snapshot (§8.1),
  singleton open-world eligibility row (§9), top-50 contributors (§7), one free slot change per
  character per reset epoch (§10.3).
- Codex round-2 repair: 200-entry contribution accumulator before top-50 selection (§7,
  `BOSSRAID0-RL-10b`); session-fenced reward continuation in the credited character's own admitted
  session (§8.1, §12, composition amendment).
- Owner answer applied (2026-09-30, #162): R4a, the slot swap fee as a D178 gold sink, coins then
  bank, first change per reset epoch free (§10.3, brief, §14, §16). Validators re-run PASS.

## Closeout

- PR: the one named in the #162 FREEZE_SHA entry. Merge commit/result: its squash merge.
- Amendments follow the control-plane rule (#162 5912405163): pending on acceptance.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: draft authored; not committed
status: completed
branch: claude/arch-boss-raid-0
owner_action_required: null
blocker: null
next_action: "architect review, freeze and publish"
```
