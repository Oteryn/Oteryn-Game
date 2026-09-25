# OTV2-20260924-ops-node-boot-b3

```yaml
task_id: OTV2-20260924-ops-node-boot-b3
title: OPS-NODE-BOOT-01 serving boot sequence and control socket
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/zen-dirac-mxruho-pr-b3
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
  - Cargo.lock
  - apps/game-server/Cargo.toml
  - apps/game-server/src/main.rs
  - apps/game-server/src/node/
  - apps/game-server/src/bin/oteryn-game-ops.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/fresh_evidence.rs
  - apps/game-server/src/gameplay_transport/qualification.rs
  - tools/qualification/node_boot/
  - .github/workflows/node-boot-qualification.yml
  - docs/agents/tasks/active/OTV2-20260924-ops-node-boot-b3.md
public_contracts:
  - OPS-NODE-BOOT-01
depends_on:
  - pr:843
  - pr:845
  - pr:849
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

- **Governing decision:** `OPS-NODE-BOOT-01` (merged in #830; owner accepted D1–D6 on 2026-09-24).
- **Slice PR-B3 (D3, D5, D6):** `oteryn-game-server serve --config <path>`:
  - loads and checks every input before any socket is bound;
  - connects the TLS-verified durability root and maintains it;
  - registers one `NodeId` with exact replay through ambiguity;
  - establishes S2 custody from the recorded issuance, confirms the descriptor and reconciles retained publication slots;
  - opens Character authority and waits a bounded time for the operator assignment;
  - binds the gameplay listener and the uid-0-only control socket, then publishes readiness by CAS with the declared D5 revisions;
  - serves gameplay, the Character bootstrap control socket and audit expiry;
  - on a signal, withdraws readiness first, then stops.
  - `oteryn-game-ops character bootstrap` is the socket client.
  - §4 physical qualification: `tools/qualification/node_boot/run.sh` and the `Node boot` workflow run the shipped binaries against the real Platform and a TLS PostgreSQL 17.6, and reproduce every #823 SEAM stage against the node's own port.
- **Depends on:** #843 (D4 evidence), #845 (B1), #849 (B2).
- **Excluded:** `OPS-CHANNEL-01`, resume/reconnect (#822), Platform changes, production endpoints and credentials.
