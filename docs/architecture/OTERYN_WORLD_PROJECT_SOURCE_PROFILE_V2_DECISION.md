# Oteryn World Project source profile v2

- Date: 2026-09-23
- Gate: CONTENT-504 / CW1 successor
- Status: owner-requested canonical v2; implementation and protected review pending
- Supersedes: only the editable-source shape of `OTERYN_WORLD_PROJECT_SOURCE_PROFILE/v1`
- Does not select: new runtime mechanics, World Bundle encoding, production activation or full-world resource maxima

## Decision and timing

**Must decide now: YES.** The requested complete Content/World authoring structure cannot be represented by the six-document v1 carrier. Its strict parser rejects new roles and its only executable graph is the existing Reference Playable graph. Adding the roles or interpreting candidate fields under the v1 identifiers would silently change a published source profile. A versioned successor keeps old snapshots readable and makes the new authoring surface explicit.

V2 is an editable source profile, not a second gameplay model. Its **executable** subset is still the existing `ProjectReferenceRecord -> ReferencePlayableContentSource -> link_reference_playable` path. Additional typed declarations, world occurrences, appearance bindings, assets and editor metadata are retained and validated as **declarative, non-executable** source records. They are never lowered to the Reference artifact or client gameplay projection by their mere presence. A later owned, evidence-qualified schema revision must select executable semantics for a family before its runtime path is added.

## Document and compatibility contract

The three control locators remain `project.json`, `manifest.json`, `content.lock.json`, with distinct `/v2` root, manifest and lock schema/profile identifiers. The v2 canonical writer emits one deterministic document in each of these roles:

| Logical role | Canonical locator | Meaning |
|---|---|---|
| `reference-records` | `definitions/reference.json` | Already executable Reference definition families |
| `declarative-definitions` | `definitions/declarations.json` | Typed non-executable definitions |
| `world-records` | `worlds/world.json` | World identity and authored placement occurrences |
| `presentation-bindings` | `presentations/bindings.json` | Typed appearance-to-asset declarations |
| `asset-records` | `assets/catalog.json` | Exact immutable asset identity and digest |
| `import-candidates` | `provenance/imports.json` | Existing import/reimport evidence |
| `provenance-records` | `provenance/sources.json` | Source identities bound to exact existing import/reimport batches |
| `editor-records` | `editor/author.json` | Author metadata, aliases and namespaced tags |

Manifest inventory binds each managed document's role, schema, canonical locator, byte count and SHA-256. Paths locate bytes; definitions use `(family, namespaced key, exact revision)` and placements use a separate stable placement key. Reordering files or changing a locator never remints a definition or placement. V2 does not require JSONL or select a world chunk size.

The v1 parser and writer keep their exact six-document bytes and identifiers. V1-to-v2 migration reads an admitted v1 project, carries its Reference definitions, world binding, import batches and author metadata into the v2 roles, initializes the newly supported roles empty, then rebuilds the manifest and Content Lock under v2. This changes package/source provenance; it does not claim bitwise identity of compiled artifacts. There is no implicit v2-to-v1 downgrade API that could discard populated roles.

V2 retains v1's strict UTF-8 JSON, duplicate-member and unknown-field rejection, canonical locator/no-follow filesystem admission, finite caller-selected limits, canonical deterministic writer, coherent publication, exact lock, source fingerprint and three-way reimport rules. All role schemas are versioned; unknown role or critical feature fails closed.

## Typed source graph

The existing executable families are Terrain, Presentation, LocalObject, Item, Creature, Ability, Effect, Formula and Loot, with existing Behavior. V2 adds a distinct declarative `WorldObject` family plus typed NPC, Dialogue, Service, Interaction, Quest, Transition, House and Encounter families. Their common exact identity and typed references are validated across both sets. Each declarative family can retain typed source observations (`Text`, `Integer`, `Boolean`, `SourceId`) with namespaced field selectors. These fields have no implicit authoritative interpretation and cannot contain the existing native-item binding. A declarative record is explicitly candidate-only, even when it refers to an executable definition. The existing `LocalObject` transition vocabulary and `TransitionBinding` remain the only currently owned executable transition semantics.

World records bind an authored world key, `WorldId`, coordinate frame, half-open finite `WorldBounds2D`, and a nonempty ordered set of valid `FloorId`s under `oteryn-world-spatial-v1`. Placement occurrences have a stable key, exact map revision, world and typed definition revision, `(x:i32,y:i32,floor:i16)` position inside its world and an explicit `(plane:i32,order:u32)` presentation order unique at that tile. They contain no live instance, current state or durable owner. Authored placement records are candidate-only; they are not automatically target-qualified, compiled, activated or sent to clients. Presentation order does not grant interaction or mutation priority.

NPC declarations may reference exact presentation, behavior, dialogue and service definitions; they do not own conversation state, trade, currency or durable progress. Dialogue, Service, Interaction, Quest, Transition, House and Encounter declarations carry typed source observations and remain candidate-only until their respective owner contracts/evidence close executable fields. The provenance role only references an exact validated import batch and checks its source revision/digest; new family importers require their own qualified import profile before source observations can claim reproducibility. No Lua, arbitrary callback or implicit source-ID-to-native binding is admitted. House ownership/rent/ACL, quest progress and rewards, and encounter activity/cooldown stay with their runtime/durable owners.

Appearance bindings connect an exact Presentation definition to a stable asset key/revision whose digest is inventoried. Binding is authoring data, not an appearance-ID-derived gameplay identity or automatic client entitlement. V2 editor aliases are noncanonical lookup hints: no alias can resolve or redirect gameplay identity, and duplicate aliases within one family fail admission. Tags are namespaced author/editor metadata and cannot change collision, eligibility, transition, RNG, loot, value, owner selection or client gameplay projection. A gameplay use requires a separately owned typed schema revision. V1 `EXPLICIT_UNSUPPORTED_V1` remains unchanged.

## Verification and supersession

Test v1 byte preservation, explicit migration, v2 round-trip and regrouping independence, all family variants, stable definition/placement separation, wrong-family and missing-revision references, duplicate identity/alias, unknown fields/roles, manifest/digest/lock corruption, metadata authority exclusion, candidate-only lowering, bounded inputs and filesystem containment. Repository CI and the normal protected Merge Queue qualify the implementation.

What becomes harder later: changing a published v2 field or authority classification requires a new schema revision and migration. Evidence that warrants supersession includes a real family corpus, owner-accepted gameplay semantics, measured source cost or a security/compatibility defect. Permanent World Bundle format, full NPC/Quest/House/Encounter mechanics, production maxima and Studio UI remain separately owned decisions.
