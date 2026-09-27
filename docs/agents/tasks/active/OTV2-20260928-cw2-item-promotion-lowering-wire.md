# OTV2-20260928-cw2-item-promotion-lowering-wire

```yaml
task_id: OTV2-20260928-cw2-item-promotion-lowering-wire
title: Wire the #1018 v1 Item semantic-promotion lowering candidate into cw2_b1_import
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/cw2-item-promotion-lowering-wire
issue: 162
pr: 1048
base_sha: dd209a1264e98f3d1f0f167ec3320124a071db53
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: content world import (Claude Code worker)
created_at: 2026-09-27T22:00:00Z
updated_at: 2026-09-27T23:20:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/cw2_b1_import.rs
  - apps/game-server/tests/content_world_cw2_b1_promotion_lowering.rs
  - docs/agents/evidence/OTV2-20260928-item-promotion-lowering-v1.json
  - docs/agents/tasks/active/OTV2-20260928-cw2-item-promotion-lowering-wire.md
public_contracts: []
depends_on:
  - "PR #1018 protected-merged (docs/agents/tasks/archive/OTV2-20260927-item-authoring-followups-lowering-v1.md)"
blocks: []
cross_repository_coordination_id: null
external_repositories:
  - zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a
jira: KAN-16
```

## Outcome

Wire the #1018 v1 Item semantic-promotion lowering *candidate*
(`tools/content-schema/item-authoring/lower_promotion_packet.py`,
`samples/promotion-crystal-ff7ede5.json`) into CW2-B1: a new pinned evidence packet,
its own Rust pinned constants/digest, a fail-closed decode/apply path reusing the
existing decoder's typed value shapes, and an integration test carrying the promoted
semantics through the project draft/canonicalize/link path into a compiled Reference
(typed v4) artifact. Independent promotion pass; does not compose with the existing
69-field `protected_cw2_b1_promoted_item_family_import` packet. Full narrative: PR
#1048 description and issue #162 freeze comment 5860641389.

## Architecture and source of truth

- PROVEN: `cw2_b1_import.rs` (`protected_cw2_b1_full_item_family_import`,
  `protected_cw2_b1_promoted_item_family_import`,
  `decode_item_semantic_promotion_value`, `apply_item_semantic_promotion`) is the
  accepted full-family importer and 9-field-path typed decoder/applier this task
  reuses unmodified.
