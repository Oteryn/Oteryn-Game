# Oteryn Reference — First Creature Fixture: Rat

- Date: 2026-09-12
- Programme: #486
- Reference evidence tracker: #483
- Combat resource gate: #506
- Durability item-transaction gate: #513
- Control plane: #162
- Reference target: `global-tibia-observable-2026-07-28-post-server-save`
- Scope: read-only Reference evidence consolidation; no runtime/manifest/registry mutation
- Selected creature: `Rat`

## 1. Decision

`Rat` is the recommended first creature-content fixture for the minimal Reference `kill -> XP -> corpse -> loot` path.

This supersedes `Skeleton` only as the recommended research fixture candidate. It does **not** mutate the accepted Reference parity manifest and does not claim `PARITY_CONFIRMED`.

Reason for selection:

- very low complexity (`20 HP`, `5 base XP`);
- one ordinary physical melee attack family;
- useful non-neutral elemental profile for later attack-ability composition;
- semantic corpse family `Dead Rat`;
- very small ordinary loot candidate family (`Gold Coin`, `Cheese`);
- structured target-near revision dated 2026-07-25, three days before the immutable 2026-07-28 cut.

The earlier #483 Skeleton candidate remained target-continuity pending. Rat has stronger target-near structured continuity for the exact static fields needed here.

## 2. Evidence classification discipline

Per `docs/agents/programs/OTERYN_REFERENCE_INVESTIGATION_SOURCE_REGISTRY_20260910.md`, structured-reference data does not become `PROVEN` merely because it is stable or corroborated.

Therefore the Rat static fields below are classified conservatively as:

- `DERIVED` with high confidence where target-near structured evidence and continuity reasoning agree;
- `UNKNOWN` where exact target-era proof is missing;
- no OTS value is promoted to Global Reference truth.

## 3. Atomic evidence

| Atomic field | Target candidate | Classification | Confidence | Continuity to 2026-07-28 |
|---|---:|---|---|---|
| creature | `Rat` | `DERIVED` | HIGH | `DERIVED` |
| HP | `20` | `DERIVED` | HIGH | `DERIVED` |
| base XP | `5` | `DERIVED` | HIGH | `DERIVED` |
| physical damage taken | `100%` | `DERIVED` | HIGH | `DERIVED` |
| earth damage taken | `80%` | `DERIVED` | HIGH | `DERIVED` |
| fire damage taken | `100%` | `DERIVED` | HIGH | `DERIVED` |
| death damage taken | `110%` | `DERIVED` | HIGH | `DERIVED` |
| energy damage taken | `100%` | `DERIVED` | HIGH | `DERIVED` |
| holy damage taken | `80%` | `DERIVED` | HIGH | `DERIVED` |
| ice damage taken | `110%` | `DERIVED` | HIGH | `DERIVED` |
| basic attack | physical melee `0..8` | `DERIVED` | HIGH | `DERIVED` |
| low-HP behaviour | flees at low HP | `DERIVED` | HIGH | `DERIVED` |
| exact flee threshold | unasserted | `UNKNOWN` | — | `UNKNOWN` |
| corpse family | `Dead Rat` | `DERIVED` | HIGH | `DERIVED` |
| ordinary loot candidate types | `Gold Coin`, `Cheese` | `DERIVED` | HIGH | `DERIVED` |
| Gold Coin source quantity notation | `0..4` | `DERIVED` | HIGH | `DERIVED` |
| contextual raid loot | `Silver Raid Token` | `DERIVED` | HIGH | excluded from ordinary fixture |
| loot probabilities / RNG distribution | unasserted | `UNKNOWN` | — | intentionally unresolved |
| numeric corpse item ID | unasserted | `UNKNOWN` / out of scope | — | must not be inferred from OTS |

## 4. Exact external sources

### CipSoft / official

1. Tibia Library — Creatures catalogue
   - `https://www.tibia.com/library/?subtopic=creatures`
   - Role: `PRIMARY_OFFICIAL`
   - Use: official creature catalogue / terminology anchor.
   - Limitation: the exact `race=rat` page could not be directly retrieved in the research environment, so no numeric Rat field is upgraded to `PROVEN` from the official Library alone.

2. CipSoft — 2026-07-28 `Balancing, Fixes and Changes`
   - `https://www.tibia.com/news/?id=8905&subtopic=newsarchive`
   - Role: `PRIMARY_OFFICIAL`
   - Use: exact immutable target-boundary change notice.
   - Relevant continuity point: the note explicitly names numerous creature/boss/XP/HP changes effective at the selected server-save boundary and does not identify Rat as changed.
   - Limitation: absence from a patch note is not absolute proof that no undocumented field changed; it supports `DERIVED`, not `PROVEN`, continuity.

