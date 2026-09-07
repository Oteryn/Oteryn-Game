# OTV2-20260907-final-governance-cleanup

```yaml
task_id: OTV2-20260907-final-governance-cleanup
title: Remove retired review adapter and terminal prompt dispatch
mode: GOVERNANCE
status: implementing
repository: Oteryn/Oteryn-Game
issue: 388
base_branch: main
branch: governance/final-cleanup-388
pr: null
base_sha: a793457cf3001df37109acb2c4b4a772b53db97a
owner: Astra Final Cleanup coordinator
created_at: 2026-09-07T16:30:37Z
updated_at: 2026-09-07T16:30:37Z
execution_policy: continuous_progress
owned_paths:
  - tools/agents/validate_governance.py
  - tools/agents/validate_governance_core.py
  - tools/agents/tests/test_validate_governance_lifecycle.py
  - tools/agents/tests/test_governance_lifecycle_discovery.py
  - docs/agents/CODEX_REVIEW_POLICY.json
  - docs/agents/PROMPT_LIFECYCLE.json
  - docs/agents/prompts/README.md
  - docs/agents/prompts/OTV2_CLOSE_NEXT_WAVE_BLOCKERS.md
  - docs/agents/prompts/OTV2_IMPL_SERVER_SEAM.md
  - docs/agents/prompts/OTV2_PREP_SERVER_SEAM.md
  - docs/agents/prompts/OTV2_SOL_EXECUTION_ARCHITECTURE_CONTINUATION.md
  - docs/agents/prompts/OTV2_SOL_POST_VSL_EXPANSION.md
  - docs/agents/prompts/OTV2_NEXT_WAVE_PARALLEL_PREPARATION.md
  - docs/agents/prompts/OTV2_PREP_DURABILITY_TOPOLOGY.md
  - docs/agents/tasks/active/OTV2-20260907-final-governance-cleanup.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
```

## Outcome and authority

Issue #388 owns this bounded implementation; Oteryn/Oteryn#176 owns organization closeout. The existing META v3 binding remains unchanged. Remove only the intentionally retired review controller, its runtime adapter and obsolete dispatch/framework requirements. Preserve real lifecycle/domain validation and historical specifications.

## Acceptance and validation

Run governance and lifecycle discovery including the failing-assertion canary, inherited META policy tests and changed-path repository checks. Runtime E2E and production/recovery qualification are NOT_APPLICABLE: no runtime, persisted data, authorization effect, deployment or gate-authority change is permitted. Exact-head GitHub checks, independent requested Sol review, normal protected Merge Queue and main readback remain required.

## Excluded scope

No product implementation, shared registry/Cargo/contract mutations, other task leases, workflows, required gates, maintenance controls, credentials or production. Active independent #364 remediation and W6 work retain their owners.

## Context checkpoint

Confirmed dead validator and terminal coordinator #131/#152; implementation and tests are pending. Explicit Sol child execution is unavailable in this session; no independent review has occurred. Next action: consolidate the canonical governance entry point and prove retained positive/negative checks.