- PROVEN: `docs/agents/tasks/archive/OTV2-20260927-item-authoring-followups-lowering-v1.md`
  (PR #1018, merged `d8285019`) produced the v1 candidate
  `samples/promotion-crystal-ff7ede5.json` (13,292 rows / 10,674 Items, 3,500,315
  bytes, sha256 `85130953b4e366cf60b77b2b58281a446203bc7b2f7011971ab97580f68c49aa`),
  distinct `schema`/`profile`/`status`/`next_action` literals from the wired packet.
- DERIVED: `tools/content-schema/item-authoring/**` is `read_only_producer` (open PR
  #1040 edits it); this task copies the base-commit sample bytes into its own
  `docs/agents/evidence/` file so a later tool change cannot move pinned constants.
- PROVEN: `reference_artifact.rs`'s `ReferenceArtifactProfile::for_source` (read-only
  here) auto-selects the typed v4 profile once any Item carries non-`Unknown`
  semantics; no version flag needed.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: content-import decode/apply/test change; no production mutation,
session fence, controller or persisted recovery evidence touched.

## Acceptance criteria

- [x] `docs/agents/evidence/OTV2-20260928-item-promotion-lowering-v1.json` is a
  byte-identical copy of `samples/promotion-crystal-ff7ede5.json`.
- [x] `cw2_b1_import.rs` adds its own pinned constants and a fail-closed
  `protected_cw2_b1_item_semantic_promotion_lowering_v1_import` that decodes,
  validates and applies all 13,292 rows onto a fresh
  `protected_cw2_b1_full_item_family_import` result, reusing the existing typed
  decoder/applier unmodified.
- [x] `content_world_cw2_b1_promotion_lowering.rs` (new) proves: identity/
  materializable/stack_class unchanged from base; exactly 13,292 atoms / 10,674 items
  promoted; the 3 `lower_promotion_packet.py::self_check` items carry their exact
  values (resolved by source item id); the promoted population compiles into a
  deterministic typed-v4 Reference artifact.
- [x] Changed-path checks pass on the local candidate (fmt/clippy/tests, the Python
  lowering tool's own `--check`, governance/policy validators — see Validation).
  Exact-head CI on the frozen remote candidate remains before merge.

## Excluded scope

No write to `content/**`, `imports/**`, `tools/**`, `reference_artifact.rs`,
`project_fs.rs` or `world_runtime.rs`. No edit to the existing
`content_world_cw2_b1_import.rs`. No identity reminting/regeneration of the
38,157-item map. No new parser/model/rule layer. No composition of this lowering pass
with the existing 69-field wired packet.

## Implementation / findings

- Copied `samples/promotion-crystal-ff7ede5.json` byte-for-byte into
  `docs/agents/evidence/OTV2-20260928-item-promotion-lowering-v1.json`, embedded via
  `include_bytes!`. New `ItemSemanticPromotionLoweringV1{ArtifactDigest,Lineage,
  Invariants,Packet}` Deserialize structs reuse the existing `Compiler`/`Counts`/
  `Row` structs verbatim. New validators pin schema/profile/status/next_action,
  compiler binding, per-file source digests + population bundle digest, the
  13,292/10,674 count partition, and all 7 invariant flags, fail-closed.
- **Identity finding (material, found and fixed before freeze):** the lowering
  candidate's population census already reflects the R7 P04 gold-coin rename
  (source `3031` -> `oteryn:item.currency.gold_coin`). Wiring onto a plain,
  un-renamed full-family import failed closed on exactly 1/13,292 rows (confirmed by
  a throwaway diagnostic probe, since removed). Fix: extracted
  `apply_r7_p04_gold_coin_identity_rename` out of
  `protected_r7_p04_gold_coin_item_family_import` (pure extract-method; existing R7
  P04 tests still pass unmodified) and applied it to the lowering import's own fresh
  full-family import, without inheriting that function's unrelated 69-field
  promotion (which would collide with `promote_unknown`). Detail: PR #1048
  description.
- Deliberately did not further refactor the two functions' row-application loops
  beyond the identity-rename extraction, to bound risk on already-verified
  PR-#1018-era logic.
- Test file resolves the 3 self-check items by source item id
  (`native_key_for_source_item`), not a hardcoded opaque key; its baseline compares
  against `protected_r7_p04_gold_coin_item_family_import`'s identity/materializable/
  stack_class shape (ignoring its `.semantics`).
- **Environment finding (flagged for review, not a code defect):** the mandated
  shared `CARGO_TARGET_DIR` produced a reproducible spurious `cargo clippy` `E0425`
  failure while a concurrent worker's build of a different worktree used the same
  dir; the identical invocation against an isolated target dir passed clean twice,
  and `cargo test` passed on the shared dir both before and after. Looks like
  target-dir cross-talk; reviewer may want a clean-runner reconfirmation.

## Validation

### Focused

- `cargo fmt --check -p oteryn-game-server`: PASS (no diff)
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`: PASS, zero
  errors/warnings (see environment finding above re: one spurious shared-target-dir
  run)
- `cargo test -p oteryn-game-server --test content_world_cw2_b1_promotion_lowering --test content_world_cw2_b1_import`:
  PASS 19/19 (16 existing + 3 new)

### Component/integration

- `python lower_promotion_packet.py --source <fresh zimbadev/crystalserver@ff7ede5
  clone> --check`: PASS — `{"check": "ok", ...}` (regenerates the pinned packet
  byte-identically from the live pinned source)
- `python test_lower_promotion_packet.py`: PASS 55/55
- `python tools/agents/validate_governance.py`: PASS (22 policy docs, 9 lanes)
- `python tools/repository/validate_repository_policy.py`: PASS (23 files, 45
  workflows)

### E2E

- scenario: NOT_APPLICABLE; content-import change, no runtime server/session path.

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending (recorded at freeze)
- method/reviewer: implementing session
- material findings: the source-3031 identity mismatch (see Implementation/findings)
  was found by this session's own diagnostic probe before external review, and
  repaired test-first
- verdict: PASS on the local candidate; exact remote head review remains

## Independent review

- required: YES; new decode/apply path and a new population-scale evidence file
  feeding the compiled Reference artifact.
- exact head: 7a0012d364364f3dfa9668d6949084e3044a7315 (PR #1048)
- method/auditor: Codex review found no issues on this exact head; repository CI
  (`Agent governance / validate`, `Merge gate / governance`) flagged this task
  record's bounded size and unbound `pr` field — repaired here, no code/test/
  evidence change.
- material findings: none on the implementation; the two record-hygiene findings
  above are process-only
- verdict: pending exact-head CI on the repaired record

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none known
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: repair round 1 - bound task-record size and set pr:1048; no code/test/evidence change
status: validating
branch: claude/cw2-item-promotion-lowering-wire
head_sha: null
pr: 1048
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
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: commit, push, freeze the exact remote head, re-run governance/policy validators
```
