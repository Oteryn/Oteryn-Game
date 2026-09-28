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
head_sha: 9bdaa7291d21ee3dbd038d5732bdd953d4b4aff1
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: content world build (Claude Code worker)
created_at: 2026-09-28T06:45:00Z
updated_at: 2026-09-28T08:20:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/cw2_b1_import.rs
  - apps/game-server/tests/content_reference_artifact.rs
  - apps/game-server/tests/content_world_cw2_b1_import.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - content/world/**
  - content/** (migrator output only, see Implementation/findings round 2)
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

- PROVEN: owner decision — the two packets' 69 shared `(item, field)` keys agree on 47
  values, differ only by `presentation.name` case on 22 (new lowercase correct); they
  do not compose.
- PROVEN: `docs/agents/tasks/archive/OTV2-20260928-cw2-item-promotion-lowering-wire.md`
  (PR #1048) wired the lowering pass independently; this task removes the older pass.
- DERIVED (found this task): switching the R7 P04 caller changes committed
  `content/world/**` and, downstream, the derived `content/items/**` tree; and
  `populate_items` (165-item wiki census) collides with the lowering pass on the same
  9 field paths for all 165 — coordinator extended owned_paths and set both
  reconciliation rules below (round 1: `content/world/**`; round 2: `content/**`
  migrator output, after CI caught the stale derived tree).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: content-import decode/apply/test/example change and regeneration
limited to running the repository's own tools; no production mutation, session fence,
controller or persisted recovery evidence touched.

## Acceptance criteria

- [x] Old pass + 69-only constants/structs/validator retired; shared decode/apply
  helpers kept.
- [x] R7 P04 caller validates its evidence bytes then delegates to the lowering pass.
- [x] `content_reference_artifact.rs`/`content_world_cw2_b1_import.rs` moved to the
  lowering pass; identity-drift assertions restructured around the now-unconditional
  R7 P04 rename; new coverage for source 3031's one packet field.
- [x] `content_world_project_repository.rs`: counts, gold-coin assertion,
  `DOCUMENTS`/`TREE_SHA256` table updated from a real regeneration.
- [x] `populate_items`/`promote_name` reconciliation per the coordinator's rule;
  materializer runs to completion, deterministic; `content/world/**` and the derived
  `content/items/**` tree both regenerated from tool output only.
- [x] Docstrings updated (text only).
- [x] `git grep` proves no Rust/test consumer of the old packet or old function.

## Excluded scope

`imports/**`, the pinned packet bytes/digests, `.github/workflows/**` (not edited even
where stale — see Implementation/findings), production content-loading wiring,
`reference_artifact.rs`/`project_fs.rs`/`world_runtime.rs`. Old evidence file kept,
unconsumed. No generic name normalization in `populate_items`.

## Implementation / findings

- Retired `protected_cw2_b1_promoted_item_family_import` and its 69-only
  constants/structs; kept the shared decode/apply helpers. R7 P04 caller now
  validates+delegates to the lowering pass (no double rename).
- **SHARED_LEASE_REQUIRED (resolved):** switching the R7 P04 caller changes committed
  `content/world/definitions/reference.json`. Coordinator extended owned_paths to
  `content/world/**` (regenerate-only).
- **`populate_items` conflict (resolved, coordinator rule):** lowering sets
  `presentation.name`/`weapon.*` before `populate_items` (165 wiki items) runs its
  own, stricter `promote()`. 157/165 names agreed case-insensitively (kept); 384/384
  `weapon.*` matched exactly (still strict). 8 names disagreed by more than case:
  pinned, exhaustively-hit `ITEM_NAME_LOWERING_OVERRIDES` table consulted only after
  the case check fails; every other disagreement still hard-errors. Field partition:
  `526/32/3` → `12/546/3` (promoted/equal/post_cut).
- **Tracked data-quality finding (not a blocker):** 2 of those 8 look like
  crosswalk/identity drift — `weapon.*` matches, names don't: `i00037538` wiki
  `"Staff"` vs. lowering `"pair of monk fists"`; `i00037526` wiki `"Crypt Strike"` vs.
  `"falcon sai"`. Flagged for a separate source check.
- Regenerated `content/world/**` (materializer twice, byte-identical), copied the 4
  changed documents over the tracked ones. `definitions/reference.json`: 13,861,568 →
  20,839,054 bytes (69/23 → 13,292/10,674 fields/items, +12 wiki-only).
- **Repair round 1 (Codex P2, comment 4119508551, head `dbb1fe8b`):** the docstring
  edit to `lower_promotion_packet.py` changed its own self-hash
  (`build_packet`'s `compiler_sha256` hashes `Path(__file__)`). Fix: reverted to
  byte-identical with `origin/main`; kept the README change (not hashed). Self-hash
  re-verified equal to the pinned `ITEM_SEMANTIC_PROMOTION_LOWERING_V1_COMPILER_SHA256`.
- **Repair round 2 (coordinator-flagged CI failure, head `9bdaa729`):**
  `Content tree / Item+Mount equivalence` failed `ITEM_DEFINITION_ROUNDTRIP` — the
  derived tree (`content/items/**`, generated from `content/world/**` by
  `world_project_v2_to_tree.py`) was stale. Ran the migrator (no args); 85 files
  changed: `content/items/**` (78, expected) + `content/content.lock.json`
  (top-level, allowed) + 6 non-item family `index.json` files. Diffed each of those
  7: only the `legacy_source`/`legacy_blobs` git-blob-SHA pointer to the regenerated
  `reference.json` changed; zero record/count/shard changes. Coordinator approved
  committing all 85 (extended owned_paths to `content/**`, migrator output only).
- **Workflow sweep (this round), every `content/**`/`content/world/**` reader run
  locally:** `content-tree-migration.yml` both scripts PASS.
  `g4-item-crystal-bindings.yml` self-test+`--check` PASS. `g4-item-wave1-capture.yml`
  self-tests PASS, `--check` PASS (conflicts=0); live TibiaWiki fetch NOT_APPLICABLE
  (network blocked, 403; unrelated to changed files). `native-entry-room-
  qualification.yml` 2 non-ignored Rust tests PASS; Docker+Platform test
  NOT_APPLICABLE (infra unavailable; unrelated to changed files).
  `item-content-promotion.yml` self-tests/hash-pins/crosswalk PASS; live-wiki steps
  NOT_APPLICABLE (same network block). **Unresolved (not fixed, workflows out of
  scope):** this workflow's final step hardcodes the 2 retired exact Rust test names;
  confirmed `cargo test <name> -- --exact` on a gone name prints "0 passed" and
  **exits 0** — silently validates nothing instead of failing. Needs a workflow-owner
  edit. `worldproject-v2-full-cardinality-scale.yml`'s triggers don't overlap this
  PR — not run.

## Validation

### Focused

- `cargo fmt -p oteryn-game-server -- --check`: PASS (no diff, after one `cargo fmt`
  auto-format pass)
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`: PASS, zero
  errors/warnings
- `cargo test -p oteryn-game-server --test {content_reference_artifact,
  content_world_cw2_b1_import, content_world_cw2_b1_promotion_lowering,
  content_world_project_repository} --lib`: PASS twice (before and after the round-2
  content-tree commit) — lib 661/661 (2 pre-existing ignored), 9/9, 16/16, 3/3, 3/3;
  cross-checked against `grep -c '#\[test\]'` per file
- `git grep` for the retired symbols: no remaining Rust/test consumer outside one
  historical docstring line

### Component/integration

- Materializer run twice: identical `tree_sha256`, empty recursive diff, 11 files, no
  symlinks (G4 "Materialize twice"); tracked `content/world/**` diff-identical to the
  fresh materialization (G4 "Compare tracked package")
- `world_project_v2_to_tree.py` run to completion; `test_/validate_world_project_v2_to_tree.py`
  both PASS
- Full workflow sweep for every `content/**` reader — see round 2 finding for the
  unresolved `item-content-promotion.yml` issue
- Both governance validators: PASS, both rounds

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

- exact head: `9bdaa729` (round 1); round 2 head see Context checkpoint
- method/reviewer: implementing session
- material findings: SHARED_LEASE_REQUIRED, the `populate_items` conflict, and the
  stale-derived-tree CI failure — all surfaced to and resolved/repaired with the
  coordinator; see Implementation/findings
- verdict: PASS

## Independent review

- required: YES; changes the single Item semantic-promotion source feeding a
  population-scale committed content package.
- exact head reviewed: `dbb1fe8b` (Codex); round 2 not yet independently reviewed
- method/auditor: Codex review, PR #1064 comment 4119508551
- material findings: P0/P1 none. P2 (accepted, repaired round 1): see
  Implementation/findings. Round-2 CI failure (`ITEM_DEFINITION_ROUNDTRIP`, stale
  derived tree) was a coordinator-caught CI finding, repaired the same way.
- verdict: pending re-review of the round-2 head

## PR and closeout

- changed-file review: complete locally (matches owned_paths, both rounds)
- unresolved review threads: 1 (Codex P2, repaired round 1, awaiting re-review); CI
  failure repaired round 2, awaiting green re-run
- related/superseded PRs: supersedes the old 69-field pass wired historically
- protected auto-merge: pending (Merge Queue)
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: repair round 2 pushed — regenerated content/items/** (+6 family index provenance pointers +content.lock.json) via world_project_v2_to_tree.py after coordinator-caught ITEM_DEFINITION_ROUNDTRIP CI failure; full workflow sweep + Rust retest green
status: validating
branch: claude/cw2-item-promotion-consolidation
head_sha: 9bdaa7291d21ee3dbd038d5732bdd953d4b4aff1
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
repair_cycles_for_current_gate: 2
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: item-content-promotion.yml has 2 stale exact Rust test names (silently 0-runs); needs a workflow-owner edit
blocker: null
next_action: control plane to watch PR #1064 exact-head CI (incl. content-tree-migration.yml) and re-review; freeze final_head_sha once green
```
