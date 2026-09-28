# Item authoring schema candidate v4

This package turns the Item Master Schema v1 census into an executable authoring
contract. It validates one portable Item definition, not a placed map object and not a
mutable item instance.

Architecture and boundaries:
[`docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md`](../../../docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md).

| File | Purpose |
|---|---|
| `build_formal_schema.py` | Single source for the schemas, profile catalog and templates. |
| `item.schema.json` | One capability-composed portable Item schema for every family. |
| `item-dependencies.schema.json` | Exact definitions, presentations, assets and admitted proficiency crosswalks required by the Item. |
| `item-import-readiness.schema.json` | Per-source-field mapping/disposition ledger. |
| `real-source-evidence.schema.json` | Closed evidence record for the six real examples, including exact source pins and typed field links. |
| `profile-catalog.json` | The 22 census profiles, their Wiki families and expected capabilities. |
| `wiki-field-dispositions.json` | Exact 71-field Wiki disposition registry generated from the protected census. |
| `fandom-field-dispositions.json` | Exact 84-row historical Fandom migration-table registry; corroboration only. |
| `wiki-real-item-field-supplement.json` | Exact 14-field delta observed on six pinned current TibiaWiki BR item pages. |
| `fandom-real-item-field-supplement.json` | Exact 5-field delta observed on the same six pinned Fandom item pages. |
| `canary-field-dispositions.json` | Exhaustive pinned Canary ledger: 143 parser keys plus root, nested, appearance and reverse-relation inputs. |
| `crystal-field-dispositions.json` | Exhaustive pinned Crystal ledger: 143 parser keys plus root, nested, appearance and reverse-relation inputs. |
| `source_field_catalogs.py` | Generated-ledger source of truth, aliases, defects and owner routing. |
| `proficiency_profiles.py` | Pinned Canary/Crystal proficiency sources and admitted canonical static profiles. |
| `real_item_examples.py` / `real-source-examples.json` | Six generated, validated real-item examples with source evidence and explicit blockers. |
| `templates/*.json` | Thirteen valid starting points for materially different authoring shapes. |
| `validate_item.py` | Structural, semantic, exact-reference and import-readiness validation. |
| `verify_formal_schema.py` | Focused positive/negative contract checks and deterministic fixtures. |
| `engine_items.py` | Converts one pinned Crystal/Canary `items.xml` + `appearances.dat` into candidate Item bundles: identity allocator, family_profile/taxonomy rules, field mapping, appearance/Presentation binding, and the `delivery_task_eligible` decision from the `ADOPT_CRYSTAL_DELIVERY_LIST@ff7ede5` authoring rule. Digest-verifies every input artifact first (text artifacts after CRLF->LF normalization, `appearances.dat` as exact raw bytes); a missing pinned artifact is a hard error. |
| `delivery-task-overrides.json` | Per-Item exceptions to the Delivery Task adoption rule (`{key: {eligible, reason}}`), strictly validated; starts empty. |
| `population_census.py` | Runs `engine_items` over an engine's full item universe, validates every emitted bundle (the real `delivery_task_eligible` decision, not a proposal), and writes one deterministic outcome census (counters, top blockers/validator errors, per-raw-field coverage, Delivery Task decision/observation/crystal-list counts, and a `routed_non_item` owner/reason breakdown). `--self-check` runs required engine-specific assertions for both engines; `--check` diffs an in-memory regeneration against the committed file instead of writing. |
| `test_engine_items.py` | Fixture-checkout tests for `engine_items`/`population_census`: LF/CRLF digest portability, per-engine Delivery Task pool parsing, the Crystal-list adoption rule and its per-item overrides, plus direct synthetic-`sources` tests (real identity index and disposition catalogs, fabricated `items`/`appearances`) for every implemented field family and value route. Run with `python test_engine_items.py`. |
| `samples/population-crystal-ff7ede5.json`, `samples/population-canary-47dfd51f.json` | Committed census outputs for the two pinned engine revisions. |
| `lower_promotion_packet.py` | Lowers every zero-validator-error Crystal Item bundle's authored values for the 9 field paths (`presentation.name`, `weapon.attack`/`defense`/`extra_defense`/`range_cells`/`hit_chance`, `protection.armor`, `charges.count`, `container.capacity`) that `apps/game-server/src/content/cw2_b1_import.rs`'s `decode_item_semantic_promotion_value` accepts into a candidate `OTERYN_ITEM_SEMANTIC_PROMOTION_LOWERING/v1` packet, in that decoder's exact typed representation. `--self-check` checks named Crystal ids (Magic Sword `3288`, a container, a charges item); `--check` diffs an in-memory regeneration against the committed file instead of writing. See "Item semantic-promotion lowering" below. |
| `test_lower_promotion_packet.py` | No-network fixture tests for `lower_promotion_packet.py`'s encode/decode mirror of the Rust decoder and its `build_packet`/`validate_packet` wiring, against the same kind of synthetic `sources` `test_engine_items.py` uses. Run with `python test_lower_promotion_packet.py`. |
| `samples/promotion-crystal-ff7ede5.json` | Committed candidate lowering packet for the pinned Crystal revision. |

