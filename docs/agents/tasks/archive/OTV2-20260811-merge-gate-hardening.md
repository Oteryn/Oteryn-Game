# OTV2-20260811-merge-gate-hardening — archived

```yaml
task_id: OTV2-20260811-merge-gate-hardening
title: Harden PR merge gating and repository engineering drift controls
mode: REPAIR
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
original_delivery_pr: 162
superseded_replacement_pr: 237
aggregate_gate_delivery_pr: 238
aggregate_gate_merge_sha: e8f9108014d12043535b56d8fc25fcb0e3390a51
public_control_plane_repair_pr: 241
public_control_plane_repair_head_sha: 87d113271ce42b8a1369fb083caff18f4980775c
public_control_plane_repair_merge_sha: 3a8add69e76221597f2973c9873521d82fb83568
lifecycle_closeout_branch: ci/merge-gate-hardening-closeout-20260813
lifecycle_closeout_pr: 242
owner: released_after_closeout
created_at: 2026-08-11T10:30:00+02:00
completed_at: 2026-08-13T20:16:00+02:00
execution_budget_minutes: 120
owner_funded_ai_authorized_for_current_generation: false
implementation_status: COMPLETED_REPOSITORY_GOVERNANCE
runtime_client_authority: NONE
postgresql_ddl_migration_authority: NONE
platform_write_authority: NONE
production_deployment_authority: NONE
external_repositories: []
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
  - docs/agents/tasks/archive/OTV2-20260811-merge-gate-hardening.md
public_contracts:
  - .github/repository-policy.json
  - docs/repository/GITHUB_GOVERNANCE.md
  - docs/agents/BUILD_TEST_MATRIX.md
depends_on: []
blocks_released:
  - trusted repository-governance baseline for subsequent architecture delivery
```

## Outcome

The repository now has one stable aggregate pull-request authority context, `Merge gate / validate`, plus a GitHub-native public-repository control-plane boundary that prevents ordinary pull requests from self-authorizing changes to merge authority.

The final live model is:

```text
ordinary PR
-> exact-head Merge gate / validate
-> strict up-to-date Protect main
-> squash merge only

control-plane PR
-> same aggregate gate
+ base-branch Code Owner review on narrow control-plane paths
+ deterministic independent Merge Authority Audit when required by risk policy
```

The repository remains public. General required approving review count remains `0`; Code Owner review is required only for the deliberately narrow ownership map covering `.github/CODEOWNERS`, `.github/workflows/`, `.github/repository-policy.json`, and `tools/repository/`.

No gameplay, runtime, protocol, persistence/content, Platform, production deployment or external-repository semantic behavior was changed by this task.

## Durable delivery history

### Original package and successor generations

PR #162 carried the original aggregate merge-gate and repository-governance package. Multiple bounded repair generations addressed material findings including recovery semantics, task-record accuracy, rename-source classification, path-filter safety, `.cargo/**` classification, scope-producer integrity, aggregate-consumer integrity, ruleset separation, GitHub changed-file API limits and GHAS/cache-poisoning risks.

Replacement PR #237 became superseded after concurrent `main` movement and was not used as terminal delivery.

### Aggregate gate delivery — PR #238

PR #238 delivered the complete synchronized aggregate-gate package and squash-merged as:

`e8f9108014d12043535b56d8fc25fcb0e3390a51`

It established:

- one required `Merge gate / validate` context;
- always-required scope/governance/Dependency Review/CodeQL layers;
- path-proportional Rust Linux/Windows/policy/supply-chain validation;
- rename-source and `.cargo/**` classification;
- fail-closed changed-file enumeration, including GitHub's 3,000-file API cap and count mismatch;
- removal of unsafe pull-request-code execution through `workflow_dispatch` recovery;
- deterministic merge-authority audit support;
- strict branch protection/readback expectations.

The `Protect main` branch-ruleset portion of #238 applied successfully. However the required post-merge integration proof failed before the separate control-plane restriction could be installed.

## Post-#238 failure and root cause

Required `Repository configuration` run `31726698230` on exact merge SHA `e8f9108014d12043535b56d8fc25fcb0e3390a51` failed in job `94536734816` at `Apply and verify GitHub settings`.

GitHub returned:

```text
POST /rulesets failed with 422:
Target ref_name is not supported for push rulesets
```

Live inspection also established that `Oteryn-v2` is public, while GitHub push-ruleset support is not an appropriate enforcement mechanism for this ordinary public repository. Removing only `ref_name` would therefore not have been a sufficient repair.

The failure was classified as a real platform/architecture incompatibility, not flaky CI, and the task correctly remained open.

## Public control-plane repair — PR #241

PR #241 implemented the public-repository-safe enforcement model.

### Native public strategy

`.github/CODEOWNERS` is deliberately narrow and owns exactly:

```text
/.github/CODEOWNERS @blakinio
/.github/workflows/ @blakinio
/.github/repository-policy.json @blakinio
/tools/repository/ @blakinio
```

`Protect main` requires Code Owner review while keeping general `required_approving_review_count=0`. It also dismisses stale approvals on push. Because GitHub resolves CODEOWNERS from the pull request base branch, a control-plane PR cannot replace the ownership mapping in the same change to authorize itself.

Ordinary architecture/runtime/content paths are intentionally absent from CODEOWNERS and therefore do not gain a new routine human-approval requirement.

### Private/internal latent strategy

