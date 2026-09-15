# OTV2-20260807-fnd-id01-minimum-scope

```yaml
task_id: OTV2-20260807-fnd-id01-minimum-scope
title: Freeze the minimum scope of the final FND-ID-01 contract
mode: CONTRACT
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260807-fnd-id01-minimum-scope
pr: 80
base_sha: 7194f510a09bb5aba6ceab94841f9a48d95e57da
final_head_sha: 42a51af38aefacff56bad78ad26bcb71700a5a1d
merge_commit: 96760a99ce09bf20417a4a9d6dc1961785156b6c
owner: ChatGPT architecture coordinator
created_at: 2026-08-07T19:57:00+02:00
completed_at: 2026-08-07T20:04:03+02:00
execution_budget_minutes: 60
owned_paths: []
public_contracts:
  - docs/architecture/FND-ID-01_MINIMUM_CROSS_BOUNDARY_SCOPE_OWNER_BASELINE.md
depends_on:
  - ADR-0001 through ADR-0011
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
  - docs/architecture/FND-ID-01_OWNER_ACCEPTED_BASELINE.md
  - docs/architecture/UUIDV7_DURABLE_IDENTITY_OWNER_BASELINE.md
  - docs/architecture/FND-ID-01_OWNER_DECISION_CHECKPOINT_2026-08-07.md
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

The owner-accepted rule is now canonical on `main`: the final `FND-ID-01` contract is a **minimum cross-boundary foundation identifier contract**, not an exhaustive catalogue of every identifier that Oteryn may ever use.

The accepted baseline is:

- `docs/architecture/FND-ID-01_MINIMUM_CROSS_BOUNDARY_SCOPE_OWNER_BASELINE.md`.

## Accepted architecture result

`FND-ID-01` must freeze only the identity semantics required to make `FND-02`, `FND-03` and `FND-04` unambiguous.

The minimum catalogue includes the already foundation-relevant identity concepts:

- `AccountId`;
- `CharacterId`;
- `WorldId`;
- `ChannelId`;
- `NodeId`;
- `InstanceId`;
- `PartyId`;
- `GameSessionId`.

Additional identifiers enter the final foundation catalogue only when a downstream foundation contract cannot be safely defined without their semantic identity/class/owner/scope first and `FND-ID-01` can define that identity without taking over the later domain's behavior contract.

Detailed item/economy, analytics/audit, quest/event, content and operations identifier catalogues remain with their owning later gates.

## Validation

### Changed-file review

- result: `PASS`
- changed scope on delivery PR #80: exactly two declared documentation paths.

### Component/integration

- result: `NOT_APPLICABLE`
- reason: architecture documentation only; no executable runtime behavior changed.

### E2E

- result: `NOT_APPLICABLE`
- reason: architecture documentation only; no executable user/runtime journey changed.

### Exact-head CI

Exact delivery head:

- `42a51af38aefacff56bad78ad26bcb71700a5a1d`

Required runs:

- Agent governance run `31205048984`: `PASS`;
- Dependency review run `31205049408`: `PASS`;
- CodeQL run `31205049010`: `PASS`.

### Independent audit

- exact head: `42a51af38aefacff56bad78ad26bcb71700a5a1d`
- PR review ID: `4885388192`
- result: `PASS`
- material findings: `0`

## Merge and closeout

- delivery PR: #80
- merge method: squash
- merge result: `96760a99ce09bf20417a4a9d6dc1961785156b6c`
- unresolved review threads at merge: `0`
- ownership: released
- architecture decision: canonical on `main`

## Context checkpoint

```yaml
last_progress: PR #80 passed exact-head audit and required checks and was squash-merged; task ownership is released and this record is archived.
status: completed
branch: docs/OTV2-20260807-fnd-id01-minimum-scope
pr: 80
final_head_sha: 42a51af38aefacff56bad78ad26bcb71700a5a1d
merge_commit: 96760a99ce09bf20417a4a9d6dc1961785156b6c
ci_run_ids:
  - 31205048984
  - 31205049408
  - 31205049010
runner_assignment_state: completed
owner_action_required: null
blocker: null
next_action: Create a separate bounded complete FND-ID-01 contract task that consumes this minimum-scope baseline and all earlier owner-accepted identifier baselines.
```
