# OTV2-20261004-premium-activation-0

```yaml
task_id: OTV2-20261004-premium-activation-0
title: "PREMIUM-ACTIVATION-0: the trusted clock, the Premium read seam, PREM-WIRE-1 and GUILD-1"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/premium-activation-0-20261004
issue: 162
pr: 1743
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_PREMIUM_ACTIVATION0_GAMEPLAY_SWITCH_OVER_DECISION_2026-10-04.md
  - docs/agents/tasks/archive/OTV2-20261004-premium-activation-0.md
public_contracts: []
depends_on: []
blocks: [PREM-WIRE-1, GUILD-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- Control plane D490 item 4: the Premium-gated part of #1738 (ARCH-CORE-LOOP-PACKETS-2 part C)
  moves here. It covers the trusted clock (§1.1), the `PremiumStatus` seam with its activation
  gate and switch-over latch (§1.2), the consumer table (§1.3), PREM-WIRE-1 (§2.1) and GUILD-1
  (§2.2). The earlier #1738 fixes come with it: P1 4176929764, P1 4176947451, P2 4176947456 and
  P1 4176973984.
- #1738 P1 4176993795: a process that holds no latch row re-reads the durable table before it
  may return `NotActivated`, so a row written by another node is always seen. There is a
  two-node test (§1.2, §2.1).
- #1738 P1 4176993801: any latch row means Premium has been delivered, whatever the
  configuration says. A missing or mismatched activation after delivery gives `NotCurrent`. The
  latch row stores `switch_over_us` (§1.2, §2.1).
- #1738 P2 4176993804: `CasterState` carries `PremiumStatus`. A `wheel_unlock` spell skips the
  Premium check under `NotActivated`, as WHEEL-0 §6.2 requires (§1.3, §2.1).
- #1743 P1 4177047049: the latch is a single-row table keyed by the constant 1 and written by
  insert-if-absent. In a mixed rollout exactly one node wins; the others hold the winner's row and
  read `NotCurrent`. Migration lease 0073 (§1.2, §2.1).
- #1743 P1 4177047054: a `NotActivated` result exists only inside a transaction that holds a
  shared advisory lock, and the bypass is applied under it. The latch insert takes the lock
  exclusively, so no bypass applies after the durable switch-over (§1.2, §2.1, §4).
- #1743 P1 4177076440: the seam is a gate, `with_premium_gate(account_id, |status, tx| …)`, whose
  `NotActivated(PreDelivery<'g>)` cannot leave the closure. The Wheel cast and GUILD-1's writes
  apply their bypass inside it, under the shared lock; there is a test where the latch commits
  between read and cast (§1.2, §2.1, §2.2, §4).
- #1743 P1 4177096277 (owner D494): the latch no longer deadlocks against the caller's shared lock.
  - The gate decides first.
  - On `upper >= S` it rolls back its shared transaction, then latches in a fresh transaction under
    a 5 s `lock_timeout`; a timeout gives `NotCurrent`.
  - The closure then runs with the row held.
  - Bounded-timeout acceptance tests cover one node, two concurrent nodes and a paused bypass.
  - The same-transaction upgrade is rejected (§1.2, §2.1, §4).
- No code, contract, wire or migration change.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
