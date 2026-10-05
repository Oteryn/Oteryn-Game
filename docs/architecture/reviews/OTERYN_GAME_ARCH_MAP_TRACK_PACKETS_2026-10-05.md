# ARCH-MAP-TRACK-0 Map track: viewport budget, first bundle World, client draws real tiles

- Decision: `ARCH-MAP-TRACK-PACKETS-V1`
- Status: **ACCEPTED WHEN THIS DECISION MERGES**, after exact-head validation, the independent
  review on the frozen head and protected integration. Amendment MAPW-A1 (§1.3) was rejected by
  the owner (D730, answer 1b): capability 18 keeps requiring 4 and 6, and MAP-CUTOVER-1b and
  MAP-CLIENT-1 wait for ITEM-MOVE-1 (`OTV2-20261003-item-move-1`, offers capability 4).
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: control plane D725 (#162): packet the map track in parallel with login-first
  (`ARCH-LOGIN-FIRST-PACKETS-V1`, #1815) on disjoint paths: the viewport budget, the cutover and
  the client drawing real tiles.
- Runtime, migration, production and protected-World authority: NONE. Each packet needs its own
  #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Implementation brief

1. **MAP-VIEWPORT-PERF-1 (impl).** Bring the server cost of one 18x14 domain-17 snapshot over
   every floor in view (compose, assign handles, encode) from about 3.5-4.7 ms to the
   `MAP01-VIEWPORT-US` gate, 100 us p99, with the wire bytes unchanged. Flat arrays indexed by
   window offset replace the per-call maps; buffers are reused; the diff compares in place (§2.2).
2. **MAP-CUTOVER-1a (hard).** A testing or preproduction node boots one World from a pinned
   bundle and walks it: config pins, `map::load`, an empty overlay, a third collision-index
   variant over the bundle, a configured start. It serves no session until 1b, because a bundle
   World needs capability 18 (§2.3).
3. **MAP-CUTOVER-1b (hard, protocol).** The bundle World serves domain 17: a production
   `MapFacts`, `observe_world_map`, capability 18 offered on a bundle World only, and
   CAPABILITY_MISMATCH without it. Waits for ITEM-MOVE-1 (capability 4) (§2.4).
4. **MAP-CUTOVER-1c (hard).** The old SOCIAL-MAP §2.8, renamed: Ground persistence and the
   reset-driven first cutover after MAP-OVERLAY-1c. Unchanged (§2.5).
5. **MAP-CLIENT-1 (impl), re-issued.** The client selects capability 18 and draws the real
   tiles in place of the N2N3-1 placeholder grid; the ground-speed source switches on server and
   client together (§2.6).

Order: MAP-VIEWPORT-PERF-1 and MAP-CUTOVER-1a in parallel -> MAP-CUTOVER-1b (also after login
N8-1 and ITEM-MOVE-1) -> MAP-CLIENT-1 (also after login N2N3-1). MAP-CUTOVER-1c follows MAP-OVERLAY-1c on its own
track. No packet here owns a login-first path except where §2.1 orders it after the login packet.

Owner question (§1.5): 1. capability 18 without capability 4. Answered 1b (D730): rejected.

## 1. Rulings

### 1.1 The viewport gate is the whole snapshot

The registry offer gate of capability 18 already binds "the composition plus encode p99 of an
18x14 viewport over 8 floors" to `MAP01-VIEWPORT-US` (100 us). That is the target of
MAP-VIEWPORT-PERF-1: compose, assign handles and encode one full snapshot, measured with the
release `map_viewport_measure` test on the reference node class (MAP-SPIKE-0, 4 vCPU), p99 over
at least 20,000 seeded viewports on floors -7..=0. Today: p50 2.04 ms, p99 3.47 ms (MAP-WIRE-2),
of which planning 1.09 ms, handles and tiles 0.74 ms, encode 0.43 ms. The base-only composition
of MAP-LOAD-1 is 72-83 us p99, so the cost is in allocation and copying, not in the base reads.
A delta is measured and recorded too; it has no separate gate. No registry row is added. If the
optimised snapshot is still over 100 us p99, the worker records the stage split and returns
`BLOCKER` to the architect; revising the budget is an owner decision (owner answer 8a set it).

### 1.2 A bundle World before Ground persistence

ADR-0021 D193: a World runs either the `native_entry_room` fixture or one bundle, never both.
MAP-CUTOVER-1a/1b serve a bundle World on testing and preproduction only, before MAP-OVERLAY-1c,
so there is no Ground write path on it:

- the overlay is empty at boot and stays empty (no item view, no Ground item, no drop);
- nothing durable names the bundle, so a restart rebuilds the same World from the config pins;
- `readiness.map_revision` must equal `overlay::map_revision(base)` (`sha256:<digest>`), and
  boot refuses on any mismatch; a production World (`BundlePins.production`) is refused;
- admission places the actor at the configured start, because there is no durable Character
  position (CHAR-POSITION-0 is not accepted). Boot refuses a start tile that is not enterable;
- the entry-room door and chest are not placed; their targets refuse as not found and do not
  block. Spell area placement and field items resolve against the bundle collision index or
  refuse; a feature that needs more than refusal returns to the architect;
- the ground speed stays the Engineering 150 source on server and client, set at one switch
  point in `map/boot.rs`, until MAP-CLIENT-1 (ADR-0021 MAP-LOAD-1 amendment, ARCH-MAP-WIRE §1.6).

A tile is enterable on a bundle World iff its ground is walkable (`TileView::walkable`), no
entry is Terrain kind `wall`, and no base entry's item definition has `block_solid`. The rule is
a third variant `Bundle` of the sealed `NativeMovementCollisionIndex`, so `step_in_channel` and
its tests are unchanged.

MAP-CUTOVER-1c (the old SOCIAL-MAP §2.8) stays the first cutover with durable Ground items: the
§4.7 reset, the overlay rebuild and the position fallback.

### 1.3 Amendment MAPW-A1: capability 18 without capability 4 (rejected)

**Rejected by the owner (D730, answer 1b).** Capability 18 keeps requiring 4 and 6, the
contract §3 and §4 are unchanged, and there is no `display_only` fallback for item-handle
entries. MAP-CUTOVER-1b and MAP-CLIENT-1 depend on ITEM-MOVE-1 (`OTV2-20261003-item-move-1`),
which offers capability 4. The proposal is kept below as the record of what was rejected.

Capability 18 requires 4 only because entries with origin `item_handle` carry capability-4
handles. Capability 4 is not offered and has no implementation packet (ITEM-MOVE-1), so no
client can select 18 and a bundle World admits nobody. Proposed, effective on owner answer 1a:

- capability 18 requires 6 only;
- with 4 not selected, the server sends every entry that would have origin `item_handle` as
  `display_only`; no handle is assigned and none is counted against `MAX_MAP_VIEW_HANDLES`;
- with 4 selected, contract §4 is unchanged.

Capability 18 was never offered, so no peer has negotiated it. MAP-CUTOVER-1b changes the
registry `requires`, `CAPABILITY_WORLD_MAP_VIEW_REQUIRES` and the module comment in one PR; the
contract text is amended here (§3 of the MAP-WIRE-1 contract). With answer 1b the bundle World
waits for ITEM-MOVE-1 and 1b does not offer 18.

### 1.4 Checklist before freeze

1. Amendments in the owning documents: MAP-WIRE-1 contract §3 (MAPW-A1 marked rejected),
   SOCIAL-MAP §2.8 and ARCH-MAP-WIRE §2.3 carry pointer notes to this decision.
2. Concurrency: the bundle base is an immutable `Arc<WorldBase>`; the overlay and the map view
   state live in the Channel-owner state and are read under the same lock `step` takes, so a
   view is composed against one overlay state and one actor position.
3. Restart-sufficient: the config pins (digest, schema versions, content revision, start) and
   `readiness.map_revision` rebuild the same World; nothing else names the bundle (§1.2).
4. Typed references: `BundlePins`, `WorldBase` digest, `map_revision` string, `TilePos`; no
   free-text join.
5. Older peers: capability 18 was never offered; its first offer requires 4 and 6 as registered.
6. Split work: 1a lands a bundle World that serves nobody (fail-closed), 1b opens it; each is
   observable alone by its tests, and a revert of 1b returns to the 1a state.

### 1.5 Owner question (batched to the control plane)

1. Capability 18 without capability 4 (§1.3). a) Amend MAP-WIRE-1 §3: 18 requires 6 only, and
   item-handle entries are `display_only` when 4 is not selected (recommended: the bundle World
   becomes playable now, and ITEM-MOVE-1 later turns on handles with no wire change). b) Keep
   `[4, 6]` and wait for ITEM-MOVE-1.

   Answer (D730): **1b**.

