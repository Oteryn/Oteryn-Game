# Shards of a Broken Moon — executable micro-transition plan

Status: **EVIDENCE / PROPOSED BINDING PLAN ONLY**  
No runtime/content authority is granted by this document.

This refines the 16-stage authored recipe after comparing:

- the supplied full walkthrough;
- the pinned/current quest reference material;
- Crystal Summer `00ce02a57ca5a12e48f32a3476e37471167e4c3f`;
- current Oteryn Quest/NPC/Item/WorldObject/Creature infrastructure.

## Why the 16 visible stages cannot be lowered 1:1

The existing authored recipe is a useful Quest Log summary, but it compresses multiple executable facts into single stages and serializes work that the source allows in parallel.

The key corrections are:

1. **Initial investigations are parallel.** Yukti, Tides and Plants may be completed in any order.
2. **Alternative NPC paths exist.** Source material permits alternate start/completion and clue NPCs; they must not be collapsed into a single mandatory NPC merely because the authored summary chose one.
3. Several visible stages require multiple durable sub-facts so that repeated use of one object cannot satisfy a multi-location objective.
4. Several Crystal Summer NPC files contain the correct dialogue prose but perform **no item/progress mutation at all**. Their text is evidence for the conversation, not executable donor logic.
5. The laboratory and Magnolia sections contain intermediate interactions not represented as separate visible stages.

The implementation should preserve the 16 visible Quest Log stages where practical while using hidden/native sub-tracks and exact transitions underneath.

## Source callback readback at Crystal Summer pin

Exact callback keywords in `00ce02a5`:

| NPC | Crystal callback | Source behavior |
|---|---|---|
| Saraki | `moon`, `report` | starts the investigation; later sends the player to Udu |
| Tarisu | `tides` | says to take the Shell Gauge and use three markers; **does not grant the item in code** |
| Udu | `moon` | identifies Emerald Gardens impact area; text only |
| Sharai | `moon`, `machines` | first gives hidden-complex route; second sends the player to Oskayaat; text only |
| Javala | `moon` | says she gives the lower-area key; **does not grant an item in code** |
| Niral | `moon`, `seal`, `consecrate` | Rakesh task, post-kill sealing instructions, lava blessing; **no item/progress mutation in code** |
| Sundara | none for this quest | Crystal file is shop-only; no Yukti quest callback |

Therefore Crystal is not an executable quest donor for these conversations.

## Proposed hidden/native facts

Names below are **PROPOSED semantic labels**, not allocated canonical keys.

```text
started
yukti_done

tides_started
tide_marker_lagoon_a
tide_marker_lagoon_b
tide_marker_emerald
tides_done

plants_started
tide_veil_sample_obtained
whisper_reed_sample_obtained
tide_veil_refined
whisper_reed_refined
plants_done

three_clues_done
report_done
impact_location_known
crater_discovered

sharai_route_known
secret_entrance_opened
stone_door_observed
javala_alliance
citadel_key_eligible
laboratory_entered
lab_note_read
crystal_plinth_inspected
crystal_apparatus_inspected
laboratory_done
sharai_machines_reported

niral_rakesh_task
rakesh_defeated
niral_seal_instructions
empty_flask_obtained
blue_lava_obtained
blue_lava_blessed

rune_silent_moon
rune_silverflow
rune_veiled_night
rune_fallen_star
ritual_done

magnolia_admitted
magnolia_phase_2
magnolia_defeated

prison_torch_obtained
prison_chisel_obtained
prison_wall_prepared
prison_escaped

completed
```

The exact final key naming belongs to QUEST-CONTENT-2 / its active successor.

## Execution graph

### Visible s1 — start

Accept one qualified start route.

Preferred exact route already represented by the authored summary:

```text
Saraki + keyword "moon"
  -> started
```

The source also contains an alternative start path through Nilavarna. A faithful content packet should either admit both or explicitly retain the one-route restriction as an Oteryn approximation.

No NPC handler may write QuestState directly. The talk runtime must emit the typed NPC quest outcome consumed by the existing Quest transition writer.

### Visible s2–s4 — three parallel investigations

These are **parallel branches**, not `s2 -> s3 -> s4` dependencies.

#### Yukti branch

Preferred authored route:

```text
Sundara quest dialogue
  -> yukti_done
```

Source material also permits Nipuna on the alternate route.

Crystal Summer has no Sundara Yukti callback, so the executable Dialogue node must come from the qualified NPC/reference content rather than copying Crystal shop code.

#### Tides branch

```text
Tarisu + "tides"
  -> tides_started
  -> grant/materialize Shell Gauge i53783 through Item/Durability owner

use Shell Gauge on marker A
  -> tide_marker_lagoon_a

use Shell Gauge on marker B
  -> tide_marker_lagoon_b

use Shell Gauge on Emerald Gardens marker
  -> tide_marker_emerald

all three marker facts
  -> tides_done
```

