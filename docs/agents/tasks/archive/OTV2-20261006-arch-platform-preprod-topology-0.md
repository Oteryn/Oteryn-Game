# OTV2-20261006-arch-platform-preprod-topology-0

```yaml
task_id: OTV2-20261006-arch-platform-preprod-topology-0
title: Persistent Platform preproduction topology for the Synology Game node (design only)
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-platform-preprod-topology-0-20261006
pr: 1893
issue: 1622
base_sha: ee71e79eccd1d498d6c39ea25ac01ee74ccd118c
head_sha: "exact frozen head in the control-plane FREEZE_SHA entry"
final_head_sha: "exact frozen head in the control-plane FREEZE_SHA entry"
owner: architect worker for the control plane (#1622 D852)
created_at: 2026-10-06
updated_at: 2026-10-06
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_PLATFORM_PREPROD_TOPOLOGY_2026-10-06.md
  - docs/agents/tasks/archive/OTV2-20261006-arch-platform-preprod-topology-0.md
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

git diff --check: pass
python tools/agents/validate_governance.py: pass
python -m unittest discover -s tools/agents/tests: OK

## Review and closeout

Codex round 1 on `7f6f8cd3` raised four P1 findings, all fixed in one push:
- a client-facing HTTPS Platform endpoint without mTLS, with tester CA trust and LAN DNS (§4.1);
- `platform-preproduction` provisioning before any dispatch, plus a fail-closed
  `protection-check` that reads the rule back (§3);
- the Game runbook runs the README one-time first-start sequence before any workflow dispatch
  (§8);
- this record is archived.

The same push records D855 (owner-confirmed Registry issuance) and D607 (dedicated MariaDB).
Review is decided by the control plane on the frozen head. Merge result: squash merge of
#1893, pending CI and Merge Queue at authoring.
