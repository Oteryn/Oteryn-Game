# Interaction relocation and world-object owners: proposal v1

- Date: 2026-09-27
- DecisionStatus: CANDIDATE with owner decisions D37 and D38 taken (§6, 2026-09-27). The contract
  text becomes accepted after the independent review that authority changes require; until then
  the Movement and WorldObject children stay blocked. §7 (`revert_after` progression) is a decision
  delta added after the independent review of D38 returned `ACCEPT_WITH_CONDITIONS`; its direction is
  ACCEPTED by the owner (2026-09-28, issue #162, §7's own `DecisionStatus` line), with several items
  delegated to the owning lane and one (attribute-bearing object state) owner-decided `YES` and under
  design in a further section of this document — see §7 for the exact scope of what is accepted.
  §10 (created local objects: §7 open decisions 8 and 9) is `CANDIDATE`, pending independent
  review (owner decisions of 2026-09-28, issue #162 comment 5879299188).
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
- §7's recommended `revert_after` option adds exactly one new scope-owned step driver plus one
  scope-owned lifecycle-record store (one record per `InteractionChildOccurrenceRef`, `PENDING`→
  `IN_FLIGHT`→`TERMINAL`, §7 Round 14) — both owned by the same scope-runtime owner named in D38, one
  mechanism per scope, never per object — and reuses the already-implemented
  `crates/foundation::time` `Deadline`/`MonotonicClock` primitive rather than inventing a new unit; it
  adds no per-object timer service or durable persistence (§7's evidence: scope-ephemeral, dropped on
  scope restart). It does retain a bounded terminal-outcome record per identity within one live scope
  generation — required by the owning GAME-INTERACTION-01 identity contract (§7 Round 13) — whose
  exact retention/eviction policy §7 leaves to the owning lane (§7 "Open decisions").

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

## 7. `revert_after` logical progression: direction ACCEPTED by owner 2026-09-28 (#162)

- DecisionStatus: direction ACCEPTED by owner 2026-09-28 (issue #162), round 21. Accepted: the
  scope-owned lifecycle record (one record per `InteractionChildOccurrenceRef`, `PENDING`→
  `IN_FLIGHT`→`TERMINAL`, Round 14); commit-only scheduling (a non-timer-origin operation schedules a
  revert only as part of its own staged commit, and only once `prepare` has already returned
  `DISPOSITION_COMMITTED`/`PreparedMutation::Publish`, Round 20/21); the state+collision-only revert
  scope (`revert_after_ms` covers exactly what `LocalObjectStateDefinition`/`PreparedMutation::Publish`
  model today, Round 16); and rejection of attribute-changing transitions at authoring/lowering (Round
  17/19). Delegated to the owning lane, unresolved by this document: open decisions 1, 2, 5, 6, 7 and
  8 below. Open decision 3 (attribute-bearing object state) is owner-decided `YES` — needed for the
  playable path — with its design, and open decision 4's resolution (coupled to it), in a further
  `CANDIDATE` §9 (Round 21 narrative below; task `OTV2-20260928-cw1-attribute-bearing-object-state`);
  §9's own Round 2 narrowed that design to the teleporter-transform shapes only and added open
  decision 8 for the runtime-created-placement gap it found.

The independent review of D38 (2026-09-27/28) returned `ACCEPT_WITH_CONDITIONS` with an
`EVIDENCE_GAP`: `revert_after` names no scheduling mechanism. This section originally answered only
which existing owner supplies the logical progression input `revert_after` is measured against,
without accepting the answer on the owner's behalf; the owner has since accepted this section's
direction (Round 21, `DecisionStatus` above). It binds condition C2 (the revert is the scope
runtime's own later operation: derived child identity, same World/scope/content-generation/
overlay-revision fences, fired from the scope runtime's existing progression, cleared on scope
restart — no per-object timer service, queue, receipt store or persistence; concretely, one
scope-owned lifecycle record per identity, Round 14 below, never a per-object mechanism).

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
`OCCUPIED` — under this one stored identity, distinguishing a distinct occurrence from a repeat by
that identity's nested-child derivation (round 12 below corrects how a *repeat* is actually handled).

Round 12 correction (owner-authorized): round 11 said firing "tells apart" a redelivery of the same
occurrence from a distinct one the way any GAME-INTERACTION child already is — implying redelivery
looks up and replays a stored outcome. Checked directly (evidence below): no store in this codebase
can key a terminal outcome by `InteractionChildOccurrenceRef` — `CommandIngress` (the only
terminal-retention/replay mechanism the codebase has) is `CommandId`-keyed, holds exactly one record
(`MAX_RETAINED_TERMINAL_RECORDS = 1`), and is reachable only through the session-gated
`apply`/`resume_pending` this timer already does not use (P1, rounds 5/6). A timer-origin revert has
no client awaiting a reply, so it needs exactly-once *execution*, not outcome *replay* — and exactly-
once execution already comes entirely from the existing atomic pending-removal-before-`accept_input`
step, unchanged since round 6. Fixed: a re-presentation of an already-removed entry is now specified
as a plain no-op (nothing minted, nothing mutated, nothing looked up); the single terminal outcome
firing does emit still carries the entry's stored `InteractionChildOccurrenceRef` as its identity, but
that outcome is not retained afterward, and adding a store for it is out of scope unless a later
accepted requirement needs a retrievable timer outcome. Client-command replay via `CommandIngress` is
unaffected — this correction is timer-origin-firing-only.

**Round 12 is superseded by round 13 below: it was wrong.** Round 12 treated "no existing store can
key by this identity" as license to retain nothing at all. That conflicts with the owning contract
this section itself already cites for the identity: GAME-INTERACTION-01 §7 (evidence below) —
"Duplicate delivery of the same sibling MUST converge to one lifecycle/outcome," `COMMITTED`/
`REJECTED` are terminal and never reapplied/reevaluated, and "loss of a retained result payload MUST
NOT re-enable execution." That last clause presupposes a retained result payload normally exists.
`CommandIngress`'s unsuitability (round 12's evidence, still correct) only rules out *reusing that
one mechanism* — it says nothing about whether GAME-INTERACTION-01 requires retention at all. It
does. The owning contract wins; round 12's "not retained... out of scope" text is corrected below,
not carried forward.

Round 13 correction (owner-authorized): read GAME-INTERACTION-01 in full for its terminal-lifecycle
retention rule (§7, quoted below) and its explicit position on numeric retention bounds (§5.9, §25):
the contract requires retaining terminal lifecycle evidence and converging duplicate delivery onto it,
but explicitly leaves "retention window/count" unfrozen and rules out inventing "numeric
cascade/resource/retry limits" here — the same decided-semantics/undecided-number split this section
already uses for FND-03 §15.4 timer capacity. Fixed: the revert's terminal outcome
(`InteractionChildOccurrenceRef` → `TerminalSemanticOutcome`) is now scope-owned state, retained from
firing until scope restart, with its retention capacity reserved in the *same* staged commit as the
timer capacity (FND-03 §15.4) so exhaustion fails the whole original operation before anything
commits — never silently evicted mid-generation, so "loss" only happens together with the rest of the
scope-ephemeral overlay on restart, which is exactly when GAME-INTERACTION-01's re-enable-execution
concern stops applying (the object's own overlay state resets with it). A duplicate presentation now
finds and returns that retained outcome instead of finding nothing.

**Round 13's two-structure design is corrected by round 14 below: it had gaps at the seams.** Keeping
a separate pending set and a separate retained-outcome store, joined only by "the driver checks one,
then the other," left two windows unrepresented: between removing the pending entry and writing the
terminal record (a lost race or an interruption there erased the identity from both structures at
once), and a duplicate could hit round 9's scope/content/incarnation fences *before* the
retained-outcome lookup, discarding an already-terminal identity instead of returning its outcome.
Both are seam bugs from having two structures instead of one; round 14 removes the seam.

Round 14 correction (owner-authorized): replace the two structures with **one scope-owned lifecycle
record per `InteractionChildOccurrenceRef`**, matching GAME-INTERACTION-01 §7's own lifecycle
directly instead of approximating it with a pending set plus a side store. The record's state is
`PENDING(entry fields) | IN_FLIGHT | TERMINAL(outcome)` — `PENDING` here is this document's name for
what §7 calls `UNSTARTED` (accepted, not yet executed); `IN_FLIGHT` is what §7 calls `PENDING`
(outcome unresolved, execution underway); `TERMINAL(outcome)` collapses §7's `COMMITTED`/`REJECTED`
into one state carrying whichever it is. It is created `PENDING` in the original operation's staged
commit, with capacity reserved *once* for the whole lifecycle (FND-03 §15.4) — not twice, since one
record now serves what round 13 split across a timer-capacity reservation and a retention-capacity
reservation. Every presentation of an identity, whatever produced it, follows one fixed order: look
the identity up; `TERMINAL` returns the first outcome verbatim, no fences, no ordinal, no mutation;
`IN_FLIGHT` converges — do not execute, do not mint, the in-flight execution already owns resolving
it; only `PENDING` reaches the scope/content/incarnation fences, and only now, after the lookup has
already found no terminal or in-flight identity to answer from. A fence failure at that point
atomically transitions the record to its own `TERMINAL(REJECTED, reason)`, never a bare discard, so
the *next* duplicate finds a terminal record too, not a fence check repeated on stale data. Passing
the fences atomically transitions `PENDING`→`IN_FLIGHT`, mints the ordinal and runs `prepare`/commit,
then atomically transitions `IN_FLIGHT`→`TERMINAL(outcome)`. There is never a bare removal at any
step; a scope restart drops every record regardless of state, the same as the rest of the overlay. An
interruption between `IN_FLIGHT` and writing `TERMINAL` leaves the record `IN_FLIGHT`; round 15 below
corrects what this document can actually prove about that case while the scope generation stays live.

Round 15 correction (owner-authorized, convergence round — Codex findings 4120028037/4120028027/
4120028033 on frozen head `a2aab063`): three findings, fixed where concrete, handed to the owning
lane where genuinely open, per PLAYABLE_FIRST and this section's own CANDIDATE status.

First, `prepare`'s `TERMINAL` mapping was incomplete: it named `COMMITTED`/`STALE_STATE`/`OCCUPIED`
but never `DISPOSITION_NO_CHANGE` or `DISPOSITION_REVISION_EXHAUSTED`, both real returns of `prepare`
(evidence below). Fixed by re-deriving the mapping from `prepare`'s own structure rather than
enumerating dispositions by hand again: every disposition `prepare` builds via `PreparedTerminal::
unchanged` (`PreparedMutation::None` — no mutation) maps to `TERMINAL(REJECTED, <disposition>)`;
`DISPOSITION_COMMITTED` alone pairs with `PreparedMutation::Publish` and maps to `TERMINAL(COMMITTED)`
— a rule tied to `prepare`'s own code shape, not a list this document must keep re-synchronizing by
hand.

Second, this document previously asserted an interrupted `IN_FLIGHT` record is never "left to be
silently re-executed or silently forgotten while the scope generation is still live" (round 14's
text, now corrected) — but nothing in the read code proves that. Checked directly (evidence below):
`LocalObjectRuntime`'s own methods (`bind`/`prepare`/`apply`/`terminalize_current`) are plain
synchronous `fn`, never `async fn`; the one existing precedent for "one owner work item" —
`ComposedFreshAdmission::step` (`gameplay_transport/mod.rs` ~546-585) — acquires its `tokio::sync::
Mutex` with a single `.await`, then runs the entire read-check-commit sequence synchronously with no
further `.await` inside, per its own doc comment "One Channel-owner work item for one actor... Nothing
moves unless the step commits." That is consistent with — but does not itself prove — the same shape
for a not-yet-built revert-timer driver. Separately, and independently of that: no code anywhere in
`runtime_actor_carrier.rs`, `movement.rs` or elsewhere defines what happens to a scope owner's task on
panic or abort — no `catch_unwind`, no `JoinHandle`/abort handling, no task-supervision policy exists
to check against. Without that, "an interruption cannot leave the scope live with a stranded
`IN_FLIGHT` record" cannot be asserted as a proven fact; it can only be named as the implementation
choice the owning lane should make. Fixed: removed the unproven claim; added it to "Open decisions for
the owning lane" below, with the concrete evidence and the two possible resolutions named, per
coordinator instruction rather than designing phase/commit reconciliation machinery here.

Third, this document previously said retained records are "never evicted mid-generation" and dropped
"only on scope restart" as a hard rule (rounds 13/14) — an over-claim: nothing requires the owning
lane's eventual store to hold every terminal record forever within one generation, and
GAME-INTERACTION-01 itself leaves retention window/count open (§5.9/§25, evidence above). Fixed:
removed "never evicted"/"only on scope restart" as a hard rule everywhere it appeared; added the
eviction/compaction policy to "Open decisions for the owning lane" below, decided together with the
retention window/count, with one fixed requirement carried forward from GAME-INTERACTION-01 §7 itself:
whatever policy the owning lane picks MUST preserve no-reexecution — for example, a compact tombstone
of identity → outcome code survives even where a full `TerminalSemanticOutcome` does not.

Round 16 correction (owner-authorized; Codex finding 4120154570 on frozen head `24141ed7`): a timed
`map_item` transform can change a teleporter's `destination` attribute, restored via an authored
`revert_destination` override (`OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md` line 144; a concrete
authored instance in `the_lord_of_the_lice/encounter.json` lines 70-74, evidence below) — but nothing
this section's `revert_after_ms` design restores can represent that: `LocalObjectStateDefinition` is
state key plus collision only, and `PreparedMutation::Publish` is state/revision/blocking only
(evidence below). An inverse `TransitionKey` cannot revert an attribute it never touches. Scoped, not
designed: `revert_after_ms` under this proposal covers only what those two types already model; an
attribute-changing transition is fail-closed rejected — round 17 below corrects *where*. Retaining
and restoring attributes is a new capability this document does not design; it is recorded as an open
decision for the owning lane, with two candidate directions named and neither chosen.

Round 17 correction (owner-authorized; Codex finding 4120251303 on frozen head `a1cad472`): round 16
said "`bind` rejects it" — wrong boundary. `bind` operates on `TransitionBinding` alone (evidence
below), which carries no `revert_after_ms` field and no attribute payload; `destination`/
`revert_destination` exist only on the authored `map_item` action, a layer `bind` never sees. By the
time a transition reaches `bind`, an attribute-changing teleporter like `the_lord_of_the_lice`'s and
a genuinely state-only transform are already the same shape — `bind` structurally cannot tell
them apart (Round 18 below: `DepthWarzoneBossDeath` itself is not an example of the latter). Fixed:
moved the rejection to the boundary that *does* see the authored action —
authoring/lowering, fail-closed, with a named error, so an attribute-changing `revert_after_ms`
binding never reaches `bind` at all. Verified directly (evidence below): no server-side encounter
lowering step exists in the read code yet; the closest existing precedent is
`tools/content-schema/encounter-authoring/validate_encounter.py`'s `map_item` validation, offline
tooling explicitly marked "the server does not read these files" — so this is stated as an obligation
on whichever lowering/validation step is built, naming that file as the natural host, not a claim that
enforcement already exists anywhere today.

Round 18 correction (owner-authorized; Codex finding 4120357243 on frozen head `95f7db70`): every
prior round's text called `DepthWarzoneBossDeath` "state-only," citing that it authors no
`destination`/`revert_destination` — wrong: the encounter-format line (171-172) is narrative
shorthand, and all three generated Depth encounters it actually produces —
`the_duke_of_the_depths`/`the_baron_from_below`/`the_count_of_the_core` (evidence below) — carry both
`destination` and `revert_destination`, so they are attribute-changing, not state-only, and are
rejected at authoring/lowering under rounds 16/17 the same as `the_lord_of_the_lice`. Checked directly
(evidence below): no authored `transform` anywhere in `encounter-authoring/samples/**` omits
`destination`/`revert_destination`; no genuinely state-only fixture for the "encounter-originated
timed transform" test obligation exists in the corpus today. Fixed: corrected every place this
section called `DepthWarzoneBossDeath` or a Depth encounter state-only; replaced that test
obligation's fixture with a clearly-labeled synthetic transform, and stated plainly that no authored
encounter currently exercises a state-only timed revert.

Round 19 correction (owner-authorized, under the owner's stop rule — fix the one P1, record the P2s
as open decisions, no new design; Codex findings 4120487852/4120487841/4120487862 on frozen head
`4e450a08`): (1) P1 — round 18 left `mazzinor`/`gaz_haragoth`/`cult_soul_remains`'s `interaction`
binding undecided as disqualifying or not. Fixed: `interaction` is classified a non-state attribute,
the same as `destination`/`revert_destination` (evidence below — it is a binding to
interaction-domain content, not something the state-key-plus-collision model can apply or revert).
Consequence stated plainly: with this rule, no currently authored encounter `map_item` action is
admissible for `revert_after_ms` at all — every `transform` carries `destination`/
`revert_destination`, every `create` that lacks those carries `interaction`. Timed revert for
authored content awaits "Open decisions" item 3 (now naming `interaction` too); the synthetic fixture
is the only runnable case until then. Added a test obligation for interaction-bearing timed creates.
(2) P2 (Codex 4120487841): "Exact delta" claimed `revert_after_ms` as a field on `TransitionBinding`;
corrected — different invocations of the same bound transition can carry different or no duration, so
a single shared field cannot be the whole answer. Added as open decision 4, without redesigning where
the duration actually lives. (3) P2 (Codex 4120487862): a compact tombstone still consumes one entry
per occurrence, so compaction alone does not reclaim capacity without a bounded duplicate-delivery
horizon too. Added as open decision 5 and qualified open decision 1's compaction text and the
"no re-execution after compaction" test obligation accordingly.

Round 20 correction (owner-authorized, under the owner's stop rule; Codex findings
4120634397/4120634418/4120634408 on frozen head `3827d885`): (1) P1 — the staging bullet, canonical
field list and Must-decide-now previously created a `PENDING` lifecycle record for *any*
non-timer-origin `revert_after`-carrying operation, without gating on whether that same operation's
own `prepare` result actually committed. Fixed: a record is created only when `prepare` returns
`DISPOSITION_COMMITTED`/`PreparedMutation::Publish`; every `unchanged` disposition (`NO_CHANGE`/
`STALE_STATE`/`OCCUPIED`/`REVISION_EXHAUSTED`/`BINDING_MISMATCH`) registers no record, and the
capacity reserved for one is released or never committed together with the rest of that operation's
staged commit. The stored expected state/revision now names `PreparedMutation::Publish`'s own
`next_state`/`next_revision` as its source, not an independently-computed `transition.target_state`.
Added a matching test obligation. (2) P2 (Codex 4120634418): the encounter-origin test obligation
still said whether `interaction` disqualifies `mazzinor`/`gaz_haragoth`/`cult_soul_remains` was
undecided — a plain contradiction of round 19's own classification. Fixed to match round 19. (3) P2
(Codex 4120634408): `ScopeRuntimeFence::accept_input` can return `GenerationError::Exhausted`, not
just reject a stale generation; the presentation-order text calling it "atomically" alongside the
`PENDING`→`IN_FLIGHT` transition did not account for that failure mode. Qualified that text and added
open decision 6: fold ordinal issuance into the same atomic step, or invoke FND-03's own scope-terminal
exhaustion recovery — this document does not redesign which.

Round 21 correction (owner-authorized, owner acceptance recorded on issue #162, 2026-09-28): the owner
accepted this section's direction, with open decisions 1, 2, 4, 5, 6 and 7 (below) delegated to the
owning lane, and decided open decision 3 (attribute-bearing object state) `YES` — needed for the
playable path (teleporter `destination`/`revert_destination` and `interaction` bindings) — with its
design in progress as a further `CANDIDATE` section of this same document (task
`OTV2-20260928-cw1-attribute-bearing-object-state`, not yet added at this round).
Before this round, one further Codex finding (4120777222) had been raised on PR #1045's own thread
after that PR's round-20 freeze and merge, too late to fold into that PR: the round-20 fix reserved
lifecycle-record capacity for every non-timer-origin `revert_after`-carrying operation *before*
`prepare` ran, then released that reservation on an `unchanged` result — correct in outcome (no record
survives an `unchanged` result) but reserving capacity speculatively before the outcome is known is
unnecessary work. Fixed here as open decision 7: capacity for one new lifecycle record is now reserved
only once `prepare` has already returned `DISPOSITION_COMMITTED`/`PreparedMutation::Publish`, in the
same staged commit as everything else — never before `prepare` runs, and never at all for an
`unchanged` result. Corrected the staging bullet and the capacity-atomicity/no-revert-on-unchanged
test obligations that the old pre-`prepare` reservation wording superseded.

Round 22 correction (owner-authorized; Codex finding 4121718203 on PR #1097's thread, ~line 521 on
`main`): round 21's own text above, and open decision 7's own list entry below, misattributed the
reserve-only-after-`Publish` ordering to an "FND-03 §15.4 discipline." FND-03 §15.4 itself (evidence
above, its exact text) requires only that committing an operation needing a timer fails before commit
when capacity is unavailable, and that an already-accepted timer is not discarded for due-queue
congestion — it says nothing about *when* a reservation attempt happens relative to `prepare`. Fixed:
the reserve-only-after-`Publish` ordering is restated as this section's own accepted requirement
(open decision 7, part of §7's owner-accepted direction, Round 21), never presented as something
FND-03 mandates; FND-03 §15.4's fail-before-commit rule is cited only for what happens once a
reservation is actually attempted and found wanting. Grepped the whole document for every other place
that cites FND-03 §15.4 near this ordering; the staging bullet and the capacity-atomicity test
obligation already correctly separated the two (FND-03 for fail-before-commit only, evidence above),
so open decision 7's own text was the only other misattributed occurrence, fixed the same way.

Round 23 correction (owner-authorized; Codex finding on PR #1099 thread 4122104484, ~line 1793):
§9's Round 2 fix (design point 3) bound a `revert_destination`-bearing occurrence's inverse as
B→C (the forward transition's own target to a fresh post-revert state C), but never reconciled this
against §7's own literal unique-inverse rule below (Option 2 above and "Exact delta" below), which
before this round accepted only a candidate whose `target_state` **exactly equals** the forward
transition's own `source_state` — C ≠ A, so `bind` would find zero qualifying candidates and reject
every `revert_destination`-bearing forward transition, including all four samples §9 covers. Fixed
by widening §7's rule itself (both citations below), not by asserting the old rule was already
satisfied: a new, optional, content-level field on `LocalObjectStateDefinition`,
`attribute_variant_of: Option<ProductionKey>`, names which other declared state (if any) a state is
a pure attribute-variant of — same `collision`, differing only in per-placement
`local_object_state_attributes`. The inverse-uniqueness search now accepts a candidate whose
`target_state` *either* equals the forward transition's `source_state` *or* has its own declared
`attribute_variant_of` equal to the forward transition's `source_state` — i.e.
`states[candidate.target_state].attribute_variant_of == Some(forward.source_state)` (Round 24, Codex
finding 4122246542: an earlier phrasing of this predicate read backwards and would have checked the
*source* state's own `attribute_variant_of` instead) — still requiring *exactly one* matching
candidate — uniqueness is unchanged, only the equality test is widened. §9 (design point 3, corrected
this round) registers each post-revert state with its *own* `attribute_variant_of` set to the natural
source state it stands in for.

Round 24 correction (owner-authorized; PR #1099 round 4 on head `93940915`, 2 P1s + 1 P2, all
accepted, owner stop rule applies): (1) Codex finding 4122246542 — Round 23's own predicate for the
widened inverse-uniqueness check read backwards: it said a candidate qualifies when its `target_state`
"is that source state's declared `attribute_variant_of`," which names the *source* state's own field,
not the *candidate*'s. The correct, and now-corrected, predicate is
`states[candidate.target_state].attribute_variant_of == Some(forward.source_state)` — checking the
*candidate target's own* declared field against the forward transition's `source_state`, never the
reverse. Fixed everywhere the rule is stated: both §7 citations above, §9's design point 3, and the
matching test obligation. (2) Codex finding 4122246550 — the still-binding "revert restores exactly
the pre-operation state" test obligation (below) did not account for the widened rule: a
`revert_destination`-bearing revert now lands on a declared `attribute_variant_of`, not literally the
pre-operation `source_state` key. Added a narrow, explicit exception to that obligation: the revert
lands on the declared variant — same `collision`, differing only in declared attributes — only when
the bound inverse carries one; scanned §7 and §9 for every other "exactly the prior state" claim
this could contradict and found none beyond the one fixed (the encounter-format line-144 quotation
and the C3 "no partial footprint" obligation are unaffected — evidence above). (3) Codex finding
4122246563 (P2, not designed for, owner stop rule) — pre-authored `CREATE` teleporters carrying
`destination` together with `revert_after_ms` at an authored anchor (`death_priest_shargon`,
`the_ravager`, and any other matching samples found this round) are a *distinct* shape from open
decision 8's runtime-resolved-anchor gap: these have a pre-authored `PlacementRef`, so `bind` is not
structurally blocked the way `death_position` creates are, but this section's design (transform-only,
design point 4) does not cover a `CREATE` carrying `destination` either. Recorded as new open
decision 9, not designed for; corrected the corpus-enumeration claim (Problem section below) so it no
longer asserts completeness.

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
  generation-changed cancellation exactly. `accept_input`'s return type is
  `Result<RuntimeExecutionOrdinal, GenerationError>` (~1040-1050); `GenerationError::Exhausted`
  (~858-871) is returned when the raw ordinal counter's own `checked_successor` overflows (~894-898) —
  ordinal issuance can fail, not just reject a stale generation. `FND-03_RUNTIME_EXECUTION_CONTRACT.md`
  line 266: "representational exhaustion is scope-terminal until safe ownership lifecycle recovery
  establishes a new generation" — the contract's own recovery path for this failure (Round 20). It is
  currently instantiated per `GameSession`
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
  session-command lifecycle around it, can be reused. PROVEN, round 12 (`apps/game-server/src/
  foundation/mod.rs` `CommandIngress` ~574-578, `RetainedTerminalRecord`/`CommandId` ~202/~498-537,
  `reserve`/`terminalize`/`classify_duplicate` ~618-742, `MAX_RETAINED_TERMINAL_RECORDS`/
  `MAX_RETAINED_TERMINAL_CHARGED_BYTES` ~53-55): `CommandIngress` is the only terminal-outcome
  retention/replay mechanism in the codebase, and it cannot key by `InteractionChildOccurrenceRef` —
  it is keyed by a bare sequential `CommandId(u64)` requiring strict in-order presentation
  (`next_command_id`/`SequenceGap`), it retains exactly one record at a time
  (`MAX_RETAINED_TERMINAL_RECORDS = 1`, evicted on every new `terminalize`), and it is reachable only
  through the session-gated `apply`/`resume_pending` this timer already does not use. `grep -rl
  CommandIngress apps/game-server/src crates` — PROVEN: only `world_runtime.rs` and this
  `foundation/mod.rs` reference it; no other retention path exists to check instead.
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
  of those three do not match `self`. The lifecycle record's stored "overlay revision" alone cannot
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
- PROVEN, round 13 (`GAME-INTERACTION-01_..._CANDIDATE.md` §7, lines 265-280, read in full): child
  lifecycle is `UNSTARTED | PENDING | COMMITTED | REJECTED`; "`COMMITTED`: semantic commit proven
  exactly once; replay/retry never reapplies it"; "`REJECTED`: logical child terminal; replay does
  not reevaluate it as fresh work"; "Sibling refs MUST be distinct. Duplicate delivery of the same
  sibling MUST converge to one lifecycle/outcome."; "Loss of a retained result payload MUST NOT
  re-enable execution." This is the owning contract for every `InteractionChildOccurrenceRef`,
  timer-origin children included (§4.1 explicitly lists "timer -> stable timer occurrence identity"
  as a `RootSourceOccurrenceRef` kind) — it requires retention, not a no-op. PROVEN (§5.9, lines
  214-225, and §25 "Explicit non-decisions", lines 780-798): the same contract explicitly leaves
  "retention window/count" unfrozen and lists "no numeric cascade/resource/retry limits" among its
  `DECISIONS_NOT_TAKEN` — it mandates the retention *semantics* above without naming a numeric bound,
  the identical decided-semantics/undecided-number split this section already uses for FND-03 §15.4's
  timer-capacity bound (deferred to `RESOURCE_LIMITS_REGISTRY.json`, "not decided here").
- PROVEN, round 15 (`apps/game-server/src/world_runtime.rs` `prepare` ~973-1050, `PreparedTerminal::
  unchanged` ~1136-1147, `PreparedMutation` ~1149-1159): every `unchanged(...)` call site —
  `DISPOSITION_BINDING_MISMATCH` (~983), `DISPOSITION_STALE_STATE` (~990, expected-revision mismatch;
  ~1008, source-state mismatch — same disposition string, two trigger sites), `DISPOSITION_NO_CHANGE`
  (~998), `DISPOSITION_OCCUPIED` (~1026) and `DISPOSITION_REVISION_EXHAUSTED` (~1031) — constructs
  `PreparedMutation::None`; only the `DISPOSITION_COMMITTED` path (~1040-1053) constructs
  `PreparedMutation::Publish`. This is the complete, exhaustive set of strings `prepare` can return —
  a bounded grep of `apps/game-server/src/world_runtime.rs` for `DISPOSITION_` finds no others declared
  (~21-26) or used outside this function and its own tests.
- PROVEN, round 15 (`apps/game-server/src/gameplay_transport/mod.rs` `ComposedFreshAdmission::step`
  ~546-585): the one existing "one owner work item" precedent acquires its `tokio::sync::Mutex` with a
  single `.await` (~552), then runs its entire read/check/commit sequence with no further `.await`
  inside, per its own doc comment (~542-545) "One Channel-owner work item for one actor... Nothing
  moves unless the step commits." UNKNOWN/NOT PROVEN beyond this: a bounded grep of
  `apps/game-server/src/foundation/runtime_actor_carrier.rs` and `movement.rs` for `async fn`,
  `.await`, `tokio::spawn`, `catch_unwind`, `JoinHandle` and `panic` finds none — no owner-turn task
  supervision, panic or abort handling exists anywhere in the read code for scope-owner work, movement
  included. The revert-timer driver itself is proposed, not built (evidence above: no scope-wide
  cadence/scheduler exists yet).
- PROVEN, round 16 (`apps/game-server/src/content/reference_playable.rs`
  `LocalObjectStateDefinition` ~810-813): exactly two fields, `key: ProductionKey` and
  `collision: LocalObjectCollisionPresence` — no attribute/payload field of any kind. PROVEN
  (`apps/game-server/src/world_runtime.rs` `PreparedMutation::Publish` ~1152-1158, evidence above):
  exactly `expected_state`/`expected_revision`/`next_state`/`next_revision`/`next_blocking` — the same
  state-key-plus-collision-footprint shape, nothing else a revert could restore. PROVEN
  (`docs/architecture/OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md` line 144): `map_item`'s own field
  description — "a teleporter carries `destination` and optionally `revert_destination` anchors; a
  revert restores the original item with its original attributes unless `revert_destination`
  overrides the destination" — names an attribute-level revert (destination, not state) that nothing
  in `LocalObjectStateDefinition`/`PreparedMutation` can represent today. PROVEN
  (`tools/content-schema/encounter-authoring/samples/the_lord_of_the_lice/encounter.json` lines
  70/71/73/74): a concrete authored instance — `anchor: exit_teleporter`, `destination: godbreaker`,
  `revert_after_ms: 60000`, `revert_destination: ascendant_exit` — of exactly this case. NOT a
  contrast, corrected Round 18 (evidence below): the `DepthWarzoneBossDeath` example this section
  cites as a test obligation (`OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md` lines 171-172) reads as
  authoring no `destination`/`revert_destination`, but that line is narrative shorthand only — every
  generated Depth encounter it actually produces carries both (evidence below); it is not a
  state-only example either.
- PROVEN, round 17 (`apps/game-server/src/content/reference_playable.rs` `TransitionBinding`
  ~1334-1342): exactly `key`/`definition`/`source_state`/`normalized_intent_family`/`target_state`/
  `owner_capability`/`policy_guard_refs` — no `revert_after_ms` field and no attribute payload;
  `bind` operates on this type alone and cannot see the authored `map_item` action `destination`/
  `revert_destination` came from, so it cannot tell an attribute-changing transition like
  `the_lord_of_the_lice`'s apart from a genuinely state-only one — round 16's "`bind` rejects it"
  named a boundary that structurally cannot do the check. PROVEN
  (`tools/content-schema/encounter-authoring/validate_encounter.py` ~188-201):
  the existing `map_item` validation block already reads `destination`/`revert_destination`/
  `revert_after_ms` together on one authored action and already enforces one related cross-field rule
  ("`revert_destination` needs `revert_after_ms`", ~200-201) — it does not yet reject
  `revert_after_ms` co-occurring with `destination`/`revert_destination`. PROVEN
  (`tools/content-schema/encounter-authoring/README.md`): "Evidence only: the server does not read
  these files" — this validator is offline tooling for a `CANDIDATE` format, not a wired production
  boundary. No server-side encounter-lowering step exists in the read code (evidence above:
  `TransitionBinding` has no `revert_after_ms` field; CW4 shipped without `revert_after`).
- PROVEN, round 18 (`tools/content-schema/encounter-authoring/samples/*/encounter.json`, exhaustive
  check across every sample): all three generated Depth encounters carry both `destination` and
  `revert_destination` alongside `revert_after_ms` — `the_duke_of_the_depths/encounter.json` lines
  65-68 (`anchor: exit_teleporter`, `destination: reward_destination`, `revert_after_ms: 1200000`,
  `revert_destination: warzone_exit`), `the_baron_from_below/encounter.json` lines 75-78 and
  `the_count_of_the_core/encounter.json` lines 75-78 (identical shape) — `DepthWarzoneBossDeath` is
  attribute-changing, not state-only. PROVEN: every authored `map_item` action combining `operation:
  transform` with `revert_after_ms` across the whole sample corpus also carries `destination`/
  `revert_destination` — no exception found. The only `revert_after_ms`-carrying actions that omit
  both are `create` actions — `mazzinor/encounter.json` lines 47-56, `gaz_haragoth/encounter.json`
  lines 123-132, `cult_soul_remains/encounter.json` lines 53-62 and 70-79 — each of which carries an
  `interaction` binding instead (Round 19: also disqualifying, evidence and rule below). No authored
  sample under `encounter-authoring/samples/**` is a proven state-only `revert_after_ms` usage.
- PROVEN, round 19 (`OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md` line 144, evidence above): `map_item`'s
  own field list names `interaction` as "the key of interaction-domain content that defines what the
  item does when used or stepped on" — a binding to interaction-domain content, not a value
  `LocalObjectStateDefinition`'s `key`/`collision` pair or `PreparedMutation::Publish`'s
  `next_state`/`next_revision`/`next_blocking` (evidence above) can apply or revert; nothing in the
  runtime model represents "which interaction this state is bound to" as part of the state itself.
  `interaction` is therefore classified a non-state attribute, exactly like `destination`/
  `revert_destination`.

### Options (minimum real set)

1. **Reuse an existing Foundation/global simulation tick.** Rejected as not currently available:
   no such tick exists (evidence above), and SIM-DETERMINISM-01 explicitly defers ever requiring
   one. Adopting or creating one now would be a new Foundation-owned decision, out of this task's
   `excluded_scope` (Foundation/runtime/protocol/registry) and disproportionate to one `revert_after`
   field.
2. **An FND-03 §10 authoritative timer bound to a monotonic `Deadline` computed from the authored
   duration (RECOMMENDED).** `revert_after_ms` is admissible only on a bound transition with exactly
   one *inverse* — a transition bound on the same runtime instance, for the same placement
   definition, whose `source_state` equals this transition's `target_state` and whose `target_state`
   either equals this transition's `source_state` or has its *own* declared `attribute_variant_of`
   equal to this transition's `source_state` (Round 24, correcting Round 23's reversed phrasing, Codex
   finding 4122246542: `states[candidate.target_state].attribute_variant_of ==
   Some(this_transition.source_state)`, never the other direction; §9 below:
   `LocalObjectStateDefinition.attribute_variant_of`, same `collision`, differing only in
   per-placement attributes) *and* whose `normalized_intent_family`
   is this transition's matching inverse family (TRANSFORM↔TRANSFORM, CREATE↔REMOVE, RETAG↔RETAG,
   OPEN↔CLOSE); `bind` rejects (`InvalidBinding`) a `revert_after_ms`-carrying transition with zero
   such matches or more than one — an ambiguous inverse is exactly as invalid as a missing one — so
   firing never has to guess which delta restores the object (evidence above). At commit time, for a non-timer-origin operation — every authoritative input except the
   firing of a pending revert timer itself, which never re-arms itself even when its own transition
   also carries `revert_after_ms` (Round 7/8) — whose own `prepare` result is `DISPOSITION_COMMITTED`/
   `PreparedMutation::Publish` (Round 20, Codex finding 4120634397: every `unchanged` disposition —
   `NO_CHANGE`/`STALE_STATE`/`OCCUPIED`/`REVISION_EXHAUSTED`/`BINDING_MISMATCH` — registers no record;
   there is nothing for a revert to undo when nothing changed), as part of the *same* staged commit as
   that `TRANSFORM`/`CREATE`/`REMOVE`/`RETAG` (FND-03 §15.4, below), create one `PENDING` lifecycle
   record under an FND-03 §10.1 scheduling key. The record's exact contents — every field it needs and why —
   are the single canonical **"Lifecycle record: complete field list"** at the start of Exact delta
   below (Round 14, replacing round 11's pending-entry-only table); this option does not repeat them.
   Among them is the revert's own `InteractionChildOccurrenceRef` (Round 11, evidence above), computed
   once now as a nested child of the original operation's own GAME-INTERACTION identity (§3/§4) and
   never re-derived from the scheduling `RuntimeExecutionOrdinal`, which stays a pure
   ordering/authority value (evidence above), never logical identity — it is the record's own lookup
   key, present in every state. One scope-owned driver — one per scope owner, never per object, never
   per pending revert — wakes at the earliest pending deadline and admits at most a registered bounded
   batch of due entries per cycle (FND-03 §7/§14, evidence above), yielding the remainder to a later
   cycle through the same owner arbitration any other input source uses, so a same-deadline burst
   cannot starve unrelated scope work. Every presentation of an admitted entry's identity — a normal
   due-wake, a second driver pass, a crash-recovery rescan, any of them — follows the one fixed order
   Round 14 fixes at the record level (Exact delta below, evidence above): look the identity up;
   `TERMINAL` answers with its first outcome, no fences, no ordinal, no mutation; `IN_FLIGHT`
   converges (no execution, no ordinal — the in-flight attempt already owns resolving it); only
   `PENDING` reaches the scope/content/incarnation fences, and only then does a fence pass atomically
   transition `PENDING`→`IN_FLIGHT` and call the scope's ordinal issuer's `accept_input` (§10.2,
   reusing `RuntimeExecutionOrdinal`/`ScopeRuntimeFence` — evidence above — as one scope-wide instance
   rather than per-`GameSession`). This single order is what round 13's two separate structures (a
   pending set plus a side retained-outcome store) could not guarantee on their own — a duplicate
   could reach the fences before the terminal-outcome check, or a lost race between "remove from
   pending" and "write the terminal record" could leave the identity briefly unrepresented; one record
   with one presentation order closes both gaps (evidence above). The revert's state/footprint delta
   reuses `prepare`'s pure transition logic; committing it does **not** go through
   `apply`/`resume_pending`/`CommandIngress`, because there is no live client command or session to
   replay (P1, evidence above) — the lifecycle record is a separate, minimal mechanism, not a reuse of
   `CommandIngress` (round 12's evidence for why `CommandIngress` cannot serve still holds; it only
   ruled out that one type, not retention itself). A fence failure at the `PENDING` step atomically
   transitions the record to its own `TERMINAL(REJECTED, reason)` — never a bare discard — so a later
   duplicate finds that terminal record instead of re-running a fence check against data that may have
   moved on again; `prepare`'s own stale-state check is still what accepts or rejects a same-incarnation
   changed anchor once the record is `IN_FLIGHT` (below). The driver and the ordinal-issuer promotion
   are still one real new mechanism each, but the unit it stores, the tie-break it uses and the
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
   - *Determinism/exactly-once.* Both give the same exactly-once execution guarantee in this
     proposal's sense (the lifecycle record's atomic `PENDING`→`IN_FLIGHT`→`TERMINAL` transitions,
     Round 14, no re-fire after commit — see test obligations below); `ManualClock` (evidence above)
     already gives deterministic,
     test-controlled time for the `Deadline` option, so option 2 does not trade away deterministic
     testing to gain authored-duration fidelity.
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
   not the revert's derived child identity), its single-record de-duplication (Round 14: one
   scope-owned lifecycle record per identity, `PENDING`→`IN_FLIGHT`→`TERMINAL`, matching
   GAME-INTERACTION-01 §7's own lifecycle directly — every presentation looks the identity up first,
   `TERMINAL` returns the first outcome, `IN_FLIGHT` converges, and only `PENDING` reaches the fences),
   bounded per-cycle due-work admission (§7/§14), the fail-closed unique-bound-inverse precondition on
   `revert_after_ms` itself (zero or ambiguous inverse ⇒ `InvalidBinding`, never a guessed delta), the
   single firing path (a changed object is rejected only inside `prepare`, never by a separate
   pre-`prepare` cancellation), and the one-shot origin test (suppress registration only when this
   execution *is* the firing of a pending revert timer; every other authoritative origin —
   player/command, encounter/server-event, or otherwise — registers its own one-shot revert only when
   that same operation's own `prepare` result is `DISPOSITION_COMMITTED`/`Publish` (Round 20 — an
   `unchanged` outcome registers nothing, evidence above), so a mutually timed pair cannot ping-pong
   and an encounter-originated timed transform is not
   silently starved of its revert; no periodic/repeating semantics), the complete lifecycle-record
   field list (Round 14, "Exact delta" below) — target identity (`PlacementKey`/`incarnation`/
   `content_generation`, addressed exactly and never re-derived from the overlay revision alone, with
   a changed `incarnation` like `scope_generation`/`content_generation` transitioning the record to
   `TERMINAL(REJECTED, reason)` at the `PENDING` fence step) *and* the revert's own derived
   `InteractionChildOccurrenceRef` (GAME-INTERACTION-01 §5.1's nested-cascade rule, computed once at
   scheduling time, never re-derived from the scheduling ordinal per §4.4/§5.8) as the record's own
   lookup key, present in every state — and that the record's terminal outcome is retained as
   scope-owned state, with creation capacity reserved once for the whole lifecycle in the same staged
   commit as everything else (Round 14, folding round 13's separate timer/retention reservations into
   one), per GAME-INTERACTION-01 §7 (the retention/convergence *semantic* is decided now; how long a
   `TERMINAL` record is kept within one live generation, like the timer-capacity bound, is not —
   §5.9/§25, "Open decisions for the owning lane" below) — all nine are bound to existing FND-03
   sections, GAME-INTERACTION-01's already-implemented identity model, or the merged CW4 bind-time
   model, not open design questions. `NO` for the driver's exact wake
   mechanism, whether `ScopeRuntimeFence` is
   promoted to a scope-wide instance or a new scope-owned ordinal issuer is introduced, the exact
   lifecycle-record storage representation, where `revert_after_ms` and inverse-selection metadata
   are actually carried (Round 19, "Open decisions" below — not necessarily `TransitionBinding`),
   and the concrete timer-capacity/due-batch numeric
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
   representation of the lifecycle-record store, the concrete timer-capacity and per-cycle due-batch
   numeric bounds in `RESOURCE_LIMITS_REGISTRY.json`, and — see "Open decisions for the owning lane"
   (Round 15/19, "Exact delta" below) — the `TERMINAL`-record retention window/count and
   eviction/compaction policy, where `revert_after_ms`/inverse-selection metadata are actually
   carried (Round 19 — not `TransitionBinding` alone), and whether the `PENDING`→`IN_FLIGHT`→
   `TERMINAL` transition is one
   atomic owner-turn step or needs explicit reconciliation for an interrupted `IN_FLIGHT` record. Those
   belong to the owning lane's implementation, not this architecture delta.

### Exact delta the owning lane must provide

Owner: the scope-runtime/Foundation carrier lane behind `ChannelRuntimeV1`/`InstanceRuntime`
(`apps/game-server/src/foundation/runtime_actor_carrier.rs`), coordinated with the CW4 world-runtime
lane (`apps/game-server/src/world_runtime.rs`) that owns `LocalObjectRuntime`. This document does
not implement it; it is CANDIDATE and not owner-accepted.

**Lifecycle record: complete field list (Round 14, replaces round 11's pending-entry-only table).**
One scope-owned record per `InteractionChildOccurrenceRef`, state ∈
`PENDING(entry fields) | IN_FLIGHT | TERMINAL(outcome)` — see the round 14 narrative above for the
mapping onto GAME-INTERACTION-01 §7's own `UNSTARTED | PENDING | COMMITTED | REJECTED`. Every field
below is captured exactly once, at scheduling time, from the original operation's own
already-validated commit; firing never looks up, re-derives or guesses any of them. Every other place
in this section (Option 2, and each bullet below) refers back to this one list instead of
re-enumerating fields.

| Field | Present in | Consumed for | Source at scheduling time |
|---|---|---|---|
| `InteractionChildOccurrenceRef` | every state (the key) | looking the record up before anything else (Round 14 presentation order) | a nested child of the original operation's own GAME-INTERACTION identity (§3/§4), never the scheduling ordinal (evidence above) |
| World/Channel/InstanceId | `PENDING` | scope identity fence | the scope's own binding |
| `scope_generation` | `PENDING` | fence — mismatch transitions the record to `TERMINAL(REJECTED, reason=SCOPE_GENERATION_CHANGED)` | the scope's own binding |
| `PlacementKey` | `PENDING`→`IN_FLIGHT` | which anchor to address | the original operation's own placement |
| `incarnation` | `PENDING` | fence — mismatch transitions to `TERMINAL(REJECTED, reason=INCARNATION_CHANGED)` (Round 9) | the original operation's own target |
| `content_generation` | `PENDING` | fence — mismatch transitions to `TERMINAL(REJECTED, reason=CONTENT_GENERATION_CHANGED)` | the original operation's own content pin |
| `Deadline` | `PENDING` | when the timer becomes due (ordering) | `Deadline::after(clock, revert_after_ms)` |
| scheduling `RuntimeExecutionOrdinal` + within-resolution sequence | `PENDING` | equal-deadline tie-break, FND-03 §10.1 (ordering) | the scheduling resolution's own ordinal |
| inverse `TransitionKey` | `PENDING`→`IN_FLIGHT` | which delta `prepare`'s `operation` applies | the bind-time unique-inverse check (rounds 5/6) |
| expected post-operation state + overlay revision | `PENDING`→`IN_FLIGHT` | `prepare`'s `expected_revision`/stale-state check | `PreparedMutation::Publish`'s `next_state`/`next_revision` (Round 20 — the record is created only when the original operation's own `prepare` result is `Publish`, never `unchanged`; evidence above) |
| outcome (`TerminalSemanticOutcome`, or the fence-discard reason above) | `TERMINAL` only | the record's one answer to every later presentation of this identity | `prepare`'s outcome, or the `PENDING`-step fence that discarded it |

Together: `PlacementKey`/`incarnation`/`content_generation`/inverse `TransitionKey`/expected
revision are exactly what `LocalObjectCommand` needs to call `prepare` (evidence above);
World/Channel/InstanceId/`scope_generation` are the same scope-identity fences every other overlay
operation already carries; `Deadline`/ordinal/sequence are FND-03 §10.1's own scheduling key;
`InteractionChildOccurrenceRef` is the GAME-INTERACTION-01 identity every prior round assumed but
never listed, now the record's own key; and `outcome` is what `TERMINAL` returns to every later
presentation, whichever of `prepare`'s dispositions or the `PENDING`-step fences produced it. Capacity
is reserved *once*, for the whole record's lifecycle, in the same staged commit that creates it
`PENDING` (below) — not once for a "timer entry" and again for a "retained outcome" (round 13's split,
now removed): the scope's own capacity counter is live scope state, not part of the record. Lifecycle
(Round 14, replaces round 13's two-structure split; retention bound corrected by round 15): the
record is never bare-removed at any transition — a scope restart drops every record regardless of
state, together with the rest of the scope-ephemeral overlay. Whether and how a `TERMINAL` record may
also be evicted or compacted *within* one live scope generation is explicitly not decided by this
document (Round 15 — GAME-INTERACTION-01 itself leaves retention window/count open, §5.9/§25,
evidence above); see "Open decisions for the owning lane" below for the one fixed requirement any such
policy must satisfy. Nothing the firing path consumes is missing from this list or derivable only from
something outside it plus the scope's own live state (its clock, its ordinal issuer, and — as one
thing, not two — its lifecycle-record store).

- Provide `revert_after_ms` as authored metadata reaching `bind` for the transitions that use it —
  its exact home (a `TransitionBinding` field, or metadata carried by the lowered operation
  occurrence that invokes the transition) is an open decision, not this one (Round 19, "Open
  decisions" below: different invocations of the same bound transition can carry different or no
  `revert_after_ms`, so a single shared field on `TransitionBinding` cannot be the whole answer).
  Whichever home is chosen, validate it fail-closed inside `bind`
  (`apps/game-server/src/world_runtime.rs` ~590-750): for every transition invoked with
  `revert_after_ms`, find every OTHER bound transition for the same `definition` whose
  `source_state` equals this one's `target_state`, whose `target_state` either equals this one's
  `source_state` or has its *own* declared `attribute_variant_of` equal to this one's `source_state`
  — `states[candidate.target_state].attribute_variant_of == Some(this.source_state)`, never the
  reverse (Round 24, correcting Round 23's reversed phrasing, Codex finding 4122246542; §9 below —
  `LocalObjectStateDefinition.attribute_variant_of: Option<ProductionKey>`, same `collision`,
  differing only in per-placement `local_object_state_attributes`; `None` for every state outside
  §9's covered shape, so this widening is inert for ordinary content), AND whose
  `normalized_intent_family` is this one's matching inverse family — TRANSFORM↔TRANSFORM,
  CREATE↔REMOVE, RETAG↔RETAG, OPEN↔CLOSE (this covers TRANSFORM a→b needing bound b→a, CREATE
  needing the bound REMOVE of the same anchor/def, REMOVE needing the bound CREATE, and RETAG
  needing the reverse RETAG; RETAG's own same-collision-class constraint is already enforced
  elsewhere in CW3's linker, so this check adds only the family-pairing rule, not a second
  collision-class check). Reject the whole binding with `WorldRuntimeError::InvalidBinding` unless
  *exactly one* transition matches — zero matches is a missing inverse, more than one is an ambiguous
  inverse, and both are equally invalid — producing the one unique inverse `TransitionKey` the
  lifecycle record stores (complete field list above). CW3 link time validates `attribute_variant_of`
  fail-closed, mirroring `validate_local_object_placement_state`'s existing pattern (evidence above,
  §9): a named variant base must exist in the same declared vocabulary and share the same `collision`
  presence as the state naming it.
- Scope of what `revert_after_ms` covers, rejected at the boundary that can actually see it (Round
  17, corrects round 16's "`bind` rejects it"): `revert_after_ms` is admissible only on a transition
  whose full effect is modeled today by `LocalObjectStateDefinition` — the state `key` plus
  `LocalObjectCollisionPresence` (evidence above) — because that is exactly what
  `PreparedMutation::Publish` restores: `next_state`/`next_revision`/`next_blocking`, nothing else
  (evidence above). `bind` **cannot** enforce this itself: `TransitionBinding` (evidence above) carries
  only `key`/`definition`/`source_state`/`normalized_intent_family`/`target_state`/`owner_capability`/
  `policy_guard_refs` — no `revert_after_ms` field and no attribute payload of any kind — while a
  `map_item` teleporter's `destination`/`revert_destination` exist only on the authored encounter
  action, a layer `bind` never sees (evidence above: `TransitionBinding` cannot even tell a genuinely
  state-only transform apart from `the_lord_of_the_lice`'s attribute-changing one — both would
  already have been reduced to the same shape by the time either reaches `bind`; `DepthWarzoneBossDeath`
  itself is not a state-only example, Round 18 below). The rejection therefore belongs to the
  boundary that *does* see the authored
  action: an authored `map_item` action carrying `revert_after_ms` together with `destination`,
  `revert_destination`, `interaction` (Round 19 — classified a non-state attribute, evidence above:
  a binding to interaction-domain content, not something `LocalObjectStateDefinition`/
  `PreparedMutation` can apply or revert), or any other non-state attribute is rejected fail-closed,
  with a named error, at authoring/lowering — it must never reach `bind` at all as a revert-bearing
  operation. Consequence, stated plainly (Round 19): under this rule, no currently authored encounter
  `map_item` action is admissible for `revert_after_ms` — every `transform` carries `destination`/
  `revert_destination` (the Depth trio and `the_lord_of_the_lice`, evidence above) and every `create`
  that lacks those carries `interaction` (`mazzinor`/`gaz_haragoth`/`cult_soul_remains`, evidence
  above). Timed revert for authored content becomes available only once "Open decisions" item 3
  (attribute-bearing object state, now explicitly including interaction bindings) is resolved; the
  synthetic fixture in the test obligations below is the only runnable case until then. Concretely
  (evidence above): `tools/content-schema/encounter-authoring/validate_encounter.py`'s existing
  `map_item` validation (~188-201) already reads every authored field on the action, including
  `destination`/`revert_destination`/`revert_after_ms` together, and already enforces one related
  cross-field rule ("`revert_destination` needs `revert_after_ms`", ~200-201) — this new rule is the
  same shape, added to the same block, and is the natural host for it. That script is explicitly
  offline tooling for a `CANDIDATE` format ("the server does not read these files" —
  `encounter-authoring/README.md`), so it is evidence of where the check belongs, not proof a
  production boundary already enforces it: no server-side encounter lowering step exists yet either
  (evidence above — `TransitionBinding` has no `revert_after_ms` field at all, and CW4 shipped without
  `revert_after`). This rejection is therefore an explicit obligation on whichever lowering/validation
  step is built, offline or server-side, before `revert_after_ms` reaches production content — not
  something this proposal can claim is already enforced. This is a new-capability gap
  (attribute-bearing object state has no bound-inverse mechanism today), not a defect in the revert
  design above; see "Open decisions for the owning lane" below.
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
  responsibility, not the issuer's, and (Round 14) it is one fixed presentation order over the single
  lifecycle record, applied to *every* presentation of an identity before anything else runs:
  1. Look the identity up in the lifecycle-record store.
  2. `TERMINAL` — return the first outcome exactly as recorded; no fences, no ordinal minted, no
     mutation. GAME-INTERACTION-01 §7's "duplicate delivery of the same sibling MUST converge to one
     lifecycle/outcome" is satisfied unconditionally here, before any fence could re-discard it
     (evidence above; this ordering is what round 13 got wrong).
  3. `IN_FLIGHT` — converge: mint no ordinal, run no mutation. The presentation already under way owns
     resolving this identity to `TERMINAL`; a second presentation observes only that the identity is
     still unresolved and takes no further action.
  4. `PENDING` — only now, having found no terminal or in-flight identity to answer from, apply the
     `scope_generation`/`content_generation`/`incarnation` fences (Round 9, field list above). A fence
     failure atomically transitions the record straight to `TERMINAL(REJECTED, reason)` (next bullet
     names the reason) — never a bare discard, so the very next duplicate finds step 2, not step 4
     again. Passing all three fences atomically transitions `PENDING`→`IN_FLIGHT` and calls
     `accept_input`; only a presentation that made it this far mints an ordinal. `accept_input` itself
     returns `Result<RuntimeExecutionOrdinal, GenerationError>` and can fail
     (`GenerationError::Exhausted`, evidence below) — whether ordinal issuance is inside the same
     atomic step as the `PENDING`→`IN_FLIGHT` transition, or can independently fail afterward leaving
     the record `IN_FLIGHT` with no ordinal minted, is not decided here (Round 20, "Open decisions"
     below, item 6); this document does not claim that ordering is safe.
  This single order is what prevents both P1 gaps round 13's two separate structures left open: a
  fence check landing before the terminal-outcome lookup (Codex finding, evidence above), and a lost
  race between "remove the pending entry" and "write the terminal record" leaving the identity
  briefly unrepresented in either structure.
- Provide one shared `MonotonicClock` instance per scope (constructed once, never `SystemClock::new()`
  called again per call site) and use it for every `Deadline::after` at commit time and every
  `has_elapsed`/`remaining` at wake time for that scope — never mix two clock instances (evidence
  above: `SystemClock::new()` starts a fresh, incomparable origin each time).
- On a `revert_after`-carrying overlay operation whose origin is *not* the firing of a pending
  revert timer (evidence above: player/command via `apply`/`resume_pending`, a state-only
  encounter/server-event-originated overlay operation that has already passed the authoring/lowering
  rejection above (Round 18: `DepthWarzoneBossDeath` itself is not such an example — it is
  attribute-changing and rejected before it gets here), or any other non-timer authoritative input —
  the test is "is this execution the firing of a pending revert timer," not which entry point
  produced it): run that operation's own `prepare` first, exactly as for any operation without
  `revert_after_ms` (evidence above). Reserve safe bounded capacity for one new lifecycle record only
  when, and as part of the *same* staged commit as, that `prepare` call returns
  `DISPOSITION_COMMITTED`/`PreparedMutation::Publish` (Round 21, Codex finding 4120777222 — corrects
  Round 20's "reserve capacity before `prepare` runs, release it on an `unchanged` result"; evidence
  above). For every `unchanged` disposition (`NO_CHANGE`/`STALE_STATE`/`OCCUPIED`/
  `REVISION_EXHAUSTED`/`BINDING_MISMATCH`, the exhaustive set, evidence above), no capacity is
  reserved at all and no lifecycle record is created — `prepare`'s own result already determined
  nothing will change, so there is nothing for a revert to undo. When `prepare` returns `Publish`:
  reserve capacity for one new lifecycle record — Round 14 folds round 13's separate "timer capacity"
  and "retained-outcome capacity" reservations into this one reservation, since one record now serves
  the identity's whole life; if capacity is unavailable, fail the *entire* original operation before
  anything commits — no object mutation and no lifecycle record survives (FND-03 §15.4). If capacity
  is available, create the record `PENDING` — with the complete field
  list above (World/Channel/InstanceId, `scope_generation`, `PlacementKey`, `incarnation`,
  `content_generation`, `Deadline`, scheduling `RuntimeExecutionOrdinal`/sequence, inverse
  `TransitionKey`, expected state/revision sourced from that same `Publish`'s `next_state`/
  `next_revision`) keyed by the revert's own `InteractionChildOccurrenceRef`
  computed now as a nested child of the original operation's own identity, under the FND-03 §10.1
  scheduling key, in the same staged commit as the object mutation `Publish` itself applies — so
  nothing about the revert is derived later, only applied from what was stored.
  This record is `scope_generation`-scoped, owned by the same `ChannelRuntimeV1`/`InstanceRuntime`
  instance as the rest of the overlay; a scope restart is a new instance (corrected evidence above),
  so it is dropped whole, in whichever state it was in, with no separate cleanup path — the same
  restart that would make "loss of a retained result payload" (GAME-INTERACTION-01 §7) possible is
  exactly the restart that also resets the object's own overlay state, so nothing is left that could
  be wrongly re-enabled. Map the capacity failure to `CAPACITY_EXCEEDED` (FND-03 §28) and register
  the concrete numeric bound in `RESOURCE_LIMITS_REGISTRY.json` per FND-03 §14.1 — that number is not
  decided here.
- On each driver wake, present admitted due entries (bounded above) as normalized FND-03 §10.2
  authoritative inputs, in their stored (deadline, scheduling ordinal, within-resolution sequence)
  tie-break order (field list above), and run the fixed presentation order above for each one: a
  `TERMINAL` or `IN_FLIGHT` identity is answered or deferred without ever reaching `prepare`; only a
  `PENDING` identity that passes its fences (below) is transitioned to `IN_FLIGHT`, mints its
  `RuntimeExecutionOrdinal` via the scope's ordinal issuer, and reaches `prepare`, addressed exactly
  by the record's stored `PlacementKey`/`incarnation`/`content_generation` (never looked up, never
  guessed, never re-derived from the overlay revision alone) with its stored inverse `TransitionKey`
  and its stored expected revision as `expected_revision` — reusing `prepare`'s existing
  stale-precondition checks (`DISPOSITION_STALE_STATE`, `apps/game-server/src/world_runtime.rs`
  ~988-1013) exactly as they already work for any command, so an object the record's own fences did
  not already catch (changed again after this revert was scheduled, but still the same incarnation)
  is rejected there, never guessed at. Every disposition `prepare` can return atomically transitions
  the record `IN_FLIGHT`→`TERMINAL(outcome)`, and the mapping is exhaustive by construction, not by
  enumeration (Round 15, evidence above): `prepare` builds its outcome via exactly two constructors,
  `PreparedTerminal::unchanged` (`PreparedMutation::None`) or the inline `PreparedMutation::Publish`
  arm, and nothing else — so `TERMINAL(COMMITTED)` is the `DISPOSITION_COMMITTED`/`Publish` path alone,
  and `TERMINAL(REJECTED, <disposition>)` is every `unchanged` disposition: `BINDING_MISMATCH`,
  `STALE_STATE` (either trigger), `NO_CHANGE`, `OCCUPIED` and `REVISION_EXHAUSTED` (Round 15 —
  `self.revision.checked_add(1)` overflow, `apps/game-server/src/world_runtime.rs` ~1029-1034). Per
  GAME-INTERACTION-01 §7: "`COMMITTED`: semantic commit proven exactly once; replay/retry never
  reapplies it"; "`REJECTED`: logical child terminal; replay does not reevaluate it as fresh work." A
  distinct occurrence (a new `revert_after` registration, even at the same anchor) carries its own
  distinct nested-child identity and its own distinct record, so one occurrence's outcome never stands
  for another's. Commit through the
  same scope-authority path (`PreparedMutation::Publish`/`TerminalSemanticOutcome`, ~1040-1054) —
  never `apply`/`resume_pending`/`CommandIngress`, which require a live `GameSessionAuthoritySnapshot`
  this timer does not have (P1, evidence above). This commit is a timer-origin execution: it reuses
  only the state/footprint commit mechanics above, never the capacity-reservation-and-create step of
  the previous bullet, even when the inverse `TransitionKey` it just executed itself carries
  `revert_after_ms` (Round 7/8) — a mutually timed pair fires one direction and stops, it does not
  re-arm itself.
- Fences (§10.3's "scope ownership generation changed"/"invalidated" triggers) apply only to a
  `PENDING` identity (Round 14, step 4 of the presentation order above), never to `TERMINAL` or
  `IN_FLIGHT` — the lookup in the previous bullet already routed those away before fences could ever
  run against them (this is the second P1 gap round 13 left open: fences must never see an
  already-terminal identity). For a `PENDING` identity: `scope_generation` or `content_generation`
  changing invalidates the timer itself, the same reset as the rest of the scope-ephemeral overlay
  (§4's "Lifetime"); the record's stored `incarnation` no longer matching the current object at that
  `PlacementKey` (Round 9 — a replacement incarnation at the same anchor) invalidates it the same
  way — all three are checked before the record is ever presented to `prepare`, and any one failing
  atomically transitions the record to `TERMINAL(REJECTED, reason)`, naming which fence failed
  (`SCOPE_GENERATION_CHANGED` / `CONTENT_GENERATION_CHANGED` / `INCARNATION_CHANGED`, field list
  above) — never a bare discard, so a later duplicate finds step 2 of the presentation order (`return
  the first outcome`), not this fence check re-run against data that may have moved on again. A
  mismatched overlay revision on the anchor, or any other same-incarnation state/revision change, is
  deliberately *not* one of these `PENDING`-step fences — that case is the previous bullet's job: the
  record still reaches `prepare` with its stored inverse key and expected revision, and `prepare`'s
  own stale-precondition check rejects it there, itself atomically transitioning the record to
  `TERMINAL(REJECTED)` as any other disposition does. One path, not two, for "the object changed
  within the same incarnation."
- On an occupancy conflict, `prepare` returns `DISPOSITION_OCCUPIED`, which (previous bullets)
  atomically transitions the record to `TERMINAL(REJECTED)` under its own `InteractionChildOccurrenceRef`
  and stops; do not retry it on a later wake (decided below, not left open) — a later duplicate
  presentation is answered by step 2 of the presentation order above, never a fresh occupancy check.

### Open decisions for the owning lane (Rounds 15/16/19/20/21; item 8 added from §9 Round 2; item 9
added from §9 Round 4)

These are genuinely open — this document deliberately does not resolve them, per PLAYABLE_FIRST; the
owner's Round 21 acceptance covers this section's *direction* only (`DecisionStatus` above) and
explicitly delegates every item below except open decision 3 (owner-decided `YES`, design in
progress) to the owning lane. The owning lane resolves them alongside the exact delta above, not this
architecture decision.

1. **`TERMINAL`-record retention window/count and its eviction/compaction policy.**
   GAME-INTERACTION-01 §5.9/§25 leaves this unfrozen for every `InteractionChildOccurrenceRef`, not
   just this one (evidence above); this document does not invent a number or a policy shape for the
   revert case either. Whatever the owning lane picks — an unbounded per-generation store, an LRU, a
   time-window, a compact tombstone keyed by identity → outcome code once a record ages out of full
   detail, or something else — it MUST preserve GAME-INTERACTION-01 §7's "loss of a retained result
   payload MUST NOT re-enable execution": no policy may let a duplicate presentation fall through to
   the `PENDING` fences or `prepare` once its identity has ever reached `TERMINAL`. Compaction to a
   tombstone by itself does not reclaim capacity (Round 19, item 5 below): a tombstone still consumes
   one entry per occurrence, so a bounded duplicate-delivery horizon or another finite dedup
   representation is also needed before space is actually freed. Decide this together with the
   matching `RESOURCE_LIMITS_REGISTRY.json` bound (FND-03 §14.1) — the same registry entry as the
   lifecycle-record creation capacity above, or a related one, is the owning lane's call.
   **Owning-lane resolution (task `OTV2-20260928-cw1-timed-revert-runtime`):** no eviction or
   compaction. `TERMINAL` records stay for the whole scope generation, under one per-scope cap on
   records in any state (`WOBJ-RL-04`, the creation-capacity entry). A full store fails the next
   forward operation `CAPACITY_EXCEEDED` before commit, so no identity that reached `TERMINAL` can
   fall through to the fences or `prepare` again.
2. **`IN_FLIGHT` reconciliation for an owner-turn interruption while the scope stays live.** Checked
   directly (evidence above): the one existing "one owner work item" precedent
   (`ComposedFreshAdmission::step`) runs synchronously once its lock is acquired, with no internal
   `.await` — consistent with a synchronous, unyielding `PENDING`→`IN_FLIGHT`→`TERMINAL` transition —
   but no code anywhere defines what happens to a scope owner's task on panic or abort, so this cannot
   be asserted as already proven. The owning lane resolves this one of two ways: (a) implement the
   transition as one atomic, unyielding owner-turn step, following the existing precedent, and treat
   any panic/abort during that step as fatal to the whole scope generation (forcing a restart, which
   already clears every record regardless of state) — if chosen, "interrupted with the scope still
   live" becomes impossible by construction, and only the "transition happens within one owner turn"
   test obligation below is needed; or (b) allow an owner-turn interruption to leave the scope
   generation live with a record stranded `IN_FLIGHT` — if chosen, the owning lane must design explicit
   `IN_FLIGHT` reconciliation (how a stranded record eventually reaches `TERMINAL`, or is recognized as
   needing owner intervention), which this document does not design and does not choose between.
   **Owning-lane resolution (task `OTV2-20260928-cw1-timed-revert-runtime`):** option (a).
   `PENDING`→`IN_FLIGHT`→`TERMINAL` is one synchronous owner-turn step with no await. An error inside
   it leaves the record `IN_FLIGHT` and makes the driver scope-terminal until a restart drops it.
3. **Attribute-bearing object state and its timed revert (e.g. teleporter `destination`/
   `revert_destination`, and — Round 19 — `interaction` bindings). Owner-decided `YES` (Round 21,
   issue #162, 2026-09-28) — needed for the playable path; design in progress as a further section of
   this document (task `OTV2-20260928-cw1-attribute-bearing-object-state`). The rest of this item is
   kept as the evidence and framing that design builds on; unlike items 1, 2 and 4-7, whether to
   support this case is no longer open, only how.** `revert_after_ms` under this
   proposal covers only the state-key-plus-collision footprint `LocalObjectStateDefinition`/
   `PreparedMutation::Publish` already model (Round 16, evidence above; "Exact delta" above rejects
   anything wider at authoring/lowering, Round 17 — `bind` cannot do this check itself, evidence
   above) — it does not cover a transition that also changes non-state object attributes, such as
   `map_item`'s authored `destination`/`revert_destination` (`OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md`
   line 144; sample `the_lord_of_the_lice/encounter.json` lines 70-74, evidence above) or an
   `interaction` binding (Round 19, evidence above: the same field-144 vocabulary; concrete instances
   in `mazzinor`/`gaz_haragoth`/`cult_soul_remains`, evidence above) — with `interaction` now included,
   **no currently authored encounter `map_item` action is admissible for `revert_after_ms`** (Round
   19, "Exact delta" above); the synthetic fixture in the test obligations is the only runnable case
   until this decision resolves. This is a new capability (attribute-bearing object state), not a gap
   in the revert design above, and this document does not design it. Two candidate directions, named
   without choosing between them: retain the resolved inverse *attribute payload* (e.g. the
   pre-transform `destination`, or the bound `interaction` key) alongside the lifecycle record's
   existing fields, applied outside the state-key/collision model; or fold attributes into
   `LocalObjectStateDefinition`/`PreparedMutation` themselves so a state carries payload and a revert
   is again a pure state transition. Either belongs to the owning lane (and likely CW3/CW4's
   content-model lane) as its own decision, not this one. Whichever direction is chosen, the
   authoring/lowering rejection above (Round 17/19) is what keeps an attribute-changing transition
   from reaching `bind` in the meantime — this open decision is about eventually *supporting* the
   case, not
   about how it is safely refused today.
4. **Where `revert_after_ms` and inverse-selection metadata are actually carried (Round 19, Codex
   finding 4120487841) — resolved by §9 (CANDIDATE).** "Exact delta" above described adding
   `revert_after_ms` to `TransitionBinding` or its content source; Codex's rationale for why that is
   not simply an implementation detail: `TransitionBinding` is shared, content-level state for a
   definition's transition, but different invocations of the same bound transition can legitimately
   carry different `revert_after_ms` durations, or none at all — a single shared field cannot
   represent that. This document did not redesign the home here at the time this item was written; it
   only removed the claim that `TransitionBinding` is settled as that home (evidence above, "Exact
   delta" corrected). §9 below (design point 5, CANDIDATE, coupled to open decision 3 since both are
   per-placement facts) now names the home: `local_object_revert_after_ms`, keyed by
   `(TransitionKey, LoweredActionId)` *per placement* (§9 Round 2, Codex finding 4121918234: a bare
   `TransitionKey` key cannot hold two authored actions at the same placement invoking the same
   transition with different durations) — this item is kept, not deleted, as the record of the
   finding and the reasoning §9's design answers; the owning lane implements §9's shape once it is
   accepted, rather than choosing its own.
5. **Compaction still needs a bounded duplicate-delivery horizon (Round 19, Codex finding
   4120487862).** Open decision 1 above names a compact tombstone (identity → outcome code) as one
   possible compaction shape once a `TERMINAL` record ages out of full detail. Codex's rationale: a
   tombstone still consumes one entry per occurrence — compaction alone does not reclaim capacity
   unless duplicate presentations are also known to be impossible past some point. A bounded
   duplicate-delivery horizon, or another finite dedup representation that lets a sufficiently old
   identity's slot be reclaimed entirely, is required before "compact to a tombstone" can be counted
   as capacity reclamation rather than a fixed-size-per-occurrence cost with a smaller constant. This
   document does not design that horizon; the owning lane decides it together with open decision 1
   and the matching `RESOURCE_LIMITS_REGISTRY.json` bound.
   **Owning-lane resolution (task `OTV2-20260928-cw1-timed-revert-runtime`):** the horizon is the
   scope generation. Capacity is reclaimed only when a restart drops the scope-ephemeral store.
6. **Ordinal issuance inside the `PENDING`→`IN_FLIGHT` step (Round 20, Codex finding 4120634408).**
   `ScopeRuntimeFence::accept_input` returns `Result<RuntimeExecutionOrdinal, GenerationError>` and can
   fail with `GenerationError::Exhausted` (evidence above), not only reject a stale generation. This
   document's presentation order (Exact delta above) calls `accept_input` immediately after the
   `PENDING`→`IN_FLIGHT` transition but does not decide whether ordinal issuance is inside that same
   atomic step (so an `Exhausted` failure prevents the transition from happening at all) or can
   independently fail afterward, leaving a record `IN_FLIGHT` with no ordinal minted and no path back
   to `TERMINAL`. This document does not redesign that step; the owning lane's implementation must
   either fold ordinal issuance into the same atomic `PENDING`→`IN_FLIGHT` step, or invoke FND-03's own
   scope-terminal exhaustion recovery (`FND-03_RUNTIME_EXECUTION_CONTRACT.md` line 266: exhaustion is
   scope-terminal until safe ownership lifecycle recovery establishes a new generation) for the record
   left `IN_FLIGHT`. Nothing in this document should be read as claiming the current ordering already
   handles this safely.
   **Owning-lane resolution (task `OTV2-20260928-cw1-timed-revert-runtime`):** issuance is folded
   into the step. After the fences pass, `accept_input` runs first and the record moves to
   `IN_FLIGHT` only on `Ok`. On `Exhausted` the record stays `PENDING` and the driver stops as
   scope-terminal (FND-03 line 266).
7. **Reserve lifecycle-record capacity only after `prepare` returns `Publish`, not speculatively
   before `prepare` runs — this section's own accepted requirement, not an FND-03 invariant (Round
   21, Codex finding 4120777222 on PR #1045's thread; corrected Round 22, Codex finding 4121718203).**
   Round 20 reserved capacity for every non-timer-origin `revert_after`-carrying operation *before*
   calling `prepare`, then released that reservation again on an `unchanged` result — correct in
   outcome, but reserving speculatively before the outcome is known is unnecessary work. This ordering
   choice is *this document's own* accepted requirement (§7's `DecisionStatus`, Round 21), not
   something FND-03 §15.4 itself mandates: FND-03 §15.4's own text (evidence above) requires only that
   committing an operation needing a timer fails before commit when capacity is unavailable, and that
   an already-accepted timer is not discarded for due-queue congestion — it says nothing about when a
   reservation attempt happens relative to `prepare`. This document now states the corrected direction
   (staging bullet and the matching test obligations above, Round 21): reserve capacity only once
   `prepare` has already returned `DISPOSITION_COMMITTED`/`PreparedMutation::Publish`, as part of the
   same staged commit as the object mutation itself; FND-03 §15.4's fail-before-commit rule then
   governs what happens once that reservation is actually attempted and found wanting. What remains
   open for the owning lane is the exact implementation-level mechanics of that atomicity — `prepare`
   is a pure query today (evidence above); making "check capacity" and "commit the object mutation and
   create the `PENDING` record" a single atomic step around an already-computed `prepare` result is an
   implementation detail this document does not design.
8. **Runtime-created local objects at a runtime-resolved anchor (`death_position`) — added from §9,
   Codex findings 4121918211/4121918220 on PR #1099's Round 2 review.** §9's attribute-bearing-state
   design (`revert_after_ms` combined with `destination`/`interaction`) was originally scoped to cover
   `mazzinor`/`gaz_haragoth`/`cult_soul_remains` as well as the teleporter-transform samples, but
   those three all author `map_item create` at `at: death_position`
   (`tools/content-schema/encounter-authoring/samples/mazzinor/encounter.json` lines 47-57;
   `.../gaz_haragoth/encounter.json` lines 123-132; `.../cult_soul_remains/encounter.json` lines
   53-62 and 70-79), a runtime-resolved position with no pre-authored `PlacementRef`. `azerus`
   (`.../azerus/encounter.json` lines 52-63, added Round 4, Codex finding 4122246563's corpus
   re-scan) shares the exact same `at: death_position` blocker, carrying `destination`+
   `revert_after_ms` instead of `interaction` — the blocker is the missing `PlacementRef`, not which
   attribute the action carries, so it is the same gap, not a new one.
   `LocalObjectRuntime::bind` (`apps/game-server/src/world_runtime.rs` ~620-630) requires its
   `placement_key: &PlacementKey` argument to already exist in `content.placements` — a fixed,
   pre-authored list, with no dynamic-placement-creation path anywhere in the read code — and C3
   (§4 "Out of scope," lines 154-162) already excludes "a `CREATE` that reserves cells not already
   known at bind time" as a general rule. §9 does **not** design a runtime-created-placement
   mechanism to satisfy these four samples; they stay rejected fail-closed at authoring/lowering.
   Whether and how a runtime-resolved anchor could ever bind a `LocalObjectRuntime` — a
   dynamically-registered `PlacementRef`, a different runtime primitive entirely, or something else —
   is a new decision this document does not make; it would need its own C3-adjacent hardening review
   given C3 explicitly excludes dynamically materialized geometry today.
   **Status (2026-09-28):** the owner decided to design this now (issue #162 comment 5879299188,
   item 1). The design is §10.3, `CANDIDATE`, pending independent review. It covers runtime
   placements bound from a lowered template, collision-`Absent` only, as a narrow C3 lift. Until
   §10 is accepted, these four samples stay rejected at lowering.
9. **Pre-authored `map_item create` teleporters carrying `destination`+`revert_after_ms` at an authored
   anchor — added from §9, Codex finding 4122246563 on PR #1099's Round 4 review.** Distinct from open
   decision 8 above: `death_priest_shargon`
   (`tools/content-schema/encounter-authoring/samples/death_priest_shargon/encounter.json` lines
   47-58, `anchor: exit_teleporter`, `destination: shargon_exit`, `revert_after_ms: 300000`) and
   `the_ravager` (`.../the_ravager/encounter.json` lines 47-58, `destination: ravager_exit`,
   `revert_after_ms: 300000`) both author `operation: create` at a pre-authored `anchor` — a
   `PlacementRef` genuinely exists, so `LocalObjectRuntime::bind`'s `PlacementKey` lookup is not
   structurally blocked the way open decision 8's `death_position` samples are. The gap is different:
   §9's design (Design point 4) admits only `map_item transform` carrying `destination`, never
   `create`, at any anchor. A full corpus re-scan this round (evidence: the two samples above) found
   no other sample combining `operation: create` with a pre-authored `anchor` and `destination`. §9
   does **not** extend its design to admit `create`; these two samples stay rejected fail-closed at
   authoring/lowering. Whether and how a `CREATE` could ever carry `destination`/`revert_after_ms` —
   treating it as equivalent to a `transform` into a synthesized state, or something else — is a new
   decision this document does not make.
   **Status (2026-09-28):** the owner decided to admit these as a §9 transform from a synthesized
   absent state (issue #162 comment 5879299188, item 2). The design is §10.4, `CANDIDATE`, pending
   independent review. Until §10 is accepted, both samples stay rejected at lowering.

### Exact test obligations

- **Fires once, mints exactly one ordinal (P2, de-duplication, Round 14 single record).** A test that
  presents the same due identity twice while it is still `PENDING` (neither presentation has reached
  `TERMINAL` yet — e.g. a second driver pass or a crash-recovery rescan re-observing it before either
  finished) must observe exactly one `RuntimeExecutionOrdinal` minted and exactly one commit; the
  atomic `PENDING`→`IN_FLIGHT` transition (step 4 of the presentation order, Exact delta above) is
  what stops the second presentation from also minting. This is deliberately not "a deterministic
  no-op result from `accept_input`": `accept_input` itself (evidence above) has no way to recognize a
  repeat, so the guarantee lives entirely in the record's atomic state transition, and the test must
  exercise that transition directly.
- **A duplicate during `IN_FLIGHT` converges (P1, Round 14).** Present the same identity a second time
  after the first presentation has minted its ordinal and started running `prepare`/commit, but before
  it has written `TERMINAL`. The second presentation must observe the record as `IN_FLIGHT` (step 3 of
  the presentation order) and take no action of its own: zero new `RuntimeExecutionOrdinal`s, zero
  calls into `prepare`, zero mutation, and it must not re-check the `PENDING`-step fences (those apply
  only to `PENDING`, never to `IN_FLIGHT` — Exact delta above). Whichever outcome the first,
  still-in-flight presentation eventually writes is what any later presentation of this identity —
  including this one, if it is retried after the first completes — receives.
- **Fenced, one path, fences apply only to `PENDING` (P1, FND-03 §10.1/§10.3, Round 14).** The revert
  commits only under the same fences named in the lifecycle record's complete field list ("Exact
  delta" above) as any other §4 operation, but only three of them are ever checked, and only while the
  record is `PENDING`: `scope_generation`, `content_generation` or the stored `incarnation` no longer
  matching (Round 9) each atomically transition the record to `TERMINAL(REJECTED, reason)` without
  ever calling `prepare` (the timer itself is invalid — §10.3's "scope ownership generation
  changed"/target-generation-mismatch triggers). A test asserting these fences are re-checked against
  a `TERMINAL` or `IN_FLIGHT` record must fail — the presentation-order lookup (Exact delta above)
  routes those away before any fence runs, which is exactly the ordering Codex's P1 finding required
  (a duplicate of an already-terminal child must never be discarded by a fence check that runs before
  the terminal-outcome lookup). A mismatched overlay revision (a same-incarnation state/revision
  change) is deliberately *not* one of these `PENDING`-step fences; a test asserting it is (a separate
  pre-`prepare` cancellation path for a changed object) must fail — the only path for a
  same-incarnation changed object is `prepare`'s own `DISPOSITION_STALE_STATE` (below).
- **A duplicate after `TERMINAL` returns the first outcome, even after the target is replaced (P1,
  Round 14, GAME-INTERACTION-01 §7).** Let a revert reach `TERMINAL` (any outcome), then replace the
  target object's incarnation at the same `PlacementKey` (the pre-existing recycling path) or otherwise
  change the object again. Present the same identity once more: it must return the first `TERMINAL`
  outcome exactly as recorded — zero new `RuntimeExecutionOrdinal`s, zero mutations, no re-evaluation
  of `prepare`, occupancy or the `PENDING`-step fences against the object's new state — because step 2
  of the presentation order (Exact delta above) answers from the record alone and never re-derives
  anything from current object state. A test asserting the replaced target changes what this duplicate
  observes must fail; per §7, "`COMMITTED`... replay/retry never reapplies it" and "`REJECTED`...
  replay does not reevaluate it as fresh work" — unconditionally, not just while the target is
  unchanged.
- **Distinct occurrences never collide (P1, Round 14, GAME-INTERACTION-01 §5.1/§5.8).** Schedule two
  distinct `revert_after` registrations that resolve to the same anchor/target/edge (e.g. the same
  transform fired twice at different times, or a mutually timed pair's two directions): each must
  compute its own distinct `InteractionChildOccurrenceRef` (a distinct nested child of its own
  triggering operation's identity, never the scheduling ordinal — §4.4/§5.8) and therefore its own
  distinct lifecycle record, so a test that lets one registration's record answer for the other's must
  fail.
- **Retention bound follows the contract's rule (P1, Round 14/15, GAME-INTERACTION-01 §5.9/§25).**
  GAME-INTERACTION-01 requires the convergence/retention behavior above but explicitly does not name a
  numeric retention window/count (§5.9, §25) — a test MUST NOT assert any specific record count or
  retention duration as architecturally required here, and MUST NOT assert that a record survives for
  the full life of its scope generation unconditionally (round 13/14's over-claim, corrected by round
  15 — see "Open decisions for the owning lane" above); it MAY assert that lifecycle-record capacity is
  reserved once, atomically, at scheduling (previous section) and that exhaustion fails the whole
  original operation before commit.
- **The `PENDING`→`IN_FLIGHT`→`TERMINAL` transition happens within one owner turn — conditional on the
  owning lane's choice (P1, Round 15, GAME-INTERACTION-01 §7).** This document does not assert as
  proven that an owner-turn interruption cannot leave the scope live with a record stranded
  `IN_FLIGHT` (evidence above; see "Open decisions for the owning lane" above). If the owning lane
  resolves that open decision by making the transition one atomic, unyielding owner-turn step (option
  (a) above): a test MUST show the whole `PENDING`→`IN_FLIGHT`→`TERMINAL` sequence completes within a
  single owner-turn invocation with no yield point in between, matching the existing
  `ComposedFreshAdmission::step` precedent (evidence above) — under that choice, a test asserting the
  identity becomes unrepresented, or that a later presentation re-executes an `IN_FLIGHT` record from
  scratch, must fail, because the record stays `IN_FLIGHT` until the scope restarts, at which point it
  is dropped with everything else the scope owns. If the owning lane instead resolves it via option
  (b), the test obligations for that reconciliation belong to that design, not to this document.
- **Cleared on scope restart (Round 14).** The lifecycle-record store (keyed by
  `InteractionChildOccurrenceRef`) is `scope_generation`-scoped state owned by the same
  `ChannelRuntimeV1`/`InstanceRuntime` instance as the rest of the overlay; a scope restart is a new
  instance (corrected evidence above), so every record — in whichever state — is dropped together with
  no separate cleanup path — the same lifetime §4 already states ("Lifetime: Scope-ephemeral") and the
  same trigger FND-03 §10.3 already names ("scope ownership generation changed"). Because it never
  crosses a process lifetime, FND-03 §9's durable-encoding requirement does not apply here (evidence
  above) — this is a positive test, not merely an absence.
- **No re-execution after compaction, whatever the eviction/compaction policy turns out to be (P1,
  Round 15, GAME-INTERACTION-01 §7).** This document does not decide whether or how a `TERMINAL`
  record may be evicted or compacted within one live scope generation (Round 15 — corrects rounds
  13/14's "never evicted" over-claim; see "Open decisions for the owning lane" below). What it does
  require, because GAME-INTERACTION-01 §7 requires it regardless of representation ("loss of a
  retained result payload MUST NOT re-enable execution"): whatever the owning lane picks, a test MUST
  show that once an identity's presence is compacted down to a minimal tombstone (at minimum, identity
  → outcome code — evidence above), a later presentation of that identity still converges to that
  outcome and never falls through to step 4 (`PENDING` fences) or re-executes `prepare`. A test MUST
  NOT assert a specific eviction trigger, window or count as required by this architecture — those are
  the owning lane's decision (below) — but it MUST assert the no-reexecution property survives
  whichever policy is chosen. This obligation is about correctness under compaction, not capacity: a
  test MUST NOT assert that compacting to a tombstone by itself reclaims store capacity (Round 19,
  "Open decisions" item 5 below — a tombstone still consumes one entry per occurrence).
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
- **An attribute-changing `map_item` action outside §9's admitted shape is rejected at
  authoring/lowering, a state-only one passes (P1, Round 17, corrects round 16's "rejected at bind";
  narrowed by §9, owner-accepted 2026-09-28).** An authored `map_item` action carrying
  `revert_after_ms` together with `destination`, `revert_destination`, or any other non-state
  attribute must be rejected fail-closed, with a named error, at the authoring/lowering boundary that
  reads the action (evidence above) — **except** exactly §9's admitted shape: a `map_item transform`
  at a pre-authored `anchor` carrying `destination` and optionally `revert_destination` (plus the
  presentational `effect`), which lowers into §9's per-placement attributes, per-action
  `revert_after_ms` and (for `revert_destination`) dedicated post-revert state and inverse (§9
  design points 3/4). Everything else stays rejected: `interaction` or any other attribute,
  `at: death_position` (open decision 8), and `create` carrying `destination` (open decision 9). A
  rejected action must never reach `bind` as a revert-bearing operation at all, so a test asserting
  `bind` itself is what performs this check must fail (`bind` structurally cannot:
  `TransitionBinding` carries no attribute payload, evidence above). Conversely, an authored
  `map_item` action carrying `revert_after_ms` with *no* `destination`/`revert_destination` and no
  other non-state attribute (a genuinely state-only transform — the synthetic fixture below, since no
  authored sample under `encounter-authoring/samples/**` qualifies today, evidence above;
  `DepthWarzoneBossDeath` itself does not, Round 18) must pass lowering and reach `bind` normally. A
  test asserting a binding for an attribute-changing transition outside §9's admitted shape reaches
  `bind` at all and later fires `DISPOSITION_COMMITTED` with the object's attributes left unrestored
  (stale `destination`) must fail — outside §9, `revert_after_ms` covers only what
  `LocalObjectStateDefinition`/`PreparedMutation::Publish` model (evidence above), never a silent
  partial revert.
- **An interaction-bearing timed create is rejected at lowering (P1, Round 19).** An authored
  `map_item` `create` action carrying `revert_after_ms` together with an `interaction` binding —
  concretely, `mazzinor`/`gaz_haragoth`/`cult_soul_remains`'s shape (evidence above) — must be
  rejected fail-closed at authoring/lowering; a test asserting it reaches `bind` because it lacks
  `destination`/`revert_destination` must fail. Combined with the previous obligation, a test suite
  exercising the authored samples under `encounter-authoring/samples/**` for `revert_after_ms`
  admissibility must observe passes only for §9's admitted `transform` shape (and the synthetic
  state-only fixture); every open decision 8 sample (`mazzinor`/`gaz_haragoth`/`cult_soul_remains`/
  `azerus`) and open decision 9 sample (`death_priest_shargon`/`the_ravager`) must still be rejected
  fail-closed until that decision is resolved.
- **An original operation that does not commit registers no revert, and reserves no capacity for one
  (P1, Round 20/21, Codex findings 4120634397/4120777222).** Bind a `revert_after_ms`-carrying
  transition and drive an operation against it whose
  `prepare` result is `NO_CHANGE`, `STALE_STATE`, `OCCUPIED`, `REVISION_EXHAUSTED` or
  `BINDING_MISMATCH` (any `unchanged` disposition, the exhaustive set, evidence above) — for example,
  replaying it against an already-stale `expected_revision`
  or a target cell that is currently occupied. The test must observe zero lifecycle records created for
  that operation (no `PENDING` record exists afterward) and zero capacity reservation attempts: since
  capacity for one new lifecycle record is reserved only after `prepare` returns `Publish` (Round 21 —
  corrects round 20's "reserve before `prepare`, release on `unchanged`"), an `unchanged` result never
  reaches the reservation step at all. A test asserting a record is created (and a timer later fires),
  or that capacity was reserved and then released, for an operation whose own `prepare` result was
  `unchanged` must fail — a revert has nothing to undo when the original operation changed nothing.
- **Revert restores exactly the pre-operation state, or its declared attribute variant (P1; Round 24
  narrow exception, Codex finding 4122246550).** Firing a scheduled revert whose fences and expected
  revision still hold must land the object in precisely the state the bind-time inverse check
  validated as the unique match — the one unique `source_state`/`target_state` pair, not an
  approximation. For the ordinary case, that `target_state` is literally the pre-operation
  `source_state` — unchanged from before this round. **Narrow exception:** when the bound inverse's
  `target_state` is a declared `attribute_variant_of` of the pre-operation `source_state` (§9
  evidence above — a `revert_destination`-bearing occurrence), the revert lands on that variant
  instead — the *same* `collision` as the pre-operation state, differing *only* in declared
  attributes (§9's own construction, evidence above), never a different footprint or a different
  underlying item. This is the sole exception; a test must not assert the revert lands on a declared
  variant when the bound inverse carries none (still literal equality), nor that it lands on the
  literal pre-operation state key when the bound inverse is a declared variant.
- **Intervening change yields STALE_STATE, no mutation, via the one path (P1).** If the object was
  changed again after the revert was scheduled (a later operation on the same anchor moved it away
  from the state the revert's stored expected revision names), firing must reach `prepare` — never a
  `PENDING`-step fence discard — and hit `prepare`'s existing `DISPOSITION_STALE_STATE` path (evidence
  above), committing nothing and atomically transitioning the record to `TERMINAL(REJECTED)`; the
  revert never overwrites whatever the object has become in the meantime, and there is no second,
  separate check that could instead discard the record some other way.
- **Two anchors with an equal overlay revision fire independently (P1, Round 9).** Bind two
  different `LocalObjectRuntime`s at two different `PlacementKey`s whose overlay revisions happen to
  be numerically equal, each with its own `revert_after_ms`-carrying operation scheduled. When both
  timers become due, each must fire against its own stored `PlacementKey` and mutate only its own
  object; a test that lets either fire against the other's anchor (because it addressed by overlay
  revision alone instead of the stored `PlacementKey`) must fail.
- **A replacement incarnation transitions the old timer to `TERMINAL` (P1, Round 9/14).** Schedule a
  revert, then replace the target object's incarnation at the same `PlacementKey` (the pre-existing
  recycling path, not something this decision adds). The old record must atomically transition
  `PENDING`→`TERMINAL(REJECTED, reason=INCARNATION_CHANGED)` at the fence step when its stored
  `incarnation` no longer matches — it must never reach `prepare`, never mutate the replacement
  incarnation's object, and must not appear as a `DISPOSITION_BINDING_MISMATCH` outcome either, since
  there is no command awaiting a reply for an internally-discarded timer.
- **A mutually timed pair fires once and stops (P2, Round 7).** Bind a `TRANSFORM a→b` and its
  inverse `TRANSFORM b→a` so *both* carry `revert_after_ms`; schedule the forward operation, let its
  timer become due. The test must observe: exactly one `RuntimeExecutionOrdinal` minted, exactly one
  inverse execution (the object lands in `a`), and *zero* new records registered as part of that
  firing — no `PENDING` record exists afterward for either direction. Re-arming only happens if some
  later non-timer-origin execution (a player/command, an encounter/server-event, or any other
  authoritative input) invokes `b→a` (or `a→b`) itself.
- **An encounter-originated timed transform registers and fires (P1, Round 8; corrected Round 18 —
  no authored fixture qualifies, use a synthetic one).** Round 16/17 said `DepthWarzoneBossDeath`'s
  `map_item(transform teleporter at anchor, revert_after_ms 1200000)`
  (`OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md` lines 171-172) authors no `destination`/
  `revert_destination`, so it was cited here as state-only — wrong: that line is narrative shorthand,
  and every one of the three generated Depth encounters this pattern actually produces —
  `the_duke_of_the_depths` (`.../samples/the_duke_of_the_depths/encounter.json` lines 65-68:
  `anchor: exit_teleporter`, `destination: reward_destination`, `revert_after_ms: 1200000`,
  `revert_destination: warzone_exit`), `the_baron_from_below` (lines 75-78, identical shape) and
  `the_count_of_the_core` (lines 75-78, identical shape) — carries both `destination` and
  `revert_destination` (evidence above). All three are attribute-changing and rejected at
  authoring/lowering under the Round 16/17 restriction; they are not this obligation's fixture.
  Checked directly (Round 18, evidence above): no authored `map_item` action anywhere under
  `tools/content-schema/encounter-authoring/samples/**` combines `revert_after_ms` with a `transform`
  and omits `destination`/`revert_destination` — every authored `transform` in the corpus carries
  both. The closest near-misses are `create` actions (`mazzinor`, `gaz_haragoth`,
  `cult_soul_remains` ×2) that omit `destination`/`revert_destination` but carry an `interaction`
  binding; `interaction` is classified a non-state attribute and disqualifies them too (Round 19,
  evidence above), so they are rejected at authoring/lowering exactly like the Depth trio, not claimed
  as state-only fixtures either. **No authored encounter currently exercises a
  state-only timed revert**, and the Depth encounters stay rejected until "Open decisions" item 3
  (attribute-bearing object state) is resolved. Use a **synthetic, clearly-labeled** state-only
  encounter-originated transform instead — for example, a `map_item(transform sealed_wall to
  cracked_wall at anchor, revert_after_ms 300000)` authored with no `destination`/`revert_destination`
  and no other non-state attribute, illustrative only, not drawn from any sample file. Binding it with
  its bound inverse, the `creature_died(boss)`-triggered transform — an encounter/server-event-
  originated operation, never `apply`/`resume_pending` — must register exactly one revert timer at
  commit, and that timer must later fire and restore the wall; a test asserting it stays transformed
  forever (round 7's bug) must fail.
- **Occupied target cells refuse deterministically (decided, not deferred).** `prepare`
  (`apps/game-server/src/world_runtime.rs` ~973-1050) computes and returns its terminal disposition in
  one shot from current state; it never re-evaluates against occupancy that changes later, and (Round
  14) that outcome atomically transitions the record `IN_FLIGHT`→`TERMINAL(REJECTED)` the moment it is
  produced, so a later presentation is answered by step 2 of the presentation order rather than
  re-evaluating `prepare`. A revert whose target state's footprint conflicts with currently occupied
  cells terminalizes as `DISPOSITION_OCCUPIED` (`apps/game-server/src/world_runtime.rs` ~1023-1027)
  for that one execution, under the record's own derived identity (field list above), and is refused
  permanently — the same "no silent search for a free tile unless the definition names one"
  discipline §3 already applies to a blocked relocation. There is no retry: giving a revert a fresh
  identity per wake to work around this would be new per-object retry machinery this decision does
  not introduce, and the `TERMINAL(REJECTED)` record (Round 14) is exactly what makes that
  unnecessary — any later duplicate presentation already converges to the same refusal on its own. If
  a revert must eventually succeed despite occupancy, that is a future definition-level requirement
  (the definition names a fallback, which would be a *distinct* occurrence with its own identity), not
  a hidden retry loop here.
- **Lifecycle-record capacity atomicity (FND-03 §15.4, Round 14, corrected Round 21).** If safe
  bounded capacity for one new lifecycle record is unavailable at the point a non-timer-origin
  `revert_after`-carrying operation's own `prepare` has just returned `DISPOSITION_COMMITTED`/
  `PreparedMutation::Publish` — player/command, encounter/server-event-originated, or any other
  authoritative input — the *entire* original operation fails before anything commits: neither the
  world-object mutation nor a lifecycle record survives (Round 21, Codex finding 4120777222: the
  reservation attempt itself only happens at this point, never before `prepare` runs and never for an
  `unchanged` result — previous obligation). A test asserting a capacity check or reservation attempt
  happens before `prepare` returns, or for an `unchanged` disposition, must fail. Only a timer-origin
  execution (the firing path itself) never attempts this reservation at all (Round
  7/8), so it has nothing to fail on.
  An already-accepted `PENDING`/`IN_FLIGHT` record is never discarded merely because the due queue is
  congested (FND-03 §15.4, second sentence) — that pressure produces `CAPACITY_EXCEEDED` on the *next
  incoming* operation, not eviction of an existing record.
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
   direction recorded in §4 (PR #1055, merged 2026-09-28); `revert_after` awaits the owning lane's
   implementation of §7 (direction ACCEPTED, Round 21, issue #162).
7. **Scope-runtime / Foundation carrier lane (`ChannelRuntimeV1`/`InstanceRuntime`).** Own §7's
   implementation now that its direction is owner-accepted (Round 21): supply the exact delta §7
   names, resolve open decisions 1, 2, 5, 6 and 7 — including, if no existing scope cadence is
   proven, the scope's own step driver itself. CW4 then adds `revert_after` on top of the
   already-shipped operations. Open decision 3 (attribute-bearing object state, owner-decided `YES`)
   and open decision 4 (where `revert_after_ms`/inverse-selection metadata live, coupled to decision
   3) are both designed in §9 below (CANDIDATE, narrowed to the teleporter-transform shapes in §9's
   own Round 2); this lane implements §9's design, once accepted, for every sample matching that
   covered shape (§9 Problem section — at least sixteen, not an exhaustive list; the Depth trio and
   `the_lord_of_the_lice` among them). Open decision 8 (runtime-created local objects at
   `death_position`) is a separate, undesigned gap §9's Round 2 found — `mazzinor`/`gaz_haragoth`/
   `cult_soul_remains`/`azerus` stay rejected until it is resolved, by this lane or another owner C3
   hardening review names. Open decision 9 (pre-authored `CREATE` teleporters carrying `destination`,
   §9 Round 4) is a further separate, undesigned gap — `death_priest_shargon`/`the_ravager` stay
   rejected until it is resolved. Both are now designed in §10 (`CANDIDATE`, pending independent
   review); the samples stay rejected until §10 is accepted.

## 9. Attribute-bearing object state: teleporter destination — ACCEPTED

- DecisionStatus: ACCEPTED by owner 2026-09-28 (issue #162, issuecomment-5873353684). The owner
  decided §7's open decision 3 `YES` (issue #162, 2026-09-28): supporting attribute-bearing object
  state is needed for the playable path. This section is the minimal design that makes it
  admissible; the owning lane (CW3 Content-model linker for the authoring-side shape,
  CW4/scope-runtime for `bind`/`prepare`) implements it and may refine the exact mechanics against
  real code. **Round 2 correction**
  (Codex findings 4121918211/4121918220/4121918234 on head `db49049d`, all accepted, owner stop rule
  applies): (1) `revert_destination` no longer bakes into the placement's natural source state —
  lowering creates a distinct post-revert state instead (design point 3). (2) Narrowed to the
  teleporter-transform shapes only (the Depth trio and `the_lord_of_the_lice`); the `interaction`
  field and the death-position creates (`mazzinor`/`gaz_haragoth`/`cult_soul_remains`) are *not*
  covered — see the Problem section and new §7 open decision 8. (3) `local_object_revert_after_ms`
  is now keyed by `(TransitionKey, LoweredActionId)`, not `TransitionKey` alone (design points 1/5).
  **Round 3 correction** (Codex P1 on head `d417e860`, PR #1099 thread 4122104484, accepted, owner
  stop rule applies): Round 2's post-revert state (design point 3) made every covered forward
  transition unbindable, because its dedicated inverse's `target_state` never equals the forward
  transition's own `source_state` — §7's unique-inverse rule as written before this round required
  exact equality. Fixed by widening that rule (§7 Round 23, both citations) to also accept a
  candidate whose *own* declared `attribute_variant_of` equals the forward transition's
  `source_state` — a new optional field on `LocalObjectStateDefinition` this round adds (design
  point 3), inert for every state that does not declare one. **Round 4 correction** (Codex 2 P1s +
  1 P2 on head `93940915`, accepted, owner stop rule applies): (1) Round 23's own phrasing of the
  widened predicate read backwards — it named "the source state's declared `attribute_variant_of`"
  when the check is the *candidate*'s own declared `attribute_variant_of`; corrected everywhere the
  rule is stated (both §7 citations, this section, the test obligations) to
  `states[candidate.target_state].attribute_variant_of == Some(forward.source_state)`. (2) §7's
  still-binding "revert restores exactly the pre-operation state" obligation now has a narrow,
  explicit exception for the `attribute_variant_of` case — the revert lands on the declared variant,
  same `collision`, differing only in declared attributes — added below and to the matching test
  obligation, with a full scan of §7/§9 for any other now-contradicted "exactly the prior state"
  claim. (3) Pre-authored `CREATE` teleporters carrying `destination`+`revert_after_ms`
  (`death_priest_shargon`, `the_ravager`, and any other matching samples) are recorded as a new open
  decision, not designed for — see §7 open decision 9 and the Problem section below.

### Problem

Under §7 as it stands (Round 16/17/19), `revert_after_ms` is admissible only on a transition whose
full effect is modeled by `LocalObjectStateDefinition`'s `key`+`collision` pair
(`apps/game-server/src/content/reference_playable.rs` ~805-813) — exactly what
`PreparedMutation::Publish` restores (`next_state`/`next_revision`/`next_blocking`,
`apps/game-server/src/world_runtime.rs` ~1150-1159, re-verified this task, unchanged). A `map_item`
action that also carries `destination`, `revert_destination` or `interaction` is rejected fail-closed
at authoring/lowering today, because nothing in the runtime model can apply or revert those values.
This section's design covers one shape — `map_item transform` at a pre-authored anchor, carrying
`destination` (optionally `revert_destination`) — and does not extend to `map_item create` in any
form (**Round 2**: narrowed from the original scope, see DecisionStatus above and Open decision 8).
**Round 4 correction (Codex finding 4122246563):** a full corpus re-scan this round found the covered
shape in considerably more authored samples than the four line-cited below — at least sixteen,
re-verified this round (the four below plus, among others, `deep_terror`, `ferumbras_mortal_shell`,
`glooth_horror`, `mazoran`, `plagirath`, `professor_maxxen`, `ragiaz`, `razzagorn`, `shulgrax`,
`tarbaz`, `the_shatterer`, `zamulosh`). The four cited below remain a representative, line-cited
illustrative subset, not an exhaustive list; no enumeration in this section is claimed to be
exhaustive. Two distinct `map_item create` shapes are **not** covered and stay rejected — see §7 open
decisions 8 and 9:

**Covered by this section — `map_item transform` at a pre-authored anchor, carrying `destination`:**

- `tools/content-schema/encounter-authoring/samples/the_lord_of_the_lice/encounter.json` lines 58-75:
  `map_item transform` (item `canary:item/1949` → `canary:item/22761`), `anchor: exit_teleporter`,
  `destination: godbreaker` (line 71), `revert_after_ms: 60000` (line 73),
  `revert_destination: ascendant_exit` (line 74).
- `.../samples/the_duke_of_the_depths/encounter.json` lines 53-68: same shape, `destination:
  reward_destination` (line 66), `revert_after_ms: 1200000` (line 67), `revert_destination:
  warzone_exit` (line 68).
- `.../samples/the_baron_from_below/encounter.json` lines 63-78 and
  `.../samples/the_count_of_the_core/encounter.json` lines 63-78: identical shape to
  `the_duke_of_the_depths`.

**NOT covered — `map_item create` at a runtime-resolved `death_position`, carrying `interaction`; stays
rejected at lowering; see §7 open decision 8:**

- `.../samples/mazzinor/encounter.json` lines 47-57: `map_item create` (item `canary:item/28673`),
  `at: death_position` (line 54), `revert_after_ms: 60000` (line 55),
  `interaction: canary:interaction/4951` (line 56) — no `destination`, no pre-authored anchor.
- `.../samples/gaz_haragoth/encounter.json` lines 123-132: same shape,
  `interaction: canary:interaction/33542` (line 132).
- `.../samples/cult_soul_remains/encounter.json` lines 53-62 and 70-79 (two occurrences): same shape,
  `interaction: canary:interaction/5580` (lines 62, 79).
- `.../samples/azerus/encounter.json` lines 52-63 (re-verified this round, Round 4 corpus re-scan):
  same `at: death_position` blocker, but carries `destination: azerus_escape` (line 61) and
  `revert_after_ms: 120000` (line 62) instead of `interaction` — no pre-authored anchor either way, so
  it is the same runtime-resolved-anchor gap as the other three, not a new one.

**NOT covered — `map_item create` at a pre-authored `anchor`, carrying `destination`+`revert_after_ms`;
stays rejected at lowering; see §7 open decision 9 (Round 4, Codex finding 4122246563):**

- `.../samples/death_priest_shargon/encounter.json` lines 47-58: `map_item create` (item
  `canary:item/1949`), `anchor: exit_teleporter` (line 55), `destination: shargon_exit` (line 56),
  `revert_after_ms: 300000` (line 57) — a pre-authored anchor exists (unlike the death-position shape
  above), but this section's design (Design point 4 below) admits only `map_item transform`, not
  `create`.
- `.../samples/the_ravager/encounter.json` lines 47-58: same shape, `destination: ravager_exit`
  (line 56), `revert_after_ms: 300000` (line 57).

`OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md` line 144 (re-verified this task): "`map_item` | create/
transform/remove ItemRef at an anchor or `at: death_position` ..., `revert_after_ms`; a teleporter
carries `destination` and optionally `revert_destination` anchors; a revert restores the original
item with its original attributes unless `revert_destination` overrides the destination; optional
`effect`; optional `interaction`: the key of interaction-domain content that defines what the item
does when used or stepped on (D29)."

### Evidence

- PROVEN (`apps/game-server/src/content/reference_playable.rs` `PlacementRef` ~1293-1306): a
  `LocalObject` placement already carries one authored, per-placement, fail-closed-validated field —
  `local_object_initial_state: Option<ProductionKey>` — distinct from the shared, content-level
  `LocalObjectStateDefinition` vocabulary (`ReferenceDefinitionKind::LocalObjectStates`, shared by
  every placement of the same definition). This is the existing precedent for "a fact that varies
  per placement, not per shared content definition."
- PROVEN (`validate_local_object_placement_state` ~2093-2124, called from `validate_placement`
  ~2126-2132): `local_object_initial_state` is validated fail-closed both directions — a
  `LocalObject` placement must have one from the definition's declared vocabulary; a non-`LocalObject`
  placement must not have one. This is the existing precedent for validating a new per-placement
  field's keys against the definition's own state vocabulary.
- PROVEN (`LocalObjectStateDefinition` ~805-813, re-verified this round): exactly `key: ProductionKey`
  plus `collision: LocalObjectCollisionPresence` — no relationship field between two states exists
  today. `attribute_variant_of` (design point 3, Round 23) is a genuinely new, optional field on this
  same struct, not a repurposing of anything that already exists.
- PROVEN (`LocalObjectRuntime::bind` ~590-750, re-verified this task): loads `local_object_initial_state`
  and computes `collision_cells`/`initial_blocking` once, from the placement, before constructing
  `Self` (~716-749); `LocalObjectRuntime` (~574-586) has no attribute field of any kind.
  `transitions: BTreeMap<TransitionKey, TransitionBinding>` (~679-714) is already built per-placement,
  scoped to exactly the `transition_keys` this one placement binds — not a content-level search.
- PROVEN (`prepare` ~973-1050, `PreparedMutation`/`commit` ~1148-1183, re-verified this task):
  `next_blocking` is derived fresh from `target_collision` (the target state's own authored collision
  presence) inside `prepare`, never stored as separate mutable state — collision presence is a pure
  function of `(states, target_state)`. `commit` (~1162-1183) sets `runtime.state`/`runtime.revision`/
  `runtime.blocking_cells` and asserts the pre-commit values matched what `prepare` validated; it
  touches nothing else.
- PROVEN (`TransitionBinding` ~1334-1342): shared, content-level; `source_state`/`target_state`/
  `normalized_intent_family` name the edge, `owner_capability`/`policy_guard_refs` gate it — no
  per-invocation payload of any kind, and (§7 open decision 4, Codex 4120487841) cannot represent a
  per-invocation `revert_after_ms` since different placements bind the same shared transition.
- PROVEN (`ProductionKey::new` `apps/game-server/src/content/production.rs` ~148-166): a bounded,
  namespaced (`namespace:local`) string key. `PlacementKey` (`reference_playable.rs` ~1167-1181) is
  itself a `ProductionKey` newtype — the existing "a place in the world" reference type, matching §8
  item 2 ("Anchors bind to world placements").
- PROVEN (`tools/content-schema/encounter-authoring/validate_encounter.py` ~188-201, evidence
  unchanged from §7): `destination`/`revert_destination` are validated as anchor references
  (`need('anchor', action[field], anchors, at)`), not free-form strings; `effect` is a sibling field
  on the same action with no validation tying it to state or collision — nothing in the encounter
  format or the runtime model ties `effect` to object state.
- PROVEN (`LocalObjectRuntime::bind` ~620-630, re-verified this task): `bind` requires its
  `placement_key: &PlacementKey` argument to already exist in `content.placements` — a fixed,
  pre-authored list (`content.placements.iter().filter(|placement| &placement.key ==
  placement_key)... .ok_or(InvalidBinding("PlacementKey is absent from active Content"))`); no
  dynamic-placement-creation path exists anywhere in the read code. `mazzinor`/`gaz_haragoth`/
  `cult_soul_remains` all author `at: death_position` (evidence above), never `anchor:` — a
  runtime-resolved position, not a `PlacementKey` this or any other bind-time step can look up. This
  is a distinct blocker from the attribute-modeling problem this section solves (§7 open decision 8).
- PROVEN (§4 "Out of scope," C3, lines 154-162, re-verified this task): "the supported collision
  footprint is the anchor's own fixed, bind-time-reserved cells... a `CREATE` that reserves cells not
  already known at bind time... stays out of scope." C3 already excludes exactly this shape (a
  placement materialized at a runtime-computed position) as a general rule; the death-position
  creates are a concrete instance of the already-excluded case, not a new gap this section opens.

### Design

1. **Where the attributes live: per-placement, keyed by state — not shared content, not
   `TransitionBinding`.** A shared `LocalObjectStateDefinition` (one per content definition, reused
   by every placement of that definition) cannot hold a single `destination`: the Depth trio and
   `the_lord_of_the_lice` plausibly share the same content-level teleporter transform, each with its
   *own* `destination`/`revert_destination`. `TransitionBinding` cannot represent a per-invocation
   duration either (open decision 4). The precedent already in this content model —
   `local_object_initial_state`, evidence above — is exactly "a per-placement fact, validated against
   the shared definition's vocabulary." Two new fields on `PlacementRef`
   (`reference_playable.rs` ~1293-1306), mirroring it:
   - `local_object_state_attributes: BTreeMap<ProductionKey, LocalObjectStateAttributes>` — this
     placement's own attribute values, keyed by state; empty for the overwhelming majority of
     `LocalObject` placements (doors, walls, anything state-only), which keep today's behavior
     unchanged.
   - a new struct, `LocalObjectStateAttributes { destination: Option<PlacementKey> }` — one optional
     field (**Round 2**: `interaction` removed — narrowed to the covered teleporter-transform shapes
     only, evidence above; §7 open decision 8 covers the death-position/`interaction` case
     separately, not via this struct), reusing `PlacementKey` (a place in the world, evidence above)
     rather than inventing a dedicated destination-reference type.
   - `local_object_revert_after_ms: BTreeMap<(TransitionKey, LoweredActionId), u64>` — this
     placement's own authored revert duration, keyed by which of *this placement's own bound*
     transitions it applies to *and* which authored action/occurrence invokes it (**Round 2**, Codex
     finding 4121918234: a bare `TransitionKey` key cannot hold two authored actions at the same
     placement invoking the same transition with different — or no — durations). `LoweredActionId` is
     a new, lowering-assigned identifier distinguishing one authored `map_item` action from another
     at the same placement; its exact representation (e.g., an index into the placement's authored
     action list) is the owning lane's implementation choice, the same way this document already
     leaves the exact lifecycle-record storage representation and the exhaustion-recovery mechanics
     open elsewhere. None of the four covered samples actually needs more than one entry per
     transition — each authors exactly one `map_item transform` per placement — so `LoweredActionId`
     is a structural-completeness fix, not something the covered samples exercise; resolves open
     decision 4 directly (below).

   Validated fail-closed at CW3 link time, mirroring `validate_local_object_placement_state` exactly
   (~2093-2124): every key in `local_object_state_attributes` must be a state in the definition's
   declared vocabulary; every `TransitionKey` component of a `local_object_revert_after_ms` key must
   be one of this placement's own bound `TransitionKey`s; a non-`LocalObject` placement must carry
   neither (same "must not carry" half of the existing check). `destination`'s `PlacementKey`, if
   present, must resolve to a real placement in the same content — the same existence check
   `unique_transition` (`world_runtime.rs` ~1058-1075) already performs for a bound `TransitionKey`,
   applied to a placement reference instead.

2. **How `prepare`/`Publish` carry and apply them: they don't need to — attributes are a pure
   read of `(state_attributes, current state)`, exactly like collision presence already is.**
   `LocalObjectRuntime::bind` (~590-750) loads both new maps once from the placement, in the same
   pass as `local_object_initial_state`/`collision_cells` (~716-749), as two new immutable fields on
   `LocalObjectRuntime` (~574-586): `state_attributes: BTreeMap<ProductionKey,
   LocalObjectStateAttributes>`, `transition_revert_after_ms: BTreeMap<(TransitionKey,
   LoweredActionId), u64>`. Neither
   is ever mutated after `bind` — attributes are a pure function of *which state the object is in*,
   never separately tracked mutable state, the same relationship `target_collision` already has to
   `self.states`/`transition.target_state` inside `prepare`. Concretely: **`PreparedMutation::Publish`
   needs no new field, and `commit` (~1162-1183) needs no new line.** `commit` already sets
   `runtime.state = next_state`; a caller reads the object's current attributes through a new
   accessor, `fn attributes(&self) -> Option<&LocalObjectStateAttributes>` (mirroring `state_key()`/
   `collision_cells()`, ~767-785), which does `self.state_attributes.get(&self.state)`. Attributes
   ride the existing state-transition commit path for free; this is the smallest possible extension,
   not a second, parallel payload-application mechanism next to `PreparedMutation`.

3. **How the inverse restores them, including `revert_destination` semantics (encounter-format line
   144) — Round 2 correction (Codex P1 finding 4121918211).** The revert is a bound inverse
   `TransitionKey` firing through the *same* `prepare`/`commit` path as any other operation (§7
   already establishes the revert as "the scope runtime's own later operation," never a distinct
   mechanism). `attributes()` always returns `state_attributes[self.state]` (design point 2) — since
   `attributes()` is keyed purely by *current state*, whichever state the revert lands the object in
   is what the placement exposes from the moment that revert commits onward, with no distinction
   between "before the forward action ever ran" and "after a revert." This is exactly why
   `revert_destination` **must not** be baked into the placement's *original, natural* source state:
   doing so would make `the_lord_of_the_lice`'s sealed teleporter (item `1949`, naturally no
   destination) act as a teleporter to `ascendant_exit` from the moment the placement is bound —
   before the boss-death action ever transforms it — which is wrong (the P1 this round fixes).
   Corrected design:
   - **Without `revert_destination`:** unchanged from before this round. The inverse targets the
     placement's own natural `source_state` (`local_object_initial_state`, evidence above), which
     keeps whatever attributes it naturally has (`None` for a plain state-only object). Lowering does
     not touch `local_object_state_attributes[source_state]` at all.
   - **With `revert_destination`:** lowering creates a **distinct post-revert state** — a new entry
     in the definition's declared `LocalObjectStateDefinition` vocabulary, rendering as the *same
     item* as the natural source state (so it looks identical to a player) but under its *own*,
     distinct state key, since `local_object_state_attributes` is keyed by state and two states
     needing different attribute values need different keys. This new entry's `collision` matches the
     natural source state's exactly (same footprint, the two states differ *only* in declared
     attributes), and it names the natural source state as its `attribute_variant_of` (Round 23,
     evidence above — the new `LocalObjectStateDefinition` field this round adds, required for the
     next bullet). Call it the post-revert state; lowering sets
     `local_object_state_attributes[post_revert_state].destination = Some(revert_destination)`. The
     natural `source_state` itself is left untouched — no entry, no attributes, exactly as if
     `revert_destination` had never been authored (satisfies the test obligation below: before the
     forward action commits, the placement exposes no destination). Lowering binds the inverse
     `TransitionBinding` explicitly as `source_state = target_state` (the forward transition's own
     target), `target_state = post_revert_state` — an *explicitly bound* transition from the forward's
     target to the post-revert state, not a reuse of any existing transition.
   - **Satisfying §7's unique-inverse/intent-family rule (Round 23 correction — Codex P1, PR #1099
     thread 4122104484).** The post-revert state's key is not the forward transition's own
     `source_state`, so §7's rule *as written before this round* — requiring the candidate inverse's
     `target_state` to equal the forward transition's `source_state` exactly — would find zero
     qualifying candidates for every `revert_destination`-bearing forward transition and reject all
     four covered samples; §9's earlier text claiming this was "satisfied by construction" was wrong,
     because it never checked the inverse's `target_state` against the forward transition's *actual*
     declared `source_state`. Fixed by widening §7's rule itself (evidence above), not by asserting
     the old rule already worked: the inverse-uniqueness search now accepts a candidate whose
     `target_state` *either* equals the forward transition's `source_state` *or* has its *own*
     declared `attribute_variant_of` equal to the forward transition's `source_state` —
     `states[candidate.target_state].attribute_variant_of == Some(forward.source_state)`, never the
     reverse (Round 24, correcting an earlier reversed phrasing here, Codex finding 4122246542) —
     which the post-revert state's own declared `attribute_variant_of` always is, by the previous
     bullet's construction. Uniqueness is unchanged (still *exactly one* matching candidate required);
     only the equality test admits one additional, narrowly-scoped case. For a plain
     (no-`revert_destination`) transition, no state anywhere declares an `attribute_variant_of`, so
     the widened rule reduces to exactly the original equality check — this round changes nothing
     about existing, already-covered content.
   - If more than one `revert_destination`-bearing occurrence at the same placement invokes the same
     forward `TransitionKey` (not exercised by any covered sample — each of the four covered samples
     authors exactly one `map_item transform` per placement), each occurrence needs its own dedicated
     post-revert state (each its own `attribute_variant_of: Some(source_state)`) and its own dedicated
     inverse `TransitionBinding`, keyed apart the same way `local_object_revert_after_ms` now is
     (design point 1, `LoweredActionId`); §7's widened rule would then find more than one matching
     candidate unless CW3 lowering also scopes which inverse pairs with which occurrence — this document does not
     design that case further since no covered sample exercises it.

4. **How the Round 17/19 lowering rejection is lifted for exactly these shapes (Round 2: narrowed).**
   The authoring/lowering rejection (§7, evidence above: `validate_encounter.py`'s `map_item` block,
   offline; no production lowering step exists yet either) now admits a `map_item transform` action
   at a pre-authored anchor carrying `revert_after_ms` together *only* with `destination` (optionally
   with `revert_destination`) — the Depth trio and `the_lord_of_the_lice` shape — lowering it into
   exactly the fields above (design point 3's post-revert-state handling applies when
   `revert_destination` is present):
   - `local_object_state_attributes[target_state].destination = Some(destination)`.
   - If `revert_destination` is present: a fresh post-revert state is registered with
     `local_object_state_attributes[post_revert_state].destination = Some(revert_destination)`, and
     the bound inverse targets it (design point 3) — the natural `source_state` is left untouched.
     If absent: the natural `source_state` is the inverse's target, as it already was, with no
     attribute entry.
   - `local_object_revert_after_ms` gets an entry for `(the invoked TransitionKey, this action's
     LoweredActionId)` = `revert_after_ms`.

   A `map_item create` at `at: death_position` carrying `interaction` — `mazzinor`/`gaz_haragoth`/
   `cult_soul_remains`'s shape — is **not** admitted by this section (Round 2, evidence above: no
   pre-authored `PlacementKey` exists for `death_position`; §7 open decision 8). A `map_item create` at
   a pre-authored `anchor` carrying `destination`+`revert_after_ms` — `death_priest_shargon`/
   `the_ravager`'s shape — is also **not** admitted: this design point names only `map_item transform`,
   never `create`, regardless of whether a pre-authored anchor exists (Round 4, Codex finding
   4122246563; §7 open decision 9). `effect` (evidence
   above: a sibling field with no state/collision/destination tie in either the encounter format or
   the runtime model) is presentational and stays entirely outside the world-object overlay model —
   its presence on `the_lord_of_the_lice`'s action does not affect this design and is not lowered
   into `LocalObjectStateAttributes`. Any authored `map_item` action carrying `revert_after_ms`
   together with a non-state attribute *other than* `destination`/`revert_destination` on a
   pre-authored anchor stays rejected fail-closed exactly as §7 already specifies — PLAYABLE_FIRST:
   this section names exactly the one attribute shape the covered samples need (Problem section above
   — at least sixteen, not an exhaustive count), not a generic attribute system.

5. **Open decision 4, resolved: `revert_after_ms` and the inverse-selection metadata live on the
   placement (and, Round 2, the specific authored occurrence), not the shared `TransitionBinding`.**
   `local_object_revert_after_ms` (design point 1) is keyed by `(TransitionKey, LoweredActionId)`
   *per placement*, directly answering Codex's 4120487841 concern that a single shared
   `TransitionBinding` field cannot represent different invocations carrying different or no
   duration — different placements sharing the same content-level transition each author their own
   entry, or none, and (Round 2, Codex 4121918234) two different authored actions at the *same*
   placement invoking the *same* transition each get their own entry too, keyed apart by
   `LoweredActionId`. The inverse-uniqueness check §7 already specifies (exactly one bound inverse
   transition, matching `source_state`/`target_state`/`normalized_intent_family`) now runs scoped to
   *this placement's own* `transitions` map (`bind` ~679-714, already built per-placement) for every
   `TransitionKey` present in `local_object_revert_after_ms`, instead of a content-level search across
   every `TransitionBinding` — a placement only needs an inverse among the specific transitions *it*
   binds, which simplifies, not complicates, the search §7 originally envisioned; design point 3
   covers how this stays satisfied when a dedicated post-revert state/inverse is synthesized per
   occurrence.

6. **The lifecycle-record fields added: none.** The round-14 lifecycle record's existing field list
   (§7 "Exact delta," "Lifecycle record: complete field list") already stores exactly what firing
   needs to reach `prepare` again — `PlacementKey`/`incarnation`/`content_generation`/inverse
   `TransitionKey`/expected state and revision. Attributes are re-derived fresh, at firing time, from
   the *same* bind-time-loaded `state_attributes` table the original operation used, keyed by
   whichever state `prepare` lands on (design point 2); nothing about an attribute value is
   scheduling-time-dynamic, so nothing needs to be captured on the record at scheduling time and
   carried forward — it is placement-static, fixed since `bind`, for both the forward direction and
   (via the lowering-time post-revert-state construction, design point 3) the revert direction. The
   field list's `Deadline` row ("Source at scheduling time") is *clarified*, not extended:
   `Deadline::after(clock, revert_after_ms)` now names its concrete source as
   `self.transition_revert_after_ms.get(&(transition_key, lowered_action_id))` (design point 5) in
   place of the previously-unresolved "`TransitionBinding` or the lowered operation occurrence" —
   requiring the invoking operation to also carry the matching `LoweredActionId`, an addition beyond
   what design point 2 needs for attributes (duration lookup was never claimed to be a pure state
   read; it is tied to the invoking operation, not the resulting state, the same way §7 already scopes
   duration-scheduling separately from the state-commit path).

### PLAYABLE_FIRST scoping

This design adds exactly: two optional fields on `PlacementRef` (`local_object_state_attributes`,
`local_object_revert_after_ms`); one new one-field struct (`LocalObjectStateAttributes`, Round 2:
`interaction` removed); two new immutable fields on `LocalObjectRuntime` loaded once at `bind`; one
new read accessor (`attributes()`); one new optional field on `LocalObjectStateDefinition`
(`attribute_variant_of`, Round 23 — the one change to its existing shared shape, needed to keep §7's
own unique-inverse rule satisfiable, evidence above); and, for `revert_destination`-bearing
occurrences, one fresh content-level state/`TransitionBinding` pair per occurrence, synthesized at
lowering time exactly the way per-encounter content is already synthesized elsewhere in this proposal
(design point 3). It adds no new field to `PreparedMutation::Publish`, no change to `commit`'s
mutation logic, and no new lifecycle-record field. It does not implement teleportation or anything
that *consumes* a `destination` value once an object commits into a state that carries one — reading
`attributes()` and acting on it (moving a player) is the existing, separate interaction/movement
system's job, unaffected and unblocked by this design either way, exactly as it already was before
this section. It does not cover `interaction` bindings, runtime-resolved anchors (Round 2: narrowed
to the teleporter-transform shapes; §7 open decision 8), or `map_item create` at a pre-authored anchor
carrying `destination` (Round 4; §7 open decision 9) — a third authored attribute kind, a
runtime-created-placement mechanism, or admitting `create` alongside `transform`, if any is ever
needed, is a new decision, not something this shape auto-supports.

### Exact test obligations

- **A placement with no `local_object_state_attributes`/`local_object_revert_after_ms` entries
  behaves identically to today (regression).** `attributes()` returns `None` for every state; `bind`,
  `prepare`, `commit` and the existing revert design (§7) are unchanged for every currently-shipped
  door/wall/toggle placement.
- **Before the forward action commits, the source-state placement exposes no destination (P1, Round
  2, Codex finding 4121918211).** For `the_lord_of_the_lice`'s lowered content, before the boss-death
  `map_item transform` action ever commits (the placement freshly `bind`-ed, still in its natural
  `local_object_initial_state`, item `canary:item/1949`), `attributes()` must return `None` — a test
  asserting the natural source state carries `destination: Some(ascendant_exit)` (the old, incorrect
  bake-in this round replaces) must fail. The `revert_destination` value must never be reachable from
  the placement's natural state under any presentation.
- **`the_lord_of_the_lice`'s transform lowers and both directions reach `bind` (corrected, Round 2).**
  The forward transition's target state (`canary:item/22761`) carries `destination:
  Some(godbreaker)`. `revert_destination` (`ascendant_exit`) lowers onto a *distinct* post-revert
  state — same rendered item (`canary:item/1949`) as the natural source, different state key,
  `attribute_variant_of` naming the natural source state — carrying `destination:
  Some(ascendant_exit)`; the bound inverse transition's own `target_state` is that post-revert state,
  not the natural `source_state`. A test asserting the inverse targets the natural
  `local_object_initial_state`, or that the natural source state ever carries a `destination`, must
  fail.
- **`bind` accepts the post-revert-state inverse under the widened rule (P1, Round 23, Codex finding
  on PR #1099 thread 4122104484).** For every covered, `revert_destination`-bearing sample (the four
  line-cited in the Problem section, and — Round 4 corpus re-scan — at least eight more of the same
  shape; not asserted to be an exhaustive count), `bind` must accept the forward
  transition as `revert_after_ms`-carrying: the dedicated inverse's `target_state` does not equal the
  forward transition's own `source_state`, but that `target_state`'s *own* declared
  `attribute_variant_of` does equal the forward transition's `source_state` —
  `states[inverse.target_state].attribute_variant_of == Some(forward.source_state)` (Round 24
  correction, Codex finding 4122246542: checking the reverse direction is the bug, not the fix). A
  test asserting `bind` rejects these samples with `InvalidBinding` ("missing inverse") — the
  exact bug Round 23 fixes — must fail; a test asserting `bind` still accepts them under the
  *unwidened* rule, or under a predicate that checks the *source* state's own `attribute_variant_of`
  instead of the *candidate target* state's, must also fail, since that is the bug, not the fix.
- **The widened rule is inert for ordinary, non-attribute-bearing content (regression, Round 23).** A
  plain `TRANSFORM a→b` / bound inverse `b→a` pair, neither state declaring `attribute_variant_of`,
  must be accepted or rejected by `bind` exactly as it already is today — the widened rule's `OR`
  branch never matches when no state declares a variant relationship, so it must not change the
  outcome for any placement outside this section's covered shape.
- **`attribute_variant_of` is validated fail-closed at CW3 link time.** A state naming an
  `attribute_variant_of` that is absent from the same declared vocabulary, or whose `collision` does
  not match, must be rejected — mirroring `validate_local_object_placement_state`'s existing
  fail-closed pattern (evidence above). A test asserting such a mismatch reaches `bind` must fail.
- **A non-covered attribute stays rejected.** A synthetic `map_item` action carrying `revert_after_ms`
  together with any authored field other than `destination`/`revert_destination` on a pre-authored
  anchor must still be rejected fail-closed at authoring/lowering (§7's existing obligation, unchanged
  by this design).
- **`mazzinor`/`gaz_haragoth`/`cult_soul_remains` stay rejected (Round 2, narrowed scope).** Their
  `map_item create` actions at `at: death_position` carrying `interaction` must still be rejected
  fail-closed at authoring/lowering — this section's design does not admit them (§7 open decision 8).
  A test asserting any of these three samples lowers successfully under this section's design must
  fail.
- **`death_priest_shargon`/`the_ravager` stay rejected (Round 4, Codex finding 4122246563).** Their
  `map_item create` actions at a pre-authored `anchor` carrying `destination`+`revert_after_ms` (and no
  `interaction`) must still be rejected fail-closed at authoring/lowering — this section's design
  admits only `map_item transform`, never `create` (design point 4, §7 open decision 9). A test
  asserting either sample lowers successfully under this section's design must fail.
- **Different placements of the same shared transition carry independent durations.** Bind two
  `LocalObjectRuntime`s at two different `PlacementKey`s that both invoke the same content-level
  `TransitionKey`, one with a `local_object_revert_after_ms` entry set and one without. Firing the
  first must register a revert; the second, invoked identically, must register none — the duration is
  placement-scoped, never read from `TransitionBinding` itself.
- **Two authored actions at the same placement invoking the same transition carry independent
  durations (P2, Round 2, Codex finding 4121918234).** Author two distinct `map_item` actions at the
  same placement that both invoke the same bound `TransitionKey`, one with `revert_after_ms` set and
  one without (or with a different value). Each must lower to its own `local_object_revert_after_ms`
  entry keyed by its own `LoweredActionId`; a test asserting the second action's duration silently
  overwrites, is overwritten by, or is shared with the first's — because the map was keyed by
  `TransitionKey` alone — must fail.
- **Inverse-uniqueness validation is scoped to the placement's own bound transitions.** `bind` must
  reject (`InvalidBinding`) a placement whose `local_object_revert_after_ms` names a `TransitionKey`
  for which *this placement's own* bound `transitions` map contains zero or more than one matching
  inverse (by `source_state`/`target_state`/`normalized_intent_family`), even if a matching inverse
  exists elsewhere in the content's full transition set but is not bound at this placement. For a
  `revert_destination`-bearing occurrence, this is satisfied by construction (design point 3): lowering
  binds exactly one dedicated inverse per occurrence.

### Owner decisions D90 and D91 (2026-09-28)

Raised by the owning lane on #1144 (#162 comment 5875759505) and decided by the owner on #162
comment 5875958040, both "as Global":

- **D90, re-arm.** After a timed revert returns the teleporter to its natural state, the same
  forward transition fires again on the next occurrence of its owning event. When the revert lands
  on a declared post-revert variant C (design point 3, `revert_destination`), the original forward
  edge A→B no longer matches its source state, so lowering also synthesizes a forward edge C→B: the
  same owning event, action, target state, attributes and `revert_after_ms`, bound like A→B, whose
  inverse is the existing B→C revert. A covered teleporter therefore opens on every such event,
  not once per scope generation.
- **D91, event-owned transitions.** A forward transition owned by an encounter or server event is
  not reachable through USE selection or the session `apply` path. Only its owning event commits
  it, and only that commit schedules the revert (§7).
  - **Enforcement.** Each bound transition carries a typed origin, `PLAYER_USE` or
    `EVENT(owner)`, set at lowering from the authored action and never from client input.
    `select_use_transition` considers only `PLAYER_USE` edges, and session `apply` refuses an
    `EVENT` edge fail-closed. Only the owning event's execution path may commit an `EVENT` edge,
    and it names its owner. The origin is part of the binding's identity, so an edge cannot change
    origin without a new binding.
- **Owner decision 3 of 2026-09-28 (issue #162 comment 5879299188): a kill while the teleporter is
  open is a no-op.** The timer is not extended or reset. This conforms to Canary: the Depth script
  (`data-global/scripts/quests/dangerous_depth/creaturescripts_bosses_mission_depths.lua:22-26`)
  transforms and schedules only when item 1949 is present. The behaviour is implemented in #1164:
  `select_timed_forward` returns `None` while the teleporter is open. §10.4 applies the same rule
  to the `CREATE` teleporters, and §10.7 Q2 records how their Canary scripts differ.

### Open items for the owning lane

- The exact CW3 linker/validator code that performs the fail-closed key-subset checks above and the
  `attribute_variant_of` existence/`collision`-match check (Round 23) — this document specifies the
  shape and the existing precedent it mirrors, not the linker's own implementation.
- The exact `bind`-time implementation of the widened inverse-uniqueness search (Round 23, §7 Option
  2/Exact delta above) — this document specifies the accept condition, not the search code.
- The exact production lowering step that turns an authored `map_item transform` action into
  `local_object_state_attributes`/`local_object_revert_after_ms` entries and, when `revert_destination`
  is present, synthesizes the post-revert state/`TransitionBinding` pair (design point 3) — no such
  step exists in production yet (§7 evidence, unchanged); `validate_encounter.py` remains offline
  evidence of the shape, not a wired boundary.
- The exact representation of `LoweredActionId` (design point 1) and how the invoking operation
  carries it through to the commit-time duration lookup (design point 6) — this document names the
  requirement, not the code-level plumbing.
- Whatever downstream system eventually reads `attributes()` to actually move a player to
  `destination` is a separate, already-existing interaction/movement concern (GAME-INTERACTION-01) —
  explicitly out of scope here, exactly as it was before this section existed.
- `interaction` bindings and runtime-created local objects at a runtime-resolved anchor
  (`death_position`) are explicitly **not** designed by this section (Round 2) — see §7 open
  decision 8, now designed in §10.3 (`CANDIDATE`).
- `map_item create` at a pre-authored anchor carrying `destination`+`revert_after_ms`
  (`death_priest_shargon`/`the_ravager`) is explicitly **not** designed by this section (Round 4,
  Codex finding 4122246563) — see §7 open decision 9, now designed in §10.4 (`CANDIDATE`).
- §7's other open decisions (retention/compaction, `IN_FLIGHT` reconciliation, ordinal-issuance
  exhaustion, and any later addition to that list) are unaffected by this section — only open decision
  3 (this section) and open decision 4 (design point 5 above) are resolved here; open decisions 8 and 9
  are this section's own new findings, added to §7's list rather than kept separate, and are now
  designed in §10 (`CANDIDATE`) — and everything §7 lists stays the owning lane's, as written in
  §7 at whatever revision the owning lane implements against.

## 10. Created local objects: runtime placements at `death_position` (OD8) and pre-authored `CREATE` teleporters (OD9)

- DecisionStatus: CANDIDATE, pending independent review. The owner decided on issue #162
  (comment 5879299188, "OWNER DECISIONS: Content/World batch", 2026-09-28) that open decision 8 is
  designed now (item 1), that the open decision 9 teleporters are admitted as a §9 transform from a
  synthesized absent state (item 2), and that a boss kill while a teleporter is open is a no-op
  (item 3). The standing rule (item 6) applies: this section follows the CrystalServer/Canary
  behaviour where it is clear, and lists each deviation as an owner question at the end. Nothing
  here is implemented; `encounter_map_item.rs` keeps rejecting both shapes until this section is
  accepted.
- Scope: exactly the six samples §7 open decisions 8 and 9 name. OD8: `mazzinor`, `gaz_haragoth`,
  `cult_soul_remains` and `azerus`. OD9: `death_priest_shargon` and `the_ravager`.
- The mandatory decision test (`docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md`) and the options
  for the material choices are in 10.9.

### 10.1 Canary behaviour (reference evidence)

All paths are under the CrystalServer tree, `data-global/scripts/`, unless noted otherwise.

| Sample | Canary source | Created object | Lifetime and removal | Use of the object |
|---|---|---|---|---|
| `mazzinor` | `quests/the_secret_library_quest/library_area/creaturescripts_mazzinor.lua:6-19`: on the death of `wild knowledge` | item 28673 at the creature's position, action id 4951 (lines 9-11) | `addEvent` after 60 s removes item 28673 found on that tile, if any (lines 12-17) | Step-in (`movements_mazzinor.lua:7-20`, aid 4951): outfit condition, then `item:remove(1)`. The first player to step in consumes it. |
| `gaz_haragoth` | `creaturescripts/monster/minion_gaz_haragoth_vortex.lua:10-25`: on a minion's death | item 20121 at `deathPosition`, action id 33542 (lines 16-20) | `removeTeleport` after 60 s removes item 20121 found on that tile, if any (lines 1-6, 22) | Step-in (`movements/roshamuul/strange_vortex_tp.lua:5-18`, aid 33542) teleports the player to a fixed position. It is not consumed. |
| `cult_soul_remains` | `quests/cults_of_tibia/creaturescripts_carlin_vortex_spawn.lua:2-12`: on a cultist's death | item 32414 or 32415 (random) at the corpse position, action id 5580 (lines 3-5) | After 60 s, removes that item id found on that tile, if any (lines 6-11) | Step-in (`quests/cults_of_tibia/movements_task_teleport.lua:18-53`, aid 5580): either refuses (already absorbed, or at the maximum), or advances quest storage and `teleport:remove()`. |
| `azerus` | `quests/in_service_of_yalahar/creaturescritps_azerus_kill.lua:10-32`: on Azerus's death | teleporter 1949 at the death position, destination (32780, 31168, 14) (lines 13-17) | `removeTeleport` after 2 min removes item 1949 found on that tile, with a poff effect (lines 1-7, 20) | A plain teleporter (item type `teleport`). |
| `death_priest_shargon` | `quests/dark_trails/creaturescripts_kill_death_priest_shargon.lua:20-38`: on the boss's death | teleporter 1949 at the fixed `teleportPosition` (33487, 32101, 9), destination (33489, 32088, 9) (lines 1-4, 23-26) | `removeTeleport` after 5 min removes item 1949 at the fixed position, if any (lines 10-16, 36) | A plain teleporter. |
| `the_ravager` | `quests/dark_trails/creaturescripts_kill_the_ravager.lua:10-24`: on the boss's death | teleporter 1949 at the fixed (33496, 32070, 8), destination (33459, 32083, 8) (lines 13-16) | `removeTeleport` after 5 min (lines 1-7, 22) | A plain teleporter. |

Properties that follow from the source and bind this design:

1. **Creation ignores tile contents.** `Game.createItem` (`src/lua/functions/core/game/game_functions.cpp:501-590`)
   refuses only when no tile exists at the position (lines 557-564). Otherwise it adds the item
   with `FLAG_NOLIMIT` (line 566), whoever stands there.
2. **Every created object can be walked onto.** Each one is used through `onStepIn`, and its item
   type is a teleport, field or vortex (`data/items/items.xml:3938-3943` for 1949, `:54090-54092`
   for 28672-28673, `:40660-40662` for 20121, `:60706` for 32414-32416). None of them blocks
   movement.
3. **Removal is by lookup and is idempotent.** Each timer removes "the item of that id on that
   tile" if one is there. Once the item has already gone (consumed on step-in), the timer does
   nothing.
4. **Two creations on one tile stack.** Each is a separate item, and each timer removes one.
5. **Removal restores nothing.** The object disappears; nothing reappears in its place.
6. **The death-position objects are one per death.** A later death creates another object; it
   never re-arms or extends an earlier one.
7. **The fixed-position teleporters (OD9) are created without checking whether one is already
   there.** A second kill while one is open would stack a second teleporter with its own 5-minute
   timer (see owner question Q2 in 10.7).

### 10.2 The shared model: a synthesized absent state

OD8 and OD9 use one mechanism. A created object is a `LocalObject` with two states, and "create"
is a transition between them:

- **Absent state** `<action id>/absent`: synthesized at lowering. Its `collision` is `Absent`, it
  carries no attributes, and it is **non-visible**. One new optional marker on
  `LocalObjectStateDefinition`, `absent: bool` (false for every existing state), tells the
  projection to render no object for it. The CW3 linker validates it fail-closed: an absent state
  must declare `collision: Absent`, must declare no `attribute_variant_of`, and must carry no
  per-placement attributes.
- **Present state**: the created item's own state, keyed by its `ItemRef` key (the §9 lowering
  convention). Its attributes are `destination` (the §9 attribute) or `interaction` (new, see
  10.3 D5).
- **Forward** `<action id>/create`: absent → present, `CREATE` intent family,
  `LOCAL_OBJECT_TRANSITION_CAPABILITY`, no policy guards. The owning event commits it through
  `ScopeRevertDriver::apply_forward`, carrying `revert_after_ms` keyed by
  `(<action id>/create, action id)` (§9 design point 5).
- **Inverse** `<action id>/remove`: present → absent, `REMOVE` intent family. Its D91 origin is
  `EVENT(owner)` with the same owner as the create forward, which is the encounter event that
  owns the action. The revert driver commits it as that owner's own later operation (§7 C2). Its
  `target_state` equals the forward's `source_state` exactly, so `bind`'s unique-inverse rule
  (§7, with the existing `CREATE`↔`REMOVE` pairing) accepts it without the §9
  `attribute_variant_of` widening.
- **Consume edge** `<action id>/consume` (Round 5, Codex P1 4127573494): present → absent,
  `REMOVE` intent family, untimed.
  - **When it exists:** it is synthesized only for an OD8 template whose resolved interaction
    carries the typed `triggering_object` `REMOVE` (10.3 D5). Today that means `mazzinor` and
    `cult_soul_remains`. `azerus`, `gaz_haragoth` and OD9 have none.
  - **Its owner:** its D91 origin is `EVENT(owner)` with the resolved interaction as owner, so the
    step-in interaction child commits it through the scope owner. D91 gives an edge exactly one
    owner, so the revert driver and the interaction each need their own edge, and neither is
    ever a bypass of the other.
  - **Why it is not a second inverse.** It has the same states and family as `/remove`, so under
    the current rule it would qualify as a second inverse of the timed create, and `bind` would
    fail with "ambiguous bound inverse". The unique-inverse search therefore gains one condition:
    a candidate inverse must carry the same D91 origin as the forward it reverts. That is §7's
    own premise, since the revert is the same owner's later operation. `/consume` has a
    different owner, so it never qualifies, and `/remove` stays the unique inverse. The condition
    is inert for every placement without D91 origins, which is all content today. D91 is already
    a precondition for these shapes (10.3 D5).

Because a revert lands exactly on the absent state, where the authored forward matches again, no
separate re-arm edge is needed. This is the case `encounter_map_item.rs` already describes for a
`revert_after_ms` without `revert_destination` (D90).

**What changes in the §7 machinery (Round 4, Codex P1 4127532494).** `prepare`, `commit` and
`PreparedMutation` do not change. The lifecycle record's fields, states and presentation order do
not change either, with two exceptions, both used only by OD8 runtime placements (10.3 D3):

- **One new flag on `PendingRevert`.** It is set at scheduling when the target placement is
  runtime-created.
- **One new rule in `ScopeRevertDriver::terminalize`.** A record carrying that flag is released
  when it becomes `TERMINAL` instead of being retained for the scope generation. This is the
  narrow exception to §7 open decisions 1 and 5 that D3 argues for.

For every pre-authored placement, including all OD9 teleporters and all §9 content, the flag is
never set and the driver behaves exactly as it does today. Separately, `bind`'s unique-inverse
search gains the same-origin condition from the consume-edge bullet above. It has no effect until
D91 origins exist, and it has no effect on any placement without a second edge of the paired
family.

### 10.3 OD8: runtime placements at `death_position`

**D1. Template, lowered from content.** CW3 lowering turns each admitted
`map_item create … at: death_position` action into one immutable `RuntimePlacementTemplate`,
keyed by its `LoweredActionId`. The template holds:

- the `LocalObject` definition of the created item, supplied by the caller the way
  `anchor_objects` is for transforms;
- the absent and present states, and the create/remove transitions, from 10.2, added to that
  definition's vocabulary;
- `initial_state = <action id>/absent`;
- the present state's attributes;
- the one `revert_after_ms` entry;
- a collision footprint of exactly one cell, `(0, 0, 0)`.

The template is ordinary immutable content (DUR-04). It carries no position.

- **Admitted shape:** `operation: create`, `at: death_position`, `item`, and `revert_after_ms`
  (required, positive). Optionally, exactly one of `destination` (an anchor, lowered to a §9
  marker placement) or `interaction`. Optionally `effect`, which is presentational and ignored as
  in §9.
- **Rejected fail-closed with named errors:**
  - a missing `revert_after_ms` (an untimed runtime object would live until the scope restarts);
  - `revert_destination`;
  - both `destination` and `interaction`;
  - an `interaction` key that does not resolve to exactly one compatible interaction-domain
    definition in the same compiled content (D5; DUR-04);
  - any other field;
  - a created item whose declared state is not `collision: Absent` (C3; see D4);
  - `at: death_position` on a trigger that is not a death or lethal-damage trigger (the existing
    encounter-format rule, `OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md` line 152).
- **One template per branch.** Each `one_of` branch has its own `LoweredActionId`, so
  `cult_soul_remains` lowers to two templates (items 32414 and 32415). Its runtime `one_of` picks
  one per death, as Canary's `math.random` does.

**D2. Runtime placement: identity, fencing and bind.** When the owning event fires, the scope owner
(`ChannelRuntime`/`InstanceRuntime`, D38 W1) runs one synchronous owner-turn step:

1. **Resolve the position.** `death_position` is the dying creature's committed position, read
   from the scope's own position owner (VSL-MOVE-01) at the death commit. A `delay_ms` rule uses
   the position captured at the death (encounter format line 200).
2. **Check capacity.** The creation fails `CAPACITY_EXCEEDED` in either of two cases (D6):
   `WOBJ-RL-08` is already at its limit, or the scope's bound local objects (pre-authored plus
   live runtime) already number 486 (`WOBJ-RL-03`). Nothing is minted, bound or committed.
3. **Mint the key.** `PlacementKey = oteryn-runtime:<action id>/<n>`. `n` is a scope-owned `u64`
   creation sequence that starts at 1 and is never reset or reused within one scope generation;
   overflow fails closed as `CAPACITY_EXCEEDED`. The CW3 linker refuses the reserved namespace
   `oteryn-runtime` on every authored placement, and bind also checks that the key is absent from
   `content.placements`. Lowering rejects an action id that would push the key past
   `FIRST_PRODUCTION_MAX_KEY_BYTES` (512).
4. **Bind.** The scope owner synthesizes a `PlacementRef` from the template, the key and the
   resolved cell:
   - the address is the scope's World and Content coordinate frame;
   - the evidence is non-promotable, as for the entry door;
   - the footprint is the template's one cell;
   - `local_object_initial_state` is the absent state;
   - the template's attribute and revert tables are copied in.

   It then binds the `PlacementRef` with `incarnation = 1`, through exactly the validations of
   `LocalObjectRuntime::bind`. This is the existing injection precedent:
   `bind_native_entry_door` (`apps/game-server/src/world_runtime.rs` ~409-489) binds a
   synthesized placement under the unchanged content-generation fence, because an injected
   placement changes nothing `ReferenceContentGeneration::from_content` hashes. The owning lane
   may factor `bind`'s body so that it takes the synthesized `PlacementRef` directly and avoids a
   content clone per creation. The validations must stay identical. This is the whole answer to
   "`bind` requires a pre-existing `PlacementKey`"; there is no second binding path.
5. **Create.** `ScopeRevertDriver::apply_forward` runs with the create transition. That step mints
   the scheduling ordinal and schedules the §7 record in the same staged commit.
   - On `COMMITTED`, the runtime is inserted into the scope's `runtimes` map (the one
     `ScopeRevertDriver::wake` already takes) and its overlay delta is published.
   - On any other outcome or error, the freshly bound runtime is dropped. It was never inserted
     or published: it sat at revision 0 in a non-visible state, so dropping it is unobservable.

**Fences.** The runtime object carries the ordinary fences: scope and scope generation (checked
by `apply_scope_operation`), placement, incarnation and content generation (checked by `prepare`),
and the §7 record fences. Its key never repeats within a generation, so no later binding can match
an old record.

**Safety conditions for the creation step:**

- **C-A (in-turn only).** A runtime object is created only inside the scope owner's own turn
  that commits the triggering event. It is never created from an externally presented, queued or
  retried input. There is therefore no duplicate presentation of a creation to de-duplicate, and
  the creation sequence is its identity. The encounter-trigger wiring must preserve this.
- **C-B (bounded).** A creation is refused unless `WOBJ-RL-08` has room and the template carries
  `revert_after_ms`. The number of live runtime objects is therefore bounded, and so is each
  object's lifetime.

**D3. Lifetime, unbind and cleanup.** A runtime object is retired, meaning removed from `runtimes`
with its `WOBJ-RL-08` slot freed, in the same owner turn in which any commit lands it on its
absent state:

- **Timed removal.** The §7 record fires the remove inverse (`wake` → `present` →
  `apply_scope_operation`). `COMMITTED` lands on absent, and the scope owner retires the object.
  This is Canary's `removeTeleport`.
- **Consumed first (`mazzinor`, `cult_soul_remains`).** The step-in interaction child (the
  `interaction` content, GAME-INTERACTION) asks the scope owner to commit the object's own
  interaction-owned `<action id>/consume` edge (10.2). The object lands on absent and is retired. Its still-`PENDING` record later reaches `present`,
  finds no runtime at the placement, and becomes `TERMINAL` as `Fenced(Incarnation)` with no
  mutation. This is Canary's "item already gone, timer does nothing".
- **Last overlay word.** The delta that publishes the absent state is the last one sent for that
  key; later snapshots omit the retired placement. No new wire message is needed.
- **Scope restart (C2).** The scope's runtime objects, the creation sequence and the revert
  driver are all scope-ephemeral, so a restart drops them together; nothing is persisted (W2).
  Canary does not persist script-created map items either.
- **Content-generation change.** If the scope owner ever rebinds its local objects under a new
  content generation, runtime objects are retired, not rebound. Their `PENDING` records fence
  (`ContentGeneration` or `Incarnation`) as they do today.
- **Record release (a narrow exception to open decisions 1 and 5).** A lifecycle record whose
  placement is a runtime key is released when it becomes `TERMINAL`. It is not retained for the
  generation.
  - `PendingRevert` gains one flag, set at scheduling when the placement is runtime-created.
  - Why the exception is needed: without it, `cult_soul_remains` and `gaz_haragoth`
    (`channel_shared`) would exhaust `WOBJ-RL-04` (1,024 records per generation) after 1,024
    deaths in one long-lived Channel generation. From then on no remains would appear, which
    deviates from Canary.
  - Why it is safe: GAME-INTERACTION-01 §7 requires that the loss of a retained result never
    re-enables execution, and that holds structurally here. The released record's placement key
    is retired and never re-bindable (D2 step 3). The forward cannot be presented again (C-A). A
    late re-presentation of the revert identity returns `UnknownOccurrence`, never an execution.
  - `WOBJ-RL-04` still bounds records in any state at any moment. Only the retention horizon
    differs for runtime keys.

**D4. Collision and footprint: why C3's exclusion is lifted, and on what conditions.** C3 (§4, "Out
of scope") excludes dynamically materialized geometry. That is a `CREATE` that reserves cells
unknown at bind time, which would add blocking truth outside the fixed bind-time footprint the
movement owner and the `OCCUPIED` checks reason about. OD8 needs a runtime position but no
geometry, because every created object is walk-onto (Canary property 2). The exclusion is
therefore lifted only for runtime objects that contribute nothing to collision, under these
conditions:

- **C3-1.** Every state of a runtime template is `collision: Absent`. This is checked at lowering
  (D1) and again by the runtime bind, fail-closed. Absent collision means `blocking_cells` is
  always empty, `OCCUPIED` can never apply (`prepare` checks it only for a `Present` target), and
  movement truth is unchanged. This also matches Canary property 1: a creation succeeds whoever
  stands on the cell.
- **C3-2.** The footprint is exactly one cell at the resolved position. It is used only for
  presentation and addressing, never for reservation.
- **C3-3.** The resolved cell is a committed position of a creature inside this scope, so it lies
  in the scope's World and frame. `bind`'s address checks re-verify this.
- **C3-4.** Several runtime objects may share a cell with each other and with pre-authored
  placements (Canary property 4). They are distinct placements, and none of them blocks.

Runtime objects with collision `Present` (for example, a wall materialized at a creature's
position) stay excluded by C3, unchanged. That would be a new decision.

**D5. Attributes and the timed revert.**

- `LocalObjectStateAttributes` gains one optional field, `interaction: Option<ProductionKey>`. It
  holds the key of interaction-domain content (D29) and is set only on the present state. This is
  what `mazzinor`, `gaz_haragoth` and `cult_soul_remains` need.
  - **Resolved before activation, fail-closed (Round 2, Codex P1 4127432502; DUR-04: an
    unresolved reference fails compilation).** Lowering, or the CW3 link that consumes its
    output, resolves every `interaction` key against the interaction-domain definitions of the
    same content set being compiled. This happens before that content can activate a scope. The
    authored key names an action-id registration: `canary:interaction/<aid>` resolves to the
    interaction definition whose `source.target_registrations` contains `aid(<aid>)`. That is
    the shape of the transcribed corpus
    (`tools/content-schema/quest-authoring/samples/interactions/interactions.json`; for example
    `canary:interaction/the_secret_library_quest/library_area/movements_mazzinor`, which carries
    `aid(4951)`). The resolved key is what the template stores.
  - **Compatibility.** Exactly one definition must resolve, and it must meet three conditions:
    - its `source.edge` is `ON_ENTER`, since each covered object is used by stepping onto it;
    - every `WorldObject` child it carries is a `REMOVE` whose **typed** target is the
      triggering object itself, which lowers onto this template's own interaction-owned
      `<action id>/consume` edge (10.2), never onto the revert-owned `/remove`;
    - it names no other world-object target.

  - **No inferred targets (Round 3, Codex P1 4127486973).** The linker never infers a mutation
    target from `value_source_line`, a source-line reference, an item id or any other untyped
    evidence. A world-object mutation target is security-relevant, and an inferred one could
    remove an object the interaction does not own.
    - This follows `docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md` lines 54-58: explicit
      contracts, strong typing and schema validation over inference.
    - Today's `REMOVE` shape in `tools/content-schema/quest-authoring/interaction.schema.json`
      (~128-134) is `anchor`, `def` or `value_source_line`, and it has no representation for "the
      object that triggered this interaction".
    - The transcribed `aid(4951)` and `aid(5580)` definitions carry only `value_source_line` on
      their `REMOVE` child.
    - Such a child, and any `WorldObject` child without a typed target this rule accepts, fails
      compilation with the named error `InteractionWorldObjectTargetUntyped`.
    - An `anchor` or `def` target is also rejected here (`InteractionWorldObjectTargetForeign`),
      because it names an object other than the triggering one.
  - **Prerequisite owned by the interaction lane.** The interaction contract must gain a typed
    self/trigger target for `REMOVE`:
    - **Contract:** `tools/content-schema/quest-authoring/interaction.schema.json`, with its prose
      in `docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md` §6.3.
    - **Minimal shape:** one more alternative on the existing `WorldObject` `REMOVE` variant,
      `"target": {"kind": "triggering_object"}`. It mirrors the `kind`-tagged
      `relocation_target`, and it is admissible only on a definition whose `source.edge` is
      `ON_ENTER` with an `aid(...)` registration (Round 4, Codex P2 4127532499). Every covered
      object is used by stepping onto it. Admitting the target on a `USE` edge would need its own
      decision.
    - **Transcription:** the transcriber sets it only where the source provably removes the
      callback's own `item` argument (for example `movements_mazzinor.lua:15` `item:remove(1)`).
    - **Ownership:** this section does not edit that contract; the interaction lane adds it.

    Resolution also fails when no definition, or more than one, carries the registration, and
    when the definition is unresolved or incompatible. Each such failure is named, and the
    content does not compile or activate. No runtime fallback exists and no key is left
    unresolved.
  - **Current state.** `mazzinor`, `gaz_haragoth` and `cult_soul_remains` stay rejected
    fail-closed, each with a named error, until the prerequisites below are met. `azerus`
    (`destination`) is unaffected.
    - `content/interactions/` is `READY_UNPOPULATED`, so today every `interaction`-carrying
      template fails resolution (`InteractionUnresolved`).
    - `mazzinor` (`aid(4951)`) and `cult_soul_remains` (`aid(5580)`) also need the typed
      triggering-object `REMOVE` target above. Until it exists, their transcribed definitions
      fail with `InteractionWorldObjectTargetUntyped`.
    - `gaz_haragoth` (`aid(33542)`) has no transcribed definition, because its Canary handler,
      `movements/roshamuul/strange_vortex_tp.lua`, is outside the quest transcription. It fails
      with `InteractionUnresolved`. It becomes admissible only when a definition exists, and
      only if that definition either carries no `WorldObject` child or carries a typed one.
  - Executing the resolved interaction on step-in (outfit, teleport, quest storage, consumption)
    is the interaction lane's consumer, which reads it through the existing `attributes()`
    accessor. That consumer is out of scope here, as the §9 teleport consumer is.
- `destination` (`azerus`) is the §9 attribute, reused as is.
- Both attributes are a pure read of `(state_attributes, state)`: present exposes them, absent
  exposes none.
- The timed revert is §7, plus the runtime-placement flag and terminal-release rule from 10.2.
  The record's inverse is the remove edge, its expected
  state is present, its deadline is `revert_after_ms` from the create commit, and a timer-origin
  firing schedules nothing. No re-arm or extension exists, since each death creates a new object
  (Canary property 6).
- **Dependency on D91.** The create forward is timed, so it is never USE-selectable or
  session-invocable (existing rule). The untimed `/remove` and `/consume` edges must not be
  either: otherwise a player's USE on a vortex or teleporter would delete it, which Canary does
  not do. D91's typed origin (`EVENT(owner)`, excluded from `select_use_transition` and session
  `apply`) is therefore a precondition for admitting these shapes.
  - `/remove` is committed only by the revert driver, for its own owner.
  - `/consume` is committed only on behalf of the owning interaction child. That is not a USE.
  - Neither edge is ever committed by the other's owner.

**D6. Capacity: proposed row `WOBJ-RL-08`.** This is named here only; `RESOURCE_LIMITS_REGISTRY.json`
is not edited by this section.

- **Resource:** live runtime-created local objects bound by one scope owner at once.
- **Unit and value:** objects per scope; proposed hard maximum 64, configurable 1-64.
- **Failure:** `CAPACITY_EXCEEDED`. It is checked before the key is minted or anything is bound,
  so the creation commits nothing: no object, no record, no ordinal.
- **Combined overlay check (Round 2, Codex P1 4127432516).** A runtime object is one more entry
  in the scope's `WORLD_OBJECT_OVERLAY` snapshot, which carries one entry per bound local object
  and is bounded by `WOBJ-RL-03` (486). Pre-authored placements alone may use all 486, so no
  fixed share is left over for runtime objects.
  - The creation step (D2 step 2) therefore also refuses, with `CAPACITY_EXCEEDED`, unless the
    scope's bound local objects (pre-authored plus live runtime, the size of the `runtimes` map
    the scope owner already holds) number fewer than 486. Materialization can therefore never
    push a snapshot past `WOBJ-RL-03`.
  - This is the smaller of the two options: one comparison at materialization, instead of
    reserving 64 slots at content validation. It changes no content validation, and it leaves a
    pre-authored-only scope's existing bound unchanged.
- **Allocation impact:** memory per object is one `LocalObjectRuntime` (bounded keys) plus one
  `WOBJ-RL-04` record.
- **Boundary tests:**
  - At 64 live runtime objects the create commits; at 65 it fails before bind, with the object
    count, overlay and records unchanged. A retire frees a slot.
  - **Nearly full pre-authored scope:** with 485 pre-authored objects bound, one creation commits
    (486 in total) and the next fails `CAPACITY_EXCEEDED` before bind, although only 1 of the 64
    runtime slots is used. With 486 pre-authored objects bound, the first creation fails. After a
    retire, a creation commits again, and every snapshot encodes within `WOBJ-RL-03`.

The value is lane policy, not a measurement. The owning lane files the `#139` packet with it. The
same packet amends the `WOBJ-RL-04` allocation note for D3's record release.

### 10.4 OD9: pre-authored `CREATE` teleporters

- **Lowering.** The `death_priest_shargon` and `the_ravager` actions are a `map_item create` at a
  pre-authored `anchor` carrying `destination` and `revert_after_ms`. They lower onto the anchor's
  placement, whose `LocalObject` definition the caller names in `anchor_objects` as for transforms:
  - the absent state `<action id>/absent` and the create/remove pair from 10.2 are added to that
    definition;
  - the placement's `local_object_initial_state` is the absent state;
  - `local_object_state_attributes[present].destination = Some(<destination marker>)`, so the
    absent state carries none;
  - `local_object_revert_after_ms[(<action id>/create, action id)] = revert_after_ms`.

  The forward runs from absent to present: the owner's "§9 transform from a synthesized absent
  state". It runs on the §7 driver's existing retention path, since a pre-authored placement
  never carries 10.2's runtime flag, and on the §9 attribute tables.
- **Admitted shape.** `create` at an `anchor` with `destination` and `revert_after_ms` both
  required, plus optional `effect`.
- **Still rejected fail-closed:** `revert_destination` on a `create` (reverting to absent leaves
  nothing to carry a destination), `interaction` on an anchor `create`, and any other field.
  `CreateWithDestinationNotAdmitted` is retired for exactly this shape.
- **Collision.** The absent state is `collision: Absent`. The present state uses whatever the
  definition declares for the item; the teleporter 1949 is `Absent`. The anchor's footprint is
  pre-authored and fixed at bind time, so C3 holds unchanged. A `Present` item would get the
  existing `OCCUPIED` check. No covered sample has one, so no Canary deviation arises; see
  `FLAG_NOLIMIT` in 10.1.
- **Re-arm, consistent with D90.** The revert lands exactly on the natural initial state (absent),
  where the authored forward matches again. `select_timed_forward`
  (`apps/game-server/src/world_object_revert.rs` ~90-106) then behaves as follows:
  - it returns the create edge from absent;
  - it returns `None` while the teleporter is present, so a kill while open commits nothing, mints
    no ordinal and extends or resets nothing (owner decision 3);
  - it returns the create edge again after the revert.

  No `/rearm` edge is lowered; D90's synthesized C→B edge exists only for the
  `revert_destination` variant.
- **Before the first kill.** The placement is bound, non-visible and non-colliding, and it exposes
  no destination. Stepping on the anchor does nothing, as on Canary's empty tile.
- **The D91 dependency from 10.3 D5 applies too.** USE on an open teleporter must not select the
  remove edge.

### 10.5 PLAYABLE_FIRST scoping

This section adds:

- one `bool` marker on `LocalObjectStateDefinition` (`absent`);
- one optional field on `LocalObjectStateAttributes` (`interaction`);
- one immutable content artifact per `death_position` action (`RuntimePlacementTemplate`);
- one scope-owned creation sequence and live-object count, with the retire-on-absent rule;
- one interaction-owned `/consume` edge per interaction template, and the same-origin condition in
  `bind`'s inverse search (10.2);
- one flag on `PendingRevert` for record release;
- one proposed limit (`WOBJ-RL-08`);
- the lowering of `create` into the absent/present pair.

It adds no second binding path, no change to `prepare`, `commit` or `PreparedMutation`, no
persistence, no collision-bearing runtime geometry and no new wire message. The only driver
change is 10.2's flag and its terminal-release rule, which apply to runtime placements only.
It does not implement:

- the step-in interaction consumers (outfit, teleport, quest storage, consumption);
- the teleport consumer;
- the encounter-trigger wiring;
- the client-side rendering of absent states.

Each of these belongs to its existing owner.

### 10.6 Exact test obligations

OD8:

- **Lowering.** Each of `mazzinor`, `gaz_haragoth`, `cult_soul_remains` (two templates, one per
  branch) and `azerus` lowers to the template in D1, given compiled content that resolves its
  `interaction` (D5):
  - an absent state with `collision: Absent`, `absent: true` and no attributes;
  - a create/remove pair with the `CREATE`/`REMOVE` families;
  - the present-state `interaction` or `destination`;
  - one `revert_after_ms` entry.

  Each D1 rejection (a missing `revert_after_ms`, `revert_destination`, both attributes, an
  unknown field, a `collision: Present` item, the reserved namespace on an authored placement, an
  over-long key) fails with a named error.
- **Interaction resolution (Round 2, Codex P1 4127432502).**
  - With interaction-domain content that registers `aid(4951)` on an `ON_ENTER` definition whose
    only `WorldObject` child is a `REMOVE` with the typed target `{"kind": "triggering_object"}`
    (the Round 3 prerequisite), `mazzinor` lowers, and its
    template stores that definition's resolved key.
  - Each of the following fails compilation with a named error, and no content activates:
    - no definition for the aid (today's `READY_UNPOPULATED` state, and `aid(33542)` for
      `gaz_haragoth`);
    - two definitions for the aid;
    - a `USE`-edge definition;
    - a definition whose `WorldObject` child targets another object or is not a `REMOVE`;
    - an unresolved definition.
  - **Untyped `REMOVE` children are rejected (Round 3, Codex P1 4127486973).**
    - A `REMOVE` child carrying only `value_source_line` fails with
      `InteractionWorldObjectTargetUntyped`. This is the transcribed `aid(4951)` and `aid(5580)`
      shape, used verbatim as the fixture.
    - A `REMOVE` child carrying `anchor` or `def` fails with
      `InteractionWorldObjectTargetForeign`.
    - Only `"target": {"kind": "triggering_object"}` passes.
    - A test asserting that the linker derives the target from `value_source_line`, an item id or
      a source line must fail.
  - A test asserting that an `interaction` key passes on syntax alone must fail.
- **Bind parity.** A runtime placement binds through exactly `bind`'s validations. A synthesized
  `PlacementRef` that fails any of them (foreign definition, undeclared initial state, an
  inverse-less timed edge, a key present in `content.placements`, a `Present` state) is rejected
  before a runtime exists.
- **Create, then timed removal (the `azerus` shape, `ManualClock`).**
  - Death at cell P: the object binds at absent, `apply_forward` commits create (absent → present,
    revision 1) and schedules one record.
  - `attributes()` exposes `destination`.
  - After `revert_after_ms` the remove edge commits, the object is retired (absent from
    `runtimes`), its slot is freed and its record is released.
  - A second presentation of the revert identity returns `UnknownOccurrence` and mutates nothing.
- **Consumed first (the `mazzinor` shape).**
  - The interaction-owned `/consume` edge commits before the deadline, and the object is
    retired.
  - At the deadline, the record becomes `TERMINAL` as `Fenced(Incarnation)`, with no ordinal
    minted for `prepare` and nothing mutated, and is then released.
- **Occupied cell.** A creation at a cell occupied by a player commits (never `OCCUPIED`), and
  `blocking_cells` stays empty in both states.
- **Same cell, two deaths.** Two creations at P get distinct keys (`…/1`, `…/2`), are both present,
  and are each removed by their own record. A key is never reused after retirement within the
  generation.
- **Capacity (`WOBJ-RL-08`).** At the configured maximum N (tested at N=1), the next creation fails
  `CAPACITY_EXCEEDED` with no key minted, no bind, no record, no ordinal and no overlay change.
  After a retire, a creation succeeds.
- **Nearly full pre-authored scope (`WOBJ-RL-03`, Round 2, Codex P1 4127432516).**
  - With 485 pre-authored objects bound, one creation commits and the next fails
    `CAPACITY_EXCEEDED` before bind, although `WOBJ-RL-08` has room.
  - With 486 pre-authored objects bound, the first creation fails.
  - After a retire, a creation commits again.
  - The overlay snapshot encodes within 486 entries in every case.
- **Failed create is unobservable.** If `apply_forward` fails (for example `WOBJ-RL-04` is full),
  the bound runtime is dropped, and no overlay entry or record exists for its key.
- **Scope restart.** Dropping the scope drops all runtime objects, the sequence and the records.
  A new generation starts from an empty set, and its sequence restarts.
- **USE and session exclusion (after D91).** USE on a present runtime object selects nothing, and
  session `apply` naming its create, remove or consume edge is refused.
- **Two REMOVE edges, one owner each (Round 5, Codex P1 4127573494).**
  - A `mazzinor` template binds both `/remove` (`EVENT` of the encounter owner) and `/consume`
    (`EVENT` of the interaction).
  - `bind` selects `/remove` as the unique inverse of the timed `/create`. `/consume` never
    qualifies, because its origin differs.
  - A test asserting that `bind` rejects the template as "ambiguous bound inverse", or selects
    `/consume` as the inverse, must fail.
  - The revert driver committing `/consume` is refused. The interaction committing `/remove` is
    refused.
  - A template without an interaction (`azerus`) lowers no `/consume` edge.
  - The same-origin condition changes no `bind` outcome for existing content without D91
    origins.

OD9:

- **Lowering and bind.** `death_priest_shargon` and `the_ravager` lower to an anchor placement
  whose initial state is absent, with the create/remove pair and `destination` on present.
  `bind` accepts the remove edge as the unique inverse of the create edge under the unwidened
  rule. Before any kill, `attributes()` is `None` and the placement does not block.
- **Kill, revert, re-arm (`ManualClock`).**
  - Kill 1 commits absent → present with the destination and schedules a record.
  - After 300,000 ms the remove edge commits back to absent.
  - Kill 2 selects create again and opens it. The run ends with two distinct `TERMINAL` records.
- **Kill while open is a no-op (owner decision 3).** While the teleporter is present,
  `select_timed_forward` returns `None`: no ordinal, no record and no mutation. The open record's
  deadline is unchanged, so no extension or reset happens.
- **Still rejected.** A `create` at an anchor carrying `revert_destination`, or `interaction`,
  still fails with a named error.

### 10.7 Owner questions (deviations from Canary only)

- **Q1 (OD8, capacity).** Canary creates a death-position object with no limit. This design caps
  live runtime objects per scope at `WOBJ-RL-08` (proposed 64). A death beyond the cap creates no
  object (fail-closed, no queue), so for example a soul remains would not appear during a
  pathological kill rate in one Channel. Does the owner accept this cap-and-skip, or does the
  owner want a different value or behaviour, such as evicting the oldest object? The
  recommendation is to accept: the repository requires every scope resource to be bounded
  (FND-03 §14.1), and 64 simultaneous one-minute objects in one scope exceeds what these hunting
  rules produce in normal play.
- **Q2 (OD9, kill while open).** Owner decision 3 cites the Depth script
  (`quests/dangerous_depth/creaturescripts_bosses_mission_depths.lua:22-26`: transform and schedule
  only when 1949 is present), which this design follows. The OD9 bosses differ in Canary: their
  scripts call `Game.createItem` unconditionally
  (`creaturescripts_kill_death_priest_shargon.lua:23`, `creaturescripts_kill_the_ravager.lua:13`).
  A second kill within the 5 minutes would therefore stack a second teleporter with its own
  timer, keeping a teleporter open until 5 minutes after the second kill. This design applies
  decision 3 to both (a no-op). Under D26 `instance_per_party`, the scope of both samples, a
  second kill in the same instance within the window would need a second boss spawn there, so the
  difference is not reachable in normal play. Does the owner confirm that decision 3 also covers
  these `CREATE` teleporters? The recommendation is yes.
- **Q3 (OD8, same-cell replacement; Round 2, Codex P2 4127432535).**
  - **Case:** object A (item X) at cell P is consumed on step-in, and object B with the same
    item id X is then created on P, all before A's timer fires.
  - **In Canary:** A's timer removes "item X found on tile P", which is now B, so B disappears
    early, at A's deadline instead of its own. See the lookup-by-id removals in
    `creaturescripts_mazzinor.lua:12-17`, `minion_gaz_haragoth_vortex.lua:1-6` and
    `creaturescripts_carlin_vortex_spawn.lua:6-11`.
  - **In this design:** each object is removed only through its own record. A's record fences
    (`Incarnation`), and B lives its full `revert_after_ms`.
  - **Question:** does the owner accept this deviation? The recommendation is to accept it. The
    early removal is an artefact of Canary removing by item id, not an authored rule, and every
    object then gets exactly its authored lifetime. Reproducing it would mean a lookup-by-id
    removal that reaches across objects, which is the kind of cross-object mutation this owner
    model excludes.

### 10.8 Open items for the owning lane

- The exact CW3 lowering code and linker checks for D1, 10.2 and 10.4, including the reserved
  namespace, and the representation of `RuntimePlacementTemplate`.
- The compile-time resolution of `interaction` keys against the interaction-domain content (D5).
  It needs that content to be part of the compiled set; until then, `interaction` templates stay
  rejected.
- **The interaction lane's prerequisite (D5, Round 3).** A typed
  `"target": {"kind": "triggering_object"}` on the `REMOVE` variant of
  `tools/content-schema/quest-authoring/interaction.schema.json`, with prose in
  `OTERYN_QUEST_AUTHORING_FORMAT_V1.md` §6.3, and its transcription for `aid(4951)` and
  `aid(5580)`. Until it lands, those samples stay rejected.
- Whether `bind` is factored to take a `PlacementRef` (D2 step 4). The validations stay identical.
- The scope owner's creation step and retire-on-absent hook, and the `PendingRevert` release flag
  (D3), in the live Channel/Instance owner. This is not yet wired, as for §7.
- How the transport projection renders an `absent` state. Server-side, it renders no object; the
  exact client mapping belongs to the transport lane.
- D91 enforcement, a precondition for admitting both shapes live (10.3 D5).
- The `#139` resource packet for `WOBJ-RL-08` and the `WOBJ-RL-04` note (D6).

### 10.9 Decision test (Round 4, Codex P1 4127532478)

This is the mandatory test of `docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md` ("Mandatory
decision test", "Required analysis shape"). It records the choices above and does not change
them.

1. **Must decide now?**
   - **OD9: YES.** The owner admitted the shape (item 2). The two samples are rejected at lowering
     today, and their boss exits are the same live teleporter path as the Duke wiring (#162, "Live
     wiring: approved").
   - **OD8: YES** for the design (item 1: "design now"). Implementation stays gated by 10.8's
     prerequisites. The runtime identity, the retention exception and the `WOBJ-RL-03`
     interaction touch the already-implemented §7 driver and an accepted wire bound. Deciding
     them later would mean retrofitting shipped code.
2. **What it unblocks.**
   - **OD9:** lowering and live wiring for `death_priest_shargon` and `the_ravager`.
   - **OD8:** `azerus` as soon as it is implemented, because it has no interaction prerequisite.
     `mazzinor` and `cult_soul_remains` follow once the interaction lane's typed
     `triggering_object` target and their definitions land. `gaz_haragoth` follows once its
     `aid(33542)` definition exists.
   - **Both:** the CW3 lowering and linker work and the scope-owner creation hook in 10.8.
3. **What becomes harder later.**
   - The reserved `oteryn-runtime` placement namespace becomes a content-wide commitment.
   - The `absent` marker becomes part of the shared `LocalObjectStateDefinition` shape.
   - A released runtime record answers a late re-presentation with `UnknownOccurrence`, not its
     original outcome. A future consumer that needs the old outcome would need a new retention
     rule.
   - `WOBJ-RL-08` becomes player-visible behaviour at the cap.
   - The combined check ties runtime creation to how densely a scope is pre-authored.
4. **What would supersede it.**
   - Measured live runtime objects per Channel near or far from 64, which would retune
     `WOBJ-RL-08`.
   - A requirement for collision-`Present` runtime objects, which needs a new C3 decision.
   - A frozen GAME-INTERACTION-01 duplicate-delivery horizon (§7 open decisions 1 and 5) that
     covers runtime records differently.
   - A requirement for durable world-object state, which would reopen W2.
   - A `USE`-edge interaction that removes its own object, which needs its own decision (D5).
   - The owner's answers to Q1-Q3.
5. **What is not decided.**
   - Interaction and teleport consumers, encounter-trigger wiring and client rendering of
     `absent`.
   - D91's implementation.
   - Presentation order of stacked objects.
   - Collision-`Present` runtime objects.
   - A `USE`-edge `triggering_object` target.
   - Exact code representations (template, flag, key spelling), which are 10.8's.

**Options and trade-offs for the material choices** (the chosen option is listed first):

| Choice | Chosen | Alternative | Why chosen |
|---|---|---|---|
| Runtime identity (D2) | Scope-monotonic sequence in a reserved-namespace `PlacementKey`: never reused and needs no memory. | Key derived from the creating occurrence identity. It recognizes duplicate creations, but a retired key must then be remembered for the whole generation (unbounded tombstones), and occurrence identities (`WOBJ-RL-07`, 4,096 bytes) do not fit the 512-byte key bound. | Bounded and simple. The cost is safety condition C-A: creation happens in-turn only, since a sequence cannot recognize duplicates. |
| Record retention (D3) | Release a runtime placement's records at `TERMINAL`; this is structurally safe because the key is retired and the forward cannot recur. | Keep §7 open decisions 1/5 as they are: no exception, but `WOBJ-RL-04` runs out after 1,024 creations per Channel generation, a Canary deviation for `channel_shared` remains. Or evict under a bounded dedup horizon, which GAME-INTERACTION-01 has not frozen. | It keeps `channel_shared` hunting playable without inventing a horizon. The cost: late re-presentations get `UnknownOccurrence`. |
| Capacity (D6) | `WOBJ-RL-08` = 64, plus a combined `< 486` check when an object is materialized. | Reserve 64 overlay slots at content validation (pre-authored ≤ 422). This guarantees runtime room, but constrains every scope, including those that never create objects. | One comparison and no content-validation change. The cost: a creation can be refused in a densely pre-authored scope. |
| Consumption edge (10.2, Round 5) | Two present → absent `REMOVE` edges, each with one D91 owner: `/remove` for the revert driver and `/consume` for the interaction. The inverse search requires the forward's own origin. | One shared `/remove` edge committed by both callers. That needs two owners on one edge or a D91 bypass, which D91 forbids. | Each caller commits only its own typed edge, and the inverse stays unique. The cost: one extra synthesized edge per interaction template, plus the same-origin condition in `bind`. |
| Contract prerequisite (D5) | A typed `REMOVE` target `{"kind": "triggering_object"}` in the interaction contract, `ON_ENTER` only, owned by the interaction lane. | Infer the target from `value_source_line`: no wait, but it is a security-relevant inference (discipline lines 54-58). Or drop the consuming `REMOVE`: the object would outlive a step-in, a Canary deviation. | Explicit and typed. The cost: `mazzinor` and `cult_soul_remains` wait on another lane. |
| OD9 model (10.4) | A synthesized absent state on the pre-authored placement, reusing §7/§9. | Treat OD9 as an OD8 runtime creation at a fixed cell. That duplicates the runtime-placement machinery for a place that already has a placement, and loses D90's plain re-arm. | Smallest: one marker and a lowering rule. |
