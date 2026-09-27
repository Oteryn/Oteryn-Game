# OTV2-20260927-item-authoring-followups-lowering-v1

```yaml
task_id: OTV2-20260927-item-authoring-followups-lowering-v1
title: Item authoring #952 review follow-ups plus a v1 semantic-promotion lowering candidate
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/compassionate-albattani-s29syw
issue: 162
pr: null
base_sha: f404eedb3acef2a0d5ba50ce1f15c34cec118c05
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: owner-launched Claude Code session
created_at: 2026-09-27T00:00:00Z
updated_at: 2026-09-27T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/item-authoring/verify_formal_schema.py
  - tools/content-schema/item-authoring/formal-schema-validation-report.json
  - tools/content-schema/item-authoring/source_field_catalogs.py
  - tools/content-schema/item-authoring/test_engine_items.py
  - tools/content-schema/item-authoring/README.md
  - tools/content-schema/item-authoring/lower_promotion_packet.py
  - tools/content-schema/item-authoring/test_lower_promotion_packet.py
  - tools/content-schema/item-authoring/samples/promotion-crystal-ff7ede5.json
  - docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md
  - .github/workflows/item-authoring-schema.yml
  - docs/agents/tasks/active/OTV2-20260927-item-authoring-followups-lowering-v1.md
  - docs/agents/tasks/archive/OTV2-20260926-item-authoring-formal-schema-v1.md
public_contracts: []
depends_on:
  - "PR #952 protected-merged (Item authoring schema v1)"
  - docs/agents/tasks/archive/OTV2-20260926-item-authoring-formal-schema-v1.md
blocks: []
cross_repository_coordination_id: null
external_repositories:
  - zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a
  - opentibiabr/canary@47dfd51f45280a59a1d3e50ba7edd573d7234446
jira: KAN-16
```

## Outcome

Close the independent review's accepted follow-ups on PR #952 (formal-schema negative
probes for the two new engine-default states, a fail-closed duplicate-target-key guard
on the Crystal binding loader plus its test, a doc sentence on where
`population_census.py --check` runs), and add a v1 *candidate* lowering of this
package's own authored Crystal Item data into the exact typed representation
`apps/game-server/src/content/cw2_b1_import.rs` already decodes for Item semantic
promotion. Content tooling only: no Rust source touched, packet not wired in.

## Architecture and source of truth

- PROVEN: `docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md` and
  `tools/content-schema/item-authoring/**` (PR #952, merged
  `f404eedb3acef2a0d5ba50ce1f15c34cec118c05`) are the current Item authoring formal
  schema, validator and converter.
- PROVEN: `apps/game-server/src/content/cw2_b1_import.rs`
  (`validate_item_semantic_promotion_packet`, `decode_item_semantic_promotion_value`,
  `apply_item_semantic_promotion`, `ITEM_SEMANTIC_PROMOTION_*`, read-only here) is the
  exact accepted decoder for 9 field paths, fed today by the hand-compiled
  `docs/agents/evidence/OTV2-20260923-content-world-item-semantic-promotion.json`
  (`OTERYN_ITEM_SEMANTIC_PROMOTION/v1`).
- DERIVED: `lower_promotion_packet.py` re-encodes converted, zero-validator-error
  Crystal Item bundles into that same 9-field-path/typed-value shape, at population
  scale, without claiming to be the wired packet; its
  `schema`/`profile`/`status`/`next_action` are distinct literal strings from the
  pinned Rust constants and the existing packet's own values.
- UNKNOWN: whether/when the Content/World import role wires this candidate in; that
  needs its own pinned constants, a bespoke apply function, a Rust integration test
  and that role's own review.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline schema/validator/tooling and docs only; no Rust source,
production mutation, session fence, controller or recovery evidence is touched.
`lower_promotion_packet.py`'s output is an unwired JSON candidate, not a runtime
artifact.

## Acceptance criteria

- [x] `verify_formal_schema.py` adds a positive and two negative probes each for
  `ENGINE_CLASSIFICATION_TABLE_MAX_TIER` (wrong max_tier for a classification; missing
  classification) and `ENGINE_CONSTANT_MANTRA_DAMAGE_TYPES` (wrong element;
  out-of-range index), each asserting its own specific error string; the generator and
  report stay byte-deterministic (242/242, up from 236/236).
- [x] `source_field_catalogs.py`'s `_load_crystal_item_bindings` fails closed when two
  external ids bind to the same target key, with a package test driving it against a
  synthetic fixture catalog (the committed binding file has no such row).
- [x] The package README and
  `docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md` §5a each state that
  `population_census.py --check` needs the pinned engine checkouts and runs locally,
  not in CI.
- [x] `lower_promotion_packet.py` runs `engine_items`/`validate_item` over the whole
  pinned Crystal population, keeps only zero-validator-error bundles, and re-encodes
  their authored values for the Rust decoder's 9 field paths in its exact typed
  representation; a value this package's schema allows but the Rust decoder's bounds
  do not is skipped and counted, never smuggled in as a wrong/clamped value.
- [x] A Python-side validator (`validate_row`/`validate_packet`) mirrors every checked
  rule in `decode_item_semantic_promotion_value` and the packet-level partition/
  ordering/uniqueness checks, fail-closed, and runs on every emitted packet.
- [x] The committed output `samples/promotion-crystal-ff7ede5.json` supports
  `--check` (byte-compare) and `--self-check` (Magic Sword `3288` name/attack/defense,
  a container capacity item, a charges item); its size (~3.4 MiB) was confirmed under
  the ~5 MiB reporting threshold before committing to this design.
