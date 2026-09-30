# QUEST-GATE-0 Quest gates, triggers, NPC quest dialogue and the quest log

- Decision: `QUEST-GATE0-WORLD-GATES-AND-NPC-QUESTS-V1` (covers QUEST-GATE-0 and NPC-QUEST-0)
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (protocol,
  persistence and security) and protected integration. Owner questions Q1 and Q2 (§12) are open;
  only §5.4 gold hand-ins and §7 journal text wait on them.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the callers QUEST-STATE-0 names (NPC-QUEST-0 dialogue, QUEST-GATE-0 doors) and its
  deferred quest log wire (A8); NPC-0 §11 (quest-conditioned dialogue, answer 4b) and §3.4
  (quest-gated routes); the owner's direction of 2026-09-30: build now, full Tibia Global parity
- Builds on: QUEST-STATE-0 (§3 store, §4 transitions and causes, §5.2 sequencer, §5.4
  obligations, §7 predicates); the quest format V1 (§3.1 doors, §3.2 quest log, §3.3 and D36
  interactions, §4 `Gate`, §6.1, §6.5, §6.7); NPC-0 (§2.1, §4, §4.1, §5.1); the NPC schema
  (§2 `gate`/`effect`, §4 conditional services, O4, D9, D10); D37 and D38 (the relocation and
  world object owners proposal §3 and §4); D39 and the successor sections it accepts; D40-D42
  and CHEST-1; WO-0 (`door` kind, LocalObject relation §4.4); ADR-0021 §4.5 (legacy ids as
  source bindings); MAP-WIRE-1 (§3 allowlist, §6 `map_item_handle`); USE-WIRE-V1; ITEM-USE-0
  (§3, keys deferred);
  the composition decision (rules 1-6 and the quest obligation amendment); the XP writer
  (`character_progression.rs`); DUR-03 §15, §17, §39.3; D178; owner rule 5905825574
- Amends, each pending on acceptance of QUEST-GATE-0, in this PR: QUEST-STATE-0 (callers and A8
  pointer); NPC-0 §3.4 and §11; the quest format §3.1; the GAME-INTERACTION-01 successor header
  (accepted sections for §4 edges); the relocation and world object owners proposal header (§3 and
  §4 for gates and triggers); DUR-03 §15 and §39.3 (`QuestExchangeCause`, the dialogue claim
  source); the composition decision (exchange obligation).
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| QUEST-GATE-1 | hard, security review | `Gate` lowering bound to door `placement_key`s; gate checks at USE and at step-in in the channel runtime; open, pass-through, push-back and close (§3) | QUEST-PRED-1; MAP-LOAD-1; MAP-WIRE-2; QUEST-CONTENT-2 |
| QUEST-TRIGGER-1 | hard, persistence review | `USE`, `ON_ENTER` and `ON_LEAVE` interaction triggers on placed objects and tiles, their occurrence roots, and quest, relocation and overlay children (§4) | QUEST-STATE-1; QUEST-GATE-1 |
| NPC-QUEST-1 | hard, persistence review | typed quest conditions and outcomes in the NPC talk runtime; confirmation binding; the dialogue claim; the exchange transaction (§5) | NPC-TALK-1; QUEST-STATE-1; QUEST-PRED-1; CHEST-1 (merged) |
| QUEST-XP-1 | hard, persistence review | the quest XP obligation and its XP writer path (§5.5) | QUEST-STATE-1; CHAR-REV-SEQ-1 |
| QUEST-LOG-WIRE-1 | impl, protocol review | capability `QUEST_LOG_V1`, command and domain; list, quest line and tracker views (§7) | QUEST-PRED-1; QUEST-CONTENT-1 |
| QUEST-CONTENT-2 | content lane | gates, gated teleports and tiles, levers and step triggers as data, bound to placements (§8) | QUEST-CONTENT-1; MAP-BUNDLE-1 |
| NPC-QUEST-CONTENT-1 | NPC content lane | typed quest conditions and outcomes on Dialogue nodes from `requested_by` (§8) | NPC-CONTENT-1; QUEST-CONTENT-1 |

