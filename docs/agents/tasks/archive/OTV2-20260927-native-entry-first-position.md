---
task_id: OTV2-20260927-native-entry-first-position
title: Native entry first-entry position initialization
mode: IMPLEMENT
status: completed-on-merge
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/native-entry-first-position-20260927
base_sha: 139202f1aa790e573c0ae5d41f4e529fdc503016
issue: 162
jira: KAN-13
allocation_comment: 5853264702
owned_paths:
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - apps/game-server/src/foundation/mod.rs
  - apps/game-server/src/content/activation.rs
  - apps/game-server/src/content/project/native_entry.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/qualification.rs
  - docs/agents/tasks/archive/OTV2-20260927-native-entry-first-position.md
---

# Native entry first-entry position initialization

Authority: the first-entry decision #935, §"Position write and retry". Allocation: #162 comment
5853264702.

## Outcome

- **Start cell.** The start is the accepted `oteryn:cell/entry-start` (#935/#940). The qualified
  source attests that it exists and is Walkable (`NativeEntryProject::entry_start`). The
  activation pin carries it together with the map-revision digest into the Channel's fixed
  `ChannelContentPin`.
- **Position context.** A position is bound to the Channel's pin. The pin holds the full binding
  once. Each slot keeps a compact per-runtime reference: the frame-binding and map-revision
  digest prefixes and the activation sequence. The measured 192-byte slot footprint (#912) is
  unchanged.
- **Initialization.** `ChannelRuntimeV1::initialize_first_entry_position` is the single
  Channel-owned write for a committed, unpositioned player actor.
  - An exact retry reconciles the completed initialization without a write.
  - An actor positioned under another context gets no replacement.
  - A stale or removed actor gets no write.
- **Server Seam.** After the durable COMMIT and the runtime actor-slot COMMIT, the seam
  revalidates the current state:
  - the current GameSession is owned by this socket: active, at the same connection generation
    and transport, with no control loss;
  - the current Character lease matches the committed character and lease generation;
  - the World eligibility matches;
  - the current runtime scope and ownership generation equal the runtime binding;
  - the current assignment still matches.

  Only then does the seam initialize. Any failure writes nothing and fabricates no rollback. The
  actor stays unpositioned, so it is not input-eligible.

## Validation

- Lib unit tests:
  - the write lands once at the start (0,0,0) at revision 1, and an exact retry is `Reconciled`;
  - an actor positioned under another context and a stale actor are refused without a write;
  - the activation pin carries the start.
- The Server Seam qualification requires both committed player actors to be positioned at the
  start under the pinned context.

Excluded:
- movement input (the #642 and #139 owner decisions are pending);
- reconnect, visibility, and relocation or spawn activation.
