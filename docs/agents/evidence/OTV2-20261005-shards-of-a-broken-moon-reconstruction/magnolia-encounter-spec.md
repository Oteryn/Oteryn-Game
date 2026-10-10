# Magnolia Encounter owner specification

Status: **EVIDENCE / PARTIALLY REPRESENTABLE / NOT ADMITTED**.
Readback: Oteryn-Game main `3bacc59e6adbf138655e6f0c1cf0b4fdf9a36096`, 2026-10-07.
Source facts and their precision are retained in `magnolia-encounter-evidence.json`.

## Current authoring fit

Use the existing closed vocabulary in `tools/content-schema/encounter-authoring/build_schema.py`, its generated schema and semantic validator, and the typed `apps/game-server/src/content/project/v2/encounter.rs`. No separate interpreter or quest engine is proposed. The current schema has `lethal_damage`, `counter_reached`, `item_used`, `timer_elapsed`, `stepped_on`, `area_left`, `prevent_death`, `counter`, `set_phase`, `heal`, `spawn`, `map_item`, `transform`, `teleport`, `convert_damage_to_heal` and `emit_outcome`.

The merged #1891 decision, ENCOUNTER-RT-0 §17.2, admits `attribute(max_health, set)` with a u32 bound. Current schema and Rust model still expose only outgoing_damage_percent/defense and add/reset. Thus a complete Magnolia document cannot honestly pass current admission. Do not publish a placeholder encounter as complete, hand-edit generated schema, add a second Magnolia form, or substitute an ordinary 52k heal.

An eventual full record needs the ordinary identity/revision, scope, participants, anchors, phases, state, rules and outcomes. `instance_per_party` is the existing instance scope; solo admission must cap the admitted cohort at one through the Boss/Encounter admission owner. The scope label alone does not enforce solo play. Keep arena timeout/cooldown in that owner's admission contract, not invented rule fields.

## Participants, anchors and state

- Magnolia uses the existing public `oteryn:creature.the_moonsnow_magnolia` throughout both phases. Resolve exact admitted revisions from the active content generation.
- Pressure uses existing Furious Jaracal, Frost Flower Asura and Midnight Asura identities, with source-qualified roles. Do not guess stronger-asura identities from names.
- Bone Fiddle source identity is i28493; Moonsilver Drift is i54235. The owner must qualify the active Item/WorldObject/Interaction carrier and exact placement refs before authoring ItemRefs.
- Arena entry source coordinate is `33901,32718,9`. Arena area, catnip positions, drift cells and exact prison arrival tile remain held unless an exact source/World binding supplies them. The skeleton tile is not proof of the arrival tile.
- Encounter-only state contains `magnolia_phase` (initial 1), a completion guard, phase-owned pressure/drift timers and any qualified suppression state. None is durable QuestState. The public phases are phase_1 and phase_2.

## First lethal and queued revival

The accepted first-lethal ordering is two rules. A proposed rule key is local to the future Encounter; it does not allocate a canonical identity.

```text
lethal_damage(role=magnolia)
  condition: counter_compare(magnolia_phase, <, 2)
  INLINE:
    prevent_death(role=magnolia)
    attribute(role=magnolia, max_health, set, value=60000)
    counter(magnolia_phase, set, 2)

counter_reached(magnolia_phase, 2)
  QUEUED AFTER ROOT HIT DRAIN:
    heal(subject=role:magnolia, amount=full)
    set_phase(phase_2)
```

60000 is the accepted #1891 value, not a newly inferred constant. Setting maximum health does not heal implicitly. The queued heal must finish at 60000/60000 after the lethal hit's drain. First lethal emits no creature death, quest completion, loot or Bosstiary kill. The next lethal follows normal death and one qualified completion outcome. The approximate Ice burst remains held until exact damage/area/ordering is qualified, including the case where it kills the player.

