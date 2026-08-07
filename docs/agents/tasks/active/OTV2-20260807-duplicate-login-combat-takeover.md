# OTV2-20260807-duplicate-login-combat-takeover

```yaml
task_id: OTV2-20260807-duplicate-login-combat-takeover
title: Freeze newcomer-wins duplicate login with combat-aware anti-abuse
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260807-duplicate-login-combat-takeover
base_sha: f62724a09e6f3fe9e24a106b6ea7c2f196b35fef
owner: ChatGPT architecture coordinator
created_at: 2026-08-07T12:59:00+02:00
execution_budget_minutes: 30
owned_paths:
  - docs/agents/tasks/active/OTV2-20260807-duplicate-login-combat-takeover.md
  - docs/architecture/FND-ID-01_ACCOUNT_SINGLE_ONLINE_CHARACTER_OWNER_BASELINE.md
public_contracts:
  - docs/architecture/FND-ID-01_ACCOUNT_SINGLE_ONLINE_CHARACTER_OWNER_BASELINE.md
depends_on:
  - PR #70 one-online-character-per-account baseline
blocks:
  - FND-04 must consume newcomer-wins, combat-presence and fencing semantics
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform
```

## Outcome

Record the owner decision that a newly authenticated duplicate login supersedes the incumbent client session, while preventing login/character switching from becoming an escape from combat.

## Accepted decision

- newcomer wins over the incumbent client session after full authentication/admission authorization;
- incumbent client command authority is fenced/revoked;
- combat/logout state is server-authoritative and cannot be cleared by takeover/disconnect;
- if the incumbent is logout-eligible, old character leaves legally before a different character receives gameplay admission;
- if the incumbent is combat/logout locked, the old client loses control but the character remains in world simulation under normal disconnected/combat rules;
- while that mandatory world presence remains, admission of a different CharacterId on the same AccountId stays pending/blocked;
- takeover must not reset combat state, resources, position, conditions, cooldowns, death risk or already committed effects;
- same-CharacterId combat reattachment remains a deliberate `FND-04` decision;
- no runtime/protocol/persistence/Platform implementation is authorized.

## Acceptance criteria

- [x] Resolve newcomer-wins versus incumbent-wins policy.
- [x] Preserve one-character-per-account invariant.
- [x] Prevent character-switch/login combat escape.
- [x] Separate account authentication from gameplay admission.
- [x] Require stale incumbent commands to fail closed.
- [x] Preserve combat consequences during takeover.
- [x] Keep same-character combat reattachment explicit and unresolved.
- [x] Keep implementation deferred.
- [ ] Exact-head checks and independent architecture audit before merge readiness.

## Validation

Documentation-only architecture change. The semantic update modifies the existing account concurrency baseline and adds this bounded task record. Exact-head governance/security checks and independent architecture audit are required before merge readiness.
