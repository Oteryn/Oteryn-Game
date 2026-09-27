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
- §7's recommended `revert_after` option adds exactly one new scope-owned step driver plus a
  pending-`Deadline` set, both owned by the same scope-runtime owner named in D38 — one mechanism
  per scope, never per object — and reuses the already-implemented `crates/foundation::time`
  `Deadline`/`MonotonicClock` primitive rather than inventing a new unit; it adds no per-object
  timer service, queue, receipt store or persistence.

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

Round 3 correction: `revert_after` is bound to the existing FND-03 §10 authoritative-timer contract
(`docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md`), not to the client-command lifecycle a
prior draft assumed. FND-03 already defines exactly the shape a scope-owned deadline needs — a
mutation-capable timer that is "an owner-scoped input, not a direct callback" (§10) — so this
section binds `revert_after` to it rather than inventing parallel semantics.

### Problem

`revert_after` needs *some* value that advances on its own, independent of whether a player ever
sends another command to the affected anchor — a decayed wall or a timed door must still revert if
nobody touches it again, and even if the player who triggered the original operation has since
disconnected. No such input, and no non-client-command commit path for it, exists in the read code
today.

### Evidence

- PROVEN (`apps/game-server/src/world_runtime.rs` `LocalObjectRuntime::bind` ~570-712 and
  `::prepare` ~932-997): neither takes a tick, timestamp or step parameter; every fence is
  placement/incarnation/content_generation/expected_revision, all of which only change when a
  command targets that same anchor.
- PROVEN, production-only (`apps/game-server/src/foundation/runtime_actor_carrier.rs`
  `ChannelRuntimeV1::from_committed_assignment` ~702-750): production construction pins
  `scope_generation` once into `ChannelRuntimeAssignmentBinding` at construction (~727-728,
  ~738-745); there is no production method that advances it in place. A scope restart in
  production is therefore a new `ChannelRuntimeV1` instance via `from_committed_assignment`, not an
  in-place generation bump. (`advance_owner` ~2169-2176 is test-only, inside `impl
  MovementActorFixture` under `#[cfg(test)]` — struct at ~2030-2031 — and proves nothing about
  production.) Two production doc comments independently confirm no scheduler exists:
  `borrow_movement_position` (~760-762) "grants neither initial position authority nor an owner
  scheduler"; `borrow_combat_death` (~773-774) "grants no scheduler, production activation or corpse
  lifetime."
