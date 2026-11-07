# CATALOG_EXPORT_V1 and G-2

Owner scope: 2026-11-07, base `d34807f5b`, lane offset +60. No migration,
command id, capability bit, mechanic ordinal or registry number is allocated.
No Atlas exporter, production publication or activation is changed. The owner
has approved the catalogue artifact step in the existing protected-main Rust job.

## Shared loader and view

Quest schema changes must also refresh the offline Quest provenance chain, not
only canonical definitions. Run `reward_claim_variant_migration.py`,
`bundle_authoring.py`, `amend_monk_source_receipt.py`, then the dependency-ordered
Quest generators documented in `tools/content-schema/quest-authoring/README.md`,
including `quest_tree_authoring.py content --source-packet
tools/content-schema/quest-authoring/samples/migration/quest-source-packet.json`.
Replay `quest_donor_source_authoring.py` after canonical definitions settle.
The receipt amendment reconstructs the exact previous bundle and schema bytes
after removing only G-2's optional Quest visibility property; every other SOURCE
field and proof stays pinned (`source_fix_guard.py:46-50`). It rejects unrelated
schema changes, changed SOURCE records and visibility on another definition.
Validate with `run_checks.py --part checks`, `--part donor`, `--part unit-tests`,
RewardClaim discovery and both RewardClaim `--check` commands. Repeating the
generators must leave the committed checkout clean.

`oteryn_gameplay_content::content::public_content_view` accepts the immutable
`WorldProject` returned by `capture_world_project` / `ProjectSnapshot::parse`.
This is the same loader used by the server's `NpcDataCatalogue::load`.
The adapter never reparses content, donor data, family migration shards or
names into native identity. Its CLI reads `content/world` directly on each run;
no intermediate normalized input or catalogue is committed.

The view projects Item and Creature definitions, player Ability profiles with
an explicit incantation, NPC declarations and their item offers, Quest and
Achievement declarations, and direct Creature-to-Item loot rows. Stable keys
and definition revisions come from the validated graph. Creature stats and
spell requirements come from its typed authoring profiles. Item fields retain
their known/unknown envelopes. Display names, formulas, quest objectives,
source-only fields and nested loot are not invented from source evidence.
The local coverage ledger explicitly classifies every reference definition
and declaration; it is never serialized to Platform because it may name hidden
identities. Candidate records are data evidence, not runtime availability.

The 24-family small fingerprint fixture covers all populated structural fields
and extensible `field_path` names in this graph. The coverage test compares it
with the admitted graph; a new field/category fails the test. Unknown JSON
members and enum variants also fail the existing typed loader. Update the
projection or explicit unsupported disposition before regenerating the fixture:

```sh
cargo run --locked --quiet -p oteryn-gameplay-content --bin oteryn-game-platform-catalog -- content/world --inventory unused
```

## Explicit unresolved capability gate

**CATALOG is BLOCKED for supported public payloads.** Export contract §15
requires an explicit capability payload contract and matching fail-closed
Platform validator before promoting a capability. Neither a native key nor a
candidate authoring record is that evidence. `PublicContentView::native_input`
therefore deliberately emits all eight capabilities as `unsupported/unknown`,
with no entities, relations or tombstones. It cannot be switched to supported
with a CLI flag. `producer.py` remains the sole physical validator and
canonicalizer. Do not represent this artifact as a populated public catalogue.
The exact Platform boundary pinned by the export contract,
`Oteryn/Oteryn-Platform@20f8aac95ae1b890ec6ebe8a705dda7dfb6674d4`,
`docs/contracts/OTERYN_V2_GAME_CATALOG_CONTENT_CONTRACT.md:7`, explicitly says
`Implementation: deferred`; no matching capability-payload proof is supplied
by that accepted boundary. Platform validator work is outside this Game branch.

There is also a concrete source seam still requiring integration: the served
WorldProject has no Quest, Area or Achievement declarations. The server loads
quest state separately through
`crates/gameplay/quests/src/quest/loader.rs::parse_quest_state`; quest source
definitions live in `content/quests/definitions`, while the served mission
catalogue is generated in `content/quests/missions/quest-state.json`.
Reading the family shards as an independent exporter parser would violate
the requested shared-loader boundary. A populated catalogue must first expose
those owning loaders through a common admitted public source and have Platform
validate the resulting exact payload revision. This change does not claim that
editing the separate family shards updates a supported Platform catalogue.

