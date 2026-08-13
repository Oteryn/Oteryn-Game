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
original_base_sha: f184930fac66fdf9ae0cc7f606d3502c17626a79
latest_verified_main_sha: 2b813f713a70c2be91c4ef7b6f052836a4658d16
head_sha: pending_this_checkpoint_commit
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT repository engineering agent
created_at: 2026-08-11T10:30:00+02:00
updated_at: 2026-08-13T19:15:00+02:00
execution_budget_minutes: 120
large_budget_reason: Repository merge-authority transition plus exact-head Linux/Windows/security validation and independent review repair.
successor_generation: 2
predecessor_repair_cycles: 3
successor_generation_1_repair_cycles: 3
successor_generation_2_repair_cycles: 5
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

Replace the weak single-purpose required PR check with one stable `Merge gate / validate` context that composes governance, Dependency Review, CodeQL and path-proportional Rust Linux/Windows/policy/supply-chain checks. Keep merge-authority configuration outside the ordinary PR-modifiable trust domain after bootstrap through a dedicated no-bypass GitHub push ruleset protecting workflow/policy/repository-administration paths.

## Architecture and source of truth

- `PROVEN`: PR #162 is the bounded delivery vehicle for repository engineering/governance only.
- `PROVEN`: predecessor generation and successor generation 1 each consumed `3/3` repair cycles without merge.
- `PROVEN`: successor generation 1 exact head `97d3dd0a304446acbbfbb68b4365af4f8fd8c692` passed aggregate CI; independent review then proved the `scope.rust` producer remained mutable.
- `PROVEN`: later authorized Codex review on exact head `612eebd55044b21330f0065ad26ab440f8461fe9` found two material issues: `file_path_restriction` must live in a dedicated push ruleset, and changed-file enumeration must fail closed at GitHub's 3,000-file API cap.
- `PROVEN`: GitHub Advanced Security then identified cache-poisoning risk from recovery through `workflow_dispatch` while executing PR-head code. The final design removes `workflow_dispatch` from the merge gate and merge-authority audit; recovery uses the ordinary `pull_request: reopened` event on an unchanged head.
- `PROVEN`: all historical GHAS cache-poisoning threads are auto-resolved after removal of `workflow_dispatch` on exact head `169819743ad7927166b83c883d1a4839441d7223`.
- `PROVEN`: exact head `169819743ad7927166b83c883d1a4839441d7223` passed `Agent governance / validate` run `31724293027` and deterministic `Merge authority audit / validate` run `31724292979`; aggregate run `31724292965` had every completed job green with Windows still running when this checkpoint was written.
- `PROVEN`: current live `Protect main` ruleset still requires transition check `Agent governance / validate`, has strict up-to-date enforcement, zero bypass actors and squash-only merge. Post-merge repository configuration must replace that required context with `Merge gate / validate` and install the dedicated push ruleset.
- `PROVEN`: latest observed `main` is `2b813f713a70c2be91c4ef7b6f052836a4658d16`; PR #162 is behind it and therefore cannot be merged until updated through a protection-respecting GitHub operation.
- `PROVEN`: direct connector ref movement was safety-blocked; no bypass was attempted. A temporary successor branch created during the safe-alternative experiment is not the delivery branch and is not authoritative.
- `PROVEN`: current root `AGENTS.md` forbids owner-funded Codex/OpenAI use without exact-use authorization; no additional owner-funded AI review has been invoked after the authorized review of `612eebd55044b21330f0065ad26ab440f8461fe9`.
- `DERIVED`: repository-local hashes are regression checks, not an external root of trust; the live no-bypass GitHub rulesets are the post-bootstrap enforcement boundary.

## Retained repair history

### Predecessor generation — 3/3

1. initial exact-head recovery/task recoverability/GitHub governance alignment;
2. rename-source `previous_filename` Rust/workspace classification;
3. correction of overbroad path-filter validation.

### Successor generation 1 — 3/3

1. canonical always-on trigger/root contract;
2. trigger-block boundary normalization;
3. `.cargo/` Rust sensitivity plus complete aggregate `validate` job SHA-256 pinning.

### Successor generation 2 — 5 bounded cycles

1. **Pin scope producer.** Pin the complete normalized `scope` producer in addition to aggregate `validate`.
2. **Externalize merge-authority trust.** Add no-bypass path restriction and deterministic non-AI merge-authority audit.
3. **Split ruleset types.** Move `file_path_restriction` into dedicated `Protect repository control plane` `target: push`; keep `Protect main` as the branch ruleset. Update apply/verify tooling, validator, audit and governance documentation to require both.
4. **Fail closed on changed-file API limits.** Read PR `changed_files`; reject values above 3,000 and reject any mismatch between metadata count and paginated enumeration before Rust-sensitive classification.
5. **Remove unsafe manual PR-code recovery.** Remove `workflow_dispatch` from merge gate and merge-authority audit after GHAS cache-poisoning findings. Recovery of a missing unchanged-head PR run is close/reopen of the unchanged PR, producing the normal `pull_request: reopened` event and ordinary PR trust context.

## Final control-plane model

`Protect main` (`target: branch`):

- active, no bypass actors;
- deletion/non-fast-forward protection and linear history;
- pull-request-only squash merge;
- strict required status `Merge gate / validate` after post-merge apply.

`Protect repository control plane` (`target: push`):

- active, no bypass actors;
- exactly one `file_path_restriction` protecting:
  - `.github/workflows/*`;
  - `.github/workflows/**/*`;
  - `.github/repository-policy.json`;
  - `tools/repository/*`;
  - `tools/repository/**/*`.

