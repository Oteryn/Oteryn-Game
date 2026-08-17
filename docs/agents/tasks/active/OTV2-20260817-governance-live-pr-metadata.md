# OTV2-20260817-governance-live-pr-metadata

```yaml
task_id: OTV2-20260817-governance-live-pr-metadata
title: Eliminate stale PR metadata dependence from Agent governance
mode: FIX
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: fix/governance-live-pr-metadata
pr: 320
base_sha: 65b5711fc56f80c0407ce462e83cdc973535636f
head_sha: 3c91bdce08fcbfab2d749b5352c38a1171d6aa49
final_head_sha: null
final_head_frozen_at: null
owner: governance repair agent
created_at: 2026-08-17T09:22:00+02:00
updated_at: 2026-08-17T09:34:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - .github/workflows/agent-governance.yml
  - .github/workflows/merge-gate.yml
  - tools/repository/validate_repository_policy.py
  - docs/repository/GITHUB_GOVERNANCE.md
  - docs/agents/tasks/active/OTV2-20260817-governance-live-pr-metadata.md
public_contracts:
  - docs/repository/GITHUB_GOVERNANCE.md
depends_on:
  - main@65b5711fc56f80c0407ce462e83cdc973535636f
blocks:
  - reliable automatic metadata recovery for PR validation, including OTV2-TOKIO-239-class races
cross_repository_coordination_id: OTERYN-GOV-LIVE-PR-METADATA-20260817
external_repositories:
  - blakinio/Oteryn-Platform
  - blakinio/Otheryn
  - blakinio/otclient
```

## Outcome

Make PR-governance metadata validation self-healing for ordinary metadata edits without weakening exact-head controls: live PR title/body/state are resolved from GitHub, `edited` re-runs governance and the protected aggregate merge gate, and `workflow_dispatch` remains only a break-glass exact-head path where it is already appropriate.

## Proven problem

- `PROVEN`: PR #239 produced a fresh `synchronize` Agent-governance run whose event payload contained the old Dependabot body even though the PR body was repaired immediately afterwards.
- `PROVEN`: that run reported only missing `## Summary`, `## Scope`, and `## Validation`; the repaired live PR contained the headings and the exact-head Merge-gate governance job passed.
- `PROVEN`: trusted-base `Agent governance` read title/body from frozen `github.event.pull_request.*` fields on normal PR events and fetched live PR metadata only for `workflow_dispatch`.
- `PROVEN`: the active GitHub connector has rerun operations but no workflow-dispatch operation, so ordinary metadata repair must not depend on manual dispatch.
- `PROVEN`: `Merge gate / validate` is the single protected required status, and trusted-base `merge-gate.yml` did not handle `pull_request: edited`; therefore a metadata-only change could otherwise leave an older green required status covering later invalid metadata.

## Implementation

PR #320 now makes the following bounded changes:

1. `Agent governance`
   - explicitly handles `opened`, `reopened`, `synchronize`, and `edited`;
   - uses the PR event only for PR number plus expected head SHA;
   - fetches live title/body/head repository/base/state/head SHA from the GitHub REST API on both PR and dispatch paths;
   - fails closed when the live head differs from the triggering/dispatch head;
   - checks out only the verified live head;
   - retains same-repository, `main` base, conventional-title and required-heading checks.
2. `Merge gate`
   - preserves the trusted-base workflow byte-for-byte outside the trigger block;
   - adds the same explicit PR activity set including `edited`, so the protected aggregate status is recomputed after metadata changes without changing the head.
3. Repository validator/documentation
   - pins the new always-on trigger as the canonical merge-gate contract while preserving existing scope/aggregate job hashes;
   - documents metadata-edit revalidation and keeps `reopened` as recovery for genuinely suppressed initial PR events;
   - does not add `workflow_dispatch` to the merge gate or execute PR code in a privileged context.

PR concurrency remains keyed by PR number with `cancel-in-progress: true`, so a metadata `edited` event can supersede an older in-progress run for the same PR.

## Validation evidence so far

