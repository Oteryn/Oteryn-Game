# QH-AUTHORED-BINDINGS-1 — quest lane qh: 47 authored approximations to global parity

```yaml
task_id: QH-AUTHORED-BINDINGS-1
title: "Quest lane qh: playable authored bindings for 47 NATIVE_BINDINGS_PENDING quests"
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 1622
base_branch: main
branch: claude/happy-davinci-ijz3wy-qh-b2 (batch 2; batch 1 frozen at 447c051 on claude/happy-davinci-ijz3wy-qh)
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
  - tools/content-schema/npc-authoring/{chosen_placement_adjustments.py,npc_placement_scope.json,test_qh_missing_tile_cell.py} (batch 2)
  - tools/content-schema/quest-authoring/qh/ and one input of tools/content-schema/native-gameplay/authored-npc-item-custody.json (batch 2)
  - regenerated NPC placements, native item profiles and pins (by their tools only)
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
| Top of the City | binding + PG case (batch 2) | `top_of_the_city_quest.json`: Iriana `quest` (0→1), Jimmy `iriana` (1→2), pick 3456 on dirt 8504 at 32803,31259,8 teleports to the tunnel 32803,31259,9 (stage 2 only; the world callback validator refuses a read-only gate with item-on-target), crate 8771 at 32807,31252,8 gives food crate 8772 (2→3), Jimmy `quest` takes 8772 and gives Iriana's Chest 8773 (3→4), Iriana `quest` takes 8773 and gives 10 Heaven Blossom 5921 (4→5, bound to the recipe `complete` stage s6, completes). Premium. Sources TibiaWiki curid 30757 oldid 1059697, curid 31073 oldid 1050120, Jimmy curid 30848 oldid 1114684 at 32857,31383,6 on the stub the `qh_jimmy_stub` overlay restores. Iriana placed at 32849,31300,7 (Canary/Crystal agree); door 5289 at 32858,31374,6 has no action id, so it is a plain door and needs no binding. Food crate and chest got native custody through `authored-npc-item-custody.json`. PG case covers the NPC chain and the cellar crate on a one-floor crop; the dig teleport is cross-floor and is covered by the binding's boot validation only. Approximations: no first-greeting assassin summon, no wrong-bridge resets on the roofs (server lane), and the `yes` confirmations are folded into the keyword step. |

## Checks per batch

`run_checks.py --part checks` and `--part unit-tests`, `pin-check`, `cargo fmt`, clippy for the
changed crates, and the dispatch PG case (returns `Ok(())` without
`OTERYN_TEST_POSTGRES_ADMIN_URL`).

## OPEN-FACTS (owner browser check)

- Closed (qh-jimmytiles, 2026-11-17): the map gap at Jimmy is closed. TibiaWiki and the Polish
  TibiaWiki both show Jimmy at 32857,31383,6 (owner browser evidence 2026-11-17), and tibiamaps
  floor 6 is walkable at x32857..32858 down to y31383. The world overlay
  `tools/content-schema/world-authoring/qh_jimmy_stub.py` opens the closing wall row 31381 and adds
  the stub 32856..32859,31382..31384,6 with a new closing row 31385. Jimmy is placed at the wiki
  cell 32857,31383,6, with no fallback adjustment.

## Batch 2 evidence

Passed locally on the batch-2 tree: `run_checks.py --part checks` and `--part unit-tests`,
`regenerate_content.py --check`, world `pin-check`, `cargo fmt --all --check`, clippy for
`oteryn-gameplay-dispatch --all-features --tests`, and `test_qh_missing_tile_cell`. The dispatch
test binary was killed for memory twice (SIGKILL) in the cloud container, so the
`qh_` PostgreSQL cases run on the PC (packet in the READY message).

## P2 backlog (rvqh)

Recorded from the Codex rvqh review; fixed later in one grouped PR, not in this batch.

- The qh PG cases install substitute NPC placements and call the talk handler directly instead of
  walking to the registered placements.
- The PG wrapper tests the route with `var_os`, so the completion marker cannot report a skipped
  run.
- The Small Sapphire replay check compares only row counts, not the minted rows themselves.
- `tools/content-schema/npc-authoring/chosen_placement_adjustments.py:58` accepts a tie for the
  nearest ground tile.
