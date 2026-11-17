# Winter Update 2026: official forum posts on features (read 2026-11-14)

Source: the official test-server forum (board "Winter Update 2026"), threads 558-560 and 565-567, all pages, CipSoft staff posts only.

## Gem Atelier and Workshop (thread 559, Niadus / Fluffey)
- No domain affinity. Any gem of a quality fits the new slot of that quality, and existing gems lose their domain.
- No Vessel Resonance. Gem Vessel perks replace it: each unlocked Gem Vessel perk unlocks one gem slot and gives +1 damage and healing. Slots unlock in a fixed order of quality.
- A mod's grade is the grade unlocked in the Fragment Workshop. It no longer depends on the other mods on the gem.
- Reshape rerolls one mod at random:
  - slot 1: 125,000 gold + 1 lesser fragment;
  - slot 2: 1,000,000 gold + 2 lesser fragments;
  - slot 3: 5,000,000 gold + 3 greater fragments.
  Measured (`wheel-reshape-draws.md`): temple only, no confirmation, and the same mod can repeat.
- Fragment Workshop: each mod upgrade gives +5 HP, +5 mana and +5 capacity, plus the promotion point for Grade IV. The workshop shows these bonuses.
- New supreme mods with augments, two spells per vocation:
  - Knight: Shield Slam, Intense Wound Cleansing;
  - Paladin: Divine Barrage, Ethereal Barrage;
  - Sorcerer: Death Echo, Lightning;
  - Druid: Forked Glacier, Forked Thorns;
  - Monk: Chained Penance, Thousand Fist Blows.
- Gem resistance cap per element: 30%, with a notification when a setup exceeds it. First announced as 18%; Fluffey corrected it on 10.11. Not on the test server as of 13.11.
- 11.11 (Fluffey): adjustments under discussion.
- 13.11 (Niadus): a larger Wheel and gem changeset, with the full list probably on Monday 16.11. It covers:
  - changes to unwanted supreme mods (Shield Slam, Intense Wound Cleansing, Divine Barrage, Divine Dazzle, Lightning);
  - shifting perks to improve Wheel setups;
  - an option to KEEP the old mod when reshaping;
  - "some more changes".
