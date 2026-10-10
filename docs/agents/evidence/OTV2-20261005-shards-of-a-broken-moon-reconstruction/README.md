# Shards of a Broken Moon â€” reconstruction evidence packet

Task: `SHARDS-RECON-0`
Mode: evidence-only / no runtime mutation

Concrete Q1 owner lowering and acceptance cases: [q1-lowering-owner-plan.md](q1-lowering-owner-plan.md). It uses the current QuestTransitionRequest/writer and locked prerequisite checks; it does not allocate keys or activate content.

Native-loader investigation candidate: [q1-native-catalogue-candidate.json](q1-native-catalogue-candidate.json). Reproduce with `python build_q1_candidate.py --check` and `python run_q1_candidate.py --adversarial` from this packet. [Qualification receipt](q1-catalogue-qualification.json) separates 38 passing pure Quest tests and the deliberately failing missing-guard mutation from still-unqualified durable/runtime composition.

Item core candidate: [i1-item-definitions-candidate.json](i1-item-definitions-candidate.json), built through the existing admission semantics from exact client/XML witnesses. [I1 qualification](i1-core-qualification.json) records positive core outputs and five negative cases. It does not admit missing identity 54610 or modify served content.
Original base: `fc3db9abfb4ae05c13b0264752cfbbca13a276e6`
Current dependency readback: `3bacc59e6adbf138655e6f0c1cf0b4fdf9a36096` (2026-10-07)
Walkthrough: `https://www.youtube.com/watch?v=Zz-7UyufXbU&t=1295s`

## Result

The quest already exists as authored Oteryn content:

- Quest: `oteryn:quest.authored.shards_of_a_broken_moon_quest@authored-r1`
- Source recipe: `tools/content-schema/quest-authoring/samples/authored68/recipes.json`
- Stages: 16
- Current readiness: `waiting_native_bindings`
- Current lowering: `WAITING_IMPLEMENTATION`
- `runtime_enabled=false`
- Current authored-completion candidate has 16 selected transitions for this quest, but no native NPC/event/reward bindings.

The supplied 24:09 walkthrough was inspected visually through its full YouTube storyboard. Auto-captions were used only to disambiguate order/names; they were not treated as a substitute for the visual review.

## Exact world pin and route

Current Summer world source:

```
zimbadev/crystalserver@00ce02a57ca5a12e48f32a3476e37471167e4c3f
data-global/world/world.otbm
bytes: 53134513
sha256: dcb735549bd11de526c4bd441bbf62e4490efbb60ff7aa334530f1692345d8d7
```

A direct OTBM parse of the pinned file found **18/18 audited quest positions present**.

Audited positions:

```
33901,32839,7
33907,32784,7
33900,32765,7
33862,32838,7
33882,32823,7
33774,32853,7
33899,32666,8
33881,32687,9
33874,32686,9
33874,32677,9
33039,32964,8
32971,32981,7
33098,32947,6
33868,32664,9
33880,32664,9
33868,32676,9
33880,32676,9
33901,32718,9
```

The four ritual positions at `33868/33880 Ã— 32664/32676, z=9` are separate physical tiles. Their base appearance `54637` is confirmed as `Terrain:ground_or_border`; it must **not** be promoted to a quest WorldObject. The ritual binds `USE_ITEM_ON_POSITION` to the four exact placements.

Do not use the older Otheryn migration map `3bd40d14...` for this quest. It predates the Summer world additions and produces false absences.

## Stage matrix

