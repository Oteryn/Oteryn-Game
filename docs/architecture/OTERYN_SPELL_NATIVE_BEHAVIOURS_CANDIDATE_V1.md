# Oteryn spell native behaviours (candidate v1)

- Date: 2026-09-28
- Status: CANDIDATE / behaviour specifications. The owner accepted the proposed `native_behavior` keys and
  parameters on 2026-09-29 (S27); the open questions stay open for in-game tests. There is no schema, converter,
  runtime or `content/` change. Each key's implementation needs an implementing owner allocated through #162
  (S7, D13); until then the content compiler rejects it.
- Request: owner request of 2026-09-28 (finish the spell work as far as possible without the owner); programme story
  KAN-16; coordination #162.
- Parent: `OTERYN_SPELL_AUTHORING_SCHEMA_V1.md` (S3, S7, S11, S21, S24, S25, S26). The chain pattern is specified
  separately in `OTERYN_SPELL_CHAIN_BEHAVIOUR_CANDIDATE_V1.md` and is not repeated here.
- Sources:
  - Canary `opentibiabr/canary@99902524` and Crystal `zimbadev/crystalserver@ff7ede5` (hypothesis only);
  - TibiaWiki Fandom at the target date, with the revision ids given in each part;
  - the TibiaWiki BR capture of 2026-09-27;
  - the official tibia.com spell list captured 2026-09-28
    (`tools/content-schema/spell-authoring/samples/tibiacom-spell-list-2026-09-28.json`; words, level, mana and premium only);
  - official tibia.com news through the TibiaData API (ids given as N<id>), dated on or before 2026-09-27;
  - the current TibiaData spell library, used as a check only.
- Source rule (S24): an official announcement dated on or before 2026-09-27 wins, then the wiki at that date, then
  Canary/Crystal. A later official source (the tibia.com spell list) supersedes an earlier announcement. A value no
  source states is written as unknown and listed as an open question; it is never guessed.

## 1. Outcome

88 spells are blocked. For nearly all of them the reason is a behaviour the data alone cannot express; six are
removed instead (S24: Expose Weakness, Sap Strength; S25: Practise Fire Wave, Practise Healing, the conjuring spells of
Lightest Missile Rune and Light Stone Shower Rune) and need no behaviour. This
document specifies those behaviours by pattern, one shared behaviour per pattern (S7). Each section gives:
- the spells;
- the engine steps;
- the per-spell values with their sources;
- a proposed authoring shape;
- engine tests;
- open questions.

It has four parts:
- **A.** Wheel of Destiny gates and revelation stages; monk Harmony, virtues and Serene; the monk P4 scripts.
- **B.** World queries, house spells, spells that read a player parameter, item grants and caster restrictions.
- **C.** Familiars, summons and conditions shared with summons, party buffs, stances, and conditional self states.
- **D.** Delayed and repeated strikes, casts at a position, default targets, equipment-dependent spells,
  presentation-only extras, and the remaining single scripts.

Question ids are local to each part (for example, part D Q4 is "D-Q4"). They are the candidates for the owner's
in-game tests.

S26 (this change) authors the monk Harmony role (`harmony_role: builder|spender`, from Canary `monkSpellType`) on
13 spells. The game core rejects them until the Harmony resource of part A has an owner. Before this change, eight
builders were authored as ready without the role and would never have built Harmony.

## Part A. Wheel of Destiny gates and monk Harmony

Date 2026-09-28, target date 2026-09-27. Source order S24: tibia.com news <= 2026-09-27 (via TibiaData) > Fandom / BR at the
target date > Canary 99902524 (15.30 branch) and Crystal (hypothesis; Canary wins a Canary/Crystal conflict, S21). The
current TibiaData library is a check only. Abbreviations: `8944` = news 8944 (Monk Adjustments 2026-09-01), `8833`, `8872`,
`8849`/`15.25.*` = Fandom `Updates/*` pages; `F rN` = Fandom revision N; `BR` = TibiaWiki BR 2026-09-27; `lib` = TibiaData
library. "Q" = open question (section list at the end of each family). No number below is guessed; "unknown" means no
source states it.

### A.1 wheel_of_destiny: gate, revelation stages, augment hooks

- Spells (21 in the family, 20 to author), by role:
  - **A. Revelation spells (11, `wheel_unlock`, level 0 per S22):** Avatar of Steel (knight), Avatar of Light (paladin),
    Avatar of Nature (druid), Avatar of Storm (sorcerer), Avatar of Balance (monk), Executioner's Throw (knight), Divine
    Grenade (paladin), Divine Empowerment (paladin), Ice Burst and Terra Burst (druid, one perk "Twin Bursts"), Spiritual
    Outburst (monk).
  - **B. Level-unlocked spells with Conviction augments (5):** Energy Wave (sorcerer), Strong Ice Wave (druid), Mass Healing
    (druid), Front Sweep (knight), Flurry of Blows (monk). Base behaviour is a plain combat; the augment is Wheel state.
  - **C. Beam Mastery (Revelation passive) targets (2, plus Great Death Beam outside the list):** Energy Beam, Great Energy
    Beam (sorcerer).
  - **D. Wheel dependency gone at the target date (2, plus one removed spell):** Magic Shield, Swift Foot; Sap Strength is
    removed (S24) and is not authored. Details in Behaviour step 9.

#### Behaviour (what the engine implements)
1. **Wheel state is an input, not spell data.** The Wheel owner supplies, per character: `revelation_stage(perk)` in 0..3 and
   `augment_stage(augment)` in 0..2. Stage 1/2/3 of a Revelation perk need 250/500/1000 points in its domain (F r1204680);
   augment stage 1 comes first, stage 2 only after both slices of the augment are filled (F r1206174). Until a Wheel owner
   exists, every role A spell fails closed (S6).
2. **Gate.** A role A spell is castable only if its perk stage >= 1. Stage 0: the cast fails, no mana, soul, cooldown, PZ lock or
   Harmony is spent (Canary: cancel message, "poff" effect, nothing charged). The stage is read once, at cast time; all
   per-stage values of that cast (cooldown, duration, damage, targets) use that stage.
3. **Cooldown by stage.** The own cooldown set at cast time is the stage value from the table below; the 2 s attack/support
   group cooldown is unchanged. Ice Burst and Terra Burst share the secondary group `burstsofnature`; casting either puts
   both on the stage cooldown (Canary sets the group to the own value and lowers it 4 s per stage; wiki silent, Q6).
4. **Avatars (timed self state, all five identical except outfit, mana).** On cast: outfit changes for 15 s; for that time (a)
   incoming damage is reduced by 5/10/15% (Canary: each hit loses `ceil(hit * pct / 100)`), (b) the character's critical hit
   chance is 100%, (c) critical extra damage is +5/10/15 percentage points. The state ends exactly at 15 s. A second cast is
   blocked by the 120/90/60 min cooldown.
5. **Executioner's Throw.** Needs a target and a weapon; throws to the target, then chains (S23 chain block, shape
   sequential) to 2/3/4 further creatures, jump range 3 (Canary; wiki silent). For every creature hit, if its health is
   below the threshold, that hit gets +100/125/150% damage (a per-target test, not per cast). Distance effect is the weapon
   type. Range 5 (BR, Canary; news 8862 restored a lost tile without a number).
6. **Twin Bursts (Ice Burst, Terra Burst).** Self-centred ring (no centre 3x3, Canary `AREA_RING1_BURST3`). Every creature
   whose health is above 60% takes +20/40/60% damage.
7. **Divine Grenade.** Target position rule (S20 `cast_at_position`): at the cursor, at the target's feet, or at the caster's
   feet without a target (15.25.3a4a52). Range 4; a throw into a protection zone from outside one fails (15.25.a360ec, Canary).
   The marker explodes after 3 s: holy damage in a radius-2 area (Canary `AREA_CIRCLE2X2`). Cooldown is charged at cast, not
   at explosion. Stage damage: +0/+16/+32% (reading of "+16% base damage per additional stage", Q1).
8. **Divine Empowerment.** Creates a 3x3 field of holy energy centred on the caster's tile for 5 s (tiles that block or change
   floor get none). While the caster stands on a field tile that he created, on the same floor (13.10.12858), all damage he
   deals is +8/10/12% (Canary adds it as damage-multiplier percentage points, evaluated at most once per second and on cast; F says "as long as you stand in this field", so the proposal is to evaluate per damage event, Q5). Other players' fields give nothing.
9. **Wheel dependency removed (role D).** `8833` replaced the Magic Shield augments by Special Spells, the Swift Foot augments by
   Divine Barrage, and turned Sap Strength into a stance; `15.25.3a4a52` confirms. Author the spells without any Wheel branch:
   Magic Shield has no 1.25x capacity branch (Canary's `upgradeSpellsWOD` line is dead code after the release; F r1182867 still
   shows the old "Enhanced Magic Shield"); Swift Foot has the fixed -30% damage dealt and no stage (Canary hard-codes 70%;
   Crystal's 50%/no-debuff stage branch is superseded); Sap Strength is removed (S24).
10. **Augments (role B, and Chained Penance etc. outside this family).** A level-unlocked spell is authored with its
    non-augmented behaviour. The Wheel owner applies augment stage k through `ProjectV2AugmentBinding` (S6):
    - percent bonuses ("base damage +X%", "base healing +X%") add percentage points to the multiplier sum of that spell's hits
      (Canary `damageMultiplier`), together with other percentage sources;
    - "area" augments replace the area table with the enlarged one;
    - cooldown/mana augments subtract from the spell value after any other cooldown change, never below 0.
11. **Beam Mastery (role C).** With stage s >= 1 the beam is lengthened (Energy Beam 5 -> 7, Great Energy Beam 8 -> 10, Canary
    only), and beam damage of the adjacent squares is 25/40/70% of the central beam's damage (8872; F r1204680 agrees; the
    earlier 40/60/80% of 8833 is superseded). For each creature hit by the central beam, counting at most 3: damage +10/12/14%
    (max 30/36/42%) and the remaining cooldown of all the caster's spells drops 1 s.
12. **Passives that are not spell data (Wheel owner, listed so nobody searches).** Combat Mastery 1% per 14/12/10% missing
    health (8872 over 8833); Blessing of the Grove 5/7.5/10% (target 30-60% health) and 10/15/20% (below 30%), heal spells crit
    (8833); Lord of Destruction fire base power +2/3/4%, energy crit chance +2/3/4%, death crit extra damage +15/22.5/30%
    (8833; F r1204680 says 25.5 for stage 2, a typo); Gift of Life 20/25/30% of max health and max mana (Canary; mana part 8833);
    flat +4/9/20 damage and healing per perk stage (F r1204680).

#### Parameters
Revelation spells (mana and level from lib, all `premium`, group `support`/`attack` 2 s; level 0 by S22):

| Spell | Mana | Cooldown stage 1/2/3 | Stage effects | Sources | Superseded |
|---|---:|---|---|---|---|
| Avatar of Steel | 800 | 120/90/60 min | duration 15 s; reduction 5/10/15%; crit chance 100%; extra crit +5/10/15 | F r1182616, BR, lib; Canary | wikis' level 300 (S22) |
| Avatar of Light | 1500 | same | same | F r1182614, BR, lib | |
| Avatar of Nature | 2200 | same | same | F r1182615, BR, lib | |
| Avatar of Storm | 2200 | same | same | F r1182617, BR, lib | |
| Avatar of Balance | 1200 | same | same | F r1182613, BR, lib | |
| Executioner's Throw | 225 | 18/14/10 s | further targets 2/3/4; +100/125/150% below 30% health; base power 60 | F r1182682, F r1204680, BR, lib; 13.10.12892 | Canary chain 3/4/5 (bounces+1, chain doc §4.1); 13.10.12892 old cooldown 22/18/14, targets 1/2/3 |
| Divine Grenade | 160 | 26/20/14 s | damage +0/16/32%; delay 3 s; base power 190 | F r1190610, F r1204680, BR, lib | Canary 1.3/1.6/2.0 in Lua plus +30/60/100 in engine (both applied) |
| Divine Empowerment | 500 | 32/28/24 s | field 3x3, 5 s; damage +8/10/12% | F r1182659, F r1204680, lib | |
| Ice Burst / Terra Burst | 230 | 22/18/14 s | +20/40/60% above 60% health; base power 115 | F r1182741 / r1195147, F r1204680, BR, lib | |
| Spiritual Outburst | 425 | 24/20/16 s | recast 37.5/50/62.5% (part D section 1.3); chain 7 further, jump 4 (chain doc) | F r1182899, F r1204680, BR ("8 enemies"), lib | |

Augments and Beam Mastery values are in `tools/content-schema/spell-authoring/wheel-augments.json` (official 8833/8872/8944,
Fandom r1206174) and are used unchanged. Areas: Canary `AREA_*` tables (Strong Ice Wave enlarged = `AREA_WAVE7`, Energy Wave I
= `AREA_WAVE7`, Mass Healing radius: F/BR state 4 (Canary: 3 base, 4 augmented, Q9), Flurry of Blows -> `AREA_GREATER_FLURRY_OF_BLOWS`, Front Sweep II = five squares
in a row, 8833). Base powers: Front Sweep 80, Strong Ice Wave 140, Energy Wave 150, Energy Beam 60, Great Energy Beam 155,
Mass Healing 200, Flurry of Blows 55 (F/BR, 8833/8872). Formula shape is S5; Canary's legacy formulas for Divine Grenade, the
Bursts, Executioner's Throw, Energy Beam and Energy Wave are superseded by the wiki base power.

Data check against the evidence file: the avatar rows in `wheel-augments.json` currently cite Canary only (hypothesis), while Fandom
(revisions above) and BR state the same values; the file does not yet cite the wiki sources. Its Combat Mastery 14/12/10, Beam Mastery
25/40/70 and Lord of Destruction values agree with 8872 / 8833.

#### Proposed authoring shape
- Keep `requirements.wheel_unlock` (exists). Add one optional Spell field `wheel_stages`: `{perk, stages: [{stage, cooldown_ms,
  values: {name: number}}]}`, plus one formula function `wheel_stage_value(name)` for `player_expression`. The same table may
  instead be a `ProjectV2AugmentBinding` with `rank_values` 1..3 on the Ability (S6); the owner picks one route.
