# OTV2-20260912 — Reference Combat Light Healing (`exura`)

- Programme: `#486`
- Reference evidence tracker: `#483`
- Control plane: `#162`
- Lane: `combat`
- Scope: Light Healing / `exura` only
- Immutable target: `global-tibia-observable-2026-07-28-post-server-save`
- Protected base at admission: `main@1a9cb71f424a821633fd42f8a1a19920ffeff2c3`
- Research date: `2026-09-12`
- Runtime/client/protocol/DDL/production authority: **NONE**
- Implementation performed: **NO**
- Result: **research-complete with exact numeric heal formula / RNG / rounding still evidence-gated**

## 1. Purpose and evidence discipline

This packet investigates only the first Reference-playable Light Healing case required by the accepted GAME-ABILITY evidence path:

- metadata continuity;
- self-heal semantics;
- exact heal formula and rounding;
- cooldown and group cooldown;
- mana, level, vocation and Premium eligibility;
- only the failure/eligibility cases necessary for the first bounded playable heal fixture.

It does not mutate the accepted Reference manifest, GAME-ABILITY contracts or runtime implementation. Existing case IDs are retained:

- `ability_combat.light_healing.cast_metadata.v1`;
- `ability_combat.light_healing.self_heal_semantics.v1`.

Source roles are kept distinct:

1. `PRIMARY_OFFICIAL` — CipSoft/Tibia official material;
2. `STRUCTURED_REFERENCE_DATA` — TibiaWiki / equivalent structured encyclopaedia evidence;
3. `COMMUNITY_CORROBORATION` — community reverse engineering and discussion;
4. `OTS_HYPOTHESIS_ONLY` — Canary / Crystal and similar implementations.

No `UNKNOWN` field is filled from OTS convenience. Current/post-target official state does not silently move the immutable target.

## 2. Exact source register

### S1 — current official Light Healing Library

- Role: `PRIMARY_OFFICIAL`
- Locator: `https://www.tibia.com/library/?spell=lighthealing&subtopic=spells`
- Verified current content on 2026-09-12:
  - name: Light Healing;
  - formula: `exura`;
  - vocation: Druid, Monk, Paladin, Sorcerer;
  - group: Healing;
  - type: Instant;
  - magic type: Healing;
  - cooldown: `1s`;
  - group cooldown: `1s`;
  - level: `8`;
  - mana: `20`;
  - Premium: `no`;
  - official prose describes adventurers healing their **own** wounds and injuries;
  - spell is awarded free to newly chosen eligible vocations.
- Target relation: post-target/current evidence. Strong corroboration, but not sufficient by itself for target promotion.

### S2 — TibiaWiki BR Light Healing revision state

- Role: `STRUCTURED_REFERENCE_DATA`
- Current page: `https://www.tibiawiki.com.br/wiki/Exura`
- Stable revision exposed by the page: `oldid=436069`.
- History: `https://www.tibiawiki.com.br/index.php?title=Light_Healing&action=history`
- History fact: the latest revision before the immutable target is `2026-06-12 20:05`; the next visible history state is not before `2026-07-28`, so this revision spans the target date in the page history.
- Revision content includes:
  - `exura`;
  - Druids, Monks, Paladins and Sorcerers at level 8+;
  - Healing group;
  - spell cooldown 1 second;
  - group cooldown 1 second;
  - Premium false;
  - mana 20;
  - Base Power 40;
  - scales with Magic Level;
  - small HP heal whose amount depends on level and Magic Level;
  - paralysis cure is also listed, but that behavior is outside the first bounded fixture.
- Target relation: target-near structured revision whose documented page state spans the target date. This supports `DERIVED` target candidates, not `PROVEN` Global truth.

### S3 — official spell auto-unlock change

