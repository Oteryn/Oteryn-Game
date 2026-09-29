# OTV2-20260928-item-wrap-target-corpses

```yaml
task_id: OTV2-20260928-item-wrap-target-corpses
title: Engine wrap-target family inheritance and corpse-like "dead ..." item routing/profile
mode: MIGRATE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: item-wrap-corpse
issue: 162
pr: 1086
base_sha: 3a11e3f8
head_sha: fcd133f08cc26787301a1b07ea7d41bddc1b58cf
final_head_sha: fcd133f08cc26787301a1b07ea7d41bddc1b58cf
final_head_frozen_at: 2026-09-28T10:02:21Z
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
  - docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md
  - docs/agents/tasks/archive/OTV2-20260928-item-clothing-accessories.md
  - docs/agents/tasks/active/OTV2-20260928-item-wrap-target-corpses.md
public_contracts: []
depends_on:
  - "docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
jira: KAN-16
```

## Outcome

Implements two owner decisions (2026-09-28), both lowest-priority family resolutions
(below every existing engine classifier and the wiki-evidence fallback, so no
already-resolved item ever changes):

1. **Engine wrap-target inheritance**: an unresolved item whose own `items.xml`
   `wrapableto="T"` names another id `T` whose own `primarytype` resolves through the
   existing `PRIMARYTYPE_PROFILE` (one hop only, never chained, never via T's own wiki
   fallback) takes that profile. New basis `engine_wrap_target`, evidence
   `{wrap_target_id, wrap_target_primarytype}`.
2. **Corpse-like "dead ..." items** (scope: still unresolved after 1, engine name
   lower-cased starts with `"dead "`; already-flagged corpses are unaffected, routed
   earlier by the pre-existing `flags.corpse`/`flags.player_corpse` rule): without
   `flags.take`, routed non-Item (`WorldObject`/`corpse_decoration`); with `flags.take`,
   an explicit owner table (`DEAD_CREATURE_PROFILE`) maps the exact lower-cased name to a
   profile (new basis `owner_name_rule`, evidence
   `{rule: "take_able_dead_creature", name}`); an unlisted take-able name stays
   unresolved (fail closed).

## Architecture and source of truth

- PROVEN (both pinned engines, `ff7ede5`/`47dfd51`): 85 unresolved items carry
  `wrapableto` naming an id whose own `primarytype = "furniture"` (`23398` for all but
  the "dragon pinata" pair, which names `23473`, also `furniture`); all 85 resolve to
  `decoration` identically in Crystal and Canary.
- PROVEN: every take-able (`flags.take`), still-unresolved, `"dead "`-prefixed engine
  name in either engine is one of exactly 7 names -- dead troll, rat, snake, spider,
  wolf, rabbit, frog (12 items total) -- and every one is an ordinary animal/creature
  carcass, not a named/proper-noun character.
- PROVEN: every non-take-able, still-unresolved, `"dead "`-prefixed name (14 items,
  identical in both engines) is either a plain map decoration (dead dragon/bear/cyclops
  x2 each, dead goblin x2, dead lava) or a named quest corpse (dead Doctor Perhaps, dead
  Dirtbeard, dead Evil Mastermind, dead Monstor, dead Mephiles) -- none of these five
  named ones carries `flags.take`, so none reaches the owner table at all.
- DERIVED: `DEAD_CREATURE_PROFILE` therefore needs only `material_valuable` entries (all
  7 take-able names); no `quest_item` mapping is needed in this data set.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline authoring tooling only.

## Acceptance criteria

- [x] `resolve_wrap_target_profile` (one-hop `wrapableto` -> target's own `primarytype`
  -> `PRIMARYTYPE_PROFILE`) wired into `convert_item`, below the wiki fallback.
- [x] `resolve_dead_item_route_or_profile` (`corpse_decoration` routing /
  `DEAD_CREATURE_PROFILE` lookup) wired into `convert_item`, below wrap-target
  inheritance.
- [x] `build_formal_schema.py`: `family_profile_basis` enum extended
  (`engine_wrap_target`, `owner_name_rule`); `familyProfileEvidence` extended with both
  new shapes, mutually exclusive via `allOf` if/then/else (preserving the existing
  wiki-shape exclusivity checks); schema regenerated twice, no further diff.
