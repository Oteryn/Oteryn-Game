# OTV2-20260927-npc-admission-presentation-behavior

```yaml
task_id: OTV2-20260927-npc-admission-presentation-behavior
title: NPC admission slice 2b - NPC Presentation and Behavior profiles (outfit, wander)
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/dazzling-brown-1u2xxo
issue: 162
pr: null
jira: KAN-16
base_sha: c6a23eccc285a08dcbffd411ed58246894e7d631
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01RTD1d7GsT7uFSBHg5syB4T
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-npc-admission-presentation-behavior.md
  - docs/architecture/OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1.md
  - apps/game-server/src/content/project/v2/creature.rs
  - apps/game-server/tests/content_world_project_v2_creature_admission.rs
  - apps/game-server/tests/content_world_project_v2_npc_admission.rs
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner request: bind NPCs to the same typed Presentation and Behavior profiles as monsters instead of
untyped candidate fields. NPC outfits use the monster Presentation profile unchanged; the Behavior
profile's `ProjectV2Movement` gains an optional `wander` (walk interval and radius), the one NPC fact
the monster profile lacked. Declarative profiles only; no content change.
Authority: direct owner request in this session.

## Architecture and source of truth

- `docs/architecture/OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1.md` §3–§4 and §7 slice 2b.
- `docs/architecture/OTERYN_WORLD_PROJECT_V2_CREATURE_ADMISSION_V1.md` §5 (profiles reused).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: a declarative authoring-profile extension with no runtime reader; no production
mutation, fence, session, authority or persisted-recovery evidence is touched.

## Acceptance and evidence

- `content_world_project_v2_npc_admission`: an NPC with Presentation (outfit 129, four palette slots,
  an addon) and Behavior (wander 2,000 ms, radius 2) profiles round-trips; five records lower
  (Presentation, Behavior, three Items); eleven rejected invariants including wander without walking
  and a zero wander interval.
- `content_world_project_v2_creature_admission` and `content_world_project_v2` unchanged in behaviour;
  existing documents without `wander` serialize byte-identically.

## Next action

Slice 3: writer and pilot (about 20 NPCs) into WorldProject/v2.
