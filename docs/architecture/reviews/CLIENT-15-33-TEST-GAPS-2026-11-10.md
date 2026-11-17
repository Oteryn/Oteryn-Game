# Client gaps from the Tibia 15.33 test client (2026-11-10)

Source: the official test client `15.33.c68982` (`TibiaExternal`) compared with the live client `15.33.124f6c` on the
owner's PC. The comparison covered the English UI strings (`bin/client.en.qm`), the UI image resources
(`bin/graphics_resources.rcc`), `appearances`, `staticdata`, `proficiencies` and `catalog-content`. This is a file
comparison only: nobody played on the test server, and protocol changes inside `client.exe` are not covered.
Oteryn's pinned client assets are 15.30 (`imports/official/client-assets/15.30`).
The Oteryn coverage column comes from a code search on `origin/main` `43f48d1f3`, not a full read.

Scope follows the owner's rule: Crystal / current Tibia.

## Client and UI (points 1-6, this PR)

| # | Area | What the test client changes | Oteryn today |
|---|------|------------------------------|--------------|
| 1 | Spell assignment dialog | The dialog moves from the action bar to its own `assign_spell_dialog`. New "Target Options": Cast towards target, Cast in facing direction, Cast towards cursor position, crosshair, "show only known spells". This is also gap T182 of `CLIENT-GAPS-2026-11-10`. | `apps/client/src/features/local-spell-action-bar/`: no cast-direction option found |
| 2 | Wheel of Destiny / Gem Atelier | Vessel Resonance (VR I-III), Domain Affinity and the corner domain names are removed. New Vessel Node perk ("Each node grants +%1 Damage / Healing and unlocks a vessel in the Gem Atelier"). Gem reshape (re-roll each mod) only in a temple, Premium only, costs gold and fragments. Gem quality lesser/greater. Fragment Workshop: promotion points, +5 HP/mana/capacity per unlocked mod grade, new perk Revelation Mastery (`MasteryIncrease`). Notice after importing an older wheel configuration. | `apps/client/src/features/0148-wheel-allocation/`: Vessel Resonance model, no Vessel Node, reshape or Revelation Mastery found |
| 3 | Reward Wall (daily reward) | New rewards: Cosmetic Tokens, Temple Teleport Scroll, Prey Wildcards, Bounty Reroll Token. Reward Streak Bonus (up to 7% chance of double weapon proficiency XP) replaces the resting-area bonuses. Claim button. Premium teaser. | `apps/client/src/features/0182-daily-reward/`, `crates/gameplay/progression/src/daily_reward.rs`: double proficiency chance found; new reward kinds not found |
| 4 | Bounty Board (Task Board) | Difficulty Beginner / Adept / Expert / Master with tooltips. Elite (+100% damage dealt by monsters, -50% taken; +100% XP and dust, dust Premium only). Silver, Gold and Platinum tasks; the chance grows for a monster you skip. Rerolls only with the Reroll Token from the Reward Wall; "Claim Daily" on the board is removed. | `apps/client/src/features/0072-task-board/`: Elite found; difficulty levels and Platinum not found |
| 5 | XP boost and Store | Separate lines "Daily Reward XP boost" and "Store XP boost", each with "paused". Cosmetic Token currency in the store and the disabled reason "not enough Cosmetic Tokens". | `0151-daily-boost`: single boost line; `0154-store`: cosmetic tokens found |
| 6 | Small UI | Hotkey "Create Mark" (minimap marker at the current position). Option `stashSortOrder`. Changed player state flag icons. | `local-minimap`, `0062-stash`: not found |

## Content (not in this PR)

- Appearances: 1028 new objects (ids 54756-55212), 3 removed (50227, 54653, 54654).
- 21 new outfits (1985-2002, 2025-2027), outfit 1895 removed; 3 new effects (350, 351, 353).
- Monsters:
  - omega bulwark, bastion, battler, moonbreaker, cinderflayer, rampart;
  - giant rat, greedcarrier rat, hoarder rat, greed elemental, greedweaver, greater greed.
- Bosses: Rathilda, Ratasha, Rathena, Rattila, The Greedking, Banner's Nemesis.
- Quests: "I smell a rat", "Banner's Return".
- Achievements: Forged in Conflict, Ratvocat, Feeling Lucky, Just One More, Enemy Mine, Omega Knight, Riding in Style,
  Riding in Luxury, Riding in Prestige, Rat Pack.
- Map: new subareas 0547-0549, changed 0243, 0530, 0531 and 0538, changed minimap tiles.
- Proficiencies: about 256 of 443 definitions changed values; new definition 499.
- 13 new sounds and 74 new sprite sheets.
- Moving the pinned client assets from 15.30 to 15.33.