- [x] `test_lower_promotion_packet.py` (no network) covers the encode/decode mirror
  and `build_packet`/`validate_packet`, wired into
  `.github/workflows/item-authoring-schema.yml`.
- [x] The package README documents what the packet is, how to regenerate/check it,
  and that wiring it into `cw2_b1_import.rs` is left to the Content/World import role.
- [x] `docs/agents/tasks/active/OTV2-20260926-item-authoring-formal-schema-v1.md` is
  archived with its final head/merge commit (`f404eedb`, PR #952) and this follow-up
  record exists.
- [ ] Changed-path checks and independent review pass on the frozen head.

## Excluded scope

No Rust source change, no wiring of the lowering packet into `cw2_b1_import.rs`, no
Canary lowering packet (Crystal only, matching the existing wired packet), no corpus
migration, no runtime activation, no Merge Queue/automerge action, no change to the
six real-source examples or their evidence.

## Implementation / findings

- `verify_formal_schema.py`: imported `validate_non_source_default`,
  `FORGE_CLASSIFICATION_MAX_TIER` and `ENGINE_CONSTANT_MANTRA_DAMAGE_TYPES` from
  `validate_item`, added 6 direct unit-style probes (mirrors the existing
  `supplement_route_probe` pattern of calling a validator function directly). A
  structurally valid Item cannot have `forge.max_tier` without `forge.classification`
  (required together), so "missing classification" calls
  `validate_non_source_default` directly with `{"forge": {}}`, not through
  `validate_real_example`.
- `source_field_catalogs.py`: `_load_crystal_item_bindings` takes an optional `path`
  (default the pinned file; test-only override) and tracks `keys_seen` alongside
  `index`, failing closed the moment a second external_id claims a target key another
  external_id already holds.
- README/architecture doc: one clarifying sentence each.
- `lower_promotion_packet.py`: reuses `engine_items.load_engine_sources`/
  `convert_item`/`validate_item.validate` like `population_census.py`. `typed_value`
  mirrors `decode_item_semantic_promotion_value` encoding (incl. gcd-reduced
  canonical `RATIONAL_PERCENT`); `validate_row`/`validate_packet` mirror it checking,
  run on every build. Full pinned-Crystal run: 13,292 rows / 10,674 Items, 0 skipped
  (`charges.count` 121, `container.capacity` 453, `presentation.name` 10,674,
  `protection.armor` 429, `weapon.attack` 621, `weapon.defense` 636,
  `weapon.extra_defense` 160, `weapon.hit_chance` 56, `weapon.range_cells` 142),
  3,500,315 bytes.
- `test_lower_promotion_packet.py`: unit tests for every `typed_value`/`validate_row`
  branch plus an end-to-end `build_packet` test against a synthetic Crystal
  population (no network), incl. the decoder-bounds skip path.
- Task record: archived the predecessor with its real, GitHub-verified merge
  commit/tree/parent and check-run outcome (24/24 completed, none failing).

## Validation

### Focused

- `python build_formal_schema.py && git diff --exit-code -- .` (twice): PASS, no
  drift beyond the intended `verify_formal_schema.py`/
  `formal-schema-validation-report.json` changes
- `python verify_formal_schema.py`: PASS, 242/242 (up from 236/236)
- `python validate_item.py synthetic-valid-item.json synthetic-valid-dependencies.json
  --manifest synthetic-valid-import-readiness.json`: PASS; valid=true, errors=[]
- `python test_engine_items.py`: PASS, 293/293 (up from 291/291)
- `python test_lower_promotion_packet.py`: PASS, 55/55
- `python lower_promotion_packet.py --source <pinned Crystal checkout>
  --self-check`/`--check`: PASS both; counts above; self-check covers Magic Sword
  `3288` (`oteryn:item.registry.i00003167`), container `116` (capacity 15), charges
  item `814` (count 200)
- `python population_census.py --engine {crystal,canary} ... --self-check --check`:
  PASS both, unaffected by this task
- `ruff check`/`ruff format --check tools/content-schema/item-authoring`: PASS

### Component/integration

- `python tools/content-schema/validate_item_master_schema.py`: PASS, 71 fields / 50
  families / 22 profiles / 0 unassigned
- `python tools/content-schema/test_validate_item_master_schema.py`: PASS,
  positive=2 negative=5
- `python tools/content-census/g4_item_crystal_binding_generator.py --check`: PASS
  bindings=38157 bytes=10864254 (unaffected; confirms no duplicate-target-key row)
- `python tools/agents/validate_governance.py`: PASS

### E2E

- scenario: NOT_APPLICABLE; this task has no runtime consumer (schema/validator/
  tooling/docs and one unwired candidate JSON artifact only)

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing session
- material findings: confirmed "missing classification" cannot occur through a
  structurally valid Item, so that probe calls the validator function directly rather
  than through `validate_real_example`; confirmed the full Crystal lowering packet
  (13,292 rows, ~3.4 MiB) is well under the ~5 MiB report threshold before committing
  to the design
- verdict: PASS on the unpublished candidate; exact remote head review remains before
  freeze

## Independent review

- required: YES; touches the validator's test surface and a new population-scale
  generated artifact, even though no Rust source changes
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none known
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: review follow-ups (A) and the v1 lowering candidate (B) implemented and locally validated; predecessor task record archived (C)
status: implementing
branch: claude/compassionate-albattani-s29syw
pr: null
final_head_sha: null
owner_action_required: null
blocker: null
next_action: stage the exact allocated delta, publish, freeze one remote candidate and request independent review
```
