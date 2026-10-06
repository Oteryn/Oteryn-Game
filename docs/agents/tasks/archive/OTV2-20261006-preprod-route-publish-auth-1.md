# OTV2-20261006-preprod-route-publish-auth-1

```yaml
task_id: OTV2-20261006-preprod-route-publish-auth-1
title: "ARCH-PREPROD-ROUTE-PUBLISH-AUTH-V1: authority request for the preproduction route-publish operator path"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: cand/preprod-route-publish-auth-1
issue: 162
pr: 1871
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session_01WQyZ8BUWVpmDLpSTpXHvn1 (Sol Supervising Architect)
created_at: 2026-10-06
updated_at: 2026-10-06
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_PREPROD_ROUTE_PUBLISH_AUTH_2026-10-06.md
  - docs/agents/tasks/archive/OTV2-20261006-preprod-route-publish-auth-1.md
public_contracts: []
depends_on: []
blocks: [PLATFORM-NATIVE-PREPROD-OPS-1, RUNBOOK-1]
cross_repository_coordination_id: OTV2-20260929-N4P-NATIVE-GATEWAY-LOGIN
external_repositories: []
```

## Outcome

- Control plane D821 item 2 (#162, 2026-10-06). This is an authority request only. It grants
  nothing until the owner answers its §6.
- Findings from Platform `origin/main` 3896bcd:
  - no Platform preproduction deployment exists (Synology is public `APP_ENV=staging`);
  - `isolatedConnection()` admits only disposable stores;
  - neither `publishRouteForPreproduction` nor `publishTrustedKey` has an operator command;
  - issued WorldId and ChannelId are permanent.
- Options:
  - A (recommended): a disposable stack, plus one Platform PR adding two commands limited to
    testing and preproduction.
  - B: persistent private preproduction, deferred until rollout step 7.
  - C: public staging, rejected.
- Owner questions §6 items 1–4 are routed through the control plane.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
