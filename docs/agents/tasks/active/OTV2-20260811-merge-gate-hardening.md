# OTV2-20260811-merge-gate-hardening

```yaml
task_id: OTV2-20260811-merge-gate-hardening
title: Harden PR merge gating and repository engineering drift controls
mode: REPAIR
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: ci/public-control-plane-repair-20260813
pr: 241
original_delivery_pr: 162
superseded_replacement_pr: 237
merged_delivery_pr: 238
original_base_sha: f184930fac66fdf9ae0cc7f606d3502c17626a79
latest_verified_main_sha: e8f9108014d12043535b56d8fc25fcb0e3390a51
head_sha: pending_this_checkpoint_commit
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT repository engineering agent
created_at: 2026-08-11T10:30:00+02:00
updated_at: 2026-08-13T19:57:00+02:00
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
blocks:
  - trusted repository-governance baseline for subsequent architecture delivery
external_repositories: []
```

## Outcome

Deliver one stable `Merge gate / validate` context that composes repository/agent governance, Dependency Review, CodeQL and path-proportional Rust validation, while keeping merge-authority/control-plane configuration outside ordinary self-authorizing PR changes.

PR #238 delivered the aggregate gate as merge commit `e8f9108014d12043535b56d8fc25fcb0e3390a51`. The branch-ruleset portion applied successfully and live `Protect main` now requires exactly `Merge gate / validate`, strict/up-to-date, squash-only, no bypass actors. The task remains active because the required post-merge repository-configuration integration failed before control-plane protection was installed.

## Durable delivery history

- PR #162 carried the original complete change set and exhausted multiple bounded repair generations while resolving recovery, task-record, rename-source, path-filter, `.cargo/**`, scope-producer, aggregate-consumer, ruleset-separation, changed-file-cap and GHAS cache-poisoning findings.
- Replacement PR #237 was superseded after concurrent `main` movement.
- Final replacement PR #238 was based on then-current `main@9c87a1dfde6bee88a99e0b2b9d610008bc2a4aa0`, passed its required exact-head Merge Gate / Merge Authority Audit evidence, and squash-merged as `e8f9108014d12043535b56d8fc25fcb0e3390a51`.
- No owner-funded Codex/OpenAI review was invoked in the current generation. Independent evidence uses deterministic non-AI audit workflows as permitted by root `AGENTS.md`.

## Post-merge failure — FACT

Required `Repository configuration` run `31726698230` on exact merge SHA `e8f9108014d12043535b56d8fc25fcb0e3390a51` failed in job `94536734816`, step `Apply and verify GitHub settings`.

GitHub returned:

```text
POST /rulesets failed with 422:
Target ref_name is not supported for push rulesets
```

Live readback after the partial apply proves:

- `Protect main` exists, is active, has zero bypass actors, requires squash PRs and exactly `Merge gate / validate` in strict mode;
- no `Protect repository control plane` push ruleset exists.

Current GitHub documentation further establishes that push rulesets are supported for private/internal repositories (and eligible fork networks), not for an ordinary public repository. `blakinio/Oteryn-v2` is public. Therefore removing only `ref_name` would still leave an unsupported public-repository design.

## Accepted repair strategy for the current public repository

PR #241 replaces the unavailable public push-ruleset enforcement with GitHub-native **required Code Owner review** on a deliberately narrow base-branch ownership map.

The public-repository control-plane boundary is:

- `.github/CODEOWNERS` owns itself, `.github/workflows/`, `.github/repository-policy.json`, and `tools/repository/` by `@blakinio`;
- broad/default CODEOWNERS entries are removed so ordinary architecture/runtime/content PRs remain approval-count `0`;
- `Protect main` sets `require_code_owner_review=true` and `dismiss_stale_reviews_on_push=true` while keeping general `required_approving_review_count=0`;
- GitHub resolves CODEOWNERS from the PR base branch, so a PR cannot alter its own ownership mapping to self-authorize;
- the `Protect repository control plane` push-ruleset definition remains in policy only as the latent strategy for a future private/internal repository, with no branch-only `ref_name` condition;
- `apply_github_settings.py` selects enforcement from live repository visibility: public -> Code Owner fallback and no live push ruleset; private/internal -> dedicated push ruleset;
- post-merge readback must prove the selected mode, not merely accept local policy syntax.

