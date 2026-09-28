# Oteryn Spell Authoring Schema v1 and implementation plan

- Date: 2026-09-27
- Status: CANDIDATE / authoring schema with executable validation and source evidence; S1–S5, S11 and S12
  decided by the owner on 2026-09-27, S13–S24 and S6–S10 on 2026-09-28 (S6–S10: #162 comment 5867161696); no runtime, WorldProject storage or `content/` change
- Request: owner request of 2026-09-27 (schema and implementation plan for player spells, as for monsters);
  programme story KAN-16; no GitHub task allocation yet
- Machine artifacts: `tools/content-schema/spell-authoring/`
- Companion of: `OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md` (Ability/Effect/Formula, D10–D13, D15, D25),
  `OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md` (rune items), `OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md`
  (`content/abilities/**`)
- Sources: `opentibiabr/canary@99902524e052f37574194466c2949c576e4ab269` (branch
  `dudantas/fix-tibia-15-30-regressions`, S14; `47dfd51f` before S14),
  `zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a` (both `OtsHypothesisOnly`),
  TibiaWiki (Fandom) as of 2026-09-27 (S3). TibiaWiki BR is the registered primary structured source
  (`OTERYN_REFERENCE_INVESTIGATION_SOURCE_REGISTRY_20260910.md`) and is captured in phase P1.

## 1. Outcome

Before this document Oteryn had a complete legacy inventory of player spells (199 spells + 36 runes,
`OTV2-20260921-content-world-cw2-ability-family-source-catalogue.json`, all `UNRESOLVED`), the
fixture-only ability engine and the monster schema, which admits a player spell only when a monster
casts it (D11). There was no format for the player side of a spell and no plan to reach castable spells.

This candidate adds:

1. the player Spell authoring format (`spell.schema.json`), reusing the monster Ability, Effect and
   import-ledger definitions instead of a second combat model;
2. the `player_expression` Formula: the exact source damage/heal formula as an expression tree;
3. a census of every player spell and rune script in Canary and Crystal Server, with the formulas
   extracted by symbolic execution, and a field comparison with current TibiaWiki (Fandom);
4. the implementation plan (§9) from this schema to castable spells in the real game server.

The server does not read this format. Executable adoption extends WorldProject/v2 as §6 describes.

## 2. Shape

```text
spell bundle (spell.schema.json)
└── spell   identity, name, carrier (instant | rune), words, reference_spell_id, library_text,
            requirements (vocations, level, premium, learning_required, acquisition_interactions),
            costs (mana | mana_percent, soul), cooldown_ms, groups[1..2] (group, cooldown_ms),
            targeting (aggressive, self_target, needs_target, needs_direction, target_or_direction,
                       range_tiles, block_walls, allow_on_self, check_floor, parameter,
                       aim_at_target, cast_at_position),
            pz_locks_caster, needs_weapon, base_power,
            rune (item, charges, magic_level, allow_far_use, blocking)      -- carrier = rune
            execution: ability | conjure (reagent, result, count) | native_behavior (key, parameters)
            presentation (cast_cue, impact_cue)

direct dependencies (spell-dependencies.schema.json)
└── abilities[] + effects[]   monster-authoring definitions, unchanged
    formulas[]                monster kinds, or player_expression (inputs, minimum, maximum)

import disposition ledger     monster-import-readiness.schema.json, unchanged
```

Every cross-definition link is an exact `{family, key, revision}` reference. `Spell` is the new family;
Item, Interaction and Creature references resolve against the catalogue as in the monster schema.

## 3. Source census

`samples/spell-census-canary-47dfd51f-crystal-ff7ede5.json` (`spell_census.py`). Each script is loaded
in a stubbed LuaJIT sandbox; `Spell`, `Combat`, `Condition` and `createCombatArea` record their calls.
Damage/heal callbacks are called with symbolic arguments, so arithmetic builds an expression tree in
source operation order.

| | Canary 47dfd51f | Crystal ff7ede5 |
|---|---:|---:|
| registered spells (instant + rune) | 235 (199 + 36) | 252 (216 + 36) |
| monster-only registrations (words `#…`) | 3 | 5 |
| `plain_combat`, declarative (one Combat, no Lua callback, formula resolved) | 106 | 83 |
| `plain_combat` with a per-target Lua callback / unresolved formula / `setFormula` | 6 / 3 / 1 | 7 / 0 / 1 |
| `conjure` (runes, arrows, food items) | 49 | 49 |
| `custom` (native behaviour candidates) | 67 | 107 |
| damage/heal formulas resolved / unresolved | 95 / 6 | 168 / 5 |

Canary and Crystal share 233 spells; 2 are Canary-only (`lightest magic missile`, `mentor other`, which
Crystal removed) and 19 Crystal-only (for example `master of flames`, `shield bash`, `death echo`,
`divine barrage`). 128 shared spells differ: `basePower` (100, absent in Canary), `needLearn` (30),
`cooldown` (11), `range` (11), `groupCooldown` (10), the formula (93) and a few requirements.

`custom` bodies carry pattern tags (a spell may carry several), Canary / Crystal: Wheel of Destiny
gating 21 / 22, elemental or aura stance – / 27, equipment-dependent (shield, weapon bond) 1 / 16,
monk harmony/virtue 6 / 14, world query (spectators, tiles, corpses) 13 / 13, delayed or repeated
strikes 7 / 10, conditional self state 6 / 10, familiar 9 / 9, summons share the condition 7 / 7,
party 5 / 6, player parameter (`exiva name`, levitate, illusion) 5 / 5, house 4 / 4, other 1 / 1.

Formula findings:

- Crystal uses the current Reference form: `basePower` per spell (108 spells) and the level curve
  `calculateBaseDamageHealing(level)` (160 formulas); Canary mostly keeps `level / 5` (78 of its 91
  level formulas; 7 monk formulas use `calculateFlatDamageHealing`).
- The two level curves differ. Crystal (`register_spells.lua`) is an integer closed form. Canary
  `Player::calculateFlatDamageHealing` (7 monk formulas) adds the whole previous threshold instead of
  its span: level 1100 gives 284 instead of 200, level 2000 gives 566 instead of 325. This is a Canary
  defect in the same sense as the monster §8 findings.
- The TibiaWiki `Formulae` page (section "Damage and Healing", revision in the facts file) defines the
  curve since patch 13.05.12657 (October 2022): +1 every 5 levels up to 500, then every 6 up to 1100,
  every 7 up to 1800, and so on, as `S = floor((sqrt(2L + 2025) + 5) / 10)`,
  `B = floor((L + 1000) / S) + 50S - 450`. This is exactly Crystal's function (S5). The curve is
  unchanged in the current client 15.30 (July 2026); the 15.25 vocation adjustment (June 2026,
  `Updates/15.25.3a4a52`) changed per-spell base powers, not the curve.
- Primary evidence for the curve: the official news "Adjustment of Damage and Healing Scaling per
  Level" (tibia.com news 6972, server save of 2022-10-18; tibia.com answers 403 here, the full text
  is mirrored on tibiopedia.pl and tibiaevents.com): +1 every 5 levels up to 500, every 6 for 501–1100,
  7 for 1101–1800, 8 for 1801–2600, 9 for 2601–3500, "above level 3500 it continues with the same logic",
  and the worked example "level 800: 160 before, 150 after". `verify_formal_schema.py` checks that the
  closed form equals this table for every level 0–20000 (and gives 150 at level 800). Below level 500
  the bonus is `floor(level / 5)`, as players measured (Fandom `Talk:Formulae`); the older Canary
  scripts use the unfloored `level / 5`, and Canary's `calculateFlatDamageHealing` also rounds up.
