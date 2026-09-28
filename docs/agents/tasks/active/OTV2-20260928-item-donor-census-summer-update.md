# OTV2-20260928-item-donor-census-summer-update

```yaml
task_id: OTV2-20260928-item-donor-census-summer-update
title: Donor census of new ids from the Crystal summer-update donor (task B1a)
mode: MIGRATE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/compassionate-albattani-s29syw
issue: 162
pr: null
base_sha: 32ccd294
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: owner-launched Claude Code session
created_at: 2026-09-28T14:00:00Z
updated_at: 2026-09-28T15:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/item-authoring/donor_census.py
  - tools/content-schema/item-authoring/test_donor_census.py
  - .github/workflows/item-authoring-schema.yml
  - tools/content-schema/item-authoring/README.md
  - tools/content-schema/item-authoring/samples/donor-census-crystal-summer-update-00ce02a5.json
  - docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md
  - docs/agents/tasks/archive/OTV2-20260928-item-owner-leftover-table.md
  - docs/agents/tasks/active/OTV2-20260928-item-donor-census-summer-update.md
public_contracts: []
depends_on:
  - "docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md"
  - "docs/agents/tasks/archive/OTV2-20260928-item-owner-leftover-table.md"
blocks: []
cross_repository_coordination_id: null
external_repositories:
  - "zimbadev/crystalserver@00ce02a57ca5a12e48f32a3476e37471167e4c3f (summer-update, donor facts only)"
jira: KAN-16
```

## Outcome

Owner rule: an upstream donor checkout is only a source of FACTS; Oteryn decides
classification with its own rules, never upstream's. Donor source: Crystal's own
upstream `zimbadev/crystalserver` branch `summer-update` @ `00ce02a5` (full sha
`00ce02a57ca5a12e48f32a3476e37471167e4c3f`). Its `items.xml` has 38,568 ids against
38,157 in pinned `ff7ede5`: 412 new, 244 in the new 15.30 appearance range
`52977-55117`, 168 older ids simply absent from the pinned 15.25-era revision.

Added `donor_census.py`, reusing `engine_items.py`'s classification helpers directly
(`non_item_route`, `classify_family_profile`, `immovable_non_item_route`,
`resolve_wrap_target_profile`, `resolve_dead_item_route_or_profile`, the
fluid/late-placeholder/no-appearance/empty-object last-resort routes,
`fallback_entry_matches_name`) in `convert_item`'s own priority order, starting after
identity resolution instead of before it -- `engine_items.py` itself is untouched.
**No identity minted**: rows are keyed by the provisional
`donor:crystalserver@00ce02a5:item/<id>`, never `oteryn:item.registry.*`; identity
allocation is a separate, reviewed Content/World step (B1b), out of this task's scope
along with delivery-task eligibility, field mapping and Presentation binding.

Committed census: `samples/donor-census-crystal-summer-update-00ce02a5.json` (schema
`OTERYN_ITEM_DONOR_CENSUS/v1`), deterministic, `--check`/`--self-check` like
`population_census.py`. Classifies all 412 new ids: 261 resolved, 138 routed
non-Item, 13 unresolved for lack of wiki evidence this task cannot supply (structural:
the committed wiki-evidence snapshot and the owner leftover-family table are both
keyed by Oteryn registry key, which no donor id has -- the join is attempted anyway,
for exact fidelity with `convert_item`'s own order, and reports zero matches by proof,
not omission). Full breakdown: formal doc §5l.