## G-2

`PublicVisibility` is the closed `public|hidden` enum on the shared v2 Quest,
NPC and Area declarations. Absence defaults to public; default serialization
is omitted, preserving existing canonical project bytes and pins. The source
authoring JSON schemas accept the same optional enum. Invalid visibility
values fail validation. Visibility grants no runtime availability or authority.
The public candidate view omits hidden identities entirely; a hidden NPC also
loses every shop relation. Secret achievements are omitted conservatively.
No public artifact includes the private coverage ledger.

Platform consumer clarification: CATALOG_EXPORT_V1 contains **public records
only**. An optional `visibility` field anywhere in entity/relation `data`,
including nested objects and arrays, must be exactly the string `public`.
`hidden`, unknown strings, null and non-string values are rejected by both
producer and verifier, even with a valid recomputed payload digest. Invalid
records are never repaired by dropping the field. Absence preserves the
existing carrier shape; top-level record fields remain the closed v1 shape.
This is an export-boundary invariant, not permission to export a hidden source
record with its flag removed. The source view must omit that record and its
relations first. These are owner/Platform export rules, not donor gameplay rules.

The quest schema pin is regenerated with the existing
`amend_monk_source_receipt.py` tool, which verifies all 28 reviewed cores and
their SOURCE/recipe admission before refreshing compiler descriptors. Then
`quest_tree_authoring.py content --root . --output .` and
`quest_state_lowering.py --root .` regenerate the dependent metadata. Their
data-file changes are SHA256 provenance links only; quest rules and counts
remain unchanged. No receipt or generated digest is hand-edited.

## CI-ready artifact build

Installed after `Build workspace` in `.github/workflows/rust.yml`, job `linux`:

```sh
python3 tools/game-platform-catalog/build.py "$RUNNER_TEMP/game-platform-catalog.json"
```

The next step verifies it with `producer.py verify`, then the existing pinned
`actions/upload-artifact` action uploads JSON and `.json.sha256` as
`game-platform-catalog`. Triggers, permissions, secrets and other workflow
settings are unchanged. The exact workflow policy pin is regenerated with
`python3 tools/game-platform-catalog/refresh_workflow_pin.py`; that helper accepts
only these three approved steps over the previously pinned workflow. Its
`--check` mode and policy mutation tests retain fail-closed pin validation.

This checks that `content/` matches HEAD, invokes the Rust loader/view, and
drives `producer.py` to write JSON plus `.json.sha256` outside the repository.
Source revision is HEAD; `generated_at` is the commit timestamp normalized to
UTC seconds, never wall clock. Equal revision/content yields equal bytes and
digests. The current artifact proves unsupported semantics only.

```sh
cargo test --locked -p oteryn-gameplay-content public_view_coverage_visibility_and_determinism
cargo test --locked -p oteryn-gameplay-content --test public_visibility
python3 -m unittest discover -s tools/game-platform-catalog -p 'test_*.py'
python3 tools/game-platform-catalog/producer.py verify "$RUNNER_TEMP/game-platform-catalog.json"
```

## Atlas follow-up

Existing `tools/game-atlas-*` exporters can replace their content parsing with
a thin Rust caller of `public_content_view` over the same admitted revision.
Atlas must consume only visibility-filtered records and respect unsupported
capabilities; it must never consume the private coverage ledger or use donor
identity as native identity. Promote and version the exact public payloads
together with Platform validation, then make Atlas formatting a downstream
consumer of that view. No Atlas refactor is included here.

## Evidence and rule boundaries

G-2, identity omission and default-public behavior are owner-approved product
rules, not donor gameplay rules. Canonicalization, bounds, provenance and the
unsupported gate are the accepted export contract §§3–15. No gameplay formula,
drop probability conversion, merchant price inference or runtime rule is added.
The view merely carries admitted typed values; donor hypotheses are not proof
of native completeness.

The source distinction matters: Crystal
`201189f871d23597b82caf88fd611454a2ddefb8`,
`data-global/npc/ruprecht.lua:54–76,95–124`, proves token/item exchanges rather
than a generic gold-priced shop (retained evidence:
`docs/reference/content-prep/world-src-1105/ruprecht-native.md:14`). Do not
infer currency or runtime authority from an NPC's name. Likewise the Canary /
Crystal `data/scripts/spells/healing/light_healing.lua:1` difference is retained
at `docs/architecture/reviews/OTERYN_GAME_CANARY_PARITY_INVENTORY_2026-10-25_SOURCE_DIFF_LEDGER.md:320`;
the catalogue does not recalculate or choose a healing formula. These examples
justify retaining unsupported semantics instead of filling gaps from donors.

