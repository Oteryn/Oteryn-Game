---
task_id: OTV2-20260927-resumed-loss
title: Record a loss after a same-session resume as the next epoch
mode: IMPLEMENT
status: completed-on-merge
repository: Oteryn/Oteryn-Game
base_branch: main
base_sha: 2b95309a
branch: agent/resumed-loss-20260927
issue: 162
jira: KAN-13
allocation_comment: 5858400669
owned_paths:
  - apps/game-server/src/foundation/admission_recovery_inner.rs
  - apps/game-server/src/durability/fresh_admission.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/qualification.rs
  - apps/game-server/tests/durability_postgres.rs
  - docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md
  - docs/agents/tasks/archive/OTV2-20260927-resumed-loss.md
---

# Record a loss after a same-session resume as the next epoch

This is #822 stage 2, PR 5c. It follows 5b (#1000).

Authority: FND-04B §§5–8 and §§16–20.

## Outcome

- **Next epoch.** When a resumed connection ends, the loss is recorded as the resumed epoch plus one. The new epoch has its own original grace deadline and an empty recovery budget (`attempt_count` reset). The player can resume the same GameSession again, any number of times within each grace.
- **Retained history.** The loss carries `ControlLossHistoryV1::Resumed`:
  - the restored budget of the resumed epoch;
  - that epoch's original grace deadline;
  - the protection its committed recovery left, re-derived with `RecoveryProtectionContinuityV1::after_complete_reconnect` at the COMMIT decision time.
- **Durable fence.** `commit_fresh_loss` re-derives that history from the immutable receipts under the relation locks (`resumed_history_locked`) and refuses any other history or epoch. Its update requires the resumed epoch and no prepared attempt. `resumed_history` exposes the same read to the owning source.
- **Codec.** The owning-loss encoding now carries the history: tag 1 is a fresh origin (bytes unchanged), tag 2 is resumed history with its budget, grace and protection. Stored epoch-1 receipts decode as before.
- **Protection.** Protection is carried unchanged; a resume never re-arms it (§8). Re-arm stays with the deferred protection work.
- **Fallback.** A resumed session whose loss cannot be proven or is refused is still released (`ResumedHistory`), never stranded ACTIVE on a dead transport.
- **Seam.** The new `stage=resumed_loss` stage covers:
  - the resumed connection's loss becomes epoch 2 with a 60 s grace;
  - a second same-session resume: generation 3, CommandId 7, server_sequence 9, the same actor at (1,0,0) rev 4, a step west to rev 5, no new admission.

  `stage=grace_expiry` then waits for that session's epoch-3 loss to expire. Readmission retries for a bounded time, because the Channel removes the actor only after the TERMINAL fact.

Excluded:
- protection re-arm (PR 2);
- logout block (PR 3);
- protection tuning (PR 6);
- fast-reconnect proof issuance;
- early terminal replacement.