## 2. Packets

### 2.1 Order and shared files

| Packet | After | Shares with |
| --- | --- | --- |
| MAP-VIEWPORT-PERF-1 | main | `world_map.rs` with 1b |
| MAP-CUTOVER-1a | main | `serve.rs`, `map/boot.rs` with 1b |
| MAP-CUTOVER-1b | PERF-1, 1a, login N8-1, ITEM-MOVE-1 | `gameplay_transport/mod.rs`, `connection.rs`, registry with N8-1; `map/boot.rs` with MAP-CLIENT-1 |
| MAP-CLIENT-1 | 1b, login N2N3-1 (so N4-1, N5-1), N8-1, ITEM-MOVE-1 | `crates/session/src/lib.rs` with N8-1/N4-1, `apps/client/src/{lib,input,play}.rs` with N2N3-1 |

No two open packets hold the same path; the control plane allocates in this order.

### 2.2 MAP-VIEWPORT-PERF-1 (impl worker)

```yaml
task_id: OTV2-20261005-map-viewport-perf-1
decision: ARCH-MAP-TRACK-PACKETS-V1 §1.1
depends_on: []
worker: oteryn-impl-worker
review: Codex, on the frozen head
branch: agent/map-viewport-perf-1-20261005
base: main
owned_paths:
  - apps/game-server/src/map/view.rs
  - apps/game-server/src/gameplay_transport/world_map.rs
  - apps/game-server/src/gameplay_transport/world_map_tests.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json   # MAP01-VIEWPORT-US notes only
  - docs/agents/evidence/MAP-VIEWPORT-PERF-1-viewport.md
  - docs/agents/tasks/archive/OTV2-20261005-map-viewport-perf-1.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server world_map
  - cargo test --locked -p oteryn-game-server map::
  - cargo test --release --locked -p oteryn-game-server map_viewport_measure -- --ignored --nocapture
  - python tools/agents/validate_governance.py
  - git diff --check
```

