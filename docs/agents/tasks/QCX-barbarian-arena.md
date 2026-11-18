# QCX61 Barbarian Arena Quest

Base `871c1dec7d9963420d455210e4cddc0c9df80d26`.

## Evidence

- TibiaWiki Structured quest facts (curid 17003, oldid 842181). https://tibia.fandom.com/?curid=17003&oldid=842181
- TibiaWiki Structured requirements, room sequence and per-tier rewards (curid 34102, oldid 668035). https://tibia.fandom.com/?curid=34102&oldid=668035
- Crystal `201189f871d23597b82caf88fd611454a2ddefb8`, OTS hypothesis only. The retained inventory in `docs/reference/SRVENC-CONTRACT-NOTES.md` and `tools/content-schema/quest-authoring/batches/quests4/svargrond_arena.json` points to the source files. `data-global/npc/halvar.lua:74-115` controls payment, `data-global/scripts/quests/svargrond_arena/movements_arena_enter.lua:9-39` controls entry, `movements_arena_pit.lua:14-70` controls surrender and advancement, `movements_arena_trophy.lua:9-29` controls trophy creation, and `data-global/startup/tables/chest.lua:2307-2419` maps item IDs and chest UIDs. The wiki resolves the donor's separate present, backpack and weapon flags in favor of a single choice.

## Requirements, stages and placements

| Area | Global and repository comparison |
|---|---|
| Admission | Global requires Premium, The Barbarian Test, completion of all lower tiers, and 1,000 / 5,000 / 10,000 gp, with no level minimum. The quest definition and completion recipe carry Premium and the prerequisite, but native entry currently checks neither. Its sequential rank and fee agree with Global, with no level gate. Halvar's NPC definition and source transcript exist without a live quest payment offer. The native command charges the fee as it enters pit 1. |
| Sealed door and forcefields | The source map regions at x125/y121 retain `41152` at (32233, 31098–31100, 7) and tier reward doors `41171–41173` at (32227, 31066/31059/31052, 7). The native arena does not publish the corresponding paid and completed storage tracks. The three forcefield entries lack `25100` in the installed region. The command takes a broad floor-6 entrance area and jumps directly to the first pit. |
| Pits | All three ten-opponent sequences agree, as do the 600-second limit, the shared room geometry, one-player custody, immediate opponent spawn, and the west pillar's victory portal. The runtime already owns credit, spawn, timeout, lease and reset. No arena callbacks were present in either generic callback registry. |
| Surrender | Global has a south forcefield in each room. The base map contains south forcefields without `25300`. Surrender uses the command or native departure cleanup, without an exact south-forcefield action. |
| Spectators and death | Global blocks item movement across the arena barrier and removes a previous corpse on re-entry. The arena code checks spectator occupancy and has death and pit reset handling. Its reset examines only the top Thing on each source tile. The checked-in arena code has no arena-specific item crossing guard. |
| Claim placement | Weapon records group three placements per tier and are marked ready. The six present/backpack variant records retain the contents and their own UIDs but are marked waiting for generic lowering. Some generic chest use routes are admitted by the separate catalogue chest owner. There is no shared entitlement between that owner and the arena command. Trophy UIDs `3264–3266` are absent from the checked map's arena area, and trophies are claimed by command. |
| Journal and achievements | Global lists the quest under The Ultimate Challenges and names Warlord of Svargrond. The repository has both an arena definition and a separate Ultimate Challenges family recipe, with terminal progression based on all three trophy claims. Native completion grants Warlord of Svargrond and also donor Greenhorn and Scrapper achievements. |
| XP, outfits, access | No quest completion XP or outfit/addon appears in the facts or arena completion grant. Creature XP, if any, is ordinary combat settlement. Tier access and the reward room are the access effects described by Global. |

## Reward IDs and counts

Each weapon, container, goblet and unmarked child below has count 1. The runtime catalogue and imported claim payloads agree on the choice contents. The donor's power bolt row has count 200. The repository represents that total as two legal stacks.

