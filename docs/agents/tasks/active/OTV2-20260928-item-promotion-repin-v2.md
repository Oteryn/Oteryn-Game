# OTV2-20260928-item-promotion-repin-v2

```yaml
task_id: OTV2-20260928-item-promotion-repin-v2
title: Re-pin the Item semantic-promotion lowering v1 packet to the further-grown sample (v2)
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/item-promotion-repin-v2
issue: 162
pr: null
base_sha: 64720c2086ec1838c7dcd0fe4faae496557096d4
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: worker (Oteryn: content/world build, this task's sole writer)
created_at: 2026-09-28T19:00:00Z
updated_at: 2026-09-28T19:45:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/evidence/OTV2-20260928-item-promotion-lowering-v1.json
  - apps/game-server/src/content/cw2_b1_import.rs
  - docs/agents/tasks/active/OTV2-20260928-item-promotion-repin-v2.md
  - content/world/**
  - content/items/**
  - content/content.lock.json
  - content/loot/index.json
  - content/presentations/definitions/index.json
  - content/abilities/definitions/index.json
  - content/abilities/effects/index.json
  - content/abilities/formulas/index.json
  - content/behaviors/index.json
  - content/creatures/definitions/index.json
  - apps/game-server/tests/content_world_project_repository.rs
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress`. `pr:` binds to the actual PR number
once the PR tool call returns it; this record's own evidence is bound to
**the frozen final head of the PR**, never to a specific commit SHA of this
worktree, since a record cannot contain its own SHA.

## Outcome

Re-pin the Item semantic-promotion lowering v1 packet
(`docs/agents/evidence/OTV2-20260928-item-promotion-lowering-v1.json`) to
`tools/content-schema/item-authoring/samples/promotion-crystal-ff7ede5.json`
(14,927 fields / 12,301 items, up from 14,643/12,021) — the same operation as
merged PR #1084 (`da27100`) — update the pinned `cw2_b1_import.rs` constants,
regenerate `content/world/**` and the derived `content/items/**` (+family
`index.json` files, `content/content.lock.json`) via the repository's own
tooling only, and update `content_world_project_repository.rs`'s literals to
match. Lease for the regeneration paths was granted on issue #162 after an
earlier `blocked`/`SHARED_LEASE_REQUIRED` stop (see git history of this
file); this revision completes the full #1084-shaped re-pin.

## Architecture and source of truth

- #1064 (merged `13576c44`) — the lowering pass is the sole Item
  semantic-promotion source. `PROVEN`.
- New sample: 3,947,571 bytes, sha256
  `69d5c1ff24b979658fc24a310d6715c131cfca6abe759ff1fa29479da0ecffe2`;
  `counts.promoted_fields = 14927`, `counts.promoted_items = 12301`;
  `counts.per_field` moved only `charges.count` 122->126 and
  `presentation.name` 12021->12301. `protected_lineage` unchanged except
  `source_population_bundle_digest` (`78c2cd27...` -> `2f0ae301...`).
  `PROVEN` (parsed both files directly).
- No new registry identity: the decoder's own
  `validate_and_apply_item_semantic_promotion_lowering_v1` rejects any
  `native_key` absent from the embedded `CW2_B1` family; every fail-closed
  identity/catalogue-drift test passed unchanged. Independently confirmed by
  diffing the regenerated `content/items/**` shards against `main`: every
  family index `record_count` is unchanged and only existing items' semantics
  gained newly-`KNOWN` fields (e.g. `presentation.name`) — no key added or
  removed. `PROVEN`.
- Materializer determinism: `materialize_content_world_project_v2 --output-root`
  run twice into fresh directories -> identical `tree_sha256 =
  56869f0d242b9ac53f842d956728a7bce2688a71a84c17e97300759463b32669` both
  times; diffed against tracked `content/world/**` (11 documents): only the
  same 4 files #1084 touched differ (`content.lock.json`,
  `definitions/reference.json`, `manifest.json`, `project.json`). `PROVEN`.
- `world_project_v2_to_tree.py` determinism: run twice in place -> identical
  43-file diff both times (30 item shards + `content/items/index.json` +
  `content/content.lock.json` + 6 family `index.json` files + the same 4
  `content/world/**` files); every family-index diff is exactly the shared
  `legacy_source.git_blob_sha` pointer move. `PROVEN`.
- `.github/workflows/item-content-promotion.yml` needs no change: unlike
  #1084, this task renamed no test function, so its two exact test-name
  greps still match unchanged names — confirmed by
  `content_world_cw2_b1_import`/`content_world_cw2_b1_promotion_lowering`
  passing with those exact (still-`_14643_`-named) functions. `PROVEN`.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
