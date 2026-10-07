# OTV2-20261006 Quest Explore/Area Candidate Qualification

status: implementing
base_main: f6894e793c162d9d8a43578332f3a6f77d2936a3
branch: codex/quest-explore-area-candidates-20261006
runtime_activation: false

## Goal

Qualify Quest `explore` stage targets against current canonical `Area` content using exact existing Area identity only. Preserve all ambiguity and unresolved fine-grained location/object targets; do not infer containment, coordinates or runtime entry semantics.

## Owned paths

- `tools/content-schema/quest-authoring/quest_explore_area_candidates.py`
- `tools/content-schema/quest-authoring/test_quest_explore_area_candidates.py`
- `tools/content-schema/quest-authoring/samples/server-completion/explore-area-candidates.json`
- `docs/agents/tasks/OTV2-20261006-quest-explore-area-candidates.md`

No canonical Area, Quest or runtime file is changed.

## Canonical Area population inspected

The generator deterministically consumes **970** canonical Area records:

- cities: 22
- regions/subregions: 443
- hunting places: 445
- islands: 60

Streets remain unpopulated and are not invented.

Matching aliases are limited to:

1. canonical Area `name`;
2. canonical Area identity key tail.

Comparison only normalizes punctuation/case. Fuzzy matching, coordinate inference and containment inference are forbidden.

## Explore result

All **239** current `explore` stages are classified:

- `EXACT_SINGLE_AREA_ONLY_TARGET`: **10**
- `EXACT_SINGLE_AREA_WITH_OTHER_TARGETS`: **11**
- `AMBIGUOUS_MULTIPLE_AREAS`: **34**
- `NO_EXACT_AREA`: **184**

Therefore:

- exact canonical Area identity candidates: **21**
- clean one-target candidates: **10**
- native spatial bindings: **0**
- runtime admission: **0**

## Why 10 and 11 are separate

A stage that names one exact Area plus another unresolved object/location is not equivalent to a stage whose only target is that Area.

Example:

`Cults of Tibia / s2` has targets:

- `Barkless hideout`
- `Misguided hideout`

Only `Misguided hideout` resolves exactly to:

`oteryn:content.area.subregion.misguided_hideout`

The stage therefore keeps:

- `OTHER_STAGE_TARGETS_UNRESOLVED`
- `AREA_ENTRY_SEMANTICS_NOT_PROVEN`
- `SPATIAL_OCCURRENCE_BINDING_PENDING`

It is not promoted to a complete explore binding.

## Clean examples

### Hidden Threats / s1

`Corym Mines` resolves uniquely to:

`oteryn:area.hunting_place.corym_mines`

This is an exact identity candidate only. The Area record does not prove the exact runtime occurrence/entry condition required by the Quest stage.

### Orcsoberfest / s1

`Orcsoberfest Island` resolves uniquely to:

`oteryn:area.island.orcsoberfest_island`

Again, no runtime binding is created.

## Ambiguity examples

### Asura Palace

The same exact name exists as:

- `oteryn:area.hunting_place.asura_palace`
- `oteryn:content.area.subregion.asura_palace`

The stage remains `AMBIGUOUS_MULTIPLE_AREAS`.

### Goroma

Exact Area identities exist as hunting place, island and subregion. No family is preferred heuristically.

## No-exact examples

Fine-grained stage targets such as:

- `Pirate Ship upper deck`
- `Thais time-travel machine`
- `Temple of Light reward room`
- `Mintwallin throne room`

are not forced onto a broader Area by substring, coordinate guess or containment inference.

## Architecture boundary

An Area identity is content identity, not a Quest occurrence.

Before any of the 21 candidates can become a native binding, an accepted spatial owner must prove:

1. which Area representation owns the stage occurrence;
2. what event constitutes progress (entry, presence, use/interaction inside the Area, etc.);
3. how the runtime observes that event without polling or duplicated world authority;
4. the exact `QuestTransitionRequest` cause/occurrence identity;
5. how stages with additional unresolved targets are composed.

Until then every exact candidate remains:

- `native_spatial_binding: null`
- `runtime_admitted: false`

## Validation

Focused validation:

- canonical Area inventory: **970**
- explore stages: **239**
- exact Area candidates: **21**
- clean one-target candidates: **10**
- dedicated unittest: **7/7 PASS**
- generator `--check`: PASS
- committed packet byte-for-byte drift test: PASS
- `git diff --check`: PASS

## Next action

Escalate/allocate the spatial Quest-trigger owner for the 10 clean candidates first. The 11 mixed-target candidates need their non-Area targets resolved before they can be considered stage-complete. The 34 ambiguous and 184 no-exact rows remain evidence holds and must not be auto-selected.
