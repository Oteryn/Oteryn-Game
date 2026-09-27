---
task_id: OTV2-20260927-first-control-step-m2
title: First-control step M2 - Server Seam composition and E2E proof
mode: IMPLEMENT
status: completed-on-merge
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/first-control-step-m2-20260927
base_sha: a56d0b416a6b122c27f6b25018b11d8f1ea0d32f
issue: 162
jira: KAN-13
allocation_comment: 5854087291
owned_paths:
  - apps/game-server/src/content/activation.rs
  - apps/game-server/src/content/project/native_entry.rs
  - apps/game-server/src/content/static_cell_engine.rs
  - apps/game-server/src/foundation/protocol.rs
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/qualification.rs
  - apps/game-server/src/gameplay_transport/world_spatial.rs
  - apps/game-server/src/node/serve.rs
  - docs/agents/tasks/archive/OTV2-20260927-first-control-step-m2.md
---

# First-control step M2: Server Seam composition and E2E proof

Authority:
- #642 `FIRST-CONTROL-WIRE-V1`, owner acceptance 5853424280;
- #139 `FIRST-CONTROLLED-STEP-LIMITS-V1`, owner acceptance 5853424970;
- critical path #822.

Allocation: #162 5854087291. Builds on M1 (#958).

## Outcome

- **Movement cells of the active generation.** The qualified native entry room also builds an
  `EngineeringStaticCellIndex` from its own source cells. That index is scoped to the room's
  World, coordinate frame, map revision, server generation digest and content lock.
  Activation hands it to the Channel together with the Channel pin
  (`into_channel_parts`), so Movement and the pin come from one qualified generation.
- **Admitted session loop.** When admission leaves the actor `Positioned` or `Reconciled`,
  `serve_admitted` does the following:
  - It sends the domain-1 baseline as a single-chunk snapshot (target sequence 0, revision 1)
    before any command.
  - It then serves `ClientCommand`s in strict command-id order. Every command gets exactly one
    server-sequenced `CommandResult`.
  - A `MOVED` step is followed by one `StateDelta` (revision `n -> n+1`) with the own-actor
    position.
  - A step to a blocked or absent cell gets `BLOCKED`.
  - An unknown command type, a malformed step payload or a stale binding gets `REJECTED`, with
    no effect.
  - A stale generation, a non-command message or a command-id gap closes the connection with
    the matching FND-02 `ProtocolError`. A gap names the offending and the expected command
    ID. A lower (replayed) ID is never re-executed; it closes with `COMMAND_OUTCOME_EXPIRED`,
    because no outcome is retained.
  - An unregistered command type gets REJECTED with no type-owned payload.

  Actors without a position keep the previous hold behaviour.
- **Movement composition.** `ComposedFreshAdmission::step` runs one `MovementOwnerTurn` with a
  single-input budget (`MOVE-RL-02` = 1) over the Channel runtime. The selection is the actor's
  pinned position context and the active room's cell scope. A read position whose context is
  not the Channel pin is never observed or moved. Every step also checks that the cells belong
  to the pinned generation: the same World and server artifact digest.
- **Qualification.** The Server Seam and node-boot `stage=admission` runs this whole sequence
  on one connection and compares every frame byte for byte:
  1. baseline (0,0,0) at revision 1;
  2. east `MOVED` to (1,0,0) at revision 2;
  3. west `MOVED` back to (0,0,0) at revision 3;
  4. north `BLOCKED` (Blocked cell);
  5. south `BLOCKED` (outside the room);
  6. unknown command type `REJECTED`;
  7. command-id gap `CommandSequenceGap`.

  It then checks that exactly one GameSession committed.
- Unit tests drive `serve_admitted` over an in-memory stream. They cover sequencing, the
  delta after a move, the empty payload for an unregistered type, expiry of a replayed ID,
  and the close on a gap, a non-command message or a stale generation.
- The M1 `dead_code` allowances on the composed server codecs are removed. The client-side
  codecs stay test-only on the server.

Excluded:
- control loss and grace;
- positive `ClientResume`;
- other actors' visibility;
- relocation, timing and speed;
- durable position persistence of steps;
- client UI.
