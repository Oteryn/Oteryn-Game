# OTV2-20261001-charm-native-condition-expiry

```yaml
task_id: OTV2-20261001-charm-native-condition-expiry
title: Native exact-deadline non-ticking condition expiry
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: codex/charm-native-conditions-owner-20261001
branch: codex/charm-native-condition-expiry-20261001
pr: 1493
issue: 162
base_sha: 4d83662d99cfcdd3c9181850d7b05a91c0abe645
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex root, sole publisher; conditions source preparer
created_at: 2026-10-01
updated_at: 2026-10-01
owned_paths:
  - apps/game-server/src/ability/condition.rs
  - apps/game-server/src/ability/condition_tests.rs
  - apps/game-server/src/foundation/runtime_actor_conditions.rs
  - apps/game-server/src/foundation/runtime_actor_conditions_tests.rs
  - docs/agents/tasks/archive/OTV2-20261001-charm-native-condition-expiry.md
public_contracts: []
depends_on: [CONDITIONS-0, 1488]
blocks: []
external_repositories: []
```

## Bounded implementation

Speed, Light, ManaShield and Cleanse immunity expire at the exact authored deadline.
The owner prepares cleanup against the current actor revision and commits through the existing
current scope/session/generation/time checks. Reads exclude an expired speed contribution before
cleanup; applying a newer definition first expires the non-ticking candidates. Original prepared
apply/Cleanse/expiry plans cannot remove or renew later replacements.

DOT, food and recovery remain under take_due: even overdue occurrences and an exhausted per-tick
budget are preserved byte-for-byte by this cleanup. No new timer, family, resource ceiling,
protocol or persistence semantics. Source budget410 changed Rust lines across four source paths.

AuthorityInvariant × ConsumerBoundary × MutationOperator covers independently current actor
session/generation, monotonic semantic time and expected revision at cleanup commit/read.
Tests isolate wrong session, replacement generation, decreasing/future time, stale plan after
refresh and retained original replay. Durable recovery is NOT_APPLICABLE to this ephemeral state.

## Validation and remaining work

Full server library1305PASS,2existing ignored; three new core tests run in both ability aliases,
two new native tests and the extended native Cleanse test prove exact deadlines, authored speed
refresh/replacement, immunity removal and preserved overdue ticks/budgets. Workspace rustfmt,
diff check and strict all-target Clippy PASS. No PostgreSQL/client/gameplay E2E qualification.

The full nine-row Charm task remains IMPLEMENTING. SPEED-1 pacing, owner-cycle scheduling,
scheduled tick effects and the attack/Charm consumer are still separate unfinished consumers;
no availability/capability flag is activated by this child. Root publishes the actual local Git
identity through a guarded normal push and exact remote readback. Final-head independent review
and CI belong to the external FREEZE_SHA packet. Protected integration is control-plane owned;
the stack must be retargeted/requalified after PR1488's parent integrates. This archive reaches
main only if PR1493 merges; a commit cannot contain its own final SHA.