Each marker must be a distinct placed-object/tile identity. Reusing one marker three times must not satisfy the objective.

#### Plants branch

```text
Dhira quest conversation
  -> plants_started

collect/use plant source A
  -> Tide Veil sample i53692
  -> tide_veil_sample_obtained

collect/use plant source B
  -> Whisper Reed sample i53693
  -> whisper_reed_sample_obtained

use/refine i53692 at Refiner of Magic i34338
  -> tide_veil_refined

use/refine i53693 at Refiner of Magic i34338
  -> whisper_reed_refined

both refined
  -> plants_done
```

The visible authored stage may remain one “collect/refine both samples” mission while native state prevents duplicate-source/repeated-use shortcuts.

### Join after the three investigations

QUEST-GATE-0 permits a conjunction of up to four predicates, so the source ordering can be represented without new predicate architecture:

```text
yukti_done
AND tides_done
AND plants_done
  -> three_clues_done
```

The report node is gated on that conjunction.

### Visible s5 — report + Udu

```text
Saraki + "report" [three_clues_done]
  -> report_done

Udu + "moon" [report_done]
  -> impact_location_known
```

If the alternative Nilavarna start/completion route is admitted, its corresponding report path must converge on the same semantic fact rather than fork the whole quest definition.

### Between visible s5 and s6 — crater discovery

The source has a physical discovery step that the 16-stage recipe compresses.

```text
ON_ENTER / exact area-object interaction at impact crater
  [impact_location_known]
  -> crater_discovered
```

This belongs to QUEST-TRIGGER-1 + world placement binding.

### Visible s6 — Sharai + secret entrance

```text
Sharai + "moon" [crater_discovered]
  -> sharai_route_known

use exact hidden-entrance rocks [sharai_route_known]
  -> secret_entrance_opened
```

The entrance interaction is not represented by the current single talk stage and must not be skipped in executable parity.

### Pre-s7 — first stone-door encounter

The source route first reaches the sealed lower-area door and produces a discovery/voice step before Javala.

```text
USE / approach exact stone door
  [secret_entrance_opened]
  -> stone_door_observed
```

This must remain distinct from later key use.

### Visible s7 — Javala and the Asura Citadel key

```text
Javala + "moon" [stone_door_observed]
  -> javala_alliance
  -> citadel_key_eligible
  -> Item owner mints/grants the qualified Asura Citadel key
```

Important identity hold:

- donor id `54262` is only an unresolved `name="key"` candidate;
- existing family proposal calls it a possible Shards/Javala key but explicitly does **not** prove the identity;
- do not promote `i54262` to exact Asura Citadel key without stronger binding evidence.

Source behavior also allows a lost-key regrant from Javala while the quest is in the eligible state.

#### Door completion-bypass blocker

The source says the key is required during the quest, while after quest completion the door no longer requires it.

That behavior is logically:

```text
has matching Asura Citadel key
OR
Shards quest completed
```

Current accepted architecture separates:

- key doors -> `DOOR-1 / KEY-1`;
- quest gates -> conjunction-only `QUEST-GATE-1`.

No accepted implementation of this key-door completion bypass was found on current main.

**Do not encode this as an invented QuestGate OR.** Route it to the Door/Key owner for composition.

### Visible s8 — laboratory

The source decomposes the authored `explore count=3` stage further:

```text
enter laboratory
  -> laboratory_entered

inspect Note Pinned on the Wall
  -> lab_note_read

inspect/use Crystal Plinth i54515
  -> crystal_plinth_inspected

inspect Large Crystal Apparatus / relevant machinery i53514
  -> crystal_apparatus_inspected

required laboratory facts
  -> laboratory_done
```

“Laboratory note” in the authored recipe is currently a semantic label, not a resolved canonical identity. Do not mint one by name match.

### Missing source step after s8 — return to Sharai

The current 16-stage recipe jumps directly to Niral. The source has a second Sharai conversation:

```text
Sharai + "machines" [laboratory_done]
  -> sharai_machines_reported
```

This is source-significant because it supplies the reason to seek Oskayaat/Niral.

It may remain hidden under the visible s8/s9 boundary, but it should not be dropped from executable parity.

### Visible s9 — Niral first conversation

```text
Niral + "moon" [sharai_machines_reported]
  -> niral_rakesh_task
```

### Visible s10 — Rakesh and Niral post-kill

```text
qualified Rakesh encounter/death
  [niral_rakesh_task]
  -> rakesh_defeated

Niral post-kill branch [rakesh_defeated]
  -> niral_seal_instructions
```

Crystal source branch keyword is `seal`; reference UI wording may differ. The content packet should bind a semantic Dialogue node, not hard-code the legacy keyword as authority.

Rakesh Creature/Behavior/Ability already exists. The missing piece is encounter admission + exact death occurrence → quest transition.

### Visible s11 — blue lava

