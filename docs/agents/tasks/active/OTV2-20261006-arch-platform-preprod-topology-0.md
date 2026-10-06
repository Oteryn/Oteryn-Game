# OTV2-20261006-arch-platform-preprod-topology-0

```yaml
task_id: OTV2-20261006-arch-platform-preprod-topology-0
title: Persistent Platform preproduction topology for the Synology Game node (design only)
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-platform-preprod-topology-0-20261006
pr: null   # the control plane opens it
issue: 1622
base_sha: ee71e79eccd1d498d6c39ea25ac01ee74ccd118c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: architect worker for the control plane (#1622 D852)
created_at: 2026-10-06
updated_at: 2026-10-06
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_PLATFORM_PREPROD_TOPOLOGY_2026-10-06.md
  - docs/agents/tasks/active/OTV2-20261006-arch-platform-preprod-topology-0.md
public_contracts: []
depends_on: [ARCH-PREPROD-ROUTE-PUBLISH-AUTH-V1, ARCH-LCFA-PROJECTION-CONTRACT-V1, SYNOLOGY-GAME-DEPLOY-1]
blocks: [PLATFORM-PREPROD-TOPOLOGY-1, GAME-PREPROD-TOPOLOGY-1]
cross_repository_coordination_id: null
external_repositories: []   # Oteryn/Oteryn-Platform read only, at 81898fc1
```

## Outcome and authority

The owner answered **1a** on 2026-10-06 (#1622 D852). The control plane allocated one design
document. It specifies the smallest path to a persistent Platform `preproduction` topology in
which the Synology Game node can reach `ready`. It also specifies the Platform and Game
implementation packets.

This task has no runtime, deployment, production, protected-environment or Platform write
authority. Every deploy the design names needs a separate owner approval at run time.

## Source of truth

- PROVEN, from Platform `81898fc1`:
  - the disposable-store guard;
  - the route and trust publish commands;
  - Registry-issued UUIDv7 topology;
  - the runtime-status and scope-assignment mTLS peers and identities;
  - the `route_revision` digest;
  - the staging compose and the nginx configuration.
- PROVEN, from Game sources: the `deploy/synology-game/` templates and README, and the
  `login_local` runbook, including its split-store mirror.
- Bound META 3.1.0: `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`.

## Validation

- `git diff --check`
- `python tools/agents/validate_governance.py`
- `python -m unittest discover -s tools/agents/tests`
