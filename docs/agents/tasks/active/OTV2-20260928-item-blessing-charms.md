# OTV2-20260928-item-blessing-charms

```yaml
task_id: OTV2-20260928-item-blessing-charms
title: Classify blessing charms as progression material and restore the wiki batch registration
mode: MIGRATE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/compassionate-albattani-s29syw
issue: 162
pr: null
base_sha: 13efb4fc
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: owner-launched Claude Code session
created_at: 2026-09-28T00:00:00Z
updated_at: 2026-09-28T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/item-authoring/engine_items.py
  - tools/content-schema/item-authoring/test_engine_items.py
  - tools/content-schema/item-authoring/samples/population-crystal-ff7ede5.json
  - tools/content-schema/item-authoring/samples/population-canary-47dfd51f.json
  - tools/content-schema/item-authoring/samples/promotion-crystal-ff7ede5.json
  - tools/content-schema/item-authoring/README.md
  - tools/content-census/item_wiki_family_capture.py
  - imports/tibiawiki/facts/items-family-fallback.json
  - imports/tibiawiki/batches.json
  - imports/tibiawiki/sources.json
  - docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md
  - .github/workflows/item-authoring-schema.yml
  - docs/agents/tasks/archive/OTV2-20260927-item-wiki-family-fallback.md
  - docs/agents/tasks/active/OTV2-20260928-item-blessing-charms.md
public_contracts: []
depends_on:
  - "docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
jira: KAN-16
```

## Outcome

This task applies the owner's 2026-09-28 decision that blessing charms belong to the
`progression_material` family. It also repairs two follow-up gaps left after #1040.

## Architecture and source of truth

- PROVEN: blessing charms are single-use items. Using one grants exactly one blessing,
  and the charm is consumed. The Crystal `ff7ede5` sources show this:
  - `data/scripts/actions/items/blessing_charms.lua` registers the use action.
  - `data/libs/systems/blessing.lua` maps each blessing to its charm in `Blessings.All[*].charm`.
  - The charm ids are 10341–10345, 25360 and 25361.
- PROVEN: 10341–10345 carry `primarytype` "blessing charms" in `items.xml`.
- PROVEN: 25360 and 25361 carry no `primarytype`. English TibiaWiki has "Heart of the
  Mountain (Item)" and "Blood of the Mountain (Item)", both with `primarytype`
  "Blessing Charms".
- PROVEN: #1052 (`ac6d820b`) removed only the `g5-item-family-fallback-tibiawiki-r1`
  entries from `imports/tibiawiki/batches.json` and `sources.json`. It changed nothing
  else in those files.
- PROVEN: after #1040, `lower_promotion_packet.py --check` reported drift against the
  committed `samples/promotion-crystal-ff7ede5.json`.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this task changes only offline authoring tooling and import evidence.

## Acceptance criteria

- [x] `PRIMARYTYPE_PROFILE` maps "blessing charms" to `progression_material`. The capture
  tool no longer treats that value as pending an owner decision.
- [x] The wiki snapshot is recaptured with 948 records, 2 more than before
  (heart of the mountain and blood of the mountain). Apart from `captured_at`,
  no other record changed.
- [x] The `g5` batch and source entries are restored with the new snapshot and tool digests.
- [x] `test_wiki_fallback_snapshot_is_registered` guards that registration.
- [x] The workflow `paths:` now also cover `imports/tibiawiki/{batches,sources}.json`
  and the capture tool.
- [x] Both censuses and the promotion packet are regenerated, and `--check` passes on all three.
- [ ] The required checks pass on the frozen PR head.

## Excluded scope

- Clothing Accessories (old rag) still waits for an owner decision.
- The runtime effect of a blessing charm is not in scope.
- The `docs/agents/evidence/` copy pinned by #1048 is not re-pinned here. That belongs
  to the promotion-pass assignment.

## Implementation / findings

| | Crystal | Canary |
|---|---|---|
| `family_profile_unresolved` | 953 → 946 | 912 → 905 |
| fully resolved | 10,646 → 10,653 | 10,309 → 10,316 |

The promotion packet now covers 11,555 Items with 14,173 fields. At the #1048 pin it
covered 10,674 Items with 13,292 fields. Only `presentation.name` grows.

## Validation

- `test_engine_items.py`: 353 checks, PASS.
- `verify_formal_schema.py`: PASS.
- `test_lower_promotion_packet.py`: PASS.
- `--check --self-check` on both censuses: PASS.
- `lower_promotion_packet.py --check`: PASS.

## Self-review

- exact head: pending
- method/reviewer: implementing session
- material findings: the registry drop caused by #1052, repaired here and guarded by a test
- verdict: pending

## Independent review

- required: NO. The change is an owner-decided family mapping plus evidence repair.
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: #1040, #1048, #1052
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: blessing charms admitted, snapshot recaptured, registration restored, censuses and promotion packet regenerated
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
next_action: publish PR and drive CI to green
```