The profiles are guidance inside one schema. Missing a common capability produces a
warning; optional capabilities preserve the wider census union without warning noise.
This avoids making an exceptional but valid combination impossible. Unknown
fields fail closed, so map placement, collision, pathing, source numeric IDs and mutable
instance state cannot silently enter an Item definition.

```text
pip install -r requirements.txt -r requirements-dev.txt  # dev: ruff==0.16.1
python build_formal_schema.py && git diff --exit-code -- .
python verify_formal_schema.py
python validate_item.py synthetic-valid-item.json synthetic-valid-dependencies.json --manifest synthetic-valid-import-readiness.json
```

CI: `.github/workflows/item-authoring-schema.yml` runs these steps plus
`test_engine_items.py`, `test_lower_promotion_packet.py`, the Item Master census and
Ruff on every PR touching the package. `population_census.py --check` and
`lower_promotion_packet.py --check`/`--self-check` need the pinned Crystal/Canary
checkouts, so neither is a CI step; each stays a local check run by hand before a
whole-population claim is made.

Engine population census (pinned Crystal/Canary checkouts, digests verified before read
on both LF and CRLF checkouts; Crystal also needs its existing delivery list, Canary
needs the `weeklyItems` table in `data/modules/scripts/taskboard/settings.lua`, plus a
`--rule-source` Crystal checkout for the Delivery Task rule below):

```text
python population_census.py --engine crystal --source /path/to/crystalserver --self-check
python population_census.py --engine canary --source /path/to/canary --rule-source /path/to/crystalserver --self-check
python population_census.py --engine crystal --source /path/to/crystalserver --check
python population_census.py --engine canary --source /path/to/canary --rule-source /path/to/crystalserver --check
python engine_items.py --engine crystal --source /path/to/crystalserver --id 3288
python engine_items.py --engine canary --source /path/to/canary --rule-source /path/to/crystalserver --id 3031
python test_engine_items.py
```

`population_census.py` is evidence tooling: it proves what the pinned engine sources
convert to under this schema today, not a corpus migration or Game truth. It never
commits per-item rows, only counters and capped examples in `samples/`.

### Item semantic-promotion lowering (v1, not yet wired)

`apps/game-server/src/content/cw2_b1_import.rs` already decodes one hand-compiled Item
semantic-promotion packet
(`docs/agents/evidence/OTV2-20260923-content-world-item-semantic-promotion.json`,
`OTERYN_ITEM_SEMANTIC_PROMOTION/v1`) into the existing Reference Item family for 9
field paths: `presentation.name`, `weapon.attack`/`defense`/`extra_defense`/
`range_cells`/`hit_chance`, `protection.armor`, `charges.count`,
`container.capacity`. `lower_promotion_packet.py` proves this package's own authored
Item data can systematically feed that same decoder, at population scale, rather than
by hand: it runs `engine_items`/`validate_item` over the whole pinned Crystal
population exactly like `population_census.py`, keeps only Items whose bundle
validates with zero errors, and re-encodes their authored values for those 9 field
paths in that decoder's exact typed representation (`TEXT`, `SIGNED_POINTS`, `CELLS`,
canonical-reduced `RATIONAL_PERCENT`, `COUNT_U32`, `CAPACITY_U16`) — a value this
package's own schema allows but the Rust decoder's narrower bounds do not is skipped,
counted, and never smuggled in as a wrong or clamped value. Every row is then checked
again by `validate_packet`/`validate_row`, a fail-closed Python restatement of the
Rust decoder's own rules (shape, kind, source/typed equality, bounds, ordering,
uniqueness, count partition), before the packet is written.