| Tier | Weapons | Present 2856 | Backpack 7342 | Goblet |
|---|---|---|---|---|
| Greenhorn | Orcish Maul 7392, Headchopper 7380, Blacksteel Sword 7406 | Piggy Bank 2995, blue Surprise Bag 6570, Bar of Chocolate 6574, Candy 6569 ×10, Blue-Barian cone 7377 | Sniper Arrow 7364 ×100, Onyx Arrow 7365 ×100 | Bronze 5807 |
| Scrapper | Cranial Basher 7415, Heroic Axe 7389, Mystic Blade 7384 | Baby Seal Doll 7183, Bar of Chocolate 6574, Candy 6569 ×10, Chilly Cherry cone 7375 | Great Fireball emblem 11615, Heavy Magic Missile emblem 11605, Power Bolt 3450 ×200, Onyx Arrow 7365 ×100 | Silver 5806 |
| Warlord | Blessed Sceptre 7429, Royal Axe 7434, Justice Seeker 7390 | Panda Teddy 5080, Bar of Chocolate 6574, Candy 6569 ×10, Crispy Chocolate Chips cone 7372 | Infernal Bolt 6528 ×100, Mastermind Potion 7440, Bullseye Potion 7443, Sudden Death emblem 11609, Ultimate Healing emblem 11603 | Golden 5805 |

## Reward correction

The arena transaction now treats a present, backpack or weapon receipt as consuming the tier's one choice. It recognizes previously committed receipts in any of those categories. The goblet has its own entitlement. The same mint transaction records the wiki's `Awarded to <name>.` inscription using the fenced character's name. The wiki gives the same inscription for all three goblets. Crystal adds tier-specific description prose and omits the final period for Greenhorn in `data-global/lib/quests/svargrond_arena.lua:175-190`. [Spoiler revision](https://tibia.fandom.com/?curid=34102&oldid=668035).

## Remaining parity

BLOCKER Halvar/sealed door/forcefields paid admission and native entry binding `data-global/npc/halvar.lua:99`, `data-global/scripts/quests/svargrond_arena/movements_arena_enter.lua:9`

BLOCKER south exit portal exact native concession binding `data-global/scripts/quests/svargrond_arena/movements_arena_pit.lua:14`

BLOCKER shared native chest/command entitlement cross-owner reward arbitration `data-global/startup/tables/chest.lua:2307`

BLOCKER trophy tiles and item LOOK native personalized placement and shared look projection `data-global/scripts/quests/svargrond_arena/movements_arena_trophy.lua:17`

BLOCKER spectator barrier and corpse cleanup shared item-transfer and corpse ownership `data-global/lib/quests/svargrond_arena.lua:207`

OPEN-FACTS Global item numeric IDs, precise placement attributes and coordinates, fee source or bank policy, lower-tier achievements, donor surrender outfit, Warlord achievement points, and journal state timing after each tier. The reward IDs above map named Global items to pinned donor records.

PARTIAL Gate Premium and The Barbarian Test at admission, wire Halvar's payment and physical access/reward selectors, expose the stored inscription on LOOK, and resolve arena-specific item movement, corpse cleanup and tier journal updates. The reward command is the scope of the new dispatch case. Historical goblets minted before the correction lack the new inscription.

## Verification

- PG via shard (pgsplit build files applied on `92fd7373bab2595fd954b08501eb8af637cb1746`). `one_reward_choice_and_inscribed_goblet_per_tier` passed in 27.26 s. The production wire and durable claim transactions run against PostgreSQL. The test seeds the completed arena state and exercises the three tier choices, three second-choice refusals, and three goblet mints.
- Before the reward fix, the same case failed in 42.13 s. The three second choices returned `Applied` and the inscription query returned no rows.
- The same case returned `Ok(())` without `OTERYN_TEST_POSTGRES_ADMIN_URL` in 0.00 s. This is the route-less check.
- Quest authoring `run_checks.py --part checks` passed all 44 commands.
- `cargo fmt --all -- --check` passed.
- World pin check passed for one world, `oteryn`. The CLI requires an output directory, so the check used `oteryn-world-bundle-compiler pin-check . <temporary directory>`.
- Dispatch library Clippy passed with `--all-features --lib -- -D warnings`.
