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
| Waterskin of Mead, Magic Sword, Tibia Anniversary, Venore Daily Tasks, Voodoo Doll, Citizen of Issavi, Elvenbane, Colours of Magic (partial) | implemented by earlier lanes | parity review pending: Venore lacks the 20 h per-task cooldown and Norman's three separate tasks; Anniversary lacks the yearly window; Magic Sword level gate 100 is an authored deviation; Voodoo King doll id unverified. |
| 16 HELD mini world changes, seasonal events, Rathleton voting, Measuring Tibia, Last Creep Standing, Asura Palace, Child of Destiny, War Against the Hive missions, Insectoid Outfits (War Exp.), Cartography 101 (carried USE with a location gate) | server mechanic missing | sent to the server lane; not built here. |
| Top of the City, The Repenters, Royal Bounacean Advisor | data work, later batch | Repenters recipe has 8600 XP once; global is four daily tasks (2000/2100/2200/2300 XP) plus 2000 bonus, 20 h repeat. |

## Checks per batch

`run_checks.py --part checks` and `--part unit-tests`, `pin-check`, `cargo fmt`, clippy for the
changed crates, and the dispatch PG case (returns `Ok(())` without
`OTERYN_TEST_POSTGRES_ADMIN_URL`).
