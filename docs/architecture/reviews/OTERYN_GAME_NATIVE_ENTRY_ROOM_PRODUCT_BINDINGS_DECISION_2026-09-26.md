# Native entry-room product bindings and source-resource gate decision

- Decision: `NATIVE-ENTRY-ROOM-PRODUCT-BINDINGS-V1`
- Status: **OWNER-ACCEPTED; effective on exact-head validation, independent review and protected integration**
- Owner direction: owner requested one decision packet with recommendations (#162 comment `5846894605`); owner accepted the product choices of this packet in chat on 2026-09-26, verbatim "akceptuje dzialaj" ("I accept, go ahead"), after keeping one creature and deferring a nine-creature roster to a later profile amendment
- Admission baseline: `main@3b578bc5b35e40fbff70fbe5758079fdbed6a3c0`
- Post-acceptance review corrections (independent review of `bac601e1`): profile_revision bound to the unchanged profile constant; explicit compiler/canonicalization/sim_profile, package and lock identities (no owning constants exist); every `ProjectEvidenceLimits`/`ProjectFilesystemLimits` dimension bounded; explicit joins. No creature, item, class, licensing or size-limit choice changed; the added technical identities and bounds are presented to the owner for re-confirmation before integration.
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
    - "Measured prepared example bytes (#937 fit evidence): project 323 B, world 2 169 B, reference 2 943 B, declarations 4 372 B; maximum JSON nesting depth 6 under the implementation counting (root = 1)."
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

Explicit joins (all exact typed references, each definition identity is `(family, key, revision)` with revision `oteryn:rev/entry-r1` unless a row above names another):

- every cell → region `oteryn:region/entry`, area `oteryn:area/entry-room`, terrain `oteryn:terrain/stone-floor`;
- creature `oteryn:creature/rat` → behavior `oteryn:behavior/passive-idle`, presentation `oteryn:presentation/rat`;
- spawn `oteryn:spawn/entry-rat` → creature `oteryn:creature/rat`, behavior `oteryn:behavior/passive-idle`, cell `oteryn:cell/entry-east`.

The spawn and the relocation share `entry-east` only because the room has three cells. Nothing materializes the spawn or activates the relocation here; when either is later activated it must not occupy or redirect the start/east step-and-return proof cells.

Revision set (FirstProductionRevisionSet), all explicit accepted atoms:

| Field | Value |
| --- | --- |
| content | `oteryn:content/entry-r1` |
| map | `oteryn:map/entry-r1` (equal to every selected placement map revision) |
| ruleset | `oteryn:ruleset/entry-r1` |
| world_policy | `oteryn:world-policy/entry-r1` |
| compiler | `oteryn:compiler/first-production-r1` |
| canonicalization | `oteryn:canonicalization/first-production-r1` |
| sim_profile | `oteryn:sim/entry-r1` |
| profile_revision | `FIRST_PRODUCTION_CONTENT_PROFILE/v1` (the unchanged `FIRST_PRODUCTION_PROFILE_ID`; any other value is refused) |

Package and lock identities: package_key `oteryn:package/native-entry-room`, package_revision `oteryn:package-rev/entry-r1`, semantic_schema_version `oteryn:schema/first-production-v1`, licensing_metadata `oteryn-original-preproduction`, root project_revision and manifest package_revision `oteryn:package-rev/entry-r1`, Content Lock revision_digest_token is the value the existing canonical writer derives, `lock:` + project_revision = `lock:oteryn:package-rev/entry-r1`; the native writer does not override it. Digests (source manifest, manifest, lock) are computed from the actual authored bytes and are never chosen by this document.

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

Scope: the native entry-room source variant only. These are finite fail-closed first-slice bounds with headroom, not production capacity. Measured basis: the four documents shown in the #937 fit evidence (project 323 B, world 2 169 B, reference 2 943 B, declarations 4 372 B; implementation JSON depth ≤ 6; ≤ 132 decoded fields; cumulative string bytes ≤ 2 232 per document), which still contain null placeholders; the other seven documents are smaller control/metadata documents. No `RESOURCE_LIMITS_REGISTRY` row is written; the native producer passes these as its explicit `ProjectFilesystemLimits`/`ProjectEvidenceLimits` values, named and documented as `PREPRODUCTION_FIRST_SLICE`, never as a production default.

| Dimension | Bound | Basis |
| --- | --- | --- |
| Managed documents (`max_documents`) | 11 | Upper bound. Exactness (exactly these 11) is the separate #937 §2 role-set rule, tested on its own. |
| Bytes per document (`max_document_bytes`) | 64 KiB | Largest measured 4 372 B; ≈15× headroom. |
| Total source bytes (`max_total_bytes`) | 256 KiB | 11 documents at ≤ 23 KiB average. |
| JSON nesting depth (`max_json_depth`) | 16 | Measured ≤ 6 under the implementation's counting (root = 1). |
| Decoded fields (`max_decoded_fields`) | 4 096 | Measured ≤ 132 per document (declarations); fixed graph. |
| String bytes (`max_string_bytes`) | 16 KiB | Cumulative per-document budget (member names plus string values, one `parse_strict` call). Measured ≤ 2 232 B; the manifest carries 10 SHA-256 inventory entries and stays well below. |
| Locator bytes / segments (`max_locator_bytes`, `max_locator_segments`) | 128 / 4 | Longest fixed locator `definitions/declarations.json` is 29 B, 2 segments. |
| Reference records (`max_reference_records`) | 32 | Graph needs 18 definitions. |
| Import candidate records (`max_import_records`) | 1 | Smallest value `ProjectEvidenceLimits::validate` accepts (0 is rejected). Separately, the native qualifier requires the import-candidates document to hold zero records: the room is Oteryn-original and any candidate refuses. |
| Reimport states (`max_reimport_states`) | 1 | Same: smallest accepted limit; the native qualifier requires zero reimport states. |
| Entries per directory scan (`max_entries_per_directory_scan`) | 32 | New first-slice bound (the root holds 9 entries); not an existing default. |
| Total directory entries scanned (`max_total_directory_entries_scanned`) | 192 | One capture costs 111 + P: 11 locator lookups × 9 root entries = 99, plus 12 subdirectory entries, plus one scan of the parent directory (P entries). P is bounded by the per-scan limit (≤ 32), so the worst case is 143. The project root lives in a dedicated parent that holds only the root and its publication siblings (stage, journal, backup). |
| Qualification reports | 1 report per qualification, ≤ 16 KiB | One room, one bounded diagnostic. |
| Qualified bindings retained | 1 | One room, one generation; a second qualification refuses. |
| Qualification work | bounded by the above (single pass over ≤ 256 KiB, fixed graph) | No separate time bound selected. |

Every bound needs a max and max+1 test at the producer boundary; overflow refuses before staging, readiness, position or control, and preserves prior state.

## 5. What this does not decide

Damage/XP mathematics, AI behavior, respawn timing, loot probability, client appearance assets, canonical WorldId (#1416), current activation issuer, public wire IDs (#642), Movement limits (#139), production capacity or deployment.

## 6. Decision discipline

1. Must decide now? YES: #937 cannot release an executable native producer while these values are null.
2. Blocked downstream: native source/frame/pair qualification, then the first controlled actor (#822).
3. Later cost: identities become versioned product obligations; replacements are new revisions.
4. Supersession: accepted mathematics, a real asset/licensing change, measured source cost above these bounds, or a broader native authoring decision.