- PROVEN (`apps/game-server/src/gameplay_transport/connection.rs` `Liveness::tick` ~296-320,
  `apps/game-server/src/content/project/v2/creature.rs` `tick_profile` ~571-604,
  `docs/architecture/SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md` lines 224/419, and a
  bounded grep for `tokio::time::interval|tokio::time::sleep|select!\{|loop \{` against every
  `ChannelRuntimeV1` call site): no scope-wide cadence, no global tick, and every located
  `ChannelRuntimeV1` call is reactive (`movement.rs` ~197-200 itself names the missing "future owner
  scheduler"). UNKNOWN beyond this bounded read tree.
- PROVEN (`crates/foundation/src/time.rs` `Deadline`/`MonotonicClock`/`ManualClock` ~50-166, exported
  at `crates/foundation/src/lib.rs` line 5): an already-tested monotonic-deadline primitive, unused
  in `apps/game-server/src` today. PROVEN (`docs/architecture/OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md`
  line 143): the map-object authoring format already names `revert_after_ms` (milliseconds), with a
  concrete authored value at line 170 (`revert_after_ms 1200000`).
- PROVEN (`docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md` §10, lines 391-436): "Mutation-
  capable timers are owner-scoped inputs, not direct callbacks." §10.1 binds a timer to semantic
  runtime scope, current ownership generation, target entity/local generation, monotonic due
  deadline and deterministic equal-deadline order, adding: "Equal-deadline order is derived from the
  owner resolution that scheduled the timer plus a deterministic within-resolution sequence; a
  separate globally visible timer counter is not required." §10.2: "When due, the timer becomes a
  normalized authoritative input and receives a new `RuntimeExecutionOrdinal` when the current owner
  accepts it for resolution." §10.3 lists cancellation/staleness triggers: scope ownership
  generation changed, target entity/local generation mismatch, owning state/condition invalidated,
  or explicit cancellation/expiry.
- PROVEN (FND-03 §15.4, lines 560-564): "If committing a new operation requires registering a
  required authoritative timer and no safe bounded timer capacity exists, the operation fails before
  committing the state that depends on that timer," and an already-accepted timer is never silently
  discarded for due-queue congestion.
- PROVEN (FND-03 §28, line 891): the Foundation error vocabulary already names this outcome:
  "registered queue/timer/work/resource limit reached" → `CAPACITY_EXCEEDED` → "bounded rejection/
  backpressure before unsafe acceptance."
- PROVEN (FND-03 §9, lines 376-389): "An opaque process-local monotonic instant can never be the
  durable representation of a timer that must survive process failure," with a durable-encoding
  requirement for any timer "whose semantics cross a GameNode process lifetime." `revert_after` does
  not cross a process lifetime (D38 W2: scope-ephemeral, cleared on scope restart), so §9's durable-
  encoding requirement does not apply to it; this is stated explicitly so a later reviewer does not
  need to re-derive it.
- PROVEN (`apps/game-server/src/foundation/mod.rs` `RuntimeExecutionOrdinal`/`ScopeRuntimeFence`
  ~959-1058): the FND-03 §10.2 ordinal-on-accept mechanism is already implemented —
  `ScopeRuntimeFence::accept_input(generation)` mints a new `RuntimeExecutionOrdinal` only for the
  exact current `ScopeOwnershipGeneration`, rejecting a stale one, matching §10.3's generation-
  changed cancellation exactly. It is currently instantiated per `GameSession`
  (`apps/game-server/src/foundation/admission.rs` ~365-381, ~728), not yet as one scope-wide instance
  consumed by `world_runtime.rs`.
- PROVEN (`apps/game-server/src/world_runtime.rs` `apply`/`resume_pending` ~776-832,
  `validate_current_authority` ~849-859): both require a live `GameSessionAuthoritySnapshot`
  (rejecting when `session_state() != Active` or the snapshot's `game_session_id` does not match the
  command's) and a per-session `CommandIngress` for duplicate detection. A scope-owned due timer has
  neither — the player who triggered the original operation may have disconnected by the time
  `revert_after` elapses. "The same prepare/commit path as any client command" was therefore
  undefined for the revert; only `prepare`'s pure state/footprint-transition logic, not the
  session-command lifecycle around it, can be reused.
- PROVEN (`crates/foundation/src/time.rs` `SystemClock::new()` ~98-104, `Moment`/`Deadline` ~5-52):
  `SystemClock::new()` sets `origin: Instant::now()` fresh on every call, and a `Moment`/`Deadline`
  stores only elapsed duration with no origin identity — two `Deadline`s produced from two different
  `SystemClock` instances are not meaningfully comparable; evaluating `has_elapsed`/`remaining`
  against the wrong clock instance would silently misfire.

### Options (minimum real set)

1. **Reuse an existing Foundation/global simulation tick.** Rejected as not currently available:
   no such tick exists (evidence above), and SIM-DETERMINISM-01 explicitly defers ever requiring
   one. Adopting or creating one now would be a new Foundation-owned decision, out of this task's
   `excluded_scope` (Foundation/runtime/protocol/registry) and disproportionate to one `revert_after`
   field.
2. **An FND-03 §10 authoritative timer bound to a monotonic `Deadline` computed from the authored
   duration (RECOMMENDED).** At commit time — as part of the *same* staged commit as the original
   `TRANSFORM`/`CREATE`/`REMOVE`/`RETAG` (FND-03 §15.4, below) — compute `Deadline::after(clock,
   Duration::from_millis(revert_after_ms))` from the one clock instance the scope owns (evidence
   above) and register it under an FND-03 §10.1 scheduling key: World/Channel/InstanceId,
   `scope_generation`, the anchor's overlay revision captured now, the `Deadline`, and a
   deterministic equal-deadline tie-break (deadline, then derived child identity — exactly the
   "deterministic within-resolution sequence" §10.1 asks for). One scope-owned driver — one per
   scope owner, never per object, never per pending revert — wakes at the earliest pending deadline;
   each due entry becomes a normalized FND-03 authoritative input and mints a new
   `RuntimeExecutionOrdinal` when the scope's own ordinal issuer accepts it for resolution (§10.2,
   reusing `RuntimeExecutionOrdinal`/`ScopeRuntimeFence` — evidence above — as one scope-wide
   instance rather than per-`GameSession`). The revert's state/footprint delta reuses `prepare`'s
   pure transition logic; committing it does **not** go through `apply`/`resume_pending`/
   `CommandIngress`, because there is no live client command or session to replay (P1, evidence
   above). Cancellation follows §10.3 exactly: the same fences §4 already requires. The driver and
   the ordinal-issuer promotion are still one real new mechanism each, but the unit it stores and the
   ordinal/cancellation contract it follows are not invented: they are the authored `revert_after_ms`
   field, the already-implemented `Deadline` primitive, and the already-accepted FND-03 §10 timer
   contract.
3. **A new scope-owned monotonic logical step counter (considered, not recommended).** Round 1 of
   this review recommended a synthetic per-scope "step" incremented by the scope runtime's own
   cadence. Honest comparison against option 2:
   - *Authored-duration fidelity.* `revert_after_ms` is authored in milliseconds; a `Deadline`
     represents that directly. A step counter forces an arbitrary translation from milliseconds to
     "steps" — and the bounded grep evidence above found no existing per-scope cadence to define
     what one step even is, so the unit would be invented, not derived from authored content.
   - *No arbitrary cadence/unit.* Option 2 reuses `crates/foundation::time` as-is; option 3 still
     needs the same new driver as option 2, plus an invented step-to-millisecond mapping on top of
     it.
   - *Determinism/replay.* Both are equally replay-safe in this proposal's sense (idempotent derived
     identity, no re-fire after commit — see test obligations below); `ManualClock` (evidence above)
     already gives deterministic, test-controlled time for the `Deadline` option, so option 2 does
     not trade away deterministic testing to gain authored-duration fidelity.
   - *Reset on scope restart.* Identical for both: both are `scope_generation`-scoped state owned by
     the same `ChannelRuntimeV1`/`InstanceRuntime` instance as the rest of the overlay, so a scope
     restart (a new instance, per the corrected evidence above) drops both with no separate cleanup
     path.
   Option 3 is strictly dominated: it carries every cost of option 2 (new driver, new per-scope
   state) plus an invented unit option 2 does not need. Superseded by option 2.