`set_phase`, counter and heal shapes already exist; max_health/set requires the accepted schema/ProjectV2 catch-up in ENC-COMBAT-1. The occurrence queue and inline/queued semantics belong to ENC-RT-1/ENC-COMBAT-1, not a Shards handler.

## Phase-one pressure and Bone Fiddle

The source describes initial Furious Jaracal pressure, phase-one Bone Fiddle use and further pressure if it is neglected. Map a qualified interaction to existing `item_used`, gate it on phase 1 with `counter_compare`, and update the owning pressure timer/flag. Map a qualified pressure deadline to `timer_elapsed` and existing `spawn` at exact owner-bound anchors.

Do not equate a roughly 30-second player recommendation with a server cooldown, or make a roughly 70-second description an exact timer. Timer start/reset, successful use target, repeat schedule, count policy and spawn positions stay held. Existing schema requires positive numeric durations and spawn counts; UNKNOWN cannot be filled with 1 or a guessed distribution. Phase two removes the Bone Fiddle requirement and cancels the phase-one pressure schedule. Post-release official placement changes in the source packet outrank release-day video positions.

## Moonsilver Drift and Death healing

Use existing `map_item`/World interaction composition for qualified drift creation/removal and `stepped_on` for exact carrier callbacks. Player Death damage is described as approximately 350–400; it is not a proved uniform roll. Asura transformation source/target mapping, eligibility, drift cadence, cells and lifetime remain held. `transform` is usable only after exact refs and health policy are qualified.

Death damage heals Magnolia. Current `convert_damage_to_heal` is the candidate vocabulary, but its component/amount semantics must be reconciled to the qualified source and accepted combat contract. The source packet explicitly leaves the conversion formula UNKNOWN. Do not silently assume a 1:1 conversion because a same-named action exists.

## Permanent death, prison and quest boundary

On normal Magnolia death in phase 2, the Encounter owner checks exact instance, active participant/cohort, public Creature identity, death occurrence and completion guard. Emit one `magnolia_defeated` outcome through ENC-OUTCOME-1; the owning adapter constructs the existing `QuestTransitionRequest`. Credit policy must follow the accepted solo encounter/outcome contract; neither `killer` nor `damage_contributors` is selected from guesswork.

Use the existing `teleport` action only after the World owner binds the actual prison arrival anchor and verifies it in the active generation. Prison route coordinates and rope exit are separately qualified in `prison-source-readback-20261007.json`; those static facts do not establish an arrival anchor or a served placement key.

Rewards belong to terminal s16 and the existing once-per-character NPC RewardClaim transaction. Magnolia death is not terminal quest completion. No synthetic reward chest, direct QuestState write or extra persistence store is permitted.

## Cleanup and acceptance

Room leave, player death, timeout, disconnect/relog, reset and generation teardown must fence/cancel the instance's phase timers, drift callbacks and spawn work; no phase-two spawn leak after leaving. Timer generations belong to Encounter runtime. Restart discards fight-only state under the accepted contract, while previously committed quest/reward receipts remain durable.

Required owner qualification before activation:

1. Two concurrent admission attempts cannot create a two-player solo fight; stale instance/generation and unmet ritual prerequisite fail closed.
2. First lethal drains before queued heal; phase 2 ends at 60000/60000 and emits no first-death completion/loot/credit. One normal final death emits one outcome.
3. Bone Fiddle affects only phase-one pressure, with proven timing; phase-two behavior and Death healing use qualified values.
4. Drift source/target refs, damage and map cells are admitted; no unknown placeholder constants pass validation.
5. Leave/reset/restart cancels phase-owned work and cannot deliver a stale relocation/outcome.
6. Generic ENC-OUTCOME-1 -> QuestTransitionRequest replay/fencing tests cover duplicate and stale death occurrences; terminal reward remains exactly once across relog/restart.

Allocation sequence: ENC-RT-1 -> ENC-COMBAT-1 including accepted max_health catch-up -> exact Encounter content admission -> ENC-OUTCOME-1 -> Shards bindings. This spec grants no ownership and does not claim executable Encounter parity.