- No later change: the changelists of all 227 TibiaWiki update pages from 13.05.12613 to 15.33 (read
  2026-09-27) change the level scaling only in 13.05.12657; later entries touch per-spell base damage,
  the Wheel of Destiny (for example the flat +1/+2 Vessel Resonance bonus in 13.30) and hazard scaling,
  not the level curve. The `Formulae` "Base Damage and Healing" section was written on 2022-10-17/18
  and has only been reworded since (last on 2025-04-05). The 15.25.bd5a04 hotfix lowered several base
  powers after 15.25.3a4a52 (great beams 170 → 155, Death Echo 85 → 75, Forked Glacier 97 → 90), which
  is why Crystal (15.25 values) differs from the current wikis there.
- Balancing after 15.30, from the official news mirrored on tibiopedia.pl (topics 32500–32913, read
  2026-09-27): "Vocation Adjustments Changes" (2026-07-07: knight healing mana Wound Cleansing 40 → 60,
  Fair 90 → 135, Intense 200 → 300, several base damage reductions), balancing posts of 2026-07-28,
  08-04, 08-05, 08-25 and 09-01 (hunting grounds, bosses, gold and XP only) and "Monk Adjustments"
  (2026-09-01: Spirit Mend base power 210 → 240; Mystic Repulse and Thousand Fist Blows cooldown
  12 → 8 s; Devastating Knockout cooldown 24 → 8 s and range 1 → 7). The only new level-dependent rule
  is the monk Harmony base bonus, `7% + 0.005% × level` (level 200: 8%, level 700: 10.5%), raised by
  50% by Virtue of Harmony (100% while Serene); it is a monk resource rule, not the damage/healing
  curve. Neither Crystal nor Canary carries the 2026-09-01 changes yet (no spell commits since
  Crystal ff7ede5 up to 9f5a72c or in Canary up to 04b83b5). The dated official news is the tie-breaker
  evidence for the S3 BR/Fandom conflicts (for example the knight healing mana follows BR, the Mystic
  Repulse cooldown follows Fandom).
- Per-spell formulas: the `Formulae` section "Spell/Rune Damage/Healing" still lists the old
  `floor(lvl * 0.2) + mlvl * x + y` coefficients and marks them as no longer correct since 2020, so it
  is not a formula source. Crystal PR #797 (merged 2026-07-01, "15.25 Base Spell Power") implements
  the base-power forms `avg = B(L) + (bp / 25) * mlvl + bp / 6` (magic) and
  `avg = B(L) + (bp / 1000) * skill * atk + bp / 6` (skill), with base powers taken from TibiaWiki;
  the PR itself marks the healing coefficients as not yet confirmed. Formula shapes therefore stay
  source values under S4 (UNKNOWN parity until measured), while base powers follow the wiki (S3).
- Both engines draw the final value with `normal_random(min, max)` after truncating each bound to an
  integer (`LuaScriptInterface::getNumber<int32_t>`, `ValueCallback::getMinMaxValues`).

## 4. TibiaWiki (Fandom) comparison, as of 2026-09-27

`samples/wiki-spell-compare-fandom-2026-09-27.json` (`wiki_spells.py`, S3): 217 `Infobox Spell` pages
and 55 rune item pages (`Category:Runes`), each at its last revision on or before 2026-09-27, plus the
`Formulae` page. Instant and conjuring spells are joined by words (the wiki may append the parameter,
`exura sio "name`), runes by item id.

| field (match / diff) | Canary | Crystal |
|---|---:|---:|
| words | 187 / 1 | 203 / 0 |
| vocations (base vocations) | 185 / 0 | 197 / 3 |
| level | 214 / 10 | 226 / 13 |
| mana | 172 / 11 | 186 / 12 |
| soul | 187 / 1 | 202 / 1 |
| premium | 156 / 32 | 162 / 41 |
| cooldown | 174 / 14 | 181 / 22 |
| primary group cooldown | 183 / 5 | 202 / 1 |
| base power | – (absent) | 84 / 7 |
| rune magic level | 36 / 0 | 36 / 0 |
| conjure amount | 34 / 0 | 34 / 0 |

Compared: Canary 225 spells, Crystal 240; unmatched source spells are monster-only registrations and a
few conjuring spells whose wiki page name differs. 14 wiki spell pages have no Crystal spell: 11 are
`deprecated` or `ts-only`, the others are `Gift of Life` (Wheel), `Lesser Mystic Repulse` (15.12) and
`Mentor Other` (removed by Crystal). Under S3/S4 the differences are adopted from the wiki, for
example base power 155 instead of Crystal's 170 for the great beams, Spirit Mend 250 instead of 220,
the familiar summon cooldown of 30 minutes (both sources register 0), and the premium flags.

### 4.1 TibiaWiki BR, as of 2026-09-27