4. **Purely reactive re-evaluation against an already-existing counter** (`LocalObjectRuntime`'s
   own `revision`, or `scope_generation`). Rejected: both only advance when something else already
   happens to that same object or scope — `scope_generation` in particular is now shown (corrected
   evidence above) to be fixed for the entire life of a production `ChannelRuntimeV1` instance, so it
   never advances at all short of a full scope restart — so an untouched decaying wall or timed door
   would never re-evaluate and could never fire. This does not satisfy the covered `decay`/
   `revertItem`/`addEvent` use case in §4.

### Must-decide-now test

1. **Must decide now?** `YES` for the *owner, input shape and FND-03 binding* (option 2: a scope-
   owned `Deadline` fired as an FND-03 §10 authoritative timer with its own ordinal/cancellation/
   capacity contract, never a client `LocalObjectCommand` — vs. the record of why 1, 3 and 4 are
   rejected/superseded). `NO` for the driver's exact wake mechanism, whether `ScopeRuntimeFence` is
   promoted to a scope-wide instance or a new scope-owned ordinal issuer is introduced, the exact
   pending-set storage representation, and the concrete timer-capacity numeric bound (FND-03 §14.1:
   "Concrete numeric limits gate implementation, not this architecture decision" — it belongs in
   `RESOURCE_LIMITS_REGISTRY.json`). Those belong to the owning lane's implementation.
2. **What is blocked?** CW4 cannot ship `revert_after` at all (coordinator direction in §4 already
   withholds it) until some owner, input shape and commit path is named; naming nothing leaves the
   `EVIDENCE_GAP` open indefinitely, and building an ad hoc timer without the FND-03 binding would
   need reworking once FND-03 conformance is checked.
3. **What becomes harder later?** Picking option 1 later, after option 2 ships, would require
   migrating every stored `revert_after` target from a scope-local `Deadline` to a global tick
   value — real but bounded migration cost, not an irreversible one, since both are monotonic,
   FND-03-timer-shaped and scoped to the same overlay lifetime.
4. **What evidence would supersede this?** A later, separately accepted Foundation decision that
   introduces a real global simulation tick for reasons independent of `revert_after` (SIM-
   DETERMINISM-01 would have to be amended first); or measured evidence that a per-scope `Deadline`
   cannot meet a specific product timing requirement (for example cross-scope revert ordering,
   which §4 already excludes).
