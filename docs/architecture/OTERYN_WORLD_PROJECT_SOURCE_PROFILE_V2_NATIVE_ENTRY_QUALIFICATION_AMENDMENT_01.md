# WorldProject v2 native entry qualification — Amendment 01

- Decision: `NATIVE_ENTRY_SOURCE_QUALIFICATION_V1`
- Date: 2026-09-26
- Repository: Oteryn/Oteryn-Game
- Admission main: `de7c5499a2e6ad4ec533ea895754bdd82bda292e`
- Owner direction: **ACCEPTED / LIVE**, [#822 comment 5846264141](https://github.com/Oteryn/Oteryn-Game/issues/822#issuecomment-5846264141)
- Document at authoring: **CANDIDATE**; exact-head qualification, independent review and protected integration govern delivery.
- Allocation: [#162 comment 5846264304](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5846264304); this task writes architecture/evidence/task documents only.
- Scope: one native entry-room source consumer, solely `PREPRODUCTION_FIRST_SLICE`.
- Runtime/source implementation at admission: not supplied by this amendment; qualified native package/frame/pair and actual product bindings are **NOT_YET_PROVEN**.
- Actor registry delta: `[]`; production/live authority: NONE.

After protected readback, this amendment supplies the bounded source-shape and consumer semantics below. It does not release an executable worker, assign a WorldId, establish product policies/licensing, select source-resource maxima or activate Content. These remain explicit prerequisite gates.

## 1. Problem, alternatives and decision

Protected `PLAYER_FIRST_ENTRY_NATIVE_CONTENT_BINDING_V1` selects authored native start/east/north cells and genuine source-to-frame qualification. Unchanged WorldProject v2 supplies World/frame/bounds/floors, positions and some typed definitions, but lacks the required collision, explicit authored origin/contract binding and parts of the complete FirstProduction graph. Candidate observations and editor metadata have no executable meaning.

| Option | Benefit | Cost / disposition |
| --- | --- | --- |
| A: versioned source-only exception using existing project controls/roles, one closed native overlay and one explicit local consumer | Reuses admitted source capture, typed identities and unchanged ordinary compiler/artifacts. Makes only the missing data executable for this entry-room. | Requires a separately selected source variant, exact typed joins and genuine qualification. **Selected.** |
| B: a broader new serialized World Project/World Bundle and general importer | Could address future authoring needs. | Adds unrelated representation/import/runtime scope without solving an additional current blocker. Deferred pending a demonstrated requirement. |

The selected exception reduces time to a controlled step/return/blocked-cell proof. A bad join or frame proof would expose players to wrong collision or start placement; failed source qualification must withhold readiness. It creates one maintenance obligation: a versioned native source consumer and source-only canonical writer. Future additional families/rooms require their own accepted extension; they cannot enter through arbitrary fields.

## 2. Exact source variant and controls

Selection is the exact pair:

```text
root.schema = OTERYN_WORLD_PROJECT_ROOT/v2
root.source_profile = OTERYN_WORLD_PROJECT_SOURCE_PROFILE/v2-native-entry-qualification-1
declarations.schema = OTERYN_WORLD_PROJECT_DECLARATIONS/v2-native-entry-qualification-1
```

These are source schema/profile identifiers, not public runtime wire IDs. A distinct explicit native admission API selects this pair **before** capture/parse/lowering. Dispatch by root schema alone, opportunistic detection of an ignored field, and reinterpretation of an admitted ordinary v1/v2 project are forbidden.

The root retains all existing fields: schema, source_profile, project_revision, manifest_locator, manifest_sha256, content_lock_locator and content_lock_sha256. Control locators are exactly project.json, manifest.json and content.lock.json. Actual hashes are mandatory; null/missing/malformed/mismatching values refuse.

Manifest and Content Lock retain their existing v2 physical shapes/schema identifiers. Root project_revision, manifest package_revision and lock project_revision must agree. required_features and optional_features must both be empty; no feature flag enables native semantics or an extension. The lock retains exactly one matching package/provenance entry, with floating=false and dependency=false.

The native variant requires one document at each exact managed role/locator/schema:

| Role | Locator | Schema |
| --- | --- | --- |
| reference-records | definitions/reference.json | OTERYN_WORLD_PROJECT_REFERENCE_RECORDS/v1 |
| declarative-definitions | definitions/declarations.json | OTERYN_WORLD_PROJECT_DECLARATIONS/v2-native-entry-qualification-1 |
| world-records | worlds/world.json | OTERYN_WORLD_PROJECT_WORLDS/v2 |
| presentation-bindings | presentations/bindings.json | OTERYN_WORLD_PROJECT_PRESENTATION_BINDINGS/v2 |
| asset-records | assets/catalog.json | OTERYN_WORLD_PROJECT_ASSETS/v2 |
| import-candidates | provenance/imports.json | OTERYN_WORLD_PROJECT_IMPORT_CANDIDATES/v1 |
| provenance-records | provenance/sources.json | OTERYN_WORLD_PROJECT_PROVENANCE/v2 |
| editor-records | editor/author.json | OTERYN_WORLD_PROJECT_EDITOR/v2 |

Reuse strict UTF-8/JSON parsing, duplicate/unknown-field refusal, coherent capture, checked bounds, canonical enumeration/writing, filesystem containment/no-follow admission and exact byte-count/SHA-256 checks. The root/manifest/lock and managed documents must be captured from the same admitted project. Extra/missing/duplicate roles, alternate locators, mismatched document schemas, unsupported variants or incoherent hashes refuse. The listed document set is a source shape; it does not select physical-source resource maxima.

Ordinary v1/v2 readers/writers retain their exact accepted profiles and bytes. They do not accept the new declarations shape/profile. A future native writer explicitly emits this selected variant; no silent upgrade/downgrade strips the overlay. Default Reference lowering acquires none of its meaning and cannot establish native qualification.

## 3. Closed typed overlay

The new declarations document retains the existing records, optional item_authoring and optional authoring_profiles fields and adds optional native_first_entry. The native consumer requires its presence; absence is not a default recipe. It consumes only the selected native graph. Candidate declarations/fields, authoring overlays, aliases/tags and appearance bindings cannot provide missing executable values.

All native overlay fields below are mandatory for qualification and have no null/fallback semantics:

| Field | Exact typed contents |
| --- | --- |
| world_key | Existing namespaced source World key |
| frame | coordinate_profile, contract_revision, coordinate_frame, origin(x:i32,y:i32,floor:i16), x_direction, y_direction, higher_floor |
| revisions | content, map, ruleset, world_policy, compiler, canonicalization, sim_profile, profile_revision |
| region | key |
| cells | Exactly three (placement_key, region_key, collision) records |
| relocation | key, from_cell, to_cell |
| behavior | definition exact typed reference, policy_revision |
| presentations | Exactly three (definition exact typed reference, metadata_token) records |
| creature | definition exact typed reference, policy_revision |
| spawn | key, creature exact typed reference, behavior exact typed reference, cell_key, population_limit, recovery, multiplicity, eligibility_scope |
| ability | definition exact typed reference, presentation exact typed reference |
| item | definition exact typed reference, presentation exact typed reference |
| loot_table | key, entry(key, item exact typed reference, rng_purpose_key) |
| xp | key, formula exact typed reference |
| rng | profile_revision, purpose_key |

A definition reference is (family,key,revision), validated against the exact typed definition identity in these captured bytes. No key-only/latest-revision lookup, aliases, external source IDs or cross-project fallback can resolve it. Scalar key/atom/digest domains reuse existing owning types. collision uses existing Walkable/Blocked. Other classifiers use existing FirstProduction/Game-Channel domains and must satisfy their actual accepted product applicability; presence of a spelling alone is not policy evidence. Unknown fields/variants or extra native families refuse.

The full field mapping and F1/G1–G13 source gaps are in [fit evidence](../agents/evidence/OTV2_20260926_NATIVE_ENTRY_SOURCE_PROFILE_FIT.md). This selects source shape/meaning; it does not claim the example values or new consumer are qualified.

## 4. One entry-room lowering and revision joins

The selected World is the sole native World. Its actual canonical WorldId must equal the source ReferenceDocument binding and have current proof from the owning World Registry/topology issuance and Game assignment boundary required by the first-entry decision. A source-local ID, a historically used qualification UUID or caller-supplied bootstrap intent is not issuance proof. This amendment does not mint a Game WorldId or claim that every external issuer path is unavailable. Its frame name must match the explicit overlay frame and every selected placement. The source package/revisions and lock must be coherent.

The native consumer selects exactly three Terrain placements corresponding bijectively to the three overlay cell records. Placement key becomes Cell key; x/y/floor become x/y/z. Each placement must reference the sole exact Terrain revision and a present exact sole Area revision. Region comes only from overlay.region; collision comes only from overlay.cells. Duplicate coordinates/keys, missing/extra placements or mismatched World/map/frame refuse. Presentation order remains source presentation ordering and cannot select collision, ownership or entry priority.

The cells are the already accepted authored choices: start (0,0,0) Walkable, east (1,0,0) Walkable, north (0,-1,0) Blocked. Origin (0,0), floor 0, envelope [0,2) × [-1,1) and floor set [0] are this entry-room's authored choices, not implicit geography, surface or capacity defaults.

frame.coordinate_profile is exactly oteryn-world-spatial-v1 and contract_revision is 1. Directions are the canonical +x East, +y South, larger floor Up. No alternative axes, scale, coordinate conversion or independently supplied frame is supported. Bounds/floors come from the selected World; the explicit origin and all three coordinates must agree with those source declarations.

The remaining graph preserves the existing exact FirstProduction cardinality and validators:

- Area/Terrain/Behavior/Presentation/Creature/Formula/Effect/Ability/Item keys and applicable existing relations come from their exact typed source identities. Global FirstProduction key uniqueness applies in addition to source family/key/revision identity checks.
- Behavior/Creature policy revisions and three presentation metadata tokens come only from the typed overlay and real accepted product sources.
- Effect is existing Damage with an exact Formula reference; Heal refuses. Ability must have exactly one exact Effect relation, plus its overlay Presentation. A first-of-many conversion is forbidden.
- Item must meet the existing materializable=true profile requirement and exact Presentation binding. Source Item stack/physical/semantic metadata does not gain extra runtime interpretation.
- Creature behavior/presentation joins must match the overlay's exact Creature and Spawn refs. No optional source Loot relation can be treated as a native loot policy.
- Relocation uses explicit same-scope cell endpoints. Spawn has explicit cell/Creature/Behavior and accepted population/classification fields. No inference from Encounter/Transition candidate fields is permitted.
- Native LootTable/Entry, XP and RNG come from their typed overlay. The single purpose must match the single loot entry; no seed, secret, entropy or mutable RNG state enters content. Reference loot algorithm/count/probability is not converted into this different profile.
- overlay.revisions supplies every FirstProductionRevisionSet field. Its map equals all selected placement map revisions. Every remaining product/compiler/canonicalization/SIM/policy revision needs actual accepted binding; root revision alone cannot manufacture it.
- capability and migration use the unchanged owning FirstProduction constants and CompatibleNoMigration. This source variant does not widen artifact capability or profile.

Unknown/nonproduction/fixture-only formula or policy bindings, unresolved licensing, null values and missing actual World assignment refuse before a release can be qualified. A formula identity is not acceptance of a new mathematical formula. Candidate source-only observations are retained under their owning rules and cannot affect native physics, policies or current authority.

## 5. Genuine source-to-frame and pair qualification

The producer/release boundary admits actual immutable native bytes and genuine licensing/source ownership. It checks the exact variant, managed document digests, complete typed graph and joins. It reads the explicit frame from those bytes, not an operator assertion or a separately supplied wrapper.

Use the existing evidence chain:

```text
actual declarations/world/other admitted source bytes
  -> actual managed-document digests in immutable source manifest
  -> actual source_manifest_digest
  -> PackageManifestBinding and package_provenance_digest
  -> exact one-entry Content Lock
  -> typed FirstProductionContentSource
  -> deterministic OrdinaryRelease server/client artifacts
  -> validated pair and exact staged GenerationIdentity
  -> bounded typed source-qualified native frame/generation binding
```

Recompute existing digests/provenance; this amendment selects no new cryptographic formula, artifact field or extension map. The manifest does not include its own digest or downstream artifact digests. Qualification binds actual source frame/geometry to exact World/map/package/profile/compiler/revision/manifest/lock context and final server/client pair. Copied generation fields, renamed fixtures or equality of independently supplied frame labels cannot prove this chain.

The raw source parser, legal/license documents, source-manifest payloads and provenance fetch remain outside the runtime artifact/profile under Amendment02. Native qualification is a local producer/release operation feeding the unchanged typed compiler, not a runtime source-loading endpoint. Runtime may consume only the bounded binding established by genuine qualification and the exact validated pair.

This binding is immutable expected evidence, not current activation authority. Protected first-entry rules still require an independent current activation issuer, monotonic floor, expected-current/quiescence, exact Channel pin and the same qualified frame in issued location, retry and restart. Missing source proof/current continuity remains not-ready; no position/control write or authority/deadline renewal may be inferred.

## 6. Resource and executable-release gates

The source-pipeline allocation trigger named by FirstProduction §5.3 and Amendment02 §3 is now material. This amendment selects the bounded representation/consumer; it **does not close the resource gate**.

Physical source/manifest bytes, file/enumeration counts, nesting, parse/qualification work, reports and variable-size qualified-binding retention remain DEFERRED pending accepted applicable bounds and failure behavior. Reuse an existing bound only after exact applicability is demonstrated. Caller-selected WorldProject evidence limits are not production defaults. There is no registry append in this task.

Before an executable native parser/producer worker is released: protect this amendment, resolve actual World/product policy/formula/licensing bindings, classify every added resource dimension, provide measured/derived evidence and accepted limits or an explicit exclusion, serialize any separately required registry update under its own lease and verify protected readback. The implementation allocation must be explicit and path-bounded. Null examples do not bypass these prerequisites.

Actor registry delta remains []; 131072 remains only the explicit preproduction config bound under #912. #139 whole-cycle Movement/fairness/retained-work, #642 wire/schema/stable IDs, liveness, Recovery/ControlLoss/ClientResume, broader re-entry/respawn and deployment retain their separate owners.

## 7. Mandatory future qualification

The executable slice must prove actual managed bytes/licensing/joins/full graph, deterministic ordinary pair and independently loaded start/east/north geometry. Component proof does not establish Server Seam/native-client E2E or Reference parity.

Use independent single-invariant negative oracles, keeping unrelated facts valid: wrong/missing root selector or declarations schema; extra feature/role/field; mixed revisions/digests/lock; wrong-family/same-key wrong-revision references; cell/placement/map/World mismatch; wrong contract/axes/origin/bounds/floor; same coordinates/generation with substituted frame; missing product binding/licensing; forged qualification; incompatible pair; retry/restart without original source proof/current continuity. Refuse before staging/readiness/position/control as applicable and preserve prior state. Bound parsing/allocation/work/retention, max/max+1 and overflow evidence follows the separately accepted resource gate.

Cover source admission -> graph -> pair -> qualified binding and applicable activation/pin/location/retry/restart consumers under AuthorityInvariant × ConsumerBoundary × MutationOperator. No record-derived matching helper is the negative oracle. Reuse existing Content/Rust and protected CI routes; do not create a synthetic harness as the only product/source oracle.

This document candidate needs governance, architecture audit, Merge Gate, own whole-diff self-review and one independent exact-head architecture/security review, governed MQ, real merge_group game-gate and protected-main readback. Runtime/E2E execution is NOT_APPLICABLE to the docs-only candidate.

## 8. Decision discipline and supersession

1. Must decide now? YES: candidate-only source lacks the data/semantics needed to safely implement genuine native supply.
2. Blocked downstream: source/frame/pair qualification for the first controlled actor and Server Seam #822, after the separately accepted resource/product gates.
3. Later cost: native collision/frame/graph meaning becomes a versioned source compatibility obligation; silent reinterpretation would compromise placement/collision/progress.
4. Supersession: demonstrated representation gap, measured source cost, security finding or accepted broader native authoring requirement. A later decision must name the changed fields/semantics and retain current authority fences.
5. Not decided: math, public wire IDs/codec, timings/maxima, broad importer/Studio/permanent bundle, signing/network provenance, production capacity/deployment, Reference parity or durable re-entry.

This explicitly supplies only the new source-variant exception and native consumer semantics to WorldProject v2's later-owner requirement. Its existing v1/v2/default Reference meanings remain binding. FirstProduction compiler/artifact safety and Amendment02 runtime exclusion remain unchanged. The resource dimensions named above remain future accepted decisions; protected integration of this architecture alone does not make source execution ready.

## Source references

- [Owner direction](https://github.com/Oteryn/Oteryn-Game/issues/822#issuecomment-5846264141) and [exclusive docs allocation](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5846264304)
- OTERYN_WORLD_PROJECT_SOURCE_PROFILE_V2_DECISION.md
- reviews/OTERYN_GAME_PLAYER_FIRST_ENTRY_NATIVE_CONTENT_BINDING_DECISION_2026-09-26.md
- OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md and Amendments 01–03
- CONTENT-504_D1_TYPED_SOURCE_OWNER_BINDING_CONTRACT_CANDIDATE.md
- ../contracts/OTERYN_WORLD_SPATIAL_COORDINATE_PROFILE_V1.md
- ../agents/ARCHITECTURE_DECISION_DISCIPLINE.md
