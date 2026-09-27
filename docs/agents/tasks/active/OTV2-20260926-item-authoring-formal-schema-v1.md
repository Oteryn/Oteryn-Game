# OTV2-20260926-item-authoring-formal-schema-v1

```yaml
task_id: OTV2-20260926-item-authoring-formal-schema-v1
title: Formalize the Item authoring master schema and family templates
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/item-authoring-schema-v1-20260926
issue: 162
programme_issue: 504
jira: KAN-16
base_sha: e300c14102f0e78be147d380779cf1265e76a182
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: /root
created_at: 2026-09-26
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260926-item-authoring-formal-schema-v1.md
  - docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md
  - tools/content-schema/item-authoring/**
  - .github/workflows/item-authoring-schema.yml
public_contracts: []
depends_on:
  - docs/architecture/OTERYN_ITEM_AUTHORING_MASTER_SCHEMA_V1.md
blocks: []
cross_repository_coordination_id: null
external_repositories:
  - opentibiabr/canary@47dfd51f45280a59a1d3e50ba7edd573d7234446
  - zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a
```

## Outcome

Produce a generated JSON Schema package for one capability-composed portable Item
definition, an exact dependency catalog, an import-readiness ledger and valid family
templates. This is the owner-requested formal continuation of the protected Item Master
Schema v1 census; no runtime or corpus migration is authorized.

## Architecture and source of truth

- PROVEN: `OTERYN_ITEM_AUTHORING_MASTER_SCHEMA_V1.md` selects one superset Item schema
  with 22 profiles and 50 assigned navigation families.
- PROVEN: protected evidence reports 71 assigned Wiki parameters, 50 assigned families
  and zero unassigned fields/families.
- PROVEN: Canary and Crystal pinned revisions expose definition inputs across appearance,
  XML and Lua layers; their ItemType records cross Oteryn owner boundaries.
- DERIVED: the formal package admits portable static definition facts and routes placed
  object, mutable instance and reverse-relation facts to their existing owners.
- UNKNOWN: runtime lowering, complete-corpus conversion and Global parity.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline schema/documentation/tooling only; no production mutation,
session fence, authority-bearing controller or recovery evidence is touched.

## Acceptance criteria

- [x] Generator reproduces every schema, catalog and template byte-identically.
- [x] All templates structurally and semantically validate.
- [x] Negative tests reject Terrain/WorldObject, ItemInstance and source-ID leakage.
- [x] Canary/Crystal/Wiki review finds no material portable-field or routing omission.
- [x] Six real Item examples validate with exact engine presentation evidence and pinned Wiki page overlays.
- [x] Every Item explicitly declares boolean Delivery Task eligibility without task-runtime state.
- [ ] Changed-path checks and adversarial whole-diff self-review pass on the frozen head.

## Excluded scope

No runtime/compiler changes, corpus migration, schema supersession, production
activation, economy truth, Wiki execution, external repository write or Merge Queue
action.

## Implementation / findings

The package uses one schema, 22 common/optional profile descriptions and thirteen
warning-free valid templates. Exact dependencies and source readiness stay separate
from `item.json`.

Accepted review repairs:

- split mana-shield/reflection/resource/magic modifiers into typed boolean/flat/percent
  forms and reused the existing typed WorldProject/v2 augment shape;
- removed placed bed/fluid-source semantics and made unproven light radius optional;
- made dependencies/assets exact in both directions;
- bound source dispositions to a complete field inventory and routed the 71 Wiki fields
  through explicit owner and formal JSON Pointer rules;
- fixed opposite-hand, transform/decay, readable/write-once, imbuement and uniqueness
  invariants;
- corrected the distance-launcher template and split common vs optional profile signals;
- added exact pinned Canary, Crystal and historical Fandom source-profile registries;
- classified all 143 unique parser keys in each engine, including explicit no-effect
  defects, root/nested/appearance inputs and reverse bag relations;
- added signed presentation weight/display flags, movement speed, invisibility, typed
  mantra/elemental bond, weapon-kind distinctions, chain semantics, dual wielding,
  shortfall policy, wrapping and destroy transforms;
- repaired TibiaWiki BR `modificadores` so editor notes alone cannot satisfy a typed
  mapping and unresolved clauses block readiness;
- pinned BR/Fandom source identity, enabled timezone-qualified capture validation and
  made signed weight, boolean inversion and value-dependent parser routes verifiable;
- added the missing appearance upgrade-classification value leaf.
- replaced the unversioned appearance asset string with an exact `PresentationRef` and
  a dependency payload that preserves pinned appearance ID, geometry and ordered
  sprite IDs while leaving the unavailable raster atlas explicitly unresolved;
- separated independently knowable `marketable` and `tradeable` facts and narrowed
  Canary `flags.market` to `marketable=true`;
- made Canary numeric proficiency IDs provenance-only unless a pinned admitted
  `proficiency_crosswalks` entry closes the exact canonical `ProficiencyRef`;
- kept the exact 71/84 BR/Fandom base catalogs intact and added bounded 14/5-field
  overlays for twelve exact revisions of six real item pages;
