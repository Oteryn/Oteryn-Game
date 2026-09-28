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
head_sha: 2278dd3fb6193553a34ce94fb541ff03821d8dd8
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: content world build (Claude Code worker)
created_at: 2026-09-28T06:45:00Z
updated_at: 2026-09-28T09:25:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/cw2_b1_import.rs
  - apps/game-server/tests/content_reference_artifact.rs
  - apps/game-server/tests/content_world_cw2_b1_import.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - content/world/**
  - content/** (migrator output only, see Implementation/findings round 2)
  - .github/workflows/item-content-promotion.yml (Rust test step only, round 3)
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
  `content/world/**`, downstream `content/items/**`, and collides with
  `populate_items` (165-item wiki census) on the same 9 field paths — coordinator
  extended owned_paths 3x (below) as each was found.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: content-import decode/apply/test/example change and regeneration
limited to running the repository's own tools; no production mutation, session fence,
controller or persisted recovery evidence touched.

## Acceptance criteria

- [x] Old pass + 69-only constants/structs/validator retired; shared decode/apply
  helpers kept; R7 P04 caller delegates to the lowering pass.
- [x] Tests moved to the lowering pass; assertions restructured around the
  now-unconditional R7 P04 rename; new coverage for source 3031's one field.
- [x] `content_world_project_repository.rs` counts/assertions/tables updated from a
  real regeneration.
- [x] `populate_items`/`promote_name` reconciliation per the coordinator's rule;
  `content/world/**` and `content/items/**` both regenerated from tool output only.
- [x] Docstrings updated (text only); `.github` test names fixed (round 3).
- [x] `git grep` proves no Rust/test consumer of the old packet or function.

## Excluded scope

`imports/**`, the pinned packet bytes/digests, `.github/workflows/**` except
`item-content-promotion.yml`'s Rust test step (round 3, coordinator-approved — see
Implementation/findings), production content-loading wiring,
`reference_artifact.rs`/`project_fs.rs`/`world_runtime.rs`. Old evidence file kept,
unconsumed. No generic name normalization in `populate_items`. **Not re-pinning**
`docs/agents/evidence/OTV2-20260928-item-promotion-lowering-v1.json` to the
now-grown `samples/promotion-crystal-ff7ede5.json` (14,173 rows/11,555 Items after
main's #1052/#1063 wiki-family-fallback/blessing-charms Items) — that re-pin
(byte-count/digest constants, decoder if the 9 field paths change) is a separate
follow-up task, coordinator-confirmed after this PR merges.

## Implementation / findings

- Retired `protected_cw2_b1_promoted_item_family_import` + 69-only constants/structs
  (shared decode/apply helpers kept); R7 P04 caller validates+delegates to the
  lowering pass (no double rename). SHARED_LEASE_REQUIRED (content/world impact) and
  the `populate_items`/lowering field-path collision were both coordinator-resolved
  before implementing: `promote_name` accepts case-insensitive `presentation.name`
  matches (157/165) plus a pinned, exhaustively-hit `ITEM_NAME_LOWERING_OVERRIDES`
  table for the 8 non-case disagreements; `weapon.*` (384/384) stayed strict; field
  partition `526/32/3` → `12/546/3`. **Data-quality finding (not a blocker, flagged
  for a separate check):** 2 of those 8 look like crosswalk/identity drift, not
  naming style — `i00037538` wiki `"Staff"` vs. lowering `"pair of monk fists"`;
  `i00037526` wiki `"Crypt Strike"` vs. `"falcon sai"` (both `weapon.*`-exact).
  `content/world/**` regenerated (materializer twice, byte-identical); 4 documents
  changed; `definitions/reference.json` 13,861,568→20,839,054 bytes (69/23→
  13,292/10,674 fields/items, +12 wiki-only).
- **Repair round 1** (Codex P2, comment 4119508551, head `dbb1fe8b`): a docstring
  edit had changed `lower_promotion_packet.py`'s own self-hash; reverted to
  byte-identical with `origin/main`, kept the unhashed README change.
- **Repair round 2** (coordinator-flagged CI, head `9bdaa729`):
  `ITEM_DEFINITION_ROUNDTRIP` — the derived tree was stale. Ran
  `world_project_v2_to_tree.py` (no args); 85 files changed:
  `content/items/**` (78) + `content.lock.json` + 6 family `index.json` files, each
  with only its `legacy_source`/`legacy_blobs` git-blob-SHA pointer moved (zero
  record/count/shard drift, verified per-file); coordinator approved committing all
  85. Workflow sweep of every `content/**` reader: all PASS/NOT_APPLICABLE except
  `item-content-promotion.yml`'s hardcoded retired test names, fixed next.
- **Repair rounds 3/3b** (coordinator-approved, heads `bed28218`/`f95244c9`):
  `item-content-promotion.yml`'s Rust test step only — updated the 2 exact test
  names to their round-1 replacements and piped each `cargo test` through
  `grep '...' >/dev/null` (no `-q`, which can SIGPIPE `tee` mid-write) under the
  step's `pipefail` so a 0-test run fails closed; verified both new names run
  exactly 1 test and the retired name exits 1.
- **Merges with `origin/main`** (rounds 4/5, heads `e5e9e5fe`/`2278dd3f`): merge
  commits only, no rebase/force. Both times the sole conflict was
  `tools/content-schema/item-authoring/README.md` (a hot spot for the item-content
  workstream — round 4 combined both sides' additions; round 5 took main's version
  verbatim, `git checkout --theirs`, dropping this PR's docs-only README edit
  entirely — `git diff origin/main -- ...README.md` empty). Neither round touched
  `content/world/**`/`content/items/**`; round 5's `git diff 79b85ec6 origin/main
  --stat` showed nothing under `apps/game-server/**`, so per the coordinator's rule
  the Rust suite was skipped that round (content-tree scripts + materializer-twice
  G4 compare still re-ran green each time, `tree_sha256` unchanged). Did **not**
  re-pin the wired evidence file to main's grown sample (deferred, see Excluded
  scope).

## Validation

### Focused

- `cargo fmt -p oteryn-game-server -- --check`: PASS (no diff, after one `cargo fmt`
  auto-format pass)
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`: PASS, zero
  errors/warnings
- `cargo test` on the 4 content test files + `--lib`: PASS 3x (rounds 2, 4; skipped
  round 5, no `apps/game-server` change) — lib 661/661, 9/9, 16/16, 3/3, 3/3
- `git grep` for the retired symbols: no remaining Rust/test consumer outside one
  historical docstring line
- Both new `item-content-promotion.yml` names run 1 test each, pass; retired name
  fails the guard (rounds 3/3b, verified)

### Component/integration

- Materializer twice: identical `tree_sha256` (`a496f3519e...`), empty diff, 11
  files, no symlinks (G4 "Materialize twice"); tracked `content/world/**`
  diff-identical to fresh output (G4 "Compare tracked package") — re-confirmed after
  both round-4 and round-5 merges, unchanged
- `world_project_v2_to_tree.py` run to completion; both its test/validate scripts
  PASS (rounds 2, 4, 5)
- Full workflow sweep for every `content/**` reader (round 2), fix in round 3
- Both governance validators + `validate_repository_policy.py`: PASS, all 5 rounds;
  no actionlint/other workflow linter present in this repo

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

- exact head: current, see Context checkpoint (post-merge round 5)
- method/reviewer: implementing session
- material findings: SHARED_LEASE_REQUIRED, `populate_items` conflict,
  stale-derived-tree CI failure, stale `item-content-promotion.yml` test names, and
  2 routine `origin/main` merge conflicts (rounds 4/5) — all surfaced to/resolved
  with the coordinator; see Implementation/findings
- verdict: PASS

## Independent review

- required: YES; changes the single Item semantic-promotion source feeding a
  population-scale committed content package, plus one CI workflow step.
- exact head reviewed: `dbb1fe8b` (Codex); rounds 2-5 not yet independently reviewed
- method/auditor: Codex review, PR #1064 comment 4119508551
- material findings: P0/P1 none. P2 (accepted, repaired round 1). Rounds 2-3 were
  coordinator-caught CI findings, repaired. Rounds 4-5 are routine merges with
  `origin/main`, not findings.
- verdict: pending re-review of the round-5 head

## PR and closeout

- changed-file review: complete locally (matches owned_paths, all rounds)
- unresolved review threads: 1 (Codex P2, repaired round 1, awaiting re-review); all
  CI-caught findings repaired, both merge conflicts resolved, awaiting green re-run
- related/superseded PRs: supersedes the old 69-field pass wired historically
- protected auto-merge: pending (Merge Queue)
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: merged origin/main (round 5, main at 452c3e2c, #1068/#1069) via merge commit; same README.md hot-spot conflict, this time took main's version verbatim and dropped this PR's README edit entirely (diff against origin/main empty); apps/game-server untouched past 79b85ec6 so Rust tests skipped per coordinator's rule; content-tree scripts + materializer-twice G4 compare re-ran green, content/world unchanged; both governance validators PASS
status: validating
branch: claude/cw2-item-promotion-consolidation
head_sha: 2278dd3fb6193553a34ce94fb541ff03821d8dd8
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
repair_cycles_for_current_gate: 5
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: control plane to watch PR #1064 exact-head CI (incl. content-tree-migration.yml and item-content-promotion.yml) and re-review; freeze final_head_sha once green
```