Captured on a hosted runner (`.github/workflows/spell-wiki-capture.yml`, run 36355786702, artifact
`spell-wiki-br-ee6e961…`, SHA-256 `dabc861f…e939`): 186 `Infobox_Spell` pages and 36 `Infobox_Runas`
pages. `samples/wiki-spell-facts-br-2026-09-27.json` keeps the mapped short facts with revision ids
(`wiki_spells.py br-facts`, field mapping in the file): `expLvl` → level, `cooldownproprio` /
`cooldowngrupo` → own / group cooldown, `subclass` "Ataque, Focus" → primary and secondary group,
`premium` sim/não; an `Infobox_Runas` page describes both the rune (level, magic level) and its
conjuring spell (`makelvl`, `makemana`, `makeqty`, `makevoc`). BR categories that are not cooldown
groups (Suprimento, Summon, Party, Stance) map to the Support group. BR has no `Fórmulas` level curve
(page last edited 2011); the curve stays the Fandom `Formulae` one.

| field (match / diff), `samples/wiki-spell-compare-br-2026-09-27.json` | Canary | Crystal |
|---|---:|---:|
| words | 189 / 1 | 205 / 0 |
| vocations | 185 / 5 | 197 / 8 |
| level | 203 / 17 | 218 / 17 |
| mana | 168 / 17 | 183 / 17 |
| premium | 158 / 32 | 164 / 41 |
| cooldown | 170 / 20 | 177 / 28 |
| primary group cooldown | 183 / 7 | 200 / 5 |
| base power | – | 82 / 10 |

### 4.2 BR ↔ Fandom crosswalk (S3 conflicts)

`samples/wiki-spell-crosswalk-fandom-br-2026-09-27.json`: 207 spells joined by words (the Fandom words
may carry the parameter) and 32 runes by name. The wikis agree on almost everything: premium 205 / 1,
vocations 201 / 2, group cooldown 201 / 5, mana 201 / 6, level 197 / 10, cooldown 195 / 10, base power
89 / 3; rune level and magic level 32 / 0. The conflicts are the owner's S3 list, for example:

- knight healing after 15.25 (Wound Cleansing, Fair and Intense Wound Cleansing, Bruise Bane): BR has
  the 2 s cooldown and higher mana, Fandom the older values; `Mystic Repulse` base power BR 85 /
  Fandom 72 (BR matches the 2026 vocation adjustment);
- healing runes (Cure Poison, Intense and Ultimate Healing Rune): cooldown and group cooldown BR 1 s /
  Fandom 2 s; `Paralyse` cooldown BR 6 s / Fandom 2 s;
- Wheel revelation spells (Divine Grenade, Ice/Terra Burst, Executioner's Throw, Spiritual Outburst,
  Divine Empowerment): level BR 1 / Fandom 300;
- secondary groups of auras and virtues (BR stance / virtude / focus, Fandom crippling / stance);
- deprecated spells (Force Strike, Ultimate Explosion).

### 4.3 Three-reference check: Fandom, BR and tibiopedia.pl (2026-09-28)

`tibiopedia_spells.py` read the 198 spell pages of tibiopedia.pl (S12) on 2026-09-28 into
`samples/tibiopedia-spell-facts-2026-09-28.json`: 198 spell rows and 34 rune rows with the page URL and
page SHA-256, in Fandom field names. `verify_spells.py` put every converted bundle (§8.1) next to Fandom,
BR and tibiopedia.pl field by field (`samples/spell-verify-3-sources-2026-09-28.json`):

- 238 of 249 bundles join a reference (11 do not: house list spells, Blank Rune, Sharpshooter and five
  runes without a current page);
- the references and our value agree on premium 207, soul 205, group 205, vocations 204, level 224,
  group cooldown 200, cooldown 197, mana 196, base power 104, rune magic level 31 and charges 28;
- **no ready spell has a value on which all references agree against us**; the 7 such fields are in
  blocked spells (party spell mana stated as varying, two training runes' conjure amount on BR,
  Virtue of Sustain base power), which the S7 native behaviours will carry;
- before S13, 9 fields of 6 ready spells followed one reference while the other two agreed: the
  conjuring spells of Cure Poison, Intense Healing and Ultimate Healing Rune (cooldown and group
  cooldown: BR 1 s, Fandom, tibiopedia.pl, Canary and Crystal 2 s), Paralyse Rune (cooldown: BR 6 s,
  the others 2 s), Wound Cleansing (cooldown: BR 2 s, Fandom, tibiopedia.pl, Canary and Crystal 1 s) and
  Bruise Bane (cooldown: Fandom and Canary 1 s, BR and tibiopedia.pl 2 s); S11 had taken the newer
  revision;
- **after S13 (conversion `spell-p2-r2`) no field follows one reference against the other two**; the
  references still disagree on 51 fields, where our value follows at least one of them in 50 (the other is
  the blocked Summon Creature mana: BR "varies", Fandom and tibiopedia.pl 100). Readiness is unchanged
  (141 ready, 108 blocked).

### 4.4 Canary 15.30 branch and official library (2026-09-28)

The active branches of both sources were read on 2026-09-28. Neither `main` changed a spell script since
the pins. Crystal's active branches change no spell value (`summer-update` adds comments,
`feat-expert-pvp` touches PvP). Canary `dudantas/fix-tibia-15-30-regressions` (last commit 2026-07-28,
not merged) implements the Tibia 15.30 spell changes:

- 30 changed spells: knight healing, druid attack and healing formulas, Strong Ice Wave, Mass Spirit Mend,
  Mystic Repulse, stances;
- 20 new or renamed spells: auras, familiars, Master of Flames/Decay/Thunder, Elemental Synthesis, Divine
  Defiance, Shared Conservation, Thousand Fist Blows.

With S14 the census reads that branch (`samples/spell-census-canary-99902524-crystal-ff7ede5.json`):

- 252 spells: 152 ready (was 141) and 100 blocked;
- 15 spells become ready and 4 become blocked. Flurry of Blows, Front Sweep and Strong Ice Wave run Wheel
  logic in both sources (S6/S7); Divine Barrage has a `needTarget` conflict no wiki decides;
- Strong and Ultimate Energy/Flame Strike get range 7 (BR, tibiopedia.pl and the branch);
- the vote returns Wound Cleansing's cooldown to 2 s (BR and the branch against Fandom and tibiopedia.pl;
  the official news of 2026-07-07 changes only its mana).

