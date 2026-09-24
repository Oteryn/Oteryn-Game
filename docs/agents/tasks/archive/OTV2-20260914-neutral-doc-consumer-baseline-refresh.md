> Lifecycle closeout: **ARCHIVED / OWNERSHIP RELEASED**. Delivery PR #618 merged as `775a09091743af395ecb8f1e440cb9c286bc0dd2`. Any active/checkpoint language below is historical provenance only; live GitHub and protected current state supersede it.

# OTV2-20260914-neutral-doc-consumer-baseline-refresh

```yaml
task_id: OTV2-20260914-neutral-doc-consumer-baseline-refresh
title: Refresh audited neutral-document consumer baseline
mode: REPAIR
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: ci/neutral-doc-consumer-baseline-refresh-20260914
issue: 615
pr: 618
base_sha: 8dfae3b9455673feff1745b9f124b786f93fcacc
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: chatgpt-session
created_at: 2026-09-14T15:35:00+02:00
updated_at: 2026-09-14T16:38:14+02:00
execution_policy: continuous_progress
owned_paths:
  - tools/repository/classify_pr_test_lanes.py
  - docs/agents/tasks/active/OTV2-20260914-neutral-doc-consumer-baseline-refresh.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Restore the already-designed neutral-document PR optimization after a reviewed source addition made the protected document-consumer baseline stale. Preserve fail-closed behavior: future unreviewed consumer drift must still select FULL Linux and Windows lanes.

## Architecture and source of truth

- `PROVEN` — protected admission is `main@8dfae3b9455673feff1745b9f124b786f93fcacc`.
- `PROVEN` — docs-only PR #612 at `facdc834a4de5b3057568e45386ed4c9d17765e6` triggered run `34849873313`; trusted classification returned `rust=true`, `windows=true`, `surface=docs`, `reason=unreviewed-document-consumer-inputs`.
- `PROVEN` — current `AUDITED_DOC_CONSUMER_BASE_SHA` is `1d0916c7476f37589524c7181b5857bcc6c141e1`.
- `PROVEN` — comparison from that baseline to admission main contains no Cargo/toolchain/root build-input change. The selected workspace change that invalidates the bounded proof is the added `apps/game-server/src/foundation/runtime_actor_carrier.rs` plus its registration in `apps/game-server/src/foundation/mod.rs`.
- `PROVEN` — the added module was read in full. Its only standard-library import is `std::mem::size_of`; it contains no filesystem/path/document/Markdown reader, no documentation `include_*`, and no generator or workspace-boundary input.
- `DERIVED` — adopting admission main as the next manually reviewed document-consumer baseline restores the existing optimization without widening the modeled safe surface.

Historical #309/#580/#582 remain evidence. This repair does not reactivate their archived/closed lifecycle.

## Exact implementation handoff

The required material change is exactly one constant replacement in `tools/repository/classify_pr_test_lanes.py`:

```diff
-AUDITED_DOC_CONSUMER_BASE_SHA = "1d0916c7476f37589524c7181b5857bcc6c141e1"
+AUDITED_DOC_CONSUMER_BASE_SHA = "8dfae3b9455673feff1745b9f124b786f93fcacc"
```

No classifier algorithm, digest constants, workflow, ruleset, required-status or Merge Queue behavior should otherwise change.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this task changes trusted CI lane selection evidence only. It performs no runtime/production mutation, session/lease/generation authority action, persisted recovery interpretation, credential change or external-repository write.

## Acceptance criteria

- [x] Update only the reviewed document-consumer baseline identity required to adopt `main@8dfae3b9455673feff1745b9f124b786f93fcacc`.
- [x] Do not weaken added/deleted/type-changed, executable-source, malformed, unknown, special-mode or other fail-closed cases.
- [x] Do not change workflows, rulesets, required statuses, Merge Queue, Cargo, runtime or product behavior.
- [x] `python tools/repository/test_classify_pr_test_lanes.py` passes.
- [x] `python tools/repository/test_classify_post_merge_lanes.py` passes.
- [x] `python tools/repository/validate_repository_policy.py` passes.
- [x] `python tools/agents/validate_governance.py` passes.
- [ ] Exact-head hosted CI is FULL for this control-plane candidate.
- [x] Whole-diff self-review finds no material unintended change.

## Excluded scope

No automatic baseline update, heuristic absence-of-marker approval, path-family widening, workflow fan-in change, required-check removal, ruleset/protection change, Merge Queue change, runtime/product edit, Cargo/lock change or unrelated WP3 mutation.

## Implementation / findings

The existing algorithm is intentionally conservative and is retained unchanged. The repair is a manual reviewed baseline refresh, matching the repository's prior snapshot-refresh discipline while preserving future fail-closed drift detection.

An owner-authorized isolated Remote Desktop worktree was used only for the exact classifier constant change and necessary governance metadata. Linux/WSL and hosted GitHub Actions provide the validation evidence below.

A direct repository-native contents update of `tools/repository/classify_pr_test_lanes.py` was initially rejected by the OpenAI safety layer. The owner then explicitly authorized Remote Desktop solely for the exact constant change, required validation, commit/push, and necessary governance metadata; that bounded route was used without changing workflows, protections, runtime, Cargo, or product behavior.

## Validation

### Focused

- command/run: `python3 tools/repository/test_classify_pr_test_lanes.py`; `python3 tools/repository/test_classify_post_merge_lanes.py` on Ubuntu/WSL
- result: PASS

### Component/integration

- command/run: `python3 tools/repository/validate_repository_policy.py`; `python3 tools/agents/validate_governance.py`; hosted `Test canonical PG/SIM gate regressions`
- result: PASS; local PG/SIM reached only the missing-`pwsh` environment canary, which hosted run `34854611242` passed

### E2E
- scenario: `NOT_APPLICABLE` — no runtime/product behavior changes
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending after this task-only truthfulness repair
- trigger source: pull_request
- workflow/run/job: Merge gate run `34854611242` on material head `1246de8bf3561658ab04de03ee461baa18a8dd38`; final metadata-only head rerun pending
- runner assignment: GitHub Actions
- classification: FULL — `explicit-build-or-control-input`, `rust=true`, `windows=true`
- result: material head PASS including Linux, PostgreSQL E2E, Windows/SIM, supply chain, CodeQL, aggregate validate and `game-gate`; final metadata-only head pending

## Self-review

- exact head: material classifier candidate `1246de8bf3561658ab04de03ee461baa18a8dd38`; final task-only metadata head pending
- method/reviewer: implementing ChatGPT session
- material findings: no material classifier issue found; stale task evidence was discovered after hosted qualification and is corrected by this task-only metadata repair
- verdict: PASS for the material classifier diff; final metadata readback pending

## Independent review

- required: YES — trusted CI selection evidence change
- exact head: `1246de8bf3561658ab04de03ee461baa18a8dd38` (material classifier candidate)
- method/auditor: Codex deep review via `@codex review`
- material findings: P2 `4006289741` — stale task coordinates/evidence; ACCEPTED and FIXED by this task-only metadata repair. No P0/P1 findings were reported.
- verdict: material classifier KEEP; P2 fixed. No deep re-review is required for this non-risk-bearing task-metadata-only repair under the bound AI review policy.

## PR and closeout
- changed-file review: PASS - exact two-file diff reviewed; classifier delta is one baseline SHA and task delta is governance-only
- unresolved review threads: P2 `4006289741` accepted/fixed in this task-only repair; thread resolution is external PR evidence after push
- related/superseded PRs: #309/#310 and #580/#582 are historical evidence only; #612 is the triggering docs-only sample
- protected auto-merge: not authorized by this task
- merge commit/result: pending
- ownership release: active until candidate publication/handoff

## Context checkpoint

```yaml
last_progress: material classifier head `1246de8` passed complete hosted FULL CI and Codex deep review; sole P2 `4006289741` was accepted and fixed in this task-only metadata repair without changing classifier logic
status: validating
branch: ci/neutral-doc-consumer-baseline-refresh-20260914
head_sha: null
pr: 618
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: 1246de8bf3561658ab04de03ee461baa18a8dd38
ci_checks_for_current_head: 3
ci_run_ids: [34854611242, 34854611264, 34854611268]
ci_job_ids: [104010792591, 104010830059, 104010830070, 104010830091, 104010830115, 104010830118, 104010943779, 104010943835, 104010943967, 104010944001, 104014330943, 104014361390]
runner_assignment_state: material_head_full_pass_final_task_only_rerun_pending
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 3
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: publish this task-only P2 repair, verify final-head hosted gates, resolve review thread `4006289741`, and stop before integration unless fresh exact-target owner authorization is provided
```
