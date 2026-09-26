---
task_id: OTV2-20260926-native-entry-content-activation
title: Native entry Content activation issuer and Channel pin
mode: IMPLEMENT
status: completed-on-merge
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/native-entry-content-activation-20260926
base_sha: d6c6c18eb1690208213b8871a30c992681ee4f2d
issue: 162
jira: KAN-13
allocation_comment: 5849898542
owned_paths:
  - apps/game-server/migrations/0008_native_entry_content_activation.sql
  - apps/game-server/src/durability/content_activation.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/src/content/activation.rs
  - apps/game-server/src/content/project/native_entry.rs
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - apps/game-server/src/foundation/mod.rs
  - apps/game-server/src/node/serve.rs
  - apps/game-server/src/node/operator_files.rs
  - apps/game-server/src/bin/oteryn-game-ops.rs
  - apps/game-server/src/movement.rs
  - apps/game-server/src/gameplay_transport/qualification.rs
  - apps/game-server/tests/durability_postgres.rs
  - tools/qualification/node_boot/run.sh
  - docs/agents/tasks/archive/OTV2-20260926-native-entry-content-activation.md
---

# Native entry Content activation issuer and Channel pin

Authority: first-entry decision #935, §"Activation issuer and current pin". The owner decided in
this session that the monotonic activation floor is a Game Postgres row per World/Channel scope.
Allocation: #162 comment 5849898542.

## Outcome

- **Control-plane issuance.** Migration `0008` adds the immutable table
  `game_content_activations`, one row per scope and sequence. It also adds the SECURITY DEFINER
  function `game_content_record_activation`, which:
  - requires an exact-scope `game_control_scope_grants` operation 4;
  - accepts an exact replay;
  - refuses with `OTC01` a conflicting replay, a predecessor that is not the scope's current
    sequence, or a sequence that is not newer.

  The newest row of a scope is its current activation and its floor.
- **Assignment-receipt guard.** Operation 4 is grantable, so `0008` also restricts the `0006`
  receipt guard to assignment command kinds 1–3. Without that, a Content-activation grant could
  authorize an immutable kind-4 assignment receipt (independent review P1). A missing grant is a
  definitive refusal (exit code 6).
- **Operator command.** `oteryn-game-ops content activate --world --channel --sequence
  --previous empty|<n> --request <file>`:
  - computes the pair digests and the frame binding from the committed room qualified for that
    World;
  - retains the request file and records it;
  - replays the file exactly on a re-run.
- **Qualified room.** `qualify_native_entry_room(WorldId)` binds the committed source, re-admits
  it natively under the fixed limits, qualifies it and compiles it.
  - `NativeEntryProject` now keeps its qualified frame.
  - `NativeEntryFrameBinding` digests the domain tag, the World, the source manifest digest, the
    map revision and the frame.
- **Node boot.** `activate_native_entry_room` is the sealed production
  `AuthorizedContentGeneration`: an empty start with a boot quiescence proof.
  - It requires the scope's World, exact pair digests and frame binding before staging.
  - `serve` activates the scope's current issuance after assignment and before the Channel
    runtime, listener or control socket exists.
  - A missing, stale or mismatching issuance refuses readiness with exit code 19.
- **Channel pin.** `ChannelRuntimeV1` is created only with a `ChannelContentPin`: the World,
  sequence, pair digests and frame-binding digest. In production that pin comes from the
  consumed activation pin. It is fixed for the runtime's lifetime, and the wrong World refuses.
- **Restart.** A new incarnation starts not-ready and reactivates only the current row.

The layout differs from the allocation in three ways:
- The Postgres tests are in `tests/durability_postgres.rs`, which compiles `durability` directly.
- The activation tests are lib unit tests in `content/activation.rs`, because the boot
  quiescence proof is crate-private.
- `content/mod.rs` needed no change.

## Validation

- Lib unit tests cover:
  - a positive activation;
  - World, digest and frame refusals, including another World's genuine binding;
  - a non-monotonic sequence and a second activation of a controller that is already active;
  - the per-World frame binding;
  - the Channel pin carrying the same digests;
  - a Channel refusing a pin for another World.
- The Postgres floor test runs against PostgreSQL 17.6. It checks:
  - refusal without a grant;
  - an empty start, an exact replay, a conflicting replay and a stale empty start;
  - a stale predecessor, the current-row readback and per-scope isolation;
  - that the rows are immutable.
- Node-boot qualification:
  - an ungranted issuance is refused, and the granted sequence-1 issuance succeeds;
  - an exact replay succeeds, and a stale empty start is refused;
  - the `content_activated` event appears before readiness;
  - a superseding replacement reactivates the current issuance.

The node-boot run uses its fixed test WorldId and is mechanics evidence only. The Platform-issued
WorldId evidence remains `native-entry-room-qualification.yml` (#949).

Excluded:
- position initialization and control (the next #822 child);
- the #642 protocol, #139 limits and reconnect;
- Platform, production and deployment.