With S16 (patch 15.22 unlock) and the S9 catalogue: 154 ready and 98 blocked; with S17: 159 ready and 93 blocked. The 14 `needLearn` S4 conflicts are
gone, and 15 Wheel revelation spells carry `wheel_unlock`: the avatars, Divine Empowerment, Divine Grenade,
Executioner's Throw, Flurry of Blows, Focus Harmony, Forceful Uppercut, Ice Burst, Mystic Repulse, Spiritual Outburst
and Terra Burst. S22 (`spell-p2-r8`) corrects this to 13: Mystic Repulse and Flurry of Blows unlock at their tibia.com level.
The 11 revelation spells have level 0, and Focus Harmony and Forceful Uppercut keep the Canary `needLearn` gate.

S20–S22 (`spell-p2-r8`) bring readiness to 161 ready and 91 blocked:
- **S21.** Divine and Ethereal Barrage become ready. The other five former S4 conflicts now wait only on S7 native
  behaviours: Avatar of Balance, Enlighten Party, Focus Harmony, Focus Serenity and Shield Slam.
- **Three-reference check.** It now shows five `ours_differs` rows, all on blocked avatars: the level 0 of S22 against
  the wikis' 300.

S23 (`spell-p2-r9`) brings readiness to 162 ready and 90 blocked:
- **Chained Penance** becomes ready. Its combat carries the accepted `chain` block.
- **Lightning** stays ready. Its chain now has 2 further creatures and jump range 4 (Canary, official 15.25.3a4a52).
  Before, it had 3 and 5 from Crystal.
- **Forked Thorns and Forked Glacier** convert with their fork chains. They stay blocked on the secondary `special`
  group cooldown, which no source states (S9).
- The game core rejects an ability with `chain` until the chain runtime exists.

S24 (`spell-p2-r10`): official news check. The TibiaData API mirrors the tibia.com news archive; tibia.com itself
answers 403 here. It was read for all development/technical news and tickers from 2025-12-01 to 2026-09-27: 158 items,
ids 8569–8984. That gives 89 numeric player-spell field changes from the release state 8833 onwards; the test-server
announcement 8783 is not final.
- The converted data agrees with 88 of them.
- Death Echo mana 155 (8833, 2026-06-02) is superseded by the later official tibia.com list, which states 150.
- Expose Weakness and Sap Strength were removed by 8833 (they became the level-175 Aura stances). They are blocked
  as removed spells, not as native-behaviour candidates.
- A comparison with the current TibiaData spell library found a converter error. The library is the current state,
  not the target date, and was used only as a check. The group of a rune now comes from the wiki `runegroup`;
  `subclass` is the group of the spell that conjures the rune. Intense and Ultimate Healing Rune move from `support`
  to `healing`, as Canary, Crystal and the library state.

The official tibia.com spell library (S15) answers a Cloudflare browser check from the build container
and blocks GitHub-hosted runners outright ("Sorry, you have been blocked"). `tibiacom_spells.py fetch`
therefore runs on an ordinary machine the site serves (no challenge bypass).

On 2026-09-28 the owner copied the library list view: name, words, group, type, level, mana and premium for
193 spells. `tibiacom_spells.py list-facts` turns that copy into `samples/tibiacom-spell-list-2026-09-28.json`,
which records the copy's SHA-256. The converter applies it as S15 (`spell-p2-r5`):

- **Join.** The converter joins a spell to its tibia.com row by name, because tibia.com corrects words.
  Three names differ: Invisible, Paralyse Rune and Summon Monk Familiar are Invisibility, Paralyze Rune and
  Monk familiar in the sources. A list row of type "Rune" describes the conjuring spell, so it never
  applies to rune use.
- **Result.** Of the 189 joined spells, level, mana, premium and group all agree with our values. The only
  correction is words: Canary, Crystal and tibiopedia.pl swap Aura of Exposed Weakness (`exori moe tempo`)
  and Aura of Sapped Strength (`exori kor tempo`). Fandom, BR and tibia.com agree on the correct words, which
  the converter now uses and also uses to select the wiki pages.
- **Values the list does not state.** For Lesser Ethereal Spear, Lesser Front Sweep, Swift Jab and Tiger
  Clash, which are not Wheel spells, the list gives "-" for level: no value, so the source level stands.
  Five party and summon spells show "var." for mana and stay blocked.
- **Missing spell.** Lesser Mystic Repulse (`exori infir amp pug`, level 6, 30 mana) exists in neither
  source, so it has no mechanics to convert.
- **Still to capture.** The detail pages (cooldowns, soul, magic level, vocations) are not captured yet;
  until they are, the rules above decide those fields.

**Client spell id (`spell-p2-r6`).** `reference_spell_id` now follows the wiki `spellid` field under S3, as §2
always described; earlier revisions read only the source `spell:id`. 43 values change:

- **Runes.** Rune use now carries the id of its conjuring spell, for example 21 for Sudden Death Rune; no source
  states an id for rune use.
- **Instant spells that had no id.**
  - Find Fiend takes the wiki 248, which settles the Canary 248 and Crystal 20 conflict.
  - Lightest Magic Missile (Canary) and Practise Magic Missile (Crystal) both join the wiki page Practise Magic
    Missile and take 168.
- **Instant spells where the wiki supersedes the source id.**
  - Lesser Front Sweep changes from 168 to 271; in the sources, 168 also belongs to Practise Magic Missile.
  - Lesser Ethereal Spear changes from 169 to 270.
  - Sharpshooter changes from 313 to 135.
  - Bruise Bane changes from 170 to 175.
  - Mud Attack changes from 174 to 172.
  - Ice Burst and Terra Burst swap 262 and 263.

**Full-page check (2026-09-28).** The rendered Fandom `Spells` list (revision 1178141; 162 instant and 33 rune
spells) and every page of `Category:Runes` and its subcategories (62 articles, current revisions) were compared
with the converted bundles:

- **Rune spells.** All 36 rune bundles agree on item, level, magic level, vocations and base power. The pages
  without a bundle are `TS-only` "(Weak)" runes, the deprecated Envenom Rune, Combustion Rune, and three
  unobtainable Isle of Destiny runes.
- **Instant and conjuring spells.** 185 of 191 joined rows agree on words, premium, level, vocations, mana, group,
  soul and amount. Five mana values differ because tibia.com decides under S15: the three Wound Cleansing spells,
  Mass Spirit Mend and Shield Slam.