- Head `04643dfb5b4cdc92a57e41899c73cb953acdc13e`: `Agent governance / validate`, Merge Authority Audit and Architecture Semantic Audit passed before merge-gate trigger hardening was added.
- A first merge-gate edit accidentally replaced more trusted-base content than intended; CI correctly rejected it via canonical trigger/job hashes. It was repaired by restoring the trusted-base merge gate and retaining only the five-line trigger addition.
- Current diff against task base confirms `.github/workflows/merge-gate.yml` is now only `+5/-0`.
- Head `3c91bdce08fcbfab2d749b5352c38a1171d6aa49`: fresh Agent Governance, Merge Authority Audit and Architecture Semantic Audit passed; aggregate Merge Gate is still running/queued at this checkpoint.

## Acceptance criteria

- [x] `Agent governance` explicitly handles `opened`, `reopened`, `synchronize`, and `edited`.
- [x] Pull-request validation uses event identity but fetches title/body/base/head repository/state from the live GitHub API.
- [x] A stale event whose recorded head no longer matches the live PR head fails closed.
- [x] `workflow_dispatch` retains exact branch/SHA/PR identity verification and uses the same live-metadata path.
- [x] Push-to-main Agent-governance validation behavior remains unchanged.
- [x] Existing title/body/base/same-repository checks remain at least as strict as before.
- [x] The protected aggregate `Merge gate / validate` is configured to re-run on `edited` without adding manual PR-code dispatch.
- [x] Repository-policy canonical trigger updated without changing scope/aggregate job hashes.
- [ ] Full current-head Merge Gate passes.
- [ ] Metadata-only `edited` regression on an unchanged final head produces fresh successful Agent Governance and Merge Gate runs.
- [ ] Full changed-file self-review finds no safety reduction or authority expansion.
- [ ] Current-main ancestry and zero unresolved review threads/requested changes are reconfirmed.

## Excluded scope

- weakening any required check, branch protection, merge authority, repository allowlist or Code Owner boundary;
- adding `pull_request_target` or executing PR-controlled code with elevated permissions;
- changing production/deployment workflows;
- owner-funded Codex/OpenAI review;
- converting operational/manual workflows that legitimately use `workflow_dispatch` and do not exhibit the stale PR-metadata failure mode.

## Cross-repository audit

Read-only audit under `OTERYN-GOV-LIVE-PR-METADATA-20260817` covered:

- `blakinio/Oteryn-Platform`: its `agent-governance.yml` does not validate PR title/body and therefore does not have the #239 stale-body failure mode; its main CI resolves PR SHAs for checkout/classification rather than PR-message policy.
- `blakinio/Otheryn`: no `.github/workflows/agent-governance.yml` exists on `main`; repository workflow/code searches found no `EVENT_PR_BODY`, `pull_request.title`, or `pull_request.body` equivalent requiring this repair.
- `blakinio/otclient`: no `.github/workflows/agent-governance.yml` exists on `main`; repository workflow/code searches found no `EVENT_PR_BODY`, `pull_request.title`, or `pull_request.body` equivalent requiring this repair.

Result: no cross-repository write is justified by current evidence. Existing `workflow_dispatch` uses in those repositories are not being removed merely because they are manual; only the proven stale-metadata dependency is being repaired.

## Context checkpoint

```yaml
checkpoint_status: validating
last_verified_base_sha: 65b5711fc56f80c0407ce462e83cdc973535636f
current_branch: fix/governance-live-pr-metadata
current_pr: 320
current_head_sha: 3c91bdce08fcbfab2d749b5352c38a1171d6aa49
findings:
  - Agent governance live-metadata path passes exact-head validation.
  - Merge gate now differs from trusted base only by the explicit edited-capable trigger.
  - Canonical repository validator and independent merge-authority audit accept the new trigger contract.
  - No equivalent stale title/body dependency was found in the audited Oteryn-Platform, Otheryn or otclient workflows.
next_action: Let exact-head Merge Gate finish, freeze the final task head, then perform a metadata-only PR edit and require fresh Agent-governance plus aggregate Merge-gate success on that unchanged head.
```
