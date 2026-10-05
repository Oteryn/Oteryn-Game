# Shards of a Broken Moon — reconstruction evidence packet

Task: `SHARDS-RECON-0`  
Mode: evidence-only / no runtime mutation  
Base: `fc3db9abfb4ae05c13b0264752cfbbca13a276e6`  
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

The four ritual positions at `33868/33880 × 32664/32676, z=9` are separate physical tiles. Their current map appearance id is `54637`; current Oteryn appearance routing only classifies that id as `Terrain:ground_or_border`, which does not yet provide the quest ritual semantics.

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
| s7 | Javala grants lab/stone-door access | Javala NPC + Dialogue candidate | `Stone door key` is only a quest-level semantic label; no canonical source Item identity found |
| s8 | inspect lab note, Crystal Plinth, Large Crystal Apparatus | Crystal Plinth `i54515`; Crystal Apparatus `i53514`, both WorldObject-routed | lab note identity + explicit inspect/use transitions |
| s9 | Niral explains Rakesh / sealing | Niral NPC + Dialogue candidate | exact branch + progress write |
| s10 | kill Rakesh Moonfang | Creature `oteryn:creature.rakesh_moonfang` with abilities/Behavior | encounter admission + kill-credit → quest transition |
| s11 | Moon Mirror + Empty Crystal Flask at blue lava | Moon Mirror `i25975`; Empty Flask `i53696`; Blue Lava Flask `i54564`; volcano position exists | materialization/use-transform; night guard unresolved by source conflict |
| s12 | Niral consecrates blue lava | Niral dialogue content exists; Blessed Flask `i54566` identity exists | exact branch + item transform `54564 → 54566` |
| s13 | draw four runes by four crystal constructs | 4 physical ritual tiles present; Blessed Flask identity; Crystal Apparatus WorldObject | four explicit use/world-object transitions; `54637` needs quest semantics |
| s14 | The Moonsnow Magnolia encounter | Magnolia + Furious Jaracal Creature/Behavior/Ability records exist | complete special Encounter: Jaracal mechanics, Bone Fiddle, Moonsilver Drift, heal-on-Death, revive/phase 2, permanent kill credit |
| s15 | post-boss ice-prison escape | Icicle Chisel `i39578` canonical identity; Lit Torch donor `54610` aliases canonical lit torch `i34017` | split into skeleton→torch, icicles→chisel, wall/fire/chisel transitions, passage/rope exit |
| s16 | return to Saraki / complete | stage graph exists | completion reducer + reward delivery + durable persistence |

## Creature/runtime readback

At PR #1807 head `a3bd46170857054a8f5dd297df2f8e535ccaeefe`:

- `oteryn:creature.rakesh_moonfang` exists with four abilities and normal combat Behavior.
- `oteryn:creature.the_moonsnow_magnolia` exists with five abilities, normal combat Behavior and ordinary Frost Flower Asura / Midnight Asura summoning.
- `oteryn:creature.furious_jaracal` exists with melee + mana-drain behavior.
- none of the three records has an `encounters[]` association for Shards.
- no Magnolia-specific Encounter definition was found.
- no Shards QuestState record exists on that head.

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
- permanent death moves the player into the post-boss escape sequence.

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
- Lit Torch Summer donor id: `54610`, held as probable alias of canonical `oteryn:item.tibia.i34017`
- Crystal Apparatus: `oteryn:item.tibia.i53514` → WorldObject
- Crystal Plinth: `oteryn:item.tibia.i54515` → WorldObject
- Moonsilver Drift: `oteryn:item.tibia.i54235` → WorldObject
- Amati's Echo: `oteryn:achievement/amati_s_echo@1`, staticdata source id 594, 4 points, premium
- Skewered Fish donor id `54638`; qualified profile is `tool`

The current authored completion candidate still reports several of these as unresolved because exact identity availability and executable materialization/use are separate concerns.

Jaracal mount / `Six Steps Ahead` are downstream taming outcomes and must not be granted directly by quest completion.

The quest also unlocks the True Feverbloom hunting route. Current quest completion tooling explicitly rejects treating a free-text Area identity as an executable access capability. The final implementation therefore needs a real access predicate/capability binding.

## Source conflict: Blue Lava night requirement

Current authored recipe carries a night requirement from Fandom-derived evidence.

Current TibiaWiki BR instructions require Moon Mirror + Empty Crystal Flask at the blue lava but do not state a night gate.

The supplied walkthrough narrator mentions the night claim as uncertain.

Status: **SOURCE_CONFLICT**.

Do not hard-code a night-only runtime predicate until the pinned Fandom witness is re-read and source policy resolves the conflict.

## Required implementation slices

```text
SHARDS-Q1
  QuestState + NPC dialogue/progress transitions

SHARDS-I1
  item admission/materialization + use/transform bindings
  tide markers + Refiner + lab objects + ritual tiles + prison escape

SHARDS-E1
  Rakesh admission/kill-credit
  Magnolia special Encounter, phase mechanics and exit

SHARDS-R1
  completion reducer
  Skewered Fish delivery
  Amati's Echo grant
  True Feverbloom access capability
  relog/restart / duplicate-reward qualification
```

`SHARDS-W1` is only needed for placement exceptions discovered during implementation; the audited route itself is already physically present in the exact Summer world pin.

## Final acceptance target

The quest is not complete until a real character can:

```
start
→ complete all 16 authored stages through native events
→ kill Rakesh with credit
→ execute the four-rune ritual
→ complete the real Magnolia two-phase encounter
→ escape the ice prison
→ return/complete
→ receive direct rewards exactly once
→ retain quest/reward/access state across relog and server restart
```

No mutation under `content/**`, `apps/game-server/**` or currently owned #1804/#1807 paths is part of this evidence packet.