- Role: `PRIMARY_OFFICIAL`
- Locator: `https://www.tibia.com/news/?id=8675&subtopic=newsarchive`
- Date: `2026-01-27`.
- Official production change:
  - ordinary spells that previously had to be learnt from spell trainers unlock automatically on level-up;
  - no spell-learning cost;
  - trainers no longer teach them;
  - unlock levels remain unchanged;
  - existing characters receive all spells up to their current level automatically.
- Target relation: direct pre-target production anchor.

### S4 — official Magic manual

- Role: `PRIMARY_OFFICIAL`
- Locator: `https://www.tibia.com/gameguides/?section=magic&subtopic=manual`
- Current documented semantics:
  - spells are automatically learned when they become available for the vocation/level;
  - successful casting consumes mana;
  - insufficient mana produces a failed attempt and `You do not have enough mana`;
  - level-too-low and spell-not-learnt generic messages are documented;
  - after a spell is cast, that spell and its spell groups enter cooldown;
  - casting while the cooldown is active produces `You are exhausted`.
- Target relation: current official general semantics. Used only where supported by stronger chronology or for bounded current corroboration; literal target-era strings are not promoted without time-locked continuity.

### S5 — official level contribution change

- Role: `PRIMARY_OFFICIAL`
- Locator: `https://www.tibia.com/news/?id=6972&subtopic=newsarchive`
- Date: `2022-10-17`; effective with the following server save.
- Official rule:
  - through level 500: damage/healing +1 every 5 levels;
  - 501..1100: +1 every 6 levels;
  - 1101..1800: +1 every 7 levels;
  - 1801..2600: +1 every 8 levels;
  - 2601..3500: +1 every 9 levels;
  - above 3500 the same progression logic continues.
- Target relation: direct pre-target production rule with no stronger contrary target evidence found.

### S6 — official Base Power / Magical Archive introduction

- Role: `PRIMARY_OFFICIAL`
- Locator: `https://www.tibia.com/news/?id=8108&subtopic=newsarchive`
- Date: `2024-10-24`.
- Official statement: Magical Archive combat stats include base power and related spell fields; Base Power permits comparison of base attack/healing power of spells within a vocation.
- Important limitation: CipSoft does **not** publish an exact equation converting Base Power into Light Healing min/max HP, RNG distribution or integer rounding.

### S7 — target boundary and same-day production changes

- Role: `PRIMARY_OFFICIAL`
- Pre-boundary maintenance locator: `https://www.tibia.com/news/?id=8906&subtopic=newsarchive`
- Boundary change locator: `https://www.tibia.com/news/?id=8905&subtopic=newsarchive`
- Dates: `2026-07-27` / `2026-07-28`.
- Admitted use: confirms the selected July 28 post-server-save production boundary and the specific subjects changed that day.
- Rejected use: absence of Light Healing from the patch notes is **not** continuity proof.

### S8 — July 2026 vocation-adjustment delta context

- Role: `PRIMARY_OFFICIAL`
- Locator: `https://www.tibia.com/news/?id=8872&subtopic=newsarchive`
- Date: `2026-07-07`.
- Relevant context: production healing/balance values were actively adjusted before the target; Light Healing is not listed among the specific healing-spell mana changes shown there.
- Rejected use: omission is not proof that Light Healing was unchanged.

### S9 — legacy formula corpus

- Role: `COMMUNITY_CORROBORATION` / historical observation only
- Locator: `https://tibia.fandom.com/wiki/Formulae`
- Historical Light Healing formula listed:
  - min: `floor(level * 0.2) + magicLevel * 1.4 + 8`;
  - max: `floor(level * 0.2) + magicLevel * 1.795 + 11`.
- Critical warning on the source itself: observed spell/rune formulae are no longer correct after the 2020 vocation adjustments.
- Target disposition: **REJECTED as exact 2026 formula**.

### S10 — community Base Power reverse-engineering candidate

