# ARENA-1 Tibiadrome, the PvE arena and PvP arenas

- Decision: `ARENA1-TIBIADROME-AND-ARENAS-V1`
- Status: **CANDIDATE** (architect-authored). Acceptance needs exact-head validation, independent review (security, concurrency, persistence, economy, combat and protocol) and protected integration. Owner acceptance is the owner's to give; none is recorded here.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the mechanics completion plan row 106 (`OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md`, batch ARENA-1: "Tibiadrome and arenas"); the PvP arena moved here from SOCIAL-LATE-1 (`OTERYN_GAME_ACCEPT_BATCH_2026-10-15_ACCEPTANCE_DECISION.md` point 8: "SOCIAL-LATE-1: as in Tibia. No marriage. The PvP arena moves to ARENA-1."); the task record `docs/agents/tasks/ARENA-1.md`; BOSS-RAID-0 §18 and PARTY-PVP-0 (arenas and PvP zones deferred to their own decision); the gap register §20 (arenas).
- Builds on: BOSS-RAID-0 (accepted 2026-10-15: §6 boss rooms as activity instances, lever admission, cooldowns, exit and recovery; §12 lock order and World jobs); the instance baseline (`INSTANCE_SCOPE_AND_RUNTIME_OWNER_BASELINE.md`, owner-accepted); HOUSE-RUNTIME-0 §4 (SCOPE-HANDOFF-1 transition and exit fallback); ENCOUNTER-RT-0 (accepted 2026-10-14); PARTY-PVP-0 (accepted: §7 legality, §9 consequences, §10 PvP death); DEATH-0 (accepted: the death receipt); ADR-0001 §10 and §12; GAME-CHANNEL-01 §13 to §15; DUR-03 (MINT, ledgers); PREMIUM activation (accepted 2026-10-13); the content and ruleset tree; owner rule 5905825574 (Global parity).
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`
- Runtime, migration, wire and production authority: NONE until accepted and allocated. Each child needs its own #162 allocation.


## 1. Scope

Three arena kinds: the **Tibiadrome** (a group PvE wave battle with rotations, leaderboards and rewards), the **PvE arena** (the Svargrond arena, a character's sequence of ten duels per tier) and the **PvP arena** (a lever-started match between characters). This brief decides their common instance model, entry, death outcome, rewards, durable state, the Tibiadrome's wave scaling, monsters, modifiers, potions and rotation anchor, and the two other arenas' tiers, opponents, fees and match shapes. Nothing is left open: every value has a cited source (§2, §5) or is a stated engineering default with its rationale (§5).

## 2. Reference grounding

Sources, by short name:
- **Manual**: `docs/reference/tibia-manual/combat.md` §5.3.14 (the CipSoft game guide, tibia.com/gameguides, manual section Combat).
- **TibiaWiki-Drome**: tibia.fandom.com/wiki/Tibiadrome (rotation, modifiers, tie rule, HP formula; fetched through search extracts 2026-10-15 because the site blocks direct fetches).
- **TibiaQA-HP**: tibiaqa.com/26489 (accepted answer 2022-05-17: base HP and the 1.039 formula, "verified by TibiaWiki Admins").
- **TibiaQA-abilities**: tibiaqa.com/26092 (ability list per drome monster, tested at drome level 1).
- **TibiaGoals**: tibiagoals.com/2022/01/tibiadrome-era-of-new-arena.html (modifier numbers, potion list, Murmillion paralyze).
- **TibiaXplorer**: tibiaxplorer.com/tibiadrome (rotation #138 now, next boundary 28 Oct 2026 Wednesday, 14 days, potion durations).
- **Canary**: `opentibiabr/canary` `main`, `data-otservbr-global/` (Svargrond arena lib, creaturescripts and movements, NPC Halvar, `arena_pvp/`). **Crystal**: `zimbadev/crystalserver` `main` (`data-crystal/scripts/quests/arena/`, `data-global/scripts/actions/arena_pvp/`, `data/scripts/actions/items/drome_cube.lua`). Neither implements the Tibiadrome monsters, waves or rotation.
- **Repository items**: `imports/canary/items-xml/items.xml` (potion ids 36723 to 36742, drome cube 36827, goblets 5805 to 5807); `content/interactions/reward_claims/` (three Barbarian Arena reward claims, per character, once).
- **Duelling Arena**: CipSoft news 2015 (10.90 update) and TibiaWiki "Duelling Arena" (via search extracts): the real PvP arena, disabled 2022-03-29 and announced as permanently removed 2023-02-14 for low interest and abuse scenarios.

Facts:
- **Tibiadrome** (Manual):
  - Premium-only PvE practice zone; locations Ankrahmun, Kazordoon, Thais, Edron, Rathleton, entered by a demonhead-marked teleporter. Free accounts may spectate from the roof only.
  - Groups of 1 to 5 players, each level 50 or more. Run start: all participants on designated tiles, one pulls a lever, teleport, an 8-second preparation phase.
  - A battle is unlimited sequential waves of increasing difficulty with combat modifiers; each wave has a 120-second limit. Monsters per wave by group size: 1 to 4, 2 to 8, 3 to 11, 4 to 14, 5 to 17, from a pool of five drome-only monsters.
  - The battle ends when a wave is not cleared in 120 s, the whole team dies, or the team leaves by the teleporter; if one member leaves, the others continue. Afterwards players return to the entry hall with HP and mana reset to their pre-entry values, and every participant waits 60 minutes before the next fight.
  - No death penalty (XP, skills, items, blessings). Mana and health potions, arrows, bolts and runes are not consumed; rings, amulets, food and imbuement charge time are. No skill or XP gain, no stamina drain. Soul-point spells are disabled. One group per drome instance; no PvP. Familiars and Chivalrous Challenge are usable.
  - Drome level and starting level: a run starts at the lowest participant's starting level; each beaten wave raises both by 1; after a battle the starting level drops by 5. A rotation lasts 2 weeks; at its end the drome level resets to 0 and the starting level drops by 5 (floor 0); an NPC lowers the starting level by 5 on request.
  - Rotation rewards: drome points = drome level x 10 (eligible after at least one cleared wave), spent on cosmetics at an entry-hall NPC; potions by rank (1st to 5th: 3, 6th to 10th: 2, 11th to 20th: 1), plus potions equal to 20% of the remaining participants raffled among rank 21 and below, weighted by drome level; a temporary title for the top 5 until the next rotation ends. Leaderboards for the current rotation and lifetime Highscores of cumulative drome levels.
  - Character trading keeps the levels but removes the character from the leaderboard during the auction.
- **Tibiadrome, beyond the Manual** (TibiaWiki-Drome, TibiaQA-HP, TibiaGoals, TibiaXplorer):
  - Monster hit points are `base HP x 1.039^wave` with base HP Domestikion 750, Hoodinion 800, Mearidion 850, Scissorion 950, Murmillion 1100.
  - Two modifiers are drawn at the start of each rotation, the same on every World, and are active in every wave of the rotation (the Manual says "several ... each round"; the wiki and the in-game Modifier Board show two per rotation).
  - Rotations start and end at the server save of every other Wednesday, unaffected by server resets; TibiaXplorer numbers the current rotation #138, ending Wednesday 2026-10-28 (so #138 began 2026-10-14 and #1 on 2021-07-14).
  - Ranking tie: the lower experience the character had when it reached its highest drome level ranks higher (TibiaWiki-Drome).
  - 20 reward potions (6 utility, 7 resilience, 7 amplification), duration 1 hour, an individual 24-hour cooldown per potion kind, so different kinds stack; the raffle awards at most 50 potions.
  - Drome monsters grant no XP in the drome (Manual: no XP gain). The Manual lists no loot; Canary and Crystal define none.
- **PvE arena** (Canary, Halvar NPC, TibiaWiki "Barbarian Arena Quest" via search extracts, achievements snapshot `imports/tibiawiki/achievements/2026-09-29/tibiawiki-achievements-facts.json`): Svargrond, three tiers Greenhorn (1,000 gp), Scrapper (5,000 gp), Warlord (10,000 gp), ten opponents each in ten pits, 10 minutes per pit, the NPC's death rule "your corpse and backpack are gone".
- **PvP arena** (Canary and Crystal `arena_pvp/arena_2x2.lua` and `arena_10x10.lua`, Duelling Arena): lever scripts that need exactly 2 (one per side) or 10 players standing on fixed tiles and teleport them into one arena room; no death script. Real Tibia's Duelling Arena: all participants Premium, Team Deathmatch and Last Man Standing, no death penalty and no guild-war frag on an arena death, a defeated player goes to the temple of the home city, time limits for waiting and fight.
- **Tree.** `content/encounters/tibiadrome/`, `content/encounters/arenas/` ("PvP rules remain ruleset"), `rulesets/encounters/tibiadrome/` (selection, scoring, reward) and `rulesets/pvp/arena/`, all `READY_UNPOPULATED`. The task record's `content/arenas/` owned path is corrected to those directories (§5).

## 3. Rules

**A1. Every arena battle is an activity instance.** A Tibiadrome battle, a PvE arena run and a PvP arena match are each one activity instance (`WorldId + InstanceId`, scope kind 2, one `InstanceRuntime`), exactly as a BOSS-RAID-0 §6.1 boss room: a fresh InstanceId per admission, the origin channel recorded per participant, the SCOPE-HANDOFF-1 transition. The lobby, the entry hall, the teleporter and the roof are ordinary channel map. One group per instance (the manual's one group per drome instance). No channel change happens through an arena: the exit targets the origin channel first, with the HOUSE-RUNTIME-0 §4.2 fallback (ADR-0001 §12).

**A2. Entry is a lever admission, gated server-side.** BOSS-RAID-0 §6.2 applies unchanged: the channel collects the characters on the arena's entry tiles; one transaction locks each participant's fence and `character_root` in CharacterId order, checks every gate and writes the instance allocation and each cooldown row; any failure refuses the whole group (`ON_COOLDOWN`, `NOT_ELIGIBLE`, `BUSY`, `FULL`) and writes nothing; the `BOSSRAID0-RL-08` arrival window and compensation apply. Arena gates checked in that transaction:
- Tibiadrome: every participant Premium (the accepted Premium activation switch), level at least `ARENA1-RL-02`, group size 1 to `ARENA1-RL-01`, no drome cooldown;
- PvE arena: one participant; Premium (Svargrond is a Premium town, the quest is Premium-only); no level floor (Canary has none; TibiaWiki "Barbarian Arena Quest" lists none); the tier's quest predicate (QUEST-PRED-1): tier 1 open, tier n open only after tier n-1 is completed, a completed tier is never admitted again; the fee (A8) debited in the same transaction;
- PvP arena: every participant Premium (Duelling Arena), the exact participant count of the lever's mode (A9), PARTY-PVP-0 §7.2 rule 2 and rule 3 as the level and zone gate.
The client never chooses the instance, the team or the start wave.

**A3. Death policy per arena kind.** An arena instance carries `death_policy`:
- `ARENA_DEFEAT` (Tibiadrome, PvP arena). When an actor's HP would reach 0, the actor is not killed. It leaves the battle alive: no DEATH-0 receipt, no XP or skill loss, no item loss, no blessing change, no corpse or loot, no `CharacterRevision` advance, no PARTY-PVP-0 §9 consequence rows, no guild-war frag. It is moved to the instance's `defeat_anchor` on its origin channel with HP and mana restored to their pre-entry values (A5). The battle records the defeat. `defeat_anchor` is `EXIT_ANCHOR` (the entry hall) for the Tibiadrome (Manual) and `HOME_TEMPLE` for the PvP arena (Duelling Arena: defeated players go to the temple of the home city), each with the HOUSE-RUNTIME-0 §4.2 fallback. Sources: Manual (Tibiadrome), Duelling Arena (PvP).
- `STANDARD` (boss rooms, the PvE arena) is DEATH-0 unchanged: the receipt, the penalty and the corpse apply, the character respawns at the home temple (BOSS-RAID-0 §6.4), and the corpse's items retire with the instance by `InstanceRetire` (BOSS-RAID-0 §6.5 amendment). This is the real Svargrond rule: Halvar says "if you die in a pit ... your corpse and backpack are gone, so you enter the arena at your own risk"; Canary and Crystal's `SvargrondArenaPlayerDeath` script only resets the run (`PitDoor` to 0), it grants no protection.
This is a DEATH-0 §3.1 and PARTY-PVP-0 §10 amendment, pending on acceptance of this brief: the death classification first asks the scope's `death_policy`.

**A4. Tibiadrome battle.**
- After every participant has arrived (or the arrival window closed with at least one), an 8-second preparation (`ARENA1-RL-04`), then waves from the group's start wave: the lowest participant's starting level.
- **Wave composition.** Wave `w` spawns the monster count for the group size (`ARENA1-RL-05`), each monster drawn uniformly, with replacement, from the five-monster pool, under the new RNG purpose `drome_wave` seeded by (InstanceId, `w`).
- **Scaling (wave number `w` is the wave's drome level).** Hit points `round(base_hp x 1.039^w)` (TibiaWiki-Drome, TibiaQA-HP; `ARENA1-RL-20`). Damage: the same factor, `round(base_hit x 1.039^w)`. Real Tibia's damage curve is unobservable (the wiki's maximum-damage fields are unknown); the same-factor curve is the decided v1 default, chosen because it keeps a fixed damage to hit-point ratio per monster, so the wave is as hard to survive as it is to clear. A monster carries its scaled values from spawn; no other stat scales.
- **Monster pool** (all drome-only, `READY_UNPOPULATED` content in ARENA-CONTENT-1; XP 0 and no loot; protection 100% to every element, no weakness and no immunity, TibiaQA-abilities "no elemental weakness"):

| Monster | Base HP | Role | Elements | Wave-0 max hit (engineering default) | Abilities (TibiaQA-abilities) |
|---|---|---|---|---|---|
| Domestikion | 750 | support melee | energy, holy, life drain, mana drain | 150 | melee; energy bomb on target 3x3; holy beam length 3; life drain hit; mana drain hit; self-centred bomb radius 2 (no damage, removes Magic Wall and Wild Growth); damage aura for all monsters, below |
| Hoodinion | 800 | ranged, area and burst | ice, fire | 300 | melee; ice missile every turn; fire chain (several targets); ice area on target, diamond radius 5; ice wave, two shapes; self-centred bomb radius 4 (no damage, removes fields) |
| Mearidion | 850 | ranged, steady single target | death | 250 | melee; death missile every turn; death explosion on target; death beam length 7; self-centred bomb radius 5 (no damage, removes fields) |
| Scissorion | 950 | melee, high single target | earth, physical | 220 | melee; poison hit that can crit; self-centred poison bomb radius 2 (no damage, removes fields); turns invisible before attacking (TibiaGoals) |
| Murmillion | 1100 | melee tank | physical | 80 | melee; physical beam length 3; physical bomb radius 3 on itself; blade bomb radius 2 (no damage, removes fields); paralyze every 10 s (TibiaGoals) |

  Wave-0 max hit is the highest single damage any ability of that monster rolls at wave 0, with an ability's own factor of 1.0 for hits, missiles, beams, explosions and waves; the order follows TibiaQA's observed damage ranking (Hoodinion, Mearidion, Scissorion, Domestikion, Murmillion). Domestikion buff: each living Domestikion adds +10% to the damage of every drome monster, additive, at most +50% (five Domestikions). Paralyze lasts 5 s with the ruleset's standard paralyze condition; Scissorion's invisibility lasts 3 s or until its next attack. These constants are decided v1 values, the registered ruleset rows `ARENA1-RL-20`; changing them is a registry change.
- **Modifiers** (the nine; exact effects from Manual, TibiaGoals and TibiaWiki-Drome). At the start of each rotation two distinct modifiers are drawn from the nine under the new RNG purpose `drome_modifier`, seeded by the rotation ordinal alone (the same on every World, as in Tibia), written to the rotation row (A7) and active in every wave of every battle of that rotation. Effects are ENCOUNTER-RT-0 actions; every percentage below is of the affected player's current HP unless stated, so a hazard never kills by itself. Hazard tiles and seed positions are drawn under `drome_wave` seeded by (InstanceId, `w`, pulse index) from the room's walkable tiles.

| Modifier | Effect | Period |
|---|---|---|
| Somersault | a melee monster's attack has a 15% chance to teleport it next to the participant furthest from it | per melee attack |
| Going Down With Me | when a monster dies, an area (Chebyshev radius 1, centred on the last hitter) hits the last hitter and the participants in it for 40% of their current HP | per monster death |
| Exploding Corpses | when a monster dies, an area (radius 1, centred on the corpse) hits the participants in it for 25% of their current HP | per monster death |
| That Escalated Quickly | a monster below 25% of its maximum HP deals damage as if it were 5 waves higher: `base_hit x 1.039^(w+5)` | while below 25% |
| The Floor is Lava | `ARENA1-RL-09` (100) random tiles are marked; after 3 s a participant on a marked tile loses 60% of its current HP | every 15 s |
| Beam me Up! | 100 random tiles are marked; after 3 s a participant on one is teleported to a random free walkable tile of the room | every 15 s |
| Tanked Up | 100 random tiles are marked; after 3 s a participant on one becomes super drunk (the Drunk condition) for 10 s and takes 10% more damage | every 15 s |
| Sown Sorrow | a green seed appears; a participant must step on it within 6 s, else it explodes and fears every participant for 2 s, during which they take 15% more damage | every 20, 17, 14, 11 or 8 s for 1, 2, 3, 4 or 5 participants (`ARENA1-RL-19`) |
| Bad Roots | a brown seed, same timing; unstepped after 6 s it roots every participant for 3 s, during which they take 25% more damage | same as Sown Sorrow |

  Where a tile marks overlap, effects apply once per participant per pulse. The "participants" count is the group size at admission (a seed period does not change when a member is defeated).
- A wave is cleared when its monsters are dead within `ARENA1-RL-03`. The battle ends when a wave is not cleared in time, when no participant is left in the instance, or when every remaining participant leaves by the exit teleporter. A participant who leaves or is defeated leaves alone; the others continue.
- Battle duration is bounded: wave numbers are at most `ARENA1-RL-08` (300; `1100 x 1.039^300` is about 1.1e8 and fits `i32`); reaching it ends the battle as if the team left.

**A5. Tibiadrome rules inside the instance** are instance ruleset values, applied by the owning mechanics (no second combat path):
- Consumption suppression: health and mana potions, arrows, bolts and runes take effect but are not retired or decremented (no DUR-03 retirement, no charge write). Rings, amulets, food and imbuement charge time are consumed normally. ITEM-USE and RUNE-USE amendment, pending on acceptance of this brief.
- No XP receipt, no skill try, no stamina drain; soul-point spells refuse with a typed reason (`NOT_ALLOWED_HERE`). Familiars and Chivalrous Challenge are allowed.
- No PvP: PARTY-PVP-0 §7.2 gains rule 0, "an arena instance without PvP refuses", pending on acceptance of this brief.
- On leaving (any end), HP and mana are set to the values recorded at the admission commit; nothing else of the vitals is restored.
- Drome monsters grant no XP and drop nothing; the participants' reward potions (A7) are not affected by the instance's consumption rules.

**A6. Tibiadrome durable state.**
- `game_character_drome`: (CharacterId, WorldId) -> `starting_level`, `drome_level` (the best wave level of the rotation), `lifetime_levels` (the Highscores value), `reached_xp` (the character's total experience when it reached its current `drome_level`), `rotation_ordinal`, `waves_cleared` in that rotation, `cooldown_until`.
- `game_drome_battles`: one row per admission (the BOSS-RAID-0 allocation is its occurrence): participants, start wave, waves cleared, state `RUNNING`, `ENDED` or `LOST`.
- **Per wave.** A cleared wave `w` commits once, keyed (battle, `w`): each participant still in the instance gets `starting_level = max(starting_level, w + 1)` and, if `w + 1 > drome_level`, `lifetime_levels += (w + 1) - drome_level`, `drome_level = w + 1` and `reached_xp` = its experience at the commit. A fresh character that clears wave 0 gains exactly 1 of each, as the Manual says; a character that replays below its best changes only its starting level (the best-wave scoring, TibiaQA "best wave x 10"). A retry of the same key writes nothing.
- **Battle end.** One transaction, keyed by the battle: each participant's starting level drops by 5 (floor 0) and `cooldown_until = end + ARENA1-RL-06`; the battle becomes `ENDED`. **Crash** (architect ruling): the instance is lost; a recovery World job settles the battle exactly as an end (the per-wave commits already made stand; the drop and the cooldown apply), marks it `LOST`, and places participants at the exit anchor at their next admission. A server crash never preserves a starting level a normal end would have lowered.
- **Starting-level NPC.** The entry-hall NPC's `LOWER_START_LEVEL` command (free, as the Manual states no price) sets `starting_level = max(0, starting_level - 5)`, keyed by the command so a retry writes nothing.
- These are reward-domain rows like the BOSS-RAID-0 cooldowns: no `CharacterRevision` advance; BOSS-RAID-0 §12's lock order and fences apply.

**A7. Rotations and rewards, exactly once.**
- `game_drome_rotations`: (WorldId, ordinal) -> `starts_at`, `ends_at`, `modifiers` (the two drawn in A4), state `OPEN`, `SETTLING`, `SETTLED`.
- **Anchor.** Ordinals follow Tibia's: ordinal 138 begins at the planned World reset (the Tibia server save, ADR-0021) on **Wednesday 2026-10-14** (UTC date of the reset), and a rotation boundary is the planned World reset on the UTC date `2026-10-14 + 14 x (ordinal - 138)` days (always a Wednesday; ordinal 1 is 2021-07-14). `ARENA1-RL-07` is that 14-day spacing. The rotation ends when that reset activates, so a late reset moves the boundary with it; a World that opens mid-rotation starts at the current ordinal with a shorter first rotation and no catch-up. Source: TibiaWiki-Drome (every other Wednesday, at the server save, unaffected by resets) and TibiaXplorer (#138 ending 2026-10-28); the clock time is ADR-0021's operational reset time.
- **Settlement** is a World job (HOUSE-OWN-0 §9, at most `BOSSRAID0-RL-05` rows per pass). At the boundary the rotation becomes `SETTLING` and new battles count toward the next ordinal. The job freezes the ranking from the per-character rows of characters with `drome_level >= 1`: `drome_level` descending, then `reached_xp` ascending (the lower experience at the time of reaching the highest drome level wins, TibiaWiki-Drome), then CharacterId ascending (a deterministic last resort Tibia does not need because experience is rarely equal). For each such character it writes one `game_drome_rotation_rewards` row keyed (WorldId, ordinal, CharacterId), unique: drome points (`drome_level x 10`), the ranked potion count (ranks 1 to 5: 3, 6 to 10: 2, 11 to 20: 1), the raffle result and the title flag (ranks 1 to 5). Then it resets each character's `drome_level` to 0 and lowers `starting_level` by 5 (floor 0).
- **Raffle.** `R` is the number of ranked characters at rank 21 and below; the raffle awards `min(ARENA1-RL-13 (50), floor(R x ARENA1-RL-14 (20%)))` potions, one per winner, winners drawn without replacement with weight `drome_level`, under the RNG purpose `drome_raffle`. Rounding down is the decided default (the sources give no rounding).
- **Potions** (the 20 items below). Every awarded potion's kind is drawn uniformly from the 20 kinds under `drome_raffle`, seeded by (WorldId, ordinal, CharacterId, potion index) (Crystal's drome cube draws uniformly; no source states the rank distribution, so the uniform draw is the decided default).

| Potion (canary item id) | Effect (duration-based: 1 hour) |
|---|---|
| Kooldown-Aid (36723) | instantly resets the cooldowns of all spells; no duration |
| Strike Enhancement (36724) | +5 percentage points critical hit chance |
| Stamina Extension (36725) | restores 1 hour of stamina, never above the maximum; no duration |
| Charm Upgrade (36726) | +5 percentage points trigger chance of offensive charms except Cripple |
| Wealth Duplex (36727) | loot of every regular monster you kill is doubled (not bosses; the item text and TibiaWiki say "regular monsters") |
| Bestiary Betterment (36728) | each kill counts twice for the Bestiary |
| Fire, Ice, Earth, Energy, Holy, Death, Physical Resilience (36729 to 36735) | +8% protection against that element (additive points on the protection value) |
| Fire, Ice, Earth, Energy, Holy, Death, Physical Amplification (36736 to 36742) | your attacks and spells treat monsters' sensitivity to that element as 8% higher (monsters only, as the item text says) |

  A duration potion starts a 1-hour timer that runs while the character is offline; each potion kind has its own 24-hour cooldown from the drink, so different kinds stack and the same kind cannot be re-drunk within 24 hours (TibiaXplorer, TibiaWiki "Strike Enhancement" and "Wealth Duplex"). The two instant potions have the same cooldown. Potions are character-bound (the Manual: "most prizes can't be traded"). The effect hooks belong to the owning mechanics (spells, charms, loot, stamina, bestiary); DROME-DATA-1 only mints the items. The Crystal drome cube table lists "holy amplification" twice and omits ice and death amplification (a copy error); the 20 distinct kinds above are the canary item set.
- **Delivery.** Each reward row is consumed once: drome points credit a per-character balance with a ledger line keyed by the row (DUR-03 ledger discipline; balance at most `ARENA1-RL-10`); potions are DUR-03 MINTs into the character's inbox (INBOX-1) keyed by the row; the title (`DROME_TOP5`) is a timed grant ending when the next rotation ends. A retried step finds its key and writes nothing (the task's exactly-once criterion).
- **Spending** points at the entry-hall NPC (cosmetics, plushies, Drome Cubes, the Phant mount, per TibiaGoals) is a burn with a ledger line and a grant; it waits for an NPC currency exchange (§6).

**A8. PvE arena (Svargrond).** Source: Canary and Crystal (`svargrond_arena.lua`, `movements_arena_*`, Halvar), matching TibiaWiki.
- **Tiers** (arena definitions, content): Greenhorn (fee 1,000 gp), Scrapper (5,000 gp), Warlord (10,000 gp) (`ARENA1-RL-16`). Each has ten opponents in ten pits, in this order:
  - Greenhorn: Frostfur, Bloodpaw, Bovinus, Achad, Colerian the Barbarian, The Hairy One, Axeitus Headbanger, Rocky, Cursed Gladiator, Orcus the Cruel.
  - Scrapper: Avalanche, Kreebosh the Exile, The Dark Dancer, The Hag, Slim, Grimgor Guteater, Drasilla, Spirit of Earth, Spirit of Water, Spirit of Fire.
  - Warlord: Webster, Darakan the Executioner, Norgle Glacierbeard, The Pit Lord, Svoren the Mad, The Masked Marauder, Gnorre Chyllson, Fallen Mooh'tah Master Ghar, Deathbringer, The Obliverator.
  Opponents are encounter-bound creatures (ENCOUNTER-RT-0) with the stats of Canary's `monster/quests/svargrond_arena/` files (Canary and Tibia agree on the names and order; stats are content).
- **Run.** One instance per run for one character (Canary shares pits and queues players; instances remove the queue, a declared difference). The fee is a burn with a ledger line in the admission transaction (Halvar: "the fee is for one try"). Pit `i` spawns its opponent when the character enters it; killing it opens the way to pit `i + 1`; killing the tenth completes the tier. Each pit has a time limit of 600 s (`ARENA1-RL-15`); on expiry the character is moved alive to the arena's exit ("Your time ran out!"), the run ends and the fee is not refunded. The emergency exit (south portal of every pit) ends the run alive, the fee is not refunded and the character carries the "coward" outfit condition for 120 s (Canary). Death is `STANDARD` (A3): the run ends and a new attempt costs a new fee and starts at pit 1.
- **Completion.** Completing a tier is a QUEST-STATE-0 step (log step 2), granted once: the tier's achievement (Greenhorn, Scrapper, Warlord of Svargrond), the reward-claim content (`content/interactions/reward_claims/`, three claims, per character, once) and the tier's goblet at the trophy tile (bronze 5807, silver 5806, gold 5805, once per tier). A completed tier is not re-run; after Warlord the NPC refuses ("you've already completed the arena in all difficulty levels").
- No cooldown beyond the fee (Canary has none).

**A9. PvP arena.** Source: Canary and Crystal `arena_pvp/` for the shapes; the Duelling Arena for the rules.
- **Shapes.** A lever mode is fixed content: `DUEL_1V1` (the `arena_2x2` lever: 2 participants, one per side) and `TEAM_5V5` (the `arena_10x10` lever: 10 participants, 5 per side, the lever's tiles name the side). The lever requires every tile occupied by a character, else it refuses ("you need N players") and nothing is written; it refuses while the arena room is occupied ("please wait for the fighters to come out"), which instances replace with the one-instance-per-match rule. `ARENA1-RL-11` is 10. Last Man Standing and entrance-fee pools of the Duelling Arena are not offered: CipSoft removed the Duelling Arenas for low interest and abuse scenarios, and no repository or reference source defines a fee or prize flow that survives that abuse; v1 has no fee and no reward.
- Inside, PvP legality replaces PARTY-PVP-0 §7.2 rules 1, 4, 6 and 7 with "opposing teams only"; rule 2 (level), rule 3 (protection zone) and rule 5 (same party or guild cannot harm each other, applied within a team) hold. No aggression marks, skulls, unjustified points or revenge marks are written (§8 and §9 do not apply), so the World's `pvp_type` is untouched and the first World (Optional) can offer arena matches.
- `death_policy` is `ARENA_DEFEAT` with `defeat_anchor` `HOME_TEMPLE` (A3; the Duelling Arena rule: no death penalty, no frag, the temple of the home city; Canary and Crystal have no death script, so a standard death there, which Global parity does not follow).
- The match ends when one team has no participant left (that team wins) or at the time limit `ARENA1-RL-17` (30 minutes, a draw: all leave alive). The time limit is the one real limit not published by CipSoft; 30 minutes is the value of the OTLand TFS script that models the real arenas, adopted as the decided default. Waiting and preparation need no timer because the lever starts the match at once with everyone on the tiles.
- This is a PARTY-PVP-0 §7.2 amendment, pending on acceptance of this brief.

**A10. Multiplicity.** Arena rewards are Character-scoped and keyed by World and rotation, so channel count changes nothing: rotation rewards are `WORLD_SCOPED_UNIQUE` per character (GAME-CHANNEL-01 §14). Drome monsters drop nothing. PvE arena rewards are quest-once. ChannelId never enters a reward key. The two modifiers of a rotation are the same on every World.

**A11. Spectating.** The Tibiadrome roof is ordinary channel map: a character there sees what its position shows, under ordinary visibility. It is not an observer permission and grants no view into an instance; observing an instance belongs to SPECTATE-1.

**A12. Rows (proposed; registered by the children before implementation).**

| Row | Value | Source |
|---|---|---|
| `ARENA1-RL-01` Tibiadrome group size | 1 to 5 | Manual |
| `ARENA1-RL-02` Tibiadrome minimum level | 50 | Manual |
| `ARENA1-RL-03` wave time limit | 120 s | Manual |
| `ARENA1-RL-04` preparation phase | 8 s | Manual |
| `ARENA1-RL-05` monsters per wave by group size | 4, 8, 11, 14, 17 | Manual |
| `ARENA1-RL-06` battle cooldown | 60 minutes | Manual |
| `ARENA1-RL-07` rotation spacing | 14 days (anchor A7) | Manual, TibiaWiki-Drome |
| `ARENA1-RL-08` highest wave number | 300 | engineering ceiling (HP fits `i32`; real runs reach about 100) |
| `ARENA1-RL-09` marked tiles per modifier pulse | 100 | Manual |
| `ARENA1-RL-10` drome point balance | 100,000,000 | engineering ceiling |
| `ARENA1-RL-11` participants per PvP arena match | 10 | Canary and Crystal `arena_10x10` |
| `ARENA1-RL-12` active arena instances per node | set from measurement before activation, as `HOUSERT0-RL-04` | measurement |
| `ARENA1-RL-13` raffle potions per rotation per World | 50 | TibiaGoals, TibiaXplorer |
| `ARENA1-RL-14` raffle share of the remaining participants | 20%, rounded down | Manual, rounding decided |
| `ARENA1-RL-15` PvE arena pit time limit | 600 s | Canary `kickTime` |
| `ARENA1-RL-16` PvE arena fees | 1,000, 5,000, 10,000 gp | Canary `ARENA`, Halvar |
| `ARENA1-RL-17` PvP arena match time limit | 30 minutes | OTLand model of the real arena, decided |
| `ARENA1-RL-18` modifiers per rotation | 2 of 9 | TibiaWiki-Drome |
| `ARENA1-RL-19` seed period by group size | 20, 17, 14, 11, 8 s; seed step-on window 6 s | Manual |
| `ARENA1-RL-20` wave scaling | `base x 1.039^wave` for HP and damage; base HP 750, 800, 850, 950, 1100; wave-0 max hit 150, 300, 250, 220, 80; Domestikion +10% per monster, at most +50% | TibiaWiki-Drome, TibiaQA-HP; damage values decided |
| `ARENA1-RL-21` reward potion duration and cooldown | 1 hour; 24 hours per kind | TibiaXplorer, TibiaWiki |
| Lever admission | `BOSSRAID0-RL-07`, `BOSSRAID0-RL-08` (cited, unchanged) | BOSS-RAID-0 |
| Settlement pass | `BOSSRAID0-RL-05` (cited, unchanged) | BOSS-RAID-0 |
| Wave commit | 0 items, 0 value lines, 1 event | |
| Rotation reward delivery | at most 3 potions plus 1 raffle potion, 1 ledger line, 1 event | |

## 4. Packets (proposed)

Allocated one by one by the control plane after acceptance; each its own PR.
1. **ARENA-CONTENT-1** (content): `content/encounters/tibiadrome/`, `content/encounters/arenas/`, `rulesets/encounters/tibiadrome/` and `rulesets/pvp/arena/` schemas with the drome monster pool, the nine modifiers, locations, PvE arena tiers and PvP arena definitions, with provenance. No runtime.
2. **ARENA-INSTANCE-1** (hard, security and concurrency review): the arena instance kind on the BOSS-ROOM-1 admission and the SCOPE-HANDOFF-1 transition, A2 gates, the A3 `death_policy` (DEATH-0 amendment), the vitals restore, exit and recovery. Needs BOSS-ROOM-1, SCOPE-HANDOFF-1, ENC-RT-1, DEATH-1.
3. **DROME-RT-1** (hard, combat review): A4 and A5 (waves, modifiers, `drome_modifier`, consumption and progression suppression, soul-point refusal, PvP rule 0). Needs ARENA-INSTANCE-1, LOOP-MONSTER-1, ITEM-USE-1c, PVP-RT-1 (rule 0 only).
4. **DROME-DATA-1** (hard, persistence and economy review): migration and writers for A6 and A7 (per-wave commits, battle end, crash settlement, rotations, settlement job, `drome_raffle`, point ledger, potion MINTs, title). Needs DROME-RT-1, D3-3, INBOX-1, PARTY-XP-1 (group membership at admission).
5. **DROME-WIRE-1** (protocol, Reg): battle HUD (wave, timer, modifier), leaderboards, Highscores and the point balance; its capability is reserved on #162 at allocation.
6. **ARENA-PVE-1** (impl, combat review): A8. Needs ARENA-INSTANCE-1, QUEST-STATE-1.
7. **ARENA-PVP-1** (hard, combat and PvP review): A9 (PARTY-PVP-0 §7.2 amendment). Needs ARENA-INSTANCE-1, PVP-RT-1.

Order: ARENA-CONTENT-1, ARENA-INSTANCE-1, then DROME-RT-1, ARENA-PVE-1 and ARENA-PVP-1 in parallel, then DROME-DATA-1, then DROME-WIRE-1.

## 5. Sourced notes and declared differences

No point is open. Notes, each with its source:
- **Real-Tibia versus OT.** Canary and Crystal implement only the Svargrond arena and the lever PvP arenas; the Tibiadrome (waves, rotation, modifiers, leaderboards) is real-Tibia-only, taken from the Manual, TibiaWiki-Drome, TibiaQA-HP and TibiaGoals. Where an OT value differs from real Tibia, real Tibia is used (Global parity): Duelling Arena death rule (no penalty, home temple) over the OT standard death; one instance per run over Canary's shared pits and queue.
- **Engineering defaults, no source anywhere.** Damage curve and wave-0 max hits (A4, `ARENA1-RL-20`), Domestikion buff size, hazard radii and percentages not stated by TibiaGoals, Murmillion paralyze and Scissorion invisibility durations, raffle rounding (down), the uniform potion kind draw, the PvP match time limit (30 minutes), the tie fallback by CharacterId, `ARENA1-RL-08` and `ARENA1-RL-10`. Each is registered as an `ARENA1-RL` row, so a change is a registry change.
- **Excluded by design.** Duelling Arena entrance fees, pots and Last Man Standing (CipSoft removed the arenas for abuse scenarios, 2023-02-14); no fee or reward flow in the PvP arena.
- **Data.** The Crystal drome cube table has a copy error (holy amplification twice); the canary item ids 36723 to 36742 are authoritative.
- **Task record.** `docs/agents/tasks/ARENA-1.md` lists `content/arenas/` as a source input; the tree has `content/encounters/arenas/` and `content/encounters/tibiadrome/`, corrected there.

## 6. Split out, not decided here

- **Drome point spending** (cosmetics at the entry-hall NPC): needs the NPC exchange contract that CRAFT-1 §6 also splits out, plus outfit and mount ownership. Proposed DROME-SHOP-1, blocked on that decision.
- **Character trading and the leaderboard**: removal during an auction and the forfeit rule belong with the Character Bazaar (a Platform boundary).
- **Tournaments, ratings and matchmaking queues** (gap register §20): no reference requirement in this scope.
- **Observing an arena instance**: SPECTATE-1 (A11).

## 7. Handback

```yaml
result: CANDIDATE
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_ARENA1_DECISION_2026-10-15.md
resource_values_changed: false
production_authority_changed: false
implementation_may_resume: false
required_fresh_allocation: true
required_independent_review: "security, concurrency, persistence, economy, combat, protocol"
amends_pending_acceptance: "DEATH-0 §3.1 and PARTY-PVP-0 §7.2, §10 (A3, A5, A9); ITEM-USE and RUNE-USE consumption (A5)"
```
