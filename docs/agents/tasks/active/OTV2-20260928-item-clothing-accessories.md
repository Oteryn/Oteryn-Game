# OTV2-20260928-item-clothing-accessories

```yaml
task_id: OTV2-20260928-item-clothing-accessories
title: Old rag resolves event_collectible via wiki status=event; ivory comb stays material_valuable via wiki
mode: MIGRATE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/compassionate-albattani-s29syw
issue: 162
pr: null
base_sha: 4add578a
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
  - tools/content-schema/item-authoring/build_formal_schema.py
  - tools/content-schema/item-authoring/item.schema.json
  - tools/content-schema/item-authoring/samples/population-crystal-ff7ede5.json
  - tools/content-schema/item-authoring/samples/population-canary-47dfd51f.json
  - tools/content-schema/item-authoring/samples/promotion-crystal-ff7ede5.json
  - tools/content-schema/item-authoring/README.md
  - tools/content-census/item_wiki_family_capture.py
  - imports/tibiawiki/batches.json
  - imports/tibiawiki/sources.json
  - imports/tibiawiki/facts/items-family-fallback.json
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

Reverses the prior implementation of this task (commit `4add578a`, PR #162), on a
corrected 2026-09-28 owner decision: `clothing accessories` is a TibiaWiki *secondary*
type, never the item's real family, so `PRIMARYTYPE_PROFILE` never admits it as a
`primarytype` alias. TibiaWiki's own infobox `status = event` field is instead admitted
(lowest priority, only when neither `primarytype` nor `objectclass` resolves) as
`event_collectible`, because old rag (24415) is TibiaWiki's own 20th-anniversary
event-only drop (`status = event`), not a creature product. Ivory comb (32773) keeps
resolving to `material_valuable` through its own, already-committed wiki fallback
record (`primarytype = Creature Products`), unaffected by the alias revert.

## Architecture and source of truth

- PROVEN: in Crystal and Canary, exactly two items carry `primarytype`
  "clothing accessories" as their only native engine attribute: old rag (24415) and
  ivory comb (32773). Neither resolves from `engine_attribute`; both depend entirely on
  their own wiki fallback record.
- PROVEN from English TibiaWiki:
  - Ivory comb has `primarytype` Creature Products and `secondarytype` Clothing
    Accessories: its real family is Creature Products, captured directly in the wiki
    fallback snapshot.
  - Old rag's own wiki page carries `status = event`: TibiaWiki files it as dropped only
    during the 20th anniversary event, not as an ordinary creature product.
- DERIVED: `clothing accessories` names no one real family on its own (it is a secondary
  type, and TibiaWiki files other Clothing Accessories items outside Creature Products
  too), so it stays an unadmitted `primarytype` value; the admitted `status = event` rule
  is the correct, narrower fact for old rag specifically.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline authoring tooling only.

## Acceptance criteria

- [x] The `"clothing accessories": "material_valuable"` `PRIMARYTYPE_PROFILE` alias
  (and its test) is reverted; `clothing accessories` is restored to the never-resolve
  list.
- [x] `resolve_wiki_family_value` admits a `status` field (only `event` ->
  `event_collectible`), consulted only when `primarytype`/`objectclass` both fail;
  wired through the loader, `build_formal_schema.py`'s `field` enum (schema
  regenerated), and the capture tool's infobox parsing/field-priority.
- [x] The wiki fallback snapshot is recaptured; old rag resolves via `status = event`,
  ivory comb still resolves via its pre-existing direct record.
- [x] `imports/tibiawiki/batches.json`/`sources.json` are updated to the recaptured
  snapshot's digest; `test_wiki_fallback_snapshot_is_registered` and the record-count
  assertion in `test_committed_wiki_fallback_snapshot_loads_fail_closed` (949) pass.
- [x] The censuses and the promotion packet are regenerated, and `--check` passes.
  `routed_non_item` is unchanged (25,656 / 25,193); both items still fully resolve
  (only their `family_profile_basis`/evidence changed), so `not_converted`/
  `fully_resolved` are unchanged too.
- [ ] Required checks pass on the frozen PR head.

## Excluded scope

- `fireworks` remains unmapped, because no unresolved item currently uses it.

## Implementation / findings

| | Crystal | Canary |
|---|---|---|
| `family_profile_unresolved` (with the alias, superseded) | 945 | 904 |
| `family_profile_unresolved` (without the alias, baseline) | 946 | 905 |
| `family_profile_unresolved` (status=event rule, this task) | 945 | 904 |
| fully resolved (this task) | 10,654 | 10,317 |
| `family_profile_basis: engine_attribute` | 10,679 | 10,357 |
| `family_profile_basis: wiki_evidence_fallback` | 877 | 837 |

Old rag and ivory comb both still fully resolve (net counts identical to the superseded
alias-based implementation); only their basis changed from `engine_attribute` to
`wiki_evidence_fallback`. Newly resolved via the new `status = event` rule, repository-wide:
old rag (Crystal/Canary id `24415`, registry key `oteryn:item.registry.i00023587`) — the
only item in either engine's unresolved universe whose wiki page carries `status = event`.

## Validation

- `test_engine_items.py`: PASS (368 checks, including new `status`-field and real-snapshot
  old-rag/ivory-comb tests).
- `build_formal_schema.py` regenerated twice: no further diff (idempotent);
  `verify_formal_schema.py`: PASS (242 checks).
- `--check --self-check` on both censuses: PASS. `lower_promotion_packet.py --check`: PASS.
- `ruff check`/`ruff format --check`: PASS. `validate_governance.py`: PASS.
- `g4_item_crystal_binding_generator.py --check`: PASS.

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