- **Stale Magic Patch capture.** The sixth difference, Magic Patch, exposed a stale capture. The committed Fandom
  facts had no fields for Magic Patch: the page uses `{{Infobox_Spell`, which the current `infobox()` matches but
  the capture predates.
  - Regenerating the facts from the same pinned revisions changes only that page.
  - The Magic Patch vocation conflict is now explicit: BR and tibiopedia.pl include monks, Fandom does not, and
    S13 keeps monks 2:1.
  - The Fandom comparison and the crosswalk are regenerated with it.

## 5. Decisions

| # | Proposal | Basis |
|---|---|---|
| S1 | **DECIDED (owner, 2026-09-27).** A player spell is a `Spell` definition (casting layer) whose execution is an `Ability`. A spell a monster also casts keeps the one shared Ability (`oteryn:ability.spell.<name>`, D11); only the monster schedule overrides its magnitude. | D11; 7 player spells/runes already exist as shared Abilities in `content/abilities/`. |
| S2 | **DECIDED (owner, 2026-09-27).** Two carriers: `instant` (spoken words) and `rune` (an Item with charges). A conjuring spell is an instant spell whose execution is `conjure` (reagent → result × count). The rune Item stays Item authority (item schema kind 8, rune); the Spell references it. | Canary/Crystal `Spell("instant")` / `Spell("rune")`, `Player:conjureItem`; 49 conjure bodies. |
| S3 | **DECIDED (owner, 2026-09-27).** TibiaWiki BR and Fandom complement the two OTS sources because they carry the most current data; the wiki is read as of the working day, not at a historical cut. Where a wiki states a value (words, vocations, level, magic level, mana, soul, premium, cooldowns and groups, base power, conjure amount, rune item, formulas), it decides; a BR/Fandom conflict stays `CONFLICT` for the owner. Canary/Crystal supply what the wikis do not state (area, effects, conditions, formula shape, targeting flags). | Owner: "wiki ma być na dzień dzisiejszy"; D15/D25 precedent; §4 differences (premium, cooldown, level). |
| S4 | **DECIDED (owner, 2026-09-27).** Canary 47dfd51f and Crystal ff7ede5 are equal sources with no automatic winner: each field is taken from the source that has it, or has it right against the wiki (Crystal has base power and the current level curve, Canary other details). When both have a value and disagree, the wiki decides (S3); when the wikis are silent it stays `CONFLICT` for the owner. The replaced value stays `approved_omission` in the manifest. **Amended by S21 (2026-09-28):** a conflict no wiki or tibia.com states follows the Canary 15.30 branch. | Owner: "crystal/canary są równe, jeden ma jedną rzecz lepiej, drugi inną"; same rule as NPC D2. |
| S5 | **DECIDED by S3.** Damage/heal formulas are `player_expression` trees over declared inputs, evaluated in IEEE-754 double in authored order; each bound is truncated toward zero; the draw between bounds is the world damage distribution (a world rule, like D12). The level contribution is one world function `level_base_damage_healing` = the TibiaWiki `Formulae` curve (§3), which Crystal implements exactly; Canary's `calculateFlatDamageHealing` is a defect and is not used. | §3 formula findings; Fandom `Formulae`. |
| S6 | **DECIDED (owner, 2026-09-28).** Wheel of Destiny, gem, weapon-proficiency and imbuement augments are not part of a Spell. A Wheel-gated spell is authored with its base behaviour and a `wheel_unlock` native gate; augments use the existing `ProjectV2AugmentBinding`. `wheel_unlock` fails closed: the spell is not castable until a Wheel owner exists (ADR-0019). | 21–22 Wheel-gated bodies; v2 already has augment bindings. |
| S7 | **DECIDED (owner, 2026-09-28).** `custom` spells become `native_behavior` keys with data parameters, shared by pattern (party buff, house list, summon familiar, find person, levitate, stance switch, delayed strike, ...), not one per script; no Lua is admitted and a key without an implementation is rejected by the content compiler (D13 rule). Each new key needs its implementing owner and tests before admission. | §3 pattern tags. |
| S8 | **DECIDED (owner, 2026-09-28).** Vocations are an explicit sorted list of base and promoted vocation keys, as the sources register them; a promoted vocation is never implied. Vocation promotion itself stays Character state (GAME-CHAR-01 decision 6). | Canary/Crystal list both (`"druid;true", "elder druid;true"`); the wiki lists base vocations only and is compared on base vocations. |
| S9 | **DECIDED with change (owner, 2026-09-28).** Cooldowns: one own cooldown plus one or two groups, primary first, each with its group cooldown; group keys are lowercase without spaces (`greatbeams`, `burstsofnature`, `ultimatestrikes`, `stance`, ...). Cooldown groups are a declared closed catalogue: each group key is declared once in content, a spell references only declared groups, and an unknown group fails the content compiler. | Crystal/Canary `spell:group(a, b)` and `groupCooldown(a, b)`; wiki `subclass`/`secondarygroup` and `cooldowngroup`/`cooldowngroup2`. |
| S10 | **DECIDED (owner, 2026-09-28).** Learning: the Spell holds `learning_required`; trainer NPCs and prices belong to NPC services (`acquisition_interactions`), not to the Spell. Learned spells are Character state (GAME-CHAR and DUR-02); a `learning_required` spell is not castable until that owner exists. | v2 `ProjectV2AbilityAuthoring.acquisition_interactions`; NPC schema owns trade/teach services. |
| S11 | **DECIDED (owner, 2026-09-27).** A value on which TibiaWiki BR and Fandom disagree is taken from the latest official news that changed it (tibia.com, read through its tibiopedia.pl mirror); without such a news item the wiki page with the newer revision wins. The losing value stays in the manifest. | §4.2 conflicts; §3 post-15.30 balancing (neither wiki is always the fresher one). |
| S12 | **DECIDED (owner, 2026-09-27).** tibiopedia.pl is a third reference: its official news mirror is the S11 evidence and its spell pages may confirm single facts (base power, cooldowns, level, mana); only facts with their URL are recorded, never page text. | §3; tibiopedia.pl is "all rights reserved". |
| S13 | **DECIDED (owner, 2026-09-28).** Without an official change (S11), a BR/Fandom conflict is decided by tibiopedia.pl when it agrees with one of them (two of three references); only when all three differ does the newer wiki revision decide. tibiopedia.pl never supplies a value neither wiki states. Applied in `convert_spells.py` (`spell-p2-r2`); it changed 9 fields of 6 ready spells (§4.3). | §4.3: in 8 of the 9 fields the majority also matches Canary and Crystal. |
| S14 | **DECIDED (owner, 2026-09-28).** The Canary source is the Tibia 15.30 branch `dudantas/fix-tibia-15-30-regressions` at `99902524` (not yet in Canary `main`): (a) in a BR/Fandom conflict without an official change, each wiki, tibiopedia.pl and the branch back one value, the most votes win and a tie goes to the branch; (b) formulas, effects and areas come from the branch instead of the older Canary pin, with Crystal still an equal source (S4, S5); (c) the single-target range stated by BR and tibiopedia.pl decides like other wiki fields; (d) its new and renamed spells join the census. Where all wikis agree, they decide even against the branch. | §4.4 |
| S15 | **DECIDED (owner, 2026-09-28).** The official tibia.com spell library decides every field it states, ahead of the wikis, S11, S13 and S14; the wikis, tibiopedia.pl and the sources supply what it does not state. Captured by `tibiacom_spells.py fetch` on a machine tibia.com serves (it blocks hosted runners); single facts with the page URL and page SHA-256 only. Applied for the list-view fields (words, group, level, mana, premium) from the owner's copy of 2026-09-28 (§4.4, `spell-p2-r5`); detail-page fields await a capture. | §4.4; tibia.com is the game publisher's reference. |
| S16 | **DECIDED (owner, 2026-09-28).** Since patch 15.22 (27 January 2026) spells unlock automatically and free at their level and trainers no longer teach them, so `learning_required` is false for every spell. A Wheel of Destiny revelation spell carries `requirements.wheel_unlock` (S6): stated by the wiki (Fandom `wheelspell`, BR `wheelSpellType` Revelação; Convicção is a perk on a level-unlocked spell), else by the Canary 15.30 `needLearn`, the only source that implements the 15.22 unlock. The Game core rejects a `wheel_unlock` spell until a Wheel owner exists. S9 is applied as a declared closed catalogue, `cooldown-groups.json`. | https://tibiopedia.pl/updates/15.22.c93366; §4.4 |
| S17 | **DECIDED (owner, 2026-09-28).** The shared `condition` (monster schema) gains `light` (`level` 1–255, `color` 0–255), `regeneration` (`health_gain`/`health_interval_ms` and/or `mana_gain`/`mana_interval_ms`) and `buff_spell`. `light` and `regeneration` are allowed only on their own condition type; none of the three is allowed on a damage schedule. `buff_spell` (Canary `CONDITION_PARAM_BUFF_SPELL`) may mark any fixed-duration condition, as Canary also sets it on attribute conditions. Canary and Crystal agree on all five spells it unblocks: Light, Great Light, Ultimate Light, Recovery and Intense Recovery. The Game core still treats a condition effect as unsupported. | `condition:setParameter(CONDITION_PARAM_LIGHT_*, CONDITION_PARAM_HEALTHGAIN/HEALTHTICKS, CONDITION_PARAM_BUFF_SPELL)` in both sources. |
| S18 | **DECIDED (owner, 2026-09-28).** Presentation is complete and has one naming. Every effect/missile asset key is named by the Canary 15.30 enum for its client id (`canary.appearance:effect/<name>`, as in `content/`), whichever source converted the spell; Canary and Crystal name 47 of the same effect ids differently. `castSound`/`impactSound` become `presentation.cast_cue`/`impact_cue` = `canary.sound:<SoundEffect_t member>`; a constant that `lua_enums.cpp` does not register is nil in Lua, i.e. silence, and is kept as an `approved_omission`. A conjure records the effect `Player:conjureItem` shows on success (`magic_red` for a rune, else its effect argument) as `conjure.effect_asset_binding`. Revision `spell-p2-r4`. | `SoundEffect_t` and the registered constants are identical in both sources (512 members); after that resolution no spell has a sound conflict. |
| S19 | **DECIDED (owner, 2026-09-28).** A spell carries the official library text that the Cyclopedia Magical Archive and the tibia.com spell library show, verbatim, as the optional `spell.library_text`; the owner has settled the rights question for this text. It comes from the Fandom `librarytext` field at the pinned revision (the wiki quotes the official text; BR has no such field), with only wiki markup removed and never cut. Rune use shares the entry of its conjuring spell, found through the rune page's words. Revision `spell-p2-r7`: 231 of 252 bundles carry it; the other 21 have no library text on Fandom (the four house list spells, Blank Rune, six ammunition conjures from Conjure Bolt to Conjure Royal Star, Enchant Staff, the Practise and Lightest Magic Missile training spells and the two Dawnport runes). | Owner in-game check of 2026-09-28: Enlighten Party in the Magical Archive matches the Fandom text word for word. |
| S20 | **DECIDED (owner, 2026-09-28).** The two cast options of patch 15.25 are Spell targeting data. `targeting.aim_at_target`: a direction spell the player may set to turn towards the attacked creature before casting (the client's "Aim at Target"); stated by TibiaWiki BR `aimattarget` (16 spells, all direction spells; Fandom has no such field). `targeting.cast_at_position`: the spell may be cast with a crosshair, at the cursor position or at the target (Canary 15.30 `spell:optionalTarget`: Death Echo, Divine Barrage, Divine Grenade, Ethereal Barrage, Thousand Fist Blows; the wikis describe it in prose for three of them). The schema ties `aim_at_target` to `needs_direction` and forbids `needs_target` with `cast_at_position`. The player setting and the position cast need a cast-wire amendment (P3b contract, protocol-oteryn); until then the Game core rejects a `cast_at_position` spell. | Owner in-game check: the Thousand Fist Blows action-button dialog offers crosshair / cursor position / target. |
| S21 | **DECIDED (owner, 2026-09-28).** Amends S4: a value Canary and Crystal disagree on and that no wiki or tibia.com states follows the Canary 15.30 branch (S14), for fields, vocations, rune blocking, conjured items and plain-combat executions; the Crystal value stays an `approved_omission`. Against the owner's tibia.com list the 15.30 branch has 3 values wrong that Crystal has right, Crystal 30 the other way (Crystal lacks most of the 15.25 vocation adjustment), and the 15.30 branch carries the 15.25 chain mechanics. A source whose script is custom still yields to the other source's plain combat (S4). Divine and Ethereal Barrage become ready. | §4.4 comparison; patch notes 15.25.3a4a52 and 15.25.bd5a04. |
| S22 | **DECIDED (owner, 2026-09-28).** A Wheel of Destiny revelation spell has level 0: the client spell list shows 0 and tibia.com states no level; the Wheel unlock gates it (S6/S16). This applies only where tibia.com states no level (the 11 revelation spells: the five avatars, Divine Empowerment, Divine Grenade, Executioner's Throw, Ice Burst, Terra Burst, Spiritual Outburst); the wikis' level 300 for the avatars is superseded. Where tibia.com states a level, the spell unlocks at it: the Fandom `wheelspell` marking on Mystic Repulse (30) and Flurry of Blows (35) is superseded under S15 (BR states none), so they lose `wheel_unlock`; Focus Harmony and Forceful Uppercut keep the Canary `needLearn` gate with their tibia.com level. | Owner in-game screenshot: Spiritual Outburst level 0, Mystic Repulse level 30. |
| S23 | **DECIDED (owner, 2026-09-28).** A chain spell is a plain `Ability` with a `chain` block: the monster D12 `Ability.chain`, extended with `shape` (`sequential`/`fork`), `initial_range_tiles`, `damage_step_percent` and the `ranged_monsters` filter. `max_targets` counts the further creatures after the first. The parameters are the accepted values in `chain-behaviours.json`; a chain spell without a row stays blocked. The behaviour, the per-spell values and their sources are in `OTERYN_SPELL_CHAIN_BEHAVIOUR_CANDIDATE_V1.md`. The game core rejects a chain until its runtime exists; the creature admission does not accept the new fields yet (no monster uses them). | Owner in session; #162 comment 5876917107. |
| S24 | **DECIDED (owner, 2026-09-28).** Source order for a value: an official tibia.com announcement dated on or before the target date (2026-09-27), then the wiki at the target date, then Canary/Crystal (hypothesis only). A later official source (the tibia.com spell list, S15) supersedes an earlier announcement. A percent change is applied as stated (rounding marked uncertain); a change without a number is recorded as known but not quantified, never guessed. News is read through the TibiaData API (api.tibiadata.com/v4), never by bypassing tibia.com's protection. A spell an announcement removed is blocked as removed (`official-changes.json` field `removed`). | Owner in session. |

## 6. Mapping to WorldProject/v2

`ProjectV2AbilityAuthoring` (`apps/game-server/src/content/project/v2.rs`) already carries part of the
player layer; `ProjectV2AbilityDetails` (`v2/creature.rs`) carries the monster execution part.

| Authoring field | WorldProject/v2 | Status |
|---|---|---|
| `words` | `incantation` | MATCH |
| `requirements.vocations` | `vocations` (sorted, unique) | MATCH after key mapping (S8) |
| `requirements.level`, `premium` | `required_level`, `premium` | MATCH |
| `costs.mana` | `mana_cost` | MATCH |
| `cooldown_ms`, `groups[0]` | `cooldown_ms`, `group`, `group_cooldown_ms` | MATCH for the primary group |
| `groups[1]` | — | GAP: second group and its cooldown |
| `base_power`, `targeting.range_tiles` | `base_power`, `range` | MATCH |
| `requirements.acquisition_interactions` | `acquisition_interactions` | MATCH |
| `costs.soul`, `costs.mana_percent`, `requirements.learning_required` | — | GAP |
| `carrier`, `rune.*`, `execution.conjure` | — | GAP |
| `targeting.*` flags, `pz_locks_caster`, `needs_weapon`, `parameter` | `details.needs_target/needs_direction` only | PARTIAL |
| `player_expression` Formula | — (Formula family has no expression) | GAP |
| `execution.ability` | `details` (kind, range, area, effects, variants, chain) | MATCH through the shared Ability |

GAP rows are added to v2 only when a spell in the current playable slice needs them (P3).

## 7. Boundaries

This candidate does not change WorldProject/v2, the compiler, the runtime ability engine, protocol or
persistence; does not populate `content/abilities/**` or mint native Spell keys; does not admit Lua;
records the owner decisions S1–S22 (§5) but is not their implementation; does not establish Tibia Global parity or asset/licensing rights. The wiki files
keep only allowlisted short infobox values with page and revision ids, never article prose; the one exception is
the official spell library text (`librarytext`), kept whole under S19.

## 8. Validation

From `tools/content-schema/spell-authoring/` with `requirements.txt` installed:

```text
python build_formal_schema.py && git diff --exit-code -- .   # schemas and template regenerate byte-identically
python verify_formal_schema.py                               # 3 valid fixtures, 34 negative cases, level curve check
python spell_census.py self-test && python wiki_spells.py self-test
```

`validate_spell.py` checks structure, exact reference closure, lowercase words, sorted vocations,
distinct groups, carrier/execution consistency, and every reached `player_expression` formula over a
grid of levels 1–2500, magic levels 0–130 and skills 10–130: the formula may use only the inputs its
kind provides, `base_power` needs the Spell's base power, and each bound must be finite, non-negative
and `minimum <= maximum` after truncation. Success means authoring structure only
(`runtime_qualified=false`, `source_coverage_proven=false`).

## 8.1 P2 conversion and readiness

`convert_spells.py` builds one bundle per player spell or rune from both sources (`samples/spell-readiness-p2.json`):
every wiki-stated field follows S3/S11 (BR, Fandom, `official-changes.json`), every other field S4 (engine defaults
for an absent registrar call), formulas S5, executions S1/S2 through the monster `combat_ability` rules. Of 249
spells and runes (monster-only registrations excluded) **141 are ready** (115 instant, 26 runes: valid, every
manifest row resolved) and **108 blocked**:

| blocker | spells |
|---|---:|
| custom script, needs a native behaviour (S7: Wheel, stance, party, house, familiar, summons, world queries, ...) | 78 |
| `needLearn` differs between Canary and Crystal, no wiki value (mostly Wheel/avatar spells) | 15 |
| script does not reduce to plain combats in one source only | 10 |
| party spells whose wiki mana "varies" | 5 |
| Canary and Crystal combats differ (area, effects or parameters) | 5 |
| `isAggressive` differs, no wiki value | 4 |
| light and regeneration conditions (no authoring field yet) | 5 |
| other (secondary group cooldown not stated, conjure arguments, missing vocation, `setFormula`, ...) | 12 |

A combat that has a damage type but no player formula (field and wall runes, curse, envenom) deals no direct
damage in the engine; its effect becomes presentation only and the condition or field item carries the damage.
`samples/starter-bundles/` holds the P3 starter set: Light Healing, Intense Healing, Ice Strike, Energy Strike, Cure
Poison, the Sudden Death and Great Fireball runes and their conjuring spells.

## 9. Implementation plan

Playable-first: every phase ends in something checkable, and runtime work starts with the smallest set
of spells a new character actually uses.

| Phase | Result | Scope | Exit evidence |
|---|---|---|---|
| **P0** (this change) | Schema candidate, census, Fandom comparison, plan | `tools/content-schema/spell-authoring/`, this document | validator + 38 cases green; census and compare regenerate |
| **P1** Reference data | Done for capture and crosswalk (§4.1, §4.2); remaining: owner resolution of the BR ↔ Fandom conflicts (decisions S1–S17 are taken) | `.github/workflows/spell-wiki-capture.yml` (BR answers 403 here), `wiki_spells.py --wiki br` | every player spell has a per-field disposition: MATCH, adopted wiki value, CONFLICT or UNKNOWN |
| **P2** Converter and readiness | Done (§8.1): `convert_spells.py`, 141 ready / 108 blocked, starter bundles | `tools/content-schema/spell-authoring/` | readiness census and starter bundles regenerate; CI validates the starter bundles |
| **P3a** Spell core in the server (unwired) | Done: `apps/game-server/src/spell/` — spoken words and rune lookup (`SpellBook`), the cast checks in the Canary `Spell::playerSpellCheck` order (cooldowns, level, magic level, mana, soul, learning or vocation, premium, target), mana/soul debit, own and group cooldowns on `SemanticTimeMicros`, `player_expression` evaluation with the exact integer level curve, and resolved damage/heal/conjure effects; the candidate bundle reader (`spell/authoring.rs`); the hand-off of a resolved cast to the Ability pipeline as an `EffectPlan` (`spell/plan.rs`), with condition removal and conjure returned beside it | private module, no protocol, persistence or live composition | 12 unit tests on the starter bundles, including the official level table for levels 0–20000 and plans committed through `AbilityEngine` |
| **P3b** First castable slice (live) | A character casts the starter set in the running server | protocol contract (cast/talk command, use-with for runes, own HP/mana/cooldown and effect deltas in `PROTOCOL_OTERYN_V1_REGISTRY.json`), player combat state (health, mana, vocation, magic level, soul, premium) in the runtime slot, heal and player-target commits in the ability owner commit, WorldProject/v2 lowering of Spell definitions into the channel content pin, inventory for runes; the damage distribution world rule | protocol and state contracts accepted with changes ([`OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md`](OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md), SPELL-D1–D6, 2026-09-28; first child is self heal); GAME-ABILITY implementation allocation; `game-gate` green |
| **P4** Bulk declarative spells | All remaining `plain_combat` spells and runes by vocation, conditions (haste, paralyse, magic shield, utito) and areas | content population through the P2 converter; runtime only for missing Effect operations | readiness census: declarative spells admitted |
| **P5** Native behaviours | Shared behaviours by pattern in order of player need: party buffs, find person / levitate / magic rope, familiars, summons, house lists, stances, equipment-dependent spells, delayed strikes; Wheel gating and augments with the Wheel system | one behaviour per pattern with parameter contract (S7) | each pattern: contract + tests; manifest rows move from `unresolved_semantics` to `resolved_native_behavior`. The chain pattern (8 spells) is specified in `OTERYN_SPELL_CHAIN_BEHAVIOUR_CANDIDATE_V1.md` |

Dependencies: P3b needs the protocol and runtime-state contracts accepted, the rune Items in the Item catalogue, the Formula family
extended in v2 and the character state (mana, soul, cooldowns) under the existing session-generation
fenced character writes. Protocol changes for spell cast/cooldown messages go through the
`protocol-oteryn` owning contract, not this document.

## 10. Import and maintenance

**Import path.** Spells reach the game in four layers. Each layer is regenerated from the one before it and never edited by hand.

| Layer | Artifact | Owner | State |
|---|---|---|---|
| 1. Evidence | Pinned Canary/Crystal commits (`SOURCES` in `convert_spells.py`), `spell_census.py` census; wiki, tibiopedia.pl and official-change facts with revision ids | spell authoring | done |
| 2. Authoring bundles | `convert_spells.py` → `spell.json`, `dependencies.json`, `catalog.json`, `manifest.json` per spell; readiness and three-reference reports | spell authoring | done (159 ready, 93 blocked) |
| 3. WorldProject/v2 | Ready bundles lowered into `content/` and the channel content pin (§6 GAP rows added when a P3b/P4 spell needs them) | world content lane | not started |
| 4. Runtime | `apps/game-server/src/spell/` (P3a) wired through the SPELL-D1–D6 contract (P3b) | protocol / ability owners | P3a done, P3b allocated |

Spells enter layer 3 in the §9 order: the P3b starter set, then P4 by vocation, then P5 behaviour patterns. A blocked spell never enters layer 3. Its manifest keeps the blocker until an owner decision or a native behaviour resolves it.

**Maintenance.** The only inputs anyone edits are the source pins, the dated reference captures, `official-changes.json`, `cooldown-groups.json`, and the decision rows in §5. Everything else is regenerated.

- **New Tibia patch or source change:**
  1. Re-pin `SOURCES`.
  2. Regenerate the census.
  3. Bump `REVISION`.
  4. Rerun the conversion, readiness and verification.
- **Wiki or official change:**
  1. Capture a new dated cut.
  2. Rerun the comparisons.
  3. Record an official change in `official-changes.json`.
  4. S11–S15 decide.
- **Quality gate:** `verify_spells.py` must keep `ours_differs_ready` at 0.
- **CI:**
  - The `schema` job checks the schema, the negative cases, the tool self-tests and the starter bundles.
  - The `conversion` job checks out the pinned sources and fails unless the committed census, readiness report, verification report and starter bundles reproduce byte for byte.

**Known gaps.**
- Layer 3 has no owner allocation yet.
- Nothing warns when a wiki page or a source branch changes after its capture; refreshing is manual, at each patch.
- S15 covers the list-view fields only; the detail pages (cooldowns, soul, magic level, vocations) still need an owner-run capture.
