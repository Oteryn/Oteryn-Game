# Oteryn Reference — Ice Strike evidence audit — 2026-09-12

- Status: **READ-ONLY EVIDENCE RESULT / NO PARITY PROMOTION**
- Repository admission: `main@1a9cb71f424a821633fd42f8a1a19920ffeff2c3`
- Reference target: `global-tibia-observable-2026-07-28-post-server-save`
- Scope: first Reference Ice Strike case only
- Related control/evidence issues: #483, #508, #514
- Runtime/client/protocol/content/manifest authority: **NONE**
- Implementation authority: **NONE**
- Global target change authority: **NONE**

## 1. Purpose and hard boundary

This document records the completed bounded research pass for the first Oteryn Reference `Ice Strike` case (`exori frigo`). It does not implement the spell, modify the accepted Reference manifest, or promote any field above the evidence strength supported by the accepted Oteryn evidence rules.

The immutable target remains production-observable Global Tibia behavior after the 2026-07-28 server-save/maintenance boundary. A current official page does not automatically prove the historical cut. Patch-note silence is not continuity proof. Community material is corroboration/discovery evidence. Canary/TFS/other OTS implementations are `OTS_HYPOTHESIS_ONLY`.

The repository's existing manifest revision 3 therefore remains authoritative until a separately allocated and reviewed manifest update occurs. In particular, this audit **does not override** the current fail-closed status of:

- `ability_combat.ice_strike.cast_metadata.v1`;
- `ability_combat.ice_strike.targeted_ice_damage_semantics.v1`.

Both remain target-evidence/parity gated under the accepted manifest.

## 2. Classification vocabulary used here

The evidence labels are the accepted Reference classes:

- `PROVEN` — directly established for the claimed proposition by admissible evidence;
- `OBSERVED` — directly observed in a source/capture, but not necessarily sufficient to prove the immutable target cut;
- `DERIVED` — inferred from a verified evidence chain, with assumptions/boundaries stated;
- `UNKNOWN` — evidence is insufficient for the target proposition;
- `CONFLICT` — qualified evidence materially disagrees.

`continuity_to_target` below is a descriptive audit field, not a new manifest enum. Values such as `DIRECT`, `STRONG_DERIVED`, `COMMUNITY_PRE_TARGET_ONLY`, `CURRENT_ONLY`, and `NONE` describe how well the source chain reaches the immutable 2026-07-28 target.

## 3. Source hierarchy and exact locators

### 3.1 Official / primary

1. **Current official Ice Strike Library**
   - `https://www.tibia.com/library/?spell=icestrike&subtopic=spells`
   - Current indexed fields observed in this research: `exori frigo`; Druid/Sorcerer; Attack; Instant; Ice; cooldown `2s`; group cooldown `2s`; level `8`; mana `20`; Premium `no`; qualitative aimed-opponent/close-range/ice-damage/magical-ability wording.

2. **Free Account Expansion — 2024-10-07**
   - `https://www.tibia.com/news/?id=8076&subtopic=newsarchive`
   - Announces that Ice Strike becomes available to Free Account players for both Sorcerer and Druid with the following server save.

3. **A New Helping Hand II — 2025-10-15**
   - `https://www.tibia.com/news/?id=8543&subtopic=newsarchive`
   - States that Flame Strike and Ice Strike can be learnt starting at level `8`; release scheduled for 2025-10-21.

4. **Changes, Balancing and Fixes — 2026-01-27**
   - `https://www.tibia.com/news/?id=8675&subtopic=newsarchive`
   - States that spells become automatically unlocked while remaining unlocked at the **same level as before**.

5. **Target boundary — 2026-07-28**
   - `https://www.tibia.com/news/?id=8905&subtopic=newsarchive`
   - Boundary-date production change record used by the broader Reference programme.
   - Omission of Ice Strike is **not** used as proof of no change.

### 3.2 Structured community corroboration

6. **TibiaWiki BR — Ice Strike**
   - `https://www.tibiawiki.com.br/wiki/Ice_Strike`
   - Structured current/community fields: level `8`; mana `20`; cooldown `2s/2s`; Base Power `45`; Magic Level scaling; front-square fallback when no target is selected; listed Weapon Proficiency interactions.
   - Stable result exposes historical revision identity `oldid=436044`, providing a pre-target community snapshot lineage.

7. **TibiaWiki BR — Fórmula**
   - `https://www.tibiawiki.com.br/index.php?stableid=106341&title=F%C3%B3rmula`
   - Explicitly states that spell/rune formulas are based on **observed values after Update 8.1**.
   - Strike family candidate:
     - minimum `(lvl*0.2) + (mlvl*1.403) + 8`;
     - maximum `(lvl*0.2) + (mlvl*2.203) + 13`.
   - States that `lvl*0.2` is always rounded down.
   - This is community observation, not a published CipSoft algorithm.

