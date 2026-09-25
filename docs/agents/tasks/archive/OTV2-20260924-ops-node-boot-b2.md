# OTV2-20260924-ops-node-boot-b2

```yaml
task_id: OTV2-20260924-ops-node-boot-b2
title: OPS-NODE-BOOT-01 operator binary, closed configuration and file checks
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/zen-dirac-mxruho-pr-b2
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
  - Cargo.toml
  - Cargo.lock
  - apps/game-server/Cargo.toml
  - apps/game-server/src/lib.rs
  - apps/game-server/src/node/
  - apps/game-server/src/bin/oteryn-game-ops.rs
  - apps/game-server/src/character_recovery_fence.rs
  - apps/game-server/src/durability/character_authority.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/tests/character_authority_postgres.rs
  - docs/agents/tasks/active/OTV2-20260924-ops-node-boot-b2.md
public_contracts:
  - OPS-NODE-BOOT-01
depends_on:
  - pr:845
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

- **Governing decision:** `OPS-NODE-BOOT-01` (merged in #830; owner accepted D1–D6 on 2026-09-24).
- **Slice PR-B2 (D1/D2 operator side):**
  - the closed TOML configuration of the serving node and of the operator; secrets only by file reference;
  - descriptor-based, no-follow file checks, exclusive creation with ownership handoff, and operator state directory validation;
  - the Character recovery fence gets owner, mode and parent checks, and creates handed-off 0600 files;
  - `oteryn-game-ops`: launch authorization issue/reconcile/revoke, registration revoke, S2 descriptor issuance, fresh Character store and interpretation, assignment assign/replace/revoke/reconcile; runs only as root and never as the service user.
- **Depends on:** PR-B1 (#845), whose control-plane database operations the tool calls.
- **Later slice:** PR-B3, the serving boot sequence, the control socket and the §4 physical qualification with the shipped binaries.
- **Excluded:** `OPS-CHANNEL-01`, resume/reconnect (#822), Platform changes, production endpoints and credentials.
