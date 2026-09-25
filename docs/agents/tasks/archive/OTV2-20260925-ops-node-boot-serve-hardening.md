# OTV2-20260925-ops-node-boot-serve-hardening

```yaml
task_id: OTV2-20260925-ops-node-boot-serve-hardening
title: OPS-NODE-BOOT-01 serve-path follow-ups from the #850 review
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
created_at: 2026-09-25T00:00:00Z
updated_at: 2026-09-25T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/node/serve.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - tools/qualification/node_boot/run.sh
  - docs/agents/tasks/active/OTV2-20260925-ops-node-boot-serve-hardening.md
public_contracts:
  - OPS-NODE-BOOT-01
depends_on:
  - pr:850
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

- **Governing decision:** `OPS-NODE-BOOT-01` (#830), D3 steps 1, 4 and 10.
- **Scope:** the owner-approved follow-ups from the #832 checklist that affect the serving path:
  - the gameplay certificate chain and key must form the served TLS configuration when inputs are loaded, before registration or readiness;
  - SIGTERM or SIGINT during registration, the assignment wait, or before readiness is published ends the node cleanly (exit 0) and never publishes `ready=true`;
  - after an ambiguous S2 initialization, both the stored provenance and the stored descriptor must equal the authorization.
- **Validation:** a unit test for a mismatched TLS pair, and a new §4 stage `signal_before_ready`.
- **Excluded:** the other deferred #832 items (fence descriptor retention, ops `owner_of` TOCTOU, control-role recovery-admission INSERT, the shutdown budget for in-flight work).
