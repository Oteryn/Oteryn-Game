# OTV2-20260928-cw2-item-promotion-consolidation

```yaml
task_id: OTV2-20260928-cw2-item-promotion-consolidation
title: Make the #1048 lowering pass the single Item semantic-promotion source
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/cw2-item-promotion-consolidation
issue: 162
pr: 1064
base_sha: 45b6cc73d8dbd2988ec0153cfa5ad03318367d51
head_sha: 00f26734a086b06007af2ff38f482a97ce90fd32
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: content world build (Claude Code worker)
created_at: 2026-09-28T06:45:00Z
updated_at: 2026-09-28T07:35:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/cw2_b1_import.rs
  - apps/game-server/tests/content_reference_artifact.rs
  - apps/game-server/tests/content_world_cw2_b1_import.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - content/world/**
  - tools/content-schema/item-authoring/README.md
  - tools/content-schema/item-authoring/lower_promotion_packet.py
  - docs/agents/tasks/active/OTV2-20260928-cw2-item-promotion-consolidation.md
public_contracts: []
depends_on:
  - "PR #1048 protected-merged (docs/agents/tasks/archive/OTV2-20260928-cw2-item-promotion-lowering-wire.md)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
jira: KAN-16
```

## Outcome

Retire the old 69-field/23-item Item semantic-promotion pass
(`protected_cw2_b1_promoted_item_family_import`); make the #1048 lowering pass
(`protected_cw2_b1_item_semantic_promotion_lowering_v1_import`, 13,292 fields /
10,674 Items) the single promotion source for every caller, including the R7 P04
gold-coin import and the materialized `content/world/**` package.

## Architecture and source of truth

