# OTV2-20260816-final-executor-prompt-dag

```yaml
task_id: OTV2-20260816-final-executor-prompt-dag
title: Build canonical implementation executor prompt DAG
mode: COORDINATE
status: completed_on_merge
repository: blakinio/Oteryn-v2
issue: 313
pr: 314
branch: docs/final-executor-prompt-dag-20260816
historical_base_sha: bf2a2ae279516f62626a5d8f4dc1aeb587535c62
reconciled_main_sha: 3ed4ca602f389d5a8549e0fc19dcc688a7b7a78c
reconciliation_merge_sha: 73230ac57583869ff26776b2dce3345428b67f30
prompt_content_head_evaluated: 80e09b83c4215ff4378e8cc8e25f85dff7db4b2d
owner: Architecture Coordinator
created_at: 2026-08-16T21:35:59+02:00
completed_at: 2026-08-17
execution_budget_minutes: 120
owned_paths_released_on_merge:
  - docs/agents/evidence/OTV2-20260816-final-executor-prompt-evaluation.md
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_EXECUTOR_DAG.md
  - docs/agents/prompts/OTV2_IMPLEMENTATION_COORDINATOR.md
  - docs/agents/prompts/OTV2_IMPL_WORKSPACE_BOOTSTRAP.md
  - docs/agents/prompts/OTV2_IMPL_FOUNDATION_RUNTIME.md
  - docs/agents/prompts/OTV2_IMPL_SIMULATION.md
  - docs/agents/prompts/OTV2_IMPL_DOMAIN_CORE.md
  - docs/agents/prompts/OTV2_IMPL_DURABILITY.md
  - docs/agents/prompts/OTV2_IMPL_VSL_CONTENT.md
  - docs/agents/prompts/OTV2_IMPL_GAME_ABILITY.md
  - docs/agents/prompts/OTV2_IMPL_GAME_INTERACTION.md
  - docs/agents/prompts/OTV2_IMPL_GAME_AI.md
  - docs/agents/prompts/OTV2_IMPL_NATIVE_CLIENT.md
  - docs/agents/prompts/OTV2_IMPL_QA_E2E.md
  - docs/agents/prompts/OTV2_IMPL_VSL_MOVEMENT.md
  - docs/agents/prompts/OTV2_IMPL_VSL_COMBAT.md
  - docs/agents/prompts/OTV2_IMPL_GAME_CHANNEL.md
  - docs/agents/prompts/OTV2_CONTENT_FORMAT_SPIKE.md
  - docs/agents/prompts/OTV2_IMPL_ANALYTICS.md
  - docs/agents/prompts/README.md
  - docs/agents/tasks/archive/OTV2-20260816-final-executor-prompt-dag.md
  - docs/agents/tasks/active/OTV2-20260816-remaining-first-wave-owner-decisions.md
  - docs/agents/tasks/archive/OTV2-20260816-remaining-first-wave-owner-decisions.md
public_contracts:
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_EXECUTOR_DAG.md
cross_repository_coordination_id: OTV2-NATIVE-IMPLEMENTATION
external_repositories: []
implementation_authority_outside_live_coordinator_allocation: NONE
production_authority: NONE
```

## Outcome

Prepare and release one canonical coordinator-led implementation execution programme that converts the accepted Oteryn-v2 architecture into dependency-ordered, allocation-gated implementation lanes without allowing direct workers to invent architecture or self-authorize writes.

The normal post-release entry point is:

```text
Oteryn: implementation coordinator
```

A direct worker alias remains read-only without an active coordinator allocation naming its exact lane, base and owned paths.

## Delivered package

The package contains:

- one canonical implementation DAG;
- one implementation coordinator prompt;
- bounded prompts for Bootstrap, Foundation, Simulation, Domain Core, Durability, VSL Content, GAME-ABILITY, GAME-INTERACTION, GAME-AI, native client, QA-E2E, Movement VSL, Combat VSL, later Channel, content-format spike and later Analytics;
- prompt alias/README registration;
- formal prompt-evaluation evidence;
- consolidated first-wave lifecycle archive.

The first implementation transition remains serial Bootstrap. Later lanes are dependency/path-allocation gated. Movement is a hard first-slice predecessor of Combat. Stable workspace/registry/ID mutations remain serialized.

## Prompt evaluation evidence

Historical evaluated prompt-content head:

`80e09b83c4215ff4378e8cc8e25f85dff7db4b2d`

Formal result:

```text
17/17 execution prompts: PASS
Authority: PASS
Resolution: PASS
Ownership: PASS
Architecture: PASS
Completeness: PASS
Evidence: PASS
Validation: PASS
Autonomy: PASS
Handover: PASS
Safety: PASS
OPEN MATERIAL PROMPT FINDINGS: 0
```

Four material preparation findings were repaired before the final evaluation:

