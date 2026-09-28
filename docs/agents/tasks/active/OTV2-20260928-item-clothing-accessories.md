# OTV2-20260928-item-clothing-accessories

```yaml
task_id: OTV2-20260928-item-clothing-accessories
title: Old rag/ivory comb via wiki status=event + exact-id join (match_basis) for the wiki fallback
mode: MIGRATE
status: in_review
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/compassionate-albattani-s29syw
issue: 162
pr: 1073
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

Two owner decisions landed on this branch, in sequence:

1. (2026-09-28, first) Reverses the prior implementation of this task (commit
   `4add578a`, PR #162): `clothing accessories` is a TibiaWiki *secondary* type, never
   the item's real family, so `PRIMARYTYPE_PROFILE` never admits it as a `primarytype`
   alias. TibiaWiki's own infobox `status = event` field is instead admitted (lowest
   priority, only when neither `primarytype` nor `objectclass` resolves) as
   `event_collectible`, because old rag (24415) is TibiaWiki's own 20th-anniversary
   event-only drop (`status = event`), not a creature product. Ivory comb (32773) keeps
   resolving to `material_valuable` through its own, already-committed wiki fallback
   record (`primarytype = Creature Products`), unaffected by the alias revert.
   (Committed as `414b4f78`.)
2. (2026-09-28, continuation) A real miss the owner found (TibiaWiki's own
   `Kraken_Buoy_Lamp_(Unlit)` page existed but the engine name "kraken buoy lamp" landed
   in `disambiguation_no_candidates`) replaces name-guessing with the exact `itemid`
   evidence TibiaWiki already publishes. The wiki fallback capture tool now joins every
   unresolved (engine, item id) against every page's own `| itemid = ...` infobox field
   FIRST, and only falls back to the pre-existing name-based join when no page lists
   that id at all; id evidence is authoritative (no name fallback when it exists but
   fails or disagrees). Every record now carries an auditable `match_basis: "itemid" |
   "title"`. Also adds `{{ItemList ...}}` template parsing to the disambiguation-page
   candidate reader (Fandom's alternative to `[[links]]` for listing variants), which is
   what surfaced the Kraken Buoy Lamp pages in the first place.

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
  assertion in `test_committed_wiki_fallback_snapshot_loads_fail_closed` pass.
- [x] The censuses and the promotion packet are regenerated, and `--check` passes.
  `routed_non_item` is unchanged (25,656 / 25,193).
- [x] Exact-id join (`match_basis`): `fetch_infobox_object_titles`/`build_itemid_index`/
  `resolve_id_matched_pages` in the capture tool; `{{ItemList ...}}` template parsing in
  `extract_candidate_links`; `match_basis` required and validated fail-closed in
  `load_wiki_family_fallback`, added to `build_formal_schema.py`'s `familyProfileEvidence`
  (schema regenerated, idempotent); id evidence takes precedence over, and is never
  overridden by, name matching. Recaptured a second time with the id join live: snapshot
  949 -> 1,415 records (1,380 `itemid`, 35 `title`).
- [x] Tests: ItemList parsing, id-match precedence, id-matched disagreement/failure with
  no name fallback, loader rejecting a missing/unknown `match_basis` -- all added and
  passing.
- [ ] Required checks pass on the frozen PR head.

## Excluded scope

- `fireworks` remains unmapped, because no unresolved item currently uses it.

## Implementation / findings

| | Crystal | Canary |
|---|---|---|
| `family_profile_unresolved` (with the alias, superseded) | 945 | 904 |
| `family_profile_unresolved` (without the alias, baseline) | 946 | 905 |
| `family_profile_unresolved` (status=event rule, before the id join) | 945 | 904 |
| `family_profile_unresolved` (status=event rule + exact-id join, final) | 480 | 439 |
| fully resolved (before the id join) | 10,654 | 10,317 |
| fully resolved (final, with the id join) | 11,119 | 10,782 |
| `family_profile_basis: engine_attribute` (unchanged throughout) | 10,679 | 10,357 |
| `family_profile_basis: wiki_evidence_fallback` (before the id join) | 877 | 837 |
| `family_profile_basis: wiki_evidence_fallback` (final) | 1,342 | 1,302 |

`routed_non_item` stayed exactly 25,656 / 25,193 throughout both owner decisions, as
required. Old rag and ivory comb both fully resolve throughout; old rag via
`status = event`, ivory comb via `primarytype = Creature Products` -- both now joined by
exact `itemid`, not by name, since the id join runs first and both happen to have an
id-matched page.

Wiki fallback snapshot: 949 -> 1,415 records once the exact-id join went live. Of 3,563
unresolved (engine, id) pairs across 1,060 unique names: the id join matched 1,565
registry keys by their own numeric id (1,378 resolved direct, 2 disambiguation, 185 left
unresolved with no name fallback because their id-matched pages did not resolve or
disagreed), the remaining 108 names went through the pre-existing name join and resolved
35 more. 1,380 records are `match_basis: "itemid"`, 35 are `match_basis: "title"`. Ten
registry keys that already had a title-based record changed profile once the (higher
priority) id evidence disagreed with it; four keys that used to resolve by name
(`cm token`, `glowworms` x2, `empty bucket`) lost their record entirely because their own
id-matched pages exist but do not resolve -- both are the intended, owner-specified
behaviour of exact id evidence overriding a same-page name match. Kraken Buoy Lamp
(Crystal/Canary ids `37187` and `37519`) now resolves via `itemid` to `Kraken Buoy Lamp
(Lit)`/`Kraken Buoy Lamp (Unlit)` respectively, `primarytype = Decorations` ->
`decoration` -- confirming the owner's finding.

## Validation

- `test_engine_items.py`: PASS (387 checks, including `status`-field, real-snapshot
  old-rag/ivory-comb, ItemList-parsing, id-match-precedence, id-match-disagreement and
  `match_basis`-rejection tests).
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
- related/superseded PRs: #1063, #1068 (alias reverted here)
- protected auto-merge: SQUASH via Merge Queue, enabled on PR #1073
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: itemid join + status=event published as PR #1073
status: in_review
branch: claude/compassionate-albattani-s29syw
head_sha: null
pr: 1073
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
next_action: required checks, Merge Queue, archive record
```
