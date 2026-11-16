# Healing parity

Area: healing (spells, potions, runes, monk mechanics, leech, regeneration). Date: 2026-11-15. Format: `README.md` in this directory.

Sources: TW = TibiaWiki revision id; MEAS = owner measurement on Global 2026-11-15 (monk level 757, magic level 54, wheel +16 Damage and Healing); OTS = Crystal or Canary census (hypothesis only); REPO = our content.
Confidence: MEASURED, WIKI, REFERENCE (see README). Audit rows C04, C06, C09, I14, S13a, S13b refer to `docs/architecture/reviews/OTERYN_GAME_CRYSTAL_REAUDIT_2026-10-28.md`; they cover the runtime mechanics (cooldown gate, rune use, food, party), not Global numeric values, so only values are re-derived here.

## A. Spells

| item | dimension | Global value | source | measured? | ours | ours file | status |
|---|---|---|---|---|---|---|---|
| Spirit Mend (exura gran tio) | stats | base power 250, mana 210, level 80, cd 1 s, group 1 s | TW 1206169 (2026-09-14) | WIKI | base 250, mana 210, cd 1000, group 1000 | content player-spells.json `candidate:spell/spirit_mend` | MATCH |
| Spirit Mend | heal range at L757 ML54, wheel +16 | ML54: 676-856 (no virtue/Harmony/Justice), 1093-1278 Sustain; ML52: 654-829, 1059-1241 Sustain (Server Log, n=30-35 per cell, see section F) | MEAS 2026-11-15 | MEASURED | 865-1347 (pre-wheel) | `crates/gameplay/content/src/content/spell_data/formula.rs:17`, formula `candidate:formula/spell/spirit_mend/combat-1` (max lbh+20*ML+125, min lbh+12*ML+75) | MISMATCH (waits for more measurements; constants came from Crystal, base power unused by the formula) |
| Mass Spirit Mend (exura mas nia) | mana, cd | mana 400, cd about 12 s, no refund (not a spender) | MEAS; TW 1191080 (2026-06-23) update says non-spender | MEASURED | mana 400, cd 12000 | player-spells.json `candidate:spell/mass_spirit_mend` | MATCH |
| Mass Spirit Mend | group cd, base power | page pre-update (TW 1182871): group 2 s, base power 90; post-update values not published | TW 1182871 | REFERENCE | group 1000, base power 800 | same | UNKNOWN |
| Divine Healing | stats | base 250, mana 160, level 35, cd 1 | TW 1182661 | WIKI | same | player-spells.json | MATCH |
| Ultimate Healing | stats | base 250, mana 160, level 30, cd 1 | TW 1182931 | WIKI | same | player-spells.json | MATCH |
| Intense Healing | stats | base 120, mana 70, level 20, cd 1 | TW 1182799 | WIKI | same | player-spells.json | MATCH |
| Light Healing | stats | base 40, mana 20, level 8, cd 1 | TW 1182813 | WIKI | same | player-spells.json | MATCH |
| Heal Friend | stats | base 260, mana 120, level 18, cd 1 | TW 1190770 | WIKI | same | player-spells.json | MATCH |
| Heal Friend | 2026 formula rework | reworked by Vocation Adjustments 2026 | TW 1190770 | WIKI | not verified | - | UNKNOWN |
| Mass Healing | stats | base 200, mana 150, level 36, cd 2 s, group 1 s | TW 1182870 | WIKI | same | player-spells.json | MATCH |
| Nature's Embrace | stats | base 2000, mana 400, level 275, cd 60 s | TW 1190771 | WIKI | same | player-spells.json | MATCH |
| Exura-named aliases (san, vita, gran mas res, ico, med ico) | redirects | TW redirects to Divine/Ultimate Healing, Mass Healing, Wound Cleansing, Fair Wound Cleansing | TW | WIKI | alias resolution not checked | - | UNKNOWN |
| Intense Healing Rune (adura gran) | spell stats | mana 120, level 15, cd 2 s, group 2 s | TW 1186611 | WIKI | spell mana 120, level 15, cd 2000 | player-spells.json `spell/intense_healing_rune` | MATCH |
| Intense Healing Rune | rune-use cooldown | unknown; conjure-page cd is not rune-use evidence; to be measured in the Global client | TW 1186611 (conjure spell page) | REFERENCE | rune bundle cd 1000, group healing | rune bundle `rune/intense_healing_rune` | UNKNOWN |
| Ultimate Healing Rune (adura vita) | spell stats | mana 400, level 24, cd 2 s, group 2 s; 2026: not usable on other characters | TW 1186610 | WIKI | mana 400, level 24, cd 2000; targets self or own summons | player-spells.json `spell/ultimate_healing_rune` | MATCH |
| Ultimate Healing Rune | rune-use cooldown | unknown; conjure-page cd is not rune-use evidence; to be measured in the Global client | TW 1186610 (conjure spell page) | REFERENCE | rune bundle cd 1000, group healing | rune bundle `rune/ultimate_healing_rune` | UNKNOWN |
| Rune use, cooldown gate | mechanics | not a value | audit C06, C04 | REFERENCE | DONE in audit | see audit | MATCH (audit) |

