# OTV2-20260914-neutral-doc-consumer-baseline-refresh

```yaml
task_id: OTV2-20260914-neutral-doc-consumer-baseline-refresh
title: Refresh audited neutral-document consumer baseline
mode: REPAIR
status: blocked
repository: Oteryn/Oteryn-Game
base_branch: main
branch: ci/neutral-doc-consumer-baseline-refresh-20260914
pr: null
base_sha: 8dfae3b9455673feff1745b9f124b786f93fcacc
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: null
created_at: 2026-09-14T15:35:00+02:00
updated_at: 2026-09-14T15:35:00+02:00
execution_policy: continuous_progress
owned_paths: []
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

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this task changes trusted CI lane selection evidence only. It performs no runtime/production mutation, session/lease/generation authority action, persisted recovery interpretation, credential change or external-repository write.

## Acceptance criteria

- [ ] Update only the reviewed document-consumer baseline identity required to adopt `main@8dfae3b9455673feff1745b9f124b786f93fcacc`.
- [ ] Do not weaken added/deleted/type-changed, executable-source, malformed, unknown, special-mode or other fail-closed cases.
- [ ] Do not change workflows, rulesets, required statuses, Merge Queue, Cargo, runtime or product behavior.
- [ ] Repository classifier regressions, post-merge classifier regressions, repository policy and governance pass.
- [ ] Exact-head hosted CI is FULL for this control-plane candidate.
- [ ] Whole-diff self-review finds no material unintended change.

## Excluded scope

No automatic baseline update, heuristic absence-of-marker approval, path-family widening, workflow fan-in change, required-check removal, ruleset/protection change, Merge Queue change, runtime/product edit, Cargo/lock change or unrelated WP3 mutation.

## Implementation / findings

The existing algorithm is intentionally conservative and is retained unchanged. The required implementation is exactly this one production-line replacement in `tools/repository/classify_pr_test_lanes.py`:

```diff
-AUDITED_DOC_CONSUMER_BASE_SHA = "1d0916c7476f37589524c7181b5857bcc6c141e1"
+AUDITED_DOC_CONSUMER_BASE_SHA = "8dfae3b9455673feff1745b9f124b786f93fcacc"
```

No digest constant or classifier algorithm change is required: `document_consumers_safe()` will compare later candidates against this newly reviewed protected baseline, while any later added/deleted/type-changed workspace/build input again fails closed to FULL.

The local sandbox cannot resolve `github.com`, so no local checkout/test claim is made. Repository-native reads established the exact source delta. The GitHub connector allowed creation of the dedicated branch and this task record, but its safety layer explicitly blocked the attempted write to `tools/repository/classify_pr_test_lanes.py`. No lower-level Git-data workaround was attempted because that would bypass the safety decision.

## Validation

### Focused

- command/run: blocked before candidate creation
- result: `NOT_RUN`

### Component/integration

- command/run: blocked before candidate creation
- result: `NOT_RUN`

### E2E

- scenario: `NOT_APPLICABLE` — no runtime/product behavior changes
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: FULL required for the eventual `tools/repository/**` control-plane candidate
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing ChatGPT session
- material findings: no algorithm change required; exact one-line repair identified
- verdict: implementation blocked before candidate exists

## Independent review

- required: YES — trusted CI selection evidence change
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: task record only; no implementation candidate exists
- unresolved review threads: `NOT_APPLICABLE`
- related/superseded PRs: #309/#310 and #580/#582 are historical evidence only; #612 is the triggering docs-only sample
- protected auto-merge: not authorized
- merge commit/result: pending
- ownership release: released because mutation route is blocked

## Context checkpoint

```yaml
last_progress: exact one-line repair identified after full bounded consumer re-audit; connector safety blocked the classifier write
status: blocked
branch: ci/neutral-doc-consumer-baseline-refresh-20260914
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
owner_action_required: apply the recorded one-line baseline refresh through an authorized repository editor, then run the repository-native validation and protected lifecycle
blocker: CONNECTOR_WRITE_SAFETY_BLOCKED
actionable_patch: tools/repository/classify_pr_test_lanes.py AUDITED_DOC_CONSUMER_BASE_SHA 1d0916c7476f37589524c7181b5857bcc6c141e1 -> 8dfae3b9455673feff1745b9f124b786f93fcacc
next_action: create the exact one-line implementation candidate from the recorded protected base and run FULL CI
```