With a single maintainer, future control-plane changes are intentionally break-glass: explicitly alter the live Code Owner requirement in GitHub Settings for the bounded governance change, require exact-head independent audit, then restore canonical policy and prove it through repository-configuration/readback. Routine bypass actors remain forbidden.

## Independent audit

Governance/merge authority has repository-wide blast radius, so independent review is mandatory.

`Merge authority audit / validate` on the exact PR #241 head is the deterministic non-AI independent mechanism. The repaired audit checks:

- branch ruleset required-status and pull-request review parameters;
- narrow CODEOWNERS contents and absence of broad default ownership;
- public Code Owner fallback declaration;
- latent private/internal push-ruleset syntax/path list and absence of `ref_name`;
- visibility-aware apply/readback logic;
- canonical aggregate merge-gate integrity and adversarial mutation rejection.

Owner-funded Codex/OpenAI usage remains **NOT AUTHORIZED / NOT INVOKED**.

## Acceptance criteria

### Aggregate-gate delivery retained

- [x] aggregate exact-head `Merge gate / validate` implemented;
- [x] `.cargo/**`, rename-source and complete changed-file classification retained;
- [x] changed-file enumeration fails closed above GitHub's 3,000-file cap and on count mismatch;
- [x] unsafe pull-request-code recovery through `workflow_dispatch` removed;
- [x] final delivery PR #238 merged without force/bypass;
- [x] live `Protect main` readback proves strict exact aggregate required status and zero bypass actors.

### Public control-plane repair

- [x] exact post-merge failure classified as product/platform incompatibility, not flaky CI;
- [x] public/private/internal ruleset capability checked against current GitHub documentation;
- [x] CODEOWNERS narrowed to merge-authority paths only and owns itself;
- [x] branch policy requires Code Owner review for owned paths and dismisses stale approvals;
- [x] latent push ruleset removes invalid `ref_name` and remains private/internal-only policy;
- [x] apply/readback is visibility-aware and removes an unsupported stale public push ruleset if present;
- [x] deterministic Merge Authority Audit covers the repaired fallback boundary;
- [x] governance documentation records platform limitation, fallback and break-glass process;
- [ ] reconcile PR #241/task metadata and freeze exact final head;
- [ ] full-diff self-review PASS with zero material findings on exact final head;
- [ ] exact-head `Merge gate / validate` PASS with all applicable sub-gates;
- [ ] exact-head `Merge authority audit / validate` PASS as mandatory independent audit;
- [ ] zero unresolved review threads;
- [ ] current `main` remains an ancestor immediately before merge;
- [ ] squash merge #241 using unchanged expected head SHA;
- [ ] post-merge `Repository configuration` PASS;
- [ ] live readback proves Code Owner review + stale-approval dismissal + aggregate required status + zero bypass actors;
- [ ] live readback proves no unsupported public control-plane push ruleset exists;
- [ ] archive this task and release ownership.

## Excluded scope

No gameplay/client/server runtime behavior, protocol semantics, persistence/content semantics, production deployment, secret expansion, cross-repository writes, owner-funded AI use, routine bypass actors, repository visibility change, organization migration, or unrelated cleanup.

## Validation

### Focused

Inspect exact PR #241 diff against GitHub public-repository ruleset capabilities, base-branch CODEOWNERS semantics, no-bypass intent, current aggregate merge-gate invariants and apply/readback failure paths.

### Component/integration

Post-merge `.github/workflows/repository-configuration.yml` on the exact merge SHA is the required integration proof.

### E2E

`NOT_APPLICABLE` — repository governance/control-plane repair only.

### Exact-head CI

Pending final PR #241 head. Historical #238 checks do not transfer.

## Self-review

Pending final exact head.

## Context checkpoint

```yaml
status: validating
branch: ci/public-control-plane-repair-20260813
pr: 241
head_sha: pending_this_checkpoint_commit
final_head_sha: null
latest_verified_main_sha: e8f9108014d12043535b56d8fc25fcb0e3390a51
owner_action_required: false
blocker: null
next_action: inspect full PR #241 diff, repair findings, freeze exact head, require Merge Gate plus independent Merge Authority Audit, merge only with current-main ancestry, then require post-merge Repository configuration PASS and live ruleset readback before task closeout.
```
