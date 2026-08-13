# OTV2-20260811-merge-gate-hardening

```yaml
task_id: OTV2-20260811-merge-gate-hardening
title: Harden PR merge gating and repository engineering drift controls
mode: REPAIR
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: ci/OTV2-20260811-merge-gate-hardening-final3
pr: 238
original_delivery_pr: 162
superseded_replacement_pr: 237
original_base_sha: f184930fac66fdf9ae0cc7f606d3502c17626a79
latest_verified_main_sha: 9c87a1dfde6bee88a99e0b2b9d610008bc2a4aa0
head_sha: pending_this_checkpoint_commit
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT repository engineering agent
created_at: 2026-08-11T10:30:00+02:00
updated_at: 2026-08-13T19:27:00+02:00
execution_budget_minutes: 120
owner_funded_ai_authorized_for_current_generation: false
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
external_repositories: []
```

## Outcome

Replace the weak single-purpose PR check with one stable `Merge gate / validate` context that composes repository/agent governance, Dependency Review, CodeQL and path-proportional Rust Linux/Windows/policy/supply-chain checks. Protect merge-authority configuration after bootstrap with a separate no-bypass GitHub push ruleset.

## Proven final design

- `Protect main` remains a `target: branch` ruleset and after post-merge configuration requires exactly `Merge gate / validate`, strict/up-to-date, squash-only, zero bypass actors.
- `Protect repository control plane` is a separate `target: push` ruleset with zero bypass actors and exactly one `file_path_restriction` protecting `.github/workflows/*`, `.github/workflows/**/*`, `.github/repository-policy.json`, `tools/repository/*`, and `tools/repository/**/*`.
- Pull-request changed-file enumeration reads PR `changed_files`, fails closed above GitHub's 3,000-file API cap, and fails closed when paginated enumeration count differs from metadata.
- Rename source paths via `previous_filename` and `.cargo/**` are included in Rust-sensitive classification.
- Merge gate and deterministic merge-authority audit are `pull_request` only; unsafe PR-code recovery through `workflow_dispatch` was removed after GHAS cache-poisoning findings.
- Missing unchanged-head validation is recovered by close/reopen of the unchanged PR so normal `pull_request: reopened` trust semantics apply.
- Aggregate consumer and scope producer remain regression-pinned; deterministic merge-authority audit performs adversarial mutation checks.
- `tools/repository/apply_github_settings.py` applies and reads back both rulesets; `tools/repository/validate_repository_policy.py` validates their separation and canonical contract.

## Repair history

Predecessor and successor generation 1 repaired recovery/task-record drift, rename-source classification, path-filter validation, canonical trigger shape, `.cargo/**` sensitivity and aggregate consumer pinning. Successor generation 2 repaired scope-producer mutability, moved merge-authority trust to GitHub rulesets, split invalid branch/push rule types, added 3,000-file/count fail-closed behavior, and removed unsafe `workflow_dispatch` execution after GHAS findings.

PR #162 carried the original complete change set and reached exact head `d8aab7853dd8fe280c3f825dda6fb3b786e2ff36` with all applicable aggregate jobs PASS, Agent Governance PASS, deterministic Merge Authority Audit PASS and zero unresolved review threads. Strict branch freshness then blocked merge because `main` moved and the connector repeatedly blocked fast-forwarding the original ref.

A protection-respecting replacement path was used instead of force/bypass. Merge commits incorporated intervening non-overlapping GAME-ABILITY documentation from `main`; replacement PR #237 was superseded when `main` moved again. Final delivery PR #238 uses branch `ci/OTV2-20260811-merge-gate-hardening-final3`. Fast-forwarding that replacement branch succeeded. Its current history contains observed `main@9c87a1dfde6bee88a99e0b2b9d610008bc2a4aa0` through merge commit `8d9012bfc4033830dbe607cf1f39bac8375ee8d1`.

No owner-funded Codex/OpenAI review was invoked in the current generation. Independent final evidence is the deterministic non-AI `Merge authority audit / validate` workflow, as permitted by root `AGENTS.md`.

## Acceptance criteria