- Role: `COMMUNITY_CORROBORATION`
- Locator: `https://www.reddit.com/r/TibiaMMO/comments/1q1n3nh/dmg_calculator/`
- Candidate claim: for Magic-Level-based skills, community reverse engineering reports that `Base Power / 25` reveals the average skill multiplier.
- For Light Healing BP40 this implies an average coefficient candidate `40 / 25 = 1.6`.
- Target disposition: useful hypothesis only. It does not recover min/max, offsets, RNG distribution or rounding and is not promoted as Reference truth.

### S11 — Canary hypothesis

- Role: `OTS_HYPOTHESIS_ONLY`
- Repository/revision: `opentibiabr/canary@06c73ccb993b4625a966d452b7e9f867873d1f55`
- Path: `data/scripts/spells/healing/light_healing.lua`
- Candidate behavior:
  - healing;
  - paralysis dispel;
  - self target;
  - 1s/1s cooldown;
  - level 8;
  - mana 20;
  - eligible Druid/Paladin/Sorcerer/Monk families;
  - legacy numeric coefficients `1.4 / 1.795` and `8 / 11`.
- The code comment dates the formula comparison to official Tibia in **2019**.
- Target disposition: hypothesis/test-discovery input only.

### S12 — Crystal hypothesis

- Role: `OTS_HYPOTHESIS_ONLY`
- Repository/revision: `zimbadev/crystalserver@7884cfe2baccef8329324ddea3aa0bcd19326ccd`
- Path: `data/scripts/spells/healing/light_healing.lua`
- Library helper path: `data/scripts/lib/register_spells.lua`
- Crystal preserves the historical ML coefficients/offsets but substitutes a newer `calculateBaseDamageHealing(level)` helper.
- Target disposition: hypothesis only. Its level/rounding implementation is not accepted as Global truth.

## 3. Field-level result

| Field | Candidate target value | Classification | Confidence | continuity_to_target |
|---|---|---|---|---|
| spell identity | `Light Healing` | `DERIVED` | HIGH | `DERIVED` |
| words/incantation | `exura` | `DERIVED` | HIGH | `DERIVED` |
| group | `Healing` | `DERIVED` | HIGH | `DERIVED` |
| type | `Instant` | `DERIVED` | HIGH | `DERIVED` |
| magic type | `Healing` | `DERIVED` | HIGH | `DERIVED` |
| target policy | caster/self | `DERIVED` | HIGH | `DERIVED` |
| external target required | no | `DERIVED` | HIGH | `DERIVED` |
| minimum level | `8` | `DERIVED` | HIGH | `DERIVED` |
| mana cost | `20` | `DERIVED` | HIGH | `DERIVED` |
| individual cooldown | `1 s` | `DERIVED` | HIGH | `DERIVED` |
| Healing-group cooldown | `1 s` | `DERIVED` | HIGH | `DERIVED` |
| Premium required | no | `DERIVED` | HIGH | `DERIVED` |
| allowed vocations | Druid, Monk, Paladin, Sorcerer | `DERIVED` | HIGH | `DERIVED` |
| Knight eligible | no | `DERIVED` | HIGH | `DERIVED` |
| Base Power | `40` | `DERIVED` | MEDIUM-HIGH | `DERIVED` |
| depends on Magic Level | yes | `DERIVED` | HIGH | `DERIVED` |
| character level contributes to healing | yes | `DERIVED` | HIGH | `DERIVED` |
| generic level contribution through 500 | +1 damage/healing per 5 levels | `DERIVED` | HIGH | `DERIVED` |
| exact minimum heal function | unresolved | `UNKNOWN` | — | `UNKNOWN` |
| exact maximum heal function | unresolved | `UNKNOWN` | — | `UNKNOWN` |
| exact RNG distribution | unresolved | `UNKNOWN` | — | `UNKNOWN` |
| exact intermediate rounding | unresolved | `UNKNOWN` | — | `UNKNOWN` |
| exact final heal rounding | unresolved | `UNKNOWN` | — | `UNKNOWN` |
| success consumes 20 mana | yes | `DERIVED` | HIGH | `DERIVED` |
| successful cast starts spell cooldown | yes | `DERIVED` | HIGH | `DERIVED` |
| successful cast starts Healing-group cooldown | yes | `DERIVED` | HIGH | `DERIVED` |
| ordinary spell available automatically at required level | yes | `DERIVED` | HIGH | `PROVEN` for the Jan-27-2026 rule, `DERIVED` as applied to this fixture |
| exact rejected-cast mutation ordering | unresolved | `UNKNOWN` | — | `UNKNOWN` |
| simultaneous failure precedence | unresolved | `UNKNOWN` | — | `UNKNOWN` |
| exact target-era failure text | unresolved | `UNKNOWN` | — | `UNKNOWN` |