| Stage | Walkthrough / intended transition | Existing Oteryn assets | Native gap |
|---|---|---|---|
| s1 | Saraki starts moon investigation | Saraki NPC + Dialogue candidate | exact dialogue branch selection + progress write |
| s2 | Sundara / Yukti clue | Sundara NPC + Dialogue candidate | exact dialogue branch + progress write |
| s3 | Tarisu gives/uses Shell Gauge at three tide markers | Shell Gauge `i53783`; route positions exist | item materialization/use + 3 world-marker bindings |
| s4 | collect Bluish Tide Veil + Bluish Whisper Reed; use Refiner of Magic | quest items `i53692`, `i53693`; Refiner `i34338` routed WorldObject | item acquisition/count + Refiner interaction |
| s5 | report clues to Saraki; ask Udu | Saraki/Udu NPC + Dialogue candidates | exact branches + ordered transition semantics |
| s6 | Sharai gives cave access | Sharai NPC + Dialogue candidate | dialogue/progress + access/door predicate |
| s7 | Javala grants lab/stone-door access | Javala NPC + Dialogue candidate; Asura Citadel Key = `oteryn:item.tibia.i54262` (exact cross-source) | NPC progress + identity-based key-door binding; donor has no `keyNumber`; post-completion bypass needs Door/Key architecture disposition |
| s8 | laboratory / Crystal Plinth / Large Crystal Apparatus | Note Pinned on the Wall = donor `6121` at exact `33881,32684,9` (optional lore); Crystal Plinth `i54515`; Apparatus `i53514` | Plinth directly yields `lab_access_sequence_learned`; no manual colour-code UI/input; sealed-room gate + apparatus interaction still need native trigger/gate adapters |
| s9 | Niral explains Rakesh / sealing | Niral NPC + Dialogue candidate | exact branch + progress write |
| s10 | kill Rakesh Moonfang | Creature `oteryn:creature.rakesh_moonfang` with abilities/Behavior | encounter admission + kill-credit â†’ quest transition |
| s11 | Moon Mirror + Empty Crystal Flask at blue lava | Moon Mirror `i25975`; Empty Flask `i53696`; Blue Lava Flask `i54564`; volcano position exists | materialization/use-transform + source-qualified night-time predicate |
| s12 | Niral consecrates blue lava | Niral dialogue content exists; Blessed Flask `i54566` identity exists | exact branch + item transform `54564 â†’ 54566` |
| s13 | draw four runes by four crystal constructs | 4 exact physical ritual tiles present; Blessed Flask identity; base `54637` is Terrain only | four placement-driven `USE_ITEM_ON_POSITION` transitions; do not create a WorldObject identity for `54637` |
| s14 | The Moonsnow Magnolia encounter | Magnolia + Furious Jaracal Creature/Behavior/Ability records exist | complete special Encounter: Jaracal mechanics, Bone Fiddle, Moonsilver Drift, heal-on-Death, revive/phase 2, permanent kill credit |
| s15 | post-boss ice-prison escape | Icicle Chisel `i39578`; donor `54610` is distinct **Lit Torch (SU26)** and must not alias old `i34017`; exact prison route is resolved | skeleton `31920,31360,9` â†’ torch; icicles `31914,31364,9` â†’ chisel; north wall `31920,31359,9`; Rope Spot `31923,31377,9` â†’ generic rope destination `31923,31378,8`; remaining blocker is canonical admission/materialization of `i54610` plus generic trigger wiring |
| s16 | return to Saraki / complete | stage graph exists | completion reducer + reward delivery + durable persistence |

## Creature/runtime readback

Fresh runtime baseline: `main@492f25c90ebcdb85d8cc8f2d2b273f72017dc8ef` (2026-10-07).

- #1807 is merged; `oteryn:creature.rakesh_moonfang`, `oteryn:creature.the_moonsnow_magnolia` and `oteryn:creature.furious_jaracal` remain canonical Creature content.
- #1891 merged the accepted Magnolia phase-2 rule: first lethal handling prevents death, sets encounter-local `max_health=60000`, then performs an explicit full heal; the max-health write itself never heals.
- #1886 merged the durable-cause rule required by future QUEST-TRIGGER-1.
- no executable `ENC-RT-1` / `ENC-OUTCOME-1` implementation is present yet, so the special fight still has no production Encounter outcome producer into QuestState.
- current native quest bindings remain zero; Shards must consume the generic Gate/Trigger/NPC/Encounter owners rather than add a quest-private runtime.

Crystal Summer itself contains the static monster files, but no dedicated Magnolia/Bone Fiddle/Moonsilver Drift/revive quest encounter script. Therefore the special fight cannot be claimed as a Crystal copy.

## Magnolia mechanics requiring explicit Oteryn implementation

Video + current wiki evidence agree on the following encounter-level behaviors that are not provided by the Crystal monster statblock:

- solo encounter;
- Furious Jaracal add pressure;
- Bone Fiddle interaction suppresses/controls Jaracal accumulation in phase 1;
- Moonsilver Drift is an encounter world object/hazard;
- Death damage heals Magnolia;
- first lethal threshold does not finish the fight: Magnolia performs a full second-life transition;
- Bone Fiddle is no longer used in phase 2;
- permanent death moves the player into the post-boss escape sequence;
- current Global post-release behavior must use the Aug 25 spawn-placement adjustments (Furious Jaracals near noxious catnip; south minions nearer the teleporter) and the Sep 15 phase-2 leave cleanup fix; do not reproduce release-day spawn leakage/placement from the July walkthrough.

Exact cadence/damage values must remain source-qualified. Do not promote narration guesses into Reference facts.

## Important identity / reward facts

Known canonical or qualified identities:

- Sample of Bluish Tide Veil: `oteryn:item.tibia.i53692`
- Sample of Bluish Whisper Reed: `oteryn:item.tibia.i53693`
- Empty Crystal Flask: `oteryn:item.tibia.i53696`
- Shell Gauge: `oteryn:item.tibia.i53783`
- Crystal Flask with Blue Lava: `oteryn:item.tibia.i54564`
- Crystal Flask with Blessed Blue Lava: `oteryn:item.tibia.i54566`
- Moon Mirror: `oteryn:item.tibia.i25975`
- Icicle Chisel: `oteryn:item.tibia.i39578`
- Lit Torch (SU26): donor id `54610`; independent Item-ID evidence distinguishes it from old `Lit Torch (Quest)` id `34017`, so the old probable-alias hold is incorrect for Shards and requires Item-owner admission as a distinct identity
- Crystal Apparatus: `oteryn:item.tibia.i53514` â†’ WorldObject
- Crystal Plinth: `oteryn:item.tibia.i54515` â†’ WorldObject
- Moonsilver Drift: `oteryn:item.tibia.i54235` â†’ WorldObject
- Amati's Echo: `oteryn:achievement/amati_s_echo@1`, staticdata source id 594, 4 points, premium
- Skewered Fish: `oteryn:item.tibia.i54638`; donor id `54638`; current A12 canonical identity (retired alias `i00038475` resolves here), qualified as a taming tool, currently non-materializable

