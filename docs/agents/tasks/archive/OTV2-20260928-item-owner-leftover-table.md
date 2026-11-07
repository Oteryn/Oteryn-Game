# OTV2-20260928-item-owner-leftover-table

```yaml
task_id: OTV2-20260928-item-owner-leftover-table
title: Owner leftover-family decision table and empty-client-object route
mode: MIGRATE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/compassionate-albattani-s29syw
issue: 162
pr: 1109
base_sha: 6dc7e9c4
head_sha: 39985136d5ac73e0d04cbe4512a815bee5139cd4
final_head_sha: 39985136d5ac73e0d04cbe4512a815bee5139cd4
final_head_frozen_at: 2026-09-28T13:00:00Z
owner: owner-launched Claude Code session
created_at: 2026-09-28T12:00:00Z
updated_at: 2026-09-28T14:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/item-authoring/engine_items.py
  - tools/content-schema/item-authoring/test_engine_items.py
  - tools/content-schema/item-authoring/build_formal_schema.py
  - tools/content-schema/item-authoring/item.schema.json
  - tools/content-schema/item-authoring/README.md
  - tools/content-schema/item-authoring/owner-item-family-decisions.json
  - tools/content-schema/item-authoring/samples/population-crystal-ff7ede5.json
  - tools/content-schema/item-authoring/samples/population-canary-47dfd51f.json
  - tools/content-schema/item-authoring/samples/promotion-crystal-ff7ede5.json
  - docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md
  - docs/agents/tasks/archive/OTV2-20260928-item-nonitem-routing.md
  - docs/agents/tasks/archive/OTV2-20260928-item-owner-leftover-table.md
public_contracts: []
depends_on:
  - "docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md"
  - "docs/agents/tasks/archive/OTV2-20260928-item-nonitem-routing.md"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
jira: KAN-16
```

## Outcome

**Merged as PR #1109 (`a2591387`) from final head
`39985136d5ac73e0d04cbe4512a815bee5139cd4`.**

