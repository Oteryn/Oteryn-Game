# OTV2-20260928-cw2-item-promotion-repin

```yaml
task_id: OTV2-20260928-cw2-item-promotion-repin
title: Re-pin the #1048 Item semantic-promotion lowering v1 packet to the grown sample
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/cw2-item-promotion-repin
issue: 162
pr: 1084
base_sha: 13dadcaa30f803e8a3676390cee51a84179890f6
head_sha: ec7e212642002f0bbc2a3e8cf34b3a51a286b75b
final_head_sha: null
final_head_frozen_at: null
owner: worker E (Oteryn: content world build)
created_at: 2026-09-28T08:29:00Z
updated_at: 2026-09-28T09:30:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/cw2_b1_import.rs
  - apps/game-server/tests/content_reference_artifact.rs
  - apps/game-server/tests/content_world_cw2_b1_import.rs
  - apps/game-server/tests/content_world_cw2_b1_promotion_lowering.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - docs/agents/evidence/OTV2-20260928-item-promotion-lowering-v1.json
  - content/world/**
  - content/items/**
  - content/content.lock.json
  - content/abilities/**/index.json
  - content/behaviors/index.json
  - content/creatures/definitions/index.json
  - content/loot/index.json
  - content/presentations/definitions/index.json
  - .github/workflows/item-content-promotion.yml
public_contracts: []
depends_on:
  - OTV2-20260928-cw2-item-promotion-consolidation
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress`.

## Outcome

Re-pin the #1048 Item semantic-promotion lowering v1 packet
(`docs/agents/evidence/OTV2-20260928-item-promotion-lowering-v1.json`) to the
current `tools/content-schema/item-authoring/samples/promotion-crystal-ff7ede5.json`
sample. Update the pinned `cw2_b1_import.rs` constants, the affected test
names/counts, regenerate `content/world/**` and the derived `content/items/**`
tree, and update the CI test-name grep in `item-content-promotion.yml`, with no
change to `tools/content-schema/item-authoring/**` (read-only for this task).

## Architecture and source of truth

- #1064 (merged `13576c44`) — the #1048 lowering pass is the sole Item
  semantic-promotion source. `PROVEN`.
