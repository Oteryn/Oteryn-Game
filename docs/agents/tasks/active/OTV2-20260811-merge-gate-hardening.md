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
base_sha: 872b629e027b46abb4b558d61ee6104389d97ea7
original_base_sha: f184930fac66fdf9ae0cc7f606d3502c17626a79
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT repository engineering agent
created_at: 2026-08-11T10:30:00+02:00
updated_at: 2026-08-13T17:07:00+02:00
execution_budget_minutes: 120
large_budget_reason: Repository merge-authority transition plus exact-head Linux/Windows/security validation and independent review repair.
successor_generation: 2
predecessor_repair_cycles: 3
successor_generation_1_repair_cycles: 3
successor_generation_2_repair_cycles: 1
successor_generation_1_owner_authorized_at: 2026-08-11T16:07:00+02:00
successor_generation_1_owner_authorization: Autoryzuję successor repair package dla PR #162
successor_generation_2_owner_authorized_at: 2026-08-13T17:07:00+02:00
successor_generation_2_owner_authorization: dokoncz zadnaie
owner_funded_ai_authorized_for_generation_2: false
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

Replace the weak single-purpose required PR check with one stable `Merge gate / validate` context that composes governance, Dependency Review, CodeQL and path-proportional Rust Linux/Windows/policy/supply-chain checks. Repair repository-engineering drift, provide exact-head recovery, and fail closed against trigger, exact-head scope-production, path-classification, or aggregate-result-wiring regressions in the sole protected merge authority.

## Architecture and source of truth

- `PROVEN`: PR #162 remains the delivery vehicle for the repository-engineering/governance package.
- `PROVEN`: predecessor generation and successor generation 1 each exhausted bounded `3/3` repair budgets without being merged.
- `PROVEN`: successor generation 1 final exact-head `97d3dd0a304446acbbfbb68b4365af4f8fd8c692` passed its complete aggregate CI, but independent review found P1 thread `PRRT_kwDOTuGrds6YRHL9`: the `validate` consumer was pinned while the `scope.rust` producer remained mutable.
- `PROVEN`: current `main` advanced to `872b629e027b46abb4b558d61ee6104389d97ea7`; compare showed no overlap between its intervening changes and the 13 PR #162 repository-engineering paths.
- `PROVEN`: the branch was reconciled non-destructively with that exact main using two-parent merge commit `055ed2b2e8a1342f21feee2368a4e75b7acafe47`.
- `PROVEN`: current main `AGENTS.md` forbids Codex/OpenAI API or other owner-funded AI quota consumption without explicit permission for that specific use. The owner's instruction to finish the task authorizes continuation of repository work, not Codex consumption.
- `DERIVED`: because `scope` decides whether all Rust gates are required, the scope producer must be protected as strongly as the final aggregate consumer.

## Retained repair history

### Predecessor generation — 3/3

1. exact-head dispatch recovery, task recoverability and canonical GitHub governance alignment;
2. rename-source `previous_filename` Rust/workspace classification;
3. correction of an overbroad static `paths:` substring assertion.

Final predecessor review then found representation-dependent path-filter validation, which was carried into successor generation 1.

### Successor generation 1 — 3/3

1. canonical always-on trigger/root contract;
2. trigger-block boundary normalization without weakening substantive trigger checks;
3. `.cargo/` Rust sensitivity plus full aggregate `validate` job SHA-256 pinning.

Generation-1 final candidate `97d3dd0a304446acbbfbb68b4365af4f8fd8c692` passed:

- transition `Agent governance / validate`;
- `Merge gate / scope`;
- governance/repository-policy validation;
- Dependency Review;
- CodeQL for Python and Actions;
- Rust policy/metadata;
- Linux build, strict Clippy, tests and synthetic harness;
- Windows release build, strict Clippy, visible smoke and synthetic harness;
- cargo-deny;
- final `Merge gate / validate`.

Independent review then proved that changing the executable scope assignment to `rust = False` could preserve the pinned consumer and skip all Rust jobs. That valid P1 blocked generation 1 after its repair budget was exhausted.

## Successor generation 2

The owner instructed the agent to finish the task on 2026-08-13. Under the current repository AI-budget policy this authorizes repository implementation and delivery work but does **not** authorize another Codex invocation.

### Cycle 1 — pin the scope producer

Implemented hypothesis:

- retained the existing canonical `.github/workflows/merge-gate.yml` scope implementation unchanged;
- computed the normalized two-space-indented `scope` job SHA-256 as `76c77c3b2b939e955aceb63441172fd1a77cb1e384cb58ac70c0cade4ab8d729`;
- `tools/repository/validate_repository_policy.py` now extracts that exact `scope` job and rejects any digest mismatch;
- the pinned producer covers exact-head PR resolution, same-repository/main-target validation, paginated changed-file discovery, rename-source classification, Rust-sensitive exact/prefix policy including `.cargo/`, the executable `rust = any(...)` classification, and all `scope` outputs;
- the existing `validate` consumer pin `c10c941048014cfc8712b0d02eee438a3dabaf6578c212e4c861d36a02d4f11a` remains unchanged;
- therefore mutating `rust = any(...)` to `rust = False`, changing the Rust path classifier, changing exact-head resolution or changing scope output wiring now fails repository-policy validation before the aggregate can become merge-ready.

