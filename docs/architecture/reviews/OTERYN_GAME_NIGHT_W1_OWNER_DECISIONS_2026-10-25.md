# NIGHT-W1 owner decisions (2026-10-25)

The owner answered the 18 open questions from the NIGHT-W1 light-mechanics parity pass. "As global"
means Tibia Global behaviour. Where it could not be observed directly, it is read from Canary at the
pinned revision `47dfd51f45280a59a1d3e50ba7edd573d7234446` or from TibiaWiki. This record holds the
decisions and their follow-ups. The contracts named in each row are amended only by the PR that
implements that row. Decision 19 records the subsequent death-skill correction.

| # | Question | Decision | Follow-up |
|---|---|---|---|
| 1 | Damage over time inside a protection zone | As global: the conditions Canary `Combat::hasPermittedConditionInPZ` lists (fire, poison, energy, bleeding, agony, cursed, dazzled) keep ticking in a PZ | COND-1 PZ rule; separate PR |
| 2 | Spells and runes while feared | As global: refused, Canary `Spell::playerSpellCheck` | SPELL-FEARED-GATE-1 (#2420) |
| 3 | Buying a promotion without premium | As global: premium required (Canary `modules.lua:106`). Already the rule: `promotion::purchase_permitted` refuses `PremiumStatus::NotCurrent`. Only the sealed pre-delivery exception (PREMIUM-ACTIVATION-0) admits a purchase before Premium starts on a deployment, and it ends at that start. The Wheel still skips its Premium condition (`wheel.rs` `NotPremium`, "until PREM-1 delivers"), and PREM-1 has since landed | WHEEL-PREMIUM-1: read the Premium gate in the Wheel allocation transaction (`character_wheel.rs`); PostgreSQL tests |
| 4 | Logout in a PZ under a PvP block | As global: allowed (Canary `ProtocolGame::logout`) | LOGOUT-PZ-PVP-1 (#2421) |
| 5 | Do own area spells hurt own summons | Research: Canary's area target check excludes only the caster (`combat.cpp:1562`, `:1610`). No official global source was found. Oteryn keeps its rule: an aggressive area spell hits only masterless monsters in PvE | Re-open on a global capture |
| 6 | Energy Ring wearers and mana cost | Knights, paladins and monks may wear it (TibiaWiki, Monk since April 2025). Since update 15.25 (June 2026) it takes 2 mana per point of damage instead of 1 | ENERGY-RING-1: Oteryn has no Energy Ring mechanic yet, and its mana shield absorbs 1:1 |
| 7 | Melee damage formula | Keep the TibiaTools-derived formula | Verify against a global Server Log capture (about 100 hits, known stats) |
| 8 | Attack spell damage formula | Keep the official base_power model | As 7 |
| 9 | Stamina drain while hunting | Do it | STAMINA-1: heavy (migration, hunt clock, saves at logout, death and checkpoint) |
| 10 | Distance-weapon and wand content | Do it, with data gathered from the web and tools (TibiaWiki, TibiaData, Canary items) | RANGED-CONTENT-1: wand damage and element, mana per shot, ammunition type |
| 11 | Corpse decay and ground items | As global: corpses decay on their timer (D113, D137); other ground items stay until the planned reset (ADR-0021 D191, LOOP-GROUND-1). Neither rule replaces the other | None: already the reconciled reading |
| 12 | Exhaust after stairs (2 s) and after mounting (3 s) | Yes | With native stairs (FLOOR-1) and mounts |
| 13 | Training on a dummy with a wand, rod or bow from a distance | As global: allowed | With the ranged attack wiring |
| 14 | Look description at a distance | As global (owner correction, 2026-10-25): look works on anything in the whole field of view. Canary shows the weight and some details only within one tile; whether global does the same is checked at ITEM-LOOK-1, never by refusing a far look | ITEM-LOOK-1 |
| 15 | Moving or using an item from afar | As global (owner correction, 2026-10-25): using a far item walks the character next to it and then uses it. Canary `Game::playerUseItem` (`game.cpp:4783`) does this on "too far away", and `Game::playerMoveItem` (`game.cpp:2129`) does the same for a move. With no path, it answers "There is no way." Oteryn has no server-side player auto-walk yet, so the "too far" refusal stays until that task lands | USE-AUTOWALK-1: heavy (player path search, walk queue, retry of the use or move) |
| 16 | Adventurer's Blessing level limit | From global: lost on first reaching level 21, so it protects through level 20 (TibiaWiki). PARTY-PVP-0 §10.2 already says "level 20 or less" | None: already consistent |
| 17 | Rat corpse id in D113 | Correct it: D113's `i00005801` is the D3-7 alias of `oteryn:item.tibia.i5964`, the dead rat. 5801 is the Jewelled Backpack in Canary | D113 note in the D3 decision (this PR) |
| 18 | Who wires parameter spells (exiva etc.) to the session | Yes: the same dispatch owner as ordinary spell casting | PARAM-CAST-WIRE-1: heavy (Hello capability and owner caller) |
| 19 | Skill and magic level loss on death | Differs from Global: never lost (restores D58 / baseline difference 2; supersedes SKILLS-0 §3.6) | DEATH-NO-SKILL-LOSS-1: preserve every skill/ML level and progress accumulator; XP loss unchanged |
