---
task_id: OTV2-20260927-client-resume
title: Serve ClientResume through same-session reauthenticated recovery
mode: IMPLEMENT
status: completed-on-merge
repository: Oteryn/Oteryn-Game
base_branch: main
base_sha: 0d01e984
branch: agent/client-resume-20260927
issue: 162
jira: KAN-13
allocation_comment: 5857106124
owned_paths:
  - apps/game-server/src/durability/fresh_admission.rs
  - apps/game-server/src/durability/recovery_evidence_composition.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/tests/durability_postgres.rs
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/resume.rs
  - apps/game-server/src/gameplay_transport/fresh_evidence.rs
  - apps/game-server/src/gameplay_transport/qualification.rs
  - tools/qualification/node_boot/run.sh
  - docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md
  - docs/agents/tasks/archive/OTV2-20260927-client-resume.md
---

# Serve ClientResume through same-session reauthenticated recovery

This is #822 stage 2, PR 5b. It follows 5a (#991).

Authority:
- FND-04B §§16–20;
- the design and security choice on #822, comment 5856815918.

## Outcome

- **Registered recovery at the decision point.** `apply_registered_complete_reconnect` revalidates the recovery credential against the Recovery V2 floors fenced in the same deciding transaction. The lock order is: admission relations → custody → registration → floors. It uses `decide_with_registered_recovery`, and the sealed source never leaves the call. The unfenced variant is test-only.
- **Supporting store operations:**
  - `owning_loss` reads the durable original loss and its decision time;
  - `abort_complete_reconnect` withdraws a refused or unproven PREPARE, which then becomes `Terminal` in the budget;
  - `release_abandoned_session` terminally releases a resumed session whose recovered connection ended, so it is never stranded ACTIVE on a dead transport. Loss after a resume is 5c.
- **Evidence refresh.** `FreshEvidenceSource::refresh_recovery` fetches ReadRecoveryAccountSecurityV2 and ReadRecoverySigningTrustV2 into S2 custody, with replayable publication bindings.
- **FND-02 continuity.** Each `AdmittedSession` carries its `SessionContinuity`: connection generation, next CommandId, server_sequence and spatial revision, current when the connection ends. A resumed connection continues from it at the new generation.
- **ClientResume handling.** `admit_frame` serves `ClientResume` through `FreshAdmissionAuthority::resume`. `resume.rs` composes it:
  1. check the lost-session registry;
  2. refresh evidence and verify against the registered floors;
  3. build the owning snapshot (durable loss, current session, claims, budget, runtime facts, candidate, proof transition, FND-02 fence);
  4. PREPARE, then fresh reauthorization, then COMMIT, each reconciled and never decided twice;
  5. restore control of the same actor.

  The server then replies with `ServerResumeAccepted` and a snapshot at generation 2.
- **Topology.** The node-boot topology now publishes a recovery key, like WP5.
- **Seam.** The `stage=resume` stage covers:
  - a same-session resume within grace: generation 2, CommandId 6, server_sequence 7, the same actor at (0,0,0) rev 3, a step east to rev 4, no new admission;
  - a replayed credential is refused;
  - the resumed-then-lost session is released, and the character enters again.

Excluded:
- recording loss after a resume (5c);
- fast-reconnect proof issuance;
- early terminal replacement;
- moving the post-reentry protection from 4 s to ~10 s (the tuning stage).

## Independent review (head ded2f695)

The review found no security or authority blocker. It confirmed:
- only a durably lost session of this Channel can be resumed, by its own account, within grace and with a fresh credential revalidated against fenced S2 floors;
- the nonce is single-use;
- a healthy controller cannot be preempted;
- the lock order has no cycle;
- `release_abandoned_session` cannot release the wrong session.

1. **(Medium, fixed) Committed but undelivered resume left the session stranded.**
   - A failed `ServerResumeAccepted` write, and likewise a failed `ServerAccepted` write, now enter the loss lifecycle; a resumed session is then released as resumed history.
   - A COMMIT whose outcome cannot be proven triggers `release_abandoned` on the exact candidate transport, which releases it only if it landed, and then withdraws the attempt.
   - A failure to clear the runtime mark no longer discards a durable switch.
2. **(Medium, fixed) Unproven PREPARE was never withdrawn.** Every non-`Prepared` outcome, including a reconcile error, now withdraws the attempt.
3. **(Low, accepted) Recovery evidence refresh runs before credential verification.** This follows the same pattern as fresh admission. The only impact is liveness, and session ids are random.
4. **(Low, fixed) Content revisions.** The deciding transaction now also requires the runtime guard's ruleset, content, map and world-policy revisions to equal the verified credential's current evidence.
5. **(Low, fixed) `hold_admitted`** uses the session's own connection generation.
6. **(Low, fixed)** COMMIT is reauthorized at a fresh durable time.
