# QB-OUTFITS — outfit and addon quest build lane qb

```yaml
task_id: QB-OUTFITS
title: "QB: outfit and addon quests fully playable at Tibia global parity"
mode: IMPLEMENT
status: in_progress
repository: Oteryn/Oteryn-Game
issue: 1622
base_branch: main
branch: claude/happy-davinci-ijz3wy-qb
pr: null
merge_authority: CP_ONLY (lane opens, labels and merges no PR)
owned_paths:
  - docs/agents/tasks/QB-OUTFITS.md
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/qb_*.rs
  - tools/world-bundle-compiler/tests/npcs/qb_*.rs
  - tools/content-schema/npc-authoring/test_qb_*.py
  - this lane's quests' entries in shared callback, placement and index files
```

Done bar (owner): the quest passes E2E **and** its rewards and mechanics match
Tibia global, with the TibiaWiki page cited. Where a Canary or Crystal donor
differs from global, global wins.

## QB-1 — outfit hand-in NPC placements

Gap: the reward callbacks for these quests existed and their stories passed in
the PostgreSQL harness. But the NPCs that serve them were missing from the
native World placement index, so no player could reach the conversations.

| Quest | TibiaWiki | NPCs placed (x, y, z) |
|---|---|---|
| Assassin Outfits | `Assassin_Outfits_Quest/Spoiler` | Vescu 32565,32651,7 (base); Erayo 32518,32911,8; Atrad 32077,32533,9 |
| Beggar Outfits | `Beggar_Outfits_Quest/Spoiler` | Hugo 32950,32102,5; Simon the Beggar 32186,32471,7 |
| Hunter Outfits | `Hunter_Outfits_Quest/Spoiler` | Legola 32298,31785,7 (Elane was already placed) |
| Mage / Summoner Outfits | `Mage_Outfits_Quest/Spoiler`, `Summoner_Outfits_Quest/Spoiler` | Angelina 32633,32402,10; Lynda 32331,32199,7; Myra 32580,32751,6; Sandra 33257,31839,7; Zoltan 33266,31847,4 |
| Wizard Outfits | `Wizard_Outfits_Quest/Spoiler` | Lugri 32389,32118,8; The Queen of the Banshees 32260,31866,14 |

- Every NPC is placed at its single agreeing Canary/Crystal promotion-candidate
  cell (`npc_placement_scope.json`, then `convert_placements.py`, then
  `pin-refresh`).
- Vescu's donor cell holds a BlockSolid item in our map. No adjacent cell is
  clear single-item native ground, so `placement_adjustments` cannot apply.
  Vescu is therefore a source-backed `block_solid_contact`, the same mechanism
  as Zarifan. The neighbouring border items do not block movement.
- `test_m38_zarifan.py` now pins the exact exception set
  {Zarifan, Vescu} instead of "Zarifan only".

Tests (red before the placement change, green after):

- `tools/world-bundle-compiler/tests/npcs/qb_outfit_npcs.rs`: each NPC is
  admitted by `realize_npcs` at its exact cell.
- `tools/content-schema/npc-authoring/test_qb_outfit_placements.py`: the
  converter writes each NPC exactly once at its cell.
- `postgres_tests/qb_outfit_placements.rs`: every reward-callback NPC of each
  quest is in the placement index, then the whole source story runs on
  PostgreSQL.
  - PostgreSQL legs: the harness requires 18.6. This container has 16, so the
    routed legs are left to CI.
  - The placement assertions run route-less.

## QB-2 — further outfit and reward-use hand-in placements

The callbacks and stories existed; their NPCs were not placed. Each NPC is placed
at its single agreeing promotion-candidate cell, as in QB-1. One Eyed Joe
(Cursed Crystal) is not in this lane.

| Quest | TibiaWiki | NPCs placed (x, y, z) |
|---|---|---|
| Brotherhood / Nightmare Outfits | `Brotherhood_Outfits`, `Nightmare_Outfits` | The Bone Master 32793,32225,14; The Dream Master 32844,32228,14 |
| Cave Explorer Outfits | `Cave_Explorer_Outfits` | Gnommander 32248,32604,8 (source-backed solid contact: no adjacent clear single-item ground) |
| Crystal Warlord / Soil Guardian Outfits | `Crystal_Warlord_Outfits`, `Soil_Guardian_Outfits` | Gnomally 32795,31746,10 |
| Revenant Outfits | `Revenant_Outfits` | Flickering Soul 33621,31419,10 |
| Dream Warden / Dream Warrior Outfits | `Dream_Warden_Outfits`, `Dream_Warrior_Outfits` | Eruaran 33618,32372,5; Vanys 33685,32180,8 |
| Druid Outfits | `Druid_Outfits_Quest/Spoiler` | A Majestic Warwolf 33355,31991,8; Ceiron 32665,31717,7; Ustan 32580,32757,6 |
| Knight Outfits | `Knight_Outfits_Quest/Spoiler` | Gregor 32406,32201,6 |
| Hand of the Inquisition (mission 1 guards) | `The_Inquisition_Quest/Spoiler` | Grof 32372,32182,7; Kulag 32287,32264,7; Miles 32429,32173,5; Tim 32425,32223,6; Walter 32338,32277,7 |
| To Outfox a Fox | `To_Outfox_a_Fox_Quest/Spoiler` | Budrik 32525,31906,8 (adjacent to the BlockSolid donor cell 32525,31905,8) |

