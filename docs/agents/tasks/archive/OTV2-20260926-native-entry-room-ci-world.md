---
task_id: OTV2-20260926-native-entry-room-ci-world
title: Native entry room bound to a CI-issued Platform WorldId
mode: IMPLEMENT
status: completed-on-merge
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/native-entry-room-ci-world-20260926
base_sha: f6b267126d6a4ee505f614a7b740ab16aa86b7b6
issue: 162
jira: KAN-13
allocation_comment: 5849608369
owned_paths:
  - apps/game-server/src/content/project/native_entry.rs
  - apps/game-server/src/content/project/native_entry_room.json
  - apps/game-server/tests/content_native_entry_room.rs
  - tools/qualification/native_entry_room/run.sh
  - .github/workflows/native-entry-room-qualification.yml
  - docs/agents/tasks/archive/OTV2-20260926-native-entry-room-ci-world.md
---

# Native entry room bound to a CI-issued Platform WorldId

Authority: `NATIVE_ENTRY_SOURCE_QUALIFICATION_V1` (#937), `NATIVE-ENTRY-ROOM-PRODUCT-BINDINGS-V1`
(#940) and the owner decision recorded in #162 allocation 5849608369: the canonical WorldId is
issued per qualification run by the Platform World Registry disposable issuer
(Oteryn-Platform #1417, `eb65ce453949a352fdc0e3ed0132d2cde0084fb7`) and is never committed.

## Outcome

- The committed room source `native_entry_room.json` (`OTERYN_NATIVE_ENTRY_ROOM_SOURCE/v1`)
  carries the #940/#935 room and no WorldId.
- `native_entry_room_documents(WorldId)` binds that source to one WorldId and writes the native
  project through `from_native_entry_draft`, which re-admits it through the native parser. A
  WorldId already present in the source refuses.
- `tools/qualification/native_entry_room/run.sh` does the following:
  - builds Platform at the pinned SHA;
  - provisions one local World row in a disposable SQLite fixture, which the issuer guard requires;
  - runs `game-auth:native-topology:issue`;
  - passes the receipt to the ignored test, which checks the receipt shape, UUIDv7 IDs and
    distinct IDs;
  - binds the room, captures it natively twice and compiles a deterministic ordinary-release
    pair.
- The workflow `native-entry-room-qualification.yml` runs on content and harness changes.

Excluded:
- runtime activation, position and control (next #822 child);
- the protocol (#642) and Movement limits (#139);
- Platform writes, production and custody evidence.

The remaining #946 P2 max/max+1 oracles stay deferred. The task ends on merge, so this packet
is archived in the PR.
