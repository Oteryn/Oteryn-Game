# OTV2-20261007 Quest source-lowered binding plan

```yaml
task_id: OTV2-20261007-quest-source-lowered-binding-plan
title: Cover source-lowered Quest owners missing from the completion binding plan
mode: MIGRATE
status: validated
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/quest-source-lowered-binding-plan-20261007
base_sha: 66f0d4336e47df5d93bc85a53669ea1a047253c5
owner: chatgpt-quest-completion
owned_paths:
  - tools/content-schema/quest-authoring/source_lowered_binding_plan.py
  - tools/content-schema/quest-authoring/test_source_lowered_binding_plan.py
  - tools/content-schema/quest-authoring/run_checks.py
  - content/quests/missions/source-lowered-binding-plan.json
  - docs/agents/tasks/OTV2-20261007-quest-source-lowered-binding-plan.md
```

## Outcome

Generate a fail-closed routing work queue for canonical QuestState owners whose source transitions
are already `LOWERED` but intentionally do not appear in the chosen-stage completion binding
plan.

The packet must copy only existing transition keys and their retained source/request evidence.
It grants no NPC, trigger, placement or runtime authority.

## Acceptance

- derive the owner set from committed QuestState + chosen binding plan, not a manual allowlist;
- preserve exact QuestState transition/source/request data;
- route exact NPC `requested_by` evidence to the future NPC-QUEST-1 consumer lane;
- route source action/movement transition evidence to the future QUEST-TRIGGER-1 lane;
- leave every native binding null and runtime disabled;
- source and chosen binding plan owners must be disjoint and together cover all 310 current
  completion-candidate owners;
- drift-check the generated packet in the standard offline Quest suite.

## Runtime boundary

No runtime implementation. NPC-QUEST-1 and QUEST-TRIGGER-1 remain controlled children of the
accepted QUEST-GATE0 decision and require their own allocation/merged prerequisites.

## Validation

- source_lowered_binding_plan.py generation: PASS; 6 owners / 121 transitions.
- focused unittest: 4 / 4 PASS.
- source_lowered_binding_plan.py --check: PASS.
- git diff --check: PASS.
- standard Quest offline runner drift-checks this packet through run_checks.py.
- no runtime tests apply: every generated native binding is null and runtime_enabled=false.

## Exact work-queue split

- NPC-QUEST-1 candidates: 106 existing Source transitions with exact retained requested_by NPC evidence.
- QUEST-TRIGGER-1 candidates: 15 existing Source transitions (14 action, 1 movement), with source callback evidence and no invented request cause.
- completion transitions represented: 4.
- overlap with chosen completion binding plan: 0.
- total canonical completion-candidate owner coverage after combining the two plans: 310 / 310.

## Baseline repository-policy note

Repository-policy validation stops at the same pre-existing post-merge lane classifier failure as exact base main 66f0d4336e47df5d93bc85a53669ea1a047253c5: docs/architecture/note.md -> classifier-input-failure (E8002). This task does not touch that classifier or architecture path.