After PR #1105 (archived separately), Crystal had exactly 144 / Canary 103
`family_profile_unresolved` ids. The owner manually reviewed every one of the 144
Crystal ids (a per-item wiki lookup, client-flag inspection, and a proposed profile or
an explicit `UNSURE`; Canary shares the same id space so its own unresolved ids are
covered by the same review) and produced a per-item family decision (task #15). Two
new lowest-priority rules apply that review, each only once every existing classifier
has already failed, so no already-resolved/routed item ever changes:

1. **Owner leftover-family decision table** (`family_profile_basis: "owner_name_rule"`,
   evidence `{rule: "owner_leftover_review_2026_09_28", name}`). The review's 123
   non-`UNSURE` ids, filtered to the 121 still unresolved on this branch (2,
   "tic-tac-toe token", had already resolved via the actualname join before this
   review), are committed as `owner-item-family-decisions.json` (schema
   `OTERYN_ITEM_OWNER_FAMILY_DECISIONS/v1`, keyed by registry key:
   `name`/`profile`/`reason`/`source`), loaded by a strict, fail-closed loader
   (`load_owner_family_decisions`, modelled on `load_delivery_overrides`) and applied
   as the LAST classifier -- below even the appearance_title/actualname wiki-evidence
   tier -- gated by `skip_post_wiki_fallback_routes` like every other post-wiki rule.
   Resolves 121 Crystal / 80 Canary items.
2. **Empty client object last resort** (owner `WorldObject`, reason
   `appearance_placeholder_slot` -- same reason the placeholder-name checks use). An
   item WITH an `appearances.dat` object but a completely empty `flags` dict (not even
   `take`/`usable`) carries nothing to anchor a family to. Consulted only once the
   owner table above has also failed. Resolves 13 items identically in both engines:
   energy barrier (25799), skull stone (10134-10139), tentugly (39003), towel (20889),
   wilds monsters outfit (19125-19128).

**Remaining gap** (10 ids, identical in both engines, the owner's own `UNSURE`
verdict): `aligned opticording sphere` (19392), `arena certificate` (23547),
`blackened hand mirror` (36876), `cask` (34078), `eye-shaped frame` (36707), `frost
cannon` (9132), `remains of a crude dream` (20129, 20131), `the ashes of a device`
(21212), `unknow item` (32267) -- no wiki page, or a matched page with no
distinguishing fact, plus non-diagnostic client flags. Fail-closed editorial backlog;
recovering any needs new evidence or a further owner decision, not a rule change.

Full rule text/evidence: `docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md`
§5i-§5k.

## Architecture and source of truth

- PROVEN: all 121 committed owner-table ids' actual items.xml name (both engines)
  matches the table's own `name` guard exactly, case-insensitively; every id maps to a
  real registry key via `build_identity_index()`; no duplicate registry keys.
- PROVEN: the 13 empty-client-object ids (both engines) decode to `appearances.dat`
  `flags: {}` exactly -- `decode_flags`'s own "presence == key in dict" contract means
  this is a proven absence of every flag, not a decode gap.
- PROVEN: the review's raw 161 ids include 17 already resolved by the time of this
  review (2 "tic-tac-toe token" ids, non-`UNSURE`; 15 "stone" ids, `UNSURE`) --
  confirmed by diffing the review's id list against this branch's actual unresolved
  set (144 Crystal) before writing the table; both are correctly excluded.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline authoring tooling only.

## Acceptance criteria

- [x] `owner-item-family-decisions.json` added, schema
  `OTERYN_ITEM_OWNER_FAMILY_DECISIONS/v1`, keyed by registry key; only non-`UNSURE`,
  still-unresolved ids included (121 of 123 raw non-`UNSURE` ids).
- [x] `load_owner_family_decisions` fails closed: unique keys, closed key sets, known
  registry key, known profile, non-empty reason/facts, non-lower-cased name, missing
  file; tests added for every case.
- [x] Owner table applied as the LAST classifier, gated by
  `skip_post_wiki_fallback_routes`; name guard enforced; never overrides an
  already-resolved item, wrap-target or dead-item.
- [x] Empty-client-object route added, gated the same way, consulted only after the
  owner table; requires a real appearance object with zero true flags.
- [x] `rule` enum extended (`owner_leftover_review_2026_09_28`) in
  `build_formal_schema.py`; `item.schema.json` regenerated, idempotent (2 runs, no
  diff).
- [x] Tests added incl. required negatives: real committed examples resolve;
  already-resolved items never rerouted; name mismatch ignored; owner table never
  outranks wrap-target/dead-item; empty-object route requires zero flags and never
  outranks the owner table.
- [x] Both censuses + promotion packet regenerated, `--check --self-check` passes;
  formal doc and README updated.
- [x] Changed-id invariant proven against PR #1105's head (`bddc0616`), both engines:
  exactly 134 (Crystal) / 93 (Canary) ids change, every one previously unresolved; zero
  unsafe diffs.
- [x] `#1105` task record archived (status `completed`, PR #1105 noted, merge SHA
  `bddc0616`).
- [x] Merged as PR #1109.

## Excluded scope

- Does not touch any rule from PR #1086/#1105 -- only adds two new, lower-priority
  rules after all of them.
- No fix attempted for the 10-id remaining gap: the owner's own `UNSURE` verdict, not a
  bug; recovering it needs new evidence or a further owner decision.
- No `verify_formal_schema.py` negative for the new `rule` value: that pattern does not
  exist there for `take_able_dead_creature` either (242 checks unchanged).
- The workflow's `paths:` glob (`tools/content-schema/item-authoring/**`) already
  covers `owner-item-family-decisions.json`; no workflow edit needed.

## Implementation / findings

| | Crystal `ff7ede5` | Canary `47dfd51` |
|---|---|---|
| `family_profile_unresolved` (before task) | 144 | 103 |
| `family_profile_unresolved` (final) | 10 | 10 |
| `routed_non_item` (before task) | 25,833 | 25,370 |
| `routed_non_item` (final) | 25,846 | 25,383 |
| `fully_resolved` (before task) | 11,278 | 10,941 |
| `fully_resolved` (final) | 11,399 | 11,021 |
| owner table resolved | 121 | 80 |
| empty-client-object routed | 13 | 13 |
| remaining gap | 10 | 10 |

Owner table by profile -- Crystal: `quest_item` 37, `decoration` 28, `document` 15,
`tool` 9, `trash` 5, `transformation_item` 5, `light_source` 4, `material_valuable` 4,
`plant` 3, `progression_material` 3, `event_collectible` 3, `fluid` 3, `food` 2; Canary:
`quest_item` 33, `decoration` 18, `document` 3, `tool` 6, `trash` 5,
`transformation_item` 2, `light_source` 4, `plant` 3, `progression_material` 3,
`event_collectible` 3 (`material_valuable`/`fluid`/`food`: 0 -- 41 Crystal-only ids
have no Canary counterpart).

Every other outcome/blocker count is byte-identical before/after -- confirmed by
diffing every item id's full outcome against PR #1105's head (`bddc0616`), both
engines: exactly 134 (Crystal, 121+13) / 93 (Canary, 80+13) ids change, every one
previously `family_profile_unresolved`, zero unsafe diffs.

## Validation

### Focused

- command/run: `python test_engine_items.py`
- result: PASS (591 checks)

### Component/integration

- command/run: `build_formal_schema.py` (x2, no diff); `verify_formal_schema.py`;
  `test_lower_promotion_packet.py`; `ruff check`/`format --check` on
  `tools/content-schema/item-authoring` + the capture tool (from repo root);
  `validate_governance.py`
- result: reported exactly as run, not filtered. `ruff check` exits 1 -- 2
  pre-existing E402 findings (`verify_formal_schema.py`,
  `item_wiki_family_capture.py`'s `sys.path.insert`-then-import pattern), confirmed
  byte-identical on the `bddc0616` base before this task, neither an owned path,
  neither touched here. `ruff format --check` exits 0 after formatting this task's own
  new code. `build_formal_schema` idempotent (2 runs, no diff). `verify_formal_schema`
  242/242. `test_engine_items` 591/591. `test_lower_promotion_packet` 55/55.
  `validate_governance` passes.

### E2E

- scenario: changed-id invariant re-run against a fresh `bddc0616` (PR #1105 head)
  worktree, both engines; `population_census.py --check --self-check`;
  `lower_promotion_packet.py --check --self-check`
- result: PASS both engines + packet; zero unsafe diffs

### Exact-head CI

- final head/trigger/workflow/runner/classification/result: `NOT_APPLICABLE` (local
  worktree task, no PR opened in this interaction)

## Self-review

- exact head: `39985136d5ac73e0d04cbe4512a815bee5139cd4` (merged, PR #1109)
- method/reviewer: implementing session
- material findings: 17 of the review's raw 161 ids (2 "tic-tac-toe token" + 15
  "stone") had already resolved via the actualname join before this review and are
  correctly excluded; ruff-check pre-existing E402 findings reported honestly (exit 1)
- verdict: pass

## Independent review

- required: NO; owner-decided per-item family mapping, same class as prior tasks
- exact head/method/auditor/material findings/verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: completed; unresolved review threads: none blocking
- related/superseded PRs: continues `OTV2-20260928-item-nonitem-routing` (PR #1105);
  followed by `OTV2-20260928-item-donor-census-summer-update` (task B1a)
- protected auto-merge: via Merge Queue, PR #1109
- merge commit/result: merged as PR #1109, final head
  `39985136d5ac73e0d04cbe4512a815bee5139cd4`
- ownership release: released; downstream task `item-donor-census-summer-update`
  picked up on top of this merge

## Context checkpoint

```yaml
last_progress: owner table + empty-client-object route implemented and tested; censuses/promotion packet regenerated; invariant proven zero-unsafe against bddc0616; PR #1105 task record archived; merged as PR #1109; this record archived
status: completed
branch: claude/compassionate-albattani-s29syw
head_sha: 39985136d5ac73e0d04cbe4512a815bee5139cd4
pr: 1109
final_head_sha: 39985136d5ac73e0d04cbe4512a815bee5139cd4
final_head_frozen_at: 2026-09-28T13:00:00Z
ci_trigger_source: pull_request
ci_check_generation: 1
ci_checks_for_current_head: required
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: completed
terminal_ci_wait_started_at: 2026-09-28T13:00:00Z
terminal_ci_checks_for_current_generation: required
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: none
blocker: none
next_action: null
```