## QB-3 — mainland Citizen and Warrior addons

Gap: only the Rookgaard Citizen route (Amber, Norma) existed, and the Warrior
addons had no conversation at all. New generator module
`tools/content-schema/quest-authoring/qb_citizen_warrior.py`, registered in
`qengine_rewards.py`, writes the conversations from the Lubo, Hanna, Trisha and
Cornelia donors. Global wins over the donors:

| Addon | TibiaWiki | Global mechanic |
|---|---|---|
| Citizen backpack | `Citizen_Outfits_Quest/Spoiler` | Lubo 32489,32119,7 takes 100 minotaur leathers, then the backpack is ready two hours later |
| Citizen feather hat | `Citizen_Outfits_Quest/Spoiler` | Hanna 32408,32220,7 takes a legion helmet, 100 chicken feathers and 50 honeycombs |
| Warrior shoulder spikes | `Warrior_Outfits_Quest/Spoiler` | Trisha 32346,31749,7 takes 100 hardened bones, 100 turtle shells, fighting spirit and a dragon claw; Cornelia 32335,31794,7 adorns the armour two hours later, for everyone (the Crystal donor's one hour for women is not global) |
| Warrior sword | `Warrior_Outfits_Quest/Spoiler` | Morgan takes a piece of royal steel and 100 iron ore |

- Both Citizen routes share the catalogue tracks `u7_8/citizen_outfits/*`, so
  each addon is earned once, by either route.
- The full-outfit achievement is granted by the server lane's generic table
  (FULLOUT), not by these callbacks.

Tests: QB-3 cells in the Python and compiler placement tests;
`qb_citizen_hand_ins_are_placed_and_story_completes` and
`qb_warrior_hand_ins_are_placed_and_story_completes` assert the global two-hour
waits and run the whole story (`qengine-rewards-qb-citizen-story`,
`qengine-rewards-qb-warrior-story`) on PostgreSQL in CI.

## QB-VAMPIRE — Vampire Hunter Quest token ranks

| Stage | TibiaWiki | Global behaviour |
|---|---|---|
| Gate | `Vampire_Hunter_Quest` | Storkus 32589,31962,7 accepts vampire lord tokens only after The Inquisition's vampire mission (donor questline 11) |
| Ranks 1-6 | `Vampire_Hunter_Quest`, `Vampire_Lord_Token` | 1, 4, 5, 10, 30 and 50 tokens (100 in all), each rank sharing experience |
| Last rank | `Vampire_Doll`, `Blood_Skull` | a vampire doll, and the Vengoth lower-castle door (action 41731) opens to the blood skull chest |

- The experience per rank (1,000, 5,000, 10,000, 20,000, 50,000 and 100,000)
  is the Crystal donor's (`storkus.lua:143-196`). The local TibiaWiki snapshot
  confirms the 100-token total, but not the experience values; they still need
  checking against the live Wiki page.
- Razing! (`oteryn:achievement/razing`) is not granted yet. The local snapshot
  links it to Storkus and the tokens but leaves out its trigger, so the trigger
  is an open fact for the live Wiki.

Tests: Storkus's cell in the Python and compiler placement tests;
`qb_vampire_hunter_storkus_is_placed_and_ranks_complete` runs the gate refusal,
each rank's one-short refusal and exchange, the doll and the door
(`qengine-rewards-qb-vampire-hunter`) on PostgreSQL in CI.

Known global-parity gaps (later batches or the server lane):

- Decaying Defender: Bloodshade (Rotten)'s donor cell 34069,31982,14 and its
  neighbours lie on item 21017-family floor that has no item definition. The
  compiler therefore holds the cell as `NoGround`, so Bloodshade stays unplaced
  until the Rotten Blood floor items are imported (world/items lane).

- Sandra's lottery (100 empty vials for a random winning ticket) needs a
  random-outcome NPC exchange.
- The full-outfit achievement on owning both addons, from any source, has
  been sent to the server lane.
