# OTV2-20260924-ops-node-boot-b1

```yaml
task_id: OTV2-20260924-ops-node-boot-b1
title: OPS-NODE-BOOT-01 durability control plane (roles, grants, issuances)
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/zen-dirac-mxruho-pr-b1
issue: 832
pr: null
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude
created_at: 2026-09-24T00:00:00Z
updated_at: 2026-09-24T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/migrations/0006_node_boot_control_plane.sql
  - apps/game-server/src/durability/
  - apps/game-server/src/gameplay_transport/qualification.rs
  - apps/game-server/tests/runtime_scope_assignment_postgres.rs
  - apps/game-server/tests/native_admission_source_postgres.rs
  - apps/game-server/tests/character_authority_postgres.rs
  - apps/game-server/tests/wp5_s3b_composition.rs
  - docs/agents/tasks/active/OTV2-20260924-ops-node-boot-b1.md
public_contracts:
  - OPS-NODE-BOOT-01
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

- **Governing decision:** `OPS-NODE-BOOT-01` (`docs/architecture/reviews/OTERYN_GAME_NODE_BOOT_COMPOSITION_DECISION_2026-09-24.md`, merged in #830). The owner accepted D1–D6 on 2026-09-24.
- **Slice PR-B1 (database control plane for D2/D3):**
  - migration `0006`: the `oteryn_game_runtime` and `oteryn_game_control` group roles and their grants; the owner-written exact-scope grant table, enforced for the authenticated session role; recorded Platform descriptor issuances; launch-authorization revocation; the security-definer audit expiry;
  - exact-replay launch-authorization issuance; S2 initialization and descriptor registration bound to the recorded issuance; a read of the stored S2 registration;
  - the SEAM qualification runs the node on a runtime login and operator actions on a control login.
- **Later slices:** PR-B2 (`oteryn-game-ops`, configuration, file checks) and PR-B3 (the serving boot sequence, control socket and §4 qualification).
- **Excluded:** `OPS-CHANNEL-01`, resume/reconnect (#822), Platform changes, production endpoints and credentials.