- **Scope:** no per-tile `Vec`, no `BTreeMap` keyed by position, no position `Vec` from
  `view::window`, no roof sort per call; the window is a fixed array of 18x14 per floor indexed
  by offset; composed entries go into one reused buffer with per-tile ranges; handle assignment
  and the budget ordering reuse their buffers; the sent view is a flat array, and a delta
  compares the new tile against the sent one in place, cloning nothing that did not change.
- **Acceptance:** a golden test over a fixed seed set: every snapshot and delta byte-identical
  before and after; the measure test prints p50/p99 for snapshot and delta and the stage split;
  snapshot p99 <= 100 us on the reference node (§1.1); the evidence file records the machine,
  the seeds, the before and after numbers; the registry notes cite it.
- **Not in scope:** any wire change, the budget value, a new registry row, overlay storage.

### 2.3 MAP-CUTOVER-1a (hard worker)

```yaml
task_id: OTV2-20261005-map-cutover-1a
decision: ARCH-MAP-TRACK-PACKETS-V1 §1.2; ADR-0021 §4.2, D193; MAP-LOAD-1 amendment
depends_on: []
worker: oteryn-hard-worker
review: hard review (Codex, final frozen head)
branch: agent/map-cutover-1a-20261005
base: main
owned_paths:
  - apps/game-server/src/node/config.rs                # optional [world_bundle]: path, pins, start
  - apps/game-server/src/node/serve.rs                 # boot the bundle World instead of the fixture room
  - apps/game-server/src/map/boot.rs                   # new: load, empty overlay, checks, speed switch point
  - apps/game-server/src/map/mod.rs                    # the mod boot line only
  - apps/game-server/src/content/native_cell_lookup.rs # the Bundle variant
  - apps/game-server/src/content/project/native_entry.rs   # the bundle movement-cells constructor
  - apps/game-server/tests/map_cutover_boot.rs
  - docs/agents/tasks/archive/OTV2-20261005-map-cutover-1a.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server map_cutover
  - cargo test --locked -p oteryn-game-server native_cell
  - cargo test --locked -p oteryn-game-server node::config
  - cargo run --locked -p oteryn-architecture-check
  - python tools/agents/validate_governance.py
  - git diff --check
```