```text
python lower_promotion_packet.py --source /path/to/crystalserver --self-check
python lower_promotion_packet.py --source /path/to/crystalserver --check
```

The committed output is `samples/promotion-crystal-ff7ede5.json`: 14,174 rows over
11,556 Items (`charges.count` 121, `container.capacity` 453, `presentation.name`
11,556, `protection.armor` 429, `weapon.attack` 621, `weapon.defense` 636,
`weapon.extra_defense` 160, `weapon.hit_chance` 56, `weapon.range_cells` 142;
~3.4 MiB). Its `schema`/`profile`/`status`/`next_action` are deliberately different
literal strings from the pinned Rust constants and from the wired packet's own values,
so this candidate can never be mistaken for, or silently accepted as, the wired one.
The #1048 Rust wiring pins an earlier copy (13,292 rows over 10,674 Items, before the wiki
family fallback and blessing charms added Items); re-pinning that copy belongs to the
promotion-pass assignment.

This is a v1 lowering **candidate**, not a Rust-side claim: wiring it into
`apps/game-server/src/content/cw2_b1_import.rs` (pinned byte-count/digest constants
for this packet, a bespoke apply function mirroring `apply_item_semantic_promotion`,
and a Rust integration test, per the existing packet's own pattern) is left to the
Content/World import role, since it is the one that owns and can requalify the target
Item family this would apply to.

### Value-dependent fields and non-Item routing

A `VALUE_DEPENDENT` catalog field (`disposition == "VALUE_DEPENDENT"`, top-level
`status` always `unresolved_semantics`) is resolved per observed value against that
field's own `source_value_routes`: a route whose own `status` is `mapped` is applied
exactly as pinned (`destination_value_equals`/`destination_array_contains`/
`destination_value_transform`) and counts as `mapped`/`converter_missing` like any other
field; a route whose own status is a routed status (e.g. `external_domain`) counts as
`routed`, never a blocker; a value with no route at all is a distinct
`unresolved:<field>=<value>` blocker, so the remaining tail stays diagnosable by value
instead of collapsing into one generic bucket. `flags.unmove` is one such field: **both**
its `true` and `false` routes map to `/item/physical/movable` (`boolean_not_source`); the
field itself does not route to a non-Item owner. An `flags.unmove=true` entry that *does*
resolve a `family_profile` (e.g. a heavy, non-pickupable weapon or container) still
converts as an ordinary Item with `physical.movable: false`.

Some ids are never Item candidates at all: `flags.corpse`/`flags.player_corpse`
(WorldObject/Interaction-owned corpse behavior), a reserved sprite-sheet placeholder
appearance name (`reserved sprite`, `deprecated item`, `empty sprite`, case-insensitive,
with no other `items.xml` attribute), and a `primarytype` naming a Terrain/WorldObject
fixture (`artificial tiles`, `natural tiles`, `fields`, `walls`, `constructions`,
`machines (objects)`, `machines`, `traps`). `convert_item` recognizes these before
attempting `family_profile` classification and reports them as `routed_non_item`
(`{"owner": ..., "reason": ...}`) with `converted: false` and no blockers; the census
counts them under the `routed_non_item` outcome, separate from `not_converted`.

Separately, once `family_profile` classification has already failed to resolve a family,
an `flags.unmove=true` entry (formal schema §5a: immovable map geometry — walls, ground,
borders, top decorations, doors/stairs/ramps/windows — with no Item family profile) is
also routed to `routed_non_item` instead of staying `family_profile_unresolved`: owner
`Terrain`/reason `ground_or_border` when `flags.bank`, `flags.fullbank` or `flags.clip` is
set, otherwise owner `WorldObject`/reason `immovable_unclassified`. An `unmove=false` (or
absent) entry with no resolved family keeps `family_profile_unresolved`, since that case
stays editorial backlog rather than a known non-Item owner decision.

A field this converter cannot implement because the schema needs data neither pinned
engine's evidence supplies keeps a precise blocker rather than a guessed value:
`flags.upgradeclassification`/`upgradeclassification.upgrade_classification`
(`/item/forge` also requires `max_tier`, which only a Wiki infobox field supplies),
`augments` (the nested augment tree's target/value kind has no admitted crosswalk),
`mantra` (its `damage_types` are not derivable from the single `points` attribute),
`runespellname` (no admitted Ability identity crosswalk is wired into this package), and
`proficiency.proficiency_id` (the one admitted crosswalk, Magic Sword/238, still cannot
satisfy `validate_item.py`'s requirement that both the Canary *and* Crystal source
identities corroborate the same target, which a single-engine converter run can never
offer alone; every id, including 238, keeps `converter_missing:proficiency.proficiency_id`
plus a `proficiency_crosswalk_not_admitted:<id>` blocker). `flags.forceuse` is `UNRESOLVED`
with no route at all and is intentionally left as `unresolved:flags.forceuse`.

Successful validation proves authoring shape and declared dependency closure only. It
does not prove runtime lowering, gameplay parity, corpus migration or production
activation.

Dependency closure is exact in both directions: every used reference/asset must be
declared and every declared entry must be used. Import readiness likewise requires one
disposition for every entry in each source's explicit field inventory. `source_profile`
selects one pinned registry; unknown fields and wrong engine revisions fail closed.
Every mapped row is restricted to an explicit formal JSON Pointer (including
array-index patterns where required). Registered-but-ineffective engine keys are
recorded as `pinned_no_effect`, and unresolved historical semantics remain
readiness-blocking rather than being guessed into gameplay truth.

`presentation.appearance_binding` is an exact `{family,key,revision}`
`PresentationRef`, never a raw client ID or an unversioned asset string. Its dependency
record preserves the pinned engine source, appearance ID, frame geometry and ordered
sprite IDs. Repeated sprite IDs are valid. A matching raster `sprite_atlas` is a
separate versioned artifact; without it the validator warns because source sprite
numbers alone are not renderable pixels. The real examples retain that warning instead
of inventing a client asset pack. Tests independently lock the complete geometry and
ordered sprite sequence of all six fixtures and reject inline pixel/blob payloads.

`tradeable` and `marketable` are independently optional facts. Market category or
market vocation restrictions require `marketable=true`; an engine Market flag does
not prove general player-to-player tradeability. Numeric source proficiency IDs remain
provenance until a pinned `proficiency_crosswalks` entry binds them to an admitted
exact `ProficiencyRef`. The candidate admits Magic Sword source pair `238`/`3` only when
both the pinned Canary `data/items/proficiencies.json` and Crystal
`data/json/proficiencies.json` identities corroborate the same target. Those two files
are byte-identical at the pinned revisions (SHA-256
`1a915dffd9265cd1c18d39e55da7ede691b2e58add534bc186238ae028a73f22`).
The real Magic Sword Item contains the complete ordered three-level perk matrix with
explicit selection slots, one selectable perk per level, typed percentages/points and
the signed `-30000 ms` cooldown modifier. The exact admitted profile validator rejects
payload drift, missing corroboration, unknown IDs/versions and wrong source artifacts.
Character-owned XP and active perk selections are deliberately not Item fields.

Every Item must declare `delivery_task_eligible` as a JSON boolean. `true` means the
definition may be considered by a future Delivery Task pool; `false` is an explicit
exclusion, not an unknown value. The Item does not carry requested quantities, weekly
rotation, task assignment, delivery state, rewards or reset data. Those concerns stay
with the future task/ruleset system, and this authoring field does not claim that a
runtime consumer already exists. The six real-source examples currently use `false`
as an explicit Oteryn author decision rather than presenting it as a Wiki-derived
Global Tibia fact; the converter's `ADOPT_CRYSTAL_DELIVERY_LIST@ff7ede5` rule (below)
independently reaches the same `false` decision for all six.

### Delivery Task eligibility rule

`engine_items.convert_item` decides `delivery_task_eligible` for every converted Item
under the owner-approved authoring rule `ADOPT_CRYSTAL_DELIVERY_LIST@ff7ede5`
(`engine_items.RULE_ID`): an Item is eligible iff the Crystal id sharing its CW2 B1
allocator key is a member of the digest-pinned Crystal delivery list
(`data/scripts/lib/task_board_delivery_items.lua`) at
`ff7ede593c69d4c658b382c97443e8155926924a`, unless `delivery-task-overrides.json`
records an explicit per-Item exception. This is an Oteryn authoring decision applied to
Crystal-list evidence, not an engine fact.

Both engines already share one numeric item-id space through the CW2 B1 allocator (the
same `item_id` that resolves identity also resolves the rule), so a Crystal run reads the
list as its own pinned Delivery Task pool and a Canary run reads it through a required
`--rule-source <crystal checkout>` (see the CLI examples above); a Canary run started
without one is a hard error, never a silent "nothing is eligible". Each engine's *own*
Delivery Task pool (Crystal's delivery list; Canary's `weeklyItems` table) remains
separate upstream *observation* and never decides eligibility by itself — a Canary item
can be a `weeklyItems` member while the rule still finds it ineligible, because it is not
a Crystal-list member. The per-item conversion report keeps both apart:

```json
"delivery_task": {
  "observation": {"source": "canary_task_board_weekly_items", "member": true},
  "decision": {
    "rule": "ADOPT_CRYSTAL_DELIVERY_LIST@ff7ede5",
    "basis": "crystal_list_non_member",
    "eligible": false
  }
}
```

`basis` is `crystal_list_member`, `crystal_list_non_member`, or `override` (with a
`reason`) when `delivery-task-overrides.json` names that Item key. The overrides file is
`{"schema": "OTERYN_ITEM_DELIVERY_TASK_OVERRIDES/v1", "rule": "<RULE_ID>", "overrides":
{"<item key>": {"eligible": <bool>, "reason": "<non-empty string>"}}}`; it starts empty,
is validated strictly (unknown keys, wrong types, or an override for an Item key the CW2
B1 allocator never assigned all fail), and `--overrides <path>` on either CLI substitutes
it for testing. An Item with no CW2 B1 allocator key gets no decision at all: it keeps
the `identity_not_in_b1_catalog` and `delivery_task_decision_not_admitted` blockers and
never converts, so it never carries `delivery_task_eligible`.

The BR profile is pinned to stable source `424807`; the Fandom profile is pinned to
historical revision `1035268` and its revision SHA-1. Per-capture SHA-256 remains a
separate required digest. Value-dependent source fields carry `source_value` so the
validator can prove conditional owner routing and normalizations (including signed
weight, `unmove`/`immobile`, item type, equip events, weapon actions and weapon kinds).

The 71-field BR and 84-row historical Fandom base catalogs remain unchanged. The two
real-page supplements are bounded overlays: each stores six exact page IDs, revision
IDs, timestamps, MediaWiki SHA-1 values, UTF-8 wikitext SHA-256 values and complete
sorted raw-parameter inventories plus the timezone-qualified capture time. Coverage
tests require
`observed - base - supplement == empty`; unknown values still fail closed.

Every real example is validated as one bundle. `field_evidence` and
`non_source_defaults` exactly partition every authored scalar Item leaf. Each evidence
record links its typed value to exact raw Canary/Crystal/BR/Fandom observations and to
an allowed catalog destination; external, unresolved, approved-omission and fabricated
fields cannot prove an Item value. Non-source defaults are restricted to an admitted
destination/state/value registry, so they cannot be used to add an unrelated field.
The routed catalog destination must equal the evidenced leaf or be its JSON-pointer
ancestor, and every admitted observation must deterministically normalize from its raw
source value to that exact typed leaf. Correlated edits to the Item and evidence copy
therefore cannot conceal source-value drift. A separately pinned canonical SHA-256 for
each example's complete extracted `source_observations` matrix also rejects a correlated
edit of the raw observations, typed proof and Item value together.
The validator also proves the Item key, appearance ID, Presentation ref,
ordered sprite IDs, engine definition pins, Wiki page pins and unresolved blocker set.
Changing the Item value, the recorded source observation or their typed normalization
makes the bundle invalid. Engine manifest fields are additionally restricted to their actual origin:
`items.xml`, `appearances.dat` or (Crystal only) `bags.xml`, and each cited engine
artifact must carry its pinned SHA-256.

An example keeps the canonical Item key bound to its TibiaWiki BR page in
`imports/tibiawiki/bindings/items.json` (Magic Sword: `oteryn:item.registry.i00003167`).
Examples whose page has no binding yet keep a provisional key and the
`canonical_item_identity_not_bound` blocker.

Weights are exact decimals with two fractional digits (`42.00 oz` = engine weight
`4200`). The auxiliary schemas reference `item.schema.json` definitions by `$id`, like
the Monster package. The Monster package's Item projection still uses
`weight_centioz`, `collision` and a bare `asset_binding`; see the architecture document
for how it maps to this schema.
