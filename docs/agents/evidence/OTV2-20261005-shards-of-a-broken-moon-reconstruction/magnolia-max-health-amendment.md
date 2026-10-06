# Magnolia phase-2 max-health amendment — bounded Encounter-owner proposal

Status: **ARCHITECT DISPOSITION RECORDED IN #1891 / NO RUNTIME MUTATION**

## Architect disposition — #1891 supersedes the proposed action shape

PR #1891 (`ENCOUNTER-RT-0 §17`) selects **option B** and supersedes this packet's earlier proposed `set_health(max,current)` action.

Accepted shape at #1891 head `5dfe40ee3886174d3f3f56023ed5aee29c40e41a`:

```text
attribute(max_health, set 60000)
then queued heal(full)
then set_phase(phase_2)
```

Semantics:

- `attribute` gains `max_health`;
- operation is absolute `set` or `reset`;
- setting max health does **not** heal implicitly;
- percent/max-health-relative reads use the override in force;
- the override is Encounter-instance state, not a new Creature identity;
- reset/despawn returns to the Creature baseline;
- ENC-COMBAT-1 implements the authoring/ProjectV2/runtime support.

Magnolia's first lethal is intentionally two-step: inline `prevent_death + max_health=60000 + phase counter`, then queued `heal(full)` so the lethal drain cannot overwrite the new health.

The earlier `set_health(max,current)` proposal below is retained only as historical evidence of the gap and is **REJECTED/SUPERSEDED** by #1891.

Quest: Shards of a Broken Moon  
Encounter role: `The Moonsnow Magnolia`

## Exact gap

Current admitted Magnolia Creature max health is **52,000**.

Current Reference behavior for phase 2 is exact:

```text
first lethal
-> death prevented
-> same Magnolia continues
-> phase 2 begins immediately
-> current HP = 60,000
-> effective max HP = 60,000
```

Existing Encounter vocabulary already covers every other required primitive:

- `lethal_damage`
- `prevent_death`
- `set_phase`
- timers
- item-used / stepped-on
- spawn/remove
- damage->heal conversion
- encounter outcomes

The only missing representation is increasing the same live role above its Creature-definition maximum.

## Rejected workarounds

### Do not heal to 60,000 through `heal`

`heal full` is bounded by the Creature max of 52,000. Treating 60,000 as an over-heal would be an undocumented runtime exception and breaks max-health-relative rules.

### Do not mint a fake phase-2 Creature identity

Existing Transform examples such as Feroxa/Feroxa2 and Urmahlullu forms are backed by distinct source Creature forms.

Magnolia phase 2 is not a separately sourced Creature identity. Minting `the_moonsnow_magnolia_phase_2` solely to obtain a larger max-health field would duplicate one canonical boss identity and put an Encounter-local state change into Creature ownership.

## Smallest compatible amendment

Add one Encounter-owned action with absolute values, for example:

```json
{
  "kind": "set_health",
  "role": "magnolia",
  "max_health": 60000,
  "current_health": 60000
}
```

Equivalent naming is acceptable; semantics should remain narrow.

### ProjectV2 shape

Suggested Rust shape:

```rust
ProjectV2EncounterAction::SetHealth {
    role: String,
    max_health: u64,
    current_health: u64,
}
```

Do not add this to Creature attributes. It is Encounter-instance state.

### Authoring-schema shape

Suggested closed action:

```text
kind: set_health
role: declared participant role
max_health: integer >= 1
current_health: integer >= 0
current_health <= max_health
```

No percentages and no optional half-update are needed for the Magnolia source fact.

## Execution semantics

The action atomically replaces the live encounter actor's effective current/max health for the remainder of that live form/encounter instance.

Required invariants:

1. role must resolve to an existing live encounter participant;
2. `max_health >= 1`;
3. `0 <= current_health <= max_health`;
4. update is atomic — no intermediate 52k/60k state is observable;
5. percentage triggers after the action use the new 60k maximum;
6. Creature definition remains unchanged at 52k;
7. encounter reset/despawn naturally discards the override and a fresh Magnolia starts from Creature baseline;
8. no QuestState, Item, or account persistence is written;
9. the action cannot target players or arbitrary non-participant actors;
10. replay/double-fire protection stays owned by the Encounter rule runtime.

## Magnolia rule

The source-faithful lethal transition then becomes:

```text
trigger: lethal_damage(magnolia)
condition: phase == phase_1

actions, in order:
1. prevent_death(magnolia)
2. set_health(magnolia, max=60000, current=60000)
3. set_phase(phase_2)
4. apply source-qualified phase-transition visual/effect if admitted
```

