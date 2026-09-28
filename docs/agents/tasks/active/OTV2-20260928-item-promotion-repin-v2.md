# OTV2-20260928-item-promotion-repin-v2

```yaml
task_id: OTV2-20260928-item-promotion-repin-v2
title: Re-pin the Item semantic-promotion lowering v1 packet to the further-grown sample (v2)
mode: IMPLEMENT
status: blocked
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/item-promotion-repin-v2
issue: 162
pr: null
base_sha: 75e502a8e90020afacbf37a8791e5eec54ea1b41
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: worker (Oteryn: content/world build, this task's sole writer)
created_at: 2026-09-28T19:00:00Z
updated_at: 2026-09-28T19:20:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/evidence/OTV2-20260928-item-promotion-lowering-v1.json
  - apps/game-server/src/content/cw2_b1_import.rs
  - docs/agents/tasks/active/OTV2-20260928-item-promotion-repin-v2.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress`.

## Outcome

Re-pin the Item semantic-promotion lowering v1 packet
(`docs/agents/evidence/OTV2-20260928-item-promotion-lowering-v1.json`) to the
further-grown `tools/content-schema/item-authoring/samples/promotion-crystal-ff7ede5.json`
sample (same operation as merged PR #1084 / commit `da27100`), updating the
pinned `cw2_b1_import.rs` constants to match. This task's own grant is
narrower than #1084's: only the evidence file, the Rust constants and this
record. Validation surfaced that a fully green re-pin also requires edits
outside that grant (see Blocker below), so this record stops at `blocked`
pending a shared lease rather than exceeding owned paths.

## Architecture and source of truth

- #1064 (merged `13576c44`) — the lowering pass is the sole Item
  semantic-promotion source. `PROVEN`.
- `promotion-crystal-ff7ede5.json` on this head: `counts.promoted_fields =
  14927`, `counts.promoted_items = 12301`; `counts.per_field` moved only
  `charges.count` 122->126 and `presentation.name` 12021->12301 versus the
  currently-wired pin; other per-field counts unchanged. Verified by parsing
  both the sample and the previously-pinned evidence file and diffing
  `counts.per_field`. `PROVEN`.
- `protected_lineage` is byte-identical between the sample and the prior pin
  except `source_population_bundle_digest` (`78c2cd27...` -> `2f0ae301...`).
  `schema`/`profile`/`status`/`next_action`/`target_date` unchanged. `PROVEN`.
- New sample: 3,947,571 bytes, sha256
  `69d5c1ff24b979658fc24a310d6715c131cfca6abe759ff1fa29479da0ecffe2`. `PROVEN`.
- No new registry identity: the decoder's own
  `validate_and_apply_item_semantic_promotion_lowering_v1` rejects any
  `native_key` absent from the embedded `CW2_B1` family; the targeted tests
  below reached and passed every fail-closed identity/catalogue-drift case
  with the new packet wired in. `PROVEN`.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
```

Reason: no production mutation, authority/lease/generation fencing,
PREPARE/COMMIT authorization, controller install/restore, authority-bearing
session replacement, or persisted-recovery-evidence interpretation. This is a
deterministic content re-pin plus constant update.

## Acceptance criteria

- [x] Evidence file byte-identical to the sample on this head (verified via
      direct read/hash of both files).
- [x] `cw2_b1_import.rs` pinned bytes/sha256/field-count/item-count/
      bundle-digest and the 2 moved per-field counts (`charges.count`,
      `presentation.name`) updated to match.
- [x] `cargo fmt --all --check` clean.
- [x] `cargo clippy -p oteryn-game-server --all-targets -- -D warnings` clean.
- [x] The #1084-relied-on `content_world_cw2_b1_import` (16),
      `content_world_cw2_b1_promotion_lowering` (3) and
      `content_reference_artifact` (9) tests pass unchanged against the new
      pin (they assert via the `ITEM_SEMANTIC_PROMOTION_LOWERING_V1_*`
      constants, not literals; their names still say `14643`/`12021` but
      that is cosmetic, not a failure).
- [ ] `content_world_project_repository` passes — **FAILS**, see Blocker.
- [ ] `content/world/**` / `content/items/**` regenerated to match — not
      attempted; outside this task's owned paths.
- [ ] PR opened / READY_FOR_INTEGRATION — blocked, see below.

## Blocker: SHARED_LEASE_REQUIRED

`repository_package_recaptures_and_rewrites_without_identity_or_layer_drift`
in `apps/game-server/tests/content_world_project_repository.rs` links the
real on-disk `content/world/**` project and asserts
`promoted_fields == ITEM_SEMANTIC_PROMOTION_LOWERING_V1_FIELD_COUNT + 12`.
With only the owned-path edits applied it fails: `left: 14655, right: 14939`
(14655 = old pin's 14643 + 12, i.e. the on-disk materialized tree still
reflects the *previous* pin; 14939 = new pin's 14927 + 12). This proves the
on-disk `content/world/**`/`content/items/**` trees must be regenerated from
the newly-pinned evidence for the repository to stay internally consistent —
exactly what #1084 did (materializer + `world_project_v2_to_tree.py`) — and
that regeneration, plus the dependent test-file literals, sit outside this
task's grant:

- `SHARED_LEASE_REQUIRED content/world/**` — regenerate via
  `materialize_content_world_project_v2` from the newly-pinned evidence so
  the tracked package reflects the new promotion counts.
- `SHARED_LEASE_REQUIRED content/items/**` (+ `content/content.lock.json`,
  `content/loot/index.json`, `content/presentations/definitions/index.json`,
  family `index.json` files) — derive via `world_project_v2_to_tree.py` from
  the regenerated `content/world/**`; currently reflects the prior pin.
- `SHARED_LEASE_REQUIRED apps/game-server/tests/content_world_project_repository.rs`
  — `promoted_items`/`promoted_fields` (and `DOCUMENTS`/`TREE_SHA256` if the
  regenerated tree changes tracked-document bytes) need updating to match
  the freshly materialized output; currently hardcodes the prior pin's
  `12_021` and derives a now-stale expected `promoted_fields`.

`.github/workflows/item-content-promotion.yml` is protected and was not
inspected. No test function was renamed by this task (see Excluded scope),
so its exact test-name greps are believed unaffected, but that is unverified
against the fully-regenerated state — flagged for the resolving lease-holder
to confirm.

## Excluded scope

- No edits under `tools/content-schema/item-authoring/**` (read-only;
  verified clean via `git status`).
- No test renames in `apps/game-server/tests/**` — none owned by this task;
  the targeted #1084-relied-on tests pass unchanged (constant-driven, not
  literal-driven), so no rename was needed to make them green.
- No change to the compiler path/sha256 or the other two pinned artifact
  digests (`appearances.dat`, `task_board_delivery_items.lua`) — unchanged
  between the two samples.
- No `content/**`, `.github/**` or `tools/repository/**` writes — outside
  owned paths / explicitly protected.

## Implementation / findings

1. Verified the sample's `counts` (14,927 fields / 12,301 items) by parsing
   `tools/content-schema/item-authoring/samples/promotion-crystal-ff7ede5.json`
   and comparing to the task packet's target — exact match.
2. Copied the sample byte-for-byte over
   `docs/agents/evidence/OTV2-20260928-item-promotion-lowering-v1.json`
   (3,947,571 bytes, sha256 `69d5c1ff...`).
3. Updated `cw2_b1_import.rs`: `PACKET_BYTES`, `PACKET_SHA256`,
   `FIELD_COUNT` (14_927), `ITEM_COUNT` (12_301), `BUNDLE_DIGEST`
   (`2f0ae301...`), and `expected_item_semantic_promotion_lowering_v1_counts`'s
   `charges.count` (126) and `presentation.name` (12_301); extended the
   doc-comment growth history. Did not touch decoder/validation logic.
4. Ran the targeted #1084 validation tests against the new pin:
   `content_world_cw2_b1_import` (16/16 ok), `content_world_cw2_b1_promotion_lowering`
   (3/3 ok), `content_reference_artifact` (9/9 ok) — pass unchanged including
   the fail-closed identity/catalogue-drift cases, confirming no new/unknown
   registry identity and no decoder logic gap.
5. Ran `content_world_project_repository` and hit the on-disk-tree staleness
   failure above; stopped per the owned-paths contract rather than editing
   files outside this task's grant.

## Validation

### Focused

- `cargo test -p oteryn-game-server --test content_world_cw2_b1_import --test content_world_cw2_b1_promotion_lowering`
  -> 16 + 3 passed, 0 failed.
- `cargo test -p oteryn-game-server --test content_reference_artifact --test content_world_project_repository`
  -> `content_reference_artifact` 9 passed, 0 failed;
  `content_world_project_repository` 2 passed, **1 failed**
  (`repository_package_recaptures_and_rewrites_without_identity_or_layer_drift`,
  see Blocker).

### Component/integration

- `cargo +1.94.0 fmt --all --check` -> clean, no output.
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings` -> clean
  (only the pre-existing unrelated vendored `tokio` doc-lint warning).
- `validate_governance.py` / `validate_repository_policy.py` / `git diff
  --check`: run against this record at write time; see Self-review.

### E2E / exact-head CI

- not run: gated on the materializer/content-tree regeneration in Blocker,
  outside this task's owned paths; no PR opened, no CI generation exists.

## Self-review

- exact state: uncommitted working-tree edits on branch
  `claude/item-promotion-repin-v2` (not pushed), base
  `75e502a8e90020afacbf37a8791e5eec54ea1b41`.
- method/reviewer: implementing agent (this task's sole writer).
- material findings: owned-path edits independently verified (byte
  length/sha256/counts recomputed from source files, not copied from the
  task packet); the blocker is a genuine cross-file consistency requirement,
  not a defect in the owned-path edits.
- verdict: owned-path work PASS; overall task BLOCKED pending shared lease.

## Independent review

- required: not reached; no candidate is frozen or proposed for merge.

## PR and closeout

- No PR opened. Per the task packet's own protocol, `SHARED_LEASE_REQUIRED`
  is a stop: opening a PR known to fail `content_world_project_repository`
  (and therefore `game-gate`) would risk Merge Queue admission of a broken
  candidate, so the branch was not pushed.
- Recommended resolution: either (a) expand this task's owned paths to cover
  `content/world/**`, `content/items/**`, the other `content/**/index.json`
  family files, `content/content.lock.json` and
  `apps/game-server/tests/content_world_project_repository.rs` so one writer
  can complete the full #1084-shaped re-pin, or (b) coordinate with a second
  writer already holding that lease. The evidence-file and `cw2_b1_import.rs`
  edits in this worktree carry forward unchanged into whichever task
  completes the full re-pin.

## Context checkpoint

```yaml
last_progress: owned-path edits made and independently verified; content_world_project_repository proves content/world+content/items regeneration and its own test literals are also required; stopped and reported SHARED_LEASE_REQUIRED without pushing/opening a PR
status: blocked
branch: claude/item-promotion-repin-v2
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: none
ci_check_generation: none
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
owner_action_required: grant shared lease on content/world/**, content/items/**, content/content.lock.json, content/**/index.json, apps/game-server/tests/content_world_project_repository.rs (or route to a writer holding it)
blocker: SHARED_LEASE_REQUIRED — see Blocker section above
next_action: await lease decision on issue #162; resume AUTHORING once granted
```
