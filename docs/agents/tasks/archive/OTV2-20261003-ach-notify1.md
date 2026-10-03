# OTV2-20261003-ach-notify1

```yaml
task_id: OTV2-20261003-ach-notify1
title: "ACH-NOTIFY-1: earned-achievement notice (capability 8, state domain 13)"
mode: HARD
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/ach-notify1-20261003
pr: "the ACH-NOTIFY-1 PR on this branch"
base_sha: d75ba6d6bc0c02f6c13b4ff860c9bf3a439930e7
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
owner: claude-code-session-013WvUsVCbrbJ7HD2GUihBPw (HARD worker, CP session_013KJX6mv8LQveCKKXYgAX94)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - crates/protocol-oteryn/src/achievement_notices.rs
  - crates/protocol-oteryn/src/achievement_notices_tests.rs
  - crates/protocol-oteryn/src/lib.rs (mod line, registered capability list and its test)
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json (capability 8, state domain 13)
  - docs/contracts/protocol-oteryn/v1/achievement_notices_v1.proto
  - apps/game-server/src/gameplay_transport/{connection,resume}.rs
  - apps/game-server/src/gameplay_transport/mod.rs (CP extension, minimal)
  - apps/game-server/src/interaction/chest_use.rs (CP extension, minimal)
  - apps/game-server/src/durability/reward_claim_mint.rs (grant caller site)
  - apps/game-server/tests/support/chest_use_postgres_cases.rs (CP-required concurrency test)
  - docs/agents/tasks/archive/OTV2-20261003-ach-notify1.md
public_contracts:
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - docs/contracts/protocol-oteryn/v1/achievement_notices_v1.proto
depends_on:
  - "D292 ACHIEVEMENT-0 §5 (merged 0d9bdda8)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Allocation D292 (#1622), ACHIEVEMENT-0 §5. CP answers 1a, 2a and 3a; #1534 overlap accepted
under D319.

- **Wire.** `AchievementEarnedV1 {key=1, name=2, fact_count=3, total_points=4}` (worst case 241
  of 250 bytes) and `AchievementNoticesSnapshotV1 {fact_count=1, total_points=2}` (worst case 12
  of 16 bytes). Both codecs are strict in both directions. Capability 8 (`offered: false`, no
  command type) and domain 13 are registered.
- **Grant.** A reward-claim commit whose grant is `Granted` also returns the key and the
  account's fact keys. The keys are read in the same transaction after the insert (read-only,
  at most 4,096). `AlreadyHeld`, `Retired` and a replayed CommandRef return none.
- **Transport.** `SessionContinuity.achievement_notice_revision` is `None` unless capability 8
  is selected. **Production never selects it until capability negotiation lands** (the server
  selects `[]` at admit and resume). When selected:
  - the join, resync and reconnect snapshot carries the watermark at the current revision;
  - each committed `Granted` sends one delta after its command result;
  - the revision advances before the write, so it is never reused;
  - a resume carries the revision and the FND-02 reconciliation fence includes it.
- **Unchanged.** Session-generation fencing, the grant's persistence semantics, the panel query
  and every other granter. Quest, encounter and counter grants are later tasks.

## Validation

- `cargo fmt --all --check`; `cargo clippy -p oteryn-protocol-oteryn -p oteryn-game-server
  --all-targets -D warnings`.
- `cargo test -p oteryn-protocol-oteryn -p oteryn-game-server`. The PostgreSQL 17 suites
  `chest_use_postgres`, `reward_claim_mint_postgres` and `account_achievement_postgres` were run
  locally against PG 17.
- New tests:
  - codec bounds and fail-closed;
  - one delta per `Granted` and none without one;
  - no delta without capability 8;
  - snapshot only, never a delta, on reconnect, with the revision monotonic across it;
  - fail-closed on an unreadable watermark;
  - two concurrently issued grants with distinct, exact watermarks.

## Notes for review

- Watermark exactness relies on reward-claim commits being serialized: by the one-connection
  durability root on a node, and by the EXCLUSIVE admission locks across nodes. Later granters
  (quest, encounter, counter) do not take those locks and need their own account serialization
  before they reuse this read.
- A delta whose encoding fails disconnects the session, as for the other domains. The fact stays
  durable and the next snapshot restores the watermark.
