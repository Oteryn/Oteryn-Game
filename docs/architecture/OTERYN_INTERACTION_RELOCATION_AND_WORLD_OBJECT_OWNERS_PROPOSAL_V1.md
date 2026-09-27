# Interaction relocation and world-object owners: proposal v1

- Date: 2026-09-27
- DecisionStatus: CANDIDATE with owner decisions D37 and D38 taken (§6, 2026-09-27). The contract
  text becomes accepted after the independent review that authority changes require; until then
  the Movement and WorldObject children stay blocked.
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
  | `RETAG(action)` | the action-id change that re-arms a trigger |

- **Timed revert.** Every operation may carry `revert_after`, which covers `decay` and
  `revertItem`/`addEvent` reverts. The revert is the same owner's own later operation, with its
  own identity derived from the first.
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
  - Multi-cell footprints.
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

## 7. Follow-up

1. The quest transcription splits `remove` into overlay removal and DUR-03 consumption (done in
   the same change as D37 and D38; see the quest format §6.3).
2. Anchors bind to world placements.
3. The GAME-INTERACTION-01 successor names these owners in §19.3.
4. Independent review of the accepted text, then implementation in the scope runtime.
