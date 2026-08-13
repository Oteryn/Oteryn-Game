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
base_sha: 5518a562bfea55f4f75e3aae03775b33fb55581e
original_base_sha: f184930fac66fdf9ae0cc7f606d3502c17626a79
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT repository engineering agent
created_at: 2026-08-11T10:30:00+02:00
updated_at: 2026-08-13T17:38:00+02:00
execution_budget_minutes: 120
large_budget_reason: Repository merge-authority transition plus exact-head Linux/Windows/security validation and independent review repair.
successor_generation: 2
predecessor_repair_cycles: 3
successor_generation_1_repair_cycles: 3
successor_generation_2_repair_cycles: 2
successor_generation_1_owner_authorized_at: 2026-08-11T16:07:00+02:00
successor_generation_1_owner_authorization: Autoryzuję successor repair package dla PR #162
successor_generation_2_owner_authorized_at: 2026-08-13T17:07:00+02:00
successor_generation_2_owner_authorization: dokoncz zadnaie
owner_funded_ai_authorized_for_generation_2: false
owned_paths:
  - .github/workflows/merge-gate.yml
  - .github/workflows/merge-authority-audit.yml
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

Replace the weak single-purpose required PR check with one stable `Merge gate / validate` context that composes governance, Dependency Review, CodeQL and path-proportional Rust Linux/Windows/policy/supply-chain checks. Move the merge-authority root of trust out of the ordinary PR-modifiable code domain by applying a live GitHub ruleset file-path restriction to workflow/policy/repository-administration paths.

## Architecture and source of truth

- `PROVEN`: PR #162 is the bounded delivery vehicle for repository engineering/governance only.
- `PROVEN`: predecessor generation and successor generation 1 each consumed `3/3` repair cycles without merge.
- `PROVEN`: successor generation 1 exact head `97d3dd0a304446acbbfbb68b4365af4f8fd8c692` passed complete aggregate CI; independent review then found P1 `PRRT_kwDOTuGrds6YRHL9`, proving the `scope.rust` producer remained mutable.
- `PROVEN`: current branch contains `main@5518a562bfea55f4f75e3aae03775b33fb55581e` as merge-base and is not behind that main baseline.
- `PROVEN`: current root `AGENTS.md` forbids owner-funded Codex/OpenAI use without exact-use authorization; generation 2 has not invoked Codex.
- `PROVEN`: GitHub's repository ruleset API supports `file_path_restriction` with `restricted_file_paths`, and GitHub documents fnmatch path matching.
- `DERIVED`: hashes stored in the same PR-modifiable repository are useful regression checks but are not an independent root of trust. The authoritative protection must therefore be enforced by the live GitHub ruleset outside the commit graph being reviewed.

## Retained repair history

### Predecessor generation — 3/3

1. exact-head dispatch recovery, task recoverability and canonical GitHub governance alignment;
2. rename-source `previous_filename` Rust/workspace classification;
3. correction of an overbroad static `paths:` substring assertion.

### Successor generation 1 — 3/3

1. canonical always-on trigger/root contract;
2. trigger-block boundary normalization;
3. `.cargo/` Rust sensitivity plus full aggregate `validate` job SHA-256 pinning.

### Successor generation 2

#### Cycle 1 — pin the scope producer

- retained the canonical merge-gate scope implementation;
- pinned the complete normalized `scope` job as SHA-256 `76c77c3b2b939e955aceb63441172fd1a77cb1e384cb58ac70c0cade4ab8d729`;
- retained aggregate consumer pin `c10c941048014cfc8712b0d02eee438a3dabaf6578c212e4c861d36a02d4f11a`;
- this closes isolated mutations such as replacing the executable Rust classifier with `rust = False` when the repository validator is trusted.

#### Cycle 2 — move root of trust to GitHub Rulesets

Self-review of generation-2 cycle 1 found a broader common-mode risk: `.github/workflows/merge-gate.yml`, `.github/repository-policy.json` and `tools/repository/validate_repository_policy.py` were all modifiable by the same PR, so a coordinated malicious change could alter the workflow and its expected hashes together.

Repair:

- `.github/repository-policy.json` now includes active `file_path_restriction` with no bypass actors for:
  - `.github/workflows/*`;
  - `.github/workflows/**/*`;
  - `.github/repository-policy.json`;
  - `tools/repository/*`;
  - `tools/repository/**/*`;
- `tools/repository/apply_github_settings.py` already submits the complete machine ruleset payload, so the new rule is applied by the existing post-merge repository-configuration workflow;
- after apply, ordinary PRs cannot change the required-status workflow, add a workflow that consumes `REPO_ADMIN_TOKEN`, change the machine ruleset policy, or change repository administration/validation scripts;
- future legitimate merge-authority changes require explicit out-of-band owner action in GitHub Settings to alter the restriction, then a bounded PR, exact-head validation/review, and restoration/verification of the intended restriction;
- no routine bypass actors are introduced;
- `docs/repository/GITHUB_GOVERNANCE.md` records this control-plane boundary.

