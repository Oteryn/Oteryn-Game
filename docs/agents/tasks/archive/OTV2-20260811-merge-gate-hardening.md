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
repository_configuration_run: 31729712428
repository_configuration_result: PASS
owner: released_after_closeout
completed_at: 2026-08-13T20:17:00+02:00
owner_funded_ai_used_current_generation: false
external_repositories: []
```

## Outcome

The repository merge-authority hardening programme is complete.

The final model is:

- one stable required exact-head status: `Merge gate / validate`;
- aggregate governance, Dependency Review and CodeQL, with Rust policy/Linux/Windows/supply-chain checks when Rust/workspace-sensitive paths change;
- fail-closed changed-file enumeration above GitHub's 3,000-file API cap and on metadata/enumeration mismatch;
- rename-source and `.cargo/**` classification retained;
- no `workflow_dispatch` execution of pull-request code for merge-gate recovery;
- `Protect main` active, strict/up-to-date, squash-only, zero bypass actors;
- for this public repository, merge-authority control-plane paths are protected through required Code Owner review using a deliberately narrow base-branch `.github/CODEOWNERS` map;
- stale Code Owner approvals are dismissed after reviewable pushes;
- the unsupported public push-ruleset is absent;
- a latent no-bypass path-restriction push-ruleset policy remains only for future private/internal repository visibility.

## Delivery history

PR #238 delivered the aggregate merge gate after the original PR #162 and replacement PR #237 were superseded during bounded repair/freshness handling. PR #238 squash-merged as `e8f9108014d12043535b56d8fc25fcb0e3390a51`.

Its required post-merge Repository configuration run `31726698230` exposed a real GitHub platform incompatibility: the attempted push ruleset was not valid for the public-repository design. The failure was treated as a product/platform finding rather than retried or bypassed.

PR #241 repaired the model for the actual public repository. Exact final delivery head `87d113271ce42b8a1369fb083caff18f4980775c` passed:

- full-diff self-review: **PASS**, open material findings `0`;
- Agent Governance run `31728395200`: **PASS**;
- deterministic independent Merge Authority Audit run `31728395191`: **PASS**;
- aggregate Merge Gate run `31728395207`: **PASS**, including all applicable sub-gates;
- unresolved review threads: `0`;
- current-main ancestry before merge: **PASS**.

PR #241 squash-merged as `3a8add69e76221597f2973c9873521d82fb83568`.

## Post-merge integration proof

Repository configuration run `31729712428` on merge SHA `3a8add69e76221597f2973c9873521d82fb83568`: **PASS**.

Live GitHub ruleset readback after that run proved:

- `Protect main` is active;
- target is `branch` and default branch is selected;
- bypass actors: `[]` and `current_user_can_bypass: never`;
- only squash merge is allowed;
- `required_status_checks` contains exactly `Merge gate / validate` with strict mode enabled;
- `require_code_owner_review: true`;
- `dismiss_stale_reviews_on_push: true`;
- `required_approving_review_count: 0` for ordinary non-owned paths;
- `required_review_thread_resolution: true`;
- no `Protect repository control plane` push ruleset exists on the public repository.

The narrow CODEOWNERS map owns `.github/CODEOWNERS`, `.github/workflows/`, `.github/repository-policy.json`, and `tools/repository/` by `@blakinio`; ordinary architecture/runtime/content paths are not globally owner-gated.

## Safety and scope

No gameplay/client/server runtime behavior, protocol semantics, persistence/content semantics, production deployment, protected environment action, secret expansion, cross-repository write, repository visibility change or owner-funded Codex/OpenAI invocation was performed by this task generation.

## Closeout

All acceptance criteria are satisfied. Ownership of the task's advisory paths is released after this archive closeout merges.

```yaml
status: completed
blocker: null
owner_action_required: false
next_action: Resume normal Oteryn-v2 architecture/programme work on the hardened repository-governance baseline.
```
