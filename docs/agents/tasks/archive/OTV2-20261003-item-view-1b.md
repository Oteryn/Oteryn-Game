# OTV2-20261003-item-view-1b

```yaml
task_id: OTV2-20261003-item-view-1b
title: "ITEM-VIEW-1b: item handles, domains 9 and 11 and opening a corpse (server side)"
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/item-view-1b-20261003
pr: "the PR named in the #1622 FREEZE_SHA entry"
base_sha: 68c0d7b
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_frozen_at: null
owner: oteryn-hard-worker (CP #1622)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
migration_lease: none
owned_paths:
  - apps/game-server/src/gameplay_transport/item_view.rs
  - apps/game-server/src/gameplay_transport/item_view_tests.rs
  - apps/game-server/src/gameplay_transport/resume.rs
  - apps/game-server/src/gameplay_transport/world_spatial.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/interaction/dispatch.rs
  - apps/game-server/src/interaction/corpse_open.rs
  - crates/protocol-oteryn/src/lib.rs
  - crates/protocol-oteryn/src/world_spatial_entities.rs
  - docs/agents/tasks/archive/OTV2-20261003-item-view-1b.md
public_contracts: []
depends_on: [ITEM-VIEW-1a, CAP-NEG-1, CAP-NEG-RESUME-FALLBACK-1]
blocks: [ITEM-MOVE-1]
cross_repository_coordination_id: null
external_repositories: []
```

Packet: `docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_ITEM_EQUIP_PACKETS_2026-10-03.md` §2.4
(ITEM-MOVE-WIRE-0 §4, server side; ruling §1.1). Also closes the deferred P2s #1703 4175400422 and
4175400425 and the #1711 P2 (capability 12 requires 4).

## Outcome

- `gameplay_transport/item_view.rs`:
  - The handle table issues handles from one counter per GameSession. Handles are monotonic and
    never reused.
  - A handle is live while its item is in at least one view (domain 1 objects, domain 9, domain
    11). An item that leaves every view is dropped, and one that returns gets a new handle.
  - Live handles are bounded by `ITEMV0-RL-03` (301). Over the bound the view is left unchanged.
  - A handle maps only to a server-internal `ItemKey`, so no ItemInstanceId, placement key or row
    reaches the wire.
  - Domain 9 is built from `CharacterBackpack` (`read_character_backpack`) through a fail-closed
    definition-ref resolver. Domain 11 is the one open corpse, which keeps its domain 1 handle.
  - Each delta carries the whole view and is emitted only when the view changed. Revisions advance
    before the write and every snapshot is above the high-water revision, so a revision is never
    reused (FND-02 §15).
- `ItemViewContinuity` (the handle counter, both high-water revisions and the open corpse) is a
  field of `SessionContinuity`:
  - A resume carries it whole.
  - `across_channel_transfer` carries it with the corpse closed.
  - A new connection starts an empty table from the carried counter, so every older handle is
    `STALE`.
- `interaction/corpse_open.rs` holds reach (Chebyshev 1, same floor), the open decision and the
  §4.3 closing triggers. Opening writes nothing.
  - **D133 disclosure, recorded as `PARITY_PENDING`:** a non-owner in reach can see a corpse's
    contents during the exclusivity window but cannot take them.
- `connection.rs`. With capability 4 selected:
  - The join snapshot carries domains 9 and 11 after domain 3. An unreadable backpack fails closed.
  - A carried open corpse is shown again only while it is in reach.
  - `USE` is decoded with `decode_use_intent_target`.
  - An item target resolves its handle: a stale handle is `STALE_STATE` with no read. Otherwise
    the Channel owner observes the target, and the result is `COMMITTED`, `TOO_FAR`,
    `STALE_STATE` or `NOTHING_TO_USE`. After `COMMITTED`, the domain 11 delta follows the result.
  - A step out of reach or to another floor sends the empty domain 11 delta.
  - A committed world-object `USE` (the chest MINT) re-reads the backpack and sends the domain 9
    delta after the result, only when the view changed.
  - Two new authority seams: `observe_character_inventory` and `observe_item_target`. Both
    default to `None`.
- `resume.rs`: with capability 4, the reconnect fence carries the domain 9 and 11 high-water
  revisions, between domains 2 and 13.
- `world_spatial.rs`: with capability 4, domain 1 v2 objects carry their item handle on the
  snapshot and the delta.
- Protocol crate:
  - `ServerAccepted` and `ServerResumeAccepted` refuse a selected set with 4 without 6, or 12
    without 4. This holds on encode and on decode, including ingress.
  - Spatial snapshots and deltas refuse two entities with the same nonzero `item_handle`, on encode
    and decode.
- Capability 4 stays `offered: false`. The tests negotiate it directly. `interaction/dispatch.rs`
  is a generic proposal dispatcher with no `USE` arm, so it needed no change.

## Follow-ups (composition, not this slice)

These are recorded for the composition that offers capability 4 (ITEM-MOVE-1 / VIS-3):

- The production `observe_character_inventory` (`read_character_backpack` with the definition-ref
  mapping) does not exist yet. Neither does `observe_item_target` (corpse contents).
- The death, teleport, logout and corpse-decay hooks that call `SessionItemView::close` are not
  wired. Every trigger is tested at the `SessionItemView` level.
- No channel transfer is composed yet. When one is, it uses
  `ItemViewContinuity::across_channel_transfer`.
- The VIS-3 domain 1 v2 composition calls `SessionItemView::attach_spatial_handles`.

## Tests

- `item_view_tests`:
  - The handle bound at max and max+1.
  - Handles are monotonic and never reused, stale after leaving every view, and the counter
    refuses to wrap.
  - Domain 9 from a `CharacterBackpack`, with no instance id on the wire and fail-closed mapping.
  - Inventory deltas only on change, with monotonic revisions.
  - Resume and channel transfer reissue handles and leave every older one stale.
  - Spatial handles attach and detach.
  - Every `USE` disposition. Opening another corpse closes the first. Every closing trigger.
  - A reconnect shows the corpse only while it is in reach.
- `item_view_tests::connection` runs `serve_admitted` with capability 4 negotiated:
  - The snapshot carries domains 9 and 11 only with capability 4, and fails closed without a
    backpack.
  - Each disposition reaches the wire, with the domain 11 delta after `COMMITTED`.
  - A step out of reach closes the corpse.
  - A committed `USE` sends the inventory delta only on change.
  - A resumed connection reissues handles, and its old handle is `STALE_STATE` with no read.
- `corpse_open`: reach and dispositions, and every closing trigger.
- `world_spatial`: an object carries its handle only with capability 4.
- `resume`: fence order 1, 2, 9, 11, 13.
- Protocol: `acceptance_refuses_a_selected_capability_without_its_requirement` (accepted and
  resume-accepted, encode and decode) and `a_repeated_item_handle_fails_closed_on_encode_and_decode`
  (snapshot and delta).

## Validation

- `cargo fmt --all --check`: pass
- `cargo clippy --locked --all-targets --quiet -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server --quiet`: pass
- `cargo test --locked -p oteryn-protocol-oteryn --quiet`: pass
- `python3 tools/agents/validate_governance.py`: pass
- `python3 -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
