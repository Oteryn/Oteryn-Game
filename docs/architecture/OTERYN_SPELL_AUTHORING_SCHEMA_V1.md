# Oteryn Spell Authoring Schema v1 and implementation plan

- Date: 2026-09-27
- Status: CANDIDATE / authoring schema with executable validation and source evidence; S1–S5, S11 and S12
  decided by the owner on 2026-09-27, S13–S15 on 2026-09-28, S6–S10 PROPOSED; no runtime, WorldProject storage or `content/` change
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
└── spell   identity, name, carrier (instant | rune), words, reference_spell_id,
            requirements (vocations, level, premium, learning_required, acquisition_interactions),
            costs (mana | mana_percent, soul), cooldown_ms, groups[1..2] (group, cooldown_ms),
            targeting (aggressive, self_target, needs_target, needs_direction, target_or_direction,
                       range_tiles, block_walls, allow_on_self, check_floor, parameter),
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

Compared: Canary 224 spells, Crystal 239; unmatched source spells are monster-only registrations and a
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

`samples/wiki-spell-crosswalk-fandom-br-2026-09-27.json`: 206 spells joined by words (the Fandom words
may carry the parameter) and 32 runes by name. The wikis agree on almost everything: premium 204 / 1,
vocations 201 / 1, group cooldown 200 / 5, mana 200 / 6, level 196 / 10, cooldown 194 / 10, base power
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

The official tibia.com spell library (S15) answers a Cloudflare browser check from the build container
and is read by the `tibiacom` job of `spell-wiki-capture.yml` on a hosted runner (`tibiacom_spells.py`).

## 5. Decisions

