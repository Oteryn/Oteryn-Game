# Architect batch: core loop client packets (chat, attack, items, item use)

- Batch: `ARCH-CORE-LOOP-PACKETS-2` part B (D486 item 3)
- Status: **ACCEPTED WHEN THIS DECISION MERGES** for the rulings, lane and packets below. They
  implement accepted semantics only: FND-02 §14 and §15, CHAT-0 §7 as registered by CHAT-1b-1,
  ITEM-MOVE-WIRE-0 §4, ITEM-USE-0 §3, ATTACK-0 §3 as leased in part A (#1735), and owner
  decision D93.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: control plane D486 item 3. It gives the CHAT-CLIENT-1, ATTACK-CLIENT-1, ITEM-CLIENT-1
  and ITEM-USE-CLIENT packets on top of CLIENT-NEG-1, with the client lane's serialization.
- Amends, in this PR: `OTERYN_GAME_ARCH_BATCH_ITEM_EQUIP_PACKETS_2026-10-03.md` §2.7 and §2.7a.
- Runtime, migration, wire and production authority: NONE. No packet here needs a migration, a
  new number or a registry change. Each packet needs its #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Findings on `main` (8a005cf6)

1. **The production client cannot enter gameplay.** In `apps/client`,
   `request_gameplay_entry()` returns `NativeProtocolUnavailable`. ADR-0020 N4 (the client half
   of the ticket and Gateway chain) is not allocated. The lock entry
   `OTV2-20260929-N4P-NATIVE-GATEWAY-LOGIN` still records the Platform contract text as
   CANDIDATE, pending Game architect acceptance (rollout step 1). `apps/client` also has no edge
   to `oteryn-session-tcp`, and its scene is the placeholder one.
2. **A client can already play.** Owner decision D93 put the graphical dev client in the
   non-production `tools/synthetic-client-harness` live mode (`--live`). It runs on
   `oteryn-dev-client`, so on `oteryn-session` and `oteryn-session-tcp`. It admits with a dev
   grant and plays step, door use and cast against a dev server today.
3. **The session reads no server-initiated delta.** `Session::serve_liveness_until` accepts only
   `LivenessProbe`. Each command exchange reads exactly its `CommandResult` and then one delta of
   its own domain. The server already sends a delta without a command: the Serene cadence in
   `gameplay_transport/connection.rs` writes an `ACTOR_VITALS` delta between commands. An idle
   client with vitals poisons its session on the first Serene change (`UnexpectedMessage`).
   Chat lines from other players (CHAT-1b-2b), entity movement (VIS-3) and combat state
   (ATTACK-1b) arrive the same way.
4. **The CLIENT-NEG-1 candidate** (`claude/otv2-20261004-client-neg-1`, a216f8bd) advertises
   `CLIENT_SUPPORTED_CAPABILITIES = [13]`. It refuses any command or domain whose capability is
   not selected (`GATED_ROUTES`), and checks that a resume does not change the selection. It adds
   no decoder for a new domain.
5. **ITEM-CLIENT-1, as packeted, has no session half.** It owns only `apps/client/src/**`, but
   neither the session nor the client can send command 9 or decode domains 9 and 11. It also
   cannot run against a server until N4.
6. **No packet exists** for N4, N6, N7 or N8, or for ITEM-USE-WIRE-1.

## 1. Rulings

### 1.1 Two tracks: playable now, production after N4

Playable-first, minimum-sufficient: the four features land first where a person can play them,
which is D93's live harness.

- **Playable track (this batch).** Each feature lands as:
  - its session half in `crates/session` (encode the commands, decode and store the domains,
    advertise the capability);
  - a verb in `tools/dev-client`;
  - its view and input in `tools/synthetic-client-harness/src/live/**`.
  It is playable as soon as the server packet that offers its capability merges.
- **Production track (later).** The `apps/client` windows (ITEM-CLIENT-1P to 4P, ATTACK-CLIENT-1P,
  CHAT-CLIENT-1P, which is ADR-0020 N7) follow N4. They consume the same `oteryn-session` API.
  None is allocatable before N4 merges. They are not packeted here, because N4 fixes the
  `apps/client` session loop they plug into (decision queue, §5).
- `apps/client` stays fail-closed (ADR-0011, ADR-0020 §2). Nothing here weakens that.
  `apps/client` gets no edge to `oteryn-dev-client`, so the production closure is unchanged.
- Duplication is bounded. The harness `live/model.rs` is pure mapping. The protocol state, the
  revision checks and the capability gates live once, in `oteryn-session`.

### 1.2 Server-initiated deltas (FND-02 §14, §15)

The client applies every server-sequenced frame in sequence order, whatever caused it.

- **The domain store.** The session keeps, per selected domain, the applied revision and the
  decoded value. A `StateDelta` applies only when its `base_revision` equals the stored revision,
  its domain is in the selected set and its `delta_type` is registered for that domain. Anything
  else fails closed: `StateRevisionMismatch`, an unselected domain or an unregistered type
  poisons the session. Resync is not built. A mismatch stays terminal until a later RESYNC
  packet, as today.
- **Idle.** `service_liveness` answers probes, applies each `StateDelta` and queues an event for
  the caller. Any other message type stays a protocol error.
- **Commands.** The server writes one command's `CommandResult` and that command's own deltas as
  one contiguous run. `serve_admitted` is the connection's single writer, and its Serene branch
  writes only between commands.
- **Push-driven reading (#1734 P2).** The session never predicts the domain of the next frame.
  Every inbound `StateDelta`, before or after a `CommandResult`, goes through the one domain
  store above, by its own domain and revision, and is queued as an event.
- **No attribution (#1736 P1 4176908866).** `StateDelta` carries no command id or causal marker,
  so the client never decides which command a delta belongs to. When the `CommandResult`'s
  disposition says the command changed one known domain (`Moved` for domain 1, `Cast` for
  domain 3), the exchange keeps reading. It stops once that domain's stored revision has
  advanced past the revision it had at the `CommandResult`, while deltas of other domains apply on the way. The outcome's `*_delta`
  field is that domain's first applied delta after the result, by revision alone, whatever caused
  it. The store's value is correct either way.
- **USE returns at its result (#1736 P1 4176969941).** A `COMMITTED` use names no domain: it can
  change domain 2 (a door), domain 9 (a reward chest), domain 11 (a corpse), several of them or
  none. `exchange_use` therefore returns as soon as it has read and checked the `UseResult`, and
  reads no delta. Every later delta goes through the domain store and the event queue (idle
  read or the next exchange). `UseOutcome` keeps its shape. Its `world_object_overlay_delta`
  is always `None`, and `world_spatial_delta` stays `None` as it is today. A caller reads the
  use's effects from `take_events()` or the domain store.
- **Shapes unchanged.** `StepOutcome`, `UseOutcome` and `CastOutcome` keep their shape, so
  `apps/game-server`'s dev-client qualification stage compiles unchanged.
- **The Server Seam qualification follows the USE change (#1736 P1 4177030271).** That
  qualification is `gameplay_transport/qualification.rs`, the test
  `server_seam_real_owners_over_tcp_tls`. It expects each door USE to return
  `Some(AppliedDelta)`, and its final state relies on the closing USE consuming the following
  delta. SESSION-PUSH-1 therefore owns that file, and changes only its door-USE expectations:
  - each `COMMITTED` door use expects `world_object_overlay_delta: None`;
  - the door's domain 2 delta (base revision, new revision, entry) is asserted from
    `take_events()` or the domain store before the next command;
  - the final door revision and state are asserted unchanged.

  The packet runs that qualification and records its `S3B_RESULT=SEAM_PASS`.
- **The server invariant is binding.** Every server packet that adds a pushed delta (VIS-3,
  CHAT-1b-2b, ATTACK-1b, ITEM-MOVE-1) writes it only between command runs, and its server-side
  review and tests check this. The client does not check it, because it cannot tell an unrelated
  same-domain delta from the command's own (#1736 P1 4176908866).
- **Events.** `take_events()` drains the applied pushed deltas in order, typed per domain. The
  harness redraws from them.

### 1.3 Capability advertisement

- A packet adds its capability to `CLIENT_SUPPORTED_CAPABILITIES` in the same change that decodes
  every domain and encodes every command the capability gates. A capability is never advertised
  before then.
- The advertised set is closed under `requires`: 4 needs 6, 17 needs 6, and 15 needs 4. A test
  checks closure against `PROTOCOL_OTERYN_V1_REGISTRY.json`, so a later packet cannot break it.
- Advertising a capability the server does not offer is harmless. It is simply not selected, and
  the client keeps the matching UI hidden. So a client packet may merge before its server packet.

### 1.4 The client lane

One writer at a time owns these paths. Each packet starts from `main` after the previous one
merges:

- `crates/session/**`
- `tools/dev-client/**`
- `tools/synthetic-client-harness/src/live/**`

| Order | Packet | Needs on `main` | Playable after |
|---|---|---|---|
| 0 | CLIENT-NEG-1 (running) | — | — |
| 1 | SESSION-PUSH-1 | CLIENT-NEG-1 | now (Serene vitals) |
| 2 | ENTITY-CLIENT-1 | SESSION-PUSH-1 | VIS-3 offers capability 6 |
| 3 | CHAT-CLIENT-1 | ENTITY-CLIENT-1 | CHAT-1b-2b offers capability 7 |
| 4 | ITEM-CLIENT-1 (retargeted, §3) | CHAT-CLIENT-1 | ITEM-MOVE-1 offers capability 4 |
| 5 | ATTACK-CLIENT-1 | ITEM-CLIENT-1, ATTACK-WIRE-1 | ATTACK-1b offers capability 17 |
| 6 | ITEM-USE-CLIENT-1 | ATTACK-CLIENT-1, ITEM-USE-WIRE-1 | ITEM-USE-1 offers capability 15 |

The order follows the wire lane (VIS-3, CHAT-1b-2b, ITEM-MOVE-1, ATTACK-1b), so each client half
is on `main` by the time its server half is. ENTITY-CLIENT-1 comes before chat because a local
line names its speaker by spatial identity. CHAT-CLIENT-1 may swap with ITEM-CLIENT-1 only if
ITEM-MOVE-1 merges first. The control plane decides that at allocation.

No lane outside this one writes these paths. `apps/game-server` uses `oteryn-dev-client` as a
dev-dependency (`gameplay_transport/qualification.rs`). A client packet therefore makes only
additive API changes. A change that breaks `cargo check -p oteryn-game-server --tests` is out of
scope: the worker returns BLOCKER. The one exception is SESSION-PUSH-1's door-USE expectations in
`qualification.rs` (§1.2, §2.1).

### 1.5 Tests

- **Session half:** an in-process scripted peer (`tokio::io::duplex`) that writes frames built
  with the real `oteryn-protocol-oteryn` encoders, as the existing session tests do. Each test
  covers the happy path, a revision mismatch, an unselected domain, and a pushed delta between
  frames.
- **Harness half:** pure tests on `RenderModel` and the input mapping, in `live/tests.rs`.
- **Live play** against a dev server is manual evidence in the record, once the server packet has
  merged. It is not a gate for merging the client packet.
- The production-path admission test that selects the capability belongs to the server packet,
  as already packeted.

### 1.6 Common packet fields

Every packet in §2 shares these:

```yaml
worker: oteryn-impl-worker
review: client review (Codex, final frozen head)    # SESSION-PUSH-1: protocol review
base: main after the previous client-lane packet merges (§1.4)
migration_lease: none
owned_paths (common):
  - crates/session/src/**
  - tools/dev-client/src/**
  - tools/synthetic-client-harness/src/live/**
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked -p oteryn-session -p oteryn-dev-client -p oteryn-synthetic-client-harness --all-targets -- -D warnings
  - cargo test --locked -p oteryn-session -p oteryn-dev-client -p oteryn-synthetic-client-harness --quiet
  - cargo check --locked -p oteryn-game-server --tests
  - python tools/agents/validate_governance.py
  - git diff --check
```

A packet touches no protocol crate, registry, proto, server or `apps/client` file. If it needs one,
it returns BLOCKER.

## 2. Packets

### 2.1 SESSION-PUSH-1

```yaml
task_id: OTV2-20261004-session-push-1
decision: this batch §1.2; FND-02 §14, §15
review: protocol review (Codex, final frozen head)
branch: claude/session-push-1-20261004
depends_on: [CLIENT-NEG-1]
owned_paths: common (§1.6) + docs/agents/tasks/archive/OTV2-20261004-session-push-1.md
  + apps/game-server/src/gameplay_transport/qualification.rs   # door-USE expectations only (§1.2)
validation: common (§1.6) + the Server Seam qualification
  # WP5_QUALIFICATION=seam bash tools/qualification/wp5_s3b/run.sh locally, or
  # gameplay-server-seam.yml dispatched on the packet's frozen head; S3B_RESULT=SEAM_PASS
  # recorded in the task record
```

Builds:

- The domain store and `take_events()` (§1.2), for domains 1, 2 and 3 as already decoded.
- `serve_liveness_until` applies a pushed `StateDelta`.
- Command exchanges read push-driven (§1.2): any delta, before or after the `CommandResult`,
  applies by its own domain and revision. A step or cast stops once the domain its disposition
  names has advanced, and a use stops at its `UseResult` (§1.2), so `read_delta`'s fixed
  next-domain expectation is removed.
- The harness `LiveController::idle` and `dispatch` drain events into `RenderModel`, and the
  vitals line redraws.

Acceptance:

- A pushed domain 3 delta during `service_liveness` applies, and is queued once.
- A pushed delta before a step's `CommandResult` applies, and the step's own domain 1 delta
  still applies after it.
- A base-revision mismatch, an unselected domain and an unregistered delta type each poison the
  session.
- A domain 3 delta between a step's `CommandResult` and its domain 1 delta applies as pushed,
  and the step still completes with its domain 1 delta. No order of domains is assumed.
- Two consecutive domain 1 deltas after a `Moved` result: the step ends after the first and its
  outcome carries it. The second is not read in that exchange. The next command's exchange or
  the next idle read applies it by revision and queues it as an event, so nothing is lost or
  double-applied (#1736 P1 4176941104). No attribution check exists.
- A `COMMITTED` use returns at its `UseResult` with `world_object_overlay_delta` `None`, for a
  domain 2, a domain 9, a domain 11, a two-domain and a no-delta use alike. The following deltas
  apply through the next idle read or exchange and are queued as events. No use waits for a
  delta or times out (#1736 P1 4176969941).
- The game-server dev-client qualification stage compiles. The Server Seam qualification passes
  (`S3B_RESULT=SEAM_PASS`), with each door USE expecting `None` and its domain 2 delta asserted
  from the event path (#1736 P1 4177030271).

### 2.2 ENTITY-CLIENT-1 (the playable-track N6)

```yaml
task_id: OTV2-20261004-entity-client-1
decision: MOVE-RL11-VISIBILITY §4.2-4.5 (VIS-2 v2 wire); this batch §1.3
branch: claude/entity-client-1-20261004
depends_on: [SESSION-PUSH-1]
owned_paths: common (§1.6) + docs/agents/tasks/archive/OTV2-20261004-entity-client-1.md
```

Builds:

- The client advertises capability 6.
- When capability 6 is selected, domain 1 snapshot type 2 and delta type 2 are decoded with
  `world_spatial_entities`. This includes the initial snapshot that `Session::admit` reads
  (#1734 P2), not only later deltas. Each entity is stored by its `EntityRefV1` (identity and generation).
- Without capability 6, the v1 path is unchanged.
- The harness draws players, creatures, corpses and ground items as distinct glyphs, one entity
  per glyph, with the own actor still `@`. Clicking a tile selects the top entity on it, ordered
  creature, then player, then corpse, then item.

Acceptance:

- Snapshot and delta tests at 0, 1 and 256 entities. `Session::admit` accepts a type-2 initial
  snapshot when capability 6 is selected and refuses it otherwise.
- A v2 payload without capability 6 selected is refused.
- The own actor is read from the header `actor_position` and from its PLAYER entity, and the two
  must agree.
- The harness model test: an entity appears, moves and disappears through pushed deltas.

### 2.3 CHAT-CLIENT-1

```yaml
task_id: OTV2-20261004-chat-client-1
decision: CHAT-0 §7 as registered (CHAT-1b-1, chat_v1.proto); this batch §1
branch: claude/chat-client-1-20261004
depends_on: [ENTITY-CLIENT-1]
owned_paths: common (§1.6) + docs/agents/tasks/archive/OTV2-20261004-chat-client-1.md
```

Builds:

- The client advertises capability 7.
- `Session::chat(ChatIntentV1)` sends command 13 and returns `ChatIntentResultV1`.
- Domain 12: snapshot type 1 (open rooms), delta type 1 (one line) and delta type 2 (the room
  set). Lines are kept in a ring of the last 64 per session. The CHAT0-RL-11 queue sets the size.
- The harness gets these input lines: `say <text>`, `yell <text>`, `whisper <text>`,
  `pm <name> <text>`, `room <n> <text>`, `open <n>` and `close <n>`. It shows a chat pane with
  `Name says:`-style rendering for each `ChatLineV1` kind, and a `DROPPED` marker line.
- `MUTED` and `EXHAUSTED` show their `wait_seconds`.

Acceptance:

- Encode/decode tests for every intent variant at the 1,020-byte text bound.
- A pushed line from another speaker applies while idle.
- A `DROPPED` marker renders.
- A result of `MUTED` or `EXHAUSTED` changes no local state.
- A domain 12 delta without capability 7 selected is refused.

### 2.4 ITEM-CLIENT-1 (retargeted to the playable track)

```yaml
task_id: OTV2-20261003-item-client-1
decision: ITEM-MOVE-WIRE-0 §4 (client views); item batch §1.1; this batch §1.1, §3
branch: claude/item-client-1-20261003
depends_on: [CHAT-CLIENT-1]          # ITEM-MOVE-1 is needed to play, not to merge (§1.3)
owned_paths: common (§1.6) + docs/agents/tasks/archive/OTV2-20261003-item-client-1.md
```

Builds:

- The client advertises capability 4 (6 is already advertised).
- Domain 9 (inventory) and domain 11 (open container) snapshots and deltas are decoded with
  `item_view`.
- With capability 4 selected, domain 1 entities are decoded with the `_with_item_handles` codecs
  (field 10 required on corpses and ground items, D212). Without it, field 10 is refused.
- `Session::use_item(ItemTargetV1)` sends command 2 with the field 2 item target, which opens a
  corpse. `Session::move_item(ItemMoveIntentV1)` sends command 9 and returns its result.
- The harness: clicking a corpse entity opens it, using its `item_handle`. A backpack pane and a
  corpse pane show the entries. `loot <n>` moves corpse entry `n` to the backpack.

Acceptance:

- The scripted peer opens a corpse and loots one entry: the domain 11 and domain 9 deltas
  follow the result.
- A `STALE` result refreshes nothing locally.
- A duplicate `item_handle` in one snapshot or delta is refused.

Not in scope: equipment, nested bags and Ground (ITEM-CLIENT-2, -3, -4, retargeted in §3).

### 2.5 ATTACK-CLIENT-1

```yaml
task_id: OTV2-20261004-attack-client-1
decision: ATTACK-0 §3 as leased in part A (#1735) §0.1, §2.1; this batch §1
branch: claude/attack-client-1-20261004
depends_on: [ITEM-CLIENT-1, ATTACK-WIRE-1]
owned_paths: common (§1.6) + docs/agents/tasks/archive/OTV2-20261004-attack-client-1.md
```

Builds:

- The client advertises capability 17.
- `Session::attack(Option<EntityRefV1>)` sends command 11, and `Session::fight_modes(..)` sends
  command 12. Both return `AttackIntentResultV1`.
- Domain 10 `ActorCombatStateV1` snapshot and delta are decoded.
- The harness: right-clicking a creature entity, or `attack <tile>`, attacks it, and `stop` clears
  the target. `mode <offensive|balanced|defensive>` and `secure <on|off>` set the fight modes. The
  target is marked, and an in-fight flag shows. Chase is sent as STAND until CHASE-1 (part A §2.1
  review question).

Acceptance:

- The scripted peer: attack is selected, then a domain 10 delta is pushed, then the target is
  cleared, then the target is lost through a pushed delta.
- Every `AttackIntentResultV1` value renders as status text and changes no local state.
- A domain 10 delta without capability 17 selected is refused.

### 2.6 ITEM-USE-CLIENT-1

```yaml
task_id: OTV2-20261004-item-use-client-1
decision: ITEM-USE-0 §3 (with its BAGS-0 §8 amendment once accepted); this batch §1
branch: claude/item-use-client-1-20261004
depends_on: [ATTACK-CLIENT-1, ITEM-USE-WIRE-1]
owned_paths: common (§1.6) + docs/agents/tasks/archive/OTV2-20261004-item-use-client-1.md
```

Builds:

- The client advertises capability 15 (`ITEM_USE_V1`, its #162 lease).
- `use_item` gains field 5 (`ItemByDefinitionV1`, the hotkey form) and field 4 (`use_with` a
  creature) for potions. The four new dispositions decode.
- The harness: `eat <n>` and `drink <n>` act on a backpack entry, `hotkey <definition_index>` uses
  by definition, and `drink <n> on <tile>` uses a potion on the creature on an adjacent tile.

Acceptance:

- Every disposition renders.
- The client sends field 4 for any item. It has no item semantics, so the server decides, and its
  refusal renders as a disposition (#1736 P2 4176908870).
- The scripted peer runs one burn and one flask transform, each followed by its domain 9 delta.

Allocation needs ITEM-USE-WIRE-1 on `main`. Playing needs ITEM-USE-1 (§5 item 2).

## 3. Amendments

- **Item batch §2.7, ITEM-CLIENT-1:** the packet in §2.4 above replaces it. The task_id and
  branch are unchanged. The owned paths, `depends_on` and acceptance are replaced. The
  `apps/client` backpack and corpse windows become ITEM-CLIENT-1P on the production track (§1.1).
- **Item batch §2.7a, ITEM-CLIENT-2, -3 and -4:** retargeted the same way. Owned paths are §1.6
  common. Each keeps its capability, acceptance and server dependencies, and also needs the
  client-lane packet before it on `main` (§1.4), which the control plane orders at allocation.
  Their `apps/client` windows become ITEM-CLIENT-2P to 4P.

## 4. Rejected options

- **Building the features in `apps/client` now.** Nothing could be played until N4 and N4-P, and
  the owner goal is playable.
- **An internal dev entry in `apps/client` through the dev grant.** ADR-0020 §7 keeps any build
  before N4-P fail-closed, and D93 already gives a playable client in the harness. Reopening that
  needs an owner decision and buys nothing the harness does not.
- **Correlating deltas through `authoritative_revision`.** No registered command emits it, and
  adding it is a wire change. The single-writer contiguity in §1.2 is already true on `main`.
- **One client packet per feature in parallel.** All of them write `crates/session/src/lib.rs`
  and the harness model. Serializing them costs less than merge conflicts on one file.

## 5. Decision queue for the control plane

1. **N4 is unblocked by one architect acceptance.** The Game architect must accept the Platform
   contract text `OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md` at ed6c0d38 (lock rollout step 1).
   That needs a read of `Oteryn/Oteryn-Platform` at that commit. Recommendation: allocate
   ARCH-N4P-ACCEPT-1 to this architect with read-only access to that repository. N4 can then be
   packeted client-first-safe (it fails closed while the Platform branch is off), together with
   the `apps/client` -> `oteryn-session-tcp` edge that ADR-0020 §1 already decides.
2. **ITEM-USE-WIRE-1 has no packet.** ITEM-USE-0 §3 defines it fully. Recommendation: the
   architect packets it into the wire lane after ATTACK-1b, in part C or the next batch. Capability
   15 is already leased.
3. **N8 (admission refusal message) has no packet.** Until it lands, the harness shows one generic
   refusal, as ADR-0020 §2 says. It does not block this batch.

## 6. Decision test

| Question | Answer |
|---|---|
| Does any packet change a wire, registry, migration or production boundary? | No |
| Is each feature playable as soon as its server packet merges? | Yes, through the D93 harness |
| Can two packets write one file at once? | No, one client lane (§1.4) |
| Does `apps/client` stay fail-closed? | Yes |
| Is the latent idle poison on a Serene vitals change fixed? | Yes, SESSION-PUSH-1 |
