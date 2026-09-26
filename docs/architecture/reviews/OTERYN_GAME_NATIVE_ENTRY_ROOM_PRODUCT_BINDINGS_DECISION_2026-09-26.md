# Native entry-room product bindings and source-resource gate decision

- Decision: `NATIVE-ENTRY-ROOM-PRODUCT-BINDINGS-V1`
- Status: **OWNER-ACCEPTED; effective on exact-head validation, independent review and protected integration**
- Owner direction: owner requested one decision packet with recommendations (#162 comment `5846894605`); owner accepted this exact content in chat on 2026-09-26, verbatim "akceptuje dzialaj" ("I accept, go ahead"), after keeping one creature and deferring a nine-creature roster to a later profile amendment
- Admission baseline: `main@3b578bc5b35e40fbff70fbe5758079fdbed6a3c0`
- Extends: `NATIVE_ENTRY_SOURCE_QUALIFICATION_V1` (#937) §4 and §6; `PLAYER_FIRST_ENTRY_NATIVE_CONTENT_BINDING_V1` (#935)
- Related: Issues #162, #822, #930; Oteryn/Oteryn-Platform#1416
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Resolution packet

```yaml
classification: ARCHITECTURE_RESOLUTION
repository: Oteryn/Oteryn-Game
main_sha: 3b578bc5b35e40fbff70fbe5758079fdbed6a3c0
blocking_question: >-
  Which owner-accepted product identities, classes, licensing and source-resource
  limits fill the null prerequisites of the one native entry-room, so that an
  executable native source parser/producer worker can later be allocated?
facts:
  proven:
    - "FirstProduction requires the complete 18-definition graph even for the three-cell room (decision 2026-09-09, cardinality line 134)."
    - "Ordinary release rejects keys and atoms containing fixture, synthetic, evidence, test-only or a 'test' segment (production.rs has_nonproduction_marker)."
    - "Keys are namespaced 'namespace:local' ASCII [A-Za-z0-9:._/-] (production.rs ProductionKey::new)."
    - "Accepted class domains: SpawnRecoveryClass{EphemeralScopeReset, CheckpointedRuntimeContinuity, DurableEventOccurrence}; MultiplicityClass{ChannelLocalRepeatable, ChannelLocalSharedEligibility, WorldScopedUnique, ExplicitEventPolicyRequired}; EligibilityScope{CharacterWorld, AccountWorld, World}; EffectFamily{Damage}; CollisionClass{Walkable, Blocked} (model.rs)."
    - "#937 §4 requires product formula/policy bindings, presentation tokens, spawn classes and licensing to be actual accepted values; nulls refuse."
    - "#937 §6 leaves source-pipeline bytes, file count, nesting, parse/qualification work and binding retention DEFERRED."
    - "No accepted damage or XP mathematics exists on protected main."
  derived:
    - "The entry-room's first executable purpose is a controlled step/return/blocked-cell proof; creature, ability, loot and XP are graph members only and are not activated by it."
    - "Measured prepared example bytes (#937 fit evidence): project 323 B, world 2 169 B, reference 2 943 B, declarations 4 372 B; maximum JSON nesting depth 5."
  unknown:
    - "Canonical WorldId (Platform #1416)."
    - "Damage and XP mathematics."
    - "Client appearance assets behind presentation tokens."
accepted_decision: NATIVE-ENTRY-ROOM-PRODUCT-BINDINGS-V1 (owner-accepted 2026-09-26)
production_authority_changed: false
registry_mutation: none
```

## 1. Product identities

All identities are Oteryn-authored product identities for `PREPRODUCTION_FIRST_SLICE`. They are genuine, owner-accepted names, not fixtures. Where a later owner rejects or replaces one, the replacement is a new revision, never an in-place edit.

| Graph member | Key | Revision / token |
| --- | --- | --- |
| Region | `oteryn:region/entry` | — |
| Area | `oteryn:area/entry-room` | — |
| Terrain | `oteryn:terrain/stone-floor` | — |
| Cells | `oteryn:cell/entry-start` (0,0,0) Walkable; `oteryn:cell/entry-east` (1,0,0) Walkable; `oteryn:cell/entry-north` (0,-1,0) Blocked | — (as accepted by #935) |
| Relocation | `oteryn:relocation/entry-east-return` from `entry-east` to `entry-start` | — |
| Behavior | `oteryn:behavior/passive-idle` | policy `oteryn:policy/passive-idle-r1` |
| Creature | `oteryn:creature/rat` | policy `oteryn:policy/creature-rat-r1` |
| Presentations | `oteryn:presentation/rat`, `oteryn:presentation/bite`, `oteryn:presentation/cheese` | tokens `oteryn:appearance/rat-r1`, `oteryn:appearance/bite-r1`, `oteryn:appearance/cheese-r1` |
| Spawn | `oteryn:spawn/entry-rat` on `oteryn:cell/entry-east` | see §2 |
| Formula profile | `oteryn:formula/entry-melee-r1` | — |
| Effect | `oteryn:effect/bite` Damage → `oteryn:formula/entry-melee-r1` | — |
| Ability | `oteryn:ability/bite` → `oteryn:effect/bite`, `oteryn:presentation/bite` | — |
| Item | `oteryn:item/cheese`, materializable = true, → `oteryn:presentation/cheese` | — |
| Loot table / entry | `oteryn:loot/rat` with entry `oteryn:loot-entry/rat-cheese` → `oteryn:item/cheese`, purpose `oteryn:rng/rat-loot` | — |
| XP definition | `oteryn:xp/rat` → `oteryn:formula/entry-melee-r1` | — |
| RNG context | purposes [`oteryn:rng/rat-loot`] | profile `oteryn:rng-profile/entry-r1` |

Revision set (FirstProductionRevisionSet): content `oteryn:content/entry-r1`, map `oteryn:map/entry-r1` (equal to every selected placement map revision), ruleset `oteryn:ruleset/entry-r1`, world_policy `oteryn:world-policy/entry-r1`, compiler, canonicalization and sim_profile use the current owning constants of the unchanged FirstProduction compiler, profile_revision `oteryn:profile/native-entry-r1`.

**Formula boundary.** `oteryn:formula/entry-melee-r1` is an accepted product identity only. It does not select damage or XP mathematics. No consumer may evaluate it until a separate owner decision accepts its mathematics; until then Ability/combat and XP consumers of this room stay unactivated and any evaluation attempt refuses. The one shared profile serves both Effect and XP because FirstProduction admits exactly one formula profile.

## 2. Spawn classes

| Field | Value | Reason |
| --- | --- | --- |
| population_limit | 1 | Accepted profile shape. |
| recovery | `EphemeralScopeReset` | The room is disposable preproduction; a restart resets it and no durable occurrence is claimed. |
| multiplicity | `ChannelLocalRepeatable` | One rat per Channel scope, repeatable after reset; no world-unique or event policy. |
| eligibility_scope | `CharacterWorld` | Narrowest existing scope; no account/world-wide entitlement. |

These classes are applicability choices for this room only. They do not activate AI, respawn timing or loot rolls; each remains owned by its domain and unactivated here.

## 3. Licensing and provenance

All values in this document and in the room's source bytes are **Oteryn original work, authored by the owner, preproduction**. No third-party map data, sprite, name list or asset is imported for this room. Tibia-reference names (`rat`, `cheese`) are generic words used as Oteryn identities, not copied assets. The native package's licensing metadata records `oteryn-original-preproduction`; the producer refuses if it differs. Presentation tokens name future Oteryn client appearances; the client asset itself is a separate delivery.

## 4. Source-pipeline resource limits (closes #937 §6 for this room only)

Scope: the native entry-room source variant only. These are finite fail-closed first-slice bounds derived from measured prepared bytes with headroom, not production capacity. No `RESOURCE_LIMITS_REGISTRY` row is written; the parser/producer carries them as explicit typed constants tagged `PREPRODUCTION_FIRST_SLICE`, as #912 did for actors.

| Dimension | Bound | Basis |
| --- | --- | --- |
| Managed files | exactly 11 (3 controls + 8 roles) | #937 §2 exact set; any other count refuses. |
| Bytes per file | 64 KiB | Largest measured example 4 372 B; ≈15× headroom for real authored values. |
| Total source bytes | 256 KiB | Sum of 11 files at a realistic ≤ 23 KiB average; well below per-file × count. |
| JSON nesting depth | 16 | Measured 5; stops pathological nesting before allocation growth. |
| Directory entries scanned | 32 | Existing `max_entries_per_directory_scan`; the room has no extra entries. |
| Qualified bindings retained | 1 | One room, one generation; a second qualification replaces nothing and refuses. |
| Qualification work | bounded by the above (single pass over ≤ 256 KiB, fixed 18-definition graph) | No separate time bound selected. |

Every bound needs a max and max+1 test at the producer boundary; overflow refuses before staging, readiness, position or control, and preserves prior state.

## 5. What this does not decide

Damage/XP mathematics, AI behavior, respawn timing, loot probability, client appearance assets, canonical WorldId (#1416), current activation issuer, public wire IDs (#642), Movement limits (#139), production capacity or deployment.

## 6. Decision discipline

1. Must decide now? YES: #937 cannot release an executable native producer while these values are null.
2. Blocked downstream: native source/frame/pair qualification, then the first controlled actor (#822).
3. Later cost: identities become versioned product obligations; replacements are new revisions.
4. Supersession: accepted mathematics, a real asset/licensing change, measured source cost above these bounds, or a broader native authoring decision.
