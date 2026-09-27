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
pr: null
base_sha: dd209a1264e98f3d1f0f167ec3320124a071db53
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: content world import (Claude Code worker)
created_at: 2026-09-27T22:00:00Z
updated_at: 2026-09-27T22:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/cw2_b1_import.rs
  - apps/game-server/tests/content_world_cw2_b1_promotion_lowering.rs
  - docs/agents/evidence/OTV2-20260928-item-promotion-lowering-v1.json
  - docs/agents/tasks/active/OTV2-20260928-cw2-item-promotion-lowering-wire.md
public_contracts: []
depends_on:
  - "PR #1018 protected-merged (Item authoring follow-ups + v1 lowering candidate, docs/agents/tasks/archive/OTV2-20260927-item-authoring-followups-lowering-v1.md)"
blocks: []
cross_repository_coordination_id: null
external_repositories:
  - zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a
jira: KAN-16
```

## Outcome

Wire the #1018 v1 Item semantic-promotion lowering *candidate*
(`tools/content-schema/item-authoring/lower_promotion_packet.py`,
`samples/promotion-crystal-ff7ede5.json`) into the CW2-B1 Item import: a new pinned
evidence packet under this task's own owned path, its own Rust pinned
constants/digest, a fail-closed decode/apply path reusing the existing decoder's typed
value shapes, and a Rust integration test that carries the promoted semantics all the
way through the production project draft/canonicalize/link path into a compiled
Reference (typed v4) artifact. This is an independent promotion pass over the
protected full Item family; it does not compose with, replace, or require the
existing 69-field `protected_cw2_b1_promoted_item_family_import` packet.

## Architecture and source of truth

- PROVEN: `apps/game-server/src/content/cw2_b1_import.rs`
  (`protected_cw2_b1_full_item_family_import`,
  `protected_cw2_b1_promoted_item_family_import`,
  `decode_item_semantic_promotion_value`, `apply_item_semantic_promotion`,
  `ProtectedCw2B1PromotedItemFamilyImport`) is the exact accepted full-family importer
  and the existing 9-field-path typed decoder/applier this task reuses unmodified.
- PROVEN: `docs/agents/tasks/archive/OTV2-20260927-item-authoring-followups-lowering-v1.md`
  (PR #1018, merged `d8285019`) produced the v1 lowering candidate
  `tools/content-schema/item-authoring/samples/promotion-crystal-ff7ede5.json`
  (13,292 rows / 10,674 Items, 0 skipped, 3,500,315 bytes,
  sha256 `85130953b4e366cf60b77b2b58281a446203bc7b2f7011971ab97580f68c49aa`) over the
  same pinned Crystal `ff7ede5` population and the same 9 field paths the Rust decoder
  already accepts, with its own distinct `schema`/`profile`/`status`/`next_action`
  literals (`OTERYN_ITEM_SEMANTIC_PROMOTION_LOWERING/v1` family) so it can never be
  mistaken for the existing wired packet.
- DERIVED: `tools/content-schema/item-authoring/**` is `read_only_producer` here (open
  PR #1040 edits it); this task copies the exact base-commit sample bytes into its own
  `docs/agents/evidence/` file rather than depending on the live tool tree, so a later
  change to the authoring tool cannot silently move this task's pinned Rust constants.
- PROVEN: `apps/game-server/src/content/reference_artifact.rs`
  (`ReferenceArtifactProfile::for_source`, read-only here) auto-selects the typed v4
  artifact profile once any Item carries non-`Unknown` semantics, so no separate
  version flag is needed to compile the promoted population.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this is a content-import decode/apply/test change; no production
mutation, session fence, controller or persisted recovery evidence is touched.

## Acceptance criteria

- [x] `docs/agents/evidence/OTV2-20260928-item-promotion-lowering-v1.json` is a
  byte-identical copy of the base-commit
  `tools/content-schema/item-authoring/samples/promotion-crystal-ff7ede5.json`
  (same 3,500,315 bytes / sha256).
- [x] `cw2_b1_import.rs` adds its own pinned constants (packet bytes/sha256, field/item
  counts, schema/profile/status/next_action/target_date, compiler path/sha256,
  lineage bundle digest and per-file source digests) distinct from the existing
  wired packet's constants, plus a fail-closed
  `protected_cw2_b1_item_semantic_promotion_lowering_v1_import` that decodes, validates
  and applies all 13,292 rows onto a fresh `protected_cw2_b1_full_item_family_import`
  result, reusing the existing `decode_item_semantic_promotion_value`/
  `apply_item_semantic_promotion`/`ItemSemanticPromotionRow` unmodified.
- [x] `apps/game-server/tests/content_world_cw2_b1_promotion_lowering.rs` (new,
  separate from the existing `content_world_cw2_b1_import.rs`) proves: identity/
  materializable/stack_class are unchanged from the base full-family import; exactly
  13,292 atoms across 10,674 items are promoted; the three
  `lower_promotion_packet.py::self_check` items (magic sword attack/defense/name,
  a container's capacity, a charges item's count) carry their exact self-checked
  values, resolved by source item id rather than a hardcoded opaque key; and the
  promoted population round-trips through `ProjectDraft` ->
  `CanonicalProjectDocuments` -> `ProjectSnapshot` -> `WorldProject::link()` ->
  `compile_reference_playable` into a deterministic typed-v4 Reference artifact.
- [x] Changed-path checks pass on the local candidate (`cargo fmt --check`;
  `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`; both new and
  existing `cw2_b1_import` test binaries; the Python lowering tool's own `--check`
  regenerates `samples/promotion-crystal-ff7ede5.json` byte-identically;
  `validate_governance.py`; `validate_repository_policy.py`). Exact-head CI on the
  frozen remote candidate remains before merge.

## Excluded scope

No write to `content/**`, `imports/**`, `tools/**`, `reference_artifact.rs`,
`project_fs.rs` or `world_runtime.rs`. No edit to the existing
`apps/game-server/tests/content_world_cw2_b1_import.rs`. No identity reminting or
regeneration of the 38,157-item map. No new parser/model/rule layer: this task reuses
the existing typed decoder and Reference artifact compiler exactly as they stand. No
composition of this lowering pass with the existing 69-field wired packet.

## Implementation / findings

- Copied `tools/content-schema/item-authoring/samples/promotion-crystal-ff7ede5.json`
  byte-for-byte (confirmed via `sha256sum`) into
  `docs/agents/evidence/OTV2-20260928-item-promotion-lowering-v1.json`, embedded via
  `include_bytes!` from `cw2_b1_import.rs`.
- Added `ItemSemanticPromotionLoweringV1{ArtifactDigest,Lineage,Invariants,Packet}`
  Deserialize structs (`deny_unknown_fields`), reusing the existing
  `ItemSemanticPromotionCompiler`/`ItemSemanticPromotionCounts`/
  `ItemSemanticPromotionRow` structs verbatim since both packets share the exact same
  `compiler`/`counts`/row shape.
- `validate_item_semantic_promotion_lowering_v1_bytes` pins the embedded packet's
  whole-file length + `world_project_sha256`, mirroring
  `validate_r7_p04_gold_coin_evidence`'s style for a compiled-in evidence file.
  `validate_item_semantic_promotion_lowering_v1_packet` then checks schema/profile/
  status/target_date/next_action, compiler binding, the 3 pinned per-file source
  digests plus the population bundle digest, the count partition (13,292/10,674 total,
  9-field per-field breakdown from the archived task record), and all 7 invariant
  flags, fail-closed on any mismatch.
- `protected_cw2_b1_item_semantic_promotion_lowering_v1_import(evidence_bytes)` builds
  a fresh `protected_cw2_b1_full_item_family_import(evidence_bytes)` result (Default/
  Unknown semantics), derives the same `source_item_id -> native_key` closure and
  Item-record index the existing 69-field function derives, then applies all 13,292
  rows through the existing decode/apply functions with the same ordering/uniqueness/
  exact-identity/applied-partition fail-closed checks. Returns the existing
  `ProtectedCw2B1PromotedItemFamilyImport` type unchanged (no new public result type
  needed, since the shape is identical).
- **Identity finding (material, fixed before freeze):** the lowering candidate's
  population census resolves identity through this package's own committed
  `imports/crystalserver/bindings/items.json` (`engine_items.build_identity_index`),
  which already reflects the R7 P04 editorial rename of source item `3031` from its
  opaque registry key to `oteryn:item.currency.gold_coin`
  (`docs/agents/tasks/active/...` R7 P04 lineage; `protected_r7_p04_gold_coin_item_family_import`).
  A first pass wiring the packet straight onto a plain
  `protected_cw2_b1_full_item_family_import` result therefore failed closed with
  `EvidenceMismatch("... exact identity mismatch")` on exactly 1 of 13,292 rows
  (confirmed by an isolated diagnostic probe: 13,291 matches / 1 mismatch, source
  `3031`, before removal). Fix: factored the identity-rename half of
  `protected_r7_p04_gold_coin_item_family_import` out into a shared, non-`pub` helper
  `apply_r7_p04_gold_coin_identity_rename(&mut ProtectedCw2B1FullItemFamilyImport)`
  (pure extract-method; no behavior change, re-verified by the existing R7 P04 tests
  still passing unmodified), and the new lowering function now applies that same
  rename to its own fresh full-family import before matching rows — without also
  inheriting `protected_r7_p04_gold_coin_item_family_import`'s unrelated existing
  69-field semantic promotion (which would otherwise collide with `promote_unknown`
  on overlapping atoms). The existing 69-field
  `protected_cw2_b1_promoted_item_family_import` function itself is unchanged.
- Deliberately did not further refactor the two functions' row-application loops (the
  ordering/uniqueness/exact-identity/applied-partition checks) beyond the identity-
  rename extraction above: that PR-#1018-era logic is left otherwise byte-for-byte
  untouched to bound the risk surface on already-verified behavior; the new function
  duplicates that small remaining amount of loop logic instead of a deeper shared
  abstraction.
- New test file resolves the 3 self-check items by source item id via the batch's
  candidates (`native_key_for_source_item`) rather than hardcoding opaque registry
  keys, so it stays correct if the opaque allocation ever shifts for an unrelated
  reason while still failing loudly if the promoted values themselves regress. Its
  `base_family_import()` compares identity/materializable/stack_class shape against
  `protected_r7_p04_gold_coin_item_family_import`'s output (ignoring its `.semantics`,
  which also carries the unrelated 69-field promotion) rather than the plain
  full-family import, since that is the correct post-rename baseline.
- **Environment finding (risk to flag for review, not a code defect):** the mandated
  shared `CARGO_TARGET_DIR=/home/user/.cargo-shared-target` produced a reproducible
  false-negative `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`
  (10x `E0425 cannot find value ITEM_SEMANTIC_PROMOTION_LOWERING_V1_*`/`cannot find
  function protected_cw2_b1_item_semantic_promotion_lowering_v1_import`) while a
  concurrent worker's build for a *different* worktree was also using that same
  shared target dir; `cargo test` against the same shared dir at the same head
  passed cleanly both before and after, and the exact same clippy invocation against
  a dedicated, non-shared `CARGO_TARGET_DIR` passed with zero errors/warnings on the
  first try. This looks like shared-target-dir incremental-fingerprint cross-talk
  between concurrent worktree builds of the identical crate name/version, not a real
  code defect; the reviewer may want to re-run clippy on a clean/CI runner to
  reconfirm outside this specific concurrency window.

## Validation

### Focused

- `cargo fmt --check -p oteryn-game-server`: PASS (no diff)
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`: PASS, zero
  errors/warnings on `oteryn-game-server` (see the shared-target-dir finding above:
  one run against the shared target dir produced a spurious `E0425` cross-talk
  failure; a rerun against an isolated `CARGO_TARGET_DIR` at the same head passed
  clean, and a second isolated rerun after `touch`ing the changed files reproduced
  the same clean PASS)
- `cargo test -p oteryn-game-server --test content_world_cw2_b1_promotion_lowering --test content_world_cw2_b1_import`:
  PASS, 19/19 (16 existing + 3 new), reproduced identically against both the shared
  and an isolated `CARGO_TARGET_DIR`

### Component/integration

- `python lower_promotion_packet.py --source <fresh zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a clone> --check`:
  PASS — `{"check": "ok", "out": ".../samples/promotion-crystal-ff7ede5.json"}`; this
  is the real required "regenerates the pinned packet byte-identically" evidence
  (network was available in this session; confirms the committed sample, and
  therefore this task's byte-identical evidence copy, still regenerates exactly from
  the live pinned source at the pinned revision)
- `python test_lower_promotion_packet.py` (package's own unit tests, unaffected by
  this task): PASS, 55/55
- `python tools/agents/validate_governance.py`: PASS (22 policy documents, 9 project
  lanes)
- `python tools/repository/validate_repository_policy.py`: PASS (23 files, 45
  workflows; "Post-merge exact-candidate routing regressions PASS")

### E2E

- scenario: NOT_APPLICABLE; this is a content-import decode/apply/test change with no
  runtime server/session/network path.

### Exact-head CI

- final head: pending (set at freeze, after the final authoring write)
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending (recorded at freeze)
- method/reviewer: implementing session
- material findings: the source-3031 identity mismatch (see Implementation/findings)
  was found by this session's own diagnostic probe before any external review, and
  repaired test-first (the fix is exercised by both the existing R7 P04 tests, which
  still pass unmodified, and the new lowering-v1 tests)
- verdict: PASS on the local candidate; exact remote head review remains before
  freeze/PR

## Independent review

- required: YES; new decode/apply path and a new population-scale evidence file feeding
  the compiled Reference artifact.
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none known
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: fixed the source-3031 identity mismatch (shared R7 P04 rename helper extracted), all local validation PASS (fmt/clippy/tests/lowering-tool --check against a live pinned checkout/governance/policy)
status: validating
branch: claude/cw2-item-promotion-lowering-wire
head_sha: null
pr: null
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
next_action: commit, push, freeze the exact remote head and open the PR
```
