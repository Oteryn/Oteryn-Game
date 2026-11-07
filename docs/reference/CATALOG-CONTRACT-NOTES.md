# CATALOG_EXPORT_V1 and G-2

Owner scope: 2026-11-07, base `d34807f5b`, lane offset +60. No migration,
command id, capability bit, mechanic ordinal or registry number is allocated.
No workflow, Atlas exporter, production publication or activation is changed.

## Shared loader and view

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

The quest schema pin is regenerated with the existing
`amend_monk_source_receipt.py` tool, which verifies all 28 reviewed cores and
their SOURCE/recipe admission before refreshing compiler descriptors. Then
`quest_tree_authoring.py content --root . --output .` and
`quest_state_lowering.py --root .` regenerate the dependent metadata. Their
data-file changes are SHA256 provenance links only; quest rules and counts
remain unchanged. No receipt or generated digest is hand-edited.

## CI-ready artifact build

One-line workflow step for the owner to add (no workflow edit in this lane):

```sh
python3 tools/game-platform-catalog/build.py "$RUNNER_TEMP/game-platform-catalog.json"
```

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