### Structured Reference data

3. TibiaWiki BR — Rat revision, `oldid=441001`, dated 2026-07-25
   - `https://www.tibiawiki.com.br/index.php?oldid=441001&title=Rat`
   - Role: `STRUCTURED_REFERENCE_DATA`
   - Provides the target-near Rat static record used here: HP, XP, elemental modifiers, melee `0-8`, low-health flee behaviour and ordinary/raid loot separation.

4. TibiaWiki BR — previous-to-`441001` diff
   - `https://www.tibiawiki.com.br/index.php?diff=prev&oldid=441001&title=Rat`
   - Role: `STRUCTURED_REFERENCE_DATA`
   - Use: continuity reasoning. The 2026-07-25 edit changes mitigation while the fixture-relevant HP/XP/elements/attack/behaviour/loot fields remain stable across that diff.

5. TibiaWiki BR — current Rat page
   - `https://www.tibiawiki.com.br/wiki/Rat`
   - Role: `STRUCTURED_REFERENCE_DATA`
   - Use: post-target structured cross-check only; current data does not independently prove the target cut.

6. TibiaWiki BR — Dead Rat structured record
   - `https://www.tibiawiki.com.br/index.php?stableid=45839&title=Dead_Rat`
   - Role: `STRUCTURED_REFERENCE_DATA`
   - Use: semantic corpse family `Dead Rat`; associated with Rat/Cave Rat/Munster. First-stage volume/weight/decay details are useful secondary data but not required by the first creature record.

## 5. Target continuity

The strongest static anchor is the Rat structured revision dated `2026-07-25`, three days before the immutable Reference cut.

Continuity chain:

```text
TibiaWiki Rat revision 2026-07-25
  -> exact diff shows the fixture-relevant static fields stable in that revision
  -> CipSoft 2026-07-28 target-boundary balance/fix note does not identify Rat as changed
  -> target cut 2026-07-28 post-server-save
```

Classification:

```yaml
continuity_to_target: DERIVED
confidence: HIGH
```

This is intentionally not `PROVEN`: no target-era primary CipSoft Rat record or controlled 2026-07-28 Global observation was obtained for the entire tuple.

## 6. Relevant resistance / weakness profile

Reference candidate:

```yaml
damage_taken:
  physical: 1.00
  earth: 0.80
  fire: 1.00
  death: 1.10
  energy: 1.00
  holy: 0.80
  ice: 1.10
```

Relevant non-neutral relationships:

- resistant to Earth: `80%` damage taken;
- resistant to Holy: `80%`;
- weak to Death: `110%`;
- weak to Ice: `110%`.

This makes Rat useful for later composition with the first Ice Strike Reference case without making Ice Strike evidence part of this creature record.

## 7. Basic attack / retaliation behaviour

Minimum creature-side attack contract:

```yaml
attack:
  type: melee
  damage_type: physical
  listed_damage_range:
    min: 0
    max: 8

behaviour:
  melee_combat: true
  flees_at_low_hp: true
  exact_flee_threshold: UNASSERTED
```

The exact flee threshold is intentionally not frozen. Contemporary structured community data may suggest a concrete threshold, but the target-near record only safely supports the qualitative `flees at low HP` statement.

## 8. Corpse family

Reference semantic corpse family:

```yaml
corpse:
  family: Dead Rat
```

Do not encode a numeric corpse/item ID from Canary, Crystal, TFS or another OTS. Different item mappings can use different numeric IDs and are not Global Reference authority.

The original creature-fixture scope requires the corpse family, not a protocol/content numeric identifier.

## 9. Minimal ordinary loot candidate set

For an ordinary, non-raid Rat:

```yaml
ordinary_loot:
  candidate_item_types:
    - Gold Coin
    - Cheese

  gold_coin_source_quantity_notation:
    min: 0
    max: 4

  probabilities: UNASSERTED
```

Contextual exclusion:

```yaml
excluded_contextual_loot:
  - item: Silver Raid Token
    reason: raid/invasion-only context in the target-near structured record
```

Correct first-fixture assertion:

```text
generated ordinary Rat loot item types
must be a subset of {Gold Coin, Cheese}
```

Incorrect assertions:

- Rat always drops Gold Coin;
- Rat always drops Cheese;
- any numeric Gold Coin/Cheese drop probability;
- OTS loot percentages as Global truth.

No loot probability is guessed in this evidence case.

## 10. Required fixture preconditions

The creature record must not accidentally absorb global reward modifiers that are not Rat properties.

Recommended baseline preconditions:

