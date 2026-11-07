# OTV2-20261003-ach-notify2

```yaml
task_id: OTV2-20261003-ach-notify2
title: "ACH-NOTIFY-2: fail-closed achievement notice (tri-state, bounded read)"
mode: HARD
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/ach-notify2-20261003
pr: "the ACH-NOTIFY-2 PR on this branch"
base_sha: "main after #1653 merged"
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
owner: claude-code-session-013WvUsVCbrbJ7HD2GUihBPw (HARD worker, CP session_013KJX6mv8LQveCKKXYgAX94)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/interaction/chest_use.rs
  - apps/game-server/src/durability/reward_claim_mint.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/resume.rs
  - apps/game-server/tests/support/chest_use_postgres_cases.rs
  - crates/protocol-oteryn/src/achievement_notices.rs
  - crates/protocol-oteryn/src/achievement_notices_tests.rs
  - docs/agents/tasks/archive/OTV2-20261003-ach-notify2.md
public_contracts: []
leases: none (capability 8 and domain 13 belong to ACH-NOTIFY-1)
depends_on:
  - "#1653 (ACH-NOTIFY-1)"
  - "D327 packet: ARCH-BATCH-D327-PACKETS-V1 §1.1 (PR #1655)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

D327, ARCH-BATCH-D327 §1.1. This repairs the two P2 findings Codex raised on #1653
(4173748648 and 4173748650), which were deferred under D245.

- **Tri-state notice.** `UseOutcome.earned` is now a `connection::EarnedNotice`, built from a
  `reward_claim_mint::GrantNotice`. It has three states:
  - `NoneEarned`: no grant, `AlreadyHeld`, `Retired` or a replay.
  - `Earned`: a committed `Granted` grant with its notice.
  - `Unknown`: the read overflowed, a fact key is missing from the catalogue, or the commit
    was unproven and a replay followed.

  `Unknown` is never read as `NoneEarned`. Each `Unknown` writes one operator event,
  `event=achievement_notice_unknown level=error`.
- **Bounded read.** The fact read inside the transaction is limited to `catalogue_len + 1`
  rows, where `catalogue_len` is the loaded catalogue's size, passed in by `settle_chest_use`.
  This replaces the fixed 4,096. More than `catalogue_len` rows gives `Unknown`; the read is
  never truncated. `commit_reward_claim_mint` passes `None` and does no read.
- **Disconnect.** With capability 8 selected, an `Unknown` notice ends the connection right
  after the command result, through the existing fail-closed path. No delta is sent and the
  revision is unchanged. The resumed connection's snapshot restores the watermark. A failed
  delta send already disconnects. Without capability 8 nothing changes. The durable grant is
  never rolled back.
- **Unchanged.** The wire, the registry, capability 8 and domain 13, fencing, persistence and
  migrations. Production still never selects capability 8.

## Validation

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p oteryn-protocol-oteryn -p oteryn-game-server`, against a local
  PostgreSQL 17.6.
- `python3 tools/agents/validate_governance.py`
- New tests:
  - the three states and their operator events (`mod.rs`);
  - an `Unknown` notice disconnects only with capability 8, and a resume delivers the snapshot
    with the grant (`connection.rs`);
  - a read of exactly `catalogue_len` keys passes, one more gives `Unknown` with the grant
    durable, and `AlreadyHeld` gives `None` (`chest_use_postgres_cases.rs`).
