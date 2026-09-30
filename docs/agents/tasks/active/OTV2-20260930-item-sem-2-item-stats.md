# OTV2-20260930-item-sem-2-item-stats

```yaml
task_id: OTV2-20260930-item-sem-2-item-stats
title: ITEM-SEM-2 Item stats (requirements, elements, leech, skills, slots, classification, weight) into content
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/compassionate-albattani-s29syw   # after #1319; one writer branch
issue: 162
pr: null   # per part, recorded in the FREEZE_SHA packets on #162
base_sha: 54c9ca18
owner: owner-directed Claude Code session, claim #162 comment 5907977795
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:   # ITEM-SEM-2a
  - tools/content-census/item_wiki_stats_capture.py
  - tools/content-census/item_wiki_stats_capture_self_test.py
  - imports/tibiawiki/facts/items-stats.json
  - imports/tibiawiki/sources.json
  - imports/tibiawiki/batches.json
  - docs/agents/tasks/active/OTV2-20260930-item-sem-2-item-stats.md
public_contracts:
  - DUR04-REFERENCE-ITEM-PROFILE-V1 (re-derived in 2b)
jira: null   # sync pending (coordinator batch)
```

## Why

The owner found that Soulcrusher (`tibia.i34086`) in content has only attack, defense and defense modifier; level,
vocation, ice attack, imbuement slots, classification, leech, club fighting and weight are UNKNOWN. The Item model has a
place for all of them except the forge classification; the promotion lowering v1 admits only 9 field paths. The
owner asked for every item that should have these fields ("zrób to dla wszystkich itemów").

Crystal's own values are not Tibia's: Soulcrusher in Crystal `ff7ede5` has club +5, life leech 5% and mana leech 3%,
TibiaWiki (Tibia 15) says club +4, life leech 2%, mana leech 1%. The source policy ranks tibia.com, then TibiaWiki;
Crystal and Canary are hypotheses. So the facts come from TibiaWiki, with Crystal as a fallback only where the wiki is
silent and never where it disagrees.

## Plan

| Part | Scope | State |
|---|---|---|
| 2a | Pinned TibiaWiki stat snapshot for every `Infobox Object` page with an item id (`items-stats.json`), capture tool and offline self-test | this branch |
| 2b | Promotion lowering v2: typed values from the snapshot (wiki first, Crystal fallback, conflicts recorded) for requirements, hands and slot, weapon type, elemental attacks, imbuement slots, leech and critical hit, skill boosts, resistances, weight; Rust decoder/apply v2; content regen; DUR04 re-derived | next |
| 2c | Model gap: forge upgrade classification on `ReferenceItemSemantics` (needs contract review) | after 2b |
| — | Proficiency stays ITEM-PROF-1 | — |

## 2a facts

- PROVEN (capture 2026-09-30T09:13:32Z): 9,980 `Infobox Object` pages, 9,354 with an item id; 13,826 item ids carry
  at least one admitted stat; 12,560 of them are Items in content. 185 ids are listed by more than one page (kept as
  separate observations). 6 pages have a malformed `itemid` and are reported, not captured.
- Coverage among content Items: weight 6,379; required level 1,269; upgrade classification 983; vocation 840;
  hands 737; defense 733; attack 643; imbuement slots 632; attributes 504; armor 470; resistances 417.
- Only admitted infobox parameters (`STAT_PARAMS`) and page/revision identity are stored; no article text.

## Owner questions (batched)

- Workflow authorization to run `item_wiki_stats_capture_self_test.py` in CI (item-authoring-schema), as for D179.
- Permission to push ITEM-SEM-2 on its own branch while #1319 waits for review (one writer branch otherwise).