8. **TibiaWiki/Fandom — Ice Strike**
   - `https://tibia.fandom.com/wiki/Ice_Strike`
   - Corroborates candidate range `3` and no-target front-square fallback.

### 3.3 OTS hypothesis only

9. **OpenTibiaBR Canary — Ice Strike**
   - Repository: `opentibiabr/canary`
   - Inspected revision: `06c73ccb993b4625a966d452b7e9f867873d1f55`
   - File: `data/scripts/spells/attack/ice_strike.lua`
   - Candidate fields in that OTS implementation:
     - `min = (level / 5) + (maglevel * 1.403) + 8`;
     - `max = (level / 5) + (maglevel * 2.203) + 13`;
     - mana `20`;
     - range `3`;
     - target-or-direction;
     - `blockWalls(true)`;
     - cooldown/group cooldown `2s`.
   - The same file also contains `level(15)` and `isPremium(true)`, which conflict with stronger official Global evidence. Therefore it cannot independently establish any Global target behavior.

10. **Canary spell/map numeric plumbing**
    - `src/lua/functions/lua_functions_loader.hpp`
    - `src/creatures/combat/combat.cpp`
    - `src/creatures/combat/spells.cpp`
    - `src/map/map.cpp`
    - `src/utils/tools.cpp`
    - These files were inspected only to understand one OTS interpretation of rounding, geometry and RNG. They are test-design hypotheses, not Global evidence.

### 3.4 Source not admitted

11. **Tibiopedia.pl**
    - Direct structured retrieval was blocked by the site's `robots.txt` in this research environment.
    - No formula, range, rounding or legality fact is attributed to Tibiopedia without direct verification.

## 4. Atomic field evidence