```yaml
fixture_environment:
  players: 1
  party: false
  pvp: false

  ordinary_spawn: true
  raid: false
  event_context: false
  boosted_creature: false

  stamina:
    above_14_hours: true

  weapon_proficiency_effects_affecting_case: none
  charms_affecting_case: none
```

Stamina is explicit because #483 evidence already separates ordinary reward behaviour from the `<=14h` stamina loot-suppression/XP state. Boosted-creature state is also excluded because boosted creatures alter XP/loot and would contaminate the base creature fixture.

## 11. Final Reference creature record

```yaml
fixture_id: reference.first_creature.rat
reference_target: global-tibia-observable-2026-07-28-post-server-save

creature:
  name: Rat

  hp: 20
  base_xp: 5

  damage_taken:
    physical: 1.00
    earth: 0.80
    fire: 1.00
    death: 1.10
    energy: 1.00
    holy: 0.80
    ice: 1.10

  attack:
    type: melee
    damage_type: physical
    listed_damage_range: [0, 8]

  behaviour:
    flees_at_low_hp: true
    exact_flee_threshold: UNASSERTED

  corpse:
    family: Dead Rat

  ordinary_loot:
    candidate_item_types:
      - Gold Coin
      - Cheese
    gold_coin_source_quantity_notation: [0, 4]
    probabilities: UNASSERTED

  excluded_contextual_loot:
    - Silver Raid Token
```

## 12. Boundary with #506 / #513

This creature evidence supplies static creature inputs only.

It does **not** decide:

- kill attribution;
- XP recipient/ownership/distribution;
- party/shared XP;
- stamina/boost arithmetic beyond fixture precondition selection;
- one-death occurrence identity/replay semantics;
- first-10-second corpse access authority;
- corpse immovability authority;
- loot-selection RNG algorithm/probabilities;
- durable ItemInstance ownership;
- pickup transaction authority;
- protocol IDs / SQL schema.

Those remain under their existing owners:

- #506 / VSL-COMBAT for death occurrence, corpse projection, loot-selection intent and XP workflow composition;
- #513 / DUR-03 for durable item materialization and pickup custody transfer;
- GAME-CHAR/DUR-02 for persistent XP/progression consequences;
- GAME-ITEM for item definition/container/equipment legality.

## 13. Relationship to existing #483 evidence

The protected #483 evidence pack previously used ordinary Skeleton as a low-complexity research candidate but explicitly kept its target HP/XP/loot continuity unresolved.

Rat is recommended as the improved first creature-content evidence case because the 2026-07-25 structured revision directly narrows that continuity gap while keeping the fixture substantially simpler than target-date Summer Update creatures whose XP is directly official but whose full HP/AI/loot tuple is incomplete.

This document does not mutate `REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json`.

## 14. Remaining unknowns

No unresolved field blocks the **original FIRST CREATURE FIXTURE selection/research scope**.

Three fields remain deliberately unasserted:

1. exact target-era flee threshold — `UNKNOWN`;
2. loot probabilities / RNG distribution — `UNKNOWN BY REQUIREMENT`;
3. numeric `Dead Rat` item ID — `UNKNOWN / OUT OF ORIGINAL SCOPE`, to be resolved only from the correct target content/item mapping if implementation later requires it.

Evidence-strength caveat:

- HP/XP/resists/attack/corpse/loot are `DERIVED HIGH`, not `PROVEN`, because the exact target tuple is not backed by a directly retrievable target-era CipSoft Rat record or controlled Global observation.

## 15. Recommended control-plane action

```yaml
recommended_control_plane_action:
  action: REGISTER_OR_REVIEW_FIRST_CREATURE_EVIDENCE_CASE
  candidate: Rat
  evidence_strength: DERIVED_HIGH
  mutate_manifest_from_this_document: false
  implementation_authority_from_this_document: none
```

The #162/#483 authority path may now decide whether to register/promote a small Rat creature evidence case. Any promotion must preserve the explicit `DERIVED`/`UNKNOWN` boundaries above and must not manufacture `PARITY_CONFIRMED` without stronger admissible evidence.

## 16. Completion status

```yaml
programme: 486
lane: combat
task: FIRST_CREATURE_FIXTURE
selection: Rat

required_scope:
  hp: closed
  xp: closed
  relevant_resist_weakness: closed
  basic_attack_retaliation: closed
  corpse_family: closed
  minimal_loot_candidate_set: closed
  continuity_to_2026_07_28: closed_as_DERIVED_HIGH

material_conflicts: none_remaining_for_required_scope
loot_probabilities_guessed: false
ots_promoted_to_reference_truth: false
runtime_or_manifest_mutation: false

research_status: COMPLETE
parity_status: NOT_PROMOTED_BY_THIS_DOCUMENT
```