- [x] Tests added matching existing style: wrap target resolves; wrap target
  unresolved (missing id, unadmitted primarytype, no primarytype) stays unresolved; wrap
  never overrides an already-resolved (engine- or wiki-) item; corpse without take
  routes `corpse_decoration`; corpse with take in the table resolves; take-able unlisted
  name stays unresolved; a real `flags.corpse` item is unchanged by the new rule.
- [x] Both censuses and the promotion packet regenerated; `--check --self-check` passes
  on both censuses; `lower_promotion_packet.py --check` passes; README promotion counts
  updated.
- [x] Formal doc `docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md` §5a/§5b
  numbers updated, new §5c documents both rules with the exact name lists above.
- [x] Prior task record `OTV2-20260928-item-clothing-accessories` moved to
  `tasks/archive/` with status `completed` (PR #1073 merged).
- [x] Required checks pass on the frozen head (PR #1086, squash-merged as `a1af019c`).

## Excluded scope

- No `quest_item` entries are added to `DEAD_CREATURE_PROFILE`: no take-able "dead ..."
  name in either pinned engine is a named/proper-noun quest character.
- Does not touch the Delivery Task rule, delivery-list membership, or any
  already-resolved item's `family_profile`.

## Implementation / findings

| | Crystal `ff7ede5` | Canary `47dfd51` |
|---|---|---|
| `family_profile_unresolved` (before) | 480 | 439 |
| `family_profile_unresolved` (after) | 369 | 328 |
| `routed_non_item` (before) | 25,656 | 25,193 |
| `routed_non_item` (after) | 25,670 | 25,207 |
| `family_profile_basis: engine_wrap_target` | 85 | 85 |
| `family_profile_basis: owner_name_rule` | 12 | 12 |
| `routed_non_item` owner/reason `WorldObject:corpse_decoration` | 14 | 14 |
| fully resolved (before) | 11,119 | 10,782 |
| fully resolved (after) | 11,216 | 10,879 |

All other outcome/blocker counts (`blocked` 902/877, `identity_not_in_b1_catalog` 236,
Delivery Task eligible 433/401) are unchanged; no delivery-list member is among the 111
newly-resolved items in either engine. Promotion packet: 12,021 -> 12,118 Items,
14,643 -> 14,742 rows (`charges.count` 122 -> 124; every other field unchanged).

## Validation

### Focused

- command/run: `python test_engine_items.py`
- result: PASS (423 checks, including the 7 new wrap-target/corpse tests and the
  extended evidence-schema exclusivity test)

### Component/integration

- command/run: `python build_formal_schema.py` (x2, no diff on the second run);
  `python verify_formal_schema.py`; `python test_lower_promotion_packet.py`
- result: PASS (idempotent; 242 checks; 55 checks)

### E2E

- scenario: `population_census.py --engine {crystal,canary} --check --self-check`;
  `lower_promotion_packet.py --check --self-check`
- result: PASS on both engines and the packet

### Exact-head CI

- final head: `NOT_APPLICABLE` (local worktree task, no PR opened in this interaction)
- trigger source: `NOT_APPLICABLE`
- workflow/run/job: `NOT_APPLICABLE`
- runner assignment: `NOT_APPLICABLE`
- classification: `NOT_APPLICABLE`
- result: `NOT_APPLICABLE`

## Self-review

- exact head: `fcd133f08cc26787301a1b07ea7d41bddc1b58cf`
- method/reviewer: implementing session
- material findings: none
- verdict: PASS

## Independent review

- required: NO; owner-decided family mapping, same class as the prior clothing/blessing
  tasks it follows
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: done
- unresolved review threads: none
- related/superseded PRs: none
- protected auto-merge: SQUASH via Merge Queue, enabled on PR #1086
- merge commit/result: squash-merged via Merge Queue as `a1af019c` (PR #1086)
- ownership release: released

## Context checkpoint

```yaml
last_progress: PR #1086 squash-merged as a1af019c; archived
status: completed
branch: item-wrap-corpse
head_sha: fcd133f08cc26787301a1b07ea7d41bddc1b58cf
pr: 1086
final_head_sha: fcd133f08cc26787301a1b07ea7d41bddc1b58cf
final_head_frozen_at: 2026-09-28T10:02:21Z
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
next_action: none
```