## Additive revision proposal: schema 1.1.0, `area` (scheduled FOLLOW-UP)

**Design only; not implemented or advertised by the current producer.** The
coordinator-scheduled Area source-admission / bundle change is a separate
follow-up. No SQL migration is needed, but it does not fit the current catalogue
adapter package: it changes the compiler-owned World bundle manifest contract,
the corresponding server decoder/profile and regenerated bundle pins. Merely
adding an Area parser to this exporter would violate the shared-loader rule.
The current v1 verifier continues to reject both capability `area` (even when
unsupported) and schema `1.1.0`; the existing eight-capability output is unchanged.

### Existing source and smallest server-owned seam

The 443 region/subregion definitions and 22 city definitions already live at
`content/world/areas/{regions,cities}/areas-*.json`, under
`OTERYN_AREA_AUTHORING/candidate-1`. The same records must remain the only
editable source. Islands, streets and hunting-place catalogues have separate
schemas; they are explicitly unsupported by this proposal, with no name-based
crosswalk or invented geometry. Dawnport population remains excluded by the
owner's lane rules.

There is an existing admitted geographic carrier despite the absence of Area
declarations in WorldProject: the compiler's
`tools/world-bundle-compiler/src/compile/subareas.rs::Source::load` reads these
shards and admits source masks into `oteryn_world_bundle::subarea::Table`.
The server uses `bundle::read` through its map boot loader
(`crates/gameplay/world/src/map/boot.rs:296`). Its decoded bundle contains
`manifest.subareas`, spawn points and NPC placements. `Table::resolve`
(`crates/world-bundle/src/subarea.rs:76`) is the existing membership lookup;
label positions and hometown temples are not area boundaries.

However, the current `subarea::Area` carrier
(`crates/world-bundle/src/subarea.rs:21`) loses source `visibility`, `kind` and
`identity.revision`. `Source::load` currently constructs it without those
fields. An old bundle therefore cannot prove G-2 visibility, even if a new
exporter supplies a default `public`. Do not export those bundles as public
Area authority or infer kind/revision from names or keys.

The minimal follow-up is to extend that existing carrier, not create a second
Area list: retain the source definition revision, closed area kind and G-2
visibility during compiler admission. Validate missing source visibility as
public and reject values outside `public|hidden`. Bind this admission to a
versioned `PUBLIC_AREA_SOURCE_V1` bundle profile; the public view must refuse
area promotion for a bundle without that proof marker. The bundle owner must
choose the required format/runtime compatibility revision and server-first
rollout under `OTERYN_WORLD_BUNDLE_FORMAT_V1.md`; old bundle bytes are never
relabelled or treated as visibility-qualified.

Add an admitted-bundle entry point beside `public_content_view`, taking the
already server-decoded, digest-checked bundle and admitted WorldProject.
Require the bundle's content revision and content-lock digest to match that
project and its committed World pin. Keep this qualification loader-owned:
derive a read-only carrier from the original bounded bytes, not from a
caller-populated `Bundle.digest` or reconstructed manifest DTO.
The CLI uses that same decoder and proof
checks for an explicit matching bundle input; it never parses area JSON or
bitmap assets. A changed area is recompiled by the existing content/bundle
tooling and then appears in the view without a catalogue-specific re-export or
editable duplicate. Existing v1 callers remain unchanged.

### Proposed payload, identity and visibility

Schema `1.1.0` adds one capability, `area`; region is a kind, not another
capability. An entity retains the existing carrier, with `type: area`,
`capability_id: area`, and `content_key` equal to the source's native key, e.g.
`oteryn:content.area.region.thais`. Numeric source/town ids, display names and
asset hashes are never canonical identity. The exact `data` keys are:

| Field | Contract |
| --- | --- |
| `payload_revision` | Integer constant `1`. |
| `definition_revision` | Source `identity.revision`, 1–128 UTF-8 bytes. |
| `name` | Admitted display name, 1–128 characters and at most 256 UTF-8 bytes; no exporter normalization. |
| `kind` | `region`, `subregion` or `city`, retained from source. |
| `visibility` | Required constant `public`. |
| `is_hometown` | Admitted boolean; no citizenship, access or runtime availability claim. |

