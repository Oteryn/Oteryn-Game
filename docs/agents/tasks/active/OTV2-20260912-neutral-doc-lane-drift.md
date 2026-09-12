# OTV2-20260912-neutral-doc-lane-drift

```yaml
task_id: OTV2-20260912-neutral-doc-lane-drift
title: Stabilize neutral-document CI routing across unrelated source drift
mode: REPAIR
status: corrective_repair
repository: Oteryn/Oteryn-Game
base_branch: main
branch: ci/neutral-doc-drift-proof-corrective-580
pr: 582
base_sha: 06c06a69d8d5affaf07049bad07f45b83df89392
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: chatgpt-session
created_at: 2026-09-12T08:59:35Z
updated_at: 2026-09-12T09:38:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/repository/classify_pr_test_lanes.py
  - tools/repository/test_classify_pr_test_lanes.py
  - tools/repository/test_classify_post_merge_lanes.py
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

Neutral Markdown pull requests remain on always-required checks without Linux/Windows Rust runtime lanes after proven harmless source changes, while uncertain or real new document/file consumers still fail closed to FULL.

## Architecture and source of truth

- `PROVEN` — original incident: PR #579 changed exactly two Markdown paths but run `34684158384` classified them `unreviewed-document-consumer-inputs` and selected both Rust and Windows lanes.
- `PROVEN` — PR #581 integrated as protected `main@06c06a69d8d5affaf07049bad07f45b83df89392`.
- `PROVEN` — independent exact-head Codex review `5186012563` of #581 found blocking P1 `3995788327`: grouped/aliased Rust filesystem use such as `use std::{fs}; fs::read(path)` can evade the finite whole-blob marker scan.
- `PROVEN` — the same review found P2 `3995788330`: rescanning a full changed blob rediscovers unchanged, previously audited consumers such as an existing `include_str!`, unnecessarily recreating repin pressure.
- `DERIVED` — the corrective implementation must reason about baseline-to-current consumer-relevant drift, not merely current-blob marker presence, while treating unrecognized or uncertain source drift as FULL.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this task changes trusted CI selection only. It performs no production mutation, session/lease/generation authority action, controller installation, persisted recovery interpretation, credential mutation or protected-environment write.

## Acceptance criteria

- [x] Reproduce the original docs-only incident from exact run/PR evidence.
- [x] Keep PR #579 unchanged as an optimization workaround.
- [x] Record the #581 protected-main integration and exact independent-review findings.
- [x] Repair P1 so grouped/aliased filesystem APIs and other uncertain source drift fail closed to FULL.
- [x] Repair P2 so unchanged pre-existing audited consumer behavior does not itself invalidate harmless unrelated source drift.
- [x] Preserve build/Cargo, deletion/type-change, special-mode and unavailable/malformed Git evidence as FULL.
- [x] Preserve existing server-only consumer snapshot behavior.
- [x] Add focused and real-Git PR/post-merge regression coverage for P1 and P2.
- [x] Keep BUILD_TEST_MATRIX claims truthful to the final algorithm.
- [ ] Pass fresh exact-head repository-native FULL CI.
- [ ] Obtain one fresh independent deep review on the final exact head with no blocking findings.
- [ ] Integrate only through normal Merge Queue and verify protected-main readback.

## Excluded scope

No product/runtime behavior, Cargo/lock change, workflow fan-in change, required-check removal, ruleset/protection change, direct merge, bypass, force push, production mutation or external-repository write. Do not reopen or mutate merged PR #581 as the delivery vehicle.

## Implementation / findings

The #581 implementation is now protected-main evidence, not the final accepted solution. Its finite whole-current-blob marker scan is insufficiently fail-closed for alias/grouped filesystem syntax and over-conservative for unchanged audited consumers in otherwise harmless edits.

The corrective branch starts from exact protected `main@06c06a69d8d5affaf07049bad07f45b83df89392`. The proof compares the fixed audited baseline with the current tree and admits only modified Rust whose changed lines are blank or ordinary non-doc `//` comments. Scalar constants/functions, executable syntax, attributes, `///`/`//!` doc comments and uncertain syntax fail closed. Added, deleted, type-changed, build/Cargo and non-Rust workspace inputs also fail closed. This lexical proof does not rely on finite marker absence, while unchanged baseline consumer lines are ignored.

## Validation

### Focused

- command/run: `python tools/repository/test_classify_pr_test_lanes.py`
- result: PASS including grouped/aliased filesystem negatives and unchanged-baseline-consumer positive coverage

### Component/integration

- command/run: `python tools/repository/test_classify_post_merge_lanes.py`; `python tools/repository/test_validate_pr_gate_pg_sim.py`; `python tools/repository/validate_repository_policy.py`; `python tools/agents/validate_governance.py`
- result: PR/post-merge, repository-policy and governance checks PASS; PR-gate simulation reaches its existing PowerShell canary and cannot complete because `pwsh` is unavailable in this environment

### E2E

- scenario: `NOT_APPLICABLE` — no product/runtime behavior changes; hosted classifier execution and real-Git classifier fixtures are the relevant integration evidence
- result: pending

### Exact-head CI

- final head: pending
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: repository-native GitHub Actions
- classification: FULL expected because this corrective PR changes `tools/repository/`
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing ChatGPT session
- material findings: pending final whole-diff inspection
- verdict: pending

## Independent review

- required: yes — trusted CI control-plane change
- exact head: pending
- method/auditor: native GitHub Codex deep review on stable final SHA
- material findings: pending
- verdict: pending

## PR and closeout

- original merged PR: #581 / protected merge `06c06a69d8d5affaf07049bad07f45b83df89392`
- original blocking review: `5186012563`; P1 `3995788327`; P2 `3995788330`
- corrective PR: #582
- unresolved corrective review threads: pending
- protected integration: normal Merge Queue only
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: #581 reached protected main before its independent review findings were repaired; P1/P2 are now a protected-main corrective repair under Issue #580
status: corrective_repair
branch: ci/neutral-doc-drift-proof-corrective-580
head_sha: null
pr: 582
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pending
ci_check_generation: pending
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
next_action: implement and validate P1/P2 corrective proof on a new PR from protected main
```
