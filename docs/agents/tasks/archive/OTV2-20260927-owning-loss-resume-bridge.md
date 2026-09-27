---
task_id: OTV2-20260927-owning-loss-resume-bridge
title: Durable same-session recovery of an owning-loss GameSession
mode: IMPLEMENT
status: completed-on-merge
repository: Oteryn/Oteryn-Game
base_branch: main
base_sha: 6dd122f2
branch: agent/owning-loss-resume-bridge-20260927
issue: 162
jira: KAN-13
allocation_comment: 5856391446
owned_paths:
  - apps/game-server/src/durability/fresh_admission.rs
  - apps/game-server/tests/durability_postgres.rs
  - docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md
  - docs/agents/tasks/archive/OTV2-20260927-owning-loss-resume-bridge.md
---

# Durable same-session recovery of an owning-loss GameSession

This is #822 stage 2, PR 5a: the durable half of resume during grace. Authority: FND-04B §§11–13, 16 and 20; the plan is on #822 (comment 5856391015).

## Decision

The allocation first proposed bridging the owning-loss receipt into the legacy reconnect journal. That was **rejected during implementation**. The legacy PREPARE writes the protection-continuity row in the connection-generation namespace. An existing regression test deliberately forbids this projection, and that test was kept unchanged.

Foundation already defines the typed `CompleteReconnect` V1 flow for exactly this case. It covers:
- the owning loss;
- the retained budget;
- the recovery credential;
- the proof transition;
- protection activation in the receipt namespace.

What was missing is its durable adapter, and this PR adds it.

## Outcome

**Canonical receipts.** `encode_complete_reconnect` is the canonical, bounded encoding of a same-session recovery operation. It covers the loss, snapshot, budget, candidate, proof transition, FND-02 fence, recovery evidence, claims and credential (with the V1 trust or V2 audit). Immutable PREPARE and COMMIT receipts are keyed by session, loss epoch and attempt, and are compared in full. Fast-reconnect proof delivery and replacement onto a new session are refused (they have no owner yet).

**`FreshAdmissionStore::apply_complete_reconnect`.** Under the admission relation locks it binds:
- the exact durable owning-loss receipt and its decision time;
- the fresh-admission account;
- the exact current session;
- current claim ownership;
- the ready runtime guard;
- the retained budget reconstructed from the receipts and reservations.

It then samples the single decision time and lets the sealed request validate its source.
- **PREPARE** reserves the candidate transport and names the attempt on the session. At most one attempt is prepared, and at most 8 per epoch.
- **COMMIT** requires the exact PREPARE receipt. It consumes the RecoveryGrantNonce and switches the same GameSession to ACTIVE at the candidate generation and transport.

Exact replays return the original decision, and a conflicting receipt is a stored-state error.

**Supporting operations.** `reconcile_complete_reconnect` recovers the outcome of a lost acknowledgement. `recovery_budget` exposes the retained budget to the owning source.

**Postgres test (a real signed recovery JWT).** The owning loss goes through PREPARE (with a replay) and a fresh reauthorization, then COMMIT: ACTIVE at generation 2 on the candidate transport, lease kept, nonce consumed once, the budget restored with the winner committed, and replay and reconcile idempotent. Negatives:
- a forged budget is refused before any write;
- an already consumed nonce refuses the switch and leaves the session RECONNECTABLE;
- a fresh authorization against the resumed session is refused.

## Excluded

- transport `ClientResume`, the runtime source (FND-02 fence, proof owner) and actor rebinding (5b);
- loss after a resume (5c);
- fast-reconnect proof issuance;
- early terminal replacement;
- withdrawing a refused PREPARE (5b adds it together with its caller).
