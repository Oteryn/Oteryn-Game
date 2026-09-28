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

  The CW3 Content-model worker (allocation `OTV2-20260928-cw3-local-object-state-model`, PR #1046,
  merged) and the CW4 runtime worker (PR #1055, merged 2026-09-28) already shipped the delta this
  operation table depended on:
  - **1a.** Per-state collision presence on a `LocalObject` definition: each declared state is
    `{key, collision: Present | Absent}`. `apps/game-server/src/world_runtime.rs`'s `prepare`
    (~973-1055) now derives the next blocking footprint and the `OCCUPIED` check from the *target*
    state's own authored collision presence (PROVEN, read directly), replacing the former
    hard-wiring to the Open/Close two-state pair.
  - **1b.** An authored initial state on a `LocalObject` placement, validated fail-closed:
    `LocalObjectRuntime::bind` (~590-750) reads `placement.local_object_initial_state` and rejects
    binding when it is absent from the definition's declared states (PROVEN, read directly), rather
    than defaulting silently.
  - **1c.** `RETAG` is the transition between two states of the same collision class that differ
    only in interaction binding (for example, re-arming which trigger fires), under
    `LOCAL_OBJECT_RETAG_INTENT_FAMILY`, with no action-id field — distinct from `TRANSFORM`, which
    may also change collision class. `bind` now accepts an arbitrary, non-empty set of pre-authored
    `TransitionKey`s over the definition's full state vocabulary; `LocalObjectOperation` (~397) is a
    thin wrapper around one bound `TransitionKey`, so `TRANSFORM`/`CREATE`/`REMOVE`/`RETAG` share
    one execution path instead of four Rust variants (PROVEN, read directly).

  CW4 shipped (PR #1055, merged) `TRANSFORM`/`CREATE`/`REMOVE`/`RETAG` **without** `revert_after`,
  per D38 W1; `revert_after` still awaits §7 below being accepted.

- **Timed revert.** Any non-timer-origin operation may carry `revert_after`, which covers `decay`
  and `revertItem`/`addEvent` reverts — a player/command operation, an encounter/server-event-
  originated one (for example a boss-death map-object transform), or any other authoritative input
  that is not itself the firing of a pending revert. The revert is the same owner's own later
  operation, with its own identity derived from the first, and firing it never itself schedules
  another timer — even when the inverse transition it executes also carries `revert_after`, so a
  mutually timed pair cannot ping-pong (§7 Round 7/8). §7 records the still-open decision on what
  logical progression input `revert_after` is measured against; nothing here is implemented yet.
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
    (`apps/game-server/src/world_runtime.rs` ~716) and every later `CREATE`/`REMOVE`/`TRANSFORM`/
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
- The 1a/1b/1c Content-model delta (§4, now merged) generalizes a formerly hard-wired two-state
  runtime field (collision presence) and a formerly hard-wired binding default (initial state); it
  added no new owner, persistence or domain. C3 does not add multi-scope or dynamic geometry; it
  only names the footprint boundary the runtime already enforces by computing collision cells once
  at bind time.
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

Round 4 correction (owner-authorized, after PR #1055/#1046 merged CW4's typed runtime and CW3's
Content-model delta — §4/§8 below now cite the merged state): three remaining gaps were all in FND-03
timer *internals*, resolved by binding tighter to FND-03 rather than by re-designing it. The
equal-deadline tie-break is §10.1's own — the scheduling resolution's `RuntimeExecutionOrdinal` plus
its within-resolution sequence, retained in the timer key — never the revert's derived child
identity. De-duplication happens by an atomic pending-removal *before* `ScopeRuntimeFence::
accept_input` is called, because `accept_input` itself tracks no timer identity. Due-work admission
is bounded per owner cycle under FND-03 §7/§14, the same shape `MovementOwnerTurn` already uses for
a different input source.

Round 5 correction (owner-authorized): a scheduled revert previously retained only fencing/deadline/
ordering data, but firing still has to call `prepare`, which needs a concrete *bound* `TransitionKey`
(evidence below) — nothing said which one restores the object, especially for an arbitrary
`TRANSFORM`/`RETAG`. Fixed by requiring, fail-closed, that `revert_after_ms` is admissible only on a
bound transition whose *inverse* is itself authored and bound for the same placement definition, and
by having the staged commit store that inverse key with the exact expected post-operation state and
revision `prepare` already computes — so firing never guesses a delta, it replays a specific,
pre-validated one.

Round 6 correction (owner-authorized): two consistency fixes inside round 5's own design. First, the
inverse-transition check must pick a *unique* inverse — requiring only "a bound transition with
swapped states" without also requiring exactly one match, and a matching inverse intent family, left
two candidate inverses unresolved; `bind` now rejects unless exactly one qualifies. Second, an
earlier draft of this section discarded a due timer on an overlay-revision mismatch *before* calling
`prepare`, which meant the `DISPOSITION_STALE_STATE` outcome the test obligations promised for that
same case could never actually occur — two paths claiming to handle one case. Fixed by making it one
path: pre-`prepare` discard is now reserved for fences that invalidate the timer itself
(`scope_generation`/`content_generation` changing at the time of round 6; round 9 below adds a third,
`incarnation`), and a due revert whose target identity still matches reaches `prepare` with its
stored inverse key and expected revision, so a same-incarnation changed anchor is rejected there, by
the same mechanism any other command already uses.

Round 7 correction (owner-authorized): if both a transition and its bound inverse each carry
`revert_after_ms` (a mutually timed pair — e.g. a wall that decays and a re-creation that also
expires), firing one timer executes the inverse, and the inverse's own commit is itself
`revert_after_ms`-carrying — so a timer that also stages a new timer for whatever it just did would
ping-pong the two transitions back and forth forever. Fixed, minimal and one-shot: a timer-origin
execution never registers a new timer for itself, even when its own transition carries
`revert_after_ms`. No periodic or repeating timer semantics are introduced.

Round 8 correction (owner-authorized): round 7's own fix over-restricted *who else* registers a
timer to "player/command-initiated ... via `apply`/`resume_pending`," but a `revert_after_ms`-carrying
operation need not arrive that way at all — `DepthWarzoneBossDeath`'s `creature_died(boss)` →
`map_item(transform teleporter at anchor, revert_after_ms 1200000)`
(`docs/architecture/OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md` lines 171-172) is an encounter/
server-event-originated overlay operation, never a client command, so round 7's wording would have
left its teleporter transformed forever. The real rule is an origin test, not an allow-list of entry
points: *is this execution the firing of a pending revert timer?* If yes, suppress registration (that
is the whole point of round 7's fix). If no — whatever authoritative input produced it, player/
command via `apply`/`resume_pending`, an encounter/server-event-originated overlay operation, or any
other non-timer authoritative input — it registers its one-shot revert exactly as any other
`revert_after_ms`-carrying operation does, under the same staged capacity reservation.

Round 9 correction (owner-authorized): the pending entry described through round 8 stores fences and
a `Deadline` but never the target's own identity — which `PlacementKey`, `incarnation` or
`content_generation` it belongs to (evidence above: `LocalObjectCommand` requires all three, and
`prepare`'s first check rejects a mismatch on any of them with `DISPOSITION_BINDING_MISMATCH`, before
ever reaching `expected_revision`/state). The overlay revision alone cannot stand in for that
identity: it is a per-anchor local counter, so an old timer could fire against a different anchor
whose revision happens to match, or against a replacement incarnation at the same anchor. Fixed: the
pending entry now also retains the exact `PlacementKey`, `incarnation` and `content_generation` of
its target, captured at scheduling time alongside the inverse key and expected state/revision; the
firing path addresses the object with exactly those stored values, never a lookup or a guess. A
changed `incarnation` is added to the pre-`prepare` discard class — the timer itself is invalid, the
same class as `scope_generation`/`content_generation` — while a same-incarnation state or revision
change still goes through `prepare`'s own `DISPOSITION_STALE_STATE`, unchanged from round 6.

Round 11 correction (owner-authorized): every round from the original C2 condition onward asserted
the revert has its own "derived child identity" for fire-once/de-duplication and terminalization
(evidence below: GAME-INTERACTION-01 §5.1's nested-cascade rule, already implemented in
`apps/game-server/src/interaction/identity.rs`), but no round ever added that identity to what the
pending entry actually stores — the same one-field-per-round gap this section has now hit repeatedly
(rounds 9, 10 and now 11). Fixed by closing it once: §7 now carries one canonical **"Pending entry:
complete field list"** (start of Exact delta below) that every other place in this section references
instead of re-enumerating fields, and that list adds the revert's own `InteractionChildOccurrenceRef`
— computed once at scheduling time as a nested child of the original operation's own already-known
GAME-INTERACTION identity (§3/§4), *never* re-derived at firing and *never* derived from the
scheduling `RuntimeExecutionOrdinal`, which GAME-INTERACTION-01 §4.4 names as authority-fence
evidence, not logical occurrence identity (§5.8: "Transient runtime ownership generation MUST NOT
create a new logical child"). Firing terminalizes every disposition — `COMMITTED`, `STALE_STATE`,
`OCCUPIED` — under this one stored identity, so a redelivery of the same occurrence and a genuinely
distinct occurrence are told apart the same way any other GAME-INTERACTION child already is.

### Problem

`revert_after` needs *some* value that advances on its own, independent of whether a player ever
sends another command to the affected anchor — a decayed wall or a timed door must still revert if
nobody touches it again, and even if the player who triggered the original operation has since
disconnected. No such input, and no non-client-command commit path for it, exists in the read code
today.

### Evidence

- PROVEN (`apps/game-server/src/world_runtime.rs` `LocalObjectRuntime::bind` ~590-750 and
  `::prepare` ~973-1055, current post-#1055-merge line numbers): neither takes a tick, timestamp or
  step parameter; every fence is placement/incarnation/content_generation/expected_revision, all of
  which only change when a command targets that same anchor.
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
  line 144): the map-object authoring format already names `revert_after_ms` (milliseconds), with a
  concrete authored value at line 172 (`revert_after_ms 1200000`).
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
  `ScopeRuntimeFence::accept_input(generation)` (~1040-1050) mints a new `RuntimeExecutionOrdinal`
  only for the exact current `ScopeOwnershipGeneration`, rejecting a stale one, matching §10.3's
  generation-changed cancellation exactly. It is currently instantiated per `GameSession`
  (`apps/game-server/src/foundation/admission.rs` ~365-381, ~728), not yet as one scope-wide instance
  consumed by `world_runtime.rs`. `accept_input` takes only a `ScopeOwnershipGeneration`: it tracks
  no timer/command identity and cannot by itself detect that a given due timer was already accepted
  — a second call for the same already-fired entry would mint a second, distinct ordinal with
  nothing to stop it.
- PROVEN (`apps/game-server/src/world_runtime.rs` `apply`/`resume_pending` ~817-873,
  `validate_current_authority` ~890-950): both require a live `GameSessionAuthoritySnapshot`
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
- PROVEN (FND-03 §7 "Cross-session and cross-source ordering", lines 322-341): "normalized ready
  inputs from different sources are admitted through bounded arbitration"; "one continuously busy
  session or timer/work source cannot monopolize the owner indefinitely"; "the current owner assigns
  the resulting `RuntimeExecutionOrdinal`." FND-03 §14 item 4 (line ~515) separately lists "timer
  population per scope and due/catch-up work" among the runtime classes §14.1 requires an explicit
  bound for. PROVEN (`apps/game-server/src/movement.rs` `MovementOwnerTurn` ~201-249, already cited
  above): the codebase already has exactly this pattern for a different input source — a bounded
  `max_inputs` batch per invocation, with the caller deciding what to offer next turn — a direct
  precedent for bounding due-timer admission the same way.
- PROVEN (`apps/game-server/src/world_runtime.rs` `LocalObjectCommand` ~412-419,
  `LocalObjectOperation` ~397, `transition_for` ~787-796, `prepare` ~973-1055): every prepared
  mutation is keyed by a concrete, already-*bound* `TransitionKey` — `transition_for` looks it up in
  `self.transitions` (the map `bind` populated from the `transition_keys` this instance was bound
  with) and fails the whole binding if it is absent. Firing a revert therefore cannot invoke "the
  opposite of whatever happened"; it must name one specific bound `TransitionKey` up front.
  `LocalObjectCommand` (~412-419) requires `placement`, `incarnation` and `content_generation` in
  addition to the operation and `expected_revision`, and `prepare`'s *first* check (~978-987) rejects
  with `DISPOSITION_BINDING_MISMATCH` — a different outcome from `DISPOSITION_STALE_STATE` — if any
  of those three do not match `self`. The pending entry's stored "overlay revision" alone cannot
  address an object: it is a per-anchor local counter, so two different anchors can coincidentally
  share a revision, and nothing before round 9 stored which anchor, incarnation or content
  generation a given timer targets.
  `TransitionBinding` (`apps/game-server/src/content/reference_playable.rs` ~1334-1342) has `key`,
  `source_state`, `target_state`, `normalized_intent_family` and no `revert_after_ms` field today —
  nothing currently associates a transition with an inverse, and CW4 shipped without `revert_after`
  (§4), so this is unimplemented, not merely unbound. PROVEN (grep of
  `apps/game-server/src/world_runtime.rs`): only `LOCAL_OBJECT_RETAG_INTENT_FAMILY` (~1195, ~1646)
  is a named intent-family constant today; no `..._TRANSFORM_...`/`..._CREATE_...`/`..._REMOVE_...`/
  `..._OPEN_...`/`..._CLOSE_...` family constant exists in the merged runtime — the
  TRANSFORM/CREATE/REMOVE/OPEN/CLOSE side of the inverse-family pairing below is a naming scheme the
  owning lane still has to establish, not a check already enforced anywhere.
- PROVEN (`docs/architecture/GAME-INTERACTION-01_SUCCESSOR_CHILD_IDENTITY_RETRY_CONTRACT_CANDIDATE.md`
  §4.1/§4.4/§5.1/§5.8): `RootSourceOccurrenceRef` is "a stable authoritative source occurrence
  accepted by its owning domain"; `InteractionChildOccurrenceRef` normatively equals
  `(ParentSourceOccurrenceRef, InteractionDefinitionRef, AuthoritativeTargetDiscriminator,
  TypedEdgeOrCapabilityDiscriminator, OptionalCanonicalChildOrdinal, SemanticRevisionContext)`; for a
  nested cascade "`ParentSourceOccurrenceRef` = parent `InteractionChildOccurrenceRef`"; and §4.4/§5.8
  are explicit that `AuthorityFenceEvidence` (current ownership generation, state/domain revisions —
  exactly what a `RuntimeExecutionOrdinal` is) "is not automatically logical occurrence identity" and
  "MUST NOT create a new logical child." PROVEN (`apps/game-server/src/interaction/identity.rs`
  `RootSourceOccurrenceRef` ~15-22, `ChildOccurrenceRef`/`ParentOccurrenceRef` ~69-157): this is
  already implemented — `ChildOccurrenceRef::for_child(parent: &Self, definition, target, edge,
  ordinal, revisions)` builds exactly the nested-cascade shape §5.1 describes, and `root()` walks the
  `parent` chain back to the originating `RootSourceOccurrenceRef`. Nothing before round 11 named
  which of these the pending entry stores; every prior round's text asserted a "derived child
  identity" existed without saying where it came from or that it was retained.

### Options (minimum real set)

1. **Reuse an existing Foundation/global simulation tick.** Rejected as not currently available:
   no such tick exists (evidence above), and SIM-DETERMINISM-01 explicitly defers ever requiring
   one. Adopting or creating one now would be a new Foundation-owned decision, out of this task's
   `excluded_scope` (Foundation/runtime/protocol/registry) and disproportionate to one `revert_after`
   field.
2. **An FND-03 §10 authoritative timer bound to a monotonic `Deadline` computed from the authored
   duration (RECOMMENDED).** `revert_after_ms` is admissible only on a bound transition with exactly
   one *inverse* — a transition bound on the same runtime instance, for the same placement
   definition, whose `source_state`/`target_state` are this transition's swapped *and* whose
   `normalized_intent_family` is this transition's matching inverse family (TRANSFORM↔TRANSFORM,
   CREATE↔REMOVE, RETAG↔RETAG, OPEN↔CLOSE); `bind` rejects (`InvalidBinding`) a `revert_after_ms`-
   carrying transition with zero such matches or more than one — an ambiguous inverse is exactly as
   invalid as a missing one — so firing never has to guess which delta restores the object (evidence
   above). At commit time, for any non-timer-origin operation — every authoritative input except the
   firing of a pending revert timer itself, which never re-arms itself even when its own transition
   also carries `revert_after_ms` (Round 7/8) — as part of the *same* staged commit as the original
   `TRANSFORM`/`CREATE`/`REMOVE`/`RETAG` (FND-03 §15.4, below), register a pending entry under an
   FND-03 §10.1 scheduling key. The entry's exact contents — every field it needs and why — are the
   single canonical **"Pending entry: complete field list"** at the start of Exact delta below; this
   option does not repeat them. Among them is the revert's own `InteractionChildOccurrenceRef`
   (Round 11, evidence above), computed once now as a nested child of the original operation's own
   GAME-INTERACTION identity (§3/§4) and never re-derived from the scheduling `RuntimeExecutionOrdinal`,
   which stays a pure ordering/authority value (evidence above), never logical identity. One
   scope-owned driver — one per scope owner, never per object, never per pending revert — wakes at
   the earliest pending deadline and admits at most a registered bounded batch of due entries per
   cycle (FND-03 §7/§14, evidence above), yielding the remainder to a later cycle through the same
   owner arbitration any other input source uses, so a same-deadline burst cannot starve unrelated
   scope work. For each admitted entry, the driver first atomically transitions it out of the pending
   set (removed/in-flight) and only then calls the scope's ordinal issuer's `accept_input` (§10.2,
   reusing `RuntimeExecutionOrdinal`/`ScopeRuntimeFence` — evidence above — as one scope-wide
   instance rather than per-`GameSession`) — `accept_input` itself tracks no timer identity (evidence
   above), so this ordering is what makes a re-presented, already-fired entry a no-op instead of a
   second ordinal. The revert's state/footprint delta reuses `prepare`'s pure transition logic;
   committing it does **not** go through `apply`/`resume_pending`/`CommandIngress`, because there is
   no live client command or session to replay (P1, evidence above). Pre-`prepare` discard is
   reserved for fences that invalidate the timer itself — `scope_generation`, `content_generation` or
   the target's `incarnation` changing (Round 9) — never for a changed object: a due revert whose
   stored target identity still matches always reaches `prepare` with its stored inverse key and
   expected revision, one path, and `prepare`'s own stale-state check is what accepts or rejects a
   same-incarnation changed anchor (below). The driver and the ordinal-issuer promotion are
   still one real new mechanism each, but the unit it stores, the tie-break it uses and the
   ordinal/cancellation contract it follows are not invented: they are the authored `revert_after_ms`
   field, the already-implemented `Deadline` primitive, the already-implemented `ChildOccurrenceRef`
   nested-identity mechanics, and the already-accepted FND-03 §7/§10/§14 timer contract.
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
   rejected/superseded), including its equal-deadline tie-break (§10.1's own ordinal-plus-sequence,
   not the revert's derived child identity), its atomic pending-removal-before-`accept_input`
   de-duplication, bounded per-cycle due-work admission (§7/§14), the fail-closed unique-bound-inverse
   precondition on `revert_after_ms` itself (zero or ambiguous inverse ⇒ `InvalidBinding`, never a
   guessed delta), the single firing path (a changed object is rejected only inside `prepare`, never
   by a separate pre-`prepare` cancellation), and the one-shot origin test (suppress registration
   only when this execution *is* the firing of a pending revert timer; every other authoritative
   origin — player/command, encounter/server-event, or otherwise — registers its own one-shot
   revert as normal, so a mutually timed pair cannot ping-pong and an encounter-originated timed
   transform is not silently starved of its revert; no periodic/repeating semantics), and the
   complete stored pending-entry field list (Round 11, "Exact delta" below) — target identity
   (`PlacementKey`/`incarnation`/`content_generation`, addressed exactly and never re-derived from
   the overlay revision alone, with a changed `incarnation` like `scope_generation`/
   `content_generation` discarding the timer pre-`prepare`) *and* the revert's own derived
   `InteractionChildOccurrenceRef` (GAME-INTERACTION-01 §5.1's nested-cascade rule, computed once at
   scheduling time, never re-derived from the scheduling ordinal per §4.4/§5.8) — all eight are bound
   to existing FND-03 sections, GAME-INTERACTION-01's already-implemented identity model, or the
   merged CW4 bind-time model, not open design questions. `NO` for the driver's exact wake
   mechanism, whether `ScopeRuntimeFence` is
   promoted to a scope-wide instance or a new scope-owned ordinal issuer is introduced, the exact
   pending-set storage representation, the exact field/encoding of `revert_after_ms` on
   `TransitionBinding` or its content source, and the concrete timer-capacity/due-batch numeric
   bounds (FND-03 §14.1: "Concrete numeric limits gate implementation, not this architecture
   decision" — they belong in `RESOURCE_LIMITS_REGISTRY.json`). Those belong to the owning lane's
   implementation.
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
   representation of the pending-timer set, the exact field/encoding of `revert_after_ms` on
   `TransitionBinding` or its content-authoring source, and the concrete timer-capacity and
   per-cycle due-batch numeric bounds in `RESOURCE_LIMITS_REGISTRY.json`. Those belong to the owning
   lane's implementation, not this architecture delta.

### Exact delta the owning lane must provide

Owner: the scope-runtime/Foundation carrier lane behind `ChannelRuntimeV1`/`InstanceRuntime`
(`apps/game-server/src/foundation/runtime_actor_carrier.rs`), coordinated with the CW4 world-runtime
lane (`apps/game-server/src/world_runtime.rs`) that owns `LocalObjectRuntime`. This document does
not implement it; it is CANDIDATE and not owner-accepted.

**Pending entry: complete field list (Round 11).** Every field below is captured exactly once, at
scheduling time, from the original operation's own already-validated commit; firing never looks up,
re-derives or guesses any of them. Every other place in this section (Option 2, and each bullet
below) refers back to this one list instead of re-enumerating fields.

| Field | Consumed for | Source at scheduling time |
|---|---|---|
| World/Channel/InstanceId | scope identity (fence) | the scope's own binding |
| `scope_generation` | timer-invalidating fence, pre-`prepare` discard | the scope's own binding |
| `PlacementKey` | which anchor to address | the original operation's own placement |
| `incarnation` | timer-invalidating fence, pre-`prepare` discard (Round 9) | the original operation's own target |
| `content_generation` | timer-invalidating fence, pre-`prepare` discard | the original operation's own content pin |
| `Deadline` | when the timer becomes due (ordering) | `Deadline::after(clock, revert_after_ms)` |
| scheduling `RuntimeExecutionOrdinal` + within-resolution sequence | equal-deadline tie-break, FND-03 §10.1 (ordering) | the scheduling resolution's own ordinal |
| inverse `TransitionKey` | which delta to replay (`prepare`'s `operation`) | the bind-time unique-inverse check (rounds 5/6) |
| expected post-operation state + overlay revision | `prepare`'s `expected_revision`/stale-state check | `transition.target_state`/`next_revision`, already computed by the original commit |
| the revert's own `InteractionChildOccurrenceRef` | fire-once/de-duplication identity; terminal-outcome identity for every disposition; redelivery detection (Round 11) | a nested child of the original operation's own GAME-INTERACTION identity (§3/§4), never the scheduling ordinal (evidence above) |

Together: `PlacementKey`/`incarnation`/`content_generation`/inverse `TransitionKey`/expected
revision are exactly what `LocalObjectCommand` needs to call `prepare` (evidence above);
World/Channel/InstanceId/`scope_generation` are the same scope-identity fences every other overlay
operation already carries; `Deadline`/ordinal/sequence are FND-03 §10.1's own scheduling key; and
`InteractionChildOccurrenceRef` is the GAME-INTERACTION-01 identity every prior round assumed but
never listed. Capacity release needs no separate stored field: it is implicit in the same atomic
pending-removal step that also enables de-duplication (below) — the scope's own capacity counter is
live scope state, not part of the entry. Nothing the firing path consumes is missing from this list
or derivable only from something outside it plus the scope's own live state (its clock, its ordinal
issuer, its pending set).

- Add `revert_after_ms` as an optional field on the authored transition (`TransitionBinding` or its
  content-authoring source, evidence above), and validate it fail-closed inside `bind`
  (`apps/game-server/src/world_runtime.rs` ~590-750): for every bound transition that carries
  `revert_after_ms`, find every OTHER bound transition for the same `definition` whose
  `source_state` equals this one's `target_state`, whose `target_state` equals this one's
  `source_state`, AND whose `normalized_intent_family` is this one's matching inverse family —
  TRANSFORM↔TRANSFORM, CREATE↔REMOVE, RETAG↔RETAG, OPEN↔CLOSE (this covers TRANSFORM a→b needing
  bound b→a, CREATE needing the bound REMOVE of the same anchor/def, REMOVE needing the bound
  CREATE, and RETAG needing the reverse RETAG; RETAG's own same-collision-class constraint is
  already enforced elsewhere in CW3's linker, so this check adds only the family-pairing rule, not a
  second collision-class check). Reject the whole binding with `WorldRuntimeError::InvalidBinding`
  unless *exactly one* transition matches — zero matches is a missing inverse, more than one is an
  ambiguous inverse, and both are equally invalid — producing the one unique inverse `TransitionKey`
  the pending entry stores (complete field list above).
- Introduce the scope owner's own step driver: one per scope owner (`ChannelRuntimeV1`/
  `InstanceRuntime`), never a per-object or per-revert timer, that wakes at the earliest pending
  `Deadline` and drives that scope's own due reverts forward. Whether it piggybacks on a cadence the
  scope runtime is later shown to already have, or adds one new minimal wake for exactly this
  purpose, is the owning lane's implementation choice — either way it is one mechanism per scope, not
  new infrastructure per object.
- Bound due-work admission per driver wake (FND-03 §7/§14, evidence above): admit at most a
  registered maximum batch of due entries per owner cycle; a remainder above that bound is not
  dropped, it yields back to the same owner arbitration every other input source goes through and is
  presented again on a later cycle. This is the same shape `MovementOwnerTurn`'s bounded
  `max_inputs` already uses for a different input source (evidence above), applied to due timers so
  a same-deadline burst cannot monopolize the scope owner.
- Provide one scope-wide instance of the FND-03 §10.2 ordinal issuer
  (`RuntimeExecutionOrdinal`/`ScopeRuntimeFence`, currently instantiated per `GameSession` in
  `apps/game-server/src/foundation/admission.rs`), or an equivalent scope-owned issuer. Because
  `accept_input` tracks no timer identity by itself (evidence above), de-duplication is the driver's
  responsibility, not the issuer's: for each admitted due entry, atomically transition it out of the
  pending set (remove it, or mark it in-flight) *before* calling `accept_input`; only an entry that
  was still pending at that moment proceeds to mint an ordinal and commit. A due entry presented a
  second time (already removed/in-flight) finds nothing to transition and is a no-op — it never
  reaches `accept_input`, so it never mints a second ordinal.
- Provide one shared `MonotonicClock` instance per scope (constructed once, never `SystemClock::new()`
  called again per call site) and use it for every `Deadline::after` at commit time and every
  `has_elapsed`/`remaining` at wake time for that scope — never mix two clock instances (evidence
  above: `SystemClock::new()` starts a fresh, incomparable origin each time).
- On any `revert_after`-carrying overlay operation whose origin is *not* the firing of a pending
  revert timer (evidence above: player/command via `apply`/`resume_pending`, an encounter/
  server-event-originated overlay operation such as `DepthWarzoneBossDeath`'s boss-death teleporter
  transform, or any other non-timer authoritative input — the test is "is this execution the firing
  of a pending revert timer," not which entry point produced it), as part of the *same* staged
  commit as the original `TRANSFORM`/`CREATE`/`REMOVE`/`RETAG` (FND-03 §15.4): reserve safe bounded
  timer capacity first; if none is available, fail the *entire* original operation before anything
  commits — no object mutation and no partial timer entry survive. If capacity is available, populate
  and register a pending entry with the complete field list above (World/Channel/InstanceId,
  `scope_generation`, `PlacementKey`, `incarnation`, `content_generation`, `Deadline`, scheduling
  `RuntimeExecutionOrdinal`/sequence, inverse `TransitionKey`, expected state/revision, and the
  revert's own `InteractionChildOccurrenceRef` computed now as a nested child of the original
  operation's own identity) under the FND-03 §10.1 scheduling key — so nothing about the revert is
  derived later, only replayed. This state is `scope_generation`-scoped, owned by the same
  `ChannelRuntimeV1`/`InstanceRuntime` instance as the rest of the overlay; a scope restart is a new
  instance (corrected evidence above), so it is dropped with no separate cleanup path. Map the
  capacity failure to `CAPACITY_EXCEEDED` (FND-03 §28) and register the concrete numeric bound in
  `RESOURCE_LIMITS_REGISTRY.json` per FND-03 §14.1 — that number is not decided here.
- On each driver wake, present admitted due entries (bounded above) as normalized FND-03 §10.2
  authoritative inputs, in their stored (deadline, scheduling ordinal, within-resolution sequence)
  tie-break order (field list above): run the pending-removal-before-`accept_input` de-duplication
  step above (after the incarnation/content-generation fence below has already passed), mint the
  surviving entries' `RuntimeExecutionOrdinal`s via the scope's ordinal issuer, then call `prepare`
  addressing exactly the entry's stored `PlacementKey`/`incarnation`/`content_generation` (never
  looked up, never guessed, never re-derived from the overlay revision alone) with the entry's stored
  inverse `TransitionKey` and its stored expected revision as `expected_revision` — reusing
  `prepare`'s existing stale-precondition checks (`DISPOSITION_STALE_STATE`,
  `apps/game-server/src/world_runtime.rs` ~988-1013) exactly as they already work for any command, so
  an object the entry's own fences did not already catch (changed again after this revert was
  scheduled, but still the same incarnation) is rejected there, never guessed at. Every disposition
  this commit reaches — `DISPOSITION_COMMITTED`, `DISPOSITION_STALE_STATE`, `DISPOSITION_OCCUPIED` —
  terminalizes under the entry's stored `InteractionChildOccurrenceRef` (field list above), never a
  freshly derived one: a redelivery of the same due entry finds that identity already terminal and
  replays its outcome instead of re-executing, while a distinct occurrence (a new `revert_after`
  registration, even at the same anchor) carries its own distinct nested-child identity and cannot
  collide with it. Commit through the
  same scope-authority path (`PreparedMutation::Publish`/`TerminalSemanticOutcome`, ~1040-1054) —
  never `apply`/`resume_pending`/`CommandIngress`, which require a live `GameSessionAuthoritySnapshot`
  this timer does not have (P1, evidence above). This commit is a timer-origin execution: it reuses
  only the state/footprint commit mechanics above, never the timer-capacity-reservation-and-register
  step of the previous bullet, even when the inverse `TransitionKey` it just executed itself carries
  `revert_after_ms` (Round 7/8) — a mutually timed pair fires one direction and stops, it does not
  re-arm itself.
- Pre-`prepare` discard (§10.3's "scope ownership generation changed"/"invalidated" triggers) is
  reserved for fences that invalidate the *timer itself*, never for a changed object:
  `scope_generation` or `content_generation` changing clears the pending entry with the rest of the
  scope-ephemeral overlay (same reset as §4's "Lifetime"); the entry's stored `incarnation` no
  longer matching the current object at that `PlacementKey` (Round 9 — a replacement incarnation at
  the same anchor) discards the pending entry the same way — all three are checked before the entry
  is ever presented to `prepare`. A mismatched overlay revision on the anchor, or any other
  same-incarnation state/revision change, is deliberately *not* one of these pre-`prepare` checks —
  that case is the previous bullet's job: the entry still reaches `prepare` with its stored inverse
  key and expected revision, and `prepare`'s own stale-precondition check rejects it there. One path,
  not two, for "the object changed within the same incarnation."
- On an occupancy conflict, terminalize `DISPOSITION_OCCUPIED` for the entry's stored derived
  identity (field list above) and stop; do not retry it on a later wake (decided below, not left open).

### Exact test obligations

- **Fires once, mints exactly one ordinal (P2, de-duplication).** The pending→removed/in-flight
  transition happens *before* `accept_input` is ever called (Exact delta above); a test that
  presents the same due entry twice — once normally, once as if a second driver pass or a crash-
  recovery rescan re-observed it — must observe exactly one `RuntimeExecutionOrdinal` minted and
  exactly one commit; the second presentation finds the entry no longer pending and returns without
  calling `accept_input` at all. This is deliberately not "a deterministic no-op result from
  `accept_input`": `accept_input` itself (evidence above) has no way to recognize a repeat, so the
  guarantee lives entirely in the driver's atomic removal-before-accept ordering, and the test must
  exercise that ordering directly, not just its usual-case effect.
- **Replay-safe.** Re-evaluating "is this due" for an entry already committed and removed must be
  side-effect free: with the entry gone from the pending set, there is nothing left to admit,
  transition or mint an ordinal for.
- **Fenced, one path (P1, FND-03 §10.1/§10.3).** The revert commits only under the same fences named
  in the pending entry's complete field list ("Exact delta" above) as any other §4 operation, but only
  three of them are checked *before* `prepare`: `scope_generation`, `content_generation` or the stored
  `incarnation` no longer matching (Round 9) each discard the pending entry without ever calling
  `prepare` (the timer itself is invalid — §10.3's "scope ownership generation changed"/target-
  generation-mismatch triggers). A mismatched overlay revision (a same-incarnation state/revision
  change) is deliberately *not* checked there; a test asserting it is (a separate pre-`prepare`
  cancellation path for a changed object) must fail — the only path for a same-incarnation changed
  object is `prepare`'s own `DISPOSITION_STALE_STATE` (below).
- **Redelivery replays, a distinct occurrence does not collide (P1, Round 11, GAME-INTERACTION-01
  §5.1/§5.8).** Present the same due pending entry a second time after it has already terminalized
  (any disposition): the second presentation must find its stored `InteractionChildOccurrenceRef`
  already terminal and replay that same outcome, never re-execute `prepare` or mutate state again —
  this is the fire-once obligation above, restated at the identity layer. Separately, schedule two
  distinct `revert_after` registrations that resolve to the same anchor/target/edge (e.g. the same
  transform fired twice at different times, or a mutually timed pair's two directions): each must
  compute its own distinct `InteractionChildOccurrenceRef` (a distinct nested child of its own
  triggering operation's identity, never the scheduling ordinal — §4.4/§5.8), so a test that lets one
  registration's terminal outcome answer for the other's must fail.
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
- **Missing inverse is rejected at bind (P1, fail-closed).** Binding a `revert_after_ms`-carrying
  transition with zero matching bound inverse transitions (Exact delta above) must fail the whole
  `bind` call with `InvalidBinding` — there is no partial binding that accepts the forward operation
  and silently drops its revert.
- **Two candidate inverses are rejected at bind (P2, fail-closed).** Binding a `revert_after_ms`-
  carrying transition with *two or more* bound transitions matching the swapped-states-plus-
  matching-family inverse rule (Exact delta above) must also fail the whole `bind` call with
  `InvalidBinding` — an ambiguous inverse is not resolved by picking one arbitrarily.
- **Revert restores exactly the pre-operation state (P1).** Firing a scheduled revert whose fences
  and expected revision still hold must land the object back in precisely the state it was in
  immediately before the original operation committed — the one unique `source_state`/`target_state`
  pair the bind-time inverse check validated, not an approximation.
- **Intervening change yields STALE_STATE, no mutation, via the one path (P1).** If the object was
  changed again after the revert was scheduled (a later operation on the same anchor moved it away
  from the state the revert's stored expected revision names), firing must reach `prepare` — never a
  pre-`prepare` cancellation — and hit `prepare`'s existing `DISPOSITION_STALE_STATE` path (evidence
  above), committing nothing; the revert never overwrites whatever the object has become in the
  meantime, and there is no second, separate check that could instead silently discard the entry.
- **Two anchors with an equal overlay revision fire independently (P1, Round 9).** Bind two
  different `LocalObjectRuntime`s at two different `PlacementKey`s whose overlay revisions happen to
  be numerically equal, each with its own `revert_after_ms`-carrying operation scheduled. When both
  timers become due, each must fire against its own stored `PlacementKey` and mutate only its own
  object; a test that lets either fire against the other's anchor (because it addressed by overlay
  revision alone instead of the stored `PlacementKey`) must fail.
- **A replacement incarnation discards the old timer (P1, Round 9).** Schedule a revert, then replace
  the target object's incarnation at the same `PlacementKey` (the pre-existing recycling path, not
  something this decision adds). The old timer's entry must be discarded pre-`prepare` when its
  stored `incarnation` no longer matches — it must never reach `prepare`, never mutate the
  replacement incarnation's object, and must not appear as a `DISPOSITION_BINDING_MISMATCH` outcome
  either, since there is no command awaiting a reply for an internally-discarded timer.
- **A mutually timed pair fires once and stops (P2, Round 7).** Bind a `TRANSFORM a→b` and its
  inverse `TRANSFORM b→a` so *both* carry `revert_after_ms`; schedule the forward operation, let its
  timer become due. The test must observe: exactly one `RuntimeExecutionOrdinal` minted, exactly one
  inverse execution (the object lands in `a`), and *zero* new timers registered as part of that
  firing — no pending entry exists afterward for either direction. Re-arming only happens if some
  later non-timer-origin execution (a player/command, an encounter/server-event, or any other
  authoritative input) invokes `b→a` (or `a→b`) itself.
- **An encounter-originated timed transform registers and fires (P1, Round 8).** Bind
  `DepthWarzoneBossDeath`'s `map_item(transform teleporter at anchor, revert_after_ms 1200000)`
  (`OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md` lines 171-172) with its bound inverse. The
  `creature_died(boss)`-triggered transform — an encounter/server-event-originated operation, never
  `apply`/`resume_pending` — must register exactly one revert timer at commit, and that timer must
  later fire and restore the teleporter; a test asserting it stays transformed forever (round 7's
  bug) must fail.
- **Occupied target cells refuse deterministically (decided, not deferred).**
  `apps/game-server/src/world_runtime.rs` `terminalize_current` (~875-888) commits and terminalizes
  every prepared outcome in the same call, and `resume_pending`'s
  `DuplicateDisposition::ReplayRetainedOutcome` (~862-864) replays that same terminal outcome for
  any later submission with the same derived identity — it does not re-evaluate against later
  occupancy. Reusing this path therefore cannot implement "retry later" for a revert: a revert whose
  target state's footprint conflicts with currently occupied cells terminalizes as
  `DISPOSITION_OCCUPIED` (`apps/game-server/src/world_runtime.rs` ~1023-1027) for that one derived
  identity and is refused permanently — the same "no silent search for a free tile unless the
  definition names one" discipline §3 already applies to a blocked relocation. There is no retry:
  giving a revert a fresh identity per wake to work around the terminal/replay path would be new
  per-object retry machinery this decision does not introduce. If a revert must eventually succeed
  despite occupancy, that is a future definition-level requirement (the definition names a
  fallback), not a hidden retry loop here.
- **Timer-capacity atomicity (FND-03 §15.4).** If safe bounded timer capacity is unavailable when
  any non-timer-origin `revert_after`-carrying operation is about to commit — player/command,
  encounter/server-event-originated, or any other authoritative input — the *entire* original
  operation fails before anything commits: neither the world-object mutation nor a partial/orphaned
  timer entry survive. Only a timer-origin execution (the firing path itself) never attempts this
  reservation at all (Round 7/8), so it has nothing to fail on.
  An already-accepted timer already in the pending set is never discarded merely because the due
  queue is congested (FND-03 §15.4, second sentence) — that pressure produces `CAPACITY_EXCEEDED` on
  the *next incoming* operation, not eviction of an existing entry.
- **Equal-deadline order replays scheduling order (P2, FND-03 §10.1).** Two timers scheduled in two
  *different* owner resolutions (different `RuntimeExecutionOrdinal`s at scheduling time) that later
  become due with the identical `Deadline` must fire in the order their scheduling resolutions
  occurred — the earlier-scheduled timer first — not in an order derived from either timer's own
  GAME-INTERACTION child identity, which §10.1 does not name as an ordering key. Re-running the same
  due set against the same clock and `scope_generation` must always reproduce the same order and the
  same newly minted ordinals (a determinism/replay test, per FND-03 §11's "multiple equal-deadline
  timers" case).
- **Bounded due work does not starve other inputs (P2, FND-03 §7/§14).** A burst of due entries above
  the registered per-cycle batch bound does not get admitted in one pass: the driver processes at
  most that bound, and the scope's other input sources (client commands, control/fencing input) still
  get their own owner-arbitration turn before the remainder of the burst is retried — mirroring
  `MovementOwnerTurn`'s bounded `max_inputs` for a different source (evidence above). The remainder
  is retried on a later cycle, not dropped and not retried within the same cycle's bound.
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
5. **CW3 (Content model) — done.** Implemented the 1a/1b/1c delta in §4: per-state collision
   presence, authored initial state validated fail-closed, and the `RETAG`/
   `LOCAL_OBJECT_RETAG_INTENT_FAMILY` decision (PR #1046, merged).
6. **CW4 (runtime) — done.** Shipped `TRANSFORM`/`CREATE`/`REMOVE`/`RETAG` without `revert_after`,
   generalizing `LocalObjectRuntime` off the Open/Close two-state pair, per the coordinator
   direction recorded in §4 (PR #1055, merged 2026-09-28); `revert_after` awaits §7 being accepted.
7. **Scope-runtime / Foundation carrier lane (`ChannelRuntimeV1`/`InstanceRuntime`).** Own §7's
   decision: accept or supersede the recommended `revert_after` progression option and supply the
   exact delta §7 names — including, if no existing scope cadence is proven, the scope's own step
   driver itself. CW4 then adds `revert_after` on top of the already-shipped operations.
