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
updated_at: 2026-08-11T16:34:00+02:00
execution_budget_minutes: 120
large_budget_reason: Repository merge-authority transition plus exact-head Linux/Windows/security validation and independent review repair.
successor_generation: 1
predecessor_blocked_head_sha: 07c38012015711857ad716d6586829d37efc6801
predecessor_repair_cycles: 3
successor_repair_cycles: 3
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

Replace the weak single-purpose required PR check with one stable `Merge gate / validate` context that composes governance, Dependency Review, CodeQL and path-proportional Rust Linux/Windows/policy/supply-chain checks. Repair repository-engineering drift, provide exact-head recovery, and fail closed against trigger, path-classification, or aggregate-result-wiring regressions in the sole protected merge authority.

## Architecture and source of truth

- `PROVEN`: PR #162 was blocked after predecessor repair budget `3/3` was exhausted by independent-review thread `PRRT_kwDOTuGrds6YLEL3`.
- `PROVEN`: the owner explicitly authorized bounded successor generation 1 for PR #162 on 2026-08-11 at 16:07 +02:00.
- `PROVEN`: the branch was reconciled with `main@c88f778a3d4a8d26efeb3a2ad2f328b4efca3768` using a true two-parent merge; the intervening GAME-VISION changes did not overlap this ownership.
- `PROVEN`: `.github/workflows/merge-gate.yml` is always-on for PRs to `main`, provides bounded exact-head dispatch recovery, and classifies rename-source paths.
- `DERIVED`: security-sensitive workflow representation is intentionally canonical rather than accepting arbitrary equivalent YAML spellings; noncanonical forms fail closed.
- `DERIVED`: because `Merge gate / validate` becomes the sole required context, the aggregate job implementation itself must be immutable under repository validation, not merely discoverable by job name or scattered fragments.

## Predecessor history retained

The predecessor generation consumed three repair cycles and is not being silently continued as cycle 4:

1. exact-head dispatch recovery, task recoverability, canonical GitHub governance alignment;
2. rename-source `previous_filename` Rust/workspace classification;
3. correction of an overbroad static `paths:` substring assertion.

The predecessor final review then found P2 `PRRT_kwDOTuGrds6YLEL3`: indentation-dependent path-filter detection could miss valid YAML representations. The PR was returned to draft until explicit owner successor authorization.

## Successor generation 1

### Cycle 1 — canonical trigger/root contract

- replaced indentation-dependent path-filter detection with an exact canonical `on:` block contract;
- required exact canonical top-level keys in order: `name`, `run-name`, `on`, `permissions`, `concurrency`, `jobs`;
- alternate/quoted/duplicate root keys, inline trigger mappings, extra root mappings, alternate trigger layout, `paths`, `paths-ignore`, or other trigger drift fail closed;
- retained exact-head recovery, rename-source classification, explicit Dependency Review refs and aggregate sub-gates.

### Cycle 2 — trigger-block boundary normalization

Pre-freeze self-review found that the extractor included the blank separator before `permissions`, while the expected canonical trigger constant did not. The repair normalizes CRLF and trailing blank separators only (`rstrip("\n") + "\n"`) while leaving substantive trigger content exact.

### Cycle 3 — aggregate implementation and Cargo repository configuration

Independent review of frozen candidate `51d3be4d08219358edf25d18f48f1fe959e3993d` found two valid issues:

1. **P1 — `PRRT_kwDOTuGrds6YQvvI`: aggregate implementation not canonicalized.** A malicious change could replace only the `validate` job's fail-closed script with `echo` while retaining names/fragments, allowing the sole required status to succeed despite failed sub-gates.
2. **P2 — `PRRT_kwDOTuGrds6YQvvP`: `.cargo/**` not Rust-sensitive.** Repository-wide Cargo target/runner/rustflags/network configuration could change while all four PR Rust jobs and the retained post-merge Rust workflow skipped.

Repairs:

- `.cargo/` is now in merge-gate Rust classification prefixes, including rename-source classification;
- `.cargo/**` is now in `.github/workflows/rust.yml` push paths;
- repository validation asserts both selectors remain present;
- the entire normalized two-space-indented `validate` job is SHA-256 pinned as `c10c941048014cfc8712b0d02eee438a3dabaf6578c212e4c861d36a02d4f11a`, covering `if: always()`, every `needs` edge, result/output env wiring, and the fail-closed implementation;
- any modification to the aggregate job's dependencies, wiring, or script therefore fails repository policy validation even if names or fragments remain unchanged.

