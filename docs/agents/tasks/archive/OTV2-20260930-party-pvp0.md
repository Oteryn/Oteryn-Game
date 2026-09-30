# OTV2-20260930-party-pvp0

```yaml
task_id: OTV2-20260930-party-pvp0
title: "PARTY-PVP-0 parties and PvP"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-party-pvp-0
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
  - docs/architecture/reviews/OTERYN_GAME_PARTY_PVP0_PARTIES_AND_PVP_DECISION_2026-09-30.md
  - docs/agents/tasks/archive/OTV2-20260930-party-pvp0.md
  - docs/architecture/reviews/OTERYN_GAME_ATTACK0_ATTACK_TARGET_AND_AUTO_ATTACK_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_REFERENCE_FIRST_PLAYER_DEATH_DECISION_2026-09-28.md
  - docs/architecture/reviews/OTERYN_GAME_DEATH0_CHARACTER_DEATH_RECEIPT_DECISION_2026-09-28.md
  - docs/architecture/reviews/OTERYN_GAME_D3_CORPSE_CONTAINER_LOOT_WINDOW_DECAY_DECISION_2026-09-29.md
  - docs/architecture/reviews/OTERYN_GAME_CHARACTER_REVISION_ITEM_TRANSACTION_COMPOSITION_DECISION_2026-09-27.md
  - docs/architecture/MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md
public_contracts:
  - docs/architecture/MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

PARTY-PVP-0 decides parties and PvP at full Global parity (owner direction, 2026-09-30).

- **Parties:** World-scoped party, member and invitation tables (UUIDv7 PartyId, at most 50
  members, D109); invite a visible character, accept, decline, revoke, leave (not under a logout
  block), pass leadership, succession by invitation order; membership survives a channel switch;
  a node `PartyView` cache refreshed by a sealed hint on the CHAT-0 World relay; party chat room.
- **Party benefits** on the same channel only: shared experience (D118, activity window 2 minutes,
  the "battle sign" read as the PZ block), party immunity and friendly fire, the D3 party loot
  right, a `members_in_area` hook for `party_buff` and the monk rules.
- **PvP:** the World PvP type as a ruleset field (Optional, Open, Hardcore; Retro types refused
  until sourced); legality in the GAME-ABILITY-01 legality stage; 50% PvP damage; aggression
  relations; white, yellow, red, black and orange skulls; logout, PZ and durable 15-minute kill
  blocks; PvP state, unjustified point and revenge mark tables written in the victim's death
  transaction with no `CharacterRevision` advance; rolling 24 h / 7 d / 30 d windows.
- **Death:** PvP death test, red and black loss, Twist of Fate, Adventurer's Blessing, black skull
  respawn at 40 HP and 0 mana; the unfair-fight reduction recorded as 0 until sourced.
- **GUILD-WAR-0 hooks:** `war_between`, `on_player_kill`, `AssistLedger`, the war emblem slot.
- **Owner question P1** (answered): the PvP type is per-World configuration; Optional and Open
  PvP are both delivered; the first World launches as Optional PvP (stated assumption pending
  owner confirmation).

No code, migration or content change is made.

## Architecture and source of truth

- `PROVEN`: the PartyId, solo viable party rewarded, PvP secondary-pillar and social presence
  baselines; the scope matrix; GAME-CHANNEL-01; D109, D118, D121.
- `DERIVED`: ATTACK-0, CONDITIONS-0, DEATH-0, the first player death decision, D3, CHAT-0,
  GUILD-0 (candidates); the Tibia manual (`combat.md`, `characters.md`,
  `controls_communication.md`, `world.md`).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. PARTY-1, PVP-1 and PVP-DEATH-1 need persistence and security review;
PVP-RT-1 and PARTY-XP-1 combat review; PVP-WIRE-1 protocol review; PARTY-1 privacy review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (persistence, combat, security, privacy, protocol).
- [ ] Protected Merge Queue integration.
- [x] Owner answer to P1 recorded on #162 (2026-09-30).

## Excluded scope

- Code, migrations and content; guild wars, arenas, Party Finder, Party Hunt Analyser, Retro
  Worlds, Death Redemption, blessing sales.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the draft authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the draft authoring tree.
- `git diff --cached --check`: clean.
- Codex round-1 repair (PR #1402, 5 P1): durable PvP block deadlines, leader succession and
  leader-only party end, shared-XP remote members excluded, invite block/privacy check, `PartyView`
  full refresh and bounded staleness; validators re-run PASS.
- Codex round-2 repair (PR #1402, 4 P1, 2 P2): durable party check before PvP legality and
  `top_damage_party_id`, invite consent serialized on `character_root`, durable PvP ledger
  snapshot, channel-visibility setting, Leave under the full combat lock, invitee cap and
  invitation expiry; validators re-run PASS.
- Owner answers (2026-09-30, #162): P1 b and a — per-World PvP type; Optional and Open PvP both
  delivered; the first World launches as Optional PvP (stated assumption pending owner
  confirmation); validators re-run PASS.

## Closeout

- PR: the one named in the #162 FREEZE_SHA entry. Merge commit/result: its squash merge.
- Amendments follow the control-plane rule (#162 5912405163): pending on acceptance. Amended:
  ATTACK-0 §3 (and §4 by reference), the first player death decision §4.1 (and §4.5), DEATH-0
  §3.1, D3 §4.4, the composition decision rule 1, the scope matrix.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: draft authored; awaiting architect review and publication
status: completed
branch: claude/arch-party-pvp-0
owner_action_required: "confirm the P1 reading (b and a: first World Optional, Open available)"
blocker: null
next_action: "architect reviews and commits the owner-answer update"
```