- 16.11, re-read at about 16:15 CET: the changeset is not posted yet. The FAQ post (#60907) was last edited on 13.11, thread 559 has no staff post after 13.11, and the Fixes and Changes entry for Gem Atelier and Workshop is still empty.

## Elite and Platinum Bounty Tasks (thread 560, Fluffey)
- Elite Task:
  - the target creatures deal +100% damage to you and take -50% damage from you;
  - the reward is +100% XP plus dust (original examples: Beginner 5, Silver Adept 30, Platinum Master 560 dust).
- Platinum Tasks:
  - give twice the rewards of a Gold Task;
  - a creature that is not selected gains a higher chance of appearing as Silver, Gold or Platinum;
  - Platinum needs at least one earlier reroll of that creature;
  - chance caps: Silver 75%, Gold 20%, Platinum 5%.
- 11.11: unlimited rerolls on the test server only (tokens not consumed). Not a game rule.
- 12.11 announced changes, not yet on the test server on 13.11:
  - Elite can be toggled after rolling and before starting, and is locked once the task has started.
  - New Elite Bounty Points, spent in the Hunting Task Shop on:
    - Promotion Points (up to 50);
    - a permanent Bounty Talisman upgrade that also works on Weekly Kill Tasks, except "Any Creature";
    - a doubled Bounty Re-Roll Token limit.
  - Dust and Elite points per task (dust slightly reduced):

    | Task | Dust | Elite points |
    |---|---|---|
    | Beginner | 1 | 1 |
    | Adept | 10 | 10 |
    | Expert | 30 | 30 |
    | Endgame | 60 | 60 |

  - The reroll rarity bonus chance is doubled.
  - The rarity shield icon shows when the rarity came from the reroll bonus.
  - Bounty and Weekly tasks get an icon for a completed Bestiary entry.

## Daily Reward Rework (thread 565, Ahelles / Elefor)
- A 7-day cycle of fixed rewards, claimed with one click:
  - Cosmetic Tokens: worth 1 Tibia Coin each and character-bound; they buy mounts, outfits, house decorations, furniture, beds and hireling dresses; they cannot be combined with coins in one purchase;
  - Prey Wildcards;
  - a temporary temple teleport scroll;
  - XP boosts.
- XP boost: Premium 25% on day 4 and 50% on day 7; Free Account 10% and 20%. Each lasts 30 minutes.
  - It adds to a Store XP boost (50% + 50% = 100%).
  - A new daily boost replaces any remaining daily boost.
- Every daily reward includes 1 Bounty Reroll Token, and the separate daily claim on the Bounty Board is removed.
  - 12.11: the token moved to the bottom of the reward details list. CipSoft is "working on a solution" for the players' concerns.
- Streak:
  - each consecutive day adds +1 percentage point chance of double Weapon Proficiency XP, up to 7%, for Free and Premium;
  - the update does not reset the streak;
  - no rewards for long streaks; the streak counter stays.
- Resting area no longer depends on the streak. Premium gets double HP and mana regeneration plus stamina and soul regeneration; Free Account gets HP and mana regeneration.
- Reported bug: the day 4 claim showed the old "53 Prey Cards" message.
- Owner decision 2026-11-14 19:47Z keeps a 30-day cycle for Oteryn.

## Quality-of-Life (thread 567, Xolurr / Elefor)
- Wave and beam spells get a target option per action button or hotkey:
  - facing direction (default);
  - towards the cursor, without turning;
  - towards the target.
  When the direction is unclear, east and west win over north and south. Magical Archive settings are not transferred. More spells are under review.
- Stash:
  - it reopens with its filters restored after closing a dialog opened from it (Market);
  - the sort order is restored on every open (a client option);
  - closing the Stash manually resets its filters;
  - planned: a Delivery Task items filter and an imbuement items category.
- Emael (Adventurers' Guild) dialogue converts catalysts: 20 lesser proficiency catalysts -> 1 greater; 4 proficiency catalysts -> 1 greater. No XP change.
- Hotkey "Misc.: Create Mark" creates or edits a minimap mark at the character's position. Planned: marking other positions (for example the tile in front) and more marker icons.
- Right-clicking a delivery task item on the Task Board gives a shortcut to the Market.

## Fixes and Changes (thread 558)
Dated index only; the content is in the content threads (`forum-content.md`):
- Banner's Return hunting grounds: 11.11 kill requirement and monster changes; 13.11 onyx beast summoning.
- Banner's Return boss: 12.11 fixed Nemesis dying without players; 13.11 fixes.
- Banner's Return gear: 12.11 augment changed on Corrupted Moonsilver Scorchsteps.
- I Smell a Rat quest: 10.11 Labyrinth; 12.11 Greed Monger; 13.11 Menhir access.
- I Smell a Rat bosses: 12-13.11 balancing, levers, Greedking adjustments.
- I Smell a Rat gear: 13.11 Rat Claws adjustments.
- Daily Reward: 12.11 Bounty Reroll Token display position.
- 16.11: Banner's Return boss access and balancing; Banner's Return gear holy resistance; I Smell a Rat boss adjustments (all in `forum-content.md`).

## Gap check against main (2026-11-14, code read, not verified in play)
Already on main (WU26 package):
- 1 no domains (migration 2770);
- 2 Gem Vessel slots with +1 damage and healing;
- 4 reshape costs and temple check (main also requires Premium);
- 9 Platinum tier and rarity caps;
- 11 daily reroll token, with the Bounty Board claim retired;
- 12 streak proficiency chance up to 7%;
- 13 resting area;
- 16 Create Mark hotkey.

Partial or not verified:
- 3 grade comes from the Workshop only;
- 5 +5 HP/mana/capacity per mod upgrade (not found);
- 7 30% resistance cap (client flag only; no server enforcement or notification found);
- 8 Elite (+100% creature direct damage not found; dust_by_difficulty is [5,15,3,70], and Expert 3 looks wrong);
- 14 no test that refuses a mixed coin and token purchase;
- 18 manually closing the Stash resets filters.

Missing:
- 6 the ten new supreme mods (their spells are unavailable in content; covered by the Wheel rebuild package);
- 10 Elite Bounty Points 1/10/30/60, toggle after rolling, Talisman upgrade for Weekly tasks, reroll cap 10 -> 20, doubled reroll rarity bonus, rarity and bestiary icons;
- 15 Emael catalyst conversion;
- 17 right-click on a delivery item opens the Market;
- 19-21 the content in `forum-content.md`.
