# OTV2-20260924-ops-node-boot

```yaml
task_id: OTV2-20260924-ops-node-boot
title: First startable GameNode under OPS-NODE-BOOT-01
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/zen-dirac-mxruho
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
  - apps/game-server/src/gameplay_transport/
  - apps/game-server/src/native_admission_source/descriptor.rs
  - apps/game-server/src/lib.rs
  - apps/game-server/src/main.rs
  - apps/game-server/src/bin/
  - apps/game-server/src/node/
  - apps/game-server/Cargo.toml
  - Cargo.toml
  - Cargo.lock
  - .github/workflows/gameplay-server-seam.yml
  - tools/qualification/wp5_s3b/run.sh
  - docs/agents/tasks/active/OTV2-20260924-ops-node-boot.md
public_contracts:
  - OPS-NODE-BOOT-01
depends_on:
  - pr:830
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

- **Governing decision:** `OPS-NODE-BOOT-01` (`docs/architecture/reviews/OTERYN_GAME_NODE_BOOT_COMPOSITION_DECISION_2026-09-24.md`, PR #830). The owner accepted D1–D6 on 2026-09-24.
- **Delivery in two bounded PRs:**
  - **PR-A (D4):** the fresh-admission seam fetches account security and fresh signing trust on demand for each attempt into S2 custody, before composition. The qualification lets pre-seeded evidence age past the five-second bound, so a successful admission proves the on-demand fetch.
  - **PR-B (D1–D3, D5, D6):** the configuration, the `oteryn-game-ops` control-plane binary, the serving-node boot sequence, the control socket, maintenance and shutdown, plus §4 physical qualification with the shipped binaries.
- **Excluded:** `OPS-CHANNEL-01`, resume/reconnect (#822), Platform changes, production endpoints and credentials.
