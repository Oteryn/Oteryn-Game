# OTV2-20260912-neutral-doc-lane-drift

```yaml
task_id: OTV2-20260912-neutral-doc-lane-drift
title: Stabilize neutral-document CI routing across unrelated source drift
mode: REPAIR
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: ci/neutral-doc-drift-proof-580
pr: 581
base_sha: 1d0916c7476f37589524c7181b5857bcc6c141e1
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: chatgpt-session
created_at: 2026-09-12T08:59:35Z
updated_at: 2026-09-12T08:59:35Z
execution_policy: continuous_progress
owned_paths:
  - tools/repository/classify_pr_test_lanes.py
  - tools/repository/test_classify_pr_test_lanes.py
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/tasks/active/OTV2-20260912-neutral-doc-lane-drift.md
public_contracts: []
depends_on: []
blocks:
  - issue: 580
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Neutral Markdown pull requests remain on always-required checks without Linux/Windows Rust runtime lanes after unrelated source changes, while uncertain or real new document/file consumers still fail closed to FULL.

## Architecture and source of truth

- `PROVEN` — protected admission is `main@1d0916c7476f37589524c7181b5857bcc6c141e1`.
- `PROVEN` — PR #579 changes exactly two Markdown paths but run `34684158384` classified them `unreviewed-document-consumer-inputs` and selected both Rust and Windows lanes.
- `PROVEN` — the existing classifier uses static broad consumer digests last refreshed by #377/#378; unrelated later workspace edits therefore invalidate neutral-document eligibility.
- `DERIVED` — preserving the existing server-only snapshot while giving neutral-document routing its own bounded protected-base consumer-drift proof removes unnecessary repins without weakening fail-closed selection.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this task changes trusted CI selection only. It performs no production mutation, session/lease/generation authority action, controller installation, persisted recovery interpretation, credential mutation or protected-environment write.

## Acceptance criteria

- [x] Reproduce the incident from exact run/PR evidence.
- [x] Keep PR #579 unchanged as an optimization workaround.
- [x] Add a bounded neutral-document consumer baseline at the exact protected admission SHA.
- [x] Allow stale broad digests only when the bounded consumer-drift proof succeeds.
- [x] Fail closed on unavailable baseline evidence, Cargo/build inputs, deletion/type-change and changed file/document-consuming source.
- [x] Preserve existing server-only snapshot behavior.
- [x] Add focused positive and negative regression fixtures.
- [ ] Pass exact-head repository-native CI.
- [ ] Integrate only through normal Merge Queue and verify protected-main readback.

## Excluded scope

No PR #579 mutation, product/runtime behavior, Cargo/lock change, workflow fan-in change, required-check removal, ruleset/protection change, direct merge, bypass, force push, production mutation or external-repository write.

## Implementation / findings

The repair separates two concerns that the old broad digest coupled together. Server-only Windows omission keeps the exact existing all-nonserver snapshot requirement. Neutral-document omission instead compares the trusted protected-base workspace/build inputs with the audited protected admission baseline. Ordinary changed source remains eligible only when the changed file itself contains none of the bounded file/document-consumer markers. Build/Cargo changes, deleted/type-changed inputs, malformed/unavailable Git evidence and changed source containing filesystem/include/path-discovery markers select FULL.

The classifier's legacy digest branch remains available for direct regression fixtures; repository execution supplies the new bounded proof only for an exact all-neutral candidate.

## Validation

### Focused

- command/run: `python tools/repository/test_classify_pr_test_lanes.py`
- result: pending repository-native execution; synthetic positive/negative pre-publication checks passed

### Component/integration

- command/run: `python tools/repository/test_validate_pr_gate_pg_sim.py`; `python tools/repository/validate_repository_policy.py`; `python tools/agents/validate_governance.py`
- result: pending repository-native exact-head execution

### E2E

- scenario: `NOT_APPLICABLE` — no product/runtime behavior changes; hosted classifier execution is the relevant integration evidence
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending external readback after final content commit
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: repository-native GitHub Actions
- classification: FULL expected because this PR changes `tools/repository/`
- result: pending

## Self-review

- exact head: pending external readback after final content commit
- method/reviewer: implementing ChatGPT session
- material findings: current whole-diff review found no unrelated changed paths; final exact-head review pending
- verdict: pending

## Independent review

- required: pending current risk-policy resolution for trusted CI selection
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending final exact-head diff
- unresolved review threads: pending
- related/superseded PRs: historical #377/#378; active #579 is not modified
- protected auto-merge: normal Merge Queue only
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: bounded consumer-drift classifier and focused regressions published on PR #581; governance task record added before final freeze
status: validating
branch: ci/neutral-doc-drift-proof-580
head_sha: null
pr: 581
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: pending_final_head
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: pending
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: run and inspect repository-native exact-head validation
```
