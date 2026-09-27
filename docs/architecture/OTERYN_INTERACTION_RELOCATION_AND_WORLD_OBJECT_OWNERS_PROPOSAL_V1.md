# Interaction relocation and world-object owners: proposal v1

- Date: 2026-09-27
- DecisionStatus: CANDIDATE with owner decisions D37 and D38 taken (§6, 2026-09-27). The contract
  text becomes accepted after the independent review that authority changes require; until then
  the Movement and WorldObject children stay blocked. §7 (`revert_after` progression) is a new
  decision delta added after the independent review of D38 returned `ACCEPT_WITH_CONDITIONS`; it is
  CANDIDATE only, has no owner acceptance yet, and does not itself resolve who accepts it.
- DeliveryStatus: OPEN
- ImplementationStatus: NOT_STARTED
- Requested by: the owner (2026-09-27) after the quest interaction transcription
  (`OTERYN_QUEST_AUTHORING_FORMAT_V1.md` §3.3, §6.3, D36).
- Unblocks: the Movement and WorldObject children that D36 keeps blocked.

## 1. Problem

Quest scripts move players and change map objects, and D36 keeps both kinds of change blocked
because neither has an owner contract. GAME-INTERACTION-01 §19.3 names the missing contract: an
owner that defines operation identity, fences, completion, timeout or cancellation, stale
completion and recovery.

The block is large. In the transcription of both servers' quest scripts (`samples/interactions/`):

| Child | Count | Quest directories | Breakdown |
|---|---:|---:|---|
| Movement (`teleportTo`) | 800 | 93 | to a fixed anchor 219, back to the previous tile 209, to a computed target 372 |
| WorldObject | 891 | 89 | transform 493, remove 181, create 141, action-id change 39, decay 32, timed revert 5 |

Without these owners most quests with map mechanics stay unplayable, even with complete quest
data: seal walls, levers, boss-room entries and ejections.

## 2. What already exists

- **VSL-MOVE-01** (ACCEPTED 2026-08-16, `OTERYN_V2_STAGE_C_VSL_OWNER_ACCEPTANCE_20260816.md`).
  `ChannelRuntime`/`InstanceRuntime` is the one authoritative position owner of its scope.
  - It already classes "a simple authored teleport/link", requested by an accepted interaction
    child, as a movement-owner relocation within the current scope
    (`VSL-MOVE-01_MINIMAL_MOVEMENT_VISIBILITY_CONTRACT_CANDIDATE.md` §3).
  - `SCOPE_HANDOFF` (a move to another Channel or Instance) is outside its first slice.
- **GAME-INTERACTION-01 successor** (PROPOSED). It coordinates trigger, children, retry and
  reconciliation. It deliberately does not choose the movement owner (§19.3).
- **Local transition candidate** (`docs/agents/evidence/OTV2-20260917-content-world-local-transition-contract-candidate.md`,
  PROPOSED_NONCANONICAL).
  - Covers a two-state OPEN/CLOSE world-object transition, executed by the current scope runtime,
    scope-ephemeral, fenced on session, connection, scope, owner generation and content generation.
  - Explicitly excludes quest progress, rewards, keys and value-bearing objects.
- **Position seam** (`docs/agents/evidence/OTV2-20260924-channel-actor-position-owner-seam.md`).
  A per-actor position slot with a revision and compare-and-commit inside the Foundation actor
  carrier. It is infrastructure, not an owner.
- **Constraints that bind any answer:**
  - ADR-0019: no second owner of shared truth.
  - The one-writer-per-scope rule (the scope matrix and VSL-MOVE-01).
  - DUR-03 for anything that carries value.
  - DUR-04: content is immutable input, and scripts only propose plans.
  - FND-02 for client command identity.

## 3. Proposal R: relocation children

The owner is the current scope's `ChannelRuntime`/`InstanceRuntime`, as VSL-MOVE-01 already
accepts. This proposal adds no new domain; it only fills the §19.3 fields for one child kind.

- **Operation.** Relocate one actor within the current scope. The target is either a content anchor
  bound to a world placement, or the tile the actor stood on before the triggering step (the
  `previous_position` target of D36).
- **Identity.** The GAME-INTERACTION child identity (trigger identity plus child ordinal). A repeated
  request with the same identity returns the first outcome and never moves twice.
- **Fences** (checked before commit, any mismatch rejects):
  - WorldId, ChannelId or InstanceId;
  - the actor's session generation;
  - the actor position revision observed by the trigger;
  - the content generation of the interaction definition.
- **Completion.** The committed position revision is the completion. There is no asynchronous
  phase, because the scope runtime commits within its own tick.