## 4. Metadata continuity decision

The previous accepted Oteryn package correctly remained fail-closed because the August 2026 official Library capture did not bridge the immutable July target.

The stronger chain now available is:

```text
TibiaWiki BR revision state dated 2026-06-12 20:05
  -> page history shows that state spanning 2026-07-28
  -> current official CipSoft Library independently reports the same core tuple
```

This does **not** turn structured wiki data into primary proof. It does support a `DERIVED` candidate for the core cast metadata under the Reference Investigator source strategy because:

- the source is target-near and revisioned;
- the history establishes the page state at the target date;
- current official data independently agrees;
- no stronger contrary target evidence was found.

The existing manifest remains unchanged until the proper manifest-authority path performs provenance/legal review and an explicit classification update.

## 5. Self-heal semantics

The first bounded observable semantic is:

```text
eligible injured caster invokes "exura"
-> no external creature/player target acquisition is required
-> healing recipient is the caster
-> caster HP increases
```

Classification: `DERIVED`.

Evidence:

- S1 official prose explicitly describes adventurers quickly healing their **own** wounds and injuries;
- S2 target-near structured state identifies the healing spell and small HP healing effect;
- S11/S12 independently use self-target semantics, but only as OTS corroboration.

Paralysis removal is deliberately excluded from the first fixture even though S2 and OTS sources surface it. The first fixture should start with `paralysis=false`.

## 6. Exact formula and rounding — final disposition

### 6.1 What is known

The target-era heal has at least these evidenced inputs/families:

- character level contributes to damage/healing (S5 and official Manual character semantics);
- Magic Level contributes to spell/healing power (S1/S2 and official Manual);
- Light Healing has structured Base Power `40` (S2).

### 6.2 Why the historical equation is inadmissible

The well-known legacy formula family:

```text
min = floor(level * 0.2) + magicLevel * 1.4   + 8
max = floor(level * 0.2) + magicLevel * 1.795 + 11
```

cannot be promoted for the 2026 target because:

- S9 explicitly marks these observed formulas incorrect after the 2020 vocation adjustments;
- S5 changed the generic level contribution again in 2022;
- S11's own formula comment traces its Global comparison to 2019;
- S12 already differs from S11 in the level-component implementation.

### 6.3 Base Power does not close the equation

S6 defines Base Power as a comparison metric but publishes no exact Light Healing conversion formula.

The community `BasePower / 25` candidate from S10 gives an interesting average ML coefficient hypothesis:

```text
40 / 25 = 1.6
```

The historical coefficient midpoint is:

```text
(1.4 + 1.795) / 2 = 1.5975
```

The numerical agreement is useful investigation signal, not admissible min/max proof. It does not establish:

- exact minimum coefficient;
- exact maximum coefficient;
- flat offsets;
- RNG distribution;
- level/Base Power composition;
- intermediate integer conversion;
- final rounding.

### 6.4 Final numeric classification

