# OTV2-20260915-defect-discovery-prompts

```yaml
task_id: OTV2-20260915-defect-discovery-prompts
title: Register Defect Discovery prompts and operator runbook
mode: GOVERNANCE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs-defect-discovery-prompts
pr: null
base_sha: b65f7bbaf61268e542fbd0c6008c5e1187cb1493
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: owner-requested prompt-package publication
created_at: 2026-09-15T19:00:00+02:00
updated_at: 2026-09-15T19:00:00+02:00
execution_policy: continuous_progress
owned_paths:
  - docs/agents/prompts/OTV2_DEFECT_DISCOVERY_SUPERVISOR.md
  - docs/agents/prompts/OTV2_DEFECT_DISCOVERY_P0_P3_LEAD.md
  - docs/agents/prompts/OTV2_DEFECT_DISCOVERY_TOOL_QUALIFIER.md
  - docs/agents/prompts/OTV2_DEFECT_DISCOVERY_MODULE_LEAD.md
  - docs/agents/programs/OTERYN_DEFECT_DISCOVERY_OPERATOR_RUNBOOK.md
  - docs/agents/programs/OTERYN_DEFECT_DISCOVERY_PROMPT_REGISTRATION.json
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
- [x] Pending registration manifest records all four lifecycle entries and aliases.
- [ ] Canonical `PROMPT_LIFECYCLE.json` imports the four entries.
- [ ] `prompts/README.md` indexes the aliases.
- [ ] Governance validation passes on the exact candidate.
- [ ] PR is opened to `main` and remains unmerged until normal checks/review qualify it.

## Excluded scope

No Defect Discovery implementation, workflow creation, product fix, Cargo/runtime changes, production mutation or Merge Queue change.

## Context checkpoint

```yaml
last_progress: prompt files, runbook and pending lifecycle registration published to docs-defect-discovery-prompts
status: implementing
branch: docs-defect-discovery-prompts
head_sha: pending live readback
pr: null
owner_action_required: null
blocker: canonical lifecycle/README registration and validation still pending
next_action: update canonical prompt index/lifecycle or route that bounded registration through the current governance owner
```
