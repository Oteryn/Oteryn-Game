# OTV2-20261003-ci-stack-audit-approval

```yaml
task_id: OTV2-20261003-ci-stack-audit-approval
title: Preapprove explicit main qualification applicability
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/ci-stack-audit-approval-20261003
pr: 1632
issue: 1622
base_sha: 755af143d6727991f86ff1ea90df61059fad8143
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex CI repair task, sole publisher under direct owner instruction
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/merge-authority-audit.yml
  - tools/repository/test_merge_authority_applicability.py
  - tools/repository/test_agent_governance_pr_targets.py
  - docs/agents/tasks/archive/OTV2-20261003-ci-stack-audit-approval.md
public_contracts: []
depends_on: []
blocks: [OTV2-20261003-ci-stack-applicability]
external_repositories: []
```

## Outcome and evidence

PROVEN: prepared-branch native events reach main-only workflows despite their
branch filters. Pilot #1538 exact fddfd6845ec2378c52c01ea1579b3e12fa58b9e1
fails protected audit admission on its actual prepared base. Add an explicit main
job applicability guard; missing base metadata still enters existing fail-closed
admission. The protected audit still treats PR content as inert data and retains
its self-file rejection, permissions, exact head/current-main checks and no checkout.

Preapprove main-gate Git blob 6089ea21beb7b0305c51027797d603b75d9e77dd. Compared
with accepted 2a96d004f8a13d35bdaff9990ebecd76e3271198, only scope and the two
always aggregates receive explicit main guards. Metadata-edit isolated concurrency,
live head/base fences, product jobs and queue contracts are byte-preserved. Queue
860a684e5ec71f50ae899f9db36b7c07f9fca623 and governance eda4b9f3a006633328ffd48bd64c791274b50b08
pins remain unchanged. Activation belongs to the dependent task; this PR does not
modify the main gate itself.

## Validation and review

Six hosted-reachable target/admission/applicability tests PASS. Full repository
policy PASS (23 files, 58 workflows), including actual PowerShell routing
regressions. Exact future workflow blob/bytes and complete bounded diff reviewed.
Implementing agent self-review: no unresolved material findings. Separate read-only
local reviewer independently reran all six tests and policy and confirmed the audit
changes are exactly the guard and pin; this does not replace external exact-head review.

High-risk production authority/recovery qualification: NOT_APPLICABLE; this change
neither operates production state nor installs a runtime controller. CI trust-boundary
qualification remains required. Independent provider review REQUIRED on the final
freeze under bound META policy; pending through the unique programme control plane.
Native exact-head CI and review are pending at final authoring. The protected audit's
self-file error is expected; a NEW exact-commit owner exception is required before MQ.
Old #1608/#1610/#1620 exceptions are not inherited. No protections/results are changed.

## Closeout

Final SHA and freeze are recorded outside the commit on the PR/coordination issue.
Merge result: squash merge of #1632, pending review, owner exception and Merge Queue.
The archive reaches main only if this PR merges. Ownership releases after verified
terminal merge; no bulk feature-stack merge authority is claimed.