This consumes successor repair budget **3/3**. A new material finding on the next frozen head is a blocker requiring new explicit owner authorization, not a fourth successor repair cycle.

## Acceptance criteria

- [x] successor authorization is recorded without erasing predecessor `3/3` evidence;
- [x] branch reconciled with accepted current main baseline;
- [x] canonical always-on trigger/root contract implemented;
- [x] trigger boundary normalization avoids the cycle-1 false failure without weakening substantive checks;
- [x] `.cargo/` changes and rename sources force PR Rust validation;
- [x] `.cargo/**` changes trigger retained post-merge Rust validation;
- [x] entire aggregate `validate` implementation is cryptographically pinned by repository policy validation;
- [x] exact-head recovery, explicit Dependency Review refs and rename-source classification remain enforced;
- [ ] final PR metadata names the resulting frozen cycle-3 head;
- [ ] exact-head full-diff self-review passes on that frozen head;
- [ ] transition `Agent governance / validate` passes on that exact head;
- [ ] aggregate `Merge gate / validate` and every applicable sub-gate pass on that exact head;
- [ ] independent Codex review of that exact head has no open material finding;
- [ ] all repaired historical review threads are resolved with final-head evidence;
- [ ] branch is still up to date with current `main` immediately before merge;
- [ ] squash merge uses the unchanged validated expected head SHA;
- [ ] post-merge repository configuration proves the live `Protect main` ruleset requires exactly `Merge gate / validate`.

## Excluded scope

- no gameplay/client/server runtime behavior;
- no protocol, persistence or content semantics;
- no production deployment or secret expansion;
- no cross-repository writes;
- no weakening of exact-head, review or branch-protection requirements;
- no unrelated branch/task cleanup.

## Validation

### Focused

- cycle 1 canonical trigger/root implementation: reviewed;
- cycle 2 boundary-normalization repair: reviewed;
- cycle 3 `.cargo` selectors + full aggregate job hash pin: implemented; executable exact-head validation pending final freeze.

### Component/integration

- repository configuration apply/verify: post-merge by design;
- no product component integration applies.

### E2E

- `NOT_APPLICABLE` — repository governance/CI transition only; no game journey behavior changes.

### Exact-head CI

- final head: pending this task-record commit;
- trigger source: pending `pull_request/synchronize`;
- transition governance: pending;
- aggregate merge gate: pending;
- result: pending.

## Self-review

- candidate `51d3be4d08219358edf25d18f48f1fe959e3993d`: self-review PASS before independent review; independent review then found P1/P2 above, so it is superseded;
- final cycle-3 head: pending this task-record commit;
- method/reviewer: implementing/coordinating agent full-diff + successor-delta review;
- verdict: pending.

## Independent review

- required: `YES` — sole protected merge-authority transition has repository-wide common-mode-error risk;
- auditor: automatic `chatgpt-codex-connector` PR review;
- latest reviewed candidate `51d3be4d08219358edf25d18f48f1fe959e3993d`: P1/P2 repaired in successor cycle 3;
- final cycle-3 head verdict: pending.

## PR and closeout

- PR: #162, draft during final successor validation;
- changed-file review: pending frozen cycle-3 head;
- unresolved review threads: intentionally remain until final-head evidence exists;
- merge: pending exact-head CI + independent review + up-to-date branch proof;
- live ruleset verification: pending post-merge repository-configuration workflow;
- archive/ownership release: separate terminal closeout after merge and live verification.

## Context checkpoint

```yaml
last_progress: Final successor review found aggregate-job bypass and missing .cargo sensitivity; both were repaired by full validate-job SHA-256 pinning and .cargo selectors in PR/post-merge Rust CI.
status: validating
branch: ci/OTV2-20260811-merge-gate-hardening
head_sha: null
pr: 162
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pending-final-freeze
ci_check_generation: successor-1-cycle-3-pre-freeze
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
predecessor_repair_cycles: 3
repair_cycles_for_current_gate: 3
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Treat this task-record commit as the last content commit, update PR metadata to its exact resulting SHA without moving the branch, then freeze and perform complete exact-head self-review, CI and independent review. Any new material finding blocks the successor package.
```