- [x] aggregate exact-head merge gate implemented;
- [x] scope producer and aggregate consumer protected against isolated mutation;
- [x] `.cargo/**` and rename-source classification retained;
- [x] branch and push rulesets separated by valid GitHub rule type/target;
- [x] control-plane push ruleset has exact protected paths and no bypass actors;
- [x] changed-file enumeration fails closed above 3,000 and on count mismatch;
- [x] unsafe manual PR-code recovery removed;
- [x] GHAS cache-poisoning findings resolved after the security repair;
- [x] P1/P2 findings on ruleset split and changed-file cap repaired and resolved on #162;
- [x] final delivery branch incorporates observed `main@9c87a1dfde6bee88a99e0b2b9d610008bc2a4aa0` without force or bypass;
- [ ] freeze resulting exact PR #238 head after this checkpoint commit;
- [ ] full-diff self-review PASS with zero material findings on that exact head;
- [ ] exact-head `Agent governance / validate` PASS;
- [ ] exact-head `Merge gate / validate` PASS with every applicable sub-gate;
- [ ] exact-head `Merge authority audit / validate` PASS;
- [ ] zero unresolved review threads on #238;
- [ ] current `main` remains an ancestor immediately before merge;
- [ ] squash merge #238 using unchanged expected head SHA;
- [ ] close superseded PRs #162 and #237;
- [ ] post-merge `repository-configuration.yml` PASS;
- [ ] live readback proves `Protect main` exact required status, strict mode and zero bypass actors;
- [ ] live readback proves dedicated push ruleset exact protected paths and zero bypass actors;
- [ ] archive this task and release ownership.

## Excluded scope

No gameplay/client/server runtime behavior, protocol semantics, persistence/content semantics, production deployment, protected-environment authority, secret expansion, cross-repository writes, owner-funded AI use, routine bypass actors, or unrelated cleanup.

## Validation

### Focused

Full changed-file set remains the 14 declared repository governance/CI files. Repository validator and deterministic audit cover canonical trigger, exact-head resolution, fail-closed changed-file classification, ruleset split, scope/aggregate integrity and recovery security.

### Component/integration

`repository-configuration.yml` is the required post-merge integration and must apply/read back both rulesets.

### E2E

`NOT_APPLICABLE` — repository governance/CI transition only.

### Exact-head CI

Previous #162 exact head `d8aab7853dd8fe280c3f825dda6fb3b786e2ff36`: Agent Governance PASS; Merge Authority Audit PASS; aggregate scope/governance/dependency/CodeQL/Rust policy/Linux/Windows/supply-chain/validate all PASS. These results do not transfer to #238; fresh exact-head evidence is required.

## Self-review

Previous full-diff self-review repaired documentation drift and found no new material security/architecture issue after the final ruleset/cap/recovery repairs. A fresh self-review is required on the resulting #238 head because this checkpoint changes SHA.

## Independent review

- required: YES — merge authority has repository-wide blast radius;
- mechanism: deterministic non-AI `Merge authority audit / validate` on exact final head;
- owner-funded Codex/OpenAI: NOT AUTHORIZED / NOT INVOKED for current generation;
- verdict: pending fresh #238 run.

## PR and closeout

PR #238 is the only authoritative delivery PR. PRs #162 and #237 are superseded and must not be merged. Merge only after fresh exact-head gates, zero unresolved threads, full-diff self-review and current-main ancestry all hold simultaneously.

## Context checkpoint

```yaml
status: validating
branch: ci/OTV2-20260811-merge-gate-hardening-final3
pr: 238
head_sha: pending_this_checkpoint_commit
final_head_sha: null
final_head_frozen_at: null
latest_verified_main_sha: 9c87a1dfde6bee88a99e0b2b9d610008bc2a4aa0
ci_trigger_source: pull_request
ci_check_generation: final-replacement-checkpoint
owner_action_required: false
blocker: null
next_action: Freeze the resulting PR #238 SHA; perform final full-diff self-review and fresh exact-head Agent Governance, Merge Gate and Merge Authority Audit; verify zero unresolved threads and current-main ancestry; squash-merge unchanged head; close superseded PRs; verify post-merge repository configuration and both live rulesets; archive this task and release ownership.
```
