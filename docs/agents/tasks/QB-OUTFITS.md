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

- Both Citizen routes grant the same addon entitlements. Mainland hand-ins
  check ownership before taking materials.
- Full-outfit achievement support is tracked in the server lane (FULLOUT).

Tests: QB-3 cells in the Python and compiler placement tests;
`qb_citizen_hand_ins_are_placed_and_story_completes` and
`qb_warrior_hand_ins_are_placed_and_story_completes` assert the global two-hour
waits and run the whole story (`qengine-rewards-qb-citizen-story`,
`qengine-rewards-qb-warrior-story`) on PostgreSQL in CI.

Known global-parity gaps (later batches or the server lane):

- Decaying Defender: Bloodshade (Rotten)'s donor cell 34069,31982,14 and its
  neighbours lie on item 21017-family floor that has no item definition. The
  compiler therefore holds the cell as `NoGround`, so Bloodshade stays unplaced
  until the Rotten Blood floor items are imported (world/items lane).

- Sandra's lottery (100 empty vials for a random winning ticket) needs a
  random-outcome NPC exchange.
- The full-outfit achievement on owning both addons, from any source, has
  been sent to the server lane.

## QB-shaman — Shaman outfit and addons (Hairycles, Chondur)

Gap: Hairycles and Chondur had no placement and no outfit or addon conversation. New generator
module `tools/content-schema/quest-authoring/qb_shaman.py`, registered in `qengine_rewards.py`,
writes the three stages from the Hairycles and Chondur donors; Hairycles 32826,32574,6 and
Chondur 32361,32550,7 are placed at their single agreeing promotion-candidate cells. Global wins
over the donors.

| Step | TibiaWiki | Global mechanic |
|---|---|---|
| Outfit | `Shaman_Outfits_Quest/Spoiler`, `Shaman_Outfits` | After the Banuta questline (Ape City track 18), Hairycles gives the outfit for "shamanic powers" (looktype 154 male, 158 female) |
| Staff start | `Shaman_Outfits_Quest/Spoiler` | Chondur: "task", "addon", "yes", once the 5 pirate voodoo dolls of the Sabrehaven mission are handed in (reputation track 12 or more) and the outfit is owned |
| Voodoo Staff (addon 2) | `Shaman_Outfits_Quest/Spoiler` | Chondur takes 5 dworc voodoo dolls (3002) and 1 mandrake (5014) |
| Mask start | `Shaman_Outfits_Quest/Spoiler` | "addon", "yes", "yes" after the staff |
| Voodoo Mask (addon 1) | `Shaman_Outfits_Quest/Spoiler` | Chondur takes 5 tribal masks (3403) and 5 banana staffs (3348) |

- The Way of the Shaman achievement is granted by the generic full-outfit table, not here.
- Tracks are the existing `u7_8/shaman_outfits/addon_staff_mask`, `mission_staff`, `mission_mask`
  and `u7_6/the_ape_city/shaman_outfit`; the items 3403 and 3348 were added to the native
  custody profile list (`qengine_reward_item_profiles.py`) and the regenerated pins follow.

Not modelled (reported, not faked):

- The donor's extra Counterspell and Djinn in Love requirements (the global wiki asks only for
  the pirate voodoo dolls).
- Chondur's stage-dependent greeting, his "Maybe next time" reply to "no", and Hairycles'
  "You already are shaman and doctor" and "journey for wisdom" replies.

Tests: `postgres_tests/qb_shaman.rs` (placement index and `qengine-rewards-qb-shaman-story` on
PostgreSQL in CI), `tests/npcs/qb_shaman_npcs.rs` and `test_qb_shaman_placements.py`.

## QB-barbarian — Barbarian addons (Bron, Ajax)

Gap: Bron and Ajax had no placement and no addon conversation. New generator module
`tools/content-schema/quest-authoring/qb_barbarian.py`, registered in `qengine_rewards.py`,
writes both routes from the Bron and Ajax donors; Bron 32366,31628,7 and Ajax 32417,31583,7 are
placed at their single agreeing promotion-candidate cells (Bron as a source-backed `block_solid_contact`: his donor cell holds a BlockSolid item and no adjacent cell is clear single-item ground; `test_m38_zarifan.py` pins the exception set with Bron added). Global wins over the donors.

| Step | TibiaWiki | Global mechanic |
|---|---|---|
| Start | `Barbarian_Outfits_Quest/Spoiler` | Bron asks about "violence"; two confirmations start the task |
| Provocation | `Barbarian_Outfits_Quest/Spoiler` | Ajax: "mine", three times "no", "say please", then "yes"; the player must then stay silent for one hour (3601 s) |
| Report and Gelagos | `Barbarian_Outfits_Quest/Spoiler` | Ajax reports; Bron hears "Brother is right. Fist not always good."; "person" and "yes"; Ajax names the fighting spirit |
| Wig (addon 2) | `Barbarian_Outfits_Quest/Spoiler` | Bron takes a fighting spirit, then 50 red and 50 green pieces of cloth, 10 spools of yarn and a warrior's sweat, and gives the wig |
| Axe (addon 1) | `Barbarian_Outfits_Quest/Spoiler` | Ajax takes 100 iron ore, a huge chunk of crude iron, 50 perfect behemoth fangs and 50 lizard leathers, then forges the axe two hours later (7201 s) |
| Outfit | `Barbarian_Outfits` | Premium outfit (looktype 143 male, 147 female); addons from this quest |

- Brutal Politeness is part of the full-outfit achievement follow-up (FULLOUT).
- Items are the catalogue ids 5884, 5910, 5911, 5886, 5885, 5880, 5892, 5893 and 5876.
- Tracks are the existing `u7_8/barbarian_outfits/barbarian_addon` and `barbarian_addon_wait_timer`.

Not modelled (reported, not faked):

- Ajax setting the player on fire when he accepts "please" (no apply-condition in the callback schema).
- The one-hour silence restarting when the player talks to Ajax early (the timer is only read).
- The stage-dependent greetings of Ajax ("You back...") and Bron ("Oh no! Was that really me?"); the
  engine has no greeting-triggered callback, so Ajax's report is bound to "back" or "brother" and
  Bron's follow-up to "yes".
- Ajax's "Axe is not done yet!" reply before the wait ends (the callback is refused instead).

Tests: `postgres_tests/qb_barbarian.rs` (placement index, the 3601 s and 7201 s waits and
`qengine-rewards-qb-barbarian-story` on PostgreSQL in CI), `tests/npcs/qb_barbarian_npcs.rs`
and `test_qb_barbarian_placements.py`.