1. hidden accepted-but-unimplemented SIM/Domain/Ability/Interaction/AI dependencies;
2. missing hard Movement → Combat integration order;
3. implicit worker lifecycle/budget/handover requirements;
4. implicit trusted-source/evidence/unmerged-sibling rules.

## Stage-C prerequisite closeout

The historical Stage-C blocker is terminally resolved:

- PR #311 exact final head `c5d9f839abd8998d42f4f37b203882f03bb51ce0` received genuinely independent review `4949049662` with zero material findings;
- #311 squash-merged as `e0ea9ef87c01dec720a22e8df6d54bfd669cb62c`;
- governance/status closeout PR #318 squash-merged as `a6a5180d98cf7791e40d9e1d08b25a5c8b4eff96`;
- issue #310 closed completed;
- maintained status/register surfaces record `VSL-MOVE-01`, `VSL-COMBAT-01`, `VSL-CONTENT-01` as `ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED`.

No Stage-C owner decision remains open.

## Live-main reconciliation

Before final #314 validation, the branch was reconciled against exact:

`main@3ed4ca602f389d5a8549e0fc19dcc688a7b7a78c`

The reconciliation commit:

`73230ac57583869ff26776b2dce3345428b67f30`

has both the prepared #314 branch and that live main as parents. Its tree preserves current-main governance and overlays only the #314 package paths.

Verified current baseline after reconciliation:

- workspace remains the pre-native 19-member shape;
- `workspace-boundaries.toml` still requires the atomic Bootstrap transition and forbids premature `protocol-oteryn` / transport / game-session / game-server / persistence production packages;
- FND-02 gameplay `capabilities`, `command_types` and `state_domains` remain empty;
- game-event `event_types` remain empty;
- Stage-C architecture is accepted/lifecycle-closed;
- permanent World Project/World Bundle encoding remains undecided behind the DUR-04 evidence spike and later owner decision;
- Reference parity remains evidence-gated;
- `PROD-ENTITLEMENTS-01` remains excluded until separately accepted;
- current root governance, including central Spark pre-review policy, remains authoritative.

## Re-evaluation after reconciliation

The 17 execution prompt bodies were not changed by reconciliation. Only package evidence/DAG/lifecycle wording was updated to remove the historical Stage-C release blocker and describe release semantics.

Current `PROMPTING_STANDARD.md` and `PROMPT_EVAL_STANDARD.md` retain the same required prompt structure and ten evaluation gates used by the package.

```text
PROMPT_CONTENT_DELTA_AFTER_RECONCILIATION: NONE
LIVE_PREREQUISITE_RECONCILIATION: PASS
FULL_PROMPT_RE-EVALUATION_REQUIRED: NO
PROMPT_QUALITY: PASS
OPEN MATERIAL PROMPT FINDINGS: 0
```

## Review classification

Mandatory full-diff self-review and exact-head repository CI are required on the final #314 head.

A separate independent review is not required for this prompt-package delivery under current root risk triggers because the final delta is prompt/programme/lifecycle documentation only, changes no runtime/protocol/persistence/value semantics, weakens no governance gate and grants no production/cross-repository/owner-funded-AI authority.

If an external central Spark pre-review posts a material P0/P1 finding before merge, it is treated as a material review finding and must be resolved under current governance. This task does not directly invoke Spark/Codex/OpenAI.

## Release semantics

PR #314 becomes canonical only after lawful merge to `main` on an unchanged exact head with all required gates satisfied.

That merge releases the **implementation coordinator programme**. It does not start implementation work.

It does not:

- create a Bootstrap allocation;
- start any worker;
- authorize direct worker writes;
- authorize production/protected/live-data actions;
- authorize Platform or external-repository mutation;
- authorize owner-funded AI use;
- choose the permanent content format;
- establish Reference parity;
- accept entitlement implementation.

Implementation begins only after a later explicit invocation of the released coordinator programme, which must inspect live main and create bounded allocations under current governance.

## Final validation contract

Before #314 merge:

- full final diff self-review must report zero open material findings;
- Agent governance / repository policy / Merge authority / applicable semantic audit / `Merge gate / validate` must pass on the exact final head;
- unresolved review threads must be zero;
- no `REQUEST_CHANGES` may remain;
- the branch must not be behind live `main`;
- any material central Spark finding, if one appears, must be resolved;
- final merge must use the exact frozen head SHA.

Because a commit cannot contain its own final SHA or merge SHA, those immutable terminal identifiers remain in PR/check/merge evidence rather than creating a self-referential follow-up commit.

## Terminal state on lawful merge

```text
PROMPT_QUALITY: PASS
EXECUTOR_PROGRAMME: RELEASED
IMPLEMENTATION_WORKERS_STARTED: NO
IMPLEMENTATION_AUTHORITY_OUTSIDE_LIVE_COORDINATOR_ALLOCATION: NONE
PRODUCTION_AUTHORITY: NONE
```
