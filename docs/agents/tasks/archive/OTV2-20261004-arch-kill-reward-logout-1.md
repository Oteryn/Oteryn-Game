# OTV2-20261004-arch-kill-reward-logout-1

```yaml
task_id: OTV2-20261004-arch-kill-reward-logout-1
title: "ARCH-KILL-REWARD-LOGOUT-1: KILL-REWARD-COMP-1 and LOGOUT-WIRE-1 packets"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-kill-reward-logout-20261004
issue: 162
pr: 1802
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_KILL_REWARD_LOGOUT_PACKETS_2026-10-04.md
  - docs/agents/tasks/archive/OTV2-20261004-arch-kill-reward-logout-1.md
public_contracts: []
depends_on: []
blocks: [KILL-REWARD-COMP-1, LOGOUT-WIRE-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- KILL-REWARD-COMP-1 packet (hard, persistence review), after ATTACK-1b #1798 and D3-7:
  - the creature reward binding: an optional pinned `loot_tables` section in the native gameplay
    manifest and a per-generation `CreatureRewardTable` (XP from `profile.experience`, the
    corpse through the D3-7 `i00005801` alias, the loot table, the Bestiary race), fail closed
    per creature;
  - the settle functions take owned `ProjectedCreatureDeathFacts` read under the runtime lock,
    and every durable write and slot acquisition runs after the channel guards are released;
  - a per-Channel settlement queue (`KILLRW-RL-01` = 64), drained by the reward principal's own
    session, and before its terminal release;
  - auto-attack and spell kills (the `CombatBatchReceipt` walk) feed the same queue;
  - a live-path PG test.
- LOGOUT-WIRE-1 packet (hard, protocol review), after ATTACK-1b #1798: `LOGOUT_INTENT` under
  `LOGOUT_V1` (numbers proposed, leased by the control plane), results `ACCEPTED`, `IN_FIGHT`
  with `retry_after_ms` and `BUSY`; an accepted logout runs `TerminalRelease::Logout` at once,
  with no grace and no PvE re-entry protection; the client binds Ctrl+L and Ctrl+Q.
- No code, registry or contract change.
- #1802 Codex round 1 (CP D659), on `cce35f04`:
  - 4179801655 (P1): the pin carries a `creature_loot` binding per creature, so a `loot: null`
    creature is told apart from a referenced table missing from the pin (§1.1).
  - 4179801659 (P1): the principal's session, lease generation and actor ref are captured under
    the lock by a new `top_damage_contributor` accessor; the carrier scope is widened (§1.3,
    §2.1).
  - 4179801661 (P1): `tools/content-schema/native-gameplay/**` (the manifest producer and its
    tests) is added to KILL-REWARD-COMP-1 owned paths and validation (§2.1).
  - 4179801664 (P2): a replayed receipt (`applied = false`) enqueues nothing, and the queue is
    unique per death key (§1.3, §1.4).
- #1802 Codex round 2 (CP D662), on `039c343c`:
  - 4179827380 (P1): a replay (`applied = false`) is walked again and repairs an interrupted
    projection or append; the death key uniqueness and the idempotent settle make it safe
    (§1.3, §1.4). This replaces the round 1 suppression.
  - 4179827384 (P1): the loot MINT capacity counts only loot MINTs in flight from running
    plans, not queued entries or corpses; a refused plan stays queued (§1.3).
  - 4179827388 (P1): `ACCEPTED` is sent only after the terminal release commits; a retryable
    failure answers `BUSY`, an unknown outcome closes without a result (§1.6, §2.2).
- #1802 Codex round 3 (CP), on `f78063cf`:
  - 4179853937 (P1): `spell_timer_callbacks.rs::apply_due_under_current_owners` walks
    `FireReport.receipts` for delayed creature kills, and the file and its new tests are in the
    KILL-REWARD-COMP-1 owned paths. Neither #1796 nor #1798 changes it (§1.4, §2.1).
  - 4179853940 (P1): the release handshake. The seal check and the `releasing` mark share the
    `attack` critical section with every append, so no entry is left behind by the session's own
    release; a retryable failure clears the mark (§1.3, §1.6, §2.1).
- #1802 Codex round 4 (CP), on `af8da914`:
  - 4179881274 (P1): the logout marker is derived, not stored. A committed logout is
    `session_state = 3` (TERMINAL, absorbing); resume of a terminal session is refused and
    protection exists only on resume, so no migration or 0078 lease is needed. LOGOUT-WIRE-1
    adds `release_logout_session` for a session with no control-loss epoch (§1.6, §2.2).
  - 4179881277 (P1): before `BUSY`, a current durable read and `settle_unended` lift the
    `TransitionFence`; a test shows a character write succeeds after `BUSY` (§1.6, §2.2).
- #1802 Codex round 5 (CP), on `8f348204`:
  - 4179908233 (P1): the handshake keeps the D132 attribution. An entry for a releasing
    principal is parked; one found before the transaction is sent aborts the release back to
    the drain. One parked while the transaction is in flight follows the outcome: committed
    logs `principal_gone` with the winner, and a retryable or non-terminal outcome queues it
    (§1.3, §2.1).
  - 4179908235 (P1): a lost terminal acknowledgement is reconciled. `commit_control_loss`
    returns `Terminal` on a TERMINAL row, and `reconcile_terminal` re-reads the row and runs
    `retire_reconciled`; a test covers it (§1.6, §2.2).
- #1802 Codex round 6 (CP), on `644068bb`:
  - 4179937822 (P1): the handshake also runs in `release_after_grace`, before
    `release_expired_loss`, with its outcomes mapped to step 5; a test covers a queued kill
    across ordinary loss and grace expiry (§1.3, §2.1).
  - 4179937825 (P1): the step 4 check and the move to phase `committing` are one `attack`
    critical section, the session's end point; only a park after it can be `principal_gone`
    (§1.3, §2.1).
  - 4179937828 (P1): `KILLRW-RL-01` bounds queued, in-flight and parked entries; an entry keeps
    one slot until settled or logged, so a retryable failure neither loses it nor exceeds the
    bound (§1.3, §2.1).
- #1802 Codex round 7 (CP), on `981b2c8b`:
  - 4179965906 (P1): the `Recorded` arm of `control_loss_lifecycle` retries a grace-expiry
    `Unknown` at a bounded rate and forgets the session only on a final result, so a long
    store outage keeps the fence, mark and parked entries for the next attempt; a test covers
    it (§0.2, §1.3, §2.1).
  - 4179965909 (P2): the death-key uniqueness check includes parked entries (§1.3, §2.1).

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
