---
task_id: OTV2-20260926-native-entry-source-consumer-v1
title: Native entry-room source consumer v1
mode: IMPLEMENT
status: completed-on-merge
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/native-entry-source-consumer-v1-20260926
base_sha: 4f22d85aed9e4c6a0ba99b2c57253541d2651bf4
issue: 162
jira: KAN-13
allocation_comment: 5849195233
owned_paths:
  - apps/game-server/src/content/project/native_entry.rs
  - apps/game-server/src/content/project.rs
  - apps/game-server/src/content/project/v2.rs
  - apps/game-server/src/content/project_fs.rs
  - apps/game-server/src/content/mod.rs
  - apps/game-server/tests/content_native_entry.rs
  - docs/agents/tasks/archive/OTV2-20260926-native-entry-source-consumer-v1.md
---

# Native entry-room source consumer v1

Authority: `NATIVE_ENTRY_SOURCE_QUALIFICATION_V1` (#937) and the owner-accepted
`NATIVE-ENTRY-ROOM-PRODUCT-BINDINGS-V1` (#940).

## Outcome

- Explicit native admission (`capture_native_entry_project`, `ProjectSnapshot::parse_native_entry`)
  selects the native root profile and declarations schema before parsing; ordinary v1/v2 capture
  refuses the native variant and native admission refuses ordinary projects.
- Closed, deny-unknown `native_first_entry` overlay with no null or default.
- Lowering to one `FirstProductionContentSource`: exact typed joins, single World/frame/contract,
  three cells bijective with three Terrain placements, relocation/spawn endpoints in those cells,
  one Damage Effect sharing the XP formula profile, three distinct presentations, no Reference
  record outside the graph, licensing `oteryn-original-preproduction`, zero imports.
- The #940 limits are the fixed `native_entry_first_slice_limits()`; callers do not choose them.
- Canonical native writer (`CanonicalProjectDocuments::from_native_entry_draft`) re-admits its
  output through the native parser.
- `compile_first_production(OrdinaryRelease)` yields a deterministic pair.

Excluded: committed room source bytes, a real WorldId, activation, position, control, Server
Seam, registry, protocol, Cargo and workflows. The task ends on merge, so this packet is archived
in the PR.
