# OTV2-20260927-creature-admission-design

```yaml
task_id: OTV2-20260927-creature-admission-design
title: WorldProject/v2 creature admission route for the resolved Canary monsters
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: 981
jira: KAN-16
base_sha: 68d0ae0005a8bed45f2427dd7a0d762eeeaf7239
head_sha: a21d89e7d68b1471b0738a3af9896d2f5f2b5004
final_head_sha: a21d89e7d68b1471b0738a3af9896d2f5f2b5004
final_head_frozen_at: null
owner: claude/nice-edison-h9aqh0
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-creature-admission-design.md
  - docs/agents/tasks/archive/OTV2-20260927-encounter-slice-8.md
  - docs/architecture/OTERYN_WORLD_PROJECT_V2_CREATURE_ADMISSION_V1.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

The owner chose on 2026-09-27 ("druga droga") to admit the fully resolved Canary monsters into
WorldProject/v2 after extending v2 with the fields the authoring format carries. This task records
the admission route: wave A scope, Oteryn identities and the Item crosswalk, the executable Reference
records, the declarative profiles and the slices that implement it.
Authority: direct owner request in this session; runtime behaviour stays unallocated.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: architecture decision document only; no production mutation, fence, session,
authority or persisted-recovery evidence is touched.

## Acceptance and evidence

- `OTERYN_WORLD_PROJECT_V2_CREATURE_ADMISSION_V1.md` (CANDIDATE): wave A = 1,332 of the 1,490 resolved
  monsters (142 wait for an Encounter runtime, 59 for 68 Item registrations); identity scheme; Item
  crosswalk through the protected Item identity map (2,655 of 2,723 referenced ids registered); Reference
  records the current linker accepts; declarative profiles; five implementation slices.

## Completion

Merged as PR #981 (`12d96e0e6dff227145267deadddb0b085806e293`) from final head `a21d89e`; required checks passed on that head.
Owner released.