- generated and validated Magic Sword, Demon Armor, Backpack, Red Apple, Sudden Death
  Rune and Vial examples with source IDs kept outside canonical Item identity.
- added a closed evidence schema and semantic cross-checks that bind every authored
  field to exact raw engine/Wiki observations, page captures and blocker state;
- restricted engine fields to their real source artifact and admitted raw-field alias,
  initially kept proficiency pair `238`/`3` blocked until an exact canonical target
  was accepted, and locked all six Presentation fixtures plus the raster
  atlas boundary with positive and negative regression checks.
- replaced capability-level evidence with an exact scalar-leaf evidence/default
  partition and reject fabricated, external, unresolved or omission-routed fields as
  proof of an Item value; non-source defaults are also restricted to an admitted
  destination/state/value registry.
- bound every `route_destination` to its evidenced scalar leaf (or exact ancestor) and
  require each raw observation to reproduce the typed leaf through a pinned exact
  boolean, integer, weight, enum or catalog normalization.
- independently pinned the canonical SHA-256 of each complete extracted raw-value
  matrix, closing correlated edits of source observations, evidence and Item together.
- after owner correction, returned frozen head `d77b109ebe5b7a4befa109b2a20dc5cf925d1671`
  to AUTHORING and admitted Magic Sword proficiency pair `238`/`3` against both exact
  Canary and Crystal profile artifacts; the pinned files are byte-identical at SHA-256
  `1a915dffd9265cd1c18d39e55da7ede691b2e58add534bc186238ae028a73f22`.
- added the complete static three-level/six-perk Magic Sword profile with ordered
  selection slots, one-active-per-level policy, typed skill/ability/shield targets,
  signed cooldown milliseconds and exact payload equality; character XP and active
  choices remain rejected as player-owned state.
- closed independent-review regressions for otherwise-valid but unused proficiency
  crosswalks and for any `selection_count` other than exactly one.
- after the owner selected an Item-owned boolean boundary, returned frozen head
  `7e450684bfc8633ec093506c9e040f1b129ad57c` to AUTHORING and added required
  `delivery_task_eligible`; missing, null, string and numeric values fail closed;
- kept pool construction, quantities, weekly assignment, delivery, rewards, reset and
  player progress outside Item authoring for the later task/ruleset design;
- set all thirteen templates and six real-source examples explicitly to `false`, with
  the examples recording an Oteryn author decision rather than claiming source parity.

## Validation

### Focused

- command/run: `python -X utf8 tools/content-schema/item-authoring/verify_formal_schema.py`
- result: PASS, 236/236; 13 templates, 6 real-source examples, 4 Draft 2020-12 metaschemas, generator byte determinism,
  positive bundles and fail-closed boundary/semantic cases
- command/run: `python -m ruff check tools/content-schema/item-authoring`
- result: PASS
- command/run: `python -X utf8 tools/content-schema/item-authoring/validate_item.py ... --manifest ...`
- result: PASS; valid=true, errors=[]; expected warning records that no matching raster sprite atlas is admitted

### Component/integration

- command/run: `test_engine_items.py`
- result: PASS, 291/291
- command/run: `validate_item_master_schema.py` + `test_validate_item_master_schema.py`
- result: PASS; 71 fields, 50 families, 22 profiles, zero unassigned; positive=2 negative=5
- command/run: `tools/agents/validate_governance.py` + lifecycle unit tests
- result: PASS; 22 policy documents, 9 lanes, 13/13 lifecycle tests

### E2E

- scenario: NOT_APPLICABLE; this slice has no runtime consumer
- result: NOT_APPLICABLE

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing agent
- material findings: import owner routing, exact dependency closure, opposite-hand
  reservation, transform ambiguity, warning noise and lint findings repaired before freeze
- verdict: PASS on the unpublished candidate; exact remote head review remains before freeze

## Independent review

- required: YES; cross-source semantic/routing review requested by owner
- exact head: pending
- method/auditor: three read-only Canary, Crystal and Wiki subagents, followed by bounded
  repair verification
- material findings: v1-v3 (trade booleans, appearance binding, Wiki deltas,
  proficiency crosswalk) repaired; 517daa6b review (leaf partition, crashes, digests)
  repaired; audit 56a39d15 (Delivery Task evidence/decision, CRLF, CI) repaired;
  converter completed and blockers resolved; owner adopted
  `ADOPT_CRYSTAL_DELIVERY_LIST@ff7ede5` rule (2026-09-27)
- verdict: implementation repaired; fresh exact-head final review remains pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none known
- protected auto-merge: NOT_AUTHORIZED
- owner authorization: 2026-09-27 owner explicitly authorized `.github/workflows/item-authoring-schema.yml`
  (policy `explicit_owner_authorization_plus_merge_queue`, approvals 0)
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: converter completed (identity from Crystal bindings, non-Item routing, admitted engine defaults); docs updated; next freeze and independent review
status: implementing
branch: codex/item-authoring-schema-v1-20260926
head_sha: 517daa6b017647f52f2632c43c12461e1d0be95d
pr: 952
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: owner selects the v4 repair branch for #952, then freeze and exact-head review
```