```yaml
exact_min_formula:
  classification: UNKNOWN
  continuity_to_target: UNKNOWN
exact_max_formula:
  classification: UNKNOWN
  continuity_to_target: UNKNOWN
rng_distribution:
  classification: UNKNOWN
  continuity_to_target: UNKNOWN
intermediate_rounding:
  classification: UNKNOWN
  continuity_to_target: UNKNOWN
final_rounding:
  classification: UNKNOWN
  continuity_to_target: UNKNOWN
community_average_ml_coefficient_candidate:
  value: 1.6
  classification: UNKNOWN
  source_role: COMMUNITY_CORROBORATION
  admissible_as_exact_reference_formula: false
```

This is a genuine evidence gap. Copying Canary/Crystal would be an invented Reference claim.

## 7. Cooldown and mana semantics

Target candidate metadata:

```yaml
mana_cost: 20
spell_cooldown_ms: 1000
primary_group: Healing
group_cooldown_ms: 1000
```

The minimal successful-cast observable may therefore assert:

```text
successful eligible exura cast
-> caster receives healing occurrence
-> mana decreases by 20
-> exura cooldown begins
-> Healing-group cooldown begins
```

Classification: `DERIVED`.

Do **not** infer an internal ordering such as reserve-mana -> heal -> commit -> cooldown. The exact cast-cost/cooldown commit anchor and rollback semantics remain `UNKNOWN` for the target until observed or otherwise evidenced.

## 8. Spell-learning consequence for the first fixture

A normal target-era `unlearned exura` failure must **not** be part of the first playable fixture.

S3 is a direct pre-target production anchor: as of 2026-01-27 ordinary trainer-taught spells unlock automatically at the required level and existing characters automatically receive qualifying spells.

Therefore the first `exura` fixture treats ordinary spell availability as a consequence of vocation + level eligibility, not as an independently mutable learned/unlearned bit.

## 9. Minimal first playable heal fixture

### 9.1 Success scenario

```yaml
fixture_key: blueprint.reference.light_healing.basic_cast.v1
actor:
  vocation: Sorcerer
  promotion: none
  level: 8_or_higher
  magic_level: explicitly_pinned
  account: Free
  hp: below_max
  mana: at_least_20
state:
  exura_cooldown: clear
  healing_group_cooldown: clear
  paralysis: false
  wheel_modifier: none
  vocation_stance_modifier: none
  proficiency_modifier: none
action:
  words: exura
expected:
  recipient: caster
  hp_delta: positive
  exact_hp_delta: PARITY_PENDING_EVIDENCE
  mana_delta: -20
  spell_cooldown: starts_1s
  healing_group_cooldown: starts_1s
```

Sorcerer is selected only as a simple isolation profile. The other three allowed vocations are not claimed to use different base Light Healing metadata. Any target-era vocation-specific bonus/stance state must be pinned or neutralised rather than silently ignored.

### 9.2 Minimal negative/eligibility cases

| Case | Isolated precondition | Expected bounded result | Classification |
|---|---|---|---|
| insufficient mana | mana `19`, otherwise eligible | cast does not produce heal occurrence | `DERIVED` |
| level gate | level `7`, otherwise eligible | Light Healing not eligible | `DERIVED` |
| vocation gate | Knight, level/mana otherwise sufficient | Light Healing not eligible | `DERIVED` |
| own cooldown | exura cooldown active, group otherwise clear | cast blocked | `DERIVED` |
| group cooldown | Healing-group cooldown active, exura-specific otherwise clear | cast blocked | `DERIVED` |
| Free Account | normal success case | must remain eligible | `DERIVED` |

The first fixture should violate exactly one eligibility gate at a time. It should not claim simultaneous-error precedence.

### 9.3 Explicitly excluded from first fixture

- learned/unlearned ordinary-spell state;
- full-HP cast behavior;
- dead-caster behavior;
- PvP / PZ interactions;
- external targeting / range / LoS / floor legality;
- paralysis removal;
- literal client error strings as target parity oracles;
- combinations of multiple simultaneous failures;
- Wheel/stance/proficiency-modified healing;
- exact heal min/max/RNG/rounding.