Archived `docs/agents/tasks/active/OTV2-20260928-item-owner-leftover-table.md` to
archive/ (status `completed`, "merged as PR #1109 (`a2591387`) from final head
`39985136d5ac73e0d04cbe4512a815bee5139cd4`", no pending fields left).

## Architecture and source of truth

- PROVEN (facts, matched the owner's own measurement exactly): donor items.xml 38,568
  ids, base 38,157, new 412 (244 in `52977-55117`, 168 older) -- reproduced
  independently from the pinned digests, not taken on faith.
- PROVEN: donor items.xml sha256 `13a8773e...`, appearances.dat sha256 `17a72b30...`
  (pinned in `donor_census.py`; digest-verified on every run via the same
  `read_verified_artifact` `engine_items.py` uses).
- PROVEN: no donor-only id has a CW2 B1 allocator key (`build_identity_index()`'s own
  key set never contains a `donor:` string, checked directly in
  `test_donor_census.py`).
- PROVEN: both pinned population censuses stay byte-identical
  (`population_census.py --check`, both engines, exit 0) -- `engine_items.py` is not
  modified by this task at all (confirmed: `git status` shows no change to it).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline authoring tooling only; no identity, no production data.

## Acceptance criteria

- [x] `donor_census.py` added: donor/base id set-difference; classification reuses
  `engine_items.py`'s helpers directly, unmodified, in `convert_item`'s own order.
- [x] Provisional `donor:crystalserver@00ce02a5:item/<id>` key, never
  `oteryn:item.registry.*`; no identity minted, allocator/bindings/definitions
  untouched.
- [x] Committed, deterministic donor census with pinned commit + artifact digests,
  totals split by 15.30-range vs. older, outcome/profile/routed/blocker counts, and
  per-id name/outcome/basis; `--check`/`--self-check` pass.
- [x] Tests: donor ids never include base ids; provisional key format; wiki/owner-table
  lookups structurally never match a `donor:` key; a few real donor ids (routed,
  engine-attribute-resolved, wrap-target-resolved) decode as expected.
- [x] Existing censuses proven byte-identical (`population_census.py --check`, both
  engines).
- [x] Formal doc §5l and README updated (donor principle, no-identity constraint,
  B1b/B2/B3 follow-ups).
- [x] `#1109` task record archived with no pending fields.

## Excluded scope

- No CW2 B1 identity minted for any donor id; no edit to the allocator, bindings, or
  `content/items/definitions` (B1b, a separate reviewed Content/World step).
- No wiki recapture for donor ids (B2); the join is attempted but structurally always
  misses, reported as a proven fact.
- No folding of donor facts into the pinned engine revision (B3); the pinned
  `items.xml`/`appearances.dat` digests are unchanged.
- No delivery-task eligibility, field mapping or Presentation binding for donor ids
  (all need identity).

## Implementation / findings

| | Value |
|---|---|
| donor items.xml ids | 38,568 |
| base (`ff7ede5`) items.xml ids | 38,157 |
| new ids | 412 |
| new ids in 15.30 range (52977-55117) | 244 |
| older new ids (pre-15.25) | 168 |
| resolved | 261 |
| routed non-Item | 138 |
| unresolved (no wiki evidence possible) | 13 |
| `wiki_evidence_fallback_resolved` | 0 (structural) |
| `owner_leftover_table_resolved` | 0 (structural) |

Resolved by profile: `decoration` 127, `material_valuable` 78, `document` 24,
`weapon_melee` 14, `equipment_armor` 5, `weapon_magic` 4, `weapon_distance` 4, `plant`
2, `light_source` 2, `food` 1 (257 by engine attribute, 4 by wrap-target inheritance).
Routed: `WorldObject:corpse` 80, `WorldObject:immovable_unclassified` 24,
`Terrain:ground_or_border` 16, `Terrain:primarytype_world_object` 14,
`WorldObject:primarytype_world_object` 4. Unresolved (13): `sample of bluish tide
veil`, `sample of bluish whisper reed`, `lunar ascension orb`, `empty crystal flask`,
`shell gauge`, `key`, `moonsilver crystals`, `auric moon sigil`, `crystal flask with
blue lava`, `crystal flask with blessed blue lava`, `skewered fish`, `scraps of a
radiant attire`, `cloud in a bottle`.

## Validation

### Focused

- command/run: `python test_donor_census.py`; `python test_engine_items.py`
- result: PASS (40 checks; 591 checks, unaffected)

### Component/integration

- command/run: `verify_formal_schema.py`; `test_lower_promotion_packet.py`;
  `ruff check`/`format --check` on `tools/content-schema/item-authoring` (repo root);
  `validate_governance.py`
- result: `verify_formal_schema` 242/242 (schema untouched by this task).
  `test_lower_promotion_packet` 55/55. `ruff check`: exit 1 -- 1 pre-existing E402 in
  `verify_formal_schema.py` (not an owned path, untouched); `ruff format --check`:
  exit 0. `validate_governance`: passes.

### E2E

- scenario: `donor_census.py --check --self-check`; both `population_census.py --check
  --self-check`; `lower_promotion_packet.py --check`
- result: all exit 0; both population censuses confirmed byte-identical to their
  pre-task committed state.

### Exact-head CI

- final head/trigger/workflow/runner/classification/result: `NOT_APPLICABLE` (local
  worktree task, no PR opened in this interaction)

## Self-review

- exact head: `db3ccf9b` (local, not pushed/frozen)
- method/reviewer: implementing session
- material findings: the wiki-evidence and owner-leftover-table joins are structurally
  guaranteed to miss for every donor id (both keyed by Oteryn registry key, which no
  donor id has) -- proven by test and reported as an explicit zero count, not silently
  skipped
- verdict: pass

## Independent review

- required: NO; a census/reporting tool with no identity, production or authority
  effect, same class as prior tasks
- exact head/method/auditor/material findings/verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: pending; unresolved review threads: `NOT_APPLICABLE` (no PR)
- related/superseded PRs: continues `OTV2-20260928-item-owner-leftover-table` (PR
  #1109); precedes B1b (identity allocation) and B2 (wiki recapture)
- protected auto-merge: `NOT_APPLICABLE`; merge commit/result: pending; ownership
  release: pending

## Context checkpoint

```yaml
last_progress: donor_census.py implemented and tested; census committed; existing censuses proven byte-identical; #1109 task record archived; formal doc/README updated; committed locally to item-donor-census at db3ccf9b
status: implementing
branch: claude/compassionate-albattani-s29syw
head_sha: null
pr: null
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
next_action: commit locally on item-donor-census; no push/PR in this interaction
```
