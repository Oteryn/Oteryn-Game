---
task_id: OTV2-20260927-liveness-control-loss
title: Liveness probe/ack and control-loss detection on admitted connections
mode: IMPLEMENT
status: completed-on-merge
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/liveness-control-loss-20260927
base_sha: 87d01a9b
issue: 162
jira: KAN-13
allocation_comment: 5854581115
owned_paths:
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/tcp_tls.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md
  - docs/agents/tasks/archive/OTV2-20260927-liveness-control-loss.md
---

# Liveness probe/ack and control-loss detection

Authority:
- owner decision `DISCONNECT-PROTECTION-V1` §1 (#822 comments 5854525644 and 5854565314), provisional values;
- FND-02 §17;
- FND-04B §4.

This is PR 1 of 6 in #822 stage 2.

## Outcome

- **Probe cadence.** An admitted, positioned connection sends a `LivenessProbe` one cadence after its baseline snapshot, and then every cadence. Probe IDs are monotonic from 1 and never wrap; exhaustion ends the transport.
- **Acks.** Only the ack of the current outstanding probe, on the current generation, clears the missed count.
  - A late ack of an older probe restores nothing.
  - An ack of a probe never sent closes the connection with `InvalidWireIdentifier`.
- **Control loss.** After `missed_limit` consecutive unanswered probes, the transport ends as `AdmittedThenControlLost`. No durable session state changes: `ControlLossEpoch`, grace and cleanup are PR 4.
- **Cancel-safe reads.** `FrameReader` reads frames incrementally, so the cadence can race reads without losing partial frame bytes. The race is written with `poll_fn`, because the vendored tokio has no `macros` feature.
- **Registry rows:**
  - `FND04B-LIVENESS-IDLE-PROBE-MS` = 5000 and `FND04B-LIVENESS-IDLE-MISSED` = 3, consumed now;
  - `FND04B-LIVENESS-COMBAT-PROBE-MS` = 1000 and `FND04B-LIVENESS-COMBAT-MISSED` = 2, registered ahead of the combat state.

  A test binds the code constants to the rows.
- **FND-04B §4** records the provisional values and the ack rules.

Tests:
- the probe/ack state machine: current-probe-only, late ack, unsent ID, reset after an answer, exhaustion;
- a silent client losing control after exactly three probes;
- an answering client keeping control and stepping between probes;
- an ack of an unsent probe closing the connection;
- partial frame bytes surviving cancelled reads.

Excluded:
- protection during loss;
- the logout block;
- grace and cleanup;
- positive `ClientResume`;
- combat-state cadence selection;
- client code (the client answers from its game loop).