`tools/repository/apply_github_settings.py` applies and independently reads back both named rulesets. `tools/repository/validate_repository_policy.py` validates their separation and canonical contracts. `.github/workflows/merge-authority-audit.yml` supplies deterministic non-AI adversarial evidence on protected merge-authority changes.

## Acceptance criteria

- [x] predecessor and successor-generation-1 evidence retained;
- [x] `scope` producer and aggregate `validate` consumer regression-pinned;
- [x] `.cargo/` and rename-source Rust sensitivity retained;
- [x] branch and push rulesets separated by valid GitHub ruleset target/type;
- [x] push ruleset protects every canonical control-plane path with no bypass actors;
- [x] changed-file enumeration fails closed above 3,000 and on count mismatch;
- [x] unsafe `workflow_dispatch` PR-code recovery removed;
- [x] recovery documented as close/reopen of unchanged PR (`pull_request: reopened`);
- [x] deterministic non-AI merge-authority audit aligned with final ruleset/recovery model;
- [x] historical GHAS cache-poisoning review threads auto-resolved after the final security repair;
- [x] owner-funded Codex/OpenAI restriction respected after the earlier explicitly authorized review;
- [ ] freeze one exact final head after this task-record checkpoint;
- [ ] full-diff self-review passes on that exact head with zero material findings;
- [ ] `Agent governance / validate` passes on exact head;
- [ ] `Merge gate / validate` and every applicable sub-gate pass on exact head;
- [ ] `Merge authority audit / validate` passes on exact head;
- [ ] P1/P2 review threads are answered and resolved with final-head evidence;
- [ ] branch is up to date with the current `main` immediately before merge;
- [ ] squash merge uses the unchanged validated expected head SHA;
- [ ] post-merge `repository-configuration.yml` succeeds;
- [ ] live readback proves branch ruleset requires exactly `Merge gate / validate`, strict=true, zero bypass actors;
- [ ] live readback proves dedicated push ruleset has the exact canonical protected paths and zero bypass actors;
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

- full PR changed-file set is bounded to 14 repository governance/CI files;
- repository validator checks canonical pull-request-only aggregate gate, ruleset split, cap/count fail-closed behavior, scope/aggregate pins, rename-source and `.cargo/` classification;
- deterministic audit independently checks ruleset contracts and adversarial mutations;
- GHAS automatically resolved all workflow-dispatch cache-poisoning findings after the trigger was removed.

### Component/integration

- `repository-configuration.yml` is the post-merge integration that applies both rulesets using the existing administration-token boundary;
- live ruleset readback is mandatory before closeout.

### E2E

- `NOT_APPLICABLE` — repository governance/CI transition only; no game journey behavior changes.

### Exact-head CI

- previous exact head: `169819743ad7927166b83c883d1a4839441d7223`;
- Agent Governance: run `31724293027` PASS;
- deterministic Merge Authority Audit: run `31724292979` PASS;
- aggregate Merge Gate: run `31724292965`; all completed jobs PASS, Windows pending at checkpoint time;
- final exact head after this checkpoint: pending new Actions generation.

## Self-review

- final pre-checkpoint full-diff review found one documentation drift: this task record and PR body still described obsolete ruleset/recovery/SHA state. This task-record commit repairs the authoritative checkpoint; PR metadata will be reconciled without changing the code head.
- final full-diff review must be repeated on the resulting exact head before merge.

## Independent review

- required: `YES` — protected merge authority has repository-wide blast radius;
- owner-funded Codex: `NOT AUTHORIZED / NOT INVOKED` after the earlier explicitly authorized review cycle;
- mechanism for final generation: dedicated deterministic `Merge authority audit / validate` workflow on exact final head, independently checking ruleset invariants and adversarial mutations;
- verdict: pending final exact-head run after this checkpoint commit.

## PR and closeout

- PR: #162, open and ready for review during final validation;
- merge: pending exact-head transition governance + aggregate gate + deterministic audit + P1/P2 thread resolution + current-main proof;
- freshness: strict branch rule is active; current branch must be updated through GitHub without bypassing protection;
- post-merge: repository-configuration workflow plus connector readback of both live rulesets;
- archive/ownership release: terminal closeout after live proof.

## Context checkpoint

```yaml
last_progress: Final architecture removes workflow_dispatch PR-code recovery, separates branch/push rulesets, fails closed on changed-file API limits, and has GHAS-clean exact-head evidence on the preceding commit.
status: validating
branch: ci/OTV2-20260811-merge-gate-hardening
head_sha: pending_this_checkpoint_commit
pr: 162
final_head_sha: null
final_head_frozen_at: null
latest_verified_main_sha: 2b813f713a70c2be91c4ef7b6f052836a4658d16
ci_trigger_source: pull_request
ci_check_generation: successor-2-cycle-5-final-checkpoint
ci_checks_for_current_head: pending
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: pending
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
predecessor_repair_cycles: 3
successor_generation_1_repair_cycles: 3
repair_cycles_for_current_gate: 5
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: branch_freshness_if_safe_update_action_remains_unavailable
next_action: Freeze this resulting SHA, reconcile PR metadata, run final full-diff self-review and exact-head governance/aggregate/audit, answer and resolve repaired P1/P2 threads, then update branch through a protection-respecting GitHub operation and revalidate the updated exact head before squash merge. Post-merge, verify repository-configuration and read back both live rulesets before archiving the task.
```
