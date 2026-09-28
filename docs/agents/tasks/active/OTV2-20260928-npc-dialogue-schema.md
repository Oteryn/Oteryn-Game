# OTV2-20260928-npc-dialogue-schema

```yaml
task_id: OTV2-20260928-npc-dialogue-schema
title: NPC dialogue - typed WorldProject/v2 Dialogue declaration (greet, farewell, keyword tree, voices)
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/dazzling-brown-1u2xxo
issue: 162
pr: 1062
jira: KAN-16
base_sha: 45b6cc73d8dbd2988ec0153cfa5ad03318367d51
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01RTD1d7GsT7uFSBHg5syB4T
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260928-npc-dialogue-schema.md
  - docs/agents/tasks/active/OTV2-20260927-npc-admission-wiki-completion.md
  - docs/agents/tasks/archive/OTV2-20260927-npc-admission-wiki-completion.md
  - apps/game-server/src/content/project/v2.rs
  - apps/game-server/src/content/project/v2/creature.rs
  - apps/game-server/tests/content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_v2_npc_admission.rs
  - docs/architecture/OTERYN_NPC_AUTHORING_SCHEMA_V1.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

The WorldProject/v2 `Dialogue` declaration, until now an untyped placeholder, gains a typed, validated,
declarative shape for NPC conversations. It carries `greet`, `farewell` and `walkaway` messages, a nested
keyword tree and ambient `voices`. Each keyword node has a slug `key`, sorted lowercase `triggers`, a
`reply` and child keywords. The node's key must be unique among its siblings, and the tree is at most
eight levels deep.

The change mirrors the typed Service offers and routes. The new fields default to empty, so existing
documents stay byte-identical. Nothing runs: there is no runtime reader, and no content is admitted in
this slice.

Decision D9 in `OTERYN_NPC_AUTHORING_SCHEMA_V1.md` replaces D5: Tibia Global NPC text is admitted 1:1 as
reference data, as `LICENSE-ASSETS.md` allows after #1050. The Dialogue fields therefore hold plain text,
not text references.

Authority: owner request in this session ("kontynuuj 1,2,3 i 4", point 3).

## Acceptance and evidence

- `content_world_project_v2_npc_admission`: an NPC with a linked Dialogue round-trips.
- Keywords, triggers and voices are canonicalized.
- Twenty-one broken invariants are rejected, and an eight-level keyword chain is admitted, and unknown fields fail closed.
- The full `oteryn-game-server` test suite, `fmt` and `clippy -D warnings` pass.
- `content_world_project_repository` is unchanged.