The machine policy retains `Protect repository control plane` as a latent dedicated push-ruleset strategy for a future supported private/internal repository. It contains no branch-only `ref_name` condition.

`tools/repository/apply_github_settings.py` selects enforcement from live repository visibility:

- public -> required Code Owner fallback and no live control-plane push ruleset;
- private/internal -> dedicated push-ruleset strategy, still requiring post-apply API verification.

This does not claim that visibility alone guarantees every future plan entitlement; a future visibility/plan change must pass the same live apply/readback proof and fails closed otherwise.

### Break-glass model

With one maintainer, future legitimate changes to the protected control plane are intentionally break-glass work: explicitly alter the live Code Owner requirement for the bounded governance change, require exact-head independent audit, merge only after all gates pass, then restore canonical policy and prove it through post-merge configuration/readback. Routine bypass actors remain forbidden.

## PR #241 final review evidence

Exact delivery head:

`87d113271ce42b8a1369fb083caff18f4980775c`

Final implementing-agent full-diff self-review: **PASS** after one final documentation/validator-compatibility repair; open material findings `0`.

Mandatory genuinely independent review used the deterministic non-AI `Merge Authority Audit / validate`, not the implementing agent and not owner-funded Codex/OpenAI.

Exact-head evidence:

- Agent Governance run `31728395200`: **PASS**;
- Merge Authority Audit run `31728395191`: **PASS**;
- aggregate Merge Gate run `31728395207`: **PASS**;
- `Merge gate / scope`: PASS;
- Dependency Review: PASS;
- CodeQL Python: PASS;
- CodeQL Actions: PASS;
- Rust policy/metadata: PASS;
- Rust Linux workspace build/Clippy/tests/synthetic harness: PASS;
- Rust Windows client build/Clippy/visible smoke/synthetic harness: PASS;
- Rust supply-chain: PASS;
- Merge Gate governance validator: PASS;
- terminal `Merge gate / validate`: PASS;
- unresolved review threads before merge: `0`;
- exact head unchanged through merge;
- current `main` ancestry verified immediately before merge.

PR #241 squash-merged exact head as:

`3a8add69e76221597f2973c9873521d82fb83568`

Owner-funded Codex/OpenAI usage was **NOT AUTHORIZED / NOT INVOKED** for this governance repair.

## Post-merge integration and live readback

Required `Repository configuration` run `31729712428` on exact merge SHA `3a8add69e76221597f2973c9873521d82fb83568`: **PASS**. Job `apply`, including `Apply and verify GitHub settings`, completed successfully.

Independent live GitHub API readback after that PASS proved:

- repository visibility: `public`;
- live rulesets contain only `Protect main`; no unsupported public `Protect repository control plane` push ruleset exists;
- `Protect main` target is `branch`, enforcement is `active`;
- `bypass_actors=[]` and current user bypass is `never`;
- pull requests: squash-only;
- `required_approving_review_count=0`;
- `require_code_owner_review=true`;
- `dismiss_stale_reviews_on_push=true`;
- `require_last_push_approval=false`;
- `required_review_thread_resolution=true`;
- required status checks contain exactly `Merge gate / validate`;
- strict/up-to-date required-status policy is enabled;
- base-branch CODEOWNERS contains exactly the four narrow control-plane ownership entries above.

This satisfies the terminal repository-governance integration proof that #238 alone had not achieved.

## Acceptance result

- [x] stable exact-head aggregate merge gate delivered;
- [x] changed-file and rename classification fail closed;
- [x] full Rust validation is path-proportional and included for repository/tooling-sensitive changes;
- [x] unsafe PR-code recovery path removed;
- [x] branch protection requires exactly the aggregate gate in strict mode;
- [x] no branch-ruleset bypass actors;
- [x] #238 post-merge platform incompatibility recorded rather than hidden;
- [x] public repository uses required Code Owner review on narrow control-plane paths;
- [x] ordinary PRs retain general approval count `0`;
- [x] stale Code Owner approvals are dismissed on push;
- [x] latent private/internal push strategy contains no invalid branch condition;
- [x] apply/readback is visibility-aware and fail-closed;
- [x] independent deterministic merge-authority audit passed on exact #241 head;
- [x] aggregate exact-head Merge Gate passed on #241;
- [x] post-merge Repository configuration passed on the exact merge SHA;
- [x] live ruleset/readback matches canonical policy;
- [x] owner-funded AI restriction preserved;
- [x] task ready for terminal archive/ownership release.

## Excluded scope preserved

No gameplay/client/server runtime change, protocol semantic change, persistence/content semantic change, production deployment, secret expansion, external-repository write, routine bypass actor, repository visibility change, organization migration or unrelated cleanup was introduced.

## Context checkpoint

```yaml
status: completed
aggregate_gate_delivery_pr: 238
aggregate_gate_merge_sha: e8f9108014d12043535b56d8fc25fcb0e3390a51
public_control_plane_repair_pr: 241
public_control_plane_repair_head_sha: 87d113271ce42b8a1369fb083caff18f4980775c
public_control_plane_repair_merge_sha: 3a8add69e76221597f2973c9873521d82fb83568
repository_configuration_run: 31729712428
repository_configuration_result: PASS
lifecycle_closeout_pr: 242
owner_action_required: false
blocker: null
next_action: Complete bookkeeping-only archive PR, release ownership, then resume pending GAME-ABILITY-01 effect-family/Reference-catalogue architecture work.
```