5. **What is deliberately not decided?** The exact wake mechanism inside the scope's step driver, the
   exact way `RuntimeExecutionOrdinal`/`ScopeRuntimeFence` is made scope-wide, the exact storage
   representation of the pending-timer set, and the concrete timer-capacity bound in
   `RESOURCE_LIMITS_REGISTRY.json`. Those belong to the owning lane's implementation, not this
   architecture delta.

### Exact delta the owning lane must provide

Owner: the scope-runtime/Foundation carrier lane behind `ChannelRuntimeV1`/`InstanceRuntime`
(`apps/game-server/src/foundation/runtime_actor_carrier.rs`), coordinated with the CW4 world-runtime
lane (`apps/game-server/src/world_runtime.rs`) that owns `LocalObjectRuntime`. This document does
not implement it; it is CANDIDATE and not owner-accepted.

- Introduce the scope owner's own step driver: one per scope owner (`ChannelRuntimeV1`/
  `InstanceRuntime`), never a per-object or per-revert timer, that wakes at the earliest pending
  `Deadline` and drives all of that scope's own due reverts forward in one pass. Whether it
  piggybacks on a cadence the scope runtime is later shown to already have, or adds one new minimal
  wake for exactly this purpose, is the owning lane's implementation choice — either way it is one
  mechanism per scope, not new infrastructure per object.
- Provide one scope-wide instance of the FND-03 §10.2 ordinal issuer
  (`RuntimeExecutionOrdinal`/`ScopeRuntimeFence`, currently instantiated per `GameSession` in
  `apps/game-server/src/foundation/admission.rs`), or an equivalent scope-owned issuer, so every
  accepted due timer mints its ordinal through the same single-owner sequence as any other scope
  mutation, per §10.2.
- Provide one shared `MonotonicClock` instance per scope (constructed once, never `SystemClock::new()`
  called again per call site) and use it for every `Deadline::after` at commit time and every
  `has_elapsed`/`remaining` at wake time for that scope — never mix two clock instances (evidence
  above: `SystemClock::new()` starts a fresh, incomparable origin each time).
- On any `revert_after`-carrying overlay operation, as part of the *same* staged commit as the
  original `TRANSFORM`/`CREATE`/`REMOVE`/`RETAG` (FND-03 §15.4): reserve safe bounded timer capacity
  first; if none is available, fail the *entire* original operation before anything commits — no
  object mutation and no partial timer entry survive. If capacity is available, compute
  `Deadline::after(clock, Duration::from_millis(revert_after_ms))` and register the FND-03 §10.1
  scheduling key (World/Channel/InstanceId, `scope_generation`, the overlay revision of the anchor,
  the `Deadline`, and the (deadline, then derived child identity) tie-break) alongside the derived
  child identity (as in §3/§4). This state is `scope_generation`-scoped, owned by the same
  `ChannelRuntimeV1`/`InstanceRuntime` instance as the rest of the overlay; a scope restart is a new
  instance (corrected evidence above), so it is dropped with no separate cleanup path. Map the
  capacity failure to `CAPACITY_EXCEEDED` (FND-03 §28) and register the concrete numeric bound in
  `RESOURCE_LIMITS_REGISTRY.json` per FND-03 §14.1 — that number is not decided here.
- On each driver wake, present every due entry as a normalized FND-03 §10.2 authoritative input, in
  the deterministic (deadline, then derived child identity) tie-break order when deadlines are
  equal: mint its `RuntimeExecutionOrdinal` via the scope's ordinal issuer's `accept_input(current
  scope_generation)`, reuse `prepare`'s pure state/footprint-transition logic for the delta, and
  commit through a scope-authority path (`PreparedMutation::Publish`/`TerminalSemanticOutcome`,
  `apps/game-server/src/world_runtime.rs` ~982-996) — never `apply`/`resume_pending`/`CommandIngress`,
  which require a live `GameSessionAuthoritySnapshot` this timer does not have (P1, evidence above).
- Cancellation follows FND-03 §10.3 exactly: `scope_generation` changed, the anchor's overlay
  revision no longer matches what was captured at scheduling, or explicit invalidation each discard
  the pending entry without mutating.
- On an occupancy conflict, terminalize `DISPOSITION_OCCUPIED` for that revert's one derived
  identity and stop; do not retry it on a later wake (decided below, not left open).

### Exact test obligations

- **Fires once.** Once a due entry's `RuntimeExecutionOrdinal` is minted and its state/footprint
  delta commits, the entry is removed from the pending set; the same physical timer entry cannot be
  presented to `accept_input` a second time, so a later wake never refires it.