| Atomic field | Target class | Evidence result | continuity_to_target | Notes |
|---|---|---|---|---|
| target identity | `PROVEN` | immutable `global-tibia-observable-2026-07-28-post-server-save` | `DIRECT` | Accepted Oteryn Reference target. |
| current case state | `PROVEN` | existing Ice Strike manifest cases remain fail-closed/pending | `DIRECT` | This audit does not mutate manifest revision 3. |
| name / incantation `Ice Strike` / `exori frigo` | `UNKNOWN` target; `OBSERVED` current official | Current official Library directly exposes the pair. | `CURRENT_ONLY` plus community pre-target corroboration | No target-day primary capture. |
| vocation Druid/Sorcerer | `DERIVED` candidate | Official 2024 Free Account notice names Ice Strike for both vocations; current Library matches. | `STRONG_DERIVED` | Strong pre/post chain, not literal target capture. |
| level `8` | `DERIVED` candidate | Official 2025 change to level 8; 2026-01-27 says unlock levels remain as before; current Library still 8. | `STRONG_DERIVED` | Stronger than post-target current-page evidence alone, but not auto-promoted here. |
| Premium / Free eligibility | `DERIVED` candidate | Official 2024 Free Account expansion includes Ice Strike; current Library says Premium `no`. | `STRONG_DERIVED` | Strong official pre/post bracket. |
| mana `20` | `UNKNOWN` target; `OBSERVED` current/community | Current official and structured community agree. | `CURRENT_PLUS_COMMUNITY_PRE_TARGET` | No dated CipSoft target bridge found. |
| spell cooldown `2s` | `UNKNOWN` target; `OBSERVED` current/community | Current official and structured community agree. | `CURRENT_PLUS_COMMUNITY_PRE_TARGET` | No dated CipSoft target bridge found. |
| Attack group cooldown `2s` | `UNKNOWN` target; `OBSERVED` current/community | Current official and structured community agree. | `CURRENT_PLUS_COMMUNITY_PRE_TARGET` | No dated CipSoft target bridge found. |
| group/type/magic family = Attack/Instant/Ice | `UNKNOWN` target; `OBSERVED` current official | Current official Library. | `CURRENT_ONLY` | Qualitatively stable-looking, but target rule remains conservative. |
| aimed single opponent semantic | `UNKNOWN` target; `OBSERVED` current official | Official prose says a freezing arrow hits an aimed opponent in close range. | `CURRENT_ONLY` | Does not fully define target cardinality/selection edges. |
| magical-ability dependence | `UNKNOWN` target; `OBSERVED` current official | Official prose says damage is determined by caster magical abilities. | `CURRENT_ONLY` | Does not publish coefficients or rounding. |
| Base Power `45` | `UNKNOWN` target; `OBSERVED` community | TibiaWiki structured field. | `COMMUNITY_PRE_TARGET_ONLY` | Useful for test design, not primary proof. |
| minimum formula coefficient | `UNKNOWN` target; `DERIVED` candidate | `(lvl*0.2)+(mlvl*1.403)+8`. | `COMMUNITY_PRE_TARGET_ONLY` | Community formula explicitly based on observations. |
| maximum formula coefficient | `UNKNOWN` target; `DERIVED` candidate | `(lvl*0.2)+(mlvl*2.203)+13`. | `COMMUNITY_PRE_TARGET_ONLY` | Community formula explicitly based on observations. |
| level-term rounding | `UNKNOWN` target; `OBSERVED` community | `lvl*0.2` is rounded down. | `COMMUNITY_PRE_TARGET_ONLY` | Only this component is explicitly described. |
| full min/max endpoint rounding | `UNKNOWN` | No admissible source resolves floor/trunc/nearest for the complete formula. | `NONE` | Must not infer from OTS implementation convenience. |
| RNG distribution between endpoints | `UNKNOWN` | No admissible Global source found. | `NONE` | Uniform vs weighted/normal and endpoint inclusion remain unknown. |
| candidate numeric range | `UNKNOWN` target; `OBSERVED` community | `3` squares/fields in TibiaWiki/Fandom. | `COMMUNITY_CORROBORATION` | Official Library says only `close range`. |
| exact range metric | `UNKNOWN` | `range=3` does not prove Manhattan/Chebyshev/other geometry. | `NONE` | Requires controlled geometry observation. |
| line-of-sight / wall blocking | `UNKNOWN` | No target-qualified official/controlled evidence found. | `NONE` | Canary `blockWalls(true)` is hypothesis only. |
| same-floor / cross-floor legality | `UNKNOWN` | No target-qualified official/controlled evidence found. | `NONE` | Canary rejects cross-floor in its instant-spell path; hypothesis only. |
| no selected target -> one front sqm | `UNKNOWN` target; `OBSERVED` community | Structured community sources describe front-square fallback. | `COMMUNITY_PRE_TARGET_ONLY` | Official current prose covers aimed-opponent path, not this fallback. |
| elemental damage family = Ice | `UNKNOWN` target; `OBSERVED` current official | Current official Library. | `CURRENT_ONLY` | Baseline test should freeze all element-changing modifiers. |
| sensitivity qualitative effect | `UNKNOWN` target; `OBSERVED` current official | Official prose says ice-sensitive/fire-dominated creatures are wounded badly. | `CURRENT_ONLY` | Exact multiplier/order remains unknown. |
| resistance/immunity arithmetic | `UNKNOWN` | No exact target formula/order found. | `NONE` | Keep neutral target in first numeric fixture. |
| self/PZ/PvP/secure-mode failure precedence | `UNKNOWN` | General combat rules are insufficient for exact Ice Strike failure semantics. | `NONE` | Exclude from first ordinary PvE fixture. |
| mana commit on failed range/LoS/floor cast | `UNKNOWN` | No admissible evidence found. | `NONE` | Must be measured with geometry legality. |
| cooldown commit on failed range/LoS/floor cast | `UNKNOWN` | No admissible evidence found. | `NONE` | Must be measured with geometry legality. |

## 5. Damage formula — exact boundary of the evidence

The strongest working hypothesis for a plain Strike-family numeric fixture is:

```text
raw_min = floor(level * 0.2) + magic_level * 1.403 + 8
raw_max = floor(level * 0.2) + magic_level * 2.203 + 13
```

Equivalent level term:

```text
floor(level / 5)
```

**Important:** the `floor` above is justified only for the `level*0.2` component by the cited community formula description. The evidence does **not** justify writing either of the following as Global truth:

```text
floor(floor(level/5) + magic_level*1.403 + 8)
floor(floor(level/5) + magic_level*2.203 + 13)
```

or any other complete-endpoint rounding rule.

The formula page explicitly says its spell formulas are based on observed values. Therefore the coefficients are a strong empirical candidate, not a published CipSoft server formula.

### 5.1 Canary interpretation is not Reference truth

The inspected Canary implementation returns the same floating-point coefficient expressions. Its Lua/native plumbing converts numeric values to integral combat values and its RNG implementation uses `normal_random`, implemented as a bounded/truncated normal-style distribution around the interval midpoint with `std::lround`.

This is useful because it exposes test questions:

- are final endpoints truncated/floored/rounded by Global?
- is the damage distribution uniform or center-weighted?
- are both endpoints reachable?

