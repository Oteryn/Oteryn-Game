# OTV2-20260928-store-catalog-authoring

```yaml
task_id: OTV2-20260928-store-catalog-authoring
title: Add the Store offer content-schema authoring package (categories, offers, product refs)
mode: MIGRATE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/compassionate-albattani-s29syw
issue: 162
pr: 1080
base_sha: 57a0fc76
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: owner-launched Claude Code session
created_at: 2026-09-28T00:00:00Z
updated_at: 2026-09-28T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/store-authoring/
  - .github/workflows/store-authoring-schema.yml
  - docs/architecture/OTERYN_STORE_CATALOG_OWNER_DECISION_2026-09-28.md
  - docs/architecture/ARCHITECTURE_ANALYSIS_GAP_REGISTER.md
  - docs/agents/tasks/active/OTV2-20260928-store-catalog-authoring.md
public_contracts: []
depends_on:
  - "docs/architecture/OTERYN_STORE_CATALOG_OWNER_DECISION_2026-09-28.md"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
jira: KAN-16
```

## Outcome

New content family "Store offer" under `tools/content-schema/store-authoring/`,
authored like `spell-authoring/` (README, `build_formal_schema.py` -> schema, converter,
validator, census, synthetic fixtures). Mirrors the Canary/Crystal GameStore catalog
model exactly; every engine field used by either engine has a schema property or an
explicit, documented omission (`store_constants.py`, `README.md`).

## Architecture and source of truth

- PROVEN by reading both pinned engines directly: Canary (`47dfd51f`,
  `data/modules/scripts/gamestore/catalog/*.lua`, 22 files, `constants.lua`, version 2.0,
  27 offer types) and Crystal (`ff7ede5`, `init.lua` + `gamestore.lua`, version 1.1, 28
  offer types) share one category/offer model but disagree on `weekly_task_expansion`'s
  numeric id (25 vs 28) and Crystal alone defines an unused `hunting_slot` (id 25).
- PROVEN: Canary always defaults a missing `coinType` to transferable
  (`gamestore.lua:82-83`); Crystal defaults to coin unless the offer description
  contains the literal marker `{transferableprice}` (`gamestore.lua:6936-6945`).
- PROVEN: `HIRELING_SKILL`/`HIRELING_OUTFIT` offers' `id` is `HIRELING_SKILLS.*`/
  `HIRELING_OUTFITS.*`, a lookup table defined outside the GameStore catalog; not
  statically resolvable by a Lua-literal reader. Modeled as `product.hireling_typed.
  typed_id = null`, documented, never guessed.
- PROVEN: one Canary offer ("Ultimate Health Keg", itemtype 25906, offer_type
  `charges`) sources its use-count from `count` instead of `charges`, unlike every
  other `charges` offer; the converter accepts either field.
- Item identity resolves through the committed Crystal id bindings
  (`imports/crystalserver/bindings/items.json`), the same table
  `item-authoring/engine_items.py::build_identity_index` uses; both engines' `itemtype`
  ids share this one Crystal id space.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this task adds offline authoring tooling, evidence and one
ownership-decision doc; it does not touch runtime, protocol, persistence or production
trust.

## Acceptance criteria

- [x] `store-offer.schema.json` (JSON Schema 2020-12, closed shapes) for categories and
      offers, sharing `key`/`revision`/`identity`/`ItemRef` with `monster-authoring`.
- [x] `convert_store.py` parses both engines' pinned catalog sources with a small strict
      Lua-literal reader (`lua_lite.py`, never an interpreter), digest-pinned
      (`EXPECTED_DIGESTS`), fail-closed on any unrecognized top-level shape.
- [x] `validate_store.py` (schema + semantic checks) and `verify_formal_schema.py`
      (positive/negative fixtures, each asserting its expected error fragment).
- [x] `store_census.py` -> both `samples/store-census-*.json`, `--check`/`--self-check`.
- [x] `.github/workflows/store-authoring-schema.yml`, modeled on
      `item-authoring-schema.yml`; census/converter-against-real-checkouts is local-only
      (documented in the workflow and README), matching the Item package.
- [x] Owner decision doc + one-line gap-register (§32) update.
- [ ] The required checks pass on a frozen PR head (not opened from this local worktree).

## Excluded scope

- No materialized `content/store/...` runtime data is added; this is authoring/evidence
  tooling only (`tools/content-schema/`).
- Coin balance, payment, purchase ledger, refunds and fraud stay with Platform (owner
  decision doc, gap register §32) — out of scope here.
- `item-authoring`'s own field-disposition ledgers are read, never modified.

## Implementation / findings

| | Canary (`47dfd51f`) | Crystal (`ff7ede5`) |
|---|---|---|
| Categories | 25 (20 leaf, 5 group) | 25 (20 leaf, 5 group) |
| Offers | 757 | 751 |
| Item-backed offers | 514 | ~491 |
| Item refs unresolved | 2 (both "Water Floor", itemtype 49216 — no Crystal binding) | 0 |
| `hireling_typed` (typed_id null, documented) | 13 | 13 |

Full offers-per-type, per-product-kind, and the cross-engine difference summary
(category/offer set diffs, offer-count-by-type deltas, coin-type-default divergence) are
in `samples/store-census-canary-47dfd51f.json` / `samples/store-census-crystal-ff7ede5.json`.

## Validation

- `build_formal_schema.py` run twice: byte-identical (`git diff --exit-code`).
- `verify_formal_schema.py`: PASS (1 positive + 14 targeted negatives).
- `validate_store.py` on the full converted Canary and Crystal catalogs: PASS.
- `test_convert_store.py`: PASS (full pipeline against a synthetic checkout, coin-type
  default divergence, state resolution).
- `store_census.py --check --self-check` (both engines): PASS, including the exact
  "Kraken Buoy Lamp" (price 60, itemtype 37187, type HOUSE, count 1) and "Great Health
  Potion" (price 18, itemtype 239, count 100) self-checks.
- `ruff check` / `ruff format --check` on the new package: PASS.
- `python3 tools/agents/validate_governance.py`: PASS.
- `tools/repository/test_classify_content_routing.py`,
  `tools/repository/test_classify_pr_test_lanes.py`: PASS (unaffected; no per-family
  lane registration exists for `tools/content-schema/<family>-authoring/`).

## Self-review

- exact head: pending (local worktree, not yet committed to a reviewable branch head)
- method/reviewer: implementing session
- material findings: none beyond what is recorded above
- verdict: pending

## Independent review

- required: pending owner/control-plane decision
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none
- protected auto-merge: SQUASH via Merge Queue, enabled on PR #1080
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: published as PR #1080; owner-decision doc scoped to catalog authorship on review
status: validating
branch: claude/compassionate-albattani-s29syw
head_sha: null
pr: 1080
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
blocker: "not pushed / no PR opened from this session, per instructions"
next_action: required checks, Merge Queue, archive record
```