- **Outcomes.**
  - `COMMITTED`.
  - `REJECTED_STALE`: a fence moved, for example the actor already moved or the session changed.
  - `REJECTED_BLOCKED`: the target is not enterable. The fallback follows VSL-MOVE-01 placement
    rules; there is no silent search for a free tile unless the definition names one.
- **Timeout, cancellation and recovery.**
  - A request that is not committed in the tick that received it is rejected as stale; it does not
    queue.
  - Position is scope state. After a crash, the character's position is restored by the existing
    character persistence path, and nothing here adds a durable record.
- **Out of scope.**
  - Relocation to another Channel or Instance. That is `SCOPE_HANDOFF`, which stays blocked until
    its own contract exists.
  - Computed targets (372). They stay unresolved until their definition can name an anchor, or a
    DUR-04 component proposes the target.

## 4. Proposal W: world-object overlay children

The owner is again the current scope's runtime, as the local transition candidate proposes. The
proposal extends that candidate from OPEN/CLOSE to the typed operations the quest scripts use, and
keeps its lifetime and exclusions.

- **State.** A scope-local overlay over immutable world placements. Content stays immutable
  (DUR-04); the overlay records only the difference from the placement.
- **Operations.** Each works on a non-value-bearing world-object definition at a placement anchor:

  | Operation | Covers |
  |---|---|
  | `TRANSFORM(from, to)` | transform, levers |
  | `CREATE(def)` | walls, magic walls, flames, portals |
  | `REMOVE(def)` | walls, stones, barriers |
  | `RETAG` | the action-id change that re-arms a trigger |

  The CW3 Content-model worker (allocation `OTV2-20260928-cw3-local-object-state-model`) is
  implementing a `SHARED_LEASE_REQUIRED` delta this operation table depends on, recorded here
  without being designed by this task:
  - **1a.** Per-state collision presence on a `LocalObject` definition: each declared state is
    `{key, collision: Present | Absent}`, replacing the current runtime's hard-wiring of collision
    to only the Open/Close two-state pair (`apps/game-server/src/world_runtime.rs` `prepare`'s
    `next_blocking` match on `LocalObjectOperation::Open`/`Close`).
  - **1b.** An authored initial state on a `LocalObject` placement, validated fail-closed: binding
    fails if the placement's authored initial state is absent from the definition's declared
    states, rather than defaulting silently.
  - **1c.** `RETAG` is the coordinator-decided transition between two states of the same collision
    class that differ only in interaction binding (for example, re-arming which trigger fires),
    under a `…local-object-retag` intent family, with no action-id field — distinct from
    `TRANSFORM`, which may also change collision class.

  Coordinator direction: CW4 ships `TRANSFORM`/`CREATE`/`REMOVE`/`RETAG` now **without**
  `revert_after`, until §7 below is accepted.

- **Timed revert.** Every operation may carry `revert_after`, which covers `decay` and
  `revertItem`/`addEvent` reverts. The revert is the same owner's own later operation, with its
  own identity derived from the first. §7 records the still-open decision on what logical
  progression input `revert_after` is measured against; nothing here is implemented yet.
- **Identity, fences and completion.** As in §3: the child identity; World, scope and content
  generation fences, plus the overlay revision of the anchor; the committed overlay revision is the
  completion.
- **Lifetime.** Scope-ephemeral: a scope restart clears the overlay back to the placements.
  Anything that must survive, such as a seal that stays open for one character, is quest state
  (D35) that conditions read. It is not world-object state.
- **Value boundary.**
  - An object a player can pick up, carry or trade is an item. Its creation is a DUR-03 hand-out
    or mint, and removing a carried item is DUR-03 consumption.
  - The transcription now tells them apart: 183 removals are DUR-03 consumption, not overlay
    removals. They are the player's `removeItem`, and `remove` on the item used or dropped onto the
    edge. A further 43 remove a creature, which is neither; they stay unresolved.
- **Out of scope.**
  - Durable world-object state.
  - **C3 (independent-review hardening).** The supported collision footprint is the anchor's own
    fixed, bind-time-reserved cells: `absolute_collision_cells(placement)` computes
    `collision_cells`/`blocking_cells` once at `LocalObjectRuntime::bind`
    (`apps/game-server/src/world_runtime.rs` ~698) and every later `CREATE`/`REMOVE`/`TRANSFORM`/
    `RETAG` on that anchor only toggles within that one fixed footprint. Dynamically materialized
    geometry — a `CREATE` that reserves cells not already known at bind time, or spans more than
    the one anchor's pre-authored footprint — stays out of scope; "multi-cell" here means only
    that a single anchor's fixed footprint may cover more than one cell, never that new cells are
    materialized at runtime.
  - Houses and access lists.
  - Objects owned by several scopes.

