# QUEST-XP-1b

```yaml
task_id: QUEST-XP-1b
title: "QUEST-XP-1b quest experience in the definition hash"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
lane_id: quest
base_branch: main
branch: claude/quest-xp-1b
pr: "opened with this record"
base_sha: c63359f
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-01GiZEBS5mFj87BsA5Y3PKHS (oteryn-hard-worker)
control_plane: claude-code-session-013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-04
updated_at: 2026-10-04
packet: "PR #1724 review finding 4176570851 (P2, deferred by the control plane as QUEST-XP-1b); QUEST-STATE-0 §6; QUEST-GATE-0 §5.5"
leases: none
owned_paths:
  - apps/game-server/src/quest/mod.rs
  - apps/game-server/tests/support/quest_xp_postgres_cases.rs
  - docs/agents/tasks/archive/QUEST-XP-1b.md
depends_on:
  - "QUEST-XP-1 merged (#1724)"
public_contracts: []
external_repositories: []
```

## Outcome

- `definition_hash` (`src/quest/mod.rs`) now covers each transition's `experience` (a presence
  byte, then the amount as a big-endian i64), with `DEFINITION_HASH_VERSION` 2. QUEST-STATE-0 §6
  exempts only journal text. A reward edit is now a `REVISION_MISMATCH` for a Character already
  pinned to the quest. Before this change, the writer accepted the edited catalogue and stamped
  an obligation with the new amount under the old pin.
- The version bump changes every quest's hash. No stored pin is affected, because production
  loads no quest catalogue yet (`quest_catalogue: None` until QUEST-LOWER-1).

## Tests

- Unit: adding a reward changes the hash, and so does changing the amount.
- PostgreSQL 17.6 (`quest_xp_postgres_cases::a_reward_edit_is_a_revision_mismatch_for_a_quest_in_progress`):
  a Character pinned by `plain` is refused `REVISION_MISMATCH` by a catalogue that changes only
  `again`'s reward. Nothing is written and no obligation exists. The pinned definition still
  commits and owes its reward.

## Validation

- `cargo fmt --all --check`: pass.
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`: pass.
- `cargo test -p oteryn-game-server` with PostgreSQL 17.6: pass.
- `python tools/agents/validate_governance.py`: pass.

## Review

Independent persistence review on the final frozen head; the control plane requests it.