The current authored completion candidate still reports several of these as unresolved because exact identity availability and executable materialization/use are separate concerns.

Jaracal mount / `Six Steps Ahead` are downstream taming outcomes and must not be granted directly by quest completion. Exact pinned donor mount identity is already known from Crystal Summer `data/XML/mounts.xml`: mount id `250`, clientid `1962`, name `Jaracal`, speed `10`, premium `yes`, type `quest`. The current 252-row canonical Mount catalogue lacks this 253rd SU26 record. `Six Steps Ahead` already exists canonically as `oteryn:achievement/six_steps_ahead@1` (source id 592, 2 points, premium). Tame chance/failure semantics remain UNKNOWN.

The quest also unlocks the True Feverbloom hunting route. Current quest completion tooling explicitly rejects treating a free-text Area identity as an executable access capability. The final implementation therefore needs a real access predicate/capability binding.

## Blue Lava night requirement â€” source disposition

The pinned post-release Fandom spoiler revision `1202455` (2026-08-20) explicitly requires taking the Empty Crystal Flask to the active volcano **during the night time** with a Moon Mirror.

Current Fandom content still carries the same explicit night-time instruction.

TibiaWiki BR and post-release player guides describe Moon Mirror + Empty Crystal Flask but do **not** state that daytime use succeeds. Under the project's conflict policy, omission is not an atomic contradiction.

The supplied walkthrough narrator explicitly says the wiki claims night-only, is unsure whether the restriction is truly necessary, believes it is currently night, and then fills the flask successfully. That recording is supporting evidence but does not independently discriminate day from night.

Status: **STRUCTURED_REFERENCE_CANDIDATE / night_time_required**.

This remains unverified by a controlled Global day-vs-night test, but it is no longer classified as `SOURCE_CONFLICT` because no retrieved source positively demonstrates successful daytime filling.

## Required implementation slices

```text
SHARDS-Q1
  QuestState + NPC dialogue/progress transitions

SHARDS-I1
  item admission/materialization + use/transform bindings
  distinct Lit Torch (SU26) 54610 admission
  tide markers + Refiner + lab objects + placement-driven ritual tiles + prison escape

SHARDS-E1
  Rakesh admission/kill-credit
  Magnolia special Encounter, phase mechanics and exit

SHARDS-R1
  completion reducer
  Skewered Fish delivery
  Amati's Echo grant
  Jaracal Mount admission (donor mount 250/clientid 1962) + later oteryn:item.tibia.i54638-on-Jaracal tame flow + existing Six Steps Ahead grant
  True Feverbloom / Forbidden Gardens access predicate (`Shards completed` AND account owns `forbidden_fruit`)
  relog/restart / duplicate-reward qualification
```

`SHARDS-W1` is only needed for placement exceptions discovered during implementation; the audited route itself is already physically present in the exact Summer world pin.

## Final acceptance target

The quest is not complete until a real character can:

```
start
â†’ complete all 16 authored stages through native events
â†’ kill Rakesh with credit
â†’ execute the four-rune ritual
â†’ complete the real Magnolia two-phase encounter
â†’ escape the ice prison
â†’ return/complete
â†’ receive direct rewards exactly once
â†’ retain quest/reward/access state across relog and server restart
```

No mutation under `content/**`, `apps/game-server/**` or currently owned #1804/#1807 paths is part of this evidence packet.

## Fresh consistency pass (2026-10-07)

- #1916 MAP-CUTOVER-1b is merged; bundle serving does not admit Shards USE/USE-WITH or native Quest/NPC/Encounter producers.
- `prison-source-readback-20261007.json` rechecks nine exact donor stacks, the center wall orientation and Rope Spot/destination, with pinned Canary position-rule cross-check. Retained video evidence is identified separately from this fresh static parse.
- `magnolia-encounter-spec.md` maps the two-phase behavior to the current closed Encounter vocabulary and the merged #1891 max-health ruling. Unknown cadence, damage distributions, transformation mapping and spawn coordinates remain held.
- Blue Lava night-only remains SOURCE_CONFLICT, with no runtime guard selected.
- Item 54610 still has no active canonical Item or exact donor binding on this main; i54262/i54638 bindings exist but both Items remain materializable=false.
- No SHARDS-Q1/I1/E1/R1 runtime/content ownership for this lead was found in live #1622. Changes remain path-disjoint evidence only.