A separate deterministic `.github/workflows/merge-authority-audit.yml` provides the required non-AI independent audit mechanism for this high-risk bootstrap. It resolves the exact PR head, independently checks the machine ruleset contract, verifies the restricted paths, and performs adversarial mutation tests proving the repository validator rejects `rust = False`, aggregate-result bypass, and merge-gate path filtering. It does not consume owner-funded AI quota.

## Acceptance criteria

- [x] branch reconciled with `main@5518a562bfea55f4f75e3aae03775b33fb55581e`;
- [x] predecessor and successor-generation-1 evidence retained;
- [x] generation 2 recorded separately;
- [x] `scope` producer and aggregate `validate` consumer regression-pinned;
- [x] `.cargo/` and rename-source Rust sensitivity retained;
- [x] machine ruleset declares `file_path_restriction` for all merge-authority/control-plane paths with no bypass actors;
- [x] deterministic non-AI merge-authority audit workflow added;
- [x] canonical GitHub governance documents immutable-by-default control-plane policy;
- [x] owner-funded Codex/OpenAI restriction respected;
- [ ] freeze one exact final generation-2 head after this task-record commit;
- [ ] full-diff self-review passes on that exact head;
- [ ] transition `Agent governance / validate` passes on exact head;
- [ ] `Merge gate / validate` and every applicable sub-gate pass on exact head;
- [ ] `Merge authority audit / validate` passes on exact head;
- [ ] repaired historical review threads are resolved with final-head evidence;
- [ ] branch remains up to date with current `main` immediately before merge;
- [ ] squash merge uses unchanged validated expected head SHA;
- [ ] post-merge repository-configuration run succeeds;
- [ ] live `Protect main` ruleset is independently read back and proves required status `Merge gate / validate`, active `file_path_restriction`, exact restricted paths, and zero bypass actors;
- [ ] task is archived and ownership released.

## Excluded scope

- no gameplay/client/server runtime behavior;
- no protocol, persistence or content semantics;
- no production deployment or secret expansion;
- no cross-repository writes;
- no owner-funded Codex/OpenAI invocation without separate exact-use authorization;
- no routine ruleset bypass actor;
- no unrelated cleanup.

## Validation

### Focused

- machine policy contains the exact file-path restriction and no bypass actors;
- merge-gate scope pin catches direct `rust = False` mutation;
- deterministic audit independently checks the policy and runs adversarial mutation probes;
- executable final result pending exact-head Actions.

### Component/integration

- existing `repository-configuration.yml` will apply the complete ruleset post-merge using the already-established administration token boundary;
- live ruleset readback is required after merge.

### E2E

- `NOT_APPLICABLE` — repository governance/CI transition only; no game journey behavior changes.

### Exact-head CI

- final head: pending this final content commit;
- transition governance: pending;
- aggregate merge gate: pending;
- deterministic merge-authority audit: pending.

## Self-review

- cycle-1 self-review finding: same-PR workflow/validator trust domain; repaired by ruleset-level file restriction in cycle 2;
- final full-diff exact-head self-review: pending freeze.

## Independent review

- required: `YES` — protected merge authority changes have repository-wide blast radius;
- owner-funded Codex: `NOT AUTHORIZED / NOT INVOKED` for generation 2;
- mechanism: dedicated deterministic `Merge authority audit / validate` workflow on the exact final head, independently checking machine ruleset invariants and adversarial mutations;
- verdict: pending final exact-head run.

## PR and closeout

- PR: #162, draft during final validation;
- merge: pending exact-head transition governance + aggregate gate + deterministic audit + review-thread hygiene + current-main proof;
- post-merge: repository-configuration workflow plus connector readback of live ruleset;
- archive/ownership release: terminal closeout after live proof.

## Context checkpoint

```yaml
last_progress: Generation 2 cycle 2 moved the merge-authority root of trust to an external GitHub ruleset file-path restriction and added a deterministic non-AI adversarial audit workflow.
status: validating
branch: ci/OTV2-20260811-merge-gate-hardening
head_sha: null
pr: 162
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pending-final-freeze
ci_check_generation: successor-2-cycle-2-pre-freeze
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
repair_cycles_for_current_gate: 2
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Treat this task-record commit as the final content commit, freeze the resulting SHA, perform full exact-head self-review, wait for transition governance + aggregate merge gate + deterministic audit, resolve repaired review threads, prove current-main freshness, squash merge unchanged head, then verify the post-merge live ruleset and archive the task.
```
