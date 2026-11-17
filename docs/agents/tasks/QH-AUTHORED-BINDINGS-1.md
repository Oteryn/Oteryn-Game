# QH-AUTHORED-BINDINGS-1 — quest lane qh: 47 authored approximations to global parity

```yaml
task_id: QH-AUTHORED-BINDINGS-1
title: "Quest lane qh: playable authored bindings for 47 NATIVE_BINDINGS_PENDING quests"
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 1622
base_branch: main
branch: claude/happy-davinci-ijz3wy-qh
pr: null
owner: quest lane qh (content session on owner instruction, 2026-11-17)
created_at: 2026-11-17
updated_at: 2026-11-17
execution_policy: continuous_progress
owned_paths:
  - content/quests/authored-bindings/<qh quest>.json
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/qh_authored_cases.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs (three qh hook branches only)
  - docs/agents/tasks/QH-AUTHORED-BINDINGS-1.md
merge_authority: CP_ONLY (lane never opens, labels or merges PRs)
```

## Done bar (owner)

A quest is done when a real-dispatch PostgreSQL case plays it end to end and its rewards and
mechanics match Tibia global, built from and citing TibiaWiki (curid/oldid in the binding
`sources`). Content that exists only on OT servers is not built (owner, 2026-11-17).

## Route

Per-quest closed documents in `content/quests/authored-bindings/` (auto-registered, recipe digest
verified at boot). PG cases live in `qh_authored_cases.rs`; names `authored-qh-*` reach the
shared harness through one `is_case` hook for callback, world and exercise. Worlds are unmodified
crops of the committed B3 map (`source_world_crop::world`), so the case proves the native
placement, not a synthetic chest.

## Status per quest

| Quest | State | Evidence |
|---|---|---|
| Small Sapphire Quest | binding + PG case | chest 2472 at 32962,32429,7; bag 2853 with small sapphire 3029, 50 gold, Women Seduced (map 2822, curid 77571 oldid 1116983), Travel Expenses (document 637, curid 77572 oldid 1116984); premium; once. Book text is not minted (recipe adaptation). |
| Voodoo Doll Quest | implemented by an earlier lane; items verified | Voodoo Doll (King) itemid 3209 and Magic Light Wand 3046 match TibiaWiki. Open deviation: the native boxes are gated by an entry step, three witch kills and a north-then-south order that global does not have (tracks and PG cases are owned by questpc8; change needs a definition-track edit). |
| Magic Sword Quest | not built | TibiaWiki status Deprecated: not on Tibia global (owner rule 2026-11-17). |
| Waterskin of Mead, Tibia Anniversary, Venore Daily Tasks, Citizen of Issavi, Elvenbane, Colours of Magic (partial) | implemented by earlier lanes | parity gaps: Venore lacks the 20 h per-task cooldown and Norman's three separate tasks; Anniversary lacks the yearly window (SEASONWIN, server lane). |
| Child of Destiny, Asura Palace | covered on main | engenc bindings (`engenc-child-of-destiny.json`, `engenc-asura-palace.json`, main e2152d05); not duplicated. |
| Mini world changes (WCVAR), seasonal events (SEASONWIN), Last Creep Standing (m23_last_creep) | server lane | not built here. |
| Rathleton voting, Measuring Tibia, War Against the Hive missions, Insectoid Outfits (War Exp.), Cartography 101 (carried USE with a location gate), The Repenters (four 20 h tasks, corpse deadline, fire bug), Royal Bounacean Advisor (saplings-alive kill, night talk), The Lightbearer (world basin event) | server mechanic missing | sent to the server lane 2026-11-17; not built here. |
| Top of the City | blocked: NPC placement data | Native route exists in region x128 y122: sealed door 5289 at 32858,31374,6 opens Jimmy's room 32857-32858,31375-31380,6; pick spot (ground 351 + 8504) at 32803,31259,8 over tunnel 32803-32806,31257-31260,9; rope hole 475 at 32810,31253,8 into the cellar with crates 8771 at 32807,31250-31252,8; food crate 8772, Iriana's Chest 8773, Heaven Blossom 5921 (TibiaWiki curid 31073 oldid 1050120). Jimmy's only placement evidence is the wiki cell 32857,31383,6, which has no native tile; Canary/Crystal world-npc.xml (201189f8) have no Jimmy, and `convert_placements.py` admits only donor overrides or ±1 adjustments. Needs a placement-tooling decision from the content owner. The first-greeting assassin summon and the wrong-bridge resets are server mechanics. |

## Checks per batch

`run_checks.py --part checks` and `--part unit-tests`, `pin-check`, `cargo fmt`, clippy for the
changed crates, and the dispatch PG case (returns `Ok(())` without
`OTERYN_TEST_POSTGRES_ADMIN_URL`).
