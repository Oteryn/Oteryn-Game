# OTV2-20260928-item-clothing-accessories

```yaml
task_id: OTV2-20260928-item-clothing-accessories
title: Classify clothing accessories (old rag, ivory comb) as material valuables
mode: MIGRATE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/compassionate-albattani-s29syw
issue: 162
pr: null
base_sha: f3c2ec57
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
  - docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md
  - docs/agents/tasks/archive/OTV2-20260928-item-blessing-charms.md
  - docs/agents/tasks/active/OTV2-20260928-item-clothing-accessories.md
public_contracts: []
depends_on:
  - "docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
jira: KAN-16
```

## Outcome

Apply the owner's 2026-09-28 decision on the last pending wiki category: the
`clothing accessories` primary type maps to `material_valuable`, the same family as
green piece of cloth. The event family `event_collectible` was considered and not used.

## Architecture and source of truth

- PROVEN: in Crystal and Canary, exactly two items carry `primarytype`
  "clothing accessories": old rag (24415) and ivory comb (32773).
- PROVEN from English TibiaWiki:
  - Green piece of cloth (5910) and ivory comb have `primarytype` Creature Products and
    `secondarytype` Clothing Accessories.
  - Old rag looks the same as green piece of cloth and drops from creatures.
  - During Tibia Anniversary, players consume 30 old rags at a Sewing Table. The rag is
    an event ingredient, not an event prize.
- DERIVED: `event_collectible` covers event prizes, fansite items and party items, so it
  does not fit. `material_valuable` already holds green piece of cloth.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline authoring tooling only.

## Acceptance criteria

- [x] `PRIMARYTYPE_PROFILE` maps "clothing accessories" to `material_valuable`, and a test
  covers the mapping.
- [x] The censuses and the promotion packet are regenerated, and `--check` passes.
  - Old rag now resolves.
  - Ivory comb moves from wiki evidence to its own engine attribute.
  - No routing changed.
- [ ] Required checks pass on the frozen PR head.

## Excluded scope

- `fireworks` remains unmapped, because no unresolved item currently uses it.

## Implementation / findings

| | Crystal | Canary |
|---|---|---|
| `family_profile_unresolved` | 946 → 945 | 905 → 904 |
| fully resolved | 10,653 → 10,654 | 10,316 → 10,317 |

## Validation

- `test_engine_items.py`: PASS.
- `--check --self-check` on both censuses: PASS.
- `lower_promotion_packet.py --check`: PASS.

## Self-review

- exact head: pending
- method/reviewer: implementing session
- material findings: none
- verdict: pending

## Independent review

- required: NO; owner-decided family mapping
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: #1063
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: clothing accessories mapped, outputs regenerated
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
next_action: publish PR, enable auto-merge
```