No owner-funded AI service was invoked for this generation-2 repair.

## Acceptance criteria

- [x] branch reconciled with `main@872b629e027b46abb4b558d61ee6104389d97ea7` without losing either history;
- [x] predecessor and successor-generation-1 evidence retained;
- [x] successor generation 2 is recorded separately rather than silently extending generation 1;
- [x] `scope` producer implementation is cryptographically pinned;
- [x] aggregate `validate` consumer implementation remains cryptographically pinned;
- [x] `.cargo/` and rename-source Rust sensitivity remains present;
- [x] current owner-funded AI restriction is respected; no Codex invocation is treated as implicitly authorized;
- [ ] final PR metadata names the resulting frozen generation-2 head;
- [ ] exact-head full-diff self-review passes on the frozen generation-2 head;
- [ ] transition `Agent governance / validate` passes on that exact head;
- [ ] aggregate `Merge gate / validate` and every applicable sub-gate pass on that exact head;
- [ ] mandatory independent review is satisfied by a genuinely independent mechanism that does not consume owner-funded AI without specific permission;
- [ ] all repaired historical review threads are resolved with final-head evidence;
- [ ] branch is still up to date with current `main` immediately before merge;
- [ ] squash merge uses the unchanged validated expected head SHA;
- [ ] post-merge repository configuration proves the live `Protect main` ruleset requires exactly `Merge gate / validate`;
- [ ] task record is archived and ownership is released after terminal delivery.

## Excluded scope

- no gameplay/client/server runtime behavior;
- no protocol, persistence or content semantics;
- no production deployment or secret expansion;
- no cross-repository writes;
- no weakening of exact-head, review or branch-protection requirements;
- no owner-funded Codex/OpenAI invocation without separate explicit authorization;
- no unrelated branch/task cleanup.

## Validation

### Focused

- generation-2 target: `tools/repository/validate_repository_policy.py` scope producer SHA-256 pin;
- adversarial invariant: any change to the canonical scope block, including `rust = False`, produces a digest different from `76c77c3b2b939e955aceb63441172fd1a77cb1e384cb58ac70c0cade4ab8d729` and must fail repository policy validation;
- executable exact-head result: pending final freeze.

### Component/integration

- repository configuration apply/verify: post-merge by design;
- no product component integration applies.

### E2E

- `NOT_APPLICABLE` — repository governance/CI transition only; no game journey behavior changes.

### Exact-head CI

- final generation-2 head: pending this task-record commit;
- trigger source: pending `pull_request/synchronize`;
- transition governance: pending;
- aggregate merge gate: pending;
- result: pending.

## Self-review

- implementing/coordinating agent must review the complete 13-path PR diff plus the generation-2 delta on the frozen final SHA;
- verdict: pending.

## Independent review

- required: `YES` — this changes protected merge authority and has repository-wide common-mode-error risk.
- current policy source: root `AGENTS.md` on `main@872b629e027b46abb4b558d61ee6104389d97ea7`.
- Codex status for generation 2: `NOT AUTHORIZED / NOT INVOKED`.
- preferred mechanism: a fresh separate qualified agent/session or human reviewer that did not implement or materially author the change and does not consume owner-funded AI quota.
- final exact-head independent verdict: pending.

## PR and closeout

- PR: #162, draft during generation-2 validation;
- changed-file review: pending frozen generation-2 head;
- unresolved historical review threads remain until exact-head repair evidence exists;
- merge: pending exact-head CI, independent review, review-thread hygiene and current-main proof;
- live ruleset verification: pending post-merge repository-configuration workflow;
- archive/ownership release: terminal closeout after merge and live verification.

## Context checkpoint

```yaml
last_progress: Successor generation 2 reconciled PR #162 with main@872b629e027b46abb4b558d61ee6104389d97ea7 and repaired P1 PRRT_kwDOTuGrds6YRHL9 by SHA-256 pinning the complete merge-gate scope producer while retaining the pinned aggregate consumer.
status: validating
branch: ci/OTV2-20260811-merge-gate-hardening
head_sha: null
pr: 162
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pending-final-freeze
ci_check_generation: successor-2-cycle-1-pre-freeze
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
predecessor_repair_cycles: 3
successor_generation_1_repair_cycles: 3
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Update PR #162 metadata to the resulting exact generation-2 content head without moving the branch, freeze it, run complete exact-head self-review and CI, and satisfy mandatory independent review without invoking owner-funded AI absent separate permission.
```
