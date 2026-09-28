# Oteryn WorldProject/v2 encounter admission v1

- Date: 2026-09-28
- Status: ACCEPTED (owner answer 2026-09-28 "tak" to E1-E5) / implementation follows in the slices of §5
- Task: `OTV2-20260928-encounter-admission-design`
- Programme: KAN-16 / #162
- Companions: `OTERYN_WORLD_PROJECT_V2_CREATURE_ADMISSION_V1.md` (creature admission, wave A),
  `OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md` (encounter vocabulary), `OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1.md`
  (coordinate frame precedent), `OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md` (`content/encounters/**`)

## 1. Problem

Creature admission wave A keeps every encounter-covered monster out of `content/world`: 164 monsters today
(`deferred_encounter`). They wait for "an Encounter runtime slice", because admitting them without their encounter
rules would change the fight (Kesar would not be immortal, Urmahlullu's forms would not follow one another).

Admission adds no spawns, placements or runtime behaviour, though. An admitted creature is not placed anywhere and no
fight runs. What is missing is the encounter itself in `content/world`. When an encounter and its creatures enter the
project together, the fight is complete as content. The runtime slice is still the one that makes it playable.

WorldProject/v2 already has the `Encounter` family: a `ProjectV2Declaration::Encounter { identity, fields }` and a
`ProjectV2EncounterAuthoring` profile (`encounter_type`, `scope`, `areas`, `cooldown_seconds`, `repeatable`,
`interactions`). No Encounter is admitted yet.

## 2. Decisions

| # | Decision | Alternative declined |
|---|---|---|
| E1 | The Encounter authoring profile gets a typed mirror of the whole v1 vocabulary: participants, phases, anchors, state, rules (17 triggers, 14 conditions, 25 actions, positions and areas), authored abilities and outcomes. Rust validates the structure (ranges, sorted and unique sets, exact references, the roles, anchors, counters, flags, timers and phases a rule names). `validate_encounter.py` stays the semantic source, as `validate_monster.py` does for monsters. | Admitting only the identity and participants and leaving the rules outside the project. That repeats the "part only" route the owner declined for monsters. |
| E2 | Anchors get structured coordinates in the project frame `global-target-2026-09-27`. A point is `{x, y, floor}`; an area is a rectangle or a radius around a point, on one floor. The Canary coordinate is the evidence, `OTS_HYPOTHESIS_ONLY`, as it already is for NPC travel destinations. Rule 5 of D28 (§9 of the format) is amended: an anchor must be bound to a map revision before the encounter is **activated** by a runtime, not before it is admitted as content. | Keeping every encounter out until a map project exists. That blocks 128 creatures on a slice that is not planned yet. |
| E3 | A creature covered by an encounter records its encounters in its Creature profile (`encounters: [Encounter ref]`). The creature admission contract gains one rule for any later spawn, placement or activation slice: an encounter-bound creature is never activated without its encounter. This keeps the reason for the old deferral as an enforced rule. | Keeping the deferral. |
| E4 | Admission is closed, as for creatures. An encounter is admitted only when its manifest has no `unresolved_semantics` row, every anchor has a location, and every Creature, Item and Ability it references is admitted. A monster covered by encounters is admitted only when all of them are admitted. The staging tool repeats this until nothing more is dropped. | Admitting an encounter with a hole in it. |
| E5 | Mapping details: identity `oteryn:encounter.<slug>`, revision `definition-r1`. `instance_per_party` becomes `Instance` and `channel_shared` becomes `Channel`. `encounter_type` is `Boss` when a participant has a bosstiary entry or the `reward_boss` flag, otherwise `Generic`. `cooldown_seconds` and `repeatable` stay unset: under D27 boss cooldowns belong to the reward domain, so `outcome_evidence` stays evidence for it. The source binding uses namespace `canary/encounter`, with the sample slug as external id, and the manifest digests go in the import batch. | Filling cooldowns from Canary into the encounter. |

### Decision test (`ARCHITECTURE_DECISION_DISCIPLINE.md`)

1. **Must decide now?** Yes. Slices 2-4 need the profile shape, the anchor locations and the admission rule, and
   164 encounter-covered monsters are blocked from `content/world` until they exist.
2. **What downstream work is blocked?** The creature staging (`creature_admission_stage.py`, `deferred_encounter`), the
   Rust admission of the Encounter profile, the `content/encounters/**` tree, and every later Encounter runtime slice,
   which needs admitted encounters to interpret.
3. **What becomes harder later?** Changing the v2 Encounter profile shape means a new profile revision and a restage.
   The Canary anchor coordinates become project data that a map project must bind or replace.
4. **What evidence would justify superseding it?**
   - An Encounter runtime slice that cannot interpret the v1 vocabulary as typed, for example a mechanic the closed
     vocabulary cannot express without scripts (D28).
   - A map project whose coordinate frame differs from the Canary map, so that anchor locations need a transform
     rather than a binding.
   - Evidence that an admitted encounter-bound creature was activated without its encounter, which E3 forbids.
   - A reward or quest domain contract that needs cooldowns or progress inside the encounter, contrary to D27.
   - A change in the owner's playable-first priorities, for example a runtime needed before admission.
5. **What is deliberately not decided?**
   - The Encounter runtime and its instancing.
   - Map binding and the World admission.
   - The reward and quest consumers of outcomes.
   - The asset bindings of anchor effects.
   - Whether later encounters that are not from Canary use the same profile unchanged.

## 3. Expected scope of the first encounter wave

The estimate below is a closure over the current census. The staging tool's counts are the authority.

| Group | Count |
|---|---:|
| Encounter samples | 83 |
| with `unresolved_semantics` rows (Alptramun, Ferumbras Mortal Shell, Gorzindel, Melting Frozen Horror, The Sandking) | 5 |
| admitted after closure | about 58 |
| Encounter-covered monsters admitted | about 128 of 164 |
| Other monsters freed by the new references | about 3 |
| `content/world` creatures | 1,319 → about 1,450 |

The other encounters that stay out have a participant that is not admitted itself: a monster still blocked in the
census (Plagirath's bog, Lady Tenebris' ultimate, Zamulosh, Mazoran, the Time Guardian forms) or one that waits for
the Item domain. The Soul War taint zones, for example, wait for six of their 15 monsters. They enter with their
creatures, in the same slices.

## 4. Records

- A `ProjectV2Declaration::Encounter { identity, fields: [] }` for each admitted encounter.
- An `Encounter` authoring profile (E1) with typed participants (`role` → Creature refs), `phases`, `anchors`
  (E2, with the Canary description kept as source text), `state`, `rules`, `abilities` and `outcomes`.
- Item references are resolved through the protected Item identity map, as for creatures. Ability references point
  to admitted Ability records. Encounter-local abilities (D34) stay inline in the profile.
- The Creature profile of every covered monster gets `encounters` (E3).
- Everything is declarative and candidate-only under the v2 rule. `link_reference_playable` is unchanged, because
  Encounter is not an executable Reference family.

## 5. Slices

1. This decision.
2. Encounter format: structured anchor locations (E2) in the schema, the validator and the transcription, with the
   prose kept as the description. The change is offline only; no content changes. Done with this decision: 141 of the
   142 anchors are located. The Soul War taint zones subtract safe areas and include the Goshnar boss rooms, so they
   stay unlocated, and their encounter is not admitted yet.
3. Rust: the typed Encounter profile (E1), the Creature `encounters` field (E3), `canonicalize`, validation, and
   focused positive and negative tests. No content changes.
   Done in `OTV2-20260928-encounter-admission-rust`, in `apps/game-server/src/content/project/v2/encounter.rs`.
   `ProjectV2EncounterAuthoring.details` holds the typed vocabulary, with percentages as exact ppm and every union as a
   tagged value. Rust checks ranges, exact references, and that every role, anchor, area, counter, flag, timer, phase,
   outcome and encounter ability a rule names is declared. An Encounter profile must carry these details (E1).
   `ProjectV2EncounterDetails.covers` names the participant creatures the encounter covers, and
   `ProjectV2CreatureAuthoring.encounters` binds a creature to its encounters (E3). The project checks both directions:
   a covered creature lists its encounter, and a listed encounter covers the creature. The D45 summon spells get `ProjectV2AbilityDetails.encounter`: exactly one of effects, variants
   and encounter, and an encounter-backed ability must be a spell. Every such ability needs an `ability_cast` rule in its
   encounter. The rule's role must be able to be a creature, participant or spawned, that owns the ability. Every
   owner of the ability is bound to, and covered by, that encounter. Tests:
   `apps/game-server/tests/content_world_project_v2_encounter_admission.rs`.
4. Writer and the first wave: `creature_admission_stage.py` stages the encounters and their creatures under E4, the
   materializer pins the result, and the content tree is regenerated. One independent exact-head review applies,
   because `content/world` changes.
   Done in `OTV2-20260928-encounter-admission-wave`.
   - **Encounters:** 58 of the 83 encounters are admitted. Each is an Encounter declaration, a typed profile and a
     `canary/encounter` source binding.
   - **Deferred encounters (25):** 19 wait for an unadmitted creature, item or ability, 5 keep an
     `unresolved_semantics` row, and 1 has an unlocated anchor (the Soul War taint zones).
   - **Creatures:** 1,450 are admitted, up from 1,319. Monsters that wait only on an encounter drop from 164 to 35.
   - **Content tree:** the admitted encounters are in `content/encounters/definitions/`. The tree contract has no
     node for Generic encounters, so all types share that node.
   - **Manifest digests (E5):** each manifest's digest stays in the pinned staged evidence
     `docs/agents/evidence/OTV2-20260927-creature-admission-wave-a-staged.json`, next to its profile.
5. Later: the remaining encounters as their creatures, items or vocabulary resolve; map binding and the Encounter
   runtime are separate owned slices.

## 6. Boundaries

This route admits no spawn, placement, map binding or runtime behaviour, and no Lua. It does not create reward,
cooldown or quest state (D27). Nothing it admits is compiled or activated.
