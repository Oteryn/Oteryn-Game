> Lifecycle closeout: **ARCHIVED / OWNERSHIP RELEASED**. Delivery PR #627 merged as `88b04c9441b38e7353e0f66be50ba8d13fdd6cf2`. Any active/checkpoint language below is historical provenance only; live GitHub and protected current state supersede it.

# OTV2-20260915-defect-discovery-prompts

```yaml
task_id: OTV2-20260915-defect-discovery-prompts
title: Register Defect Discovery prompts and operator runbook
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs-defect-discovery-prompts
pr: 627
base_sha: b65f7bbaf61268e542fbd0c6008c5e1187cb1493
head_sha: pending exact readback
final_head_sha: null
final_head_frozen_at: null
owner: owner-requested prompt-package publication
created_at: 2026-09-15T19:00:00+02:00
updated_at: 2026-09-15T19:50:00+02:00
execution_policy: continuous_progress
owned_paths:
  - docs/agents/prompts/OTV2_DEFECT_DISCOVERY_SUPERVISOR.md
  - docs/agents/prompts/OTV2_DEFECT_DISCOVERY_P0_P3_LEAD.md
  - docs/agents/prompts/OTV2_DEFECT_DISCOVERY_TOOL_QUALIFIER.md
  - docs/agents/prompts/OTV2_DEFECT_DISCOVERY_MODULE_LEAD.md
  - docs/agents/programs/OTERYN_DEFECT_DISCOVERY_OPERATOR_RUNBOOK.md
  - docs/agents/PROMPT_LIFECYCLE.json
  - docs/agents/prompts/README.md
  - docs/agents/tasks/active/OTV2-20260915-defect-discovery-prompts.md
public_contracts: []
depends_on:
  - "#162 comments 5684916133/5684920732/5684926851"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Persist the owner-approved `OTERYN_DEFECT_DISCOVERY_V1` reusable prompt family, aliases and operator instructions without implementing Defect Discovery itself.

## Source of truth

- PROVEN: #162 contains the architecture, proof package and implementation sequence.
- PROVEN: ADR-0007 remains E2E authority.
- PROVEN: prompt authoring follows the bound META 3.1 Prompting Standard and Prompt Eval Standard.

## Acceptance criteria

- [x] Four prompt files exist on one docs-only branch.
- [x] Operator runbook documents launch order and intended manual execution.
- [x] Canonical `PROMPT_LIFECYCLE.json` registers all four prompts as reusable version `1.0`.
- [x] `prompts/README.md` indexes all four aliases and module alias pattern.
- [x] Draft PR #627 is open to `main`.
- [ ] Governance validation passes on the exact candidate.
- [ ] Applicable prompt/lifecycle semantic validation passes on the exact candidate.
- [ ] Final changed-file review confirms documentation/governance-only scope.
- [ ] PR remains unmerged until normal checks/review qualify it.

## Excluded scope

No Defect Discovery implementation, workflow creation, product fix, Cargo/runtime changes, production mutation or Merge Queue change.

## Validation

### Focused

- `python tools/agents/validate_governance.py` — pending hosted exact-head result.

### Component/integration

- `NOT_APPLICABLE`: prompt/governance documentation only.

### E2E

- `NOT_APPLICABLE`: no runtime behavior changes.

### Exact-head CI

- final head: pending live readback
- trigger source: PR push
- result: pending

## Context checkpoint

```yaml
last_progress: canonical lifecycle and prompt index updated; staging manifest removed
status: validating
branch: docs-defect-discovery-prompts
head_sha: pending live readback
pr: 627
owner_action_required: null
blocker: exact-head governance and semantic validation not yet terminal
next_action: read exact PR head and current checks, then repair only proven prompt/governance findings
```