```text
obtain/mint Empty Crystal Flask i53696
  -> empty_flask_obtained

use i53696 at exact blue-lava source
  with Moon Mirror i25975 requirement
  [niral_seal_instructions]
  -> transform to Blue Lava Flask i54564
  -> blue_lava_obtained
```

Night-time use is selected as a `STRUCTURED_REFERENCE_CANDIDATE`: pinned post-release Fandom explicitly requires night; BR/guides omit time-of-day but do not demonstrate daytime success. Keep the predicate explicit and source-qualified; controlled Global day-vs-night verification remains absent.

### Visible s12 — Niral consecration

```text
Niral + "consecrate"
  [blue_lava_obtained + holds i54564]
  -> transform i54564 -> Blessed Blue Lava Flask i54566
  -> blue_lava_blessed
```

Crystal dialogue contains this branch but does not perform the transform.

### Visible s13 — four ritual points

Use four exact placement identities, not one counter attached to a generic target.

```text
use blessed lava at ritual placement 1 -> rune_silent_moon
use blessed lava at ritual placement 2 -> rune_silverflow
use blessed lava at ritual placement 3 -> rune_veiled_night
use blessed lava at ritual placement 4 -> rune_fallen_star

all four
  -> ritual_done
```

The exact current Summer map contains all four physical positions.

Current appearance id `54637` is only classified as Terrain/ground-or-border; quest ritual semantics still require a typed placement binding.

Source behavior should determine the post-four-use flask state; do not infer charge count from the authored `count=4`.

### Visible s14 — The Moonsnow Magnolia

```text
ritual_done
  -> Magnolia encounter admission

phase 1:
  Magnolia combat
  + Furious Jaracal pressure
  + Bone Fiddle interaction
  + Moonsilver Drift
  + Death-damage healing

first lethal threshold
  -> encounter-owned revive / phase transition
  -> magnolia_phase_2

phase 2:
  permanent kill
  -> magnolia_defeated
  -> post-boss prison relocation
```

The Crystal monster statblock is insufficient to implement this. A Shards-specific Encounter owner is required.

### Visible s15 — prison escape

The authored “use Torch + Icicle Chisel” objective is too compressed.

```text
inspect/search skeleton
  -> Lit Torch acquisition
  -> prison_torch_obtained

inspect/use icicles
  -> Icicle Chisel i39578 acquisition
  -> prison_chisel_obtained

use required tool/fire interactions on exact north-wall passage
  -> prison_wall_prepared

cross opened/crumbling passage + final escape route
  -> prison_escaped
```

Lit Torch uses the Item/Timed-Item owner; do not bypass its accepted lifecycle semantics.

### Visible s16 — completion

Qualified completion route:

```text
Saraki completion dialogue [prison_escaped]
  -> terminal quest transition
  -> completed
  -> direct reward intents exactly once
```

If the alternate Nilavarna route is admitted, it converges on the same terminal transition.

Direct quest outcomes:

- Skewered Fish;
- `Amati's Echo`;
- the relevant access progression.

Jaracal mount / Six Steps Ahead remain downstream taming outcomes, not direct quest rewards.

## Required producer ownership

| Event family | Required existing owner |
|---|---|
| NPC branch | NPC-QUEST-1 / NPC-QUEST-CONTENT-1 |
| placed object USE | QUEST-TRIGGER-1 + Item/WorldObject |
| ON_ENTER discovery | QUEST-TRIGGER-1 + Map/World |
| boss death | Encounter/Combat death occurrence -> Quest transition adapter |
| item grant/transform | Item + Durability transaction |
| completion/reward | existing QuestState writer + Reward/Achievement/access owners |
| key door | DOOR-1 / KEY-1 composed with Quest completion bypass |
| Forbidden Gardens | Quest completion + Achievement-owned fact; no copied state |

All these producers should terminate in the existing `QuestTransitionRequest` / QUEST-STATE-1 writer. No new quest durability model is needed.

## Minimum changes to the authored representation

A faithful implementation should not silently reinterpret the current 16 visible stages.

Before activation, either:

1. retain the 16 visible stages as Quest Log presentation while attaching the internal micrograph above; **preferred**, or
2. explicitly revise the authored stage graph and regenerate all derived content with provenance.

In both cases the current serial semantics `s2 -> s3 -> s4` must not be treated as Reference truth once source-parity activation is claimed.

## Open blockers after this refinement

1. executable QUEST-GATE-1 / QUEST-TRIGGER-1 / NPC-QUEST-1 producers are not found on current main;
2. exact Asura Citadel key identity is not yet proven;
3. key-door post-completion bypass requires Door/Key owner composition;
4. Laboratory Note placement identity is unresolved;
5. ritual appearance `54637` lacks quest semantics;
6. Magnolia special Encounter is absent;
7. Blue Lava night predicate is source-qualified but still lacks controlled Global day-vs-night verification;
8. Forbidden Gardens needs Achievement-owned predicate composition;
9. final real-character start -> reward -> relog/restart E2E remains unqualified.
