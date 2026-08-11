# OTV2-20260811-merge-gate-hardening

```yaml
task_id: OTV2-20260811-merge-gate-hardening
title: Harden PR merge gating and repository engineering drift controls
mode: REPAIR
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: ci/OTV2-20260811-merge-gate-hardening
pr: 162
base_sha: c88f778a3d4a8d26efeb3a2ad2f328b4efca3768
original_base_sha: f184930fac66fdf9ae0cc7f606d3502c17626a79
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT repository engineering agent
created_at: 2026-08-11T10:30:00+02:00
updated_at: 2026-08-11T16:07:00+02:00
execution_budget_minutes: 120
large_budget_reason: Repository merge-authority transition plus exact-head Linux/Windows/security validation and independent review repair.
successor_generation: 1
predecessor_blocked_head_sha: 07c38012015711857ad716d6586829d37efc6801
predecessor_repair_cycles: 3
successor_repair_cycles: 1
owner_authorized_at: 2026-08-11T16:07:00+02:00
owner_authorization: Autoryzuję successor repair package dla PR #162
owned_paths:
  - .github/workflows/merge-gate.yml
  - .github/workflows/codeql.yml
  - .github/workflows/dependency-review.yml
  - .github/workflows/rust.yml
  - .github/workflows/rust-cutover-terminal-audit.yml
  - .github/dependabot.yml
  - .github/repository-policy.json
  - .github/CODEOWNERS
  - tools/repository/validate_repository_policy.py
  - tools/repository/apply_github_settings.py
  - docs/repository/GITHUB_GOVERNANCE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/tasks/active/OTV2-20260811-merge-gate-hardening.md
public_contracts:
  - .github/repository-policy.json
  - docs/repository/GITHUB_GOVERNANCE.md
  - docs/agents/BUILD_TEST_MATRIX.md
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Replace the weak single-purpose required PR check with one stable `Merge gate / validate` context that composes governance, Dependency Review, CodeQL and path-proportional Rust Linux/Windows/policy/supply-chain checks. Repair repository-engineering drift, provide exact-head recovery, and make the anti-regression validator fail closed against every noncanonical representation of the security-sensitive workflow trigger.

## Architecture and source of truth

- `PROVEN`: PR #162 was blocked after predecessor repair budget `3/3` was exhausted by independent-review thread `PRRT_kwDOTuGrds6YLEL3`.
- `PROVEN`: the owner explicitly authorized a bounded successor repair package for PR #162 on 2026-08-11 at 16:07 +02:00.
- `PROVEN`: current `main` advanced to `c88f778a3d4a8d26efeb3a2ad2f328b4efca3768`; the eight intervening commits changed only GAME-VISION documents/archive task records and had no ownership overlap with this repair.
- `PROVEN`: the successor branch was reconciled with that exact `main` using a two-parent merge commit; current merge-base is `c88f778a3d4a8d26efeb3a2ad2f328b4efca3768`.
- `PROVEN`: current `.github/workflows/merge-gate.yml` uses the intended always-on `pull_request` trigger plus exact-head `workflow_dispatch`, with no path filter.
- `DERIVED`: accepting arbitrary equivalent YAML spellings in a stdlib-only static validator recreates parser ambiguity. The safer invariant is to require one canonical top-level workflow shape and one exact canonical security-sensitive `on:` block; all alternate representations fail closed.

## Predecessor history retained

The predecessor generation consumed three repair cycles and is not being silently continued as cycle 4:

1. added exact-head dispatch recovery, completed task recoverability and aligned canonical GitHub governance;
2. included rename-source `previous_filename` in Rust/workspace scope classification;
3. narrowed an overbroad `paths:` substring check after exact-head CI exposed a false positive on `classification_paths:`.

Final predecessor review then found P2 `PRRT_kwDOTuGrds6YLEL3`: the four-space regex could miss other valid YAML representations of `paths`/`paths-ignore`. PR #162 was converted to draft and left unmerged until owner authorization.

## Successor generation 1

Owner authorization created a new bounded repair generation rather than a fourth predecessor cycle.

Implemented successor hypothesis:

- reconcile the branch with current `main@c88f778a3d4a8d26efeb3a2ad2f328b4efca3768` without overwriting its eight non-overlapping GAME-VISION commits;
- replace indentation-dependent path-filter detection with an exact canonical `on:` block contract;
- require exactly the canonical merge-gate top-level keys (`name`, `run-name`, `on`, `permissions`, `concurrency`, `jobs`) in the expected order;
- reject duplicate/alternate root-key syntax, inline trigger mappings, alternate indentation/layout, extra root mappings, comments/additions inside the trigger contract, `paths`, `paths-ignore`, or any other trigger drift;
- retain all existing checks for exact-head recovery, rename-source classification, explicit Dependency Review refs and required sub-gates.

No third-party YAML library was introduced; the validator deliberately recognizes only the single approved textual governance representation.

## Acceptance criteria

- [x] successor authorization is recorded without erasing predecessor `3/3` evidence;
- [x] branch is reconciled with current `main` and `behind_by=0`;
- [x] current merge gate itself remains always-on for PRs to `main`;
- [x] successor validator rejects all noncanonical top-level workflow roots rather than accepting alternate `on` key spellings;
- [x] successor validator requires the exact canonical `on:` block, therefore any `paths`/`paths-ignore` or alternate trigger representation fails closed;
- [x] existing exact-head recovery, rename-source classification and aggregate sub-gate invariants remain enforced;
- [ ] final PR metadata is updated before freeze;
- [ ] exact-head full-diff self-review passes on the frozen successor head;
- [ ] transition `Agent governance / validate` passes on the exact successor head;
- [ ] `Merge gate / validate` passes on the exact successor head, including all Rust/security sub-gates selected by this PR;
- [ ] independent Codex review of the exact successor head has no open material finding;
- [ ] historical repaired review threads are resolved with final-head evidence;
- [ ] squash merge occurs only on the unchanged validated head;
- [ ] post-merge repository configuration applies and verifies `Merge gate / validate` as the live sole required status.

## Excluded scope

- no gameplay/client/server runtime behavior;
- no protocol, persistence or content semantics;
- no production deployment or secret expansion;
- no cross-repository writes;
- no weakening of exact-head, review or branch-protection requirements;
- no unrelated branch/task cleanup.

## Validation

### Focused

- successor code review target: `tools/repository/validate_repository_policy.py` canonical top-level and `on:` contract;
- executable result: pending frozen exact-head GitHub Actions.

### Component/integration

- repository configuration apply/verify: post-merge by design;
- result: pending merge; no product component integration applies.

### E2E

- `NOT_APPLICABLE` — repository governance/CI transition only; no game journey behavior changes.

### Exact-head CI

- final head: pending final metadata commit/freeze;
- trigger source: pending `pull_request/synchronize`;
- transition governance: pending;
- aggregate merge gate: pending;
- result: pending.

## Self-review

- exact head: pending freeze;
- method/reviewer: implementing/coordinating agent full-diff review;
- material findings: pending;
- verdict: pending.

## Independent review

- required: `YES` — sole protected merge-authority transition has repository-wide common-mode-error risk;
- predecessor findings: preserved above;
- exact successor head: pending freeze;
- auditor: automatic `chatgpt-codex-connector` PR review;
- verdict: pending.

## PR and closeout

- PR: #162, currently draft during successor repair;
- changed-file review: pending frozen successor head;
- unresolved review threads: historical findings intentionally remain open until final-head evidence exists;
- merge: pending exact-head CI + independent review;
- live ruleset verification: pending post-merge repository-configuration workflow;
- archive/ownership release: separate terminal closeout after merge and live verification.

## Context checkpoint

```yaml
last_progress: Owner explicitly authorized successor generation 1; branch was reconciled with current main and the final predecessor P2 was addressed with fail-closed canonical top-level and exact on-block validation.
status: validating
branch: ci/OTV2-20260811-merge-gate-hardening
head_sha: null
pr: 162
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pending-final-freeze
ci_check_generation: successor-1-pre-freeze
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
predecessor_repair_cycles: 3
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Update PR #162 metadata for successor generation 1, freeze that resulting head, then perform exact-head self-review, full CI and independent review without adding checkpoint-only commits.
```