- PROVEN: owner decision (this task's allocation) — the two packets' 69 shared
  `(item, field)` keys agree on 47 values and differ only by `presentation.name` case
  on 22, with the new lowercase value correct; the packets do not compose.
- PROVEN: `docs/agents/tasks/archive/OTV2-20260928-cw2-item-promotion-lowering-wire.md`
  (PR #1048) wired the lowering pass as an independent pass; this task removes the
  older pass it was explicitly independent of.
- DERIVED (found this task, coordinator-approved): switching the R7 P04 caller changes
  committed `content/world/**` (confirmed: 38,157 `Item` records in
  `definitions/reference.json` match `CW2_B1_FULL_ITEM_FAMILY_COUNT` exactly) and the
  materializer's `populate_items` (165-item wiki census) collides with the lowering
  pass on the same 9 field paths for every one of those 165 items — coordinator
  extended owned_paths and set the reconciliation rule implemented below.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: content-import decode/apply/test/example change and a content
package regeneration limited to running the repository's own materializer; no
production mutation, session fence, controller or persisted recovery evidence touched.

## Acceptance criteria

- [x] Old pass, its 69-only constants/structs/validator retired;
  `decode_item_semantic_promotion_value`/`apply_item_semantic_promotion`/
  `ItemSemanticPromotionRow`/`ProtectedCw2B1PromotedItemFamilyImport` kept (shared).
- [x] `protected_r7_p04_gold_coin_item_family_import` validates its
  `gold_coin_evidence_bytes` then delegates to the lowering pass (no double rename).
- [x] `content_reference_artifact.rs`/`content_world_cw2_b1_import.rs` moved to the
  lowering pass; identity-drift assertions restructured (not renumbered) around the
  now-unconditional R7 P04 rename; new coverage that source 3031 gets exactly the
  packet's one field (`presentation.name = "gold coin"`).
- [x] `content_world_project_repository.rs`: promoted-field/item counts, the gold-coin
  `is_all_unknown` assertion and the `DOCUMENTS`/`TREE_SHA256` byte-length/sha256
  table updated from a real regeneration.
- [x] `materialize_content_world_project_v2.rs`'s `populate_items`/`promote_name`
  reconciliation implemented exactly per the coordinator's rule (below); materializer
  runs to completion; two runs are byte-identical; `content/world/**` regenerated from
  that output only, world markers untouched.
- [x] Docstrings in `tools/content-schema/item-authoring/{README.md,
  lower_promotion_packet.py}` updated (text only).
- [x] `git grep` proves no Rust/test consumer of the old packet or old function.

## Excluded scope

`imports/**`, the pinned packet bytes/digests, `.github/workflows/**`, production
content-loading wiring, `reference_artifact.rs`/`project_fs.rs`/`world_runtime.rs`.
The historical 69-field packet file
(`docs/agents/evidence/OTV2-20260923-content-world-item-semantic-promotion.json`) is
kept, unconsumed, as evidence. No generic name normalization (no "The "/parenthetical
stripping) in `populate_items`.

## Implementation / findings

- Retired `protected_cw2_b1_promoted_item_family_import` and its 69-only
  constants/structs (`ITEM_SEMANTIC_PROMOTION_FIELD_COUNT`/`_ITEM_COUNT`/`_PACKET`
  include/`_SCHEMA`/`_PROFILE`/`_STATUS`/`_COMPILER_SHA256`,
  `ITEM_TARGET_CONTINUITY_*`, `ItemSemanticPromotionPacket`/`Lineage`/`Invariants`,
  `expected_item_semantic_promotion_counts`, `validate_item_semantic_promotion_packet`).
- **SHARED_LEASE_REQUIRED finding (resolved, coordinator-approved):** the R7 P04
  caller feeds `content/world/definitions/reference.json`
  (`materialize_content_world_project_v2.rs`); switching its promotion source changes
  that committed file. Coordinator confirmed this is the intended owner outcome and
  extended owned_paths to `content/world/**` (regenerate-only, no hand edits) and this
  test file.
- **Materializer conflict (resolved, coordinator-approved rule):** the lowering pass
  now sets `presentation.name`/`weapon.*` for items before `populate_items` (165-item
  wiki census) runs its own, stricter `promote()`. All 165 wiki items overlap the
  lowering packet on `presentation.name`; 157 differ only by ASCII case (accepted
  as-equal, existing lowercase kept) and 384/384 overlapping `weapon.*` values matched
  exactly (still strict — no relaxation there). 8 `presentation.name` pairs disagreed
  by more than case; added a pinned `ITEM_NAME_LOWERING_OVERRIDES` table (exact
  `(native_key, wiki value, lowering value)` triples) that `promote_name` consults only
  after the case-insensitive check fails, plus an "every table entry must be hit"
  assertion so the table cannot go stale. Every other disagreement still hard-errors.
- **Tracked data-quality finding (not a blocker for this task):** two of those 8 look
  like crosswalk/identity drift, not a naming-style difference — `weapon.*` facts
  match exactly, but the names are unrelated: `oteryn:item.registry.i00037538` wiki
  `"Staff"` vs. lowering/`items.xml` `"pair of monk fists"`; `i00037526` wiki
  `"Crypt Strike"` vs. `"falcon sai"`. Flagged to the coordinator for a separate
  source check; not resolved here (no generic normalization, no silent skip).
- `populate_items`' field partition shifted from `promoted=526 equal=32 post_cut=3` to
  `promoted=12 equal=546 post_cut=3` (total 561 unchanged) since the lowering pass now
  pre-populates almost all of the wiki census's overlapping fields.
- Regenerated `content/world/**` by running
  `cargo run --example materialize_content_world_project_v2 -- --output-root <dir>`
  twice (byte-identical, `tree_sha256=a496f3519e...`), diffed against the tracked
  package with the 10 world markers removed exactly as the G4 workflow's "Compare
  tracked package" step does, then copied only the 4 documents that changed
  (`content.lock.json`, `definitions/reference.json`, `manifest.json`, `project.json`)
  over the tracked ones. `definitions/reference.json`: 13,861,568 → 20,839,054 bytes
  (promoted Item semantics: 69 fields/23 items → 13,292 fields/10,674 items, plus the
  12 wiki-only fields `populate_items` still contributes net-new).

## Validation

### Focused

- `cargo fmt -p oteryn-game-server -- --check`: PASS (no diff, after one `cargo fmt`
  auto-format pass)
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`: PASS, zero
  errors/warnings
- `cargo test -p oteryn-game-server --test content_reference_artifact --test
  content_world_cw2_b1_import --test content_world_cw2_b1_promotion_lowering --lib`:
  PASS — lib 661/661 (2 ignored, pre-existing), `content_reference_artifact` 9/9,
  `content_world_cw2_b1_import` 16/16, `content_world_cw2_b1_promotion_lowering` 3/3;
  all four counts cross-checked against `grep -c '#\[test\]'` in each file
- `cargo test -p oteryn-game-server --test content_world_project_repository`: PASS
  3/3 (cross-checked against `grep -c '#\[test\]'`)
- `git grep -n 'ITEM_SEMANTIC_PROMOTION_FIELD_COUNT\|ITEM_SEMANTIC_PROMOTION_ITEM_COUNT\|protected_cw2_b1_promoted_item_family_import'` outside `cw2_b1_import.rs`'s one
  historical docstring line: no remaining Rust/test consumer

### Component/integration

- Materializer determinism: two full runs to distinct scratch roots, `tree_sha256`
  identical (`a496f3519e527b71fef755fef1d3ed462ee41453c11bdbb73073c85eaa81f4c6`),
  `diff --no-dereference --recursive` empty, 11 files each, no symlinks — the exact G4
  "Materialize twice" step
- Tracked-package parity: tracked `content/world/**` (10 world markers removed) now
  `diff --no-dereference --recursive`-identical to the fresh materialization — the
  exact G4 "Compare tracked package" step
- `python3 tools/agents/validate_governance.py`: PASS (22 policy docs, 9 lanes)
- `python3 tools/repository/validate_repository_policy.py`: PASS (23 files, 47
  workflows)

### E2E

- scenario: NOT_APPLICABLE; content-import/materializer change, no runtime
  server/session path.

### Exact-head CI

- final head: pending (not yet frozen)
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending (see Context checkpoint)
- method/reviewer: implementing session
- material findings: SHARED_LEASE_REQUIRED (content/world impact) and the
  `populate_items` reconciliation conflict, both surfaced to and resolved by the
  coordinator before implementation, per Implementation/findings above
- verdict: PASS

## Independent review

- required: YES; changes the single Item semantic-promotion source feeding a
  population-scale committed content package.
- exact head: pending
- method/auditor: pending (no `@codex review`/PR comment per this task's routing; left
  to the control plane's own review step)
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: complete locally (12 files; matches owned_paths exactly)
- unresolved review threads: none yet (PR just opened)
- related/superseded PRs: supersedes the old 69-field pass wired historically; none
  other known
- protected auto-merge: pending (Merge Queue)
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: PR #1064 opened at head 00f26734a086b06007af2ff38f482a97ce90fd32; full local validation green; awaiting exact-head CI and independent review
status: validating
branch: claude/cw2-item-promotion-consolidation
head_sha: 00f26734a086b06007af2ff38f482a97ce90fd32
pr: 1064
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
next_action: control plane to watch PR #1064 exact-head CI and route independent review; freeze final_head_sha once green
```