## B. Potions

| item | dimension | Global value | source | measured? | ours | ours file | status |
|---|---|---|---|---|---|---|---|
| Great Health (i239) | HP, level, vocation | 425-575, 80, knights | TW 1183256 | WIKI | same | items-12500-12999 | MATCH |
| Ultimate Health (i7643) | HP, level, vocation | 650-850, 130, knights | TW 1183257 | WIKI | same | items-31500-31999 | MATCH |
| Supreme Health (i23375) | HP | 875-1175, level 200, knights | TW 1183258; WIKI, not measured (monk character) | WIKI | 875-1125 | items-12000-12499 | FIXED (15bc1214c) |
| Great Mana (i238) | vocations | 150-250 mana, level 80, all vocations since 2026-06-20 | TW 1190635; WIKI, not measured (monk character) | WIKI | druid, monk, paladin, sorcerer (knight missing) | items-12500-12999 | FIXED (15bc1214c) |
| Ultimate Mana (i23373) | mana, vocations | 425-575, level 130, sorcerer and druid; 2026 change not stated for this potion | TW 1183206 | WIKI | same | items | MATCH (vocation after 2026 UNKNOWN) |
| Great Spirit (i7642) | mana | mana 100-200 (measured 124-160, n=12; Wiki 125-185 contradicted) | [ct-potions-summary.txt](evidence/2026-11-15-global-client/ct-potions-summary.txt) | MEASURED | 100-200 | items-31500-31999 | FIXED (this commit) |
| Ultimate Spirit (i23374) | mana | HP 420-580, mana 150-250, level 130, monk and paladin | TW 1183261; [ct-potions-summary.txt](evidence/2026-11-15-global-client/ct-potions-summary.txt): measurement 150-247, n=30 confirms the fix | MEASURED | HP 420-580, mana 250-350 | items-12000-12499 | FIXED (15bc1214c) |
| Supreme Spirit | all | no TW page | - | REFERENCE | not in repo | - | UNKNOWN |
| Distilled Ultimate Mana (i53164) | all | 2026 distilled variant | TW Vocation Adjustments 2026 | WIKI | present, not analysed | - | UNKNOWN |
| Potion use path | mechanics | not a value | audit I14 (PARTIAL) | REFERENCE | see audit | see audit | UNKNOWN |

## C. Monk mechanics

| item | dimension | Global value | source | measured? | ours | ours file | status |
|---|---|---|---|---|---|---|---|
| Harmony | bonus | 7% per point, doubling per point, 112% at 5 | TW 1136128 | WIKI | not verified | - | UNKNOWN |
| Virtue of Harmony | bonus | +3% (+6% serene); spender refunds 1 Harmony | TW 1197257 | WIKI | not verified | - | UNKNOWN |
| Virtue of Sustain | bonus | +35% monk spell healing, +70% serene, includes Virtue Healing | TW 1197259; MEAS 1091-1314 vs 684-849 (about +60%) | MEASURED | not verified | - | UNKNOWN |
| Virtue of Justice | bonus | +8% fist fighting, +16% serene | TW 1197258 | WIKI | not verified | - | UNKNOWN |
| Serene | effect | virtues doubled while serene | TW 1104593 | WIKI | not verified | - | UNKNOWN |
| Virtue Healing | effect | heals monk and lowest-HP party member on each Harmony gain or spend | TW 1207319 | WIKI | not verified | - | UNKNOWN |
| Spirit Mend | builder or spender | no tag on the wiki | TW 1206169 | WIKI | not verified | - | UNKNOWN |
| Wheel augments | Spirit and Mass Spirit Mend | not checked on TW | - | REFERENCE | "Augmented Mass Spirit Mend" in wheel.json; no Spirit Mend augment | wheel.json | UNKNOWN |

## D. Proficiency, leech, charms, imbuements, prey