It is **not** evidence that Global uses Canary's integer conversion or RNG. The same Canary spell record is demonstrably stale/wrong for stronger official fields (`level=15`, `Premium=true`), so copying its numeric pipeline into Reference would violate the accepted source hierarchy.

## 6. Range, line of sight and floor legality

### 6.1 Candidate range

Structured community documentation consistently presents ordinary Ice Strike as able to hit a selected target up to `3` squares/fields away, with a one-square front fallback when no target is selected.

Classification:

```text
candidate_range_value = 3       -> OBSERVED community candidate
exact_target_range_value        -> UNKNOWN
exact_target_distance_metric    -> UNKNOWN
```

The official current Library saying `close range` corroborates a short-range semantic family but does not numerically establish `3`.

### 6.2 Metric

The OTS implementation uses independent X/Y range bounds when calling its throw/sight helper. On same-floor positions that behaves like a square/Chebyshev envelope rather than a Manhattan radius. This is **OTS_HYPOTHESIS_ONLY**.

A Global/Oteryn Reference fixture must therefore distinguish at least:

```text
(dx,dy) = (3,0)
(dx,dy) = (4,0)
(dx,dy) = (3,3)
(dx,dy) = (2,3)
```

Without those observations, `range=3` is not an executable geometry rule.

### 6.3 LoS

Canary sets `blockWalls(true)` for Ice Strike and then routes target checks through its line-of-sight machinery. No official or controlled target-era proof was found that closes the same rule for Global.

Final target classification:

```text
Ice Strike wall/LoS legality = UNKNOWN
```

### 6.4 Floor legality

The inspected Canary instant-spell path rejects a target on a different `z` and returns separate up/down-stairs failures. This is again only an OTS hypothesis.

Final target classification:

```text
Ice Strike same-floor requirement = UNKNOWN
cross-floor failure semantics = UNKNOWN
```

## 7. Elemental and target edge cases required by the first fixture

The first quantitative Reference proof should deliberately minimize unrelated mechanics.

### 7.1 Required baseline fixture conditions

Use:

- one ordinary attackable PvE creature;
- target with independently evidenced neutral `100%` Ice response for the tested case;
- Druid or Sorcerer with recorded character level and Magic Level;
- no active crit/outgoing-damage modifiers;
- no Weapon Proficiency perk that modifies Ice Strike;
- no vocation stance/synthesis/conversion state that alters the spell element or output;
- no Wheel/augment/charm/prey/other spell-damage modifier unless explicitly measured and frozen;
- no party/PvP/PZ dimension in the baseline numeric proof.

### 7.2 Elemental cases to keep separate

Do not mix these into the base formula fixture:

1. neutral Ice target;
2. separately evidenced Ice-sensitive target;
3. separately evidenced Ice-resistant target;
4. immunity/zero-damage case only after exact resistance semantics are qualified.

Otherwise formula, RNG and elemental mitigation become observationally entangled.

### 7.3 Target-selection cases

Keep separate cases for:

- selected ordinary target;
- no selected target -> front square candidate;
- out-of-range selected target;
- selected target behind a projectile-blocking obstacle;
- target on another floor.

PvP, secure mode, protection zones, summons, bosses, chain/AoE behavior and broad targeting policy are not needed for the first ordinary Ice Strike Reference fixture.

## 8. Target-era modifier contamination

The 2026-07-28 target is not a pre-modern bare-spell environment. Reference measurement must freeze target-era systems that can alter observable spell damage.

At minimum record/neutralize as applicable:

- Weapon Proficiency perks affecting the equipped weapon/spell;
- target-era vocation adjustment state;
- Sorcerer elemental stance/synthesis/conversion state where relevant;
- Wheel/gem/augment effects;
- crit and other outgoing-damage modifiers;
- prey/charm/other creature modifiers;
- target elemental resistance/sensitivity.

A damage trace without those fields cannot safely be used to infer the core Ice Strike formula.

## 9. Conflicts

### C1 — Canary metadata vs stronger official evidence

Inspected Canary Ice Strike definition:

```text
level = 15
Premium = true
```

Stronger official evidence:

- CipSoft 2025-10-15 moved Ice Strike to level `8`;
- CipSoft 2024-10-07 made Ice Strike available to Free Accounts;
- current official Library reports `Exp Lvl: 8`, `Premium: no`.

Classification:

```text
CONFLICT: OTS implementation vs Global official evidence
resolution: OTS loses; retain only as hypothesis/test-discovery input
```

This conflict is the strongest practical reason not to import Canary's formula rounding, geometry or RNG as authoritative Reference behavior.

### C2 — stale community/mirror material

