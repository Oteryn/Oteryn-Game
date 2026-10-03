# OTV2-20261003-daily-reward0

```yaml
task_id: OTV2-20261003-daily-reward0
title: "DAILY-REWARD-0: the Daily Reward System"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/daily-reward0-decision-20261003
pr: 1640
base_sha: 3d8c14d0f6326e278cf09e97acd2453b6af31926
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-016c5MQoe5CoMk9fxmuuMcFJ (Sol Supervising Architect)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_DAILY_REWARD0_DAILY_REWARD_SYSTEM_DECISION_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-daily-reward0.md
public_contracts: []
depends_on:
  - "MARKET-1 CharacterInbox (gates item delivery; lines stay PENDING until then)"
  - "PREY-1 wildcard balance (gates wildcard delivery; lines stay PENDING until then)"
  - "ADR-0021 World reset epoch (gates DAILY-1)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Allocation D294 (#1622, control plane) answers EXERCISE-0 R3, the PREY-0 wildcard source and the
Store-bound refusal deferrals with DAILY-REWARD-0.

- **Claims.** One per character per World reset epoch, at an adjacent reward shrine (temples and
  depots); the wall opens anywhere view only.
- **Lane and streak.** Lane day 1..7 moves only on claims; the streak grows on consecutive epochs,
  jokers (Account + World, max 3, one per month, granted lazily) cover gaps, else it resets to 1.
- **Rewards.** The TibiaWiki lane: vocation-filtered rune and potion picks, Prey wildcards,
  inert Gold Converter and Temple Teleport scroll, 50-charge training weapons, a pending XP boost.
  Items mint into the `CharacterInbox` with a `game_item_bindings` row; missing sinks keep durable
  `PENDING` lines delivered once later.
- **Bound items.** Refused by trade, Market, mail, Stash, NPC sale and Ground drop.
- **Deferred.** Resting-area bonuses, XP boost, converter and scroll use, Instant Reward Access,
  house shrines, the Store Inbox, Double Daily Reward events.

## Architecture and source of truth

- `PROVEN`: ADR-0021; BOSS-RAID-0 §2; MARKET-0 §5; PREY-0 §4; EXERCISE-0 §3-§5; PLAYER-TRADE-0
  §3; MAIL-0 §1; DUR-03 §5.5, §11.3, §14, §38.
- `CIPSOFT_OFFICIAL`: Tibia manual `interface.md` lines 157-160, `characters.md` lines 58 and 185,
  `products.md` lines 97 and 187.
- `TIBIAWIKI_STRUCTURED`: Daily Reward System rev 905700, Exercise Weapons rev 1126820.
- `OTS_HYPOTHESIS_ONLY`: Canary `04b83b5` daily reward module.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. DAILY-1 implements and tests the claim transaction with persistence
and value review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (persistence, value, protocol).