## 10. Conflicts and rejected candidates

### C1 — stale community metadata

Older community pages can still expose level/vocation/trainer-era values that conflict with the June-2026 structured revision and current official Library.

Disposition: stale historical/community evidence; do not convert it into target `CONFLICT` where stronger target-near + official agreement exists.

### C2 — legacy Light Healing coefficients

The old `1.4 / 1.795 + 8 / 11` formula is explicitly marked obsolete after 2020 by its own community formula source.

Disposition: historical observation only.

### C3 — OTS disagreement in contemporary level handling

Canary and Crystal share historical Light Healing coefficients but differ in their level contribution implementation. This demonstrates why OTS code cannot close Global rounding or exact formula semantics.

Disposition: `OTS_HYPOTHESIS_ONLY`.

### C4 — Base Power interpretation

Official material defines Base Power as a comparison metric; community reverse engineering proposes `/25` for an average ML multiplier.

Disposition: useful hypothesis; exact target formula remains `UNKNOWN`.

## 11. Final continuity packet

```yaml
entity: reference.ability.light_healing
target_cut: 2026-07-28
fields:
  identity:
    value: Light Healing
    classification: DERIVED
    continuity_to_target: DERIVED
  invocation:
    value: exura
    classification: DERIVED
    continuity_to_target: DERIVED
  qualitative_self_heal:
    value: caster_self_heal
    classification: DERIVED
    continuity_to_target: DERIVED
  vocation_level_mana_premium:
    value:
      vocations: [Druid, Monk, Paladin, Sorcerer]
      min_level: 8
      mana: 20
      premium: false
    classification: DERIVED
    continuity_to_target: DERIVED
  cooldown:
    value:
      spell_ms: 1000
      group: Healing
      group_ms: 1000
    classification: DERIVED
    continuity_to_target: DERIVED
  base_power:
    value: 40
    classification: DERIVED
    continuity_to_target: DERIVED
  exact_heal_min_max:
    classification: UNKNOWN
    continuity_to_target: UNKNOWN
  exact_rng:
    classification: UNKNOWN
    continuity_to_target: UNKNOWN
  exact_rounding:
    classification: UNKNOWN
    continuity_to_target: UNKNOWN
  failed_cast_exact_commit_order:
    classification: UNKNOWN
    continuity_to_target: UNKNOWN
```

## 12. Oteryn consequence

This research supports preparing a manifest-authority promotion candidate for the existing Light Healing aspects:

```text
cast metadata -> DERIVED candidate
qualitative self-heal -> DERIVED candidate
```

It does **not** directly mutate `REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json` and does not justify `PARITY_CONFIRMED`.

The numeric heal oracle remains fail-closed:

```text
exact quantitative healing / RNG / rounding -> UNKNOWN
```

A structural/playable fixture may prove self-heal occurrence + metadata semantics while retaining `exact_hp_delta: PARITY_PENDING_EVIDENCE`.

## 13. One next action

`LIGHT_HEALING_EXACT_NUMERIC_GLOBAL_OBSERVATION_V1`

Run one controlled real-Global measurement package with a neutral, fully recorded target-era-compatible character profile:

- exact level;
- exact Magic Level;
- exact vocation/profile state;
- no Wheel/stance/proficiency/healing modifiers;
- many repeated injured `exura` casts at at least two Magic Levels and two level bands;
- HP before/after for every cast;
- parallel boundary attempts for mana `19/20` and active cooldown.

The observation must recover or reject, independently of OTS:

- level contribution at the tested boundaries;
- Magic-Level coefficient(s);
- minimum and maximum heal;
- distribution between min/max;
- integer conversion / rounding;
- failed-cast mana/cooldown commit anchors.

Until that new observation exists, further public-source searching should not be used to fabricate an exact Light Healing formula.
