# OTV2-20261002-npc-bulk-workbench-plan

```yaml
task_id: OTV2-20261002-npc-bulk-workbench-plan
title: Save owner-approved bulk NPC plan and reusable authoring workbench
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/npc-source-audit-r4
issue: 162
pr: 1433
jira: KAN-16
base_sha: cbde12a953bd41e1087600f2b36dec97cde1f9e6
owner: codex-root-npc-bulk-plan
created_at: 2026-10-02
updated_at: 2026-10-02
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/npc-authoring/bulk_workbench.py
  - docs/agents/evidence/OTV2-20261002-npc-bulk-workbench/PLAN.md
  - docs/agents/tasks/archive/OTV2-20261002-npc-bulk-workbench-plan.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
```

The owner explicitly prefers maximally populated, flagged approximate NPCs over requiring100% real-Tibia fidelity, with gradual refinements. The plan records this policy, differentiates field quality and basic runtime readiness, and uses the existing Canary/Crystal/wiki tooling with three45/45/43 batches. The thin authoring workbench prepares133 pending targets and runs existing offline tests in one external workspace. It adds no NPC records or runtime activation and clearly reports zero loaded NPCs. Native talk/trade/travel and development-map dependencies are explicit; gameplay testing must use real Oteryn interfaces.

Original source-audit data are already preserved in draft#1433 at predecessorcbde. This follow-up explicitly returns to AUTHORING and publishes one bounded three-file successor through the proven guarded normal-Git route; no original content/runtime/contract modifications. Exact freeze, validation, recovery and remote PR readback are retained externally. Formal paid review dispatch, protected CI, Jira and Merge Queue remain with controlplane#162.

Actual authoring validation:133 unique pending targets in45/45/43,0 runtime loaded; existing NPC authoring/schema smoke27 tests PASS; duplicateprepare preserves targetbytes; repository-local output rejects before writing; py_compile and governance PASS. No NPC/import/activation count is advanced by this helper.