Older community pages can still expose historical level `15` / Premium-only wording. Those pages describe earlier game states and are not evidence of a 2026 target conflict when the chronology is known.

Classification:

```text
historical/stale documentation != target CONFLICT
```

## 10. Continuity assessment

### 10.1 Strongest official continuity fields

The best official pre/post chain currently exists for:

- level `8`;
- Free Account / Premium `no`;
- Druid/Sorcerer eligibility family.

These are **strong DERIVED continuity candidates**, not target-day direct observations.

### 10.2 Current official + pre-target community continuity

The following have strong corroboration but not an admissible affirmative primary bridge sufficient for manifest promotion under the current accepted rules:

- mana `20`;
- cooldown `2s`;
- attack-group cooldown `2s`;
- Base Power `45`;
- Strike coefficient formula;
- no-target front-square behavior;
- candidate range `3`.

### 10.3 Fields with no adequate target bridge

Remain `UNKNOWN`:

- complete min/max endpoint rounding;
- RNG distribution;
- exact range metric;
- LoS/wall legality;
- same-floor/cross-floor legality;
- exact resistance/mitigation ordering;
- exact mana/cooldown commit behavior on failed legality;
- complete PvP/PZ/secure-mode precedence.

### 10.4 Why target-day patch silence cannot close these fields

The accepted Oteryn Reference evidence policy rejects inference from publication silence. A target-day news post describing other production changes does not affirm that Ice Strike fields were unchanged. This audit therefore does not convert `not mentioned on 2026-07-28` into continuity proof.

## 11. Minimum next proof

The smallest missing proof that can materially increase the target evidence class is **not another OTS/code search**.

### 11.1 First missing proof — continuity artifact

Obtain one provenance-cleared, target-relevant artifact that affirmatively bridges/captures Ice Strike at or through the 2026-07-28 boundary, for example:

- preserved timestamped owner capture of the Magical Archive/Spell Library around the target cut;
- a provenance-cleared archival snapshot with enough exact fields and timestamp context;
- a lawful controlled black-box capture whose date/context is target-relevant, plus an accepted continuity chain if captured after target;
- another owner-accepted affirmative evidence chain that proves persistence through the boundary.

This would improve field continuity without pretending that silence proves stability.

### 11.2 Second missing proof — one controlled geometry/numeric session

After continuity is admissible, run one controlled Global observation session with:

```text
caster:
  vocation: Druid or Sorcerer
  level: recorded
  magic_level: recorded
  modifiers: explicitly neutralized/recorded

target:
  ordinary attackable PvE creature
  ice_modifier: independently known neutral

geometry probes:
  (3,0)
  (4,0)
  (3,3)
  (2,3)
  clear path vs projectile-blocking wall
  same z vs z+1 vs z-1

for every cast attempt record:
  caster/target coordinates
  floor
  mana before/after
  cooldown before/after
  effect/projectile observation
  exact damage
  failure message/result
```

Numeric samples should include levels not divisible by five and Magic Levels chosen so the candidate coefficient terms yield fractional endpoints. This can distinguish several endpoint-rounding hypotheses.

A sufficiently large sample can characterize the observed damage distribution, but RNG must remain `UNKNOWN` if the sample cannot statistically distinguish plausible distributions.

## 12. Final disposition

The research task is complete in the sense that every requested field now has an explicit evidence state and the unresolved fields have a precise missing-proof definition.

Do **not** promote the target cases from this document alone.

Final disposition:

```text
ability_combat.ice_strike.cast_metadata.v1
  target: UNKNOWN under current accepted manifest
  notable strong continuity candidates:
    level=8
    Free Account / Premium=no
    Druid/Sorcerer

ability_combat.ice_strike.targeted_ice_damage_semantics.v1
  target: UNKNOWN under current accepted manifest
  current official observations:
    aimed close-range opponent
    Ice damage family
    damage depends on magical ability

exact damage coefficients:
  DERIVED community candidate

level-term floor:
  OBSERVED community

complete endpoint rounding:
  UNKNOWN

RNG distribution:
  UNKNOWN

range=3:
  OBSERVED community candidate
  target exact value/metric: UNKNOWN

LoS:
  UNKNOWN

floor legality:
  UNKNOWN

resistance/mitigation ordering:
  UNKNOWN
```

Repository architecture remains correctly fail-closed: #508 Phase B must not invent geometry/range/LoS/floor behavior, and #514 must not promote exact Ice Strike damage/range/LoS/rounding until admissible evidence closes those fields.

`IMPLEMENTATION_AUTHORITY: NONE`
`MANIFEST_PROMOTION_AUTHORITY: NONE`
`LIVE_DEPLOYMENT_AUTHORITY: NONE`