| item | dimension | Global value | source | measured? | ours | ours file | status |
|---|---|---|---|---|---|---|---|
| Proficiency spell_augment healing | value | no numeric perk values on TW | TW 1192598 | REFERENCE | 0.06 at level 6 (10 records), applied as BaseHealing | content/proficiencies; `weapon_proficiency_cast.rs:67` | UNKNOWN |
| Proficiency skill_scaled_healing | consumer | unknown | - | REFERENCE | value 10.0 at level 6 (59 records), no runtime consumer found | `proficiency.rs:190`, `proficiency_authoring.py:131` | GAP |
| Life and mana leech perks | value | TW Life Leech 1101811, Mana Leech 1101810 | TW | WIKI | life 0.03 at L2, mana 0.02 at L3 | content/proficiencies | UNKNOWN |
| Vampirism imbuement | life leech | 5 / 10 / 25% | TW 1194631, 1194627, 1194626 | WIKI | not located | - | UNKNOWN |
| Void imbuement | mana leech | 3 / 5 / 8% | TW 1194660, 1194645 | WIKI | not located | - | UNKNOWN |
| Vampiric Embrace charm | life leech | 1.6 / 2.4 / 3.2% | TW 1089899 | WIKI | type only, no number | content/charms | UNKNOWN |
| Void's Call charm | mana leech | 0.8 / 1.2 / 1.6% | TW 1084323 | WIKI | type only, no number | content/charms | UNKNOWN |
| Prey bonuses | value | damage boost 7-25%, reduction 12-30%, loot 13-40%; damage preys do not affect leech | TW 1168706 | WIKI | not checked | - | UNKNOWN |
| Blessings, Twist of Fate | heal/mana | no heal or mana text | TW 884226 | WIKI | not checked | - | UNKNOWN |

## E. Other factors (partial)

| item | dimension | Global value | source | measured? | ours | ours file | status |
|---|---|---|---|---|---|---|---|
| Food regeneration | rate | by vocation, faster when promoted | TW 1141317 | WIKI | mechanic DONE in audit C09; rate not compared | audit C09 | UNKNOWN |
| Hireling food | effect | Carrion Casserole 30% max HP, Consecrated Beef 30% max mana | TW 1058734 | WIKI | not checked | - | UNKNOWN |
| Party spells | effect | Heal Party small heal each turn, Enlighten Party small mana each turn, level 32+ | TW 1189634 | WIKI | `spell/heal_party` native, cost 0, cd 2000; mechanics DONE in audit S13a, S13b; numbers unknown | player-spells.json | UNKNOWN |
| Soul, achievements, outfits, mounts, bless, Exaltation Forge | heal/mana | no heal or mana text found | TW 1108003, 1194693, 1188274, 1202276, 1025146, 1084311, 1015718 | WIKI (weak) | not checked | - | UNKNOWN |
| Familiar, Gem Atelier, healing reduction, exhaustion, PvP healing reduction, store boosts | all | stub, missing or not fetched | TW 820259, 1045949 | REFERENCE | not checked | - | UNKNOWN |

## F. Spirit Mend gap (measured)

- L757: lbh = floor((757+1000)/6) + 300 - 450 = 142 (`formula.rs:17`).
- Ours at ML54: min 142+648+75 = 865, max 142+1080+125 = 1347. MEAS 684-849 with wheel +16.
- Crystal uses the same constants (OTS). Canary gives 585 / 921 without wheel, 679 / 1068 with +16%; neither reproduces MEAS.
- Server Log series 2026-11-15 (Invalid Monk L757, wheel +16 D&H, ML 52 = armor and legs off, ML 54 = on):

  | ML | virtue | n | min | max | mean |
  |---|---|---|---|---|---|
  | 52 | Justice | 30 | 658 | 829 | 728 |
  | 52 | Harmony | 30 | 654 | 829 | 758 |
  | 52 | Sustain | 35 | 1059 | 1241 | 1153 |
  | 54 | Justice | 31 | 678 | 849 | 749 |
  | 54 | Harmony | 31 | 676 | 856 | 777 |
  | 54 | Sustain | 30 | 1093 | 1278 | 1190 |

- +2 ML moves the mean by about +20 (about 10 HP per ML); the spread is about +/-11 % of the mean. Harmony and Justice do not change Spirit Mend; Sustain multiplies it by about 1.54, which matches neither +35 % nor +70 % (serene) and waits for the character's bonus inventory.
- Saying the same virtue twice turns it off; the earlier "Sustain" samples taken that way were no-virtue samples and are discarded.
- The ML span 52-54 is too narrow to fix the coefficients; the formula stays unchanged until a wider ML range is measured.