- **Scope:** §1.2. Without `[world_bundle]` the node is byte-for-byte today's fixture node.
  With it: `map::load` with the pins; `production = true` is a config reject; boot refuses a
  digest, schema, content-revision or `readiness.map_revision` mismatch and a non-enterable
  start; the Channel's movement cells use the `Bundle` index; the ground-speed source is
  Engineering 150. After the boot checks `serve.rs` stops before `serve_gameplay` with a typed
  refusal (a bundle World needs capability 18, MAP-CUTOVER-1b) and opens no listener.
- **Acceptance:** a test bundle boots and a Channel-level walk steps onto a walkable ground and
  is refused onto a non-walkable ground, a `wall` entry and a `block_solid` item; each boot
  refusal above has a test; a restart with the same config rebuilds an equal World; a fixture
  config still serves the entry room; a bundle config passes the boot checks, then refuses to
  serve.
- **Not in scope:** domain 17, capability changes, durable Ground, the reset, durable position,
  production or protected Worlds, any `gameplay_transport/` path.

### 2.4 MAP-CUTOVER-1b (hard worker, protocol)

```yaml
task_id: OTV2-20261005-map-cutover-1b
decision: ARCH-MAP-TRACK-PACKETS-V1 §1.2; MAP-WIRE-1 contract (accepted; MAPW-A1 rejected)
depends_on: [OTV2-20261005-map-viewport-perf-1, OTV2-20261005-map-cutover-1a, login N8-1, OTV2-20261003-item-move-1]
worker: oteryn-hard-worker
review: hard and protocol review (Codex, final frozen head)
branch: agent/map-cutover-1b-20261005
base: main after its dependencies merge
owned_paths:
  - apps/game-server/src/map/facts.rs                  # new: production MapFacts over bundle and content
  - apps/game-server/src/map/mod.rs                    # the mod facts line only
  - apps/game-server/src/map/boot.rs                   # hands the view owner to the Channel
  - apps/game-server/src/node/serve.rs                 # remove the 1a serve refusal
  - apps/game-server/src/gameplay_transport/mod.rs     # observe_world_map, the bundle-World offer
  - apps/game-server/src/gameplay_transport/connection.rs  # domain 17 on a bundle World
  - apps/game-server/src/gameplay_transport/capabilities.rs
  - apps/game-server/src/gameplay_transport/world_map.rs   # production facts wiring only
  - apps/game-server/src/gameplay_transport/world_map_tests.rs
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json    # capability 18 offer_gate only
  - apps/game-server/tests/map_cutover_view.rs
  - docs/agents/tasks/archive/OTV2-20261005-map-cutover-1b.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked --workspace --all-targets -- -D warnings
  - cargo test --locked -p oteryn-protocol-oteryn world_map
  - cargo test --locked -p oteryn-game-server
  - cargo run --locked -p oteryn-architecture-check
  - python tools/agents/validate_governance.py
  - python tools/repository/validate_repository_policy.py
  - git diff --check
```

- **Scope:** `MapFacts` resolves the bundle palette and the served item definitions
  (appearance, `blocks_projectile`, pickupable, bound). `ComposedFreshAdmission` implements
  `observe_world_map`. A new `BUNDLE_WORLD_OFFERED_CAPABILITIES` (the production set, which
  holds 4 after ITEM-MOVE-1, plus 18 requiring 4 and 6) is returned by `offered_capabilities()` only on a bundle World; the registry entry
  stays `offered: false` and its `offer_gate` names the bundle-World offer. `serve.rs` serves a
  bundle World (the 1a refusal is removed). On a bundle World an
  admission without 18 is refused with CAPABILITY_MISMATCH; a fixture World never offers 18 and
  keeps domain 2. Entry origins follow contract §4 unchanged.