First lethal must not emit the completion outcome.

Permanent phase-2 death emits the encounter completion outcome consumed later by ENC-OUTCOME-1 / Quest transition composition.

## Why an absolute max is required

Using only `current_health=60000` while keeping max=52000 is invalid:

- percentage conditions become wrong;
- full-heal behavior becomes wrong;
- damage/heal UI/state can clamp or misreport;
- a later reset/full heal returns to 52k unexpectedly.

The Reference fact is therefore a max-health override, not merely an absolute heal.

## Required tests

Schema / ProjectV2:

- accepts declared role, max=60000, current=60000;
- rejects unknown role;
- rejects max=0;
- rejects current > max;
- rejects unknown fields;
- canonical serialization round-trips exactly.

Runtime, once ENC-RT-1 exists:

- first lethal prevents death;
- role remains same canonical Magnolia actor;
- max/current become exactly 60000;
- health-percent trigger math uses 60000;
- phase becomes phase_2;
- no completion outcome on first lethal;
- second lethal can kill normally and emit completion;
- reset/new encounter restores Creature baseline 52000;
- no durable Quest/Creature mutation is produced.

## Ownership

This belongs exclusively to Encounter authoring / ProjectV2 / ENC-RT ownership.

It must not be implemented as:

- a Shards quest special case;
- a Creature-content duplicate;
- a generic over-heal exception;
- a direct QuestState write.

Once accepted, SHARDS-E1 can represent Magnolia without any remaining schema-level workaround.


## Exact implementation surface readback

Fresh current-main readback narrows the authoring/ProjectV2 amendment to the existing Encounter owner surfaces.

### Authoring schema

Owned/generated surfaces:

```text
tools/content-schema/encounter-authoring/build_schema.py
tools/content-schema/encounter-authoring/validate_encounter.py
tools/content-schema/encounter-authoring/verify_encounter_schema.py
tools/content-schema/encounter-authoring/encounter.schema.json   # generated output; do not hand-diverge
```

Current `build_schema.py` already defines adjacent actions:

- `heal`;
- `prevent_death`;
- `damage_modifier`;
- `set_phase`.

The bounded addition belongs alongside those action kinds.

Validator requirements are exactly:

```text
role exists in declared participants
max_health >= 1
current_health >= 0
current_health <= max_health
no unknown fields
```

No canonicalization ordering is required for two scalar values.

### ProjectV2 typed model

Exact file:

```text
apps/game-server/src/content/project/v2/encounter.rs
```

Current `ProjectV2EncounterAction` already contains `Heal`, `PreventDeath`, `ConvertDamageToHeal` and `SetPhase`, and the same file owns action validation.

Add only:

```rust
SetHealth {
    role: String,
    max_health: u64,
    current_health: u64,
}
```

and the corresponding validation branch.

Focused admission tests belong in:

```text
apps/game-server/tests/content_world_project_v2_encounter_admission.rs
```

No separate JSON->ProjectV2 converter was found: the typed ProjectV2 model is serde-backed directly, so there is no hidden lowering file that also needs a semantic amendment.

### Runtime boundary

Do not add runtime mutation before `ENC-RT-1` owns the Encounter interpreter.

When that owner exists, its action interpreter must atomically update the live Encounter actor's effective max/current HP. The Creature definition remains 52,000 and is not rewritten.

This code-surface readback makes the architecture decision independent from the later runtime allocation:
authoring + ProjectV2 can accept the fact first; runtime execution stays separately fenced.


## Why existing Transform cannot encode 60,000 HP

Fresh current-main readback confirms there is no hidden existing representation that makes the amendment unnecessary.

`ProjectV2EncounterParticipant` contains only:

```text
role
creatures[]
```

There is no participant-local stat or max-health override.

`ProjectV2EncounterHealth` is closed to:

```text
Full
KeepPercent
KeepAbsolute
Remembered
Percent { percent }
```

There is no absolute literal health value.

`ProjectV2EncounterAction::Transform` therefore cannot express “same Magnolia identity, set effective max/current HP to 60,000”. It can only choose a target Creature form and one of the relative/remembered health policies above.

That leaves exactly two possible approaches:

1. invent a second Magnolia Creature with 60k max HP — rejected because no such source identity exists; or
2. add the bounded Encounter-local health override described in this packet — preferred.

This closes the last reuse question around `Transform`; the `set_health(max,current)` amendment is not duplicating an existing ProjectV2 capability.
