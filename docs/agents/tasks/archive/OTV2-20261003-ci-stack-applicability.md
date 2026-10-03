# OTV2-20261003-ci-stack-applicability

```yaml
task_id: OTV2-20261003-ci-stack-applicability
title: Qualify native prepared-branch preflight explicitly
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/ci-stack-applicability-20261003
pr: 1634
issue: 1622
base_sha: bec95a921e9a2051b707d597dcf91334f176980f
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex CI repair task, sole publisher under direct owner instruction
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/architecture-semantic-audit.yml
  - .github/workflows/merge-gate.yml
  - tools/agents/validate_inherited_prompt_policy.py
  - tools/agents/tests/test_meta_agent_policy_adoption.py
  - tools/repository/test_agent_governance_pr_targets.py
  - tools/repository/test_main_job_applicability.py
  - tools/repository/test_classify_pr_test_lanes.py
  - tools/repository/validate_repository_policy_core.py
  - docs/agents/tasks/active/OTV2-20261003-ci-stack-applicability.md
  - docs/agents/tasks/archive/OTV2-20261003-ci-stack-applicability.md
public_contracts: []
depends_on: [OTV2-20261003-ci-stack-audit-approval]
blocks: []
external_repositories: []
```

## Outcome and evidence

PROVEN: #1538 fddfd6845ec2378c52c01ea1579b3e12fa58b9e1 passes exact native
stack admission but fails META task discovery, main gate and architecture admission.
Main-only qualification jobs now explicitly exclude prepared-base events while missing
base metadata still reaches existing fail-closed admission. The complete main gate
matches separately preapproved blob 6089ea21beb7b0305c51027797d603b75d9e77dd;
accepted metadata-edit concurrency and live head/base fences are preserved. Queue
workflow, full product lanes, PostgreSQL scenarios and canonical main gate stay intact.

Only native pull_request task discovery binds its validated event base ref. Both live
repositories, open state, head SHA, base SHA/ref and exact changed-file count must stay
stable across bounded enumeration/readback. Direct calls/dispatch retain main scope.
META validation and protected binding authentication are unchanged.

Architecture now qualifies an already-ready stack retargeted to main by edited. Pure
metadata edits skip semantic work and use separate run-id concurrency groups; they
cannot cancel a current actual audit. Source changes/base retargets replace stale runs.

## Validation and review

Focused RED/GREEN proves missing retarget event and concurrency cancellation defects
before their repair. Final local applicability10 and hosted admission16 PASS; full
agent/governance/lifecycle43 PASS. Repository policy, metadata-edit/consumer-routing,
actual PowerShell PR/queue PG/SIM failure canaries and semantic dispatch37 PASS.
Implementer whole-diff self-review accepted and repaired both related P1 findings.
An independent whole-diff local reviewer found no new P0/P1 on the repaired
2ae0e03b candidate. META author self-review is additional verification, not the
required independent provider review. Required provider review remains pending
until the final candidate is frozen. No paid provider trigger has been sent by this worker.

High-risk production authority/recovery qualification: NOT_APPLICABLE; no production
state/controller is operated. CI trust-boundary independent review remains REQUIRED.
Native prepared-base preflight passed on 2ae0e03b; it is preliminary evidence, not final main qualification.

## Final authoring and closeout

Parent #1632 has terminal protected-main integration. Normal accepted-main merge
aligns the squash ancestry; the remote main-relative delta must exclude its already-
accepted audit file. The independent main pin approval is not included as a second
self-file mutation. This task's authored gate remains exact blob
6089ea21beb7b0305c51027797d603b75d9e77dd. Final source review is limited to the
bounded activation delta plus this truthful task archive.

This record is archived in the final authoring commit before freeze, with actual
PR1634. Exact final SHA/freeze are recorded outside Git. Independent provider review
REQUIRED on that head through the unique active control plane after live same-head
de-duplication; pending at authoring. Native full-main CI, green protected audit and
Merge Queue are mandatory. The parent exception is NOT used by this activation.

Merge result: squash merge of #1634, pending review/qualification/MQ at authoring.
The archive reaches main only if this PR merges. Ownership releases after verified
terminal protected-main readback. No bulk feature refresh/held lane mutation is
claimed; only the two owner-requested CI PRs are being closed out here.