```

Reason: no production mutation, authority/lease/generation fencing,
PREPARE/COMMIT authorization, controller install/restore, authority-bearing
session replacement, or persisted-recovery-evidence interpretation. This is a
deterministic content re-pin, tool-generated tree regeneration, and constant
update.

## Acceptance criteria

- [x] Evidence file byte-identical to the sample (verified via direct
      read/hash of both files).
- [x] `cw2_b1_import.rs` pinned bytes/sha256/field-count/item-count/
      bundle-digest and the 2 moved per-field counts updated to match.
- [x] `content/world/**` regenerated via the materializer only, proven
      deterministic; `content/items/**`+family indexes+lock regenerated via
      `world_project_v2_to_tree.py`, also proven deterministic.
- [x] `content_world_project_repository.rs` `DOCUMENTS`/`TREE_SHA256`/
      `promoted_items` updated to match freshly materialized output.
- [x] Rebased onto fresh `origin/main` before final regeneration and again
      immediately before freeze; both rebases were clean, disjoint no-ops.
- [x] `cargo fmt --all --check` / `cargo clippy --all-targets -- -D warnings`
      clean.
- [x] Full targeted test suite (`content_world_project_repository` +
      `content_reference_artifact` + `content_world_cw2_b1_import` +
      `content_world_cw2_b1_promotion_lowering` + `content_world_project_v2`
      + `--lib`) passes: 718 passed, 0 failed (2 pre-existing ignored).
- [x] Both governance validators and `git diff --check` pass.
- [x] `.github/**` untouched; confirmed unaffected (see Architecture).
- [ ] PR opened; READY_FOR_INTEGRATION reported with PR/head SHA.

## Excluded scope

- No edits under `tools/content-schema/item-authoring/**` (read-only;
  verified clean via `git status`).
- No hand-edits to any generated file — `content/world/**` and
  `content/items/**`(+family indexes/lock) are tool output only, copied or
  written in place by `materialize_content_world_project_v2` /
  `world_project_v2_to_tree.py`.
- No test renames in `apps/game-server/tests/**` beyond the granted literal
  updates in `content_world_project_repository.rs`.
- No change to the compiler path/sha256 or the other two pinned artifact
  digests (`appearances.dat`, `task_board_delivery_items.lua`) — unchanged.
- No `.github/**` or `tools/repository/**` writes — protected.

## Implementation / findings

1. Verified the sample's counts (14,927/12,301) directly; copied it
   byte-for-byte over the evidence file; updated `cw2_b1_import.rs`
   constants (see prior git-history commit on this branch for the exact
   values). No lowering/decoder logic touched.
2. Coordinator granted a shared lease on issue #162 for the regeneration
   paths after this record's earlier `blocked` stop.
3. Rebased onto fresh `origin/main` (no relevant commits landed on the owned
   paths; clean no-op rebase).
4. Built and ran `materialize_content_world_project_v2 --output-root` twice
   into fresh scratch directories: identical `tree_sha256`; diffed against
   tracked `content/world/**`, copied only the 4 files that differed.
5. Ran `world_project_v2_to_tree.py` in place (writes directly under
   `content/`, no CLI output-root option): 43 files changed (matches the
   expected non-`content/world` file set); re-ran it a second time — zero
   further diff, confirming idempotence/determinism.
6. Spot-checked item shard diffs with `git diff --word-diff`: identical
   record ordering/keys, only specific items' `semantics` gained a
   newly-`KNOWN` `presentation.name`/`charges.count` value; no records
   added or removed.
7. Ran `test_world_project_v2_to_tree.py`, `validate_world_project_v2_to_tree.py`
   and `tools/content-schema/validate_materialized_game_tree.py` — all PASS.
8. Updated `content_world_project_repository.rs`'s `DOCUMENTS` entries for
   the 4 changed locators, `TREE_SHA256`, and `promoted_items` (12,021 ->
   12,301); `promoted_fields` already followed the `FIELD_COUNT` constant.
9. Ran the full targeted test suite (below) — all green. Rebased once more
   onto a second, unrelated fresh `origin/main` move immediately before
   freeze (clean no-op).

## Validation

### Focused

- `cargo test -p oteryn-game-server --test content_world_project_repository --test content_reference_artifact --test content_world_cw2_b1_import --test content_world_cw2_b1_promotion_lowering --test content_world_project_v2 --lib`
  -> 671 + 9 + 16 + 3 + 16 lib-adjacent + 3 = 718 passed, 0 failed, 2
  pre-existing ignored.

### Component/integration

- `cargo +1.94.0 fmt --all --check` -> clean.
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings` -> clean
  (only the pre-existing unrelated vendored `tokio` doc-lint warning).
- `python3 tools/agents/validate_governance.py` -> PASS.
- `python3 tools/repository/validate_repository_policy.py` -> PASS.
- `git diff --check` -> clean.

### E2E

- Materializer determinism (2 runs, identical `tree_sha256`) + tracked
  package diff = exactly the 4 expected files: PASS.
- `world_project_v2_to_tree.py` determinism (2 runs, identical 43-file
  diff) + `test_world_project_v2_to_tree.py` + `validate_world_project_v2_to_tree.py`
  + `validate_materialized_game_tree.py`: PASS.

### Exact-head CI

- final head: pending — bound to the frozen final head of PR
  `PR_NUMBER_PENDING` once opened, not to a commit SHA in this record.
- result: pending Merge Queue / `game-gate`.

## Self-review

- exact state: local commits on `claude/item-promotion-repin-v2`, rebased
  onto fresh `origin/main` (`64720c2086ec1838c7dcd0fe4faae496557096d4`) as
  final base; about to push and open the PR.
- method/reviewer: implementing agent (this task's sole writer).
- material findings: none; every regenerated byte traces to deterministic
  tool output, spot-checked for identity preservation.
- verdict: PASS.

## Independent review

- required: YES — re-pins the single Item semantic-promotion source's
  evidence feeding the committed `content/world/**`/`content/items/**`
  package.
- exact-head CI + protected-main readback for the frozen final head: pending.

## PR and closeout

- PR to be opened against `main` with `Coordination: #162`, using the
  repository PR template.
- Evidence for merge/closeout binds to the frozen final head of that PR
  (per issue #162 FREEZE convention), not to any commit SHA recorded here.
- No `@codex`, no auto-merge.

## Context checkpoint

```yaml
last_progress: full #1084-shaped re-pin complete on owned+leased paths; all targeted tests/validators green; rebased onto fresh origin/main; pushing and opening PR next
status: implementing
branch: claude/item-promotion-repin-v2
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pending
ci_check_generation: pending
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: not_started
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: none
blocker: null
next_action: push branch, open PR, report READY_FOR_INTEGRATION
```