## 5. Why this is the minimum

- Both proposals reuse accepted or candidate owners (VSL-MOVE-01 and the local transition
  candidate) and add operation fields, not domains.
- No persistence is added.
- Everything value-bearing stays with DUR-03.
- Quest-relevant memory stays with the quest domain.
- Once accepted, the D36 definitions need no new data: the blocked children become executable
  after their anchors bind to world placements.
- The 1a/1b/1c Content-model delta (§4) generalizes an existing hard-wired two-state runtime field
  (collision presence) and an existing hard-wired binding default (initial state); it adds no new
  owner, persistence or domain. C3 does not add multi-scope or dynamic geometry; it only names the
  footprint boundary the runtime already enforces by computing collision cells once at bind time.
- §7's recommended `revert_after` option adds exactly one new scope-owned step driver and counter,
  owned by the same scope-runtime owner named in D38 — one mechanism per scope, never per object;
  it adds no per-object timer service, queue, receipt store or persistence.

## 6. Owner decisions

The owner accepted every recommendation below ("zgadzam się", 2026-09-27):
- **D37** records R1-R3, the relocation owner;
- **D38** records W1-W3, the world-object owner.

| # | Question | Decision |
|---|---|---|
| R1 (D37) | Is the scope runtime (`ChannelRuntime`/`InstanceRuntime`) the owner of interaction relocation children, with the identity and fences of §3? | Yes; VSL-MOVE-01 already names it. |
| R2 (D37) | Is a request not committed in its tick rejected (no queue, no timeout state)? | Yes. |
| R3 (D37) | Does relocation to another Channel or Instance stay blocked until `SCOPE_HANDOFF` has a contract? | Yes. |
| W1 (D38) | Is the scope runtime the owner of the world-object overlay, extending the local transition candidate to `TRANSFORM`, `CREATE`, `REMOVE`, `RETAG` with `revert_after`? | Yes. |
| W2 (D38) | Is world-object state scope-ephemeral, with anything durable held as quest state (D35)? | Yes. |
| W3 (D38) | Are pick-up-able objects and carried-item removal always DUR-03, never overlay state? | Yes. |

## 7. `revert_after` logical progression: CANDIDATE, owner-acceptance still open

The independent review of D38 (2026-09-27/28) returned `ACCEPT_WITH_CONDITIONS` with an
`EVIDENCE_GAP`: `revert_after` names no scheduling mechanism. This section answers only which
existing owner supplies the logical progression input `revert_after` is measured against. It does
not accept the answer on the owner's behalf, and it binds condition C2 (the revert is the scope
runtime's own later operation: derived child identity, same World/scope/content-generation/
overlay-revision fences, fired from the scope runtime's existing progression, cleared on scope
restart — no per-object timer service, queue, receipt store or persistence).

### Problem

`revert_after` needs *some* value that advances on its own, independent of whether a player ever
sends another command to the affected anchor — a decayed wall or a timed door must still revert if
nobody touches it again. No such input exists in the read code today.

### Evidence

- PROVEN (`apps/game-server/src/world_runtime.rs` `LocalObjectRuntime::bind` ~570-712 and
  `::prepare` ~932-997): neither takes a tick, timestamp or step parameter; every fence is
  placement/incarnation/content_generation/expected_revision, all of which only change when a
  command targets that same anchor.
- PROVEN (`apps/game-server/src/foundation/runtime_actor_carrier.rs` `ChannelRuntimeV1` ~695-700,
  `advance_owner` ~2169-2176): the only progression the scope owner exposes is
  `ScopeOwnershipGeneration`, advanced on scope reassignment, not on the passage of scope time.
- PROVEN (`apps/game-server/src/gameplay_transport/connection.rs` `Liveness::tick` ~296-320): the
  only "tick" in the read tree is a per-connection transport keepalive cadence (probe/ack
  liveness), unrelated to world/scope simulation state and not addressable per scope.
- PROVEN (`apps/game-server/src/content/project/v2/creature.rs` `tick_profile`/`tick_interval_ms`/
  `tick_counts` ~571-604, ~1189-1231): an imported-content authoring schema describing
  damage-over-time timing as *data*; no runtime consumer executing it as a live clock was found in
  the read tree.
