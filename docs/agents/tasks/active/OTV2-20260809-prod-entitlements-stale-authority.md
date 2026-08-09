# OTV2-20260809-prod-entitlements-stale-authority

```yaml
task_id: OTV2-20260809-prod-entitlements-stale-authority
title: Reconcile PROD-ENTITLEMENTS-01 with Platform stale-authority security evidence
mode: CONTRACT
status: active
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/prod-entitlements-stale-authority
pr: null
issue: 115
base_sha: 27f7f647f04e3b1a4151f9b124401986910f03d8
head_sha: null
owner: chatgpt-architecture-agent
created_at: 2026-08-09T14:39:00+02:00
updated_at: 2026-08-09T14:39:00+02:00
execution_budget_minutes: 45
owned_paths:
  - docs/agents/tasks/active/OTV2-20260809-prod-entitlements-stale-authority.md
  - docs/architecture/PROD-ENTITLEMENTS-01_PLATFORM_GAME_ENFORCEMENT_DEPENDENCY.md
public_contracts:
  - docs/architecture/PROD-ENTITLEMENTS-01_PLATFORM_GAME_ENFORCEMENT_DEPENDENCY.md
depends_on:
  - existing canonical gate PROD-ENTITLEMENTS-01
  - Oteryn Platform PR #925 merge b1e5957614b29e88825ba74425e979be9b6bd070
  - Oteryn Platform Issue #944 / OPA-SEC-0007
blocks:
  - Profile-B Premium/VIP or other game-consumed entitlement implementation/activation
cross_repository_coordination_id: OTV2-PROD-ENTITLEMENTS
external_repositories:
  - blakinio/Oteryn-Platform
```

## Outcome

Record the newly proven Platform-side stale-authority security dependency under the already registered `PROD-ENTITLEMENTS-01` gate without changing FND-04A, implementing runtime code, or writing to Oteryn Platform.

## Proven state

- `PROD-ENTITLEMENTS-01` already exists in the canonical Oteryn-v2 decision horizon as the deferred Entitlements, Premium and Commerce Boundary.
- Oteryn Platform PR #925 accepted the separation between Platform entitlement lifecycle and Oteryn-v2 gameplay enforcement/game-affecting delivery.
- Oteryn Platform Issue #944 proves a P1/high-risk contract gap: Profile-B game-consumed entitlement evidence currently lacks a mandatory finite stale-authority lifetime.
- Platform #944 remains open and is the Platform-side remediation owner.
- Oteryn-v2 PR #114 owns exactly three FND-04A fresh-admission files; this package owns no overlapping path and does not modify FND-04A semantics.

## Acceptance criteria

- [ ] One canonical Oteryn-v2 architecture document records Platform #944 as a mandatory producer-side prerequisite for `PROD-ENTITLEMENTS-01` activation.
- [ ] The document preserves Platform commercial/entitlement authority and Oteryn-v2 gameplay enforcement authority.
- [ ] Finite stale-authority, anti-rollback, expiry/revocation, outage/reconnect/restart and clock semantics are registered as mandatory future contract outcomes.
- [ ] FND-04A remains unchanged and unblocked by this package.
- [ ] No Platform repository write, runtime implementation, schema, deployment or production action occurs.
- [ ] Exact-head governance/CI and independent review pass before merge.

## Excluded scope

No payment provider choice, Premium/VIP product design, entitlement runtime/schema, FND-04B/C change, game-server implementation, protocol implementation, Platform contract repair, deployment or production activation.

## Context checkpoint

```yaml
last_progress: Issue #115 created and reconciled to existing canonical gate PROD-ENTITLEMENTS-01; dedicated branch created from current main.
status: active
branch: docs/prod-entitlements-stale-authority
head_sha: null
pr: null
blocker: null
next_action: Add the bounded canonical dependency document, self-review exact diff, open a draft PR, then validate exact-head CI.
```
