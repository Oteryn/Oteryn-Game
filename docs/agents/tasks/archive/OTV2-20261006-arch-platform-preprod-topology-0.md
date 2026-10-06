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

The same push records D855 (owner-confirmed Registry issuance), D856 (owner-confirmed new
`platform-preproduction` environment, `synology-staging` not reused) and D607 (dedicated
MariaDB).
Codex round 2 on `f5424197` raised three P1 findings and one P2, all fixed in one push:
- the client settings add `OTERYN_CHARACTER_ID` and its bootstrap source;
- all four interpretation tokens are pinned and shared with the Platform bootstrap intents;
- `protection-check` also reads back `can_admins_bypass`;
- the epoch procedure depends on the scope's state: `replace` through the deploy, then
  `reconcile` or `report`, never a second `assign`.

Codex round 3 on `39929d2b` raised three P1 findings. The root cause was a §3 environment that
had not been checked against `login_local` line by line. One push rewrites §3 from the
`login_local` union (`wp5_s3a`, `wp5_s3b`, `node_boot` and `login_local` overlays) and adds a
§3.1 parity table:
- the native evidence high-water directory sits inside the state root on a persistent mount;
- Gateway native login is enabled with the service-token pair, its provisioning, and a
  compose-internal `platform-web` listener;
- the bootstrap-intent identity is the node evidence subject (OPS-NODE-BOOT-01 D1), and the
  separate bootstrap certificate is removed.
The same diff fixes the mTLS FastCGI parameters, the OAuth client command, the World row
command, the admission key placement, the Game root copies, tester registration, and the
README NAS-values rows in §10.

Review is decided by the control plane on the frozen head. Merge result: squash merge of
#1893, pending CI and Merge Queue at authoring.