- `promotion-crystal-ff7ede5.json` grew twice while this task was in flight:
  first branched at `13576c44` and pinned to 14,174/11,556 fields/items; while
  authoring, `git fetch origin main` found main had moved to `57a0fc76`
  (further wiki-family-fallback / item-classification content landed after
  this branch's base), which grew the sample again to 14,643/12,021. Rebased
  cleanly onto `57a0fc76` (no conflicts; owned paths disjoint) and re-derived
  every pinned value from the sample as it stands on that head. `PROVEN`.
- Per-field-path deltas for this final pin vs. the prior wired pin (13,292/
  10,674): `charges.count` 121->122, `protection.armor` 429->432,
  `presentation.name` 10,674->12,021; `container.capacity`, `weapon.attack`,
  `weapon.defense`, `weapon.extra_defense`, `weapon.hit_chance`,
  `weapon.range_cells` unchanged. Verified by parsing the packet and diffing
  per-field-path counters against its own `counts.per_field`. `PROVEN`.
- A third `git fetch origin main` (pre-commit) found main moved once more to
  `8d320703` (`OTV2-20260928-item-target-date`, touching the same
  `item-content-promotion.yml` on disjoint lines). Rebased cleanly again; no
  authoring-sample change this time, so no further re-pin was needed — re-ran
  the full validation suite on the rebased head. `PROVEN`.
- The 3 pinned artifact digests (`items.xml` via `CW2_B1_SOURCE_SHA256`,
  `appearances.dat`, `task_board_delivery_items.lua`) and the compiler
  path/sha256 are unchanged from the currently-wired pin; only the packet
  bytes/sha256/field-count/item-count/bundle-digest and the
  `presentation.name`/`charges.count`/`protection.armor` per-field counts
  moved. `PROVEN`.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
```

Reason: no production mutation, authority/lease/generation fencing,
PREPARE/COMMIT authorization, controller install/restore, authority-bearing
session replacement, or persisted-recovery-evidence interpretation. This is a
deterministic content re-pin plus test/constant update.

## Acceptance criteria

- [x] Evidence file byte-identical to the sample on the rebased head (`cmp` clean).
- [x] `cw2_b1_import.rs` pinned bytes/sha256/field-count/item-count/bundle-digest
      and the 3 moved per-field counts updated to match.
- [x] Renamed tests (`_14643_`/`12021` forms) pass individually and in the full run.
- [x] `content_world_project_repository.rs` `DOCUMENTS`/`TREE_SHA256`/`promoted_items`
      updated to match freshly materialized output.
- [x] `content/world/**` regenerated via the materializer only, proven
      deterministic (materialize-twice, `tree_sha256` matches the pinned constant).
- [x] `content/items/**` + 6 family `index.json` + `content/content.lock.json`
      regenerated via `world_project_v2_to_tree.py`; only the expected file set
      changed, family indexes only move `legacy_source.git_blob_sha`.
- [x] `item-content-promotion.yml` exact test-name grep updated, fail-closed
      pattern preserved (no `-q`).
- [x] Full targeted cargo test suite + lib, fmt, clippy pass on the rebased head.
- [x] Content-tree scripts and materializer G4 sweep pass.
- [x] Both governance validators pass.
- [x] PR opened; READY_FOR_INTEGRATION reported with PR/head SHA (#1084, `ec7e2126`).

## Excluded scope

- No edits under `tools/content-schema/item-authoring/**` (read-only; verified
  clean via `git status` after every re-pin).
- No extension of `ITEM_NAME_LOWERING_OVERRIDES` — `populate_items` hit no new
  name disagreements and no override entry went unused on either re-pin.
- No change to the compiler path/sha256 or the other two pinned artifact
  digests — unchanged both times.

## Implementation / findings

1. First pin (base `13576c44`): copied the sample, updated all pinned
   constants/tests/`content/world/**`/`content/items/**`, validated fully.
2. `git fetch origin main` before push found main had moved to `57a0fc76`
   with 38 more files including a further growth of
   `promotion-crystal-ff7ede5.json` (unrelated content rounds landed while
   authoring). Per the branch protocol, stopped and rebased onto the moved
   head rather than pushing a stale pin; `git rebase origin/main` was clean
   (owned paths disjoint from what moved).
3. Redid the full re-pin against the sample as it stands on `57a0fc76`: new
   packet is 3,870,781 bytes, sha256
   `dda86c1a1065de9ae29b40dd5f66ee56ddc3666104878a632c8cbe013ada5f2d`,
   14,643 fields across 12,021 items, bundle digest
   `78c2cd278be2031cfe70988e790341fa15f4fddcbc0b86621daed07779d65e29`
   (all read directly from the packet's own `protected_lineage`/`counts`).
4. Rebuilt the `materialize_content_world_project_v2` example (byte/hash pins
   decode successfully) and ran it twice: identical output,
   `tree_sha256 = 2b4dca7f739935721d7dd3452c915339e0c57df2d84bc3ccca35df1e2ea28af7`,
   11 files, no symlinks, no `populate_items` STOP. Diffed against the tracked
   `content/world/**` package (markers removed): same 4 files differ
   (`content.lock.json`, `definitions/reference.json`, `manifest.json`,
   `project.json`); copied only those.
5. Ran `world_project_v2_to_tree.py`: 56 files changed this round (43 item
   shards + `content/items/index.json` + `content/content.lock.json` + the 6
   family indexes + the 4 `content/world/**` files = matches exactly the
   expected non-item file set). Spot-checked one family index diff: only
   `legacy_source.git_blob_sha` moved.
6. Renamed the atom-count tests to `_14643_`/`12021` forms in the 3 owned test
   files and updated `content_world_project_repository.rs`'s `promoted_items`;
   `wave1_items`/`wave1_fields` (164/290) and `CW2_B1_FULL_ITEM_FAMILY_COUNT`
   (38,157, the full registry size) needed no change — both proven disjoint
   from the lowering pack in the prior round and re-confirmed by the wave1
   self-tests/`--check` runs passing unchanged.
7. Updated `item-content-promotion.yml`'s two exact test-name greps to the
   `_14643_` form. Swept the other five `content/**`/`content/world/**`-reading
   workflows for hardcoded counts: none found; reproduced their local
   scripts/self-tests (`test_world_project_v2_to_tree.py`,
   `validate_world_project_v2_to_tree.py`, `validate_materialized_game_tree.py`,
   `g4_item_crystal_binding_generator`/`g4_item_wave1_capture`/
   `g4_item_wave1_stage` self-tests + `--check`/`--expect`) — all PASS.
   `native-entry-room-qualification.yml`'s test has zero literal coupling to
   the promotion constants (grepped clean) and its docker/cross-repo-Platform
   portion is infeasible here — verified instead via a clean
   `cargo check --all-targets`.

## Validation

### Focused

- command/run: `cargo test -p oteryn-game-server --test content_world_project_repository --test content_reference_artifact --test content_world_cw2_b1_import --test content_world_cw2_b1_promotion_lowering --test content_world_project_v2 --lib` (shared target + `flock`), re-run on the rebased head.
- result: 3 + 9 + 16 + 3 + 16 + 668 passed, 0 failed (668-lib run also had 2 pre-existing ignored).

### Component/integration

- command/run: `cargo fmt -p oteryn-game-server -- --check`; `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`; `cargo check -p oteryn-game-server --all-targets`
- result: all clean (only a pre-existing unrelated vendor `tokio` doc-lint warning).

### E2E

- scenario: materializer determinism + tracked-package equivalence (G4) and content-tree migration equivalence, run locally per `g4-canonical-worldproject-package-seed.yml` and `content-tree-migration.yml`, re-run on the rebased head.
- result: PASS — `tree_sha256` identical across two runs, matches pinned constant; tracked package (markers removed) diffs clean; migrator test/validate scripts PASS.

### Exact-head CI

- final head: pending (not yet frozen/pushed)
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending (pre-freeze)
- method/reviewer: implementing/coordinating agent (worker E)
- material findings: none found; the two monk-weapon identity findings
  (i00037538, i00037526) remain out of scope (tracked in the closeout record
  `OTV2-20260928-cw2-item-promotion-consolidation`).
- verdict: pending final diff re-read before freeze

## Independent review

- required: pending
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: sequenced after #1074 (closeout) per coordinator
  ordering; not a code dependency (disjoint file sets)
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: pushed ec7e2126 and opened PR #1084; reported READY_FOR_INTEGRATION
status: validating
branch: claude/cw2-item-promotion-repin
head_sha: ec7e212642002f0bbc2a3e8cf34b3a51a286b75b
pr: 1084
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
next_action: await required checks and review on PR #1084
```
