# OTV2-20260807-combat-takeover-incumbent-protection

```yaml
task_id: OTV2-20260807-combat-takeover-incumbent-protection
title: Protect active incumbent client during combat-aware duplicate login
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260807-combat-takeover-incumbent-protection
base_sha: 18b708dcf76097cda61423a5fd6469b23aa6c09e
owner: ChatGPT architecture coordinator
created_at: 2026-08-07T13:35:00+02:00
execution_budget_minutes: 30
owned_paths:
  - docs/agents/tasks/active/OTV2-20260807-combat-takeover-incumbent-protection.md
  - docs/architecture/FND-ID-01_ACCOUNT_SINGLE_ONLINE_CHARACTER_OWNER_BASELINE.md
public_contracts:
  - docs/architecture/FND-ID-01_ACCOUNT_SINGLE_ONLINE_CHARACTER_OWNER_BASELINE.md
depends_on:
  - PR #71 combat-aware duplicate login takeover
blocks:
  - FND-04 must consume incumbent-protection and reconnect semantics
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform
```

## Outcome

Correct the duplicate-login policy so a second client cannot forcibly disconnect a healthy incumbent client while the incumbent character has an active combat/PZ/logout blocker.

## Accepted decision

- newcomer-wins remains valid only when the incumbent is logout-eligible or no longer owns a healthy active gameplay transport;
- if the incumbent client is healthy/connected and its character has an active combat/PZ/logout blocker, a second client must not fence, revoke, close or steal that incumbent gameplay session;
- the second client may authenticate at account level, but gameplay takeover/admission stays blocked or pending until the blocker ends or the incumbent session genuinely becomes unavailable;
- a different CharacterId cannot be admitted while the incumbent character still has mandatory world presence;
- same-CharacterId recovery after genuine incumbent transport loss remains allowed as a reconnect problem, but must preserve the exact same in-world actor and all combat state;
- server-side liveness/fencing, not a client claim, decides whether the incumbent session is actually unavailable;
- no runtime/protocol/persistence/Platform implementation is authorized.

## Acceptance criteria

- [x] Prevent second-client forced disconnect of an active combat/PZ-locked incumbent.
- [x] Preserve anti-X-log combat consequences.
- [x] Preserve one-character-per-account authority invariant.
- [x] Preserve safe reconnect path after genuine transport/session loss.
- [x] Prevent a second client from self-triggering or maliciously triggering loss of control during combat.
- [x] Keep implementation details deferred to FND-04.
- [ ] Exact-head checks and independent architecture audit before merge readiness.

## Validation

Documentation-only architecture correction. Exact-head governance/security checks and independent architecture audit are required before merge readiness.
