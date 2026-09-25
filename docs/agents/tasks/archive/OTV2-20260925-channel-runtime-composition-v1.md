---
task_id: OTV2-20260925-channel-runtime-composition-v1
title: Channel runtime composition v1
mode: IMPLEMENT
status: completed-on-merge
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/channel-runtime-composition-v1-20260925
base_sha: d1a236f00014d56bfaa2df9a81fa59a2cacb2b88
issue: 162
jira: KAN-26
allocation_comment: 5838196310
allocation_extensions:
  - 5838407976  # tools/qualification/node_boot/run.sh
  - 5838711560  # writer succession; compile-required test fixtures
writer_succession: 5838711560  # owner-declared hung ChatGPT writer replaced by the #162 control-plane session from 0ddeb503
pr: 913
owned_paths:
  - apps/game-server/src/node/config.rs
  - apps/game-server/src/node/serve.rs
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - apps/game-server/src/foundation/mod.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - docs/agents/tasks/active/OTV2-20260925-channel-runtime-composition-v1.md
  - docs/agents/evidence/OTV2-20260925-channel-runtime-composition-v1.md
  - tools/qualification/node_boot/run.sh
  - apps/game-server/src/gameplay_transport/qualification.rs
  - apps/game-server/src/foundation/movement_static_kernel_structural_tests.rs
---

# Channel runtime composition v1

Protected prerequisite: PR #912, decision
`RUNTIME-ACTOR-FIRST-SLICE-NONPRODUCTION-BOUND-V1`.

## Exact outcome

- explicit required non-production config bound: 131072 actors/Channel;
- consume the committed current Channel assignment into one runtime carrier before readiness;
- bind provenance to scope, node incarnation, ownership generation and source revision;
- no second actor/session map: slot-local GameSession binding plus ExactActorRef only;
- reserve actor capacity before fresh durable session commit, commit the slot only after durable success;
- definitely absent/rejected durable outcome rolls reservation back;
- genuinely ambiguous outcome stays fail-closed and retains the reservation rather than fabricating success;
- ordinary disconnect is not terminal/control-loss and does not remove the actor;
- exact-ref + GameSession terminal cleanup capability is provided but not activated without an authoritative terminal source.

Production capacity remains UNKNOWN / FAIL_CLOSED. No registry or deployment mutation.

## Validation

Candidate-specific evidence starts only after the final authoring head is frozen.
Required: focused tests, fmt, strict Clippy, affected Rust, node boot path, governance,
architecture semantic audit, full game-gate, independent exact-head review, governed
Merge Queue and protected-main readback.