No raw source ids, masks, coordinates, temples, provenance blobs or hidden
identity lists are part of this minimal payload. Export only public areas with
public ancestors; omit a hidden area's descendants as well, rather than leak a
hidden parent key or silently reparent a child. Regions have no parent edge;
subregions/cities have exactly one public region parent. Unknown/malformed
visibility, duplicate keys, unknown fields and invalid/cyclic parents fail
source admission. The private coverage ledger records omitted/unsupported
entries with reasons; it is never serialized to Platform/Atlas.

### Relations to areas, creatures and NPCs

All three relation types use capability `area` and the existing relation carrier:

| Type | Source → target | Exact data |
| --- | --- | --- |
| `area_parent` | Area → its public parent region | `payload_revision: 1`, `basis: declared_parent`. |
| `area_creature` | Area → exported public creature | `payload_revision: 1`, `basis: admitted_spawn_position`. |
| `area_npc` | Area → exported public NPC | `payload_revision: 1`, `basis: admitted_npc_position`. |

Use each admitted spawn point's absolute x/y and source floor, or each admitted
NPC placement's x/y/floor, with the same bundle's `Table::resolve`. A successful
lookup may produce subarea and parent-region relations. Deduplicate each
`(type, source_key, target_key)` tuple. Its stable relation key is
`oteryn:relation.<type>.<sha256>`, where the lowercase digest is over canonical
UTF-8 JSON `[type,source_key,target_key]` (no LF). This avoids placement-order ids.
Source and target must both exist in the same snapshot. A hidden NPC, hidden
area/ancestor or unsupported target capability produces no public relation.

Missing/unadmitted mask coverage stays unknown: never use nearest label,
rectangle guesses, donor coordinates or a temple to manufacture membership.
Relations mean static admitted placement evidence, not current occupancy,
roaming range, player access or executable/reachable availability. Record
`membership_not_admitted`, `hidden_endpoint`, `unsupported_endpoint`, or
`excluded_scope` reasons privately. Do not infer authoritative absence from
missing relations or publish an unqualified source-key crosswalk.

### Bounds, completeness and rollout

Retain server bounds: at most 1,024 areas, 512 masks, 262,144 mask runs and
4,194,304 admitted tiles (`subarea.rs`); at most 131,072 spawn points
(`spawn.rs::MAX_POINTS`) and 2,048 NPC placements (`npc.rs::MAX_PLACEMENTS`).
Reuse the compiler's existing bounded shard capture (32 directory entries and
16 MiB per source read). Area keys/names are at most 256 UTF-8 bytes; native
creature/NPC keys retain their existing 128-byte bounds. Parent depth is one
edge to a region. At most two membership edges per placement plus 1,024 parent
edges gives a maximum 267,264 area relations before deduplication.

Global catalogue limits remain 256 MiB, 200,000 entities, 1,000,000 relations,
2,048 bytes per string, signed-64-bit integers and depth 16. Family and global
limits both apply and fail closed; there is no truncation. Preserve canonical
sorting and semantic/transport digest rules. No new numeric registry allocation
or migration is reserved by this design.

After matching Game payload and Platform validators exist (§15), initial area
coverage is at most `supported/partial`; unsupported source families, excluded
population and unadmitted geometry prevent an authoritative-empty or complete
claim. Before that proof exists, a future 1.1.0 producer declares area
`unsupported/unknown`. No area tombstones are allowed under partial/unknown.
Roll out Game admission/decoder and pinned bundle, then Platform's inactive
1.1.0 validator/importer, then enable a qualified catalogue profile. Retain v1
artifacts for old consumers, which reject 1.1.0 rather than silently dropping
the new category. Activation remains a separate owner gate.

Follow-up acceptance tests must use the real bundle writer/server reader and
cover visibility propagation/defaults, rejection of unqualified old bundles,
hidden-parent and hidden-NPC relation suppression, endpoint integrity,
unadmitted mask coverage, bounds, full source-entry/field coverage and
byte/digest determinism after a one-place source edit. Source evidence is the
existing admitted mask/town contracts, not a new donor inference: the World
bundle contract §§16–17 and pinned Crystal
`00ce02a57ca5a12e48f32a3476e37471167e4c3f`, `data-global/world/world.otbm`
(town sample `tools/content-schema/area-authoring/samples/crystal-world-towns-00ce02a5.json`).
