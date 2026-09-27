---
task_id: OTV2-20260927-control-loss-lifecycle
title: Durable control loss from the composed owners
mode: IMPLEMENT
status: completed-on-merge
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/control-loss-lifecycle-20260927
base_sha: 09555d0f
issue: 162
jira: KAN-13
allocation_comment: 5855028769
allocation_addendum: 5855142430
owned_paths:
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/qualification.rs
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - apps/game-server/src/foundation/mod.rs
  - apps/game-server/src/foundation/movement_static_kernel_structural_tests.rs
  - apps/game-server/src/foundation/admission_authority_publication.rs
  - apps/game-server/src/foundation/fresh_admission_durability_tests.rs
  - apps/game-server/src/durability/fresh_admission.rs
  - apps/game-server/tests/durability_postgres.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md
  - docs/agents/tasks/archive/OTV2-20260927-control-loss-lifecycle.md
---

# Durable control loss from the composed owners

Authority:
- owner decision `DISCONNECT-PROTECTION-V1` §§1 and 4 (#822 comments 5854525644 and 5854565314), with provisional values;
- FND-04B §§4–6.

This is #822 stage 2, PR 4a.

## Outcome

- **Owning-loss source.** `ChannelOwnedLossSource`, a sealed `ControlLossSourceV1`, is resolved once at decision time from the current owners:
  - the Channel runtime: committed player actor present, placement identity and revision, and the assignment source revision;
  - the current durable GameSession: ACTIVE, the same controller transport, fresh origin, and this Channel's runtime scope;
  - the admitted account presence.

  The loss epoch is 1, and the grace deadline is the decision time plus 60 s.
- **Commit.** The existing `FreshAdmissionStore::commit_fresh_loss` revalidates the session, claims, reservation and runtime guard under relation locks. An ambiguous outcome is reconciled through `reconcile_fresh_loss`; a lost commit acknowledgement is never re-decided.
- **Claim ownership, not claim identity.** The loss store used to require both claim rows to equal the admission successors byte for byte. The seam found a real defect: a later, refused admission attempt for the same account re-observes Platform security and republishes the Account row, and the active session could then never enter loss (or reach grace). The store now uses `validate_claim_ownership_v1`, which checks:
  - the session snapshot is exact;
  - the keys match;
  - the Account presence names this character and session;
  - the Character holder is this session at the current lease generation.

  Ownership changes are still rejected (unit mutations and the Postgres scenario). The strict `validate_claim_preserving_session_v1` and terminal release are unchanged.
- **Channel mirror.** The owner records the committed `ControlLossMark` (epoch, grace deadline) on the still-present player actor. It is idempotent for the same decision and a conflict otherwise. The slot stays at 192 bytes and no second index is added.
- **Triggers.**
  - Liveness-proven loss (`AdmittedThenControlLost`) is recorded immediately.
  - A closed or failed admitted transport is recorded after the idle detection window (15 s): a socket close alone is not proof.
  - A shutdown drain never counts as loss.
- **Registry row** `FND04B-SAME-SESSION-GRACE-S` = 60 (provisional), bound to the code constant by a test.
- **Qualification.** The `stage=control_loss` stage checks that the closed and the silent sessions both become RECONNECTABLE with epoch 1 and grace 60 s. The Channel census shows two present, uncontrolled actors.

Excluded:
- grace-expiry release and actor removal (4b);
- R1b sources and positive `ClientResume` (PR 5);
- protection during loss;
- the logout block;
- resumed-history loss.