| # | Proposal | Basis |
|---|---|---|
| S1 | **DECIDED (owner, 2026-09-27).** A player spell is a `Spell` definition (casting layer) whose execution is an `Ability`. A spell a monster also casts keeps the one shared Ability (`oteryn:ability.spell.<name>`, D11); only the monster schedule overrides its magnitude. | D11; 7 player spells/runes already exist as shared Abilities in `content/abilities/`. |
| S2 | **DECIDED (owner, 2026-09-27).** Two carriers: `instant` (spoken words) and `rune` (an Item with charges). A conjuring spell is an instant spell whose execution is `conjure` (reagent → result × count). The rune Item stays Item authority (item schema kind 8, rune); the Spell references it. | Canary/Crystal `Spell("instant")` / `Spell("rune")`, `Player:conjureItem`; 49 conjure bodies. |
| S3 | **DECIDED (owner, 2026-09-27).** TibiaWiki BR and Fandom complement the two OTS sources because they carry the most current data; the wiki is read as of the working day, not at a historical cut. Where a wiki states a value (words, vocations, level, magic level, mana, soul, premium, cooldowns and groups, base power, conjure amount, rune item, formulas), it decides; a BR/Fandom conflict stays `CONFLICT` for the owner. Canary/Crystal supply what the wikis do not state (area, effects, conditions, formula shape, targeting flags). | Owner: "wiki ma być na dzień dzisiejszy"; D15/D25 precedent; §4 differences (premium, cooldown, level). |
| S4 | **DECIDED (owner, 2026-09-27).** Canary 47dfd51f and Crystal ff7ede5 are equal sources with no automatic winner: each field is taken from the source that has it, or has it right against the wiki (Crystal has base power and the current level curve, Canary other details). When both have a value and disagree, the wiki decides (S3); when the wikis are silent it stays `CONFLICT` for the owner. The replaced value stays `approved_omission` in the manifest. | Owner: "crystal/canary są równe, jeden ma jedną rzecz lepiej, drugi inną"; same rule as NPC D2. |
| S5 | **DECIDED by S3.** Damage/heal formulas are `player_expression` trees over declared inputs, evaluated in IEEE-754 double in authored order; each bound is truncated toward zero; the draw between bounds is the world damage distribution (a world rule, like D12). The level contribution is one world function `level_base_damage_healing` = the TibiaWiki `Formulae` curve (§3), which Crystal implements exactly; Canary's `calculateFlatDamageHealing` is a defect and is not used. | §3 formula findings; Fandom `Formulae`. |
| S6 | Wheel of Destiny, gem, weapon-proficiency and imbuement augments are not part of a Spell. A Wheel-gated spell is authored with its base behaviour and a `wheel_unlock` native gate; augments use the existing `ProjectV2AugmentBinding`. | 21–22 Wheel-gated bodies; v2 already has augment bindings. |
| S7 | `custom` spells become `native_behavior` keys with data parameters, shared by pattern (party buff, house list, summon familiar, find person, levitate, stance switch, delayed strike, ...), not one per script; no Lua is admitted and a key without an implementation is rejected by the content compiler (D13 rule). | §3 pattern tags. |
| S8 | Vocations are an explicit sorted list of base and promoted vocation keys, as the sources register them; a promoted vocation is never implied. | Canary/Crystal list both (`"druid;true", "elder druid;true"`); the wiki lists base vocations only and is compared on base vocations. |
| S9 | Cooldowns: one own cooldown plus one or two groups, primary first, each with its group cooldown; group keys are lowercase without spaces (`greatbeams`, `burstsofnature`, `ultimatestrikes`, `stance`, ...). | Crystal/Canary `spell:group(a, b)` and `groupCooldown(a, b)`; wiki `subclass`/`secondarygroup` and `cooldowngroup`/`cooldowngroup2`. |
| S10 | Learning: the Spell holds `learning_required`; trainer NPCs and prices belong to NPC services (`acquisition_interactions`), not to the Spell. | v2 `ProjectV2AbilityAuthoring.acquisition_interactions`; NPC schema owns trade/teach services. |
| S11 | **DECIDED (owner, 2026-09-27).** A value on which TibiaWiki BR and Fandom disagree is taken from the latest official news that changed it (tibia.com, read through its tibiopedia.pl mirror); without such a news item the wiki page with the newer revision wins. The losing value stays in the manifest. | §4.2 conflicts; §3 post-15.30 balancing (neither wiki is always the fresher one). |
| S12 | **DECIDED (owner, 2026-09-27).** tibiopedia.pl is a third reference: its official news mirror is the S11 evidence and its spell pages may confirm single facts (base power, cooldowns, level, mana); only facts with their URL are recorded, never page text. | §3; tibiopedia.pl is "all rights reserved". |
| S13 | **DECIDED (owner, 2026-09-28).** Without an official change (S11), a BR/Fandom conflict is decided by tibiopedia.pl when it agrees with one of them (two of three references); only when all three differ does the newer wiki revision decide. tibiopedia.pl never supplies a value neither wiki states. Applied in `convert_spells.py` (`spell-p2-r2`); it changed 9 fields of 6 ready spells (§4.3). | §4.3: in 8 of the 9 fields the majority also matches Canary and Crystal. |
| S14 | **DECIDED (owner, 2026-09-28).** The Canary source is the Tibia 15.30 branch `dudantas/fix-tibia-15-30-regressions` at `99902524` (not yet in Canary `main`): (a) in a BR/Fandom conflict without an official change, each wiki, tibiopedia.pl and the branch back one value, the most votes win and a tie goes to the branch; (b) formulas, effects and areas come from the branch instead of the older Canary pin, with Crystal still an equal source (S4, S5); (c) the single-target range stated by BR and tibiopedia.pl decides like other wiki fields; (d) its new and renamed spells join the census. Where all wikis agree, they decide even against the branch. | §4.4 |
| S15 | **DECIDED (owner, 2026-09-28).** The official tibia.com spell library decides every field it states, ahead of the wikis, S11, S13 and S14; the wikis, tibiopedia.pl and the sources supply what it does not state. Captured on a hosted runner (`tibiacom_spells.py`); single facts with the page URL and page SHA-256 only. | §4.4; tibia.com is the game publisher's reference. |

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
does not decide S1–S10; does not establish Tibia Global parity or asset/licensing rights. The wiki files
keep only allowlisted short infobox values with page and revision ids, never article prose.

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
| **P1** Reference data | Done for capture and crosswalk (§4.1, §4.2); remaining: owner resolution of the BR ↔ Fandom conflicts and decisions S1, S2, S6–S10 | `.github/workflows/spell-wiki-capture.yml` (BR answers 403 here), `wiki_spells.py --wiki br` | every player spell has a per-field disposition: MATCH, adopted wiki value, CONFLICT or UNKNOWN |
| **P2** Converter and readiness | Done (§8.1): `convert_spells.py`, 141 ready / 108 blocked, starter bundles | `tools/content-schema/spell-authoring/` | readiness census and starter bundles regenerate; CI validates the starter bundles |
| **P3a** Spell core in the server (unwired) | Done: `apps/game-server/src/spell/` — spoken words and rune lookup (`SpellBook`), the cast checks in the Canary `Spell::playerSpellCheck` order (cooldowns, level, magic level, mana, soul, learning or vocation, premium, target), mana/soul debit, own and group cooldowns on `SemanticTimeMicros`, `player_expression` evaluation with the exact integer level curve, and resolved damage/heal/conjure effects; the candidate bundle reader (`spell/authoring.rs`); the hand-off of a resolved cast to the Ability pipeline as an `EffectPlan` (`spell/plan.rs`), with condition removal and conjure returned beside it | private module, no protocol, persistence or live composition | 12 unit tests on the starter bundles, including the official level table for levels 0–20000 and plans committed through `AbilityEngine` |
| **P3b** First castable slice (live) | A character casts the starter set in the running server | protocol contract (cast/talk command, use-with for runes, own HP/mana/cooldown and effect deltas in `PROTOCOL_OTERYN_V1_REGISTRY.json`), player combat state (health, mana, vocation, magic level, soul, premium) in the runtime slot, heal and player-target commits in the ability owner commit, WorldProject/v2 lowering of Spell definitions into the channel content pin, inventory for runes; the damage distribution world rule | owner acceptance of the protocol and state contracts (candidate: [`OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md`](OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md), D1–D6; first child is self heal); GAME-ABILITY implementation allocation; `game-gate` green |
| **P4** Bulk declarative spells | All remaining `plain_combat` spells and runes by vocation, conditions (haste, paralyse, magic shield, utito) and areas | content population through the P2 converter; runtime only for missing Effect operations | readiness census: declarative spells admitted |
| **P5** Native behaviours | Shared behaviours by pattern in order of player need: party buffs, find person / levitate / magic rope, familiars, summons, house lists, stances, equipment-dependent spells, delayed strikes; Wheel gating and augments with the Wheel system | one behaviour per pattern with parameter contract (S7) | each pattern: contract + tests; manifest rows move from `unresolved_semantics` to `resolved_native_behavior` |

Dependencies: P3b needs the protocol and runtime-state contracts accepted, the rune Items in the Item catalogue, the Formula family
extended in v2 and the character state (mana, soul, cooldowns) under the existing session-generation
fenced character writes. Protocol changes for spell cast/cooldown messages go through the
`protocol-oteryn` owning contract, not this document.