Capability, command and domain numbers are reserved on #162 at allocation. Later, each with its
own decision: key doors and keys (KEY-DOOR-0, with ITEM-USE-0's deferred keys), outfit, addon and
mount grants, relocation to another channel or instance (`SCOPE_HANDOFF`), boss rooms.

## 1. Question

How do world objects and NPCs read quest progress and move it, and how does the player see it?

## 2. Facts

**PROVEN**

- QUEST-STATE-0: tracks, quest states and receipts per Character; only named transitions write,
  requested with a cause bound 1:1 to its trigger (a CommandRef, a USE interaction occurrence, a
  claim obligation, a creature-death occurrence); receipts keyed by (character, cause, transition);
  one revision-advancing write in flight per Character (§5.2); a claim that also moves a quest
  writes an obligation row (§5.4); predicates read the session's copy and never write (§7).
- Door sample (format §6.1): 184 quest-progress gates, 38 key gates, 14 level gates, 1 lever gate;
  1 quest gate reads a reward claim; 176 quest gates link a wiki quest; 15 doors unresolved.
- Format §3.1: a quest door passes a character whose track is set and moves it through; a level
  door passes at level >= action id − 1000; both are checked per character on each passage. The
  sealed message and "Only the worthy may pass." are presentation.
- Interaction sample (§6.3): 599 `USE`, 400 `ON_ENTER`, 14 `ON_LEAVE` edges; 215 quest children
  name a mission transition; 217 relocations to an anchor and 203 to the previous tile.
- Transitions by owner (§6.2): NPC 1,442, action 182, movement 78, creature event 31. 1,240 NPC
  transitions carry keywords, 739 topics; `requested_by` names the NPC bundle key (§3.2).
- Readiness (§6.7): progress doors and NPC dialogue take the engine-complete quests from 106
  to 133.
- NPC schema: a keyword node's `gate` is `NONE` or `LUA_PREDICATE`, its `effect` `NONE` or
  `LUA_ACTION`; the first matching sibling whose condition passes answers (§4). D9 holds
  conditional and scripted nodes. O4 (scripted behaviour) is settled for quest state by D35.
- NPC-0: dialogue never commits value (brief); one occurrence (UUIDv7) per command bound to its
  CommandRef (§5.1); one pending service invocation per conversation (§4.1); quest-gated travel
  routes are held (§3.4); quest-conditioned dialogue was not decided (§11).
- D37 and D38 (proposal §3, §4): the scope runtime relocates within its scope and keeps a
  scope-ephemeral overlay (`TRANSFORM`, `CREATE`, `REMOVE`, `RETAG`, shipped by CW4 without
  `revert_after`); a request not committed in its tick is rejected; durable memory is quest state.
- D39 accepts successor §4.1, §4.3, §5.1, §5.3-§5.7, §17 and §19.1 for a `USE` on a `once`
  chest.
- ADR-0021 §4.5 keeps action, unique and door ids as source bindings; MAP-WIRE-1 §3 never sends
  them and names a base item by a per-session `map_item_handle` mapped to its `placement_key`.
- The XP writer takes an `ExperienceRewardOccurrence` (UUIDv7) and a `reward_revision`; it does
  not prove where the occurrence came from (`character_progression.rs:1-5, 31, 62-69`).
- Composition rule 1 covers item-only transactions and, pending on QUEST-STATE-0, a quest
  obligation row written by a reward claim. Rule 6: other Character writes need their own receipt.

**CIPSOFT_OFFICIAL** (the Tibia manual, `quests.md`)

- Quests are started by asking NPCs about "quest" or "mission"; a "not worthy" door is a level
  gate; locked doors need a key (§5.5.2).
- The quest log lists started quests as "Quest Lines", each with missions and their task text;
  completed quests carry a tick; search, "Show completed", "Show hidden", sort by name and a
  per-quest quest tracker are client features; one-off treasures have no entry, and outfit and
  addon quests leave the log once solved (§5.5.3).

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b51`, CrystalServer `9f5a72c6`; QUEST-GATE-1 confirms)

- The door scripts transform a passed door to its open item and move the player onto it; the door
  movement scripts push back a character that enters a quest or level door it may not pass, and
  close the door when the last creature leaves.
- The quest log protocol has a quest list request, a quest line request and a tracker update.

## 3. World gates (QUEST-GATE-1)

### 3.1 Declaration

- A gate is the format §4 `Gate` record in `content/interactions/` with its placements. Its
  condition is a conjunction of at most `QUESTGATE0-RL-01` (4) QUEST-STATE-0 §7 predicates:
  `track op value`, `elapsed`, `quest_completed`, `account_completed`, `level >= n`. The format's
  `quest_progress` and `min_level` conditions lower to these. `door_key` and the lever
  `shared_lock` gate are not lowered here (KEY-DOOR-0; the lever is a §4 trigger).
- The compiler binds each gate placement to the one door entry at that position: a WO-0 `door`
  WorldObject presented by a LocalObject with open and closed states (WO-0 §4.4), and so to its
  `placement_key` (ADR-0021 §4.2). No door, two doors, or a WO-0 `level door` value that differs
  from the gate's level holds the gate with a diagnostic. The held list is a production release
  gate (ADR-0021 §4.6).
- Legacy action ids stay source bindings only (ADR-0021 §4.5). A gate never leaves the server
  (MAP-WIRE-1 §3); the client sees a door and its overlay state.

### 3.2 Evaluation

- The channel runtime that owns the door checks the gate:
  - at `USE` of the door (USE-WIRE-V1 with the MAP-WIRE-1 `map_item {handle}` target);
  - at every step onto the door tile, whatever moved the character.
- It reads the session's track copy (QUEST-PRED-1) and the level. A gate check writes nothing
  durable: no receipt, no revision advance, no row. The predicate is final here, because nothing is
  written that a writer could re-check.
- **Pass at USE:** a D38 `TRANSFORM` of the door to its open state and a D37 relocation onto the
  door tile, in the same tick. **Fail at USE:** the gate's message; nothing changes.
- **Step onto a gated tile:** pass lets the step stand. Fail relocates the character to its
  previous tile (D37 §3) with the message; an open door does not let an unqualified character
  follow a qualified one.
- **Leave:** when the last creature leaves an open gated door, a `TRANSFORM` closes it (assumption
  A3, §12).
- Door state is scope-ephemeral overlay (D38 W2): a restart closes every door.

### 3.3 Fail closed

The door stays sealed, and a step is pushed back, when: the gate is held or its semantics are
unresolved (format §3.1); the track copy is not loaded; a predicate names an unknown track or
quest; the handle is `STALE`; the content generation of the gate differs from the channel's. A
track or quest the active content does not declare always seals the gate; it never reads as an
initial value. Only a known track declared by content whose character row is absent reads as its
declared initial value (QUEST-STATE-0 §3), and the predicate is then evaluated on that value.

### 3.4 Tiles, teleports and routes

- **Gated tile or teleport.** A FloorChange or Transition.Teleport record (ADR-0021 §4.6) may name
  a gate. Entering it checks the gate: pass relocates to the destination anchor (D37 §3); fail
  relocates to the previous tile. A teleport's destination stays in its Transition record.
- **Quest-gated NPC travel.** A route with a gate loads (amends NPC-0 §3.4). The talk runtime
  checks the gate before the confirmation, and the travel transaction re-reads the gate's tracks
  under the `character_root` lock (read only) with its other eligibility facts (NPC-0 §6).

## 4. Triggers that move quests (QUEST-TRIGGER-1)

- **Edges.** `USE` on a placed object (levers, statues, altars), `ON_ENTER` and `ON_LEAVE` on a
  tile or placed object, as D36 interaction definitions with read-only conditions and children.
- **Accepted successor sections.** For these edges, with quest, D37 relocation, D38 overlay and
  presentation children only, this decision accepts successor §4.1, §4.3, §5.1, §5.3-§5.7,
  §6.1 (canonical order), §6.2 (partial-progress recovery), §7 (child lifecycle and exactly-once
  rule), §17, §18 and §19.1 (D39's chest set plus §6.1, §6.2, §7 and §18). Each child of a
  firing, `after_quest` children included, is part of the root's plan: after a crash, recovery
  reproduces the same child set and order and runs each `UNSTARTED` child once if its fences
  still authorize it (§7), else it is `REJECTED`. Nested cascades and every other child kind stay
  `PROPOSED / NONCANONICAL`.
- **Roots.** `USE`: the `USE_INTENT` CommandRef. `ON_ENTER` and `ON_LEAVE`: the occurrence that
  moved the character, which is its own move command, another player's push command or a D37
  relocation child. A move with no such root (an admission placement) fires nothing (successor
  §18: no ad hoc identity).
- **Quest child.** `request_transition(fence, character, transition_key, cause)` with the child
  occurrence as cause (QUEST-STATE-0 §4). Several quest children request their transitions in the
  successor's canonical child order (§6.1) within one sequencer slot.
- **Dependent children.** A relocation, overlay or presentation child declared `after_quest` runs
  only on the quest child's `COMMITTED`, as a new request of the scope runtime; if its fences
  moved meanwhile it is `REJECTED_STALE` (D37 R2) and nothing moves. Other children run in the
  trigger's tick and do not wait.
- **Items.** An interaction hands out items only through a `RewardClaim` (the CHEST-1 path, D40)
  and takes carried items only through the §5.4 exchange. It never mints or burns by itself.
- **Bounds.** Trigger firings per character are limited by `QUESTGATE0-RL-03`. The limit is
  checked before the root commits, never after: a `USE`, move, push or relocation child whose
  firing would exceed it is refused before commit (architect ruling, fail closed). The `USE`
  changes nothing, the move or push is refused and the character stays on its tile, and a
  relocation child is `REJECTED`; no child runs and no firing is dropped. A firing is never
  discarded after its root has committed.

## 5. NPC quest dialogue (NPC-QUEST-1)

### 5.1 Conditions

- A Dialogue node's `gate` gains a typed `quest` condition: a conjunction of at most
  `QUESTGATE0-RL-01` QUEST-STATE-0 §7 predicates over the session's copy, plus the level and
  `holds_item` owners. The first matching sibling whose condition passes answers (schema §4).
- A node still marked `LUA_PREDICATE` or `LUA_ACTION` stays held (D9).

### 5.2 Outcomes

- A node's `effect` gains a closed set of outcomes: at most one `transition {key}` and at most one
  of `claim {reward_claim}` or `exchange {exchange_key}` (`QUESTGATE0-RL-02`). Dialogue requests
  them; it never writes progress or commits value (NPC-0).
- **Occurrence.** The cause is the occurrence of the `NPC_TALK_INTENT` that matched the node
  (NPC-0 §5.1). A node that asks a question binds a confirmation to (NPC, node, outcome keys,
  content revision), as travel does (NPC-0 §4). The confirming `yes` within `NPC0-RL-06` is the
  cause; any other text or the timeout cancels.
- An outcome is the conversation's one pending service invocation (NPC-0 §4.1).
- **Reply.** The success or refusal reply follows the committed outcome. An ambiguous outcome
  sends no reply line and resolves by occurrence replay (FND-02 §13.3); a reconnect closes the
  conversation (NPC-0 §4).

### 5.3 Transition only

`request_transition` with the talk occurrence as cause. A refusal (`STAGE_MISMATCH` and the other
QUEST-STATE-0 §4 codes) selects the node's refusal reply and writes nothing.

### 5.4 Items: claim and exchange

- **Claim.** A node that gives items names a `RewardClaim` (per character, `once` or cooldown). The
  mint runs on the CHEST-1 path with the dialogue occurrence as a new D40 MINT source cause beside
  the `USE` child (amends DUR-03 §39.3). No room refuses and writes nothing; the NPC says why (D41,
  no ground drop). A node with a transition writes it as the claim's quest obligation
  (QUEST-STATE-0 §5.4), so items come first and the step follows.
- **Exchange** (the NPC takes items). One item-only DUR-03 transaction under the closed cause
  `QuestExchangeCause {npc, node, exchange_key, occurrence}`:
  - BURN lines (§17) of the declared items and counts from direct entries of the main backpack,
    at most `QUESTGATE0-RL-04` (8), a stack in part or whole (§11.1, §11.5);
  - when the node also rewards, the claim's MINT lines and its `RewardClaim` row (D42: inserted,
    or updated for a cooldown claim, on `(character_id, claim_key)`), so the claim, its items and
    the burn commit together or not at all; a claim that is already taken (`once`) or not yet
    allowed (cooldown) refuses the whole exchange;
  - the quest obligation row for its transition.
  All checks precede every write in the one transaction. Under the `character_root` lock it
  validates, without writing the tracks, every QUEST-STATE-0 §4 condition of the transition on the
  locked values: each `from` (`STAGE_MISMATCH`), the quest state's definition hash
  (`REVISION_MISMATCH`), every result within the track's bounds (`OUT_OF_RANGE`) and every effect
  kind supported (`NOT_SUPPORTED`); then the claim and the declared items. Any refusal writes
  nothing: no BURN, no MINT, no claim, no obligation, and the inventory is untouched. The runtime
  holds the sequencer slot from the exchange through its transition, so nothing interleaves and
  the step commits as validated; a refusal of the obligation's transition is then a defect,
  reported, and the row follows QUEST-STATE-0 §5.4 (`REFUSED`, or `WAITING_MIGRATION` on
  `REVISION_MISMATCH`). A crash leaves the obligation, requested again at admission.
- **Gold hand-ins** (a quest that asks for coins) are a `FeeBurnCause` and wait for owner Q1.
- Composition rule 1 covers the exchange and its obligation row (amended in this PR).

### 5.5 Experience (QUEST-XP-1)

- A transition may declare `experience: n` (content, checked `i64` with
  `1 <= n <= QUESTGATE0-RL-05`). Content validation refuses zero, a negative value or a value over
  the bound at compile time; a quest with such a transition is not admitted.
- The committing transition writes `game_character_quest_xp_obligations` (character, receipt key,
  amount, UUIDv7 occurrence, the quest's pinned content revision as provenance) in its own
  transaction.
- The runtime then submits the XP award in the same sequencer slot with that occurrence and the
  active progression policy, its `policy_revision` and its `reward_revision`
  (`request.reward_revision == request.policy.reward_revision`,
  `character_progression.rs:763-774`). The quest content revision stays in the obligation as
  provenance and is never passed as `reward_revision`. The XP writer deletes the obligation in
  its transaction; a guard allows the delete only with the XP receipt naming it. The award keeps
  its own receipt and revision advance (rule 6 unchanged).
- Pending XP obligations are requested again at admission, like quest obligations. An XP refusal
  keeps the row, and a defect is reported (a revision mismatch fails closed, QUEST-STATE-0 §5.2).
- Experience never comes from dialogue, a claim or an interaction directly.

### 5.6 Other rewards

An achievement with a claim uses the reward-claim achievement grant. Outfits, addons, mounts and
blessings wait for their owners' decisions; a node that needs one stays held.

## 6. Serialization

- Gate checks and conditions are reads of the session copy in the owner lane.
- Every durable step is one transaction: a transition, a claim, an exchange or an XP award. A
  chain (claim or exchange, transition, XP) holds one sequencer slot (QUEST-STATE-0 §5.2).
- Lock orders are the owners': the item writer's for claims and exchanges (composition rules 2-4),
  QUEST-STATE-0 §5.3 for transitions, the XP writer's for XP.

## 7. Quest log (QUEST-LOG-WIRE-1)

| Kind | Name | Content |
|---|---|---|
| capability | `QUEST_LOG_V1` | gates everything below |
| command type | `QUEST_LOG_QUERY` | oneof `list`, `quest {quest_index}`, `track {mission_indexes}`; an empty oneof is `REJECTED` |
| state domain | `QUEST_LOG` | the last requested list or quest line, and the tracked missions |

- **Computed from the session copy**, never stored:
  - a storyline quest shown in the log is listed once its `start` track reaches `at_least`;
    `reward_only` and `script_only` quests are never listed (manual §5.5.3);
  - a quest is completed when its state has `completed_receipt` (QUEST-STATE-0 §3); a quest
    flagged `hide_when_completed` (outfit and addon quests) then leaves the list;
  - a mission is shown while its track lies in `[start_value, end_value]`, and done at
    `end_value`;
  - its text is the journal entry for the current value (`per_stage`, `fixed` or `template`, with
    the template's tracks filled in), from the active content revision (text is outside the hash,
    QUEST-STATE-0 §6).
- Quests and missions are canonical uint32 indexes per content generation (SPELL-D1 pattern).
  Nothing else leaves the server: track keys, values other than template counters, transitions,
  gates and claims.
- **Tracker.** At most `QUESTGATE0-RL-06` tracked missions; `track` replaces the set. After a
  committed receipt changes a listed quest, the open quest line or a tracked mission, the domain
  sends a delta.
- Search, sort, "Show completed" and "Show hidden" are client features (assumption A1).
- **Revisions.** The domain is owned by the channel runtime, monotonic per `GameSessionId`; every
  admission, reconnect and transfer sends a new snapshot (NPC-0 §4 pattern).
- A client without the capability never receives the domain; its command is refused as
  unsupported. Queries are rate-limited by `QUESTGATE0-RL-09`.
- Journal text on the wire waits for owner Q2; until then a mission shows its name only.

## 8. Content lanes

- **QUEST-CONTENT-1** (QUEST-STATE-0): the Quest and Interaction families, tracks and transitions.
- **QUEST-CONTENT-2** (content lane): the 184 quest and 14 level gates, gated teleports and
  tiles, levers and step triggers from the door and interaction samples, with placements bound by
  MAP-BUNDLE-1; transition `experience` values; `hide_when_completed`.
- **NPC-QUEST-CONTENT-1** (NPC content lane): binds `requested_by` (format §3.2) to Dialogue nodes
  as typed conditions and outcomes; declares claims and exchanges per node. A conflict between
  Canary and CrystalServer is decided by D10 transcripts, else held.
- **Order:** the readiness order (format §6.7): gated doors and NPC steps of the quests that
  complete first.

## 9. Rows (registered by the children before implementation)

| Row | Value |
|---|---|
| `QUESTGATE0-RL-01` predicates per gate, trigger condition or dialogue node | 4 |
| `QUESTGATE0-RL-02` outcomes per dialogue node | 1 transition and 1 claim or exchange |
| `QUESTGATE0-RL-03` trigger firings per character per second | 10 |
| `QUESTGATE0-RL-04` burn lines per exchange | 8 |
| `QUESTGATE0-RL-05` experience per transition | 1 to 100,000,000 |
| `QUESTGATE0-RL-06` tracked missions | 10 (`PARITY_PENDING`) |
| `QUESTGATE0-RL-07` quests per list projection | 1,024 (as `QUESTSTATE0-RL-05`) |
| `QUESTGATE0-RL-08` missions per quest line and payload bytes | measured by QUEST-LOG-WIRE-1 over the catalogue |
| `QUESTGATE0-RL-09` quest log queries per second | 2 |
| `QUESTGATE0-RL-10` pending XP obligations per character | 16 |
| `DUR03-RL-01-QUEST-EXCHANGE` touched items | 8 burned plus the claim's minted items; measured |
| Gate check | 0 rows, 0 revisions |
| Exchange | 1 transaction, 1 cause record, 1 obligation, 1 event, 0 revisions |

## 10. Rejected options

- **Recording a gate pass.** A pass is a read; Tibia keeps no pass state, and D38 keeps door
  state ephemeral.
- **Checking gates only at USE.** An open door would let anyone follow.
- **Lua predicates and actions in dialogue.** The boundary forbids an NPC script engine.
- **Dialogue writing tracks or granting items.** D35 and NPC-0 forbid it.
- **Transition first, then items.** A full backpack would leave the step taken and no reward.
- **XP inside the transition's transaction.** The chain guard admits one receipt per revision.
- **A stored quest log.** Everything shown is a function of tracks and content.
- **Sending track keys and values.** They are spoilers and internal identities.

## 11. Owner-rule applications

- **Global parity** (5905825574, direction 2026-09-30): both door edges, push-back, NPC hand-ins,
  the quest log with tracker, and journal text if Q2 allows it.
- **D35:** only the quest domain writes progress. **D38 W2:** door state is ephemeral.
- **NPC-0:** dialogue commits no value. **D41:** no ground drop for rewards.
- **D178:** gold taken by a quest is a new fee source (Q1).

## 12. Owner questions and assumptions

**Q1. Admit gold hand-ins to quest NPCs as a fee source?** D178 needs an owner decision for each
new fee source. a) Yes, coins as in Tibia, from the main backpack (recommended); b) coins, then
the bank as BANK-FEE-0 does for NPC fees; c) no, hold those quest steps.

**Q2. Quest journal text on the wire?** The format keeps text references only (principle 5);
`LICENSE-ASSETS.md` allows quest text as reference data, and D9 admitted NPC text 1:1. a) Admit
Tibia Global quest-log text 1:1 as reference data with provenance, as D9 (recommended); b)
Oteryn-written text; c) mission names only.

Assumptions, reversible, taken to proceed: **A1** "Show hidden" and search are client-local in V1.
**A2** the tracker set is runtime-local and empty at admission. **A3** a gated door closes when
its last creature leaves, as in the reference servers.

## 13. Decision test

- **Must decide now:** YES. Doors and NPC steps are the next 27 quests (format §6.7), and the
  owner asked to build quests now.
- **Minimum sufficient:** one gate check at two edges, three trigger edges on accepted identities,
  typed dialogue conditions and outcomes, one exchange shape, one XP obligation, one wire view.
- **Superseding evidence:** official door or quest log behaviour; owner answers.
- **Deliberately not decided:** key doors, outfit and mount grants, boss rooms, cross-scope
  relocation, party quests, world quests.

## 14. Before-freeze checklist

1. **Contract amendments:** QUEST-STATE-0, NPC-0 §3.4 and §11, the quest format §3.1, the
   successor header, the relocation proposal header, DUR-03 §15 and §39.3 and the composition
   decision carry pointers "pending on acceptance of QUEST-GATE-0".
2. **Serialization:** §6; one sequencer slot per chain.
3. **Restart:** door state is lost with the scope; claims, exchanges, obligations and receipts are
   durable; obligations are requested again at admission.
4. **Typed references:** `placement_key`, `map_item_handle`, gate, transition, claim and exchange
   keys, talk occurrence, CommandRef, content revision.
5. **Wire:** §7, capability `QUEST_LOG_V1`; gates add no wire.
6. **Split work:** one transaction per step; at most 8 burn lines per exchange.