- **Replay-safe.** A due entry re-presented before it is removed (for example a driver wake that
  overlaps its own commit) must not mint a second `RuntimeExecutionOrdinal` or commit twice for the
  same entry; the scope's ordinal issuer's single-owner, monotonic `accept_input` (evidence above)
  and the pending-set removal together make a second acceptance for the same entry impossible, not
  merely a deterministic no-op.
- **Fenced (FND-03 §10.1/§10.3).** The revert commits only under the same World/Channel/InstanceId,
  `scope_generation`, `content_generation` and overlay-revision-of-anchor fences as any other §4
  operation; per §10.3, a `scope_generation` change, a target overlay-revision mismatch, or explicit
  invalidation each cancel the pending entry rather than letting it mutate.
- **Cleared on scope restart.** The pending-timer set (keyed by `Deadline`) is
  `scope_generation`-scoped state owned by the same `ChannelRuntimeV1`/`InstanceRuntime` instance as
  the rest of the overlay; a scope restart is a new instance (corrected evidence above), so it is
  dropped with no separate cleanup path — the same lifetime §4 already states ("Lifetime:
  Scope-ephemeral") and the same trigger FND-03 §10.3 already names ("scope ownership generation
  changed"). Because it never crosses a process lifetime, FND-03 §9's durable-encoding requirement
  does not apply here (evidence above) — this is a positive test, not merely an absence.
- **No partial footprint.** A revert applies its full target state in the same single commit as any
  other overlay operation (the existing `PreparedMutation::Publish` path); this is exactly the C3
  fixed-footprint boundary in §4 — only the anchor's pre-authored, bind-time-reserved footprint is
  ever touched, never a partially materialized one.
- **Occupied target cells refuse deterministically (decided, not deferred).**
  `apps/game-server/src/world_runtime.rs` `terminalize_current` (~834-846) commits and terminalizes
  every prepared outcome in the same call, and `resume_pending`'s
  `DuplicateDisposition::ReplayRetainedOutcome` (~817-823) replays that same terminal outcome for
  any later submission with the same derived identity — it does not re-evaluate against later
  occupancy. Reusing this path therefore cannot implement "retry later" for a revert: a revert whose
  target state's footprint conflicts with currently occupied cells terminalizes as
  `DISPOSITION_OCCUPIED` (`apps/game-server/src/world_runtime.rs` ~965-969) for that one derived
  identity and is refused permanently — the same "no silent search for a free tile unless the
  definition names one" discipline §3 already applies to a blocked relocation. There is no retry:
  giving a revert a fresh identity per wake to work around the terminal/replay path would be new
  per-object retry machinery this decision does not introduce. If a revert must eventually succeed
  despite occupancy, that is a future definition-level requirement (the definition names a
  fallback), not a hidden retry loop here.
- **Timer-capacity atomicity (FND-03 §15.4).** If safe bounded timer capacity is unavailable when a
  `revert_after`-carrying operation is about to commit, the *entire* original operation fails before
  anything commits: neither the world-object mutation nor a partial/orphaned timer entry survive.
  An already-accepted timer already in the pending set is never discarded merely because the due
  queue is congested (FND-03 §15.4, second sentence) — that pressure produces `CAPACITY_EXCEEDED` on
  the *next incoming* operation, not eviction of an existing entry.
- **Equal-deadline determinism (FND-03 §10.1/§10.2).** Two or more due entries sharing the identical
  `Deadline` resolve in the stable (deadline, then derived child identity) tie-break order, and each
  mints its own new `RuntimeExecutionOrdinal` in that order; re-running the same set of due entries
  against the same clock and `scope_generation` must always reproduce the same order and the same
  ordinals (a determinism/replay test, per FND-03 §11's "multiple equal-deadline timers" case).
- **Single clock origin (detects cross-clock comparison).** A test constructs two independent
  `MonotonicClock` instances (for example two `SystemClock`s, or a `SystemClock` and a `ManualClock`),
  computes a `Deadline` under one, and asserts the implementation's design makes it impossible to
  evaluate `has_elapsed`/`remaining` for that `Deadline` against the other — only one clock reference
  is reachable from both the commit-time scheduling call and the wake-time check for a given scope.
  This guards against exactly the hazard the evidence above proves: `SystemClock::new()` starts a
  fresh, incomparable origin on every call.

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
