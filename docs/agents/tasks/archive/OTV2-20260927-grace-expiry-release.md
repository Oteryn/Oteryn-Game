---
task_id: OTV2-20260927-grace-expiry-release
title: Grace-expiry terminal release of a lost GameSession
mode: IMPLEMENT
status: completed-on-merge
repository: Oteryn/Oteryn-Game
base_branch: main
base_sha: 50913f24
branch: agent/grace-expiry-release-20260927
issue: 162
jira: KAN-13
allocation_comment: 5856044724
owned_paths:
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/qualification.rs
  - apps/game-server/src/durability/fresh_admission.rs
  - apps/game-server/tests/durability_postgres.rs
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md
  - docs/agents/tasks/archive/OTV2-20260927-grace-expiry-release.md
---

# Grace-expiry terminal release of a lost GameSession

This is #822 stage 2, PR 4b. It follows PR 4a (#974).

Authority:
- owner decision `DISCONNECT-PROTECTION-V1` §4: 60 s grace from the loss decision; values are provisional;
- FND-04B §6.

## Outcome

**Durable release.** `FreshAdmissionStore::release_expired_loss` acts once the original grace deadline of a RECONNECTABLE loss has passed on the durable clock. It prepares the FND-04B terminal release from the *current* claim rows:
- Account presence is cleared;
- the Character holder is cleared and the lease generation is kept;
- the successor is decided at the durable time.

It then commits through the existing exact, fenced `release`, which rejects any concurrent change to the session or its claims. A lost acknowledgement is reconciled through `reconcile_lifecycle` and never decided twice. A session that is no longer a reconnectable loss, or whose ownership no longer names it, is `NotApplicable`.

**Channel.** After the durable TERMINAL release, the owner removes the exact actor with `remove_terminal_session`.

**Lifecycle outside the connection budget.** The loss wait and the grace wait used to hold the ended connection's slot. They now run in a separate listener task set that holds no socket, with at most one entry per admitted session. Shutdown cancels this set (addresses review finding 4 on #974).

**Qualification.** The `stage=grace_expiry` seam stage proves:
- the closed and the silent sessions both become TERMINAL after grace;
- the same character is admitted again on a fresh grant (3 admissions);
- the Channel census shows one actor, with no control-loss marks left behind.

Excluded:
- R1b recovery and positive `ClientResume` (PR 5);
- protection during loss;
- the logout block;
- restart recovery of undecided or unexpired losses.