- **Acceptance:** a production-path admission on a test bundle selects 18 and receives a
  snapshot equal to `plan()` for that position; a step sends the delta; a client without 18 is
  refused; a fixture World offers exactly the production set; a test keeps the bundle set equal
  to the production set plus 18; an admission selecting 18 without 4 is refused (`requires`);
  the registry and crate requires stay `[4, 6]`; the snapshot p99 of §2.2 still holds with the production facts.
- **Not in scope:** capability 4 itself (ITEM-MOVE-1), Ground items, any client path.

### 2.5 MAP-CUTOVER-1c (hard worker)

The SOCIAL-MAP §2.8 packet, unchanged except its task id `MAP-CUTOVER-1c` and its base: main
after MAP-OVERLAY-1c and MAP-CUTOVER-1b. It adds Ground persistence, the §4.7 reset and the
position fallback to the bundle World of 1a and 1b.

### 2.6 MAP-CLIENT-1 (impl worker), re-issued

```yaml
task_id: OTV2-20261005-map-client-1
supersedes: OTV2-20261004-map-client-1 (ARCH-MAP-WIRE §2.3; never allocated)
decision: ARCH-MAP-WIRE-1 §1.1-§1.6, §2.3; ARCH-MAP-TRACK-PACKETS-V1 §2.6
depends_on: [OTV2-20261005-map-cutover-1b, login N2N3-1, login N8-1, OTV2-20261003-item-move-1]
worker: oteryn-impl-worker
review: Codex, on the frozen head
branch: agent/map-client-1-20261005
base: main after its dependencies merge
owned_paths:
  - crates/session/src/lib.rs                # select 4 and 18, decode domain 17, re-export the view types
  - apps/client/src/map_view.rs              # new: snapshot, deltas, window, resync
  - apps/client/src/map_draw.rs              # new: tile stacks to quads through oteryn-client-assets
  - apps/client/src/play.rs                  # draw the map view instead of the placeholder grid
  - apps/client/src/input.rs                 # step timing from the tile ground speed; targeting
  - apps/client/src/lib.rs                   # module lines and --assets
  - apps/client/Cargo.toml                   # oteryn-client-assets
  - apps/game-server/src/map/boot.rs         # the ground-speed switch point only: MapGroundSpeed
  - apps/game-server/src/movement/speed.rs   # only if the switch needs it
  - docs/agents/tasks/archive/OTV2-20261005-map-client-1.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked --workspace --all-targets -- -D warnings
  - cargo test --locked -p oteryn-session
  - cargo test --locked -p oteryn-client
  - cargo test --locked -p oteryn-game-server
  - cargo run --locked -p oteryn-architecture-check
  - python tools/agents/validate_governance.py
  - git diff --check
```

- **Scope and acceptance:** ARCH-MAP-WIRE §2.3 Builds and Acceptance, with two changes: the
  map is drawn in `play.rs` (N2N3-1) instead of `scene.rs`; the end-to-end check runs on the
  1a/1b bundle World. Every targeting case, `item_handle` included, is as in §2.3, with the
  capability-4 handles of ITEM-MOVE-1. The ground-speed switch moves server and client in
  this one PR: a step onto a non-150 ground takes the same duration on both, and the fixture
  World keeps 150 on both.
- **Not in scope:** ARCH-MAP-WIRE §2.3 Not in scope.

## 3. Rejected options

- A new budget row for the full snapshot with 100 us left for composition only: the registry
  offer gate already names composition plus encode; splitting it would weaken the accepted gate.
- One MAP-CUTOVER-1 for boot and domain 17: two hard reviews of disjoint risk (boot and
  collision; protocol and offer) are smaller and each fails closed alone.
- Waiting for MAP-OVERLAY-1c before any bundle World: blocks the playable path on house runtime
  work that a World without Ground writes does not need.
- MAPW-A1, capability 18 without capability 4 with `display_only` item-handle entries: rejected
  by the owner (D730, answer 1b); the bundle World waits for ITEM-MOVE-1.

## 4. Decision test

The decision holds if: the snapshot is within 100 us p99 with unchanged bytes; a preproduction
node boots a pinned bundle, refuses every pin mismatch and walks only enterable tiles; a client
selecting 18 receives the bundle World's tiles and draws them in place of the placeholder grid;
and no production World, durable Ground item or older peer is touched before MAP-CUTOVER-1c.