- PROVEN (`docs/architecture/SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md` line 224:
  "No universal fixed global tick is required."; line 419: "global tick rate ... deliberately
  deferred."): there is no committed Foundation/global simulation tick to reuse.
- PROVEN (bounded grep for `tokio::time::interval|tokio::time::sleep|select!\{|loop \{` in
  `apps/game-server/src` and `crates/`, cross-checked against every `ChannelRuntimeV1` use site —
  `gameplay_transport/{qualification,mod}.rs`, `movement.rs`, `node/serve.rs`,
  `foundation/{runtime_actor_carrier,mod}.rs`): every located call into `ChannelRuntimeV1` is
  reactive. `apps/game-server/src/gameplay_transport/mod.rs`
  `ComposedFreshAdmission::release_after_grace` (~473-514) is a per-connection grace-expiry retry
  loop with its own backoff `sleep`, scoped to one admitted session, not the scope. `apps/game-
  server/src/movement.rs` `MovementOwnerTurn::begin`/`try_step` (~213-249) processes a bounded batch
  of movement inputs (`max_inputs`) per invocation, and its own doc comment (~197-200) says so
  explicitly: "No production maximum, queue, command outcome, or scheduling authority is implied.
  Fairness remains an obligation of the future owner scheduler." No independent scope-wide cadence
  that advances regardless of command activity was found anywhere driving `ChannelRuntimeV1`.
- UNKNOWN whether a scope-owned periodic driver exists outside this bounded read tree. This section
  does not assume one; option 2 below is written to hold either way.

### Options (minimum real set)

1. **Reuse an existing Foundation/global simulation tick.** Rejected as not currently available:
   no such tick exists (evidence above), and SIM-DETERMINISM-01 explicitly defers ever requiring
   one. Adopting or creating one now would be a new Foundation-owned decision, out of this task's
   `excluded_scope` (Foundation/runtime/protocol/registry) and disproportionate to one `revert_after`
   field.
2. **A new scope-owned monotonic logical step, supplied by the scope runtime owner (RECOMMENDED,
   stated honestly).** The evidence above shows no proven scope-owned cadence exists today: every
   located `ChannelRuntimeV1` call is reactive, and `movement.rs` names the missing "future owner
   scheduler" in its own doc comment. This option is therefore not "reuse an existing increment
   point" — it is introducing the one real new mechanism: the same owner D38 already names
   (`ChannelRuntimeV1`/`InstanceRuntime`) adds its own step driver, one per scope owner (not per
   object, not per pending revert), that drives all of that scope's own due work forward on
   whatever cadence the owning lane picks — piggy-backing on a cadence it can later prove already
   exists, or a minimal interval added for exactly this purpose. `revert_after` stores an absolute
   target step derived at commit time from that counter; the revert fires as the scope's own next
   step reaches or passes that target. Without deciding to introduce this driver, option 2 collapses
   into option 3.
3. **Purely reactive re-evaluation against an already-existing counter** (`LocalObjectRuntime`'s
   own `revision`, or `ScopeOwnershipGeneration`). Rejected: both only advance when something else
   already happens to that same object or scope, so an untouched decaying wall or timed door would
   never re-evaluate and could never fire — this does not satisfy the covered `decay`/
   `revertItem`/`addEvent` use case in §4.

### Must-decide-now test

1. **Must decide now?** `YES` for the *owner and input shape* only (option 2 vs. the record of why
   1 and 3 are rejected); `NO` for the exact increment cadence, storage representation or unit,
   which the owning lane decides when it implements the delta.
2. **What is blocked?** CW4 cannot ship `revert_after` at all (coordinator direction in §4 already
   withholds it) until some owner is named for the progression input; naming nothing leaves the
   `EVIDENCE_GAP` open indefinitely.
3. **What becomes harder later?** Picking option 1 later, after option 2 ships, would require
   migrating every stored `revert_after` target from a scope-local step to a global tick value —
   real but bounded migration cost, not an irreversible one, since both are monotonic counters on
   the same overlay lifetime.
4. **What evidence would supersede this?** A later, separately accepted Foundation decision that
   introduces a real global simulation tick for reasons independent of `revert_after` (SIM-
   DETERMINISM-01 would have to be amended first); or measured evidence that a per-scope counter
   cannot meet a specific product timing requirement (for example cross-scope revert ordering,
   which §4 already excludes).
5. **What is deliberately not decided?** The exact field name, integer width, whether the owning
   lane's step driver piggybacks on a cadence it later proves already exists or adds a new minimal
   interval for exactly this purpose, and whether `revert_after`'s unit is "N scope steps" or another
   monotonic unit the owning lane picks. Those belong to the owning lane's implementation, not this
   architecture delta.

### Exact delta the owning lane must provide

Owner: the scope-runtime/Foundation carrier lane behind `ChannelRuntimeV1`/`InstanceRuntime`
(`apps/game-server/src/foundation/runtime_actor_carrier.rs`), coordinated with the CW4 world-runtime
lane (`apps/game-server/src/world_runtime.rs`) that owns `LocalObjectRuntime`. This document does
not implement it; it is CANDIDATE and not owner-accepted.

- Introduce the scope owner's own step driver: one per scope owner (`ChannelRuntimeV1`/
  `InstanceRuntime`), never a per-object or per-revert timer, driving all of that scope's own due
  work forward. Whether it piggybacks on a cadence the scope runtime is later shown to already have,
  or adds one new minimal interval for exactly this purpose, is the owning lane's implementation
  choice — either way it is one mechanism per scope, not new infrastructure per object.