- New optional Effect field `target_health_bonus`: `{compare: at_or_below | above, threshold_pct, bonus_pct_by_stage: [..]}`
  (Executioner's Throw, Bursts). A per-target modifier is needed; `player_expression` has no per-target input.
- Avatars: no native key. Each is a plain `Ability` with a timed `condition` Effect (`outfit_binding`, `duration_ms` 15000,
  damage reduction, crit chance 100, crit extra damage by stage), as specified in part D section 1.5.
- `native_behavior.key = "owned_field_buff"` (part D section 1.5): `field_item`, `radius_tiles` 1 (3x3), `duration_ms`,
  `bonus_damage_percent_by_stage`, `owner_only`, `evaluate`.
- Executioner's Throw and Spiritual Outburst use the S23 chain; Divine Grenade uses `native_behavior.key = "delayed_strike"`
  (part D section 1.5) with `delay_ms` 3000 (this document adds only its stage table and the PZ rule).
- Role B, C, D spells: plain `Ability`, no native key; the `wheel_of_destiny` tag is dropped. Left over: Energy Wave (stance),
  Flurry of Blows (equipment, Harmony builder), Mass Healing (area heal rule, section A.3), Swift Foot (familiar), Magic Shield
  (a manashield condition: Fandom capacity `7*ML + 7.6*L + max(300, 0.4*L)` rounded up against Canary's `300 + 7.6*L + 7*ML`
  capped at max mana; for the condition owner).

#### Engine tests
1. Stage 0: Executioner's Throw fails; mana, cooldown and group cooldown unchanged.
2. Executioner's Throw stage 2: target A 100% health, B 30%, C 31%, D 10%, all in range 3 of the previous: 4 hits in nearest
   order, B and D get x2.25, A and C x1.0 (B at 30% follows Canary's <= 30, see Q3).
3. Avatar of Steel stage 3: 100 damage becomes 85 during 15 s, 10 becomes 8 (ceil rule, Q4); every hit crits; second cast
   before 60 min is rejected; at exactly 15 s the outfit and the effects end.
4. Divine Empowerment stage 2: bonus 10% only on the caster's own field, same floor, within 5 s; another player's field: 0%.
5. Divine Grenade cast at PZ tile from outside: rejected, nothing charged; valid cast explodes at +3000 ms, cooldown 20 s at
   stage 2 counted from the cast.
6. Ice Burst stage 3 on targets at 61% and 60%: +60% and +0%; Terra Burst is on cooldown right after Ice Burst.
7. Beam Mastery stage 2, central beam hits 4 creatures: damage +36% (3 counted), cooldowns -3 s, adjacent squares at 40%.
8. Front Sweep with augment I and II: +40% and five squares; augment II without stage 1 is impossible (Wheel state).

#### Open questions
- **Q1** Divine Grenade stage damage: wiki "+16% base damage per additional stage" (reading 0/16/32); Canary 1.3/1.6/2.0 plus 30/60/100.
- **Q2** Divine Grenade: is damage rolled at cast (F r1190610) or at explosion (Canary), and which modifiers are snapshotted?
- **Q3** "Less than 30%" and "more than 60%": Canary rounds health to a whole percent and uses <= 30 and > 60.
- **Q4** Avatar: which hits are forced critical (spells, runes, auto attacks, healing); Canary overrides the crit chance skill.
- **Q5** Divine Empowerment: which damage it boosts (Canary: every hit incl. conditions) and the 1 s evaluation lag.
- **Q6** Twin Bursts group cooldown per stage (Canary: 22/18/14 s), and whether casting one starts the other's own cooldown.
- **Q7** Augment percent: on base power or on the final hit, additive with other sources; rounding of 6.5% and 12.5% (Canary rounds up).
- **Q8** Exact tiles of enlarged areas and of lengthened beams (wikis give none; Front Sweep 5 squares is official).
- **Q9** Mass Healing base radius: F/BR text says 4, Canary base 3 and augmented 4.

### A.2 monk_harmony_virtue: Harmony, virtues, Serene

- Spells: Focus Harmony, Focus Serenity, Virtue of Harmony, Virtue of Justice, Virtue of Sustain, Spiritual Outburst (Harmony
  recast), Flurry of Blows (builder) - all monk. Also the three blocked spenders: Tiger Clash, Greater Tiger Clash, Devastating
  Knockout. The eight builders carry `harmony_role: builder` since S26 (authored as ready without the marker they would have
  cast without building Harmony, a silent wrong result): Swift Jab, Double Jab, Flurry of Blows, Greater Flurry of Blows, Chained Penance, Mystic Repulse, Forceful
  Uppercut, Thousand Fist Blows. Sweeping Takedown is a spender too (F r1182918, Canary; still in the library) and carries the
  same role (S26), but it is blocked on its own script (`equipment_dependent`, other), outside this scope.

#### Behaviour
1. **State.** Per monk: `harmony` 0..5 (emptied on death; kept across casts), `serene` flag, `virtue` in {none, harmony, justice,
   sustain}. Virtues are the `standard` stance slot (8833: stances persist across sessions, may be empty); the stance
   behaviour selects them, this section defines what each does.
2. **Harmony multiplier.** With c = charges > 0: `m = 1 + B * 2^(c-1) / 100`, else 1. `B` (percent) =
   `(7 + 0.005*level) * V + A`, `V` = 1 without Virtue of Harmony, 1.5 with it, 2.0 with it while Serene (8944, which states
   the level scaling is included); `A` = Ascetic stage 1/2/3 = +1/2/3 points, added after `V`, not doubled by Serene (F r1204680,
   Canary). Level 200: B = 8 (16 with virtue + Serene); level 700: 10.5 (21).
3. **Spender cast.** (a) All cast checks pass first; a failed cast keeps Harmony (15.01.4b0877, 15.11.dd3523). (b) The damage
   roll of both bounds uses `m` from the current charges; each bound is truncated toward zero after the multiplication.
   (c) After success: Virtue Healing with the spent charges, `harmony := 0`, then with Virtue of Harmony +1 Harmony and its
   Virtue Healing (refund, F r1197257); (d) Canary only: the remaining cooldown of builder spells drops 2 s per spent charge.
4. **Builder cast.** After success `harmony += 1` up to 5; at 5 nothing is gained and no heal happens.
5. **Virtue Healing.** On every Harmony point gained or spent, heal a party target, or the monk when solo (F r1184204). From
   8944: the monk and its lowest-health ally (Q3). Amount (Canary, S5 level function instead of Canary's flat helper): roll
   uniform in `[ceil(2.0*F*k), ceil(2.3*F*k)]`, `k = 1 + 0.05*charges`, minimums 10 and 25, `F = level_base_damage_healing(level)`.
   Virtue of Sustain applies to it (F r1197259, BR; Canary excludes it, superseded).
6. **Serene.** The monk is Serene unless a party member is visible to it (same floor, dx in [-8, 9], dy in [-6, 7], Canary) and 8
   or more creatures are adjacent (8944; Canary before it: 6). Checked once per second (Canary). Not in a party: always Serene.
   Focus Serenity forces Serene for 7 s; the automatic check cannot remove a forced state.
7. **Virtues.** Harmony: step 2. Justice: fist skill +8% (+16% Serene) of the base fist skill (Canary; F r1197258, BR).
   Sustain: healing done by monk spells x1.35 (x1.70 Serene). Only one virtue is active; casting another replaces it.
8. **Party bonuses from a monk's virtue (8944, replaces 8833's 3/6/6/12).** Any virtue active. A party member on the monk's
   screen (same rule as step 6) of vocation: knight takes -4% damage; paladin +8% auto attack damage; sorcerer +8% spell and
   rune damage (incl. their conditions); druid +16% healing by spells and runes. A Serene monk with a virtue gets the bonus
   of every vocation that is present visibly. Guiding Presence (Conviction perk) raises these passive party bonuses by 33%
   (8944, F r1206174; Q5).
9. **Focus spells.** Focus Harmony: fill Harmony to 5 (Virtue Healing for the gained charges). Focus Serenity: Serene 7 s,
   fill Harmony, reset the cooldown of all spender spells.
10. **Sanctuary (Wheel Conviction).** On a spender cast with c > 0: a field for 5 s and damage and healing dealt +2% per spent
    charge (Canary); not renewed while active. It also gives +10% damage to adjacent enemies and +10% healing to adjacent
    allies (15.25.3a4a52). Values unchanged by 8944.
11. **Spiritual Outburst.** Chain per S23 and the chain doc. A spender: it consumes Harmony. If Harmony was 5 at cast, a second
    cast of the same chain runs 1000 ms later at 37.5/50/62.5% by Wheel stage (part D section 1.3). Wiki: "percentage of its original
    damage" (Q4).

#### Parameters
| Item | Value | Sources | Superseded |
|---|---|---|---|
| Harmony base bonus | 7% + 0.005% per level | 8944 | F r1136128: 7%; Canary 8% |
| Bonus per extra charge | doubles (x1, 2, 4, 8, 16) | F r1136128, Canary | |
| Virtue of Harmony | base x1.5, x2.0 Serene | 8944 | F r1197257: +3/+6 points; Canary +4/+8 |
| Ascetic | +1/+2/+3 points, not doubled | F r1204680, Canary | |
| Serene threshold | active with <= 7 creatures adjacent | 8944 | F r1104593 and Canary: 6 or more removes it |
| Virtue party bonuses | -4% / +8% / +8% / +16% | 8944 | 8833 and Canary: -3 / +6 / +6 / +12 |
| Guiding Presence | +33% of party bonuses | 8944, F r1206174 | |
| Justice / Sustain | +8/16% fist; healing x1.35/1.70 | F r1197258/9, BR, Canary | |
| Virtue Healing | base power 10 (15.00.483ebe); depends on magic level (15.01.4b0877); Canary formula in step 5 | F r1184204, updates, Canary | |
| Virtue cast | level 20, mana 210, cooldown 10 s, groups support 2 s and virtue 10 s | F r1197257-9, lib | |
| Focus Harmony | level 275, mana 500, cooldown 120 s, `wheel_unlock` gate kept (S22 note) | F r1182717, BR, lib | |
| Focus Serenity | level 150, mana 500, cooldown 600 s; resets spender cooldowns | F r1182718, Canary | BR: "all cooldowns" (Q7) |
| Spender cooldown / mana | Tiger Clash level 0, mana 18, 8 s; Greater level 18, mana 50, 8 s; Knockout level 125, mana 210 | F r1182925/1182730/1206172, BR, lib | Canary mana 3 (Tiger Clash) |
| Base power | Tiger Clash 15, Greater Tiger Clash 44, Devastating Knockout 62 | F, BR | |
| Devastating Knockout | cooldown 8 s, range 7 (was 24 s, 1) | 8944, F r1206172, BR | Canary 24 s, range 1 |
| Tiger Clash floors | min 5, max 10 before the multiplier | Canary only | |

The formula shape is S5 (`bp * skill/100 * attack/10 + level_base_damage_healing`, bounds x0.9 and x1.1, as Canary), with the
multiplier applied to both bounds. Range 1 for Tiger Clash and Greater Tiger Clash (BR, lib, Canary). The damage type follows the
equipped weapon's elemental bond (Canary, Crystal); that belongs to `equipment_dependent`.

#### Proposed authoring shape
- Formula function `monk_harmony_multiplier()` (no arguments; reads charges, virtue, Serene, level, Ascetic stage). The three
  spenders become plain `Ability` with `min/max = trunc(monk_harmony_multiplier() * bound)`; no native key.
- Spell field `harmony_role`: `builder` | `spender` (decided, S26; on the 8 builders above and the 5 spenders). Not a native key,
  because `execution` is exclusive and these spells keep their `ability`. The game core rejects a spell with a Harmony role
  until the Harmony resource has an owner (as it rejects `chain`).
- `native_behavior.key = "monk_focus"`: `fill_harmony` (bool), `serene_ms` (int or null), `reset_spender_cooldowns` (bool).
  Focus Harmony: `{true, null, false}`; Focus Serenity: `{true, 7000, true}`.
- Virtue spells: the shared stance behaviour (slot `standard`, id) plus one ruleset record `monk_harmony_rules` holding the
  numbers of steps 2, 5, 6, 8 and 10 (not per-spell data). Spiritual Outburst: chain plus `harmony_role: spender` and
  the `delayed_strike` of part D section 1.5 (`delay_ms` 1000, `requires_full_harmony` true, damage 37.5/50/62.5% by stage).

#### Engine tests
1. Multiplier, level 200, no virtue: charges 1..5 give 1.08, 1.16, 1.32, 1.64, 2.28; virtue + Serene: 1.16, 1.32, 1.64, 2.28,
   3.56; virtue without Serene, 5 charges: 2.92; level 700 virtue + Serene, 5 charges: 4.36; Ascetic 3 adds 3 points to B.
2. Tiger Clash at 3 charges: both bounds x1.32 (level 200), then truncated; out of range: charges stay 3.
3. Builder at 5 charges: no gain, no heal. Spender with virtue of Harmony at 5: Harmony ends at 1, two heals.
4. Serene: no party always; party visible with 7 adjacent monsters: Serene; 8: not; forced 7 s by Focus Serenity holds at 8.
5. Party bonuses: visible sorcerer +8% spells; the same sorcerer 10 tiles right: none; Guiding Presence multiplies per Q5.
6. Virtue of Sustain: Spirit Mend x1.35 (x1.70 Serene) and Virtue Healing likewise.
7. Focus Serenity resets spender cooldowns only; builders keep theirs; Harmony 5.
8. Spiritual Outburst at 5 Harmony recasts at +1000 ms with stage %; at 4 Harmony no recast.

#### Open questions
- **Q1** Does Ascetic get scaled by Virtue of Harmony? 8944 says only that the level scaling is included.
- **Q2** Virtue Healing formula: wiki says it depends on magic level, Canary uses level only. Unknown coefficients.
- **Q3** 8944 "monk and its lowest health ally": two heals or one, split or full, absolute health (Canary) or percent (wiki)?
- **Q4** Spiritual Outburst recast: Canary computes it after Harmony is spent (multiplier 1); wiki says "original damage".
- **Q5** Guiding Presence +33%: relative (x1.33) or points; rounding; does it also raise the monk's own Serene copy?
- **Q6** Which creatures count for Serene (Canary: non-summon monsters, 3x3); "party members nearby" radius.
- **Q7** Focus Serenity: BR resets all cooldowns, Fandom and Canary only spenders; does the 2 s group cooldown reset?
- **Q8** Builder cooldown reduction by spenders (Canary only, and with 0 charges Canary clears them completely): real? Do
  builders that hit nothing build Harmony? A spender at 0 Harmony: Canary still heals; the wiki says only spent points heal.
- **Q9** Justice uses base or total fist skill (Canary: base). Does recasting the active virtue switch it off?
- **Q10** Other Harmony base sources (Harmony Amulet, Canary `BUFF_HARMONYBONUS`): value unknown.
- **Q11** PZ lock and aggression of Focus Harmony and Focus Serenity (S25: unknown).

### A.3 monk_p4_scripts: Mass Spirit Mend and Balanced Brawl

Neither reads Harmony at the target date: `8833` says Mass Spirit Mend "is no longer a spender spell"; Balanced Brawl never was one.
Both are blocked by per-target scripts (P4).

#### Behaviour
1. **Mass Spirit Mend.** Area 11x11 rounded, centre on the caster (Canary `AREA_MASS_SPIRIT_MEND`, hypothesis; BR gives range 5 and mentions 4 sqm, Q2). For each creature: players
   and player-owned summons are healed; monsters that are not player-owned are skipped, except an allow-listed boss (Canary
   list of 8 names, Q1). The caster gets a lesser effect "similar in power to a regular Spirit Mend" (8833): the Spirit Mend
   formula (Canary). Others: `level_base_damage_healing + bp/25*ML + bp/4`, bounds x0.9, x1.1, `bp` 800. Paralysis is dispelled.
   Virtue of Sustain and Wheel augment I (+8% healing) apply; augment II is -4 s cooldown (8833, replaces the enlarged area).
2. **Balanced Brawl.** Direction cast, no target, not aggressive. Every non-summon, non-reward-boss monster in the half-circle
   in front of the caster (Canary `AREA_BALANCED_BRAWL`, 8 rows, up to 13 wide) prefers distance 1 for 16 s; a new cast sets
   the remaining time to 16 s. Players, NPCs, summons and reward bosses are unaffected and the cast does not fail (Canary).

#### Parameters
| Spell | Values | Sources | Superseded |
|---|---|---|---|
| Mass Spirit Mend | level 150, mana 400, cooldown 12 s, healing group 1 s, base power 800, range 5 | BR, lib, Canary | F r1182871: mana 250, cooldown 8 s, base power 90 (pre-8833 page); Crystal: heal 5.7-10.43*ML |
| Mass Spirit Mend augments | I +8% healing; II -4 s | 8833, F r1206174 | earlier: II enlarged the area |
| Balanced Brawl | level 175, mana 80, cooldown 10 s, group 2 s, 16 s, half-circle in front | F r1182618, lib, Canary | BR: long range, around the target (S14a: Fandom and Canary agree) |

#### Proposed authoring shape
- Balanced Brawl: `native_behavior.key = "monster_ai_override"`, `mode` `force_melee`: `duration_ms` 16000, `area` (matrix), `skip_summons`,
  `skip_reward_bosses` (part D section 6.3). It is the per-creature effect the chain doc lists for Divine Dazzle and Chivalrous Challenge;
  one key serves all three.
- Mass Spirit Mend: plain `Ability` with an area and two heal effects. Needs an Effect field `applies_to` (`caster` |
  `others`) and `monster_heal_allowlist` (creature refs) on the Ability; the same rule serves Mass Healing. Part D section 6.4 expresses the same split with
  `affects` (`player_side`, `named_creatures`); the owner picks one of the two shapes.

#### Engine tests
1. Mass Spirit Mend: caster gets the Spirit Mend roll, an ally the bp 800 roll, a hostile monster and a wild monster nothing, a
   player's summon the ally roll.
2. Balanced Brawl: three monsters in the cone are melee for exactly 16 s; a summon and a reward boss are not; recast at 10 s
   restores 16 s.

#### Open questions
- **Q1** Boss allow-list for area heals (Canary and Crystal lists differ; Canary misspells "ravenous hunger").
- **Q2** Size of the caster's "lesser effect" beyond "similar to Spirit Mend"; BR says the radius for summons is 4, range 5.
- **Q3** Balanced Brawl: BR mentions lever bosses; Canary only ignores reward bosses. Area shape (BR long range vs Fandom).


## Part B. World queries, house spells, player parameters, item grants and caster restrictions

Keys: `N<id>` official news (TibiaData); `F<rev>` Fandom revision; `BR<rev>` TibiaWiki BR; `Ca` Canary 99902524 (`data/scripts/spells|runes`, `src/`); `Cr` Crystal ff7ede5;
`TD` current TibiaData library (check only). Rule: official > Fandom/BR > Canary (S24); Canary decides when nothing else states it (S21).
Chain mechanics (first creature, jump range, damage step) are in `OTERYN_SPELL_CHAIN_BEHAVIOUR_CANDIDATE_V1.md` §3 and are not repeated. Wheel state (stages, augments) belongs to
the Wheel owner; only the values these behaviours read are listed. Base fields (words, mana, level, cooldown, group) agree between F, BR, TD and Ca unless a row says otherwise.

### B.1 world_query: reads of tiles, items and monster types by five spell groups

- Spells (rune carrier = the item use; the conjuring instants of the same names are plain `conjure` and are not blocked):
  1a Chivalrous Challenge (knight, elite knight), Divine Dazzle (paladin, royal paladin); 1b Magic Rope (druid, sorcerer, knight, paladin, monk, promotions);
  1c Find Fiend (same five); 1d Divine Empowerment, Divine Grenade (paladin, royal paladin; Wheel revelation, `wheel_unlock`);
  1e Animate Dead Rune (druid, sorcerer), Chameleon Rune (druid), Desintegrate Rune (official "Disintegrate Rune"; druid, monk, paladin, sorcerer), Destroy Field Rune (same four).
  Conjurer vocations per TD (Cr omits monk, older). House spells and House Kick are in §2 and §3.

#### Behaviour

1a. Ranged-monster support chain (Ca `chivalrous_challenge.lua`, `divine_dazzle.lua`, `Monster::changeTargetDistance`).
1. Boss gate: if a reward-boss monster stands within 11 tiles in x and y of the caster on its floor (Ca default spectator box `MAP_MAX_VIEW_PORT` 11 x 11), the cast fails: cancel
   "You can't use this spell if there's a boss.", poof on the caster, no mana, no cooldown. F: "cannot be used in rooms of Lever Bosses" (room, not box: QW1).
2. Chain from the caster (no cast target), radius = jump range = 7, sequential (chain document). A creature is valid only if it is a monster, has no master (not a summon or convinced),
   is not a reward boss, and its type's preferred target distance is greater than 1. F/BR "prioritizing ranged creatures"; F history: melee monsters were removed in 2020.
3. No valid creature: cancel "There are no ranged monsters.", poof, cost and cooldown not spent.
4. Per creature hit: (Chivalrous only) force its target to the caster for `challenge_ms` (no effect on a summon); (both) set its target distance to 1 for `melee_lock_ms` (no effect on a
   summon or reward boss). The client shows the "turned melee" icon while the locked distance is below the type's (N8915: Exposed Weakness/Sapped Strength icons show on such a creature).
   A new cast overwrites the duration. Ca passes 12 s to the challenge; F and BR say 6 s (F/BR override Ca).

1b. Magic Rope (Ca `magic_rope.lua`, `Tile:isRopeSpot`, `Position:moveUpstairs`).
1. Poof at the caster. The caster's tile must be a rope spot (ground item in the rope-spot ground set, or any top item in the special set); else cancel "not possible", no cost, no cooldown.
2. Destination on the floor above (z-1), candidates in order: south of the caster's x/y, north, east, west, south-west, south-east, north-west, north-east (Ca's loop tests west twice;
   otherwise this order). Walkable = has ground, no blocking-solid or blocking-projectile property, no immovable blocking item, no non-movable non-field item that blocks solid or
   projectile; creatures, floor-change and PZ do not disqualify. First walkable wins; if none, the south tile if it exists.
3. Tile missing: cancel "not enough room". Else teleport the caster and show the teleport effect there. F: "Teleports you up through a hole when you are standing in a Rope Spot".

1c. Find Fiend (Ca `find_fiend.lua`, `ForgeMonster:pickClosestFiendish`). Needs the Forge fiendish-monster registry.
1. Target = the fiendish monster with the smallest `max(|dx|,|dy|,|dz|)` to the caster, over all floors and the world; tie: unspecified in Ca, proposed lowest creature id (QW7).
2. None: cancel, poof, no cost, no cooldown. Text F "At the moment there is no fiend with special loot roaming this world." (F overrides Ca "No creatures around").
3. Location phrase: `locate_message` (§3 P7) with dx, dy, dz = caster minus target (F: "in a similar way to Find Person").
4. Difficulty word: Ca maps the kills needed to complete the target's Bestiary entry: 5-25 Harmless, <=250 Trivial, <=500 Easy, <=1000 Medium, <=2500 Hard, <=5000 Challenging, above:
   "Unknown" (below 5 gives Trivial); shown only if the caster's entry is unlocked, else "Unknown". F says "completed" (F overrides, QW6). Prefer the monster's `bestiary.difficulty`.
5. Message (MESSAGE_LOOK, Ca wording): `The monster <location>. Be prepared to find a creature of difficulty level "<Difficulty>".`; if the target reverts in under 15 minutes (floored) append
   `This monster will stay fiendish for less than <m> minutes and <s> seconds.` (F/BR: 15-minute rule, revert after 1 h since 2021). Blue magic on the caster.

1d. Divine Empowerment, Divine Grenade (Ca `divine_empowerment.lua`, `divine_grenade.lua`, `player_wheel.cpp`).
1. Both fail with "You cannot cast this spell" and poof when the caster's Revelation stage is 0.
2. Empowerment: on each tile of the 3x3 around the caster that exists and has no immovable-blocking-solid or floor-change flag, create a holy field item (id 40450) owned by the caster;
   after 5 s remove one such item from each tile of that 3x3 (any owner's; F "lasts 5 seconds"). While the caster stands on a tile with a field it owns, its outgoing damage gets
   `+stage%` in the additive percent term (Ca `combat.cpp`). Ca refreshes this once a second and only while in a fight; F: "as long as you stand in this field", so evaluate per damage event (QW8).
3. Grenade position: an explicit position (crosshair or cursor) if the client sent one; else the attacked creature's tile if alive and within range 4; else the caster's tile (F). Any chosen
   position must be on the caster's floor (N8856 fixed casts to lower floors) and within range 4 with clear line of sight (N8892 fixed casts with no range limit). Ca fails when the
   attacked creature is out of range, F falls back to the caster's tile (F overrides, QW9). Ca also refuses a PZ tile from a non-PZ caster.
4. Mark the tile; 3 s later, if the caster is online and the tile exists, run the damage as a normal aggressive area ability on the 21-tile disc (5x5 without corners) centred there.
   F: the raw damage is fixed at cast from magic level and flat damage; critical, extra critical, Empowerment, elemental pierce, prey and bounty talisman apply at the explosion. Ca recomputes all at the explosion.
5. After a successful cast the cooldown is by stage 26/20/14 s (Ca replaces the spell cooldown; group cooldown 2 s still applies).

1e. Rune tile queries (Ca `runes/*.lua`, `RuneSpell::executeUse`, `Spell::playerRuneSpellCheck`).
1. Common: same floor, range and line of sight when the rune has a range, blocking rules (parameters), then the script. A charge is used only when the script succeeds.
2. Destroy Field: fails if the caster stands in a PZ. The tile's first magic-field item must be a listed fire, poison or energy field: remove it, poof on the tile, success; else cancel "not possible", failure.
3. Disintegrate: from range 1 only (`allowFarUse` false). Walk the target tile's items (at most 500 walked) and remove each that is movable, has no scripted unique id, no action id, and is not a human
   corpse (Ca ids 4240-4243, 4246-4248). Then Ca always cancels "not possible", shows poof and returns success (a charge is used even if nothing was removed). F: "up to 500 movable items",
   monster bodies included, human bodies excluded, not an aggressive action, works in a PZ (overrides Ca's aggressive default).
4. Animate Dead: the target tile's top item must be a movable corpse; the caster needs fewer than 2 summons and no black skull, else cancel "You cannot control more creatures." Create a Skeleton
   at the tile as the caster's summon, remove the corpse, blue magic, success. No corpse: poof, "not possible", failure. F adds: works in a PZ; not on non-decaying or special corpses; not on the first
   decay stage of human corpses; not within 10 s of a monster's death (text unclear, QW10).
5. Chameleon: target = the top item of a map tile, an item in an open container, or an equipped item; it must be movable, else cancel "not possible", failure. The caster gets the outfit
   "that item" for 200 s, red magic on the caster. F: not on creatures, NPCs, players or immovable objects.

#### Parameters

| Spell | Parameter | Value | Sources / superseded |
|---|---|---|---|
| Chivalrous Challenge | creatures hit / jump range / first range | 4 / 7 / 7 | F1190583 "up to 4", "jump range to 7, 3 to 4"; BR440347 "até 4", range 7; N8833 "+1 target, range 7"; N8875 range fix. Ca 6 further (7) and Cr 6 superseded |
| Chivalrous Challenge | `melee_lock_ms` / `challenge_ms` | 12000 / 6000 | F1190583, BR440347 (12 s, 6 s). Ca challenge 12000 superseded |
| Divine Dazzle | creatures hit / jump range / `melee_lock_ms` | 3 / 7 / 8000 | F1190578 "up to 3", "6 to 7", "8 seconds"; BR436981; N8833 (range 7; augments I +2 targets, II -8 s cooldown). Ca 2 further = 3 agrees; Cr 3 further and 12000 superseded |
| Both | boss-gate box | 11 x 11, one floor | Ca default spectator box; F "rooms of Lever Bosses" (QW1); no Wheel augment for Chivalrous since 15.25 (N8833) |
| Magic Rope | premium | no | F1182865 (free since 14.00 and level unlock N8675), BR435423, TD; Ca `isPremium(true)` superseded |
| Magic Rope | rope-spot ground ids / top ids | 386 421 7762 12202 12936 14238 17238 23363 21965-21968 / 12935 | Ca `data/global.lua`; item data should carry a `rope_spot` flag |
| Find Fiend | warning window / difficulty bands (kills) | 15 min / 25, 250, 500, 1000, 2500, 5000 | F1183247, BR435408 (window); Ca (bands, Bestiary data) |
| Divine Empowerment | field / duration / stage 1-2-3 damage % / cooldown s | 3x3 / 5000 ms / 8, 10, 12 / 32, 28, 24 | F1182659, `wheel-augments.json`; Ca 8/10/12 and -4 s per stage from 2. BR expLvl 1, TD level 0 (perk); Ca level 300 |
| Divine Grenade | stage 1-2-3 cooldown s / range / delay / area | 26, 20, 14 / 4 / 3000 ms / 21-tile disc | F1190610, BR436982 (26), Ca |
| Divine Grenade | stage 1-2-3 base-damage bonus | 0, +16, +32 % ("per additional stage" reading) | `wheel-augments.json` (F Wheel page); Ca 1.3/1.6/2.0 (Lua) and 30/60/100 % (`game.cpp`) superseded (QW13) |
| Destroy Field Rune | range / charges / magic level | 5 / 3 / 3 | Ca; F1189762 "any distance as long there's nothing blocking" (QW11); Cr field list has 2131 instead of 21465 |
| Disintegrate Rune | range / charges / item limit | 1 / 3 / 500 | Ca; F1189804 |
| Animate Dead Rune | summon / cap | Skeleton / 2 | Ca; F1189758; BR435680 |
| Animate Dead, Chameleon Rune | blocking solid / creature | true / false | Ca `isBlocking(true)`; Chameleon is self-target: a container or slot target skips the tile checks |
| Chameleon Rune | duration | 200000 ms | F1189759, Ca |

#### Proposed authoring shape

- 1a: `Ability.chain` with `target_filter: ranged_monsters` (chain document §5) plus one `native_behavior` `monster_ai_override` (part D section 6.3; `force_melee` for `melee_lock_ms`, `target_caster` for `challenge_ms`, absent = none),
  with `boss_gate {x_tiles: 11, y_tiles: 11}` and `no_target_message` as additions. The chain step cannot express the target-distance override or the gate.
- 1b: `native_behavior` `vertical_move`, `mode: "rope_up"`, `destination_order`, `walkable_rule` (shared with Levitate, §3). 1c: `locate_message`, `source: "nearest_fiendish"` (shared with Find Person, §3).
- 1d: Empowerment: `native_behavior` `owned_field_buff {field_item, radius_tiles: 1, duration_ms, bonus_damage_percent_by_stage: [8,10,12]}`. Grenade: the delayed area is the `delayed_strike` behaviour (part D section 1.5);
  this family adds `position_mode ["explicit","attacked_in_range","caster"]`, `snapshot_damage_at_cast: true`, `stage_cooldown_s`.
- 1e: one `native_behavior` `tile_item_operation` (part D section 2.3): `operation` in `remove_field | disintegrate | raise_corpse | mimic_item`; parameters `field_items[]`, `max_items`,
  `protected_item_rule`, `summon_creature`, `max_summons`, `duration_ms`, `allow_in_pz`. `remove_field` may instead use the Effect `remove_items` (`first_listed_per_tile`) plus a PZ precondition.

#### Engine tests

1. Chivalrous with a reward boss 10 tiles away fails with mana and cooldown unchanged; with the boss 12 tiles away it proceeds.
2. Dazzle among 2 archers, 1 melee monster and 1 archer-type summon: only the 2 archers are hit, each melee for 8 s.
3. Chivalrous on a hostile archer: target is the caster for 6 s and the monster stays melee for 12 s.
4. Magic Rope on a rope spot lands on the south tile above; with that tile blocked it lands on the north tile.
5. Find Fiend with fiendish monsters at (dx,dy,dz) = (3,2,0) and (10,0,2) picks the first.
6. Grenade with an attacked target 5 tiles away lands on the caster's tile; an explicit position 5 tiles away fails with no cost.
7. Disintegrate on a tile holding a movable item, a human corpse and an item with an action id removes only the first, and uses a charge (also on an empty tile).
8. Destroy Field in a PZ fails and keeps the charge; on a fire field it removes the field and uses the charge.

#### Open questions

- QW1. Boss gate: the room (F) or an 11 x 11 box (Ca)? In game: cast beside a Lever Boss room wall.
- QW2. Creature totals (Dazzle 3, Chivalrous 4): N8833 gives only "+1" and range 7; totals are F/BR. Test with 5 archers.
- QW3. Melee-lock and challenge durations under Wheel augments (Ca adds seconds; N8833 lists none for Chivalrous).
- QW4. Magic Rope destination order when the south tile above is blocked (Ca loop quirk); which Oteryn items are rope spots.
- QW5. Find Fiend exact wording and the revert sentence (Ca only; F gives only the no-fiend line).
- QW6. Find Fiend difficulty: unlocked (Ca) or completed (F) Bestiary entry; fewer than 5 kills.
- QW7. Tie between two equally distant fiendish monsters.
- QW8. Empowerment: instant or once-per-second bonus; additive or multiplicative with other damage buffs.
- QW9. Grenade with an attacked target beyond 4: fall back to the caster's tile (F) or fail (Ca).
- QW10. Animate Dead within 10 s of a kill: refused, or corpse destroyed without a skeleton (F text unclear).
- QW11. Destroy Field: range 5 (Ca) or the screen (F); allowed in a PZ (Ca refuses)?
- QW12. Disintegrate: does an empty tile use a charge and show poof (Ca)? Are ids 4244 and 4245 human corpses too?
- QW13. Grenade stage damage 0 / +16 / +32 % (F reading) against Ca's two mechanisms.
- QW14. Which Oteryn item flag replaces Ca's "unique id above 65535 / action id 0" protection in Disintegrate?

### B.2 house: four house-window and kick spells

- Spells: House Door List (`aleta grav`), House Guest List (`aleta sio`), House Subowner List (`aleta som`), House Kick (`alana sio`); all vocations (druid, sorcerer, knight, paladin, monk,
  promotions), level 8, mana 0, cooldown 2 s, group support 2 s. No Fandom, BR, TD or news source lists them; Ca and Cr are identical, so Ca decides (S21).

#### Behaviour

1. Access level of player P in house H: owner if P owns H (Ca can make the owning account count, QH1) or has the edit-houses flag; else subowner if in the subowner list; else guest if in the
   guest list; else not invited (owner > subowner > guest). P may edit list L: owner any list; subowner only the guest list; others none. A door list is edited by the owner only.
2. Guest List / Subowner List: H = the house of the caster's tile. No house: fails silently, no cost. If the caster may edit that list, open the list editor and record the edit session
   (house, list); success. Otherwise cancel "not possible", poof; Ca still returns success (the 2 s cooldown is spent).
3. Door List: H = house of the caster's tile. Door = the door of H on the tile in front of the caster (facing direction), else the door of H on the caster's own tile. No door, or the caster may
   not edit that door's list: cancel "not possible", poof, failure. Else open the door list editor and record the session.
4. Saving the editor text (client message): accepted only for the recorded session and window id, and only if the player still may edit the list. Saving the guest or subowner list teleports every
   player inside H who is no longer invited to the house entry position.
5. House Kick (name: §3 P4). If the parameter names the caster: the caster must stand on a house tile; poof at the old tile, teleport to that house's entry position, teleport effect there,
   success; no access check. Otherwise: the caster's tile must be a house tile and the caster may edit the guest list of that house; the target must stand on a house tile of its own house H2
   (Ca does not require H2 to be the caster's house, QH2); the caster's access level in H2 must be at least the target's, and the target must not have the edit-houses flag. Then teleport the
   target to H2's entry position, poof at the old tile, teleport effect at the entry (a failed teleport still counts as success). Any failure: cancel "not possible", poof on the caster, failure.

#### Parameters

| Spell | Parameter | Value | Sources |
|---|---|---|---|
| Guest / Subowner List | list id | guest (0x100) / subowner | Ca `map_definitions.hpp` |
| Door List | door lookup / flag `needCasterTargetOrDirection` | front tile, then own tile / true (variant unused) | Ca (QH3) |
| House Kick | parameter kind / name length / destination | player name / 1-29 / house entry position (map data; `getExitPosition` returns the same) | Ca |

#### Proposed authoring shape

One `native_behavior` `house_access`: `action` `edit_list | kick`; `edit_list`: `list` `guest | subowner | door`, `door_lookup ["front","own_tile"]`; `kick`: `parameter "player_name"`,
`self_kick true`, `require_same_house false` (Ca, QH2), `destination "house_entry"`. The house system supplies `access_level`, `can_edit_list`, `entry_position` and the editor session.

#### Engine tests

1. Owner on a house tile opens the guest list; a guest on the same tile is refused with "not possible" and the 2 s cooldown starts.
2. Subowner opens the guest list (allowed) and the subowner list (refused).
3. Door List facing a door of the caster's house opens that door's list; a subowner is refused; standing on a door tile with none in front opens the own-tile door.
4. Owner kicks a guest to the entry position; a subowner kicking the owner fails. A guest's self kick teleports out; a self kick outside any house fails.
5. Saving a guest list that omits a player standing inside teleports that player to the entry position.

#### Open questions

- QH1. House ownership by account or by character in Oteryn (Ca config `HOUSE_OWNED_BY_ACCOUNT`; Platform owns identity).
- QH2. Cross-house kick: Ca lets an owner in house A kick a player in house B when the caster's level in B is not below the target's. Test: kick a stranger standing in another house.
- QH3. Door List with an attacked creature out of reach: Ca's flag makes the cast fail with "creature not reachable".
- QH4. No source lists these spells; a test can confirm words and level 8.

### B.3 player_parameter: spells that read a typed word after the incantation

- Spells: Creature Illusion (druid, sorcerer), Find Person (five vocations), House Kick (§2), Levitate (five vocations), Summon Creature (druid, sorcerer); promotions included.

#### Behaviour

- P1 Matching (Ca `getInstantSpell`): case-insensitive; the spell whose words are the longest prefix of the text (`utevo res ina` beats `utevo res`). A spell without a parameter must match exactly;
  one with a parameter needs a space and at least one more character after its words, else the text is chat.
- P2 Parameter (Ca `playerSaySpell`): text after the words. If a quote follows the leading space, the parameter is the text between the first two quotes (an unclosed quote runs to the end;
  anything but spaces after the closing quote makes it chat). Without quotes the remainder must be one word (two words make it chat). Empty remainder: empty parameter.
  On success the spoken text shows `words "parameter"`, a player name replaced by the resolved full name.
- P3 Order: the common checks (level, vocation, mana, cooldown, group) first, then the parameter checks. Mana and soul are spent and cooldowns started only on script success, except in P4.
- P4 Player name (Find Person, House Kick, Nature's Embrace): length 1-29, else "player with this name is not online". Trailing `~` = case-insensitive prefix needing exactly one online match
  (ambiguity is an error); otherwise an exact online name. A staff-access player is offline to a caster without access. On error: cancel with that message, poof, the cooldown starts, no mana or
  soul is spent (F Find Person history: since March 2015 a missing player exhausts the caster).
- P5 Find Person on a no-PvP world (Ca `canExiva`): the target's Exiva restrictions decide; refusal "The character you are trying to find with Exiva is currently protected from your spell." (QP1).
- P6 Creature Illusion: parameter = monster name (exact, case-insensitive). Unknown: cancel "creature does not exist"; not `flags.illusionable` and no all-illusion flag: "not possible"; poof, no
  cost. Else replace any outfit condition of the caster with the monster's outfit for 180 s (F "three minutes"), red magic, spend the cost.
- P7 `locate_message` (Find Person, Find Fiend): dx, dy, dz = caster minus target, d = max(|dx|,|dy|); dz > 0: target higher, dz < 0: lower, else same. Bands: d < 5 beside, <= 100 close, <= 250 far,
  else very far (F/TibiaMaps: 5-100, 101-250, >= 251; Ca far below 275, F overrides, QP2). Direction when d >= 5: t = dy/dx (10 if dx = 0); |t| < 0.4142: west if dx > 0 else east; |t| < 2.4142:
  t > 0: north-west if dy > 0 else south-east, t <= 0: south-west if dx > 0 else north-east; else north if dy > 0 else south. Text (F lists it): beside: "is below you" / "is standing next to you" /
  "is above you"; close: "is on a lower level to the <dir>" / "is to the <dir>" / "is on a higher level to the <dir>"; far: "is far to the <dir>"; very far: "is very far to the <dir>".
  Find Person sends `<Name> <text>.` as a look message, blue magic on the caster.
- P8 Levitate: the lower-cased parameter must be `up` or `down`, else "not possible", no cost. `up` is refused on z = 8 and `down` on z = 7 (no crossing of the surface/underground boundary).
  Up: the tile above the caster has no ground and no immovable blocking item; destination = the tile in front of the caster one floor up. Down: the tile in front has no ground and no blocking-solid
  item; destination = the tile in front one floor down. The destination needs ground, no immovable blocking item, no floor-change flag. Move ignoring blocking items and creatures but under
  normal entry rules (house access); teleport effect at the destination. Any failure: "not possible", poof, no cost (F: no mana lost off a valid square; one level at a time).
- P9 Summon Creature: monster by name (case-insensitive) else "not possible". Unless the caster has the summon-all flag: the type must be summonable and the caster must have fewer than 2
  summons ("You cannot summon more creatures."). Cost = the type's `summoning.mana_cost` (BR "var."); not enough mana: "not enough mana". Create the monster as the caster's summon on the
  caster's tile or the nearest free one, else "not enough room". Then spend the cost (it trains magic level), blue magic on the caster, teleport effect on the summon.

#### Parameters

| Spell | Parameter | Value | Sources |
|---|---|---|---|
| Creature Illusion | mana / level / duration | 100 / 23 / 180 s | F1182642, BR435398, TD, Ca |
| Find Person | mana / level / distance bands | 20 / 8 / 5, 101, 251 | F1182699 (TibiaMaps), BR435409; Ca band 275 superseded |
| Levitate | mana / level / premium / boundary floors | 50 / 12 / no / 7 and 8 | F1182811, BR435420, TD (premium: F free since 14.00; Ca `isPremium(true)` superseded); floors Ca only |
| Summon Creature | level / mana / summon cap | 25 / per monster / 2 | F1182911, BR435440, Ca |

#### Proposed authoring shape

Extend `targeting.parameter` (now `none | text | player_name`) with `creature_name` and `choice` + `choices` (`["up","down"]`). Shared behaviours: `locate_message` (`source`
`named_player | nearest_fiendish`, `bands_tiles [5,101,251]`, `direction_tangents [0.4142,2.4142]`), `vertical_move` (`mode` `rope_up | levitate`, `blocked_floor_pairs [[7,8]]`),
`creature_appearance` (`duration_ms 180000`, source = parameter, `require_flag illusionable`), `summon_named_creature` (`max_summons 2`, cost from the monster). Effect `appearance_transform`
needs a parameter-bound creature or item instead of its fixed `creature`/`item`.

#### Engine tests

1. `utevo res ina "Rat"` selects Creature Illusion, `utevo res rat` Summon Creature, `utevo res` alone nothing.
2. `exiva "Foo Bar" x` is chat; `exiva Foo~` with two online Foo names fails with an error, starts the cooldown and spends no mana.
3. Find Person target at dx = 10, dy = -10, dz = 0: "is to the south-west"; dx = 0, dy = 300: "is very far to the north".
4. Levitate `up` at z = 8 fails with no cost; `up` at z = 9 with open air above and a ground tile in front one floor up succeeds.
5. Creature Illusion on a non-illusionable monster fails with no cost; on an illusionable one replaces the outfit for 180 s.
6. Summon Creature with 2 summons fails; with 1 summon and mana below the type's cost fails with no cost.

#### Open questions

- QP1. Does Oteryn run Optional PvP and need the Exiva restriction dialog (F: Nov 2017; Ca applies it only on no-PvP worlds)?
- QP2. Find Person: does a target 251-274 tiles away read "far" or "very far" (F vs Ca)?
- QP3. Levitate onto a tile occupied by a creature: Ca ignores creature blocking.
- QP4. Does Summon Creature start the fight/PZ lock (Ca `aggressive` default true)? Do familiars count toward the cap of 2?
- QP5. Wildcard ambiguity message and empty-name behaviour.

### B.4 item_grant: Food

- Spells: Food (druid, elder druid); level 14, mana 120, soul 1, cooldown 2 s, group 2 s (F1182720, BR434673, TD, Ca).

#### Behaviour

1. Common checks first (level, vocation, mana, soul, cooldown). The script then always succeeds.
2. Give the caster 1 item; with probability 50 % give a second. Each item is drawn independently and uniformly from meat (3577), ham (3582), grape (3592), apple (3585), bread (3600), roll (3601),
   cheese (3607) (ids Ca; the item data owner confirms). F: "one or two food items among grape, bread, roll, cheese, apple, meat or ham".
3. Each item is added like a pickup: into inventory or open containers where it fits, else dropped on the caster's tile (Ca `addItem` with drop-on-map); the spell still succeeds.
4. Green magic effect on the caster; spend mana and soul.

#### Parameters

| Parameter | Value | Sources |
|---|---|---|
| pool | 3577 meat, 3582 ham, 3592 grape, 3585 apple, 3600 bread, 3601 roll, 3607 cheese | Ca; F names the same seven |
| count | 1 guaranteed + 1 at 50 % | Ca; F "one or two"; the 50 % and uniform draw are Ca only (QI1) |
| history | mana 30 to 120 (7.6); druids only (8.70) | F |

#### Proposed authoring shape

`native_behavior` `random_item_grant`: `pool [ItemRef x7]`, `guaranteed 1`, `extra 1`, `extra_chance_percent 50`, `selection "uniform_independent"`, `overflow "drop_on_caster_tile"`, `effect`.
The Effect `create_item` has one fixed `created_item` and cannot draw from a pool.

#### Engine tests

1. Random source that fails the 50 % check and picks index 0: exactly 1 item, meat.
2. Source that passes the check, both draws index 6: 2 items, both cheese.
3. Full backpack: items drop on the caster's tile, mana and soul are spent.
4. 10000 seeded casts: count is 1 or 2 (about 50 % each) and each pool item is within 5 % of uniform.

#### Open questions

- QI1. Weights and the 50 % second item: the Tibia-Stats study F cites is not captured; Ca is a hypothesis. Sample 200 casts in game.
- QI2. With a full inventory, does the item drop or does the cast fail?

### B.5 caster_restriction: healing spells that refuse some casters or targets

- Spells: Nature's Embrace (druid, elder druid); Ultimate Healing Rune, the rune item (its conjuring spell: druid). Intense Healing Rune (family `other`) needs the same self-target rule.

#### Behaviour

1. Nature's Embrace: after target resolution (§3 P4, or the attacked creature), if the target is the caster: cancel "You can't cast this spell to yourself.", poof, failure, no cost, no cooldown.
   Otherwise heal the target (plain heal Ability; formula belongs to the healing family). BR436996: "Esta magia não pode ser usada em si mesmo"; F1190771 states no self rule.
2. Ultimate Healing Rune, caster: refused for monk and exalted monk: cancel "Your vocation cannot use this rune.", poof, no charge. Ca names only "exalted monk"; F1189629 lists the rune's users as
   knights, paladins, sorcerers, druids, so both monk vocations are refused (F overrides Ca).
3. Ultimate Healing Rune, target: N8783/N8833 "can no longer be used on other characters"; F1189629/F1189630 "Can only be used at self or at summoned creatures" (Intense also convinced
   creatures); BR436969 "o próprio jogador". Ca and Cr allow only the caster ("You can only use this rune on yourself.", poof, no charge). Adopted (F over Ca): the caster, or a creature that has a master
   (summon or convinced); whose summon: QC2.
4. A refusal spends no charge, mana, soul or cooldown. On success the normal heal applies (group cooldown 1 s; non-aggressive; dispels paralysis, Ca).

#### Parameters

| Spell | Parameter | Value | Sources / superseded |
|---|---|---|---|
| Nature's Embrace | `forbid_self_target` | true | BR436996, Ca, Cr |
| Nature's Embrace | base power / range / level | 2000 / 7 / 275 | N8783, N8833 (650 to 2000); F1190771 (level 300 to 275 with the Wheel), BR436996, TD; Ca level 300 superseded. Shared Conservation (30 % to a second target): stance behaviour, not here |
| Ultimate Healing Rune | `forbidden_vocations` | monk, exalted monk | F1189629; Ca exalted monk only |
| Ultimate Healing Rune | `allowed_targets` | self, creatures with a master | N8833, F1189629; Ca and Cr self only |
| Ultimate Healing Rune | level / magic level / charges / cooldown | 24 / 4 / 1 / 1 s | Ca, F, TD |

#### Proposed authoring shape

One `native_behavior` `cast_restriction` next to the ordinary heal Ability: `forbidden_vocations [..]`, `target_rule "not_self" | "self_or_summon" | "self_only"`, `refusal_message`. `target_default`
(the rune targets the caster when unclear, Ca `TARGETCASTERORTOPMOST`) stays with the target family.

#### Engine tests

1. Druid casts Nature's Embrace with its own name: refused, mana and cooldown unchanged; with another player's name: healed.
2. Monk and exalted monk using an Ultimate Healing Rune: refused, charge kept.
3. Druid uses the rune on another player: refused; on itself: healed; on its own summon: healed.
4. A refused rune use does not start the 1 s healing group cooldown.

#### Open questions

- QC1. Does the Nature's Embrace self refusal exist officially (only BR and Ca say so)? Test on self by name.
- QC2. Healing rune on another player's summon, on a master-less monster, or on a party member's summon.
- QC3. Do unpromoted monks use the rune (F excludes "monks")? Test with a level-10 monk.


## Part C. Familiars, summon slots and speed, party buffs, stances and self states

Source key. N#### = official tibia.com news id (via TibiaData). F<rev> = Fandom revision at 2026-09-27 (extra pages fetched
2026-09-28: Familiars F1108992, Summoned Creature F1177634, Convince F1177638, Stance Spells F1197856, Harmony F1136128,
Serene F1104593, Speed). BR = TibiaWiki BR 2026-09-27. CA = Canary 99902524 (15.30). CR = Crystal ff7ede5. LIB = current
TibiaData library (check only, not the target date). Order: N > F/BR > CA > CR. A change without a number is "known, not
quantified". Where F and BR disagree and nothing else states it, the value is "unknown" and listed in the questions.
Speed base: CA `110 + (level - 1)`, F Speed page `109 + L` (same value).

### C.1 familiar: Summon Familiar spells (five vocations)

- Spells: Summon Knight Familiar (194, knight), Summon Paladin Familiar (195, paladin), Summon Sorcerer Familiar (196,
  sorcerer), Summon Druid Familiar (197, druid), Summon Monk Familiar (282, monk). Each exists twice in the bundles
  (`druid familiar` and `summon druid familiar`, same spell id): author one spell per id. The speed spells Haste, Strong
  Haste, Charge and Swift Foot carry the `familiar` tag only for the speed rule in section 2 (step A5).
#### Behaviour
1. Checks, in order (CA `CreateFamiliarSpell`): caster is premium; caster owns no summon of any kind; the vocation has a
   familiar; a free tile exists. Any failure spends no mana and starts no cooldown; POFF effect at the caster. The cancel
   texts are CA only ("You need a premium account.", "You can't have other summons.").
2. Creation: the vocation's familiar creature appears next to the caster (rules of `summon_creature`), owned by the caster,
   with the caster's chosen familiar look (default: the vocation look). Effects: magic blue at the caster, teleport at
   the familiar (CA). F1108992: the four looks per vocation differ only by name and sprite.
3. Speed: a familiar moves at its owner's speed (F1108992 "same as their master's"). CA only sets it at creation and on each
   speed spell (the higher of owner speed and its own base), so items or mounts gained later do not carry over. Override
   of CA by F (see Q5).
4. Lifetime 900 s (F, BR, CA `familiarTime 30 / 2` min). F1108992: the timer does not run while offline or swimming. CA
   uses wall-clock time and re-creates the familiar on login with the remaining time. Warnings at 60 s and 10 s left
   ("Your summon will disappear in less than one minute / 10 seconds", CA only). At zero the familiar is removed.
5. Cooldown 1800 s starts on a successful cast (F, BR, LIB, CA). It keeps running while swimming (F1108992). Death or
   expiry of the familiar does not reset it (CA). The support group cooldown is 2 s.
6. Follow: the familiar teleports to the owner when the owner is on another floor or more than 15 tiles away (CA; F: "if they
   get too far behind"). An ordinary summon is instead removed beyond 30 tiles or 2 floors (CA, F1177634 "too far").
7. Refusal in Lever Boss fights (F1108992); which encounters count is Q4.
#### Parameters
| Spell | Mana | Cooldown | Level | Premium | Duration | Sources / superseded |
|---|---:|---:|---:|---|---:|---|
| Knight | 1000 | 1800 s | 200 | yes | 900 s | F1182914, BR, LIB, CA. CA cooldown field 0, computed in script. |
| Paladin | 2000 | 1800 s | 200 | yes | 900 s | F1182916, BR, LIB, CA |
| Sorcerer | 3000 | 1800 s | 200 | yes | 900 s | F1182917, BR, LIB, CA |
| Druid | 3000 | 1800 s | 200 | yes | 900 s | F1182913, BR, LIB, CA |
| Monk | 1500 | 1800 s | 200 | yes | 900 s | F1182915, BR, LIB, CA |

#### Proposed authoring shape
Ability effect `summon_creature` (D18) cannot express the refusals: it silently creates nothing when the cap is hit and has no
duration. Needs `native_behavior` key `familiar_summon`, one for all five. Parameters: `familiar_creature` (CreatureRef of the
vocation default), `duration_ms` 900000, `refuse_if_owner_summons_at_least` 1, `timer_paused_while` ["offline", "swimming"],
`expiry_warning_ms` [60000, 10000], `return_to_owner` {`distance_tiles` 15, `on_floor_change` true}, `lever_boss_refusal` true.
Level, premium, `costs.mana`, `cooldown_ms` 1800000 and the support group stay in the plain spell fields. D16 says the summon
parameters move from the creature `summoning.familiar` block to the spell: keep one source (the spell).
#### Engine tests
1. Premium level-200 knight, 1000 mana, no summons: familiar next to the caster, mana -1000, cooldown 1800000 ms.
2. Non-premium caster, or a caster with one Summon Creature summon: refused; mana, cooldown and summons unchanged.
3. Familiar warnings at 840 s and 890 s, removal at 900 s; a recast at 899 s is refused by the cooldown.
4. Familiar killed at 100 s: no new familiar until 1800 s after the cast.
5. Owner 16 tiles away or one floor off: familiar teleports beside the owner; an ordinary summon at 31 tiles is removed.
6. Owner offline for one hour at 300 s: familiar returns with 600 s left (F reading; CA gives 0).
#### Open questions
- Q1. Does a familiar count toward the cap of 2 for Summon Creature, Animate Dead and Convince, and may it be cast with an
  ordinary summon out? CA: it counts, and casting is refused with any summon. F: silent.
- Q2. Does the cooldown run while offline? (F: timer paused offline; cooldown offline not stated.)
- Q3. Do familiars vanish when the owner dies or logs out? (F1177634 says summons die with the owner; familiars not named.)
- Q4. Which fights forbid familiars (F "Lever Bosses")?
- Q5. Is familiar speed always the owner's speed including item and mount speed, or only refreshed at casts?
- Q6. Teleport distance of 15 tiles and the floor rule: CA only.

### C.2 summons_share_condition: speed spells and shared summon slots

- Spells: Haste (6; all vocations), Strong Haste (39; druid, sorcerer, monk), Charge (131; knight), Swift Foot (134; paladin),
  Summon Creature (9; druid, sorcerer), Animate Dead Rune (83; rune item 3203, conjured by the druid/sorcerer instant),
  Convince Creature Rune (12; rune item 3177, conjured by the druid instant).
#### Behaviour A: speed spells
1. Self cast, not aggressive. The caster gets a haste condition for the duration below. A later speed spell replaces the
   running value and duration (CA replaces, does not add; which one wins is Q9). N8862 (2026-06-23) fixed Haste and Swift Foot
   stacking.
2. Adding haste removes paralysis (F1182734, F1182905 "cures paralysis"; CA does it for every haste). Being paralysed removes
   haste (CA `Creature::onAddCondition`).
3. Speed gain is a percentage of the caster's base speed (F). CA uses a flat offset instead (table); the offsets are not stated
   by F, BR or N and are not adopted (Q8).
4. Swift Foot also lowers all damage the caster deals by 30 % for its duration; attacks and spells stay allowed (N8783, N8833,
   F1190580, CA, CR). Swift Foot Wheel augments were removed by N8833, so the Wheel tag is void here.
5. Familiars: a familiar's speed follows its owner's (section 1 step 3), so the spell needs no per-spell sharing code. Ordinary
   summons never receive the speed (CA filters `familiar()`). CA copies the owner formula to each familiar with a different
   constant (table) and with 33 s for Haste against 30 s for the owner, an internal inconsistency.
#### Behaviour B: summon slots
1. At most 2 owned summons in total from Summon Creature, Animate Dead and Convince (F1177634, CA). Familiar: Q1.
2. Summon Creature: the word `utevo res` plus a creature name. Refused (no mana, POFF) if the name is unknown or not summonable,
   if the caster has 2 summons ("You cannot summon more creatures."), if mana is below the creature's cost, or if no tile is free.
   On success the mana of that creature is spent (counts as magic level training, CA), the creature appears beside the caster,
   magic blue at the caster and teleport effect at the creature. The spell has no fixed cost (F "varies", LIB -1).
3. Animate Dead Rune: use on a tile whose top item is a movable corpse. Refused if the caster has 2 summons or a black skull
   (CA). Success: the corpse is removed and a Skeleton is created there, owned by the caster; the rune loses a charge.
   F1189758 adds: caster next to the corpse, corpse not too fresh (behaviour in the first 10 s unclear, B-QW10), not on non-decaying
   corpses, works inside a protection zone.
4. Convince Creature Rune: target must be a monster, convincible, and without a master (F1177638 "summoned creatures can never
   be convinced"; CA allows only a master named "a carved stone tile"). Refused at 2 summons or too little mana. Success: mana of
   the target creature is spent, the target becomes the caster's summon, a rune charge is used.
#### Parameters
| Spell | Speed gain | CA player delta (b = base speed) | Duration | Mana | Cooldown / groups | Premium | Sources / superseded |
|---|---|---|---|---:|---|---|---|
| Haste | +30 % | 0.3 b - 12 (familiar: 0.3 x - 24) | F 30 s, BR 33 s, CA 30 s (familiar 33 s) | 60 | 2 s / support 2 s | no | F1182734, BR, LIB. F cites N8076 (2024-10-08, premium dropped); CA marks it premium (overridden). |
| Strong Haste | +70 % | 0.7 b - 28 (familiar: 0.7 x - 56) | F 21 s, BR 22 s, CA 22 s | 100 | 2 s / support 2 s | yes | F1182905, BR, LIB |
| Charge | +90 % | 0.9 b - 36 (familiar: 0.9 x - 72) | 5 s (F, BR, CA) | 100 | 2 s / support 2 s | yes | F1182906, BR |
| Swift Foot | +80 % | 0.8 b (familiar: 0.8 x - 72) | 10 s (F, BR, CA) | 400 | 4 s / support 2 s, focus 2 s | yes | F1190580, N8833 (supersedes 10 s own cooldown and 10 s focus group); BR still says 10 s (stale) |

x = max(owner base, familiar base). Level: Haste 14, Strong Haste 20, Charge 25, Swift Foot 55 (F, LIB, CA).

| Spell | Level | Mana | Cooldown / group | Other | Sources |
|---|---:|---|---|---|---|
| Summon Creature | 25 | creature's `summoning.mana_cost` | 2 s / support 2 s | not premium; CA default aggressive (no cast in a protection zone, Q10) | F1182911, LIB, CA |
| Animate Dead Rune | 27 | 0 (rune) | 2 s / support 2 s | magic level 4, 1 charge; conjure 600 mana, 5 soul | F1189758, LIB, CA |
| Convince Creature Rune | 16 | target's `summoning.mana_cost` | 2 s / support 2 s | magic level 5, 1 charge; conjure 200 mana, 3 soul | F1189760, LIB, CA |

Sampled F creature pages give equal summon and convince mana (Demon Skeleton 620, Stone Golem 590, Skeleton 300), so one field
per creature is enough. Official text says only that the cost varies with the creature's strength.
#### Proposed authoring shape
- Haste, Strong Haste, Charge: plain Ability, effect `condition` {type haste, `fixed_duration`, `speed_formula`}, `duration_ms`.
  The formula is `percent of base speed` (F). Swift Foot adds a second condition with `attribute_modifiers`
  [{`damage_dealt`, `percent_of_base`, 70}]. No native key: the familiar rule lives in `familiar_summon` (section 1).
- Summon slots: one key `acquire_summon`. Parameters: `source` ("named_creature" | "corpse_tile" | "target_creature"),
  `summon_cap` 2, `mana_source` ("creature_summoning_mana_cost" | "none"), `require_flag` ("summonable" | "convinceable" |
  none), `require_masterless_target` (true for convince), `created_creature` (Skeleton for `corpse_tile`), `refuse_black_skull`
  (true, CA only). The name argument parsing belongs to the `player_parameter` pattern. Animate Dead (corpse to Skeleton) is the
  `tile_item_operation` `raise_corpse` of part D section 2.3; `corpse_tile` here names the same source.
#### Engine tests
1. Haste on a caster with a familiar: caster +30 % of base for the duration; familiar speed equals the owner's; a Summon Creature
   summon is unchanged.
2. Strong Haste, then Haste 5 s later: no addition of the two; per CA only the Haste value and a fresh duration remain (Q9).
3. Strong Haste on a paralysed caster: paralysis ends.
4. Swift Foot: damage dealt x0.7 for 10 s, attacks and casts allowed; re-cast at 3 s refused (cooldown 4 s); Haste allowed.
5. Summon Creature with 2 summons: refused, mana unchanged. With 1 summon and enough mana: creature appears, mana = its cost.
6. Animate Dead on a corpse: Skeleton, corpse gone, one charge used; on an empty tile: refused, charge kept.
7. Convince on another player's summon: refused. On a convincible wild monster: mana = its cost, it becomes the caster's; with one
   Summon Creature summon and one skeleton already out, refused.
#### Open questions
- Q7. Duration of Haste (F 30 s / BR 33 s) and Strong Haste (F 21 s / BR 22 s). Provisional: F, matched by CA for Haste only.
- Q8. Speed gain: plain percentage of base (F) against CA's flat offsets (-12, -28, -36, 0).
- Q9. Which spell wins when a second speed spell is cast while one runs (CA: the newer one, even if weaker).
- Q10. Is Summon Creature castable in a protection zone? F1189758 allows the rune there; CA blocks both.
- Q11. Animate Dead: adjacent corpse required (F) or far use (CA `allowFarUse`)? Exact corpse age and type rules are unknown.
- Q12. Is the summon mana simply the per-creature value, and is the Convince cost the same for every creature?

### C.3 party: party area buffs

- Spells: Train Party (126, knight), Protect Party (127, paladin), Heal Party (128, druid), Enchant Party (129, sorcerer),
  Enlighten Party (278, monk).
#### Behaviour
1. The caster must be in a party. Otherwise the cast fails ("No party members in range.") with no mana, no cooldown (CA;
   N8866 2026-06-30 fixed Enlighten Party taking mana in this case).
2. Affected creatures: party members (leader and members) standing in the spell area around the caster, the caster included.
   F1182927: "36 square meters around the caster"; BR Enchant Party: 37 SQM around the caster. The count matches Canary's
   `AREA_CIRCLE3X3` (radius-3 circle, rows of 3, 5, 7, 7, 7, 5, 3 tiles, centre = caster). CA instead accepts any party member
   within 36 tiles by Chebyshev distance on any floor and draws the effect on an 11 x 11 circle of 61 tiles. Both are
   treated as CA defects (F/BR over CA). Same floor is assumed (Q13).
3. At least one other member must be affected (F1182666; N8866). Else the cast fails as in step 1.
4. Mana (Train, Protect, Heal, Enchant): official text says only "Mana: var." and that the average cost per member falls
   with more members. The community formula F1182737 (Heal Party) is `ceil(base x 0.9^(X-1) x X)`, X = affected members incl.
   the caster; CA uses it for all four with its own bases. It is community-supported, not an official fact. The cast fails
   for too little mana; mana is charged only after the cast succeeds.
5. Enlighten Party costs a fixed 75 (F1182676, BR field, LIB, CR); CA's formula on base 120 is overridden. BR's note "depends on
   the number of members" contradicts BR's own field (Q14).
6. Each affected member gets the condition below. A recast replaces the same condition and restarts its full duration; it does not
   stack (CA keys by type and sub id). CA gives Heal Party and Enlighten Party the same regeneration key: keep them separate.
7. Cooldown 2 s own and 2 s support group; Enlighten Party 300 s own and 2 s support. `party` is not an official cooldown group.
#### Parameters
| Spell | Base mana (X = 1) | Condition on each member | Duration | Sources / superseded |
|---|---:|---|---:|---|
| Train Party | 60 (CA only) | sword, axe, club, distance +3 | 120 s | F1182927, BR, CA. F also lists fist; BR and CA do not (Q15). |
| Protect Party | 90 (CA only) | shielding +3 | 120 s | F1182881, BR, CA |
| Heal Party | 120 | regeneration 20 HP every 2 s (1200 HP total) | 120 s | F1182737, BR, CA, CR |
| Enchant Party | 120 (CA only) | magic level +1 | 120 s | F1182666, BR, CA |
| Enlighten Party | fixed 75 | mana regeneration, amount unknown | unknown | F: "slowly restores mana"; BR: 160 mana over 2 min; CA: 50 per 3 s for 5 min (5000); CR: 5 per 2 s for 5 min (750) |

All: level 32, premium (F, LIB detail, CA). Formula check for Heal Party, F table X = 2..10: 216, 291, 350, 394, 426, 447, 460,
465, 465; with round-up all rows match except X = 3 (formula 291.6, F says 291, CA rounds up to 292).
#### Proposed authoring shape
`native_behavior` key `party_buff`, one for all five. Parameters: `area` (areaMatrix, radius-3 circle, 37 tiles), `same_floor` true,
`min_affected` 2, `requires_party` true, `mana` {`mode` "scaled" | "fixed", `base`, `falloff` 0.9, `rounding` "up"}, `effect` (EffectRef applying the
member condition). Member conditions use existing pieces: `regeneration` {`health_gain` 20, `health_interval_ms` 2000} for Heal
Party, `attribute_modifiers` (`add`) for Enchant, Protect and Train, `buff_spell` true, `fixed_duration` 120000. `costs.mana`
is 0 for the scaled spells. Enlighten Party stays blocked until Q14 is settled.
#### Engine tests
1. Caster not in a party, or alone in the area: refused; mana and cooldown unchanged.
2. Heal Party with 3 members in the area: mana 292 by the formula (291.6 rounded up); each member receives 20 HP per 2 s for 120 s.
3. 10 members: 465 mana; the cost per member (46.5) is below the 2-member cost per member (108).
4. Member at offset (3, 1): included; at (3, 2) or (3, 3): not affected and not counted; member on another floor: not affected.
5. Recast at 60 s: the condition runs 120 s again, not 180 s, and the gain is not doubled.
6. Mana just below the computed cost: refused, no member affected.
7. Enlighten Party: 75 mana with 2 or 6 members; a second cast within 300 s is refused.
#### Open questions
- Q13. Exact area shape and floors (37 tiles is a count; the shape is Canary's); the BR field `spellrange 4` is unexplained.
- Q14. Enlighten Party: total mana, tick interval and duration (BR 160 / 2 min; CA 5000 / 5 min); fixed 75 or scaled.
- Q15. Does Train Party include fist fighting (F yes; BR and CA no)?
- Q16. Base mana of Train, Protect and Enchant Party: only CA states 60, 90, 120; F and BR say "varies".
- Q17. Rounding: F table X = 3 gives 291, formula 291.6.
- Q18. Do party buffs end on death, logout or leaving the party?

### C.4 stance: standard-slot stances and elemental interplay

- Spells: Blood Rage (133, knight), Protector (132, knight), Sharpshooter (135 in F and the bundle, 313 in CA; paladin), Energy Wave
  (13, sorcerer). Virtue of Harmony, Justice and Sustain use the same slot mechanism (section 5).
#### Behaviour
1. State: each character has one `standard` slot holding at most one of Blood Rage, Protector, Sharpshooter (monk: one virtue).
   The slot may be empty (N8833). Sorcerers also have one elemental and one crippling slot (N8833, F1197856; not in this part).
2. Casting is a normal instant cast (level, promotion, mana, cooldowns). On success: if the cast stance is active it is switched off,
   otherwise it becomes the slot's stance and replaces the old one (CA `toggleStance`).
3. The active stance persists across logout (N8833) and is not reset by death (N8933, 2026-08-18).
4. Available from the promotion on (N8783, N8833; CA lists only elite knight and royal paladin). Change of vocation drops a stance
   that no longer fits (CA `pruneStances`).
5. Skill stances multiply the final skill (gear, imbuements, potions, party buffs, Wheel included) (N8833, F1199964); CA truncates
   to an integer. Blood Rage: sword, axe, club only (N8833; BR wrongly lists fist). Protector: shielding. Sharpshooter: distance.
6. Damage stances: each non-healing damage event (health or mana drain) is scaled once, after the damage hooks (CA
   `applyVocationStanceDamageModifiers`, integer product /10000, truncated toward zero). Blood Rage: damage taken x1.15.
   Protector: damage taken x0.85 and damage dealt x0.85 (all damage the player deals, including runes and auto attacks).
7. Cooldowns: Blood Rage and Protector own 2 s, support 2 s, stance 2 s. Sharpshooter own 10 s, support 2 s, stance 10 s (F,
   CA). The 4 s / 2 s cooldown in N8783 was Swift Foot, not Sharpshooter (wording error in the news).
8. Energy Wave (elemental interplay, owned with the sorcerer elemental stance): with Master of Thunder active, an energy spell gets
   +4 crit chance points (Wheel Lord of Destruction adds 2, 3 or 4) and arms a conversion. The next spell of another element
   turns into energy damage (including its damage-over-time condition) with the same bonus, and the arming clears. Both happen only
   after a successful cast (N8833, CA `commitElementalSpellCast`). With Master of Flames or Decay armed, Energy Wave itself is
   converted. Wheel augments (area, +10 %) are the Wheel owner's.
#### Parameters
| Spell | Skill effect | Damage effect | Mana | Level | Cooldown | Sources / superseded |
|---|---|---|---:|---:|---|---|
| Blood Rage | sword, axe, club +25 % of final skill | taken +15 % | 20 | 20 | 2 s / 2 / 2 | N8872 and N8887 (25 %); superseded 30 % (N8783, N8833); F1199964, CA 125; CR 130, level 60, mana 290 |
| Protector | shielding +30 % | taken -15 %, dealt -15 % | 20 | 20 | 2 s / 2 / 2 | N8833, F1190569, CA; superseded dealt -30 % (N8783) |
| Sharpshooter | distance +32 % of final skill | none | 250 | 20 | 10 s / 2 / 10 | N8872 (32 %); superseded 40 % (N8833), 35 % (N8783), 50 % (F history); F1194817, CA 132 |
| Energy Wave | none | energy area, base power 150; Master of Thunder +4 crit points | 170 | 38 | 8 s / attack 2 | F1182675, BR, LIB, CA; N8833 (stance); not premium |

All except Energy Wave: premium (F, LIB, CA).
#### Proposed authoring shape
`native_behavior` key `stance_toggle` for the three knight and paladin stances (and the virtues). Parameters: `slot` "standard",
`modifiers` (array of {`kind`: "skill_percent_of_final" with `skills` and `percent` | "damage_taken_percent" | "damage_dealt_percent"}),
`persist_across_sessions` true, `keep_on_death` true, `toggle_off_costs_mana` true (CA, Q19). The stance is not a timed condition, so
the condition schema (`fixed_duration` only) cannot hold it. Energy Wave needs no key: a plain Ability with `damage_type` energy;
the elemental stance behaviour reads that type.
#### Engine tests
1. Blood Rage cast: slot = Blood Rage; sword 100 becomes 125, shielding unchanged; incoming 100 becomes 115.
2. Protector cast while Blood Rage is active: Blood Rage ends; shielding x1.3; outgoing 100 becomes 85; incoming 100 becomes 85.
3. Protector cast again: slot empty, all values back to base.
4. Relog and death: the stance stays.
5. Sharpshooter with distance 100 (after gear): 132; a recast at 5 s is refused (own cooldown 10 s).
6. A knight without promotion cannot cast Blood Rage.
7. Protector on: a heal the knight casts is not reduced.
8. Energy Wave under Master of Thunder: crit chance +4 points; a following fire spell deals energy damage once, the next is normal.
#### Open questions
- Q19. Does switching a stance off cost mana and start cooldowns? CA does; N and F are silent.
- Q20. Bundles list `knight`, `paladin`, `monk` for these stances; N and F say promotion only. Needs the requirements owner.
- Q21. Rounding, and which damage counts (mana drain, agony, damage by summons, reflected damage).
- Q22. Cooldown group key of the three virtues: `stance` (F infobox, CA) or `virtue` (BR, F Virtue Spells page). Bundles use `virtue`.

### C.5 conditional_self_state: Cancel Magic Shield and the virtues

- Spells: Cancel Magic Shield (245; druid, sorcerer), Virtue of Harmony (274), Virtue of Justice (275), Virtue of Sustain (276)
  (monk). Blood Rage, Protector and Sharpshooter are in section 4.
#### Behaviour
1. Cancel Magic Shield: self cast; ends the caster's Magic Shield condition; magic blue effect. Mana 50, level 14, premium,
   2 s / support 2 s (F1182630, BR, LIB, CA). The cast also succeeds without a shield in CA (Q23). Mana already absorbed is not
   refunded.
2. Virtues: stance slot `standard`, exclusivity, toggle, persistence and vocation rules of section 4. Mana 210, level 20, premium,
   own 10 s, support 2 s, stance/virtue group 10 s (F1197257-9, LIB, CA). Automatically learned with the promotion (F).
3. Serene is an input from the monk owner: the effect uses the serene value while the monk is serene. N8944 (2026-09-01) changed
   Serene to 7 or fewer adjacent creatures (was 5; F1104593 text says 6 or more removes it, superseded).
4. Justice: raises fist fighting by 8 %, 16 % serene (F1197258, BR, CA). CA adds 8 % of the base fist skill on top of the final skill;
   base or final is not stated (Q24).
5. Sustain: healing done by monk spells +35 %, +70 % serene, including Virtue Healing (F1197259, BR). CA uses x1.35 / x1.70 but
   skips the Harmony passive heal (Q27).
6. Harmony: raises the Harmony base bonus by 50 % of itself, 100 % while serene (N8944), where the base is 7 % + 0.005 % per level
   (N8944 gives level 200: 8 %, 16 % serene; level 700: 10.5 %, 21 % serene; 12 % and 15.75 % are derived from the 50 % rule). A Spender cast under this virtue refunds
   1 Harmony (F1197257, CA). Rounding: uncertain. Superseded: flat +3 % / +6 % (F1197257, F1136128), CA base 8 % plus 4 / 8 flat.
   Harmony itself is the `monk_harmony_virtue` pattern.
7. Party bonuses while a monk has any virtue active, for party members in the game window: knight -4 % damage taken, paladin
   +8 % auto attack damage, sorcerer +8 % spell and rune damage, druid +16 % healing by spells and runes (N8944; earlier -3, +6, +6,
   +12 in N8833, CA still uses -3). Monks get the same while serene when that vocation is in the party. Guiding Presence adds +33 %
   (N8944, Wheel).
#### Parameters
| Spell | Effect | Values | Sources / superseded |
|---|---|---|---|
| Cancel Magic Shield | end Magic Shield | none | F1182630, BR, CA |
| Virtue of Harmony | Harmony base bonus | x1.5, serene x2 | N8944; superseded above |
| Virtue of Justice | fist fighting | +8 %, serene +16 % | F1197258, BR, CA; earlier 10 / 20 % until N8408 (2025-06-11, via F) |
| Virtue of Sustain | healing done | +35 %, serene +70 % | F1197259, BR, CA |
#### Proposed authoring shape
- Cancel Magic Shield: no native key. Ability effect `remove_condition` (`removed_condition` magic_shield) on the caster.
- Virtues: `stance_toggle` (section 4) with modifiers `fist_bonus_percent` {`percent` 8, `serene_percent` 16, `basis` "final" | "base"},
  `healing_done_percent` {35, serene 70}, `harmony_base_bonus_scale` {50, serene 100}, and `party_bonus` [{`vocation`, `kind`, `percent`}].
  The virtue's effect on Harmony and the Serene state are read from the monk owner.
#### Engine tests
1. Cancel Magic Shield with a shield: shield gone, mana -50, later damage hits hit points; cooldown 2 s.
2. Cancel Magic Shield without a shield: result per Q23 (CA: cast succeeds, mana spent).
3. Monk casts Justice, then Sustain: only Sustain is active; casting Sustain again empties the slot; recast within 10 s refused.
4. Sustain: a 100-point heal becomes 135, or 170 while serene.
5. Level-200 serene monk with Virtue of Harmony: Harmony base bonus 16 %; not serene 12 %; without the virtue 8 %.
6. Knight in the monk's party with any virtue active: damage taken x0.96; virtue off: x1.
7. Virtue stays after relog and death.
#### Open questions
- Q23. Cancel Magic Shield with no active shield: fails or succeeds and spends mana?
- Q24. Justice: percentage of base or of final fist skill?
- Q25. Party bonuses from every virtue (N, CA) or only Justice (BR text)?
- Q26. Harmony formula rounding and the exact meaning of "increases by 50 %" (N8944 gives examples only for the serene case).
- Q27. Does Sustain multiply the passive Virtue Healing (F yes, CA no)?


## Part D. Delayed strikes, positions, default targets, equipment, presentation-only extras and other scripts
Families: `delayed_or_repeated`, `target_position`, `target_default`, `equipment_dependent`, `extra_presentation_only`, `other`.

Source keys used in the tables:
- N8783 = official news 8783 (2026-05-05, test server stage 1, not final).
- N8833 = official news 8833 (2026-06-02, release-state list).
- N8849 = official news 8849 (release notice, 2026-06-16).
- N8872 = official news 8872 (2026-07-07, balancing).
- N8875 = official news 8875 (2026-07-07, fixes).
- F:<id> = Fandom revision id.
- BR = TibiaWiki BR capture 2026-09-27.
- C = Canary 99902524.
- X = Crystal ff7ede5.
- W = `wheel-augments.json`.

Stage 1/2/3 values come from the Wheel of Destiny. All Wheel-gated spells stay behind `wheel_unlock` (fail closed, cast cancelled with no cost).

### D.1 delayed_or_repeated: timed effects, delayed strikes, repeated casts
- Spells:
  - Avatar of Balance (monk), Avatar of Light (paladin), Avatar of Nature (druid), Avatar of Steel (knight), Avatar of Storm (sorcerer).
  - Divine Empowerment (paladin).
  - Death Echo (sorcerer).
  - Divine Grenade (paladin).
  - Spiritual Outburst (monk).
- Three patterns:
  - A. timed self buff: the five Avatars;
  - B. owned field with a standing bonus: Divine Empowerment;
  - C. a strike that fires after a delay: Death Echo, Divine Grenade, and the Spiritual Outburst recast.

#### D.1.1 Behaviour, pattern A (Avatars)
1. The cast is refused when the caster has Wheel stage 0 of that Avatar: cancel message, no cost, no cooldown (C, all five scripts).
2. A successful cast:
   - changes the caster's outfit for 15 s;
   - shows the effect "avatar appear" on the caster's tile;
   - starts an avatar timer of 15 s.
   - A recast during the timer restarts the 15 s (C: the same outfit condition is added again).
3. While the timer runs, the caster's incoming damage is reduced by 5 / 10 / 15% (stage 1 / 2 / 3) (F, BR, C). C applies it after resistances and rounds the reduction up.
4. While the timer runs, the caster's critical hit chance is 100% and the critical extra damage is +5 / 10 / 15% (F, C).
5. Timer end: outfit and bonuses end together.
6. Cooldown: 120 / 90 / 60 min by stage (F; C reaches the same values as 2 h minus 30 min per stage from 2). Group cooldown 2 s (support).

Parameters (identical for the five spells apart from the row values):

| Spell | Words | Outfit lookType | Mana | Sources |
|---|---|---:|---:|---|
| Avatar of Balance | uteta res tio | 1823 | 1200 | F:1182613, BR, C |
| Avatar of Light | uteta res sac | 1594 | 1500 | F:1182614, BR, C |
| Avatar of Nature | uteta res dru | 1596 | 2200 | F:1182615, BR, C |
| Avatar of Steel | uteta res eq | 1593 | 800 | F:1182616, BR, C |
| Avatar of Storm | uteta res ven | 1595 | 2200 | F:1182617, BR, C |

Common values: duration 15 s (F, BR, C); damage reduction 5/10/15 (F, BR, C); crit chance 100 (F, C); crit extra damage 5/10/15 (F, C; BR text is cut at "5% / 10%"). No superseded value.

#### D.1.2 Behaviour, pattern B (Divine Empowerment)
1. Refused at Wheel stage 0.
2. The cast puts an item (Canary id 40450, a client-side effect item), owned by the caster, on every tile of the 3x3 around the caster. It skips tiles that do not exist, that block movement as an immovable solid, or that change floor (C).
3. The items disappear after 5 s (F, C). Canary removes every such item in the 3x3 around the cast position, whoever owns it. That is a defect for two overlapping casters; the fix is to remove only the caster's own items.
4. While the caster stands on a tile that holds one of their own items, the damage they deal is increased by 8 / 10 / 12% (F, C). Canary re-evaluates once per second and on cast, so leaving the field keeps the bonus for up to 1 s.
5. Cooldown 32 / 28 / 24 s (F, C). Mana 500, self-target, no group besides support 2 s.

#### D.1.3 Behaviour, pattern C (delayed strikes)
**Death Echo** (sorcerer, base power 75):
1. The cast resolves its position (section 2, three modes) and hits a 5x5 area with the corners cut, 21 tiles (C `AREA_CIRCLE2X2`; the wiki says "5x5"), with death damage.
2. The cast is refused when the position is on another floor, out of range 7 or out of sight (C). Range 7 and line of sight are Canary values.
3. 1 s after the cast (N8833, F; N8783 said 2 s and is superseded), the same area at the same position is hit again for 50% of the damage (N8833: "50% of the initial damage").
   - The position is fixed at cast time; the echo does not follow the target.
   - The echo cannot trigger charms (N8783, F, C).
   - The echo ignores whether the caster is still on the cast floor (C).
   - The echo is dropped when the caster is no longer online (C).
   - The echo keeps the caster's elemental-stance snapshot of the cast (C; stance family).
4. The echo's damage: C recomputes the formula at echo time with a 0.5 multiplier, so it uses the caster's current stats. Q1.
5. Official N8875 fixed "inconsistent behaviour with Death Echo when targeting through doors". The fix is not described. Q2.
**Divine Grenade** (paladin, Wheel spell):
1. Refused at Wheel stage 0.
2. The cast resolves its position (section 2). The cast is refused when the tile is a protection zone and the caster is not in one (C).
3. A marker effect is shown on the position. After 3 s (F, C) the marker is removed and the tile explodes: holy damage in the same 21-tile shape (C; F scene "2sqmballtarget").
4. Damage timing (F overrides C):
   - F: on cast, magic level and flat damage give the raw damage. On explosion, critical chance, extra critical damage, Divine Empowerment, elemental pierce, prey and bounty talisman apply.
   - "Not affected by damage buffs received after the grenade has been planted."
   - C computes the whole formula at explosion time.
5. The spell's own registered cooldown is 1 s (C only); the real cooldown is set on cast: 26 / 20 / 14 s by stage (F, C).
6. The grenade fires even if the caster changes position; it is dropped if the caster is offline (C looks the player up by id).
**Spiritual Outburst** (monk, Wheel spell): the chain is specified in `OTERYN_SPELL_CHAIN_BEHAVIOUR_CANDIDATE_V1.md` §4.1. The repeat:
1. Refused at Wheel stage 0.
2. If the caster has full Harmony (5) at cast time, the same chain is cast again 1 s later, with the same target variant, for 37.5 / 50 / 62.5% of the damage (F, W; C).
3. The first cast consumes Harmony (F, "consumes your Harmony"). Whether the repeat still gets a Harmony bonus is Q3 (`monk_harmony_virtue`).
4. Cooldown 24 / 20 / 16 s (F, W, C). Mana 425.

#### D.1.4 Parameters

| Spell | Parameter | Value | Sources and superseded values |
|---|---|---|---|
| Death Echo | base power | 75 | N8872, F:1194828, BR, C; superseded 85 (N8833) |
| Death Echo | echo delay | 1000 ms | N8833, F; superseded 2000 ms (N8783) |
| Death Echo | echo damage | 50% | N8783, N8833, F, C |
| Death Echo | echo area | same as first | N8833, F; superseded "larger" (N8783, BR text) |
| Death Echo | mana / cooldown / level | 150 / 6 s / 120 | S24: the later official tibia.com spell list (150) supersedes N8833 (155); F, BR, C agree |
| Death Echo | Wheel I / II | -2 s cooldown / +12% base damage | N8833, N8872 (II); superseded II 8% (N8833), test values (N8783) |
| Divine Grenade | delay | 3000 ms | F, BR, C |
| Divine Grenade | base power | 190 | F, BR; C uses `level/5 + magic*4..6` |
| Divine Grenade | cooldown by stage | 26 / 20 / 14 s | F, C |
| Divine Grenade | damage by stage | +0 / +16 / +32% (reading of F "+16% base damage per additional stage", as in parts A and B; Q5) | F, W; superseded C: x1.3 / x1.6 / x2.0 in the script and a further +30 / 60 / 100% in `game.cpp` |
| Divine Empowerment | bonus / cooldown by stage | 8, 10, 12% / 32, 28, 24 s | F, C |
| Spiritual Outburst | repeat / cooldown by stage | 37.5, 50, 62.5% / 24, 20, 16 s | F, W, C |

#### D.1.5 Proposed authoring shape
- Avatars, no `native_behavior`:
  - an Ability (self, no target) with a `condition` Effect: fixed duration 15 s and attribute modifiers, and a `presentation_only` Effect;
  - condition attribute keys to register: `damage_taken_reduction_percent`, `critical_chance_override_percent`, `critical_extra_damage_percent`;
  - one schema gap: `appearance_transform` takes only a creature or item, so it needs an `outfit_binding` (for example `canary.appearance:outfit/1594`).
  - Stage values stay in the Wheel owner's table.
- Divine Empowerment, `native_behavior` key `owned_field_buff`:
  - parameters `field_item` (ItemRef), `radius_tiles` (1), `duration_ms` (5000), `skip_tile_flags` (`immovable_block_solid`, `floor_change`), `bonus_damage_percent_by_stage`, `owner_only` (true), `evaluate` (`on_damage`, not a 1 s poll).
- Death Echo, Divine Grenade and the Spiritual Outburst repeat, one key `delayed_strike`:
  - `position_source` (`cast_position`), `immediate_ability` (optional AbilityRef), `delayed_ability` (AbilityRef), `delay_ms`;
  - `value_timing` (`at_cast` or `at_expiry`), `requires_caster_online` (true), `suppress_charms`, `ignore_caster_floor`;
  - `marker_asset_binding` and `marker_ms` (Grenade), `requires_full_harmony` (Outburst repeat).
  - The strikes themselves are ordinary Abilities (area, damage Effect); the echo Ability carries the 50% factor in its Formula.

#### D.1.6 Engine tests
1. Death Echo cast at a target: hit at t=0 and at t=1000 ms on the same tiles; a creature that moved away between the two hits is hit only at t=0.
2. The caster logs out at t=500 ms: no echo. Divine Grenade: no damage before 3000 ms; a buff gained at t=1000 ms does not change the damage (per F).
3. Avatar of Steel at stage 2: 100 incoming damage becomes 90 and all hits are critical for 15 s, then normal. Divine Empowerment at stage 1: +8% inside the 3x3, 0 outside and after 5 s.
4. Spiritual Outburst at Harmony 5: a second chain at t=1000 ms; at Harmony 4: none.
5. Any Wheel spell at stage 0: refused, mana and cooldown unchanged.

### D.2 target_position: casting at a position or at a tile item
- Spells: Death Echo (sorcerer), Divine Grenade (paladin), Animate Dead Rune (druid, sorcerer), Chameleon Rune (conjured by druids, part B section 1e; the rune item is used by all vocations, F item page), Desintegrate Rune (druid, sorcerer, paladin, monk).

#### D.2.1 Behaviour
**Instants with a chosen position (Death Echo, Divine Grenade).** Official N8833: three modes: with crosshair, at the cursor position, or below the target (below the caster when there is no target).
1. Crosshair or cursor: the position is the one the client sends. The spell must allow it (C `optionalTarget`), or the cast is refused.
2. No position sent: the position is the attacked creature's tile if that creature exists, is not removed and has health above 0. Otherwise it is the caster's tile.
3. Checks: same floor as the caster, within the spell range, line of sight when `block_walls` is true (C `canThrowSpell`). A failed check gives the message "creature not reachable" and a "poff" effect on the caster; no cost is spent.
4. Divine Grenade override (F): "if the target is 4 squares or less away, the grenade is placed at the target's feet; otherwise at the caster's". C refuses the cast instead. Q6.
**Rune on a tile (Animate Dead, Desintegrate).** The rune is used on a map tile.
5. The rune fails when the tile is unreachable or beyond the rune range (Desintegrate range 1, C, F "stand next to the item").
6. Animate Dead: the top item of the tile must be a corpse of a movable type (C).
   - The caster must have fewer than 2 summons and must not be black-skulled (C; BR "except if you already have two summoned creatures").
   - The engine then creates a Skeleton on the tile, owned by the caster. Only if the creation succeeds is the corpse removed and the rune charge spent. Otherwise: cancel message, "poff" on the caster.
   - F adds: the corpse must not be in a backpack; usable inside a protection zone; not on non-decaying or special corpses; not on the first stage of a human corpse; not in the first 10 s after a monster's death.
7. Desintegrate: removes up to 500 items from the tile, only items that are movable, not tagged with a script unique id, have no action id and are not one of the 7 human corpse types (C ids 4240-4243, 4246-4248; F: "will not disintegrate any dead human body, but will disintegrate monster bodies").
   - It is not an aggressive action: it works in a protection zone and does not lock the caster (F).
   - C always sends a "not possible" cancel message and a "poff" effect even after removing items, and still spends the charge. Treat this as a Canary defect.
8. Chameleon: the target is the top item of a tile, or an item in a container slot, or an equipment slot (C).
   - It must exist and be of a movable type. Otherwise: "not possible", "poff".
   - On success the caster's appearance becomes that item for 200 s (F, BR, C) and a red magic effect shows on the caster.
   - F and BR: it cannot be used on creatures, NPCs, players or immovable objects.

#### D.2.2 Parameters

| Spell | Parameter | Value | Sources |
|---|---|---|---|
| Death Echo | range | 7 | C (F, BR do not state) |
| Divine Grenade | range | 4 | C; F "4 squares" |
| Animate Dead Rune | summon limit / creature | 2 / Skeleton | C, BR; F (creature) |
| Animate Dead Rune | level / magic level / charges | 27 / 4 / 1 | C, F |
| Desintegrate Rune | range / item cap | 1 / 500 | C, F |
| Desintegrate Rune | level / magic level / charges | 21 / 4 / 3 | C, F, TibiaData |
| Chameleon Rune | duration | 200 s | F, BR, C |
| Chameleon Rune | level / magic level / charges | 27 / 4 / 1 | C, F |
| all three runes | cooldown / group | 2 s / support 2 s | C, F |

#### D.2.3 Proposed authoring shape
- Death Echo and Divine Grenade: the existing `targeting.cast_at_position: true` (S20) already names the three modes. It needs no new key. The runtime rejects it today (game core).
- The three runes, one key `tile_item_operation` with `operation` in `raise_corpse`, `disintegrate`, `mimic_item`:
  - common parameters: `source` (`tile_top_item`, `container_slot`, `equipment_slot`), `require_movable` (true);
  - `raise_corpse`: `summon: CreatureRef`, `max_summons` (2), `refuse_black_skull` (true), `consume_item` (true);
  - `disintegrate`: `max_items` (500), `exclude_items` (list of ItemRef), `exclude_script_tagged` (true), `aggressive` (false);
  - `mimic_item`: `duration_ms` (200000).
- The existing `remove_items` and `summon_creature` Effects do not fit: `summon_creature` spawns at the caster and does not consume an item, and `remove_items` has no cap or tag rules.

#### D.2.4 Engine tests
1. Death Echo with no position and no target: centre = caster tile. With an attacked, living target: centre = target tile.
2. Death Echo cast at a position on another floor or behind a wall: refused, mana unchanged.
3. Divine Grenade with a target 6 tiles away: per F, planted at the caster's tile (if Q6 is accepted).
4. Animate Dead on a rabbit corpse with 1 summon: one Skeleton appears and the corpse is gone. With 2 summons: refused, corpse kept, charge kept.
5. Desintegrate on a tile with 3 movable items and a human corpse: the 3 items are removed and the human corpse stays.
6. Chameleon on a movable item under the cursor: outfit is that item for 200 s. On an immovable item or a creature: refused.

### D.3 target_default: who a spell hits when no explicit target is given
- Spells: Intense Healing Rune (druid, knight, paladin, sorcerer users), Ultimate Healing Rune (druid, knight, paladin, sorcerer users).

#### D.3.1 Behaviour
**Expose Weakness and Sap Strength (removed, S24; not authored).** They were removed by the 15.25 update and turned into the Aura of Exposed Weakness and Aura of Sapped Strength stances (N8833, N8849; F:1190738 and F:1190736 give `status = deprecated`; BR "Removida"; `official-changes.json` has `removed`). Crystal still has them, Canary 15.30 does not. Legacy behaviour, for the record only (C/X): the centre was the target creature, or the caster with no target; area 3x3 (Expose) or 5x5 with the Wheel (Sap); 16 s; players and summons were not affected. The stance behaviour belongs to the stance family.
**Healing runes.** The rune is used on a creature.
1. The target must be the caster or, per F, one of the caster's summoned or convinced creatures. Official N8833 (released 2026-06-16, N8849): the runes "can no longer be used on other characters". C accepts only the caster and answers "You can only use this rune on yourself." for anything else. F overrides C (S3/S21). Q7.
2. Only the top creature on the target tile is healed; on the caster's own tile only the caster (C `TARGETCASTERORTOPMOST`).
3. The heal is non-aggressive, cures paralysis, shows a blue magic effect. Rune cooldown 1 s in the healing group (1 s).
4. Ultimate Healing Rune is refused for the monk and the exalted monk: cancel "Your vocation cannot use this rune." (C names only the exalted monk; the F item page lists no monk vocation, so both are refused, as in part B section 5). Q8.

#### D.3.2 Parameters

| Rune | Parameter | Value | Sources |
|---|---|---|---|
| Intense Healing Rune | base power / level / magic level | 120 / 15 / 1 | F:1189630 (base power), C |
| Ultimate Healing Rune | base power / level / magic level | 250 / 24 / 4 | F:1189629, C |
| both | allowed targets | caster, own summons | F; N8833 "no other characters"; BR "self only" (text cut); C self only |
| both | cooldown / group | 1 s / healing 1 s | C |
| both | formula | unknown; S4 keeps the source form | C: intense `level/5 + magic*3.2..5.4 + 20..40`, ultimate `level/5 + magic*7.3..12.4 + 42..90` |

#### D.3.3 Proposed authoring shape
- An Ability with a `heal` Effect and a `remove_condition` Effect (`paralyze`), both existing.
- Two additions:
  - `targeting.allowed_targets`: `any`, `self_only`, `self_or_own_summons`. Here: `self_or_own_summons`.
  - `Effect.affects.top_creature_only` must also be usable without `kind` (today `affects` requires `kind`). See section 6.2.
- No `native_behavior`.

#### D.3.4 Engine tests
1. Ultimate Healing Rune on self: heal and paralysis removed.
2. On another player: refused, rune charge kept.
3. On the caster's own summon: healed (if Q7 is accepted per F).
4. Used on a tile without a creature: refused.
5. Monk and exalted monk use Ultimate Healing Rune: refused.

### D.4 equipment_dependent: weapon, shield and slot
- Spells: Flurry of Blows (monk), Sweeping Takedown (monk), Spiritual Outburst (monk), Shield Bash (knight), Shield Slam (knight), Chameleon Rune (section 2).

#### D.4.1 Behaviour
**Weapon-skill formula (Flurry of Blows, Sweeping Takedown, Spiritual Outburst).**
1. The formula takes `attack_skill`, `attack_value` and `attack_factor` from the item in the caster's hands (the existing `skill` input: "the engine fills attack_* from the wielded weapon").
2. Canary picks the weapon from the left hand, then the right hand. Shields, ammunition and non-weapons do not count (C `Player::getWeapon`).
3. With no weapon Canary passes skill 0 and attack 7, so the base term is 0 and only the flat bonus remains. This looks wrong for an unarmed monk. Q9.
4. Flurry of Blows: base power 55 (F, BR, C). Range roll 0.9-1.1. Wheel I enlarges the area (N8833, W).
5. Sweeping Takedown: base power 48. It hits two areas: the centre area, then the outer area for 75% of the centre's rolled range (F: "additional 75% of the damage to the following connected 3x2 box"; C reuses the centre roll). The skill bonus is `(skill-110)^2 * k`, with k rising in steps from 0.022 above skill 110 to 0.043 above 250 (C only; unknown elsewhere).
6. Spiritual Outburst: base power 42, skill formula, then Harmony.
7. Area shapes differ between sources: F "3x2 box plus a connected 3x2 box", BR "up to 14 sqm", C 11 centre tiles plus 10 outer tiles. Area is `area.matrix` data. Q10.
**Shield Bash and Shield Slam.**
1. Refused unless the caster wields a shield: message "You need to equip a shield to cast this spell.", "poff", no cost (C). The shield is found in the left or right slot; the first shield found counts.
2. Damage: `base_power * (shielding / 100) * (shield_defense / 10) + flat`, rolled 0.9-1.1 (C). `shielding` is the effective shielding skill; `shield_defense` is the item's defense value. Physical damage, reduced by armor (C `BLOCKARMOR`).
3. Shield Bash hits the target (range 1). Shield Slam hits the 8 adjacent tiles (C 3x3 area, caster excluded; N8833 "all adjacent enemies").
4. Each creature hit gets a debuff for 10 s: its next auto-attack does 50% less damage (N8833, F; test server N8783 said "misses", superseded).
   - The debuff ends at the first auto-attack that reaches the damage step (melee, ranged or fist, not spells). Both damage parts are multiplied.
   - Wheel Shield Slam II adds +25 points (75% in total) (N8833, W).
   - Canary does not filter players.

#### D.4.2 Parameters

| Spell | Parameter | Value | Sources and superseded values |
|---|---|---|---|
| Shield Bash | base power / mana / cooldown / level | 55 / 30 / 4 s / 18 | N8833, F:1189542, BR, C, TibiaData |
| Shield Slam | base power / mana / cooldown / level | 52 / 110 / 6 s / 30 | N8833, BR, C, TibiaData; F:1189541 says mana 90 (loses to N8833) |
| both | debuff | 50% next auto-attack, 10 s | N8833, F, BR, C; superseded "miss" (N8783) |
| Shield Slam | Wheel I / II | +15% life leech / +25% damage reduction | N8833 |
| Flurry of Blows | base power / mana / cooldown / level | 55 / 110 / 4 s / 35 | F:1182715, BR, C |
| Sweeping Takedown | base power / mana / cooldown / level | 48 / 195 / 8 s / 60 | tibia.com spell list 2026-09-28 (mana 195, level 60), F:1182918, BR, C; Canary mana 210 superseded (S24) |
| Sweeping Takedown | outer area factor | 75% | F, C |
| Spiritual Outburst | base power | 42 | F, BR, C |

#### D.4.3 Proposed authoring shape
- Weapon-skill spells: the existing `player_expression` Formula with `inputs: skill` and, when needed, `needs_weapon`. Unarmed handling is a world rule (Q9). No `native_behavior`. Sweeping Takedown's outer area is a second Effect whose Formula is the first one times 0.75.
- Shield spells, no `native_behavior`, three additions:
  - spell field `needs_shield` (boolean, like `needs_weapon`), with the cancel message as data;
  - a Formula variable `shield_defense`;
  - a new condition type `next_auto_attack_reduction` (fixed 10 s, `attribute_modifiers` on `auto_attack_damage`, `percent_of_base` 50) with a boolean `consumed_on_use`. The Wheel adds 25 to the percentage.

#### D.4.4 Engine tests
1. Shield Bash without a shield: refused, mana and cooldown unchanged.
2. Shield Bash with a shield of defense 20 and shielding 100, base power 55, flat 0: average 110 before armor.
3. Shield Slam with 3 adjacent monsters and 1 monster 2 tiles away: 3 are hit.
4. A debuffed monster: first melee hit at 50%, second at 100%. A spell from the monster is not reduced and does not consume the debuff.
5. The debuff expires after 10 s if unused.
6. Flurry of Blows with a wielded sword uses the sword's attack and skill; a shield in the other hand is ignored.

### D.5 extra_presentation_only: an extra effect on the caster
- Spells: Heal Friend (druid), Paralyze Rune (druid; the wikis write "Paralyse").

#### D.5.1 Behaviour
1. Heal Friend: on every cast a blue magic effect shows on the caster's tile, in addition to the green effect on the healed player (C). It also cures paralysis (C dispel).
   - The target is a player chosen by name; `allow_on_self` is false (F: "not able to heal the caster themselves").
   - Base power 260 (F, BR, C). Mana 120, level 18, cooldown 1 s, healing group 1 s, range 7 (BR, C).
   - Official N8833: it "heals more consistently (lower highs and higher lows)". Not quantified. Canary already uses a 0.9-1.1 spread. Wheel I / II: +4% / +6% base heal (N8833, W).
   - The druid stance (N8783, N8833: Shared Conservation, Channeled Preservation) belongs to the stance family.
2. Paralyze Rune: after a successful cast a green magic effect shows on the caster's tile, in addition to the red impact on the target (C).
   - The target is paralysed for 6 s (C). The paralysis is a speed condition with the formula (-1, 0, -1, 0). Any healing on the target cancels it (F).
   - Cost: the conjuring spell costs 1400 mana (tibia.com spell list, F, BR, C). Cooldown 6 s (F item history, C), group support 2 s. Druids only, magic level 18, level 54 (F, TibiaData).
   - Aggressive, and it locks the caster in a PvP world (C `setPzLocked`).

#### D.5.2 Proposed authoring shape
- Both are ordinary Abilities. The only gap is the caster-position effect: add an optional `presentation.caster_effect_asset_binding` on `Effect` (shown at the caster's tile once per cast). No `native_behavior`.
- Paralyze Rune's condition: `condition` type `paralyze`, `fixed_duration` 6000 ms, with a `speed_formula`. The formula constants above are Canary's.

#### D.5.3 Engine tests
1. Heal Friend on a named player in range: heal plus a caster-tile effect event.
2. Heal Friend on the caster: refused.
3. Paralyze Rune: target's speed is reduced for 6 s; a heal on the target ends it at once.
4. A failed cast (target immune, out of range) gives no caster-tile effect.

### D.6 other

#### D.6.1 Magic Wall Rune and Wild Growth Rune (P4 scripts)
Behaviour:
1. The rune is used on a tile (`allow_far_use` true). The projectile is an energy missile.
2. The cast is refused when the tile changes floor, or holds a creature other than a player (C script). The rune spell check also refuses when any creature visible to the caster stands on the tile (C `blocking` creature). In practice any creature blocks. Q12.
3. The engine creates a blocking item on the tile.
   - In a non-PvP world it creates the "safe" variant (walkable by players in an optional-PvP setting).
   - It sets a duration and the description "Casted by: <name>".
4. Magic Wall blocks movement and shots (F). Wild Growth blocks movement but lets ammunition and spells through (F). Both are indestructible except as F states for PvP worlds. These are Item properties, not spell logic.

| Rune | Item id (C) / safe id | Duration | Charges | Level / magic level | Sources and superseded |
|---|---|---|---:|---|---|
| Magic Wall Rune | 2128 / 10181 | 16 to 24 s | 3 | 32 / 9 | F:1189784, BR, C; TibiaData amount 3 |
| Wild Growth Rune | 2130 / 10182 | 30 to 60 s | 2 | 27 / 8 | F:1056364 (30-60 s); C fixes 30 s. F overrides. Q13 |

Vocations: Magic Wall rune users are all vocations; Wild Growth rune users are druids (F item pages, C).

Proposed shape: no `native_behavior`. Extend the existing `create_item` Effect with `duration_range_ms` (`minimum`, `maximum`), `refuse_on` (`floor_change_tile`, `creature_on_tile`) and `pvp_safe_item` (ItemRef). One shared shape for both runes.

Tests: (a) a wall on an empty tile lasts between 16 and 24 s; (b) a cast on a tile holding a monster is refused with the charge kept; (c) a cast on a stairs tile is refused; (d) a Wild Growth tile lets a spell pass through and blocks a walk.

#### D.6.2 Inflict Wound (knight, monk)
1. Range 1, needs a target, aggressive. The spell hits only the top creature on the target tile; on the caster's own tile only the caster (C `TARGETCASTERORTOPMOST`, the blocker).
2. It applies a bleeding condition: 15 ticks, one every 2 s, 50 damage each (750 in all), the first tick delayed (C, X). Weapon-type missile effect, blood effect.
3. F says it uses level and melee skill and deals one quarter to players. Official news 8389 (2025-05-20, as quoted in F history) says it now "also accounts for Fist Fighting". Neither matches the fixed 50 of C and X. Q14.
- Values: base power 90 (F, BR, X); mana 30, level 40, cooldown 30 s, group attack 2 s (F, BR, C, TibiaData).
- Shape: Ability with a `condition` Effect (`bleeding`, `damage_schedule`, `fixed_ticks` [{count 15, interval_ms 2000, amount 50}]). Missing piece: an ability-level `top_creature_only` (boolean), usable on any Effect, not only under `affects`. No `native_behavior`.
- Tests: two creatures on one tile: only the top one bleeds; 15 ticks of 50 over 30 s; recast during the bleed follows the condition rules of the engine.

#### D.6.3 Challenge (elite knight) and related
1. Every monster in the 3x3 around the caster (C `AREA_SQUARE1X1`) that is not a summon is forced to target the caster. Players are not affected (F).
2. For each monster: it must be able to target the caster (a normal targeting check; login-protected players cannot be selected). If it can, it attacks and follows the caster, its normal target changes are blocked, and its flee-at-low-health behaviour is disabled for 6 s (C default `targetChangeCooldown` 6000; F "at least 6 seconds").
3. The cast is non-aggressive and succeeds even if no monster is affected.
4. The same per-monster behaviour is the "target the caster for 6 s" part of Chivalrous Challenge (chain document §4.2).
5. Balanced Brawl (monk) uses the other AI override, "force melee": each monster in a fan in front of the caster keeps its target distance at 1 for 16 s (C, F "half-circle in front of the monk", 16 s). Reward bosses are skipped (C). BR describes it differently ("around the target"). Mana 80, level 175, cooldown 10 s. Q15.
- Values: Challenge mana 40 (F, BR, TibiaData; C says 30), level 20, cooldown 2 s, group support 2 s.
- Shape: `native_behavior` key `monster_ai_override`, parameters `mode` (`target_caster`, `force_melee`), `duration_ms` (6000, 16000), `skip_summons` (true), `skip_reward_bosses` (true for `force_melee`); the area is the Ability's `area`. It is shared by Challenge, Balanced Brawl and Chivalrous Challenge.
- Tests: Challenge with a fleeing monster at low health: it turns and attacks the caster for 6 s; a summon is unaffected; a player in the area is unaffected.

#### D.6.4 Mass Spirit Mend (monk, P4)
1. The area is a circle around the caster. Canary's table is 11x11 rounded (hypothesis); BR mentions "up to 4 sqm" and range 5. The radius is unresolved (A.3-Q2, Q16).
2. For each creature: the caster gets the lesser self formula (`level*0.2 + magic*7.22..12.79 + 44..79`); every other target gets the mass formula (base power 800).
3. Monsters without a master, and summons of monsters, are skipped unless their name is one of 8 listed bosses (C: leiden, ravennous hunger, dorokoll the mystic, eshtaba the conjurer, eliz the unyielding, mezlon the defiler, malkhar deathbringer, containment crystal). Players, player summons and NPCs are healed. Paralysis is cured.
4. Official N8833: no longer a spender spell; cooldown and base power adjusted, no numbers (known, not quantified). BR and C agree on mana 400, cooldown 12 s, base power 800, healing group 1 s. F:1182871 is stale (spender text). Q16.
- Shape: an Ability with two `heal` Effects: one with `affects` `player_side` (mass formula), one for the caster, and one with `affects` `named_creatures` for the bosses. This needs converter support for `doTargetCombatHealth` inside a callback. No `native_behavior`.

#### D.6.5 Monk Harmony formulas and absent spells (pointers only)
- Tiger Clash, Greater Tiger Clash and Devastating Knockout are blocked by a Harmony input in the formula, not by a native behaviour. They belong to `monk_harmony_virtue`. F: base powers 15 / 44 / 62. Devastating Knockout: cooldown 24 to 8 s and range 1 to 7 (N8944).
- The conjuring spells of Light Stone Shower Rune and Lightest Missile Rune, Practise Fire Wave and Practise Healing are absent from the official library and removed (S25; the rune items stay). They need no behaviour.

### D.7 Open questions (candidates for the owner's in-game test)
- Q1. Death Echo echo damage: 50% of the first roll, or a new roll of the formula at 50% (C). Do buffs gained during the 1 s count?
- Q2. What exactly N8875 fixed for Death Echo through doors (does a closed door block the area, the cast, or both)?
- Q3. Spiritual Outburst repeat: does it get a Harmony bonus after the first cast consumed Harmony?
- Q4. (settled) Death Echo mana is 150: under S24 the later official tibia.com spell list supersedes the 155 of N8833.
- Q5. Divine Grenade damage by stage: C multiplies twice (x1.3/1.6/2.0 and +30/60/100%); F cites "+16% per stage". Also the formula shape (level/5 + magic*4..6 against base power 190).
- Q6. Divine Grenade with a target more than 4 tiles away: planted at the caster's tile (F) or refused (C).
- Q7. Healing runes: may they target the caster's own summons and convinced creatures (F) or only the caster (C, BR)?
- Q8. Ultimate Healing Rune: both monk vocations are refused (F item page; C names only the exalted monk). Is the unpromoted monk really excluded (B-QC3)?
- Q9. Weapon-skill spells for an unarmed monk: which skill and attack value (C gives skill 0)?
- Q10. Areas of Flurry of Blows and Sweeping Takedown: F, BR and C disagree (section 4.1 step 7).
- Q11. (settled) Sweeping Takedown mana is 195: the tibia.com spell list of 2026-09-28, F and BR agree; Canary 210 is superseded (S24).
- Q12. Magic Wall and Wild Growth: does an invisible or hidden player on the tile block the cast?
- Q13. Wild Growth duration: 30 to 60 s (F) or 30 s (C); is the random draw uniform (also Magic Wall 16 to 24 s)?
- Q14. Inflict Wound bleeding: fixed 15 x 50 (C, X) or skill-based (F, news 8389)? One quarter against players?
- Q15. Balanced Brawl area: half-circle in front (F, C) or around the target (BR).
- Q16. Mass Spirit Mend values after the 15.25 adjustment (official gives no numbers) and the radius of the area (A.3-Q2).
- Q17. Animate Dead: must the caster stand next to the corpse (F) or may it be used from range (C `allow_far_use`)? Are the corpse age rules (10 s, human first stage) in force?
- Q18. Desintegrate: mapping of Canary's "unique id above 65535" and the human corpse ids to Oteryn's item family; the extra "not possible" message in C is not reproduced.
- Q19. Avatars: does the 100% critical chance apply to heals and to runes and auto-attacks? Does the damage reduction stack with other reductions in the order C uses?
- Q20. Divine Empowerment: instant evaluation (proposed) or a 1 s poll (C); overlapping fields of two casters.
- Q21. Shield spells: does the debuff also apply to players; is it consumed by an auto-attack that misses or is fully blocked?
- Q22. Chameleon: does the condition end on attack, on taking damage, or only at 200 s? Are items inside containers and worn items allowed (C) or only items on the ground (F)?
- Q23. Challenge: exact area ("nearby" in F, BR; radius 1 in C).
