# OTV2-20261003-ci-stack-applicability

```yaml
task_id: OTV2-20261003-ci-stack-applicability
title: Qualify native prepared-branch preflight explicitly
mode: REPAIR
status: blocked
repository: Oteryn/Oteryn-Game
base_branch: codex/ci-stack-audit-approval-20261003
branch: codex/ci-stack-applicability-20261003
pr: 1634
issue: 1622
base_sha: 4bac053d3c712ca50ee649df7b976d1d2eea7217
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
Separate local reviewer found the retarget gap; META author self-review is additional
verification, not independent external review. Required provider review remains pending
until the final candidate is frozen. No paid provider trigger has been sent by this worker.

High-risk production authority/recovery qualification: NOT_APPLICABLE; no production
state/controller is operated. CI trust-boundary independent review remains REQUIRED.
Current native draft checks are authoring feedback, not final-head qualification.

## Blocker and next action

AUTHORING, not frozen. Wait for #1632 qualification, exact owner self-file exception and
terminal Merge Queue integration. Then normally merge its accepted main result into
this branch and retarget to main. Squash integration does not retain parent ancestry:
verify the remote main-relative changed-file set excludes the already-accepted audit
file before freeze. Move this record to archive with actual PR1634 in the final authoring
commit, rerun affected validation, freeze returned exact remote head, and hand off one
required provider review through the unique active control plane. Full main native CI
and Merge Queue remain mandatory; stack preflight grants no merge authority.

Merge result: pending. No ownership release before terminal protected-main readback.
Broad feature refresh and writes to active programme/held data branches stay stopped.