- Add one monotonic step counter scoped to `scope_generation`, reset to its initial value whenever
  `scope_generation` changes (the same reset `advance_owner` already causes for the rest of the
  overlay), incremented only by that one scope-owned driver.
- On any `revert_after`-carrying overlay operation, store the derived revert's target step alongside
  the same fences §4 already requires (World/Channel/InstanceId, `scope_generation`,
  `content_generation`, the overlay revision of the anchor) and a child identity derived from the
  original operation's identity (as in §3/§4).
- On each of the scope runtime's own step advances, check any due entries for anchors it owns and,
  if due, commit the revert through the same `prepare`/commit path as any other overlay operation
  (`PreparedMutation::Publish`/`TerminalSemanticOutcome`, `apps/game-server/src/world_runtime.rs`
  ~982-996) — not a separate code path.

### Exact test obligations

- **Fires once.** The derived child identity (as in §3/§4) makes a repeated arrival at or after the
  due step commit only the first outcome; once committed, the entry is removed from the pending set
  so a later step never refires it.
- **Replay-safe.** Re-evaluating "is this due" after it has already fired must be side-effect free:
  it observes the entry already cleared and returns the same deterministic no-op outcome the
  runtime already has for a stale/consumed command (`DISPOSITION_NO_CHANGE`/`DISPOSITION_STALE_STATE`,
  `apps/game-server/src/world_runtime.rs` ~947-958).
- **Fenced.** The revert commits only under the same World/Channel/InstanceId, `scope_generation`,
  `content_generation` and overlay-revision-of-anchor fences as any other §4 operation; a fence
  mismatch discards the pending revert rather than forcing it through.
- **Cleared on scope restart.** Because the step counter and the pending-revert set are both
  `scope_generation`-scoped state owned by the same instance as the rest of the overlay, a scope
  restart drops them with no separate cleanup path — the same lifetime §4 already states
  ("Lifetime: Scope-ephemeral").
- **No partial footprint.** A revert applies its full target state in the same single commit as any
  other overlay operation (the existing `PreparedMutation::Publish` path); this is exactly the C3
  fixed-footprint boundary in §4 — only the anchor's pre-authored, bind-time-reserved footprint is
  ever touched, never a partially materialized one.
- **Occupied target cells defer/refuse deterministically.** A revert whose target state's footprint
  conflicts with currently occupied cells reuses the existing `DISPOSITION_OCCUPIED` outcome
  (`apps/game-server/src/world_runtime.rs` ~965-969) rather than evicting occupants or silently
  dropping the revert. Whether "occupied" means retry on a later due step or refuse permanently is
  exactly one deliberately-not-decided implementation choice (must-decide-now item 5 above), but it
  must be one named, deterministic outcome either way.

## 8. Follow-up

1. The quest transcription splits `remove` into overlay removal and DUR-03 consumption (done in
   the same change as D37 and D38; see the quest format §6.3).
2. Anchors bind to world placements.
3. The GAME-INTERACTION-01 successor names these owners in §19.3.
4. Independent review of the accepted text, then implementation in the scope runtime.
5. **CW3 (Content model).** Implement the 1a/1b/1c delta in §4: per-state collision presence,
   authored initial state validated fail-closed, and the `RETAG`/`…local-object-retag` decision.
6. **CW4 (runtime).** Ship `TRANSFORM`/`CREATE`/`REMOVE`/`RETAG` without `revert_after`, per the
   coordinator direction recorded in §4, until §7 is owner-accepted.
7. **Scope-runtime / Foundation carrier lane (`ChannelRuntimeV1`/`InstanceRuntime`).** Own §7's
   decision: accept or supersede the recommended `revert_after` progression option and supply the
   exact delta §7 names — including, if no existing scope cadence is proven, the scope's own step
   driver itself. CW4 then adds `revert_after` on top of the already-shipped operations.
