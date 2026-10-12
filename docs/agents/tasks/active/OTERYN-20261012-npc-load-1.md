# OTERYN-20261012-npc-load-1

```yaml
task_id: NPC-LOAD-1
title: "NPC-LOAD-1: the World Bundle's NPC placements become NPC actors at Channel start"
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/npc-load-1-20261012
pr: null
issue: 1622
base_sha: e128fd6cc
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-01CwP6d84eCPvpgoEuyci8Tx
control_plane: claude-code-session-01CwP6d84eCPvpgoEuyci8Tx
coordination: "#1622"
created_at: 2026-10-12
updated_at: 2026-10-12
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/node/serve/bundle_npcs.rs
  - apps/game-server/src/node/serve.rs
  - apps/game-server/src/foundation/mod.rs
  - apps/game-server/src/foundation/npc_actor.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/world_spatial.rs
  - docs/agents/tasks/active/OTERYN-20261012-npc-load-1.md
  - docs/agents/tasks/archive/OTERYN-20261012-npc-load-1.md
public_contracts: []
external_repositories: []
```

## Outcome

A `[world_bundle]` World boots the bundle's format v4 NPC placements as NPC actors (NPC-ACTOR-1
of NPC-PLACE-1 §7). Work in progress; the archived record carries the closeout.
