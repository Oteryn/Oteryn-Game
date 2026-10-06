# QUEST-GATE-0 Quest gates, triggers, NPC quest dialogue and the quest log

- Decision: `QUEST-GATE0-WORLD-GATES-AND-NPC-QUESTS-V1` (covers QUEST-GATE-0 and NPC-QUEST-0)
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (protocol,
  persistence and security) and protected integration. Owner questions Q1 and Q2 (§12) are
  answered (2026-09-30, #162): gold hand-ins take coins, then the bank (§5.4); journal text is
  Tibia text 1:1 (§7).
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Amended (2026-10-03): §15, reconciled with `main` for acceptance (control plane D352).
- Amended (2026-10-06): §16, the durable cause of a trigger's quest child, plan loss and push
  roots, which request no quest and no `RewardClaim` child (architect ruling on #1622 6021612356); §4 "Accepted successor sections", "Roots" and
  "Quest child" amended in place; WORLD-INTERACTION-0 §7.1 and its PUSH-1 brief row amended in
  place (§16.3).
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
  (`character_progression.rs`); DUR-03 §15, §17, §39.3; D178; the gold fee decision (§4.2-§4.5 as
  amended); BANK-0 (§4.1, §4.4) and BANK-FEE-0 (§3, §4); owner rule 5905825574
- Amends, each pending on acceptance of QUEST-GATE-0, in this PR: QUEST-STATE-0 (callers and A8
  pointer); NPC-0 §3.4 and §11; the quest format §3.1; the GAME-INTERACTION-01 successor header
  (accepted sections for §4 edges); the relocation and world object owners proposal header (§3 and
  §4 for gates and triggers); WORLD-INTERACTION-0 §3 (durable trigger children follow §16.2.5,
  not occurrence recovery); DUR-03 §15 and §39.3 (`QuestExchangeCause`, the dialogue claim
  source, and the reward MINT retirement of §16.2.5); D39 §4.1 (the accepted successor §17 and
  §19.1 for a reward MINT reserved under a lost trigger plan: its reconciliation is the §16.2.5
  retirement); the composition decision (exchange obligation).
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| QUEST-GATE-1 | hard, security review | `Gate` lowering bound to door `placement_key`s; gate checks at USE and at step-in in the channel runtime; open, pass-through, push-back and close (§3) | QUEST-PRED-1; MAP-LOAD-1; MAP-WIRE-2; QUEST-CONTENT-2 |
| QUEST-TRIGGER-1 | hard, persistence review | `USE`, `ON_ENTER` and `ON_LEAVE` interaction triggers on placed objects and tiles, their occurrence roots, and quest, relocation and overlay children (§4); the root CommandRef as quest cause, its content validation and plan check (§16) | QUEST-STATE-1; QUEST-GATE-1 |
| NPC-QUEST-1 | hard, persistence review | typed quest conditions and outcomes in the NPC talk runtime; confirmation binding; the dialogue claim; the exchange transaction with its gold hand-in, coins then bank (§5) | NPC-TALK-1; QUEST-STATE-1; QUEST-PRED-1; CHEST-1 (merged); GOLD-FEE-2 (the bank part) |
| QUEST-XP-1 | hard, persistence review | the quest XP obligation and its XP writer path (§5.5) | QUEST-STATE-1; CHAR-REV-SEQ-1 |
| QUEST-LOG-WIRE-1 | impl, protocol review | capability `QUEST_LOG_V1`, command and domain; list, quest line and tracker views (§7) | QUEST-PRED-1; QUEST-LOWER-1 |
| QUEST-CONTENT-2 | content lane | gates, gated teleports and tiles, levers and step triggers as data, bound to placements; journal text 1:1 (§8) | QUEST-LOWER-1; MAP-BUNDLE-1 |
| NPC-QUEST-CONTENT-1 | NPC content lane | typed quest conditions and outcomes on Dialogue nodes from `requested_by` (§8) | NPC-CONTENT-1; QUEST-LOWER-1 |

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

**Amendment (pending on acceptance of WORLD-INTERACTION-0;
`OTERYN_GAME_WORLD_INTERACTION0_DOORS_LEVERS_FIELDS_AND_WORLD_CLOCK_DECISION_2026-10-01.md` §4-§5).**
`door_key` gates lower to key doors (DOOR-1, KEY-1: a key's immutable `key_number` against the
binding; the lock is per channel overlay; 101 and 1001 never unlock). The lever `shared_lock` gate
is a door whose state a lever's `TRANSFORM` children set (LEVER-1). The predicate set gains
`premium` (the PREMIUM-ACTIVATION §4.5 surface) and `vocation_in {set}`, read-only like the others.

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
  firing, `after_quest` children included, is part of the root's plan: while the plan is
  retained, recovery reproduces the same child set and order and runs each `UNSTARTED` child once
  if its fences still authorize it (§7), else it is `REJECTED`. In v1 the plan is retained in
  the server process only; when it is lost, recovery takes successor §6.2's fail-closed branch
  (explicit reconciliation, §16.2.5). Nested cascades and every other child kind stay
  `PROPOSED / NONCANONICAL`.
- **Roots.** `USE`: the `USE_INTENT` CommandRef. `ON_ENTER` and `ON_LEAVE`: the occurrence that
  moved the character, which is its own move command (in v1 `WORLD_ACTOR_STEP_INTENT`, §16.2)
  or another player's push command (a push root requests no quest and no `RewardClaim` child,
  §16.3). A move
  with no such root fires nothing (successor §18: no ad hoc identity): an admission placement and,
  by architect ruling (fail closed), the landing of a D37 relocation child. A relocation child
  is never a trigger root, so there is no relocation-to-trigger cascade and nested cascades
  stay `PROPOSED / NONCANONICAL`; content validation still checks the target tile, and a
  destination that carries `ON_ENTER` is a declared v1 difference (the trigger stays silent on a
  relocation landing).
- **Quest child.** `request_transition(fence, character, transition_key, cause)` with the root's
  CommandRef as cause, the durable index of the child occurrence (§16; QUEST-STATE-0 §4). Several quest children request their transitions in the
  successor's canonical child order (§6.1) within one sequencer slot.
- **Dependent children.** A relocation, overlay or presentation child declared `after_quest` runs
  only on the quest child's `COMMITTED`, as a new request of the scope runtime; if its fences
  moved meanwhile it is `REJECTED_STALE` (D37 R2) and nothing moves. Other children run in the
  trigger's tick and do not wait.
- **Items.** An interaction hands out items only through a `RewardClaim` (the CHEST-1 path, D40)
  and takes carried items only through the §5.4 exchange. It never mints or burns by itself.
- **Bounds.** Trigger firings per character are limited by `QUESTGATE0-RL-03`. The limit is
  checked before the root commits, never after: a `USE`, move or push whose firing would
  exceed it is refused before commit (architect ruling, fail closed). The `USE` changes nothing
  and the move or push is refused, the character staying on its tile; no child runs and no
  firing is dropped. A firing is never
  discarded after its root has committed.

## 5. NPC quest dialogue (NPC-QUEST-1)

### 5.1 Conditions

- A Dialogue node's `gate` gains a typed `quest` condition: a conjunction of at most
  `QUESTGATE0-RL-01` QUEST-STATE-0 §7 predicates over the session's copy, plus the level and
  `holds_item` owners. The first matching sibling whose condition passes answers (schema §4).
- **Revalidation.** A confirmation-bearing node's predicates are evaluated again under
  `character_root` inside the claim, exchange or transition transaction, before any write, like
  quest-gated travel: the node's quest, level and `holds_item` conditions must still pass on the
  locked state. A stale confirmation whose node is no longer eligible is refused as the node's
  refusal reply and writes nothing; a confirmation is never authority by itself.
- A node still marked `LUA_PREDICATE` or `LUA_ACTION` stays held (D9).

### 5.2 Outcomes

- A node's `effect` gains a closed set of outcomes: at most one `transition {key}` and at most one
  of `claim {reward_claim}` or `exchange {exchange_key}` (`QUESTGATE0-RL-02`). A node with an
  `exchange` must also declare a `transition` (content validation refuses it otherwise; architect
  ruling, fail closed): the exchange's quest obligation exists for that transition, so there is
  never an exchange without one. A `claim` may stand without a transition. Dialogue requests
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
  (QUEST-STATE-0 §5.4), so items come first and the step follows; a claim-only node (no
  transition) writes no obligation row. Its reservation has the `Other` origin and no
  §16.2.5 trigger-child row, so it is never retired: an ambiguous dialogue MINT resolves by
  occurrence replay.
- **Exchange** (the NPC takes items). One item-only DUR-03 transaction under the closed cause
  `QuestExchangeCause {npc, node, exchange_key, occurrence}`:
  - BURN lines (§17) of the declared items and counts from direct entries of the main backpack,
    at most `QUESTGATE0-RL-04` (8), a stack in part or whole (§11.1, §11.5);
  - when the node also rewards, the claim's MINT lines and its `RewardClaim` row (D42: inserted,
    or updated for a cooldown claim, on `(character_id, claim_key)`), so the claim, its items and
    the burn commit together or not at all; a claim that is already taken (`once`) or not yet
    allowed (cooldown) refuses the whole exchange;
  - the quest obligation row for its transition.
  The preflight also counts the character's locked pending quest obligations: at
  `QUESTSTATE0-RL-07` (64) the whole exchange is refused `OBLIGATIONS_FULL` (QUEST-STATE-0 §5.4)
  before any BURN, MINT, claim or obligation write, and the node selects its refusal reply.
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
- **Gold hand-ins** (a quest that asks for coins; owner Q1b, D178). An exchange may declare
  `gold: n` (content, checked `u64`, `n >= 1`); a declared item may not be a coin key (content
  validation refuses it). The coins are a fee of the exchange, in the same transaction and under
  its one cause record: the `FeeBurnCause` variant `QuestExchange(QuestExchangeCause)`, an
  item-only fee source as NPC BUY is (gold fee §4.3 and §4.4 as amended). The plan is the gold
  fee §4.2 plan as BANK-FEE-0 §3 amends it:
  - coins from direct entries of the main backpack first; when they cover `n`, the bank is not
    touched and change is at most two stacks (`ChangeDoesNotFit` refuses);
  - when they are worth less than `n`, every eligible coin is burned whole, no change is minted,
    and the rest is debited from the payer's (Account, World) balance on the BANK-0 path of
    BANK-FEE-0 §4: one `FEE_DEBIT` ledger entry referencing the fee record, the balance row
    locked after the coin entries (BANK-0 §4.1), and its value line on the transaction's one
    event (BANK-FEE-0 §4.3; no second event);
  - a balance below the rest refuses the whole exchange as `InsufficientFunds`, and a junior payer
    (BANK-0 §4.4) whose coins are short is refused. Either refusal writes nothing, like every
    other exchange check, which all precede every write.
  Coin lines are not declared items and do not count toward `QUESTGATE0-RL-04`; the gold fee plan
  bounds them. The claim's items and the change must both fit after the burn lines. NPC-QUEST-1
  widens `0023` for the source kind, as NPC-TRADE-1 does.
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
- **Capacity.** The transition transaction counts the character's locked pending XP obligations
  before any write; at `QUESTGATE0-RL-10` (16) an XP-bearing transition is refused whole with
  `OUT_OF_RANGE` (QUEST-STATE-0 §4), before quest state changes or a seventeenth row is inserted,
  and the dialogue node selects its refusal reply.
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
| command type | `QUEST_LOG_QUERY` | oneof `list`, `quest {quest_index}`, `track {quest_indexes}`; an empty oneof is `REJECTED` |
| state domain | `QUEST_LOG` | the last requested list or quest line, and the tracked quests |

- **Computed from the session copy**, never stored:
  - a storyline quest shown in the log is listed once its `start` track reaches `at_least`;
    `reward_only` and `script_only` quests are never listed (manual §5.5.3);
  - a quest is completed when its state has `completed_receipt` (QUEST-STATE-0 §3); a quest
    flagged `hide_when_completed` (outfit and addon quests) then leaves the list;
  - a mission is shown while its track lies in `[start_value, end_value]`, and done at
    `end_value`;
  - its text is the journal entry for the current value (`per_stage`, `fixed` or `template`, with
    the template's tracks filled in). An in-progress quest (a state row with no
    `completed_receipt`) is resolved against its pinned content revision (QUEST-STATE-0 §6, account
    decision §4.1 P2): its mission ranges, template reads and journal entries come from that
    revision, never from a newer active one, until an explicit DUR-04 migration repins it. A quest
    with no state row, and a completed one, use the active revision (text is outside the hash,
    QUEST-STATE-0 §6). The session copy carries each active quest's pin; a pinned revision that
    is no longer loadable hides the quest from the projection (fail closed) rather than
    reinterpreting its tracks.
- Quests and missions are canonical uint32 indexes per content generation (SPELL-D1 pattern).
  Nothing else leaves the server: track keys, values other than template counters, transitions,
  gates and claims.
- **Tracker.** At most `QUESTGATE0-RL-06` tracked quests (the "Show in quest tracker" choice is
  per quest line); `track` replaces the set, and an unlisted or unknown quest index is `REJECTED`.
  The missions shown are derived from the session copy: the currently visible missions of each
  tracked quest, so the tracker follows the quest when a mission completes and the next starts.
  After a committed receipt changes a listed quest, the open quest line or a tracked quest, the
  domain sends a delta.
- Search, sort, "Show completed" and "Show hidden" are client features (assumption A1).
- **Revisions.** The domain is owned by the channel runtime, monotonic per `GameSessionId`; every
  admission, reconnect and transfer sends a new snapshot (NPC-0 §4 pattern).
- A client without the capability never receives the domain; its command is refused as
  unsupported. Queries are rate-limited by `QUESTGATE0-RL-09`.
- **Journal text** (owner Q2a) is Tibia Global quest-log text 1:1, reference data with
  provenance, as D9 admitted NPC text.

## 8. Content lanes

- **QUEST-CONTENT-1** (QUEST-STATE-0): the Quest and Interaction families, tracks and transitions.
- **QUEST-CONTENT-2** (content lane): the 184 quest and 14 level gates, gated teleports and
  tiles, levers and step triggers from the door and interaction samples, with placements bound by
  MAP-BUNDLE-1; transition `experience` values; `hide_when_completed`; the Tibia Global journal
  text 1:1 with provenance (Q2a).
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
| `QUESTGATE0-RL-06` tracked quests | 10 (`PARITY_PENDING`) |
| `QUESTGATE0-RL-07` quests per list projection | 1,024 (as `QUESTSTATE0-RL-05`) |
| `QUESTGATE0-RL-08` missions per quest line and payload bytes | measured by QUEST-LOG-WIRE-1 over the catalogue |
| `QUESTGATE0-RL-09` quest log queries per second | 2 |
| `QUESTGATE0-RL-10` pending XP obligations per character | 16 |
| `DUR03-RL-01-QUEST-EXCHANGE` touched items | at most 20 burned (8 declared plus coins, all direct entries of the 20-entry main backpack), at most 2 change stacks, plus the claim's minted items; measured |
| `DUR03-RL-03-FEE` value lines per exchange | 1 when the bank pays part of a gold hand-in, else 0 |
| Gate check | 0 rows, 0 revisions |
| Exchange | 1 transaction, 1 cause record, 1 obligation, 1 event, 0 revisions; with gold, 1 fee record and at most 1 `FEE_DEBIT` ledger entry |

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
  the quest log with tracker, and journal text 1:1 (Q2a).
- **D35:** only the quest domain writes progress. **D38 W2:** door state is ephemeral.
- **NPC-0:** dialogue commits no value. **D41:** no ground drop for rewards.
- **D178:** gold taken by a quest is a new fee source, admitted by the owner (Q1b): coins first,
  then the bank (BANK-FEE-0).

## 12. Owner questions and assumptions

**Q1. Admit gold hand-ins to quest NPCs as a fee source?** D178 needs an owner decision for each
new fee source. a) Yes, coins as in Tibia, from the main backpack (recommended); b) coins, then
the bank as BANK-FEE-0 does for NPC fees; c) no, hold those quest steps.
Owner answer (2026-09-30, #162): b — the NPC takes backpack coins first and the missing part from
the bank balance, through the BANK-0 path in the same transaction, with the existing bank evidence
and the refusal when funds are insufficient (§5.4).

**Q2. Quest journal text on the wire?** The format keeps text references only (principle 5);
`LICENSE-ASSETS.md` allows quest text as reference data, and D9 admitted NPC text 1:1. a) Admit
Tibia Global quest-log text 1:1 as reference data with provenance, as D9 (recommended); b)
Oteryn-written text; c) mission names only.
Owner answer (2026-09-30, #162): a — Tibia text 1:1 (§7).

Assumptions, reversible, taken to proceed: **A1** "Show hidden" and search are client-local in V1.
**A2** the tracker set is runtime-local and empty at admission. **A3** a gated door closes when
its last creature leaves, as in the reference servers.

## 13. Decision test

- **Must decide now:** YES. Doors and NPC steps are the next 27 quests (format §6.7), and the
  owner asked to build quests now.
- **Minimum sufficient:** one gate check at two edges, three trigger edges on accepted identities,
  typed dialogue conditions and outcomes, one exchange shape, one XP obligation, one wire view.
- **Superseding evidence:** official door or quest log behaviour; a new owner decision.
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
6. **Split work:** one transaction per step; at most 8 declared burn lines per exchange, plus the
   gold hand-in's coin lines and at most one `FEE_DEBIT` entry.

## 15. Reconciliation with `main` (2026-10-03)

- **Content.** The Quest family is on `main` (#1596, QUEST-STATE-0 §13.2), but its tracks and
  transitions are not lowered. Gates, triggers, dialogue conditions and the quest log read only
  lowered Oteryn keys, so their content children wait on QUEST-LOWER-1 (brief). The definitions'
  `source_data.gates` and `source_data.interactions` (source keys with placements) are
  QUEST-CONTENT-2's input; their source keys stay source bindings and never become gate or
  transition identities.
- **`account_completed`** is false until QUEST-ACCOUNT-1 (QUEST-STATE-0 §13.3), so a gate or node
  that reads it stays sealed; content validation reports such a gate as held.
- **XP chain.** QUEST-XP-1's XP award takes its own receipt in the eight-kind guard (QUEST-STATE-0
  §13.1) through the CHAR-REV-SEQ-1 sequencer; nothing here changes.
- **Wire numbers.** `QUEST_LOG_V1`, `QUEST_LOG_QUERY` and `QUEST_LOG` are leased by the control
  plane when QUEST-LOG-WIRE-1 is allocated, from the then-free capability, command type and state
  domain; this decision reserves none.
- **The 68 authored quests** (`oteryn_authored_v1`, `runtime_enabled: false`) adapt Tibia quests
  and are outside this decision: no gate, trigger, dialogue or log view uses them until an
  accepted decision admits them (owner rule: Tibia fidelity).
- §1-§14 stand otherwise; the amendment pointers of the header remain pending on acceptance.

## 16. The quest child's durable cause (2026-10-06)

Answers the QUEST-TRIGGER-1 allocation blocker (#1622 6021612356). Architect ruling: **option A**,
as the representation choice that successor §5.9 leaves open (a compact index MAY stand for the
child tuple while the tuple stays authoritative and collisions are handled safely). Option B (a
new cause kind with a receipt key migration) is rejected for v1 (§16.6).

### 16.1 Facts

**PROVEN**
- `QuestCause` (`apps/game-server/src/durability/quest_state.rs`) has four kinds: `Command`,
  `Use` and `ClaimObligation`, each a CommandRef (GameSessionId, CommandId), and
  `CreatureDeath`. Migration `0056` checks that set. The receipt key is (character_id, cause_id,
  cause_ordinal, transition_key), without `cause_kind`.
- The binding covers the request only: character, transition key and cause, kind included.
  - The same key with an equal binding replays the first outcome.
  - The same key under another kind conflicts. An obligation that conflicts stays pending and is
    retried.
- A `Command` or `Use` cause must belong to the fenced GameSession, otherwise the request is
  refused (`AuthorityRejected`). This is checked before the receipt lookup.
  - A same-GameSession reconnect commits a pending cause.
  - An obligation may come from an earlier session (QUEST-STATE-0 §5.3).
  - `reconcile_character_quest_transition` reads a receipt without the session check.
- A refusal by validation writes no receipt.
- A D39 chest is a `USE` definition. Its claim and its quest obligation are keyed by the
  `USE_INTENT` CommandRef (`chest_use.rs`).
- The successor's child reference is the tuple (parent, definition, target, edge, optional
  ordinal, semantic revisions) (§5.1). Its representation is not frozen (§5.9).
- `REJECTED` is terminal, and replay does not evaluate it again as fresh work (successor §7, §9.3).
- One `WORLD_ACTOR_STEP_INTENT` moves to one destination cell
  (`apps/game-server/src/movement/source_floor_change.rs`). The floor-change cells it resolves
  through are not entered.
- The protocol registry has no push command and no multi-cell move command.

**DERIVED**
- In v1, trigger children are first-level only (§4; nested cascades are `NONCANONICAL`).
- The plan is the set of children whose conditions held at the root, in canonical order (§4;
  successor §6.1). Without retained evidence of that set, recovery fails closed (successor §6.2).
- Within one root, a quest child's tuple is therefore fixed by (character, transition_key)
  exactly when no other quest child and no claim obligation of that root shares it.

### 16.2 Ruling

1. **Cause.**
   - A quest child of a `USE` root requests its transition with `QuestCause::Use`, carrying the
     root `USE_INTENT` CommandRef.
   - A quest child of an `ON_ENTER` or `ON_LEAVE` root requests it with `QuestCause::Command`,
     carrying the CommandRef of the step.
   - In v1, only `WORLD_ACTOR_STEP_INTENT` roots `ON_ENTER` and `ON_LEAVE`. These fire nothing,
     and this is a declared v1 difference: a relocation by `USE` (rope, ladder), a spell move and
     an NPC travel.
   - The receipt key (character, root CommandRef, transition_key) is the child's durable index.
   - The full child reference stays Interaction-owned. Quest does not persist it.
   - There is no new cause kind and no cause or receipt key migration (the §16.2.5 retirement
     and trigger-child tables are separate claim records). A child reference is never hashed into a UUID,
     and `CommandId` or `cause_ordinal` are never overloaded.
2. **Invariant.**
   - The quest children and claim obligations under one root CommandRef have pairwise distinct
     transition keys. A quest child names only the root's acting character (§16.3), so the
     invariant is checked per root.
   - One root CommandRef has at most one `RewardClaim` child, the D39 chest claim included. Its
     MINT is keyed by the root CommandRef, and the DUR-03 reservation admits one logical MINT per
     CommandRef (`0012_reward_claim_backpack_mint.sql`), so a second claim would conflict with
     the first after that one may have committed. A claim-only child has no transition key, so
     the first rule alone does not catch it.
3. **Content validation.** QUEST-CONTENT-2 lowering enforces these rules; a violation fails the
   build. A binding is a definition at one placement.
   - (a) **USE.** Across every binding one `USE_INTENT` can fire, the transition keys of quest
     children and `RewardClaim` obligations are pairwise distinct. That covers the used
     placement, the used item, the `use_with` target and a D39 chest claim on the same placement.
     The same bindings hold at most one `RewardClaim` child in total, that chest claim included.
   - (b) **Step.** Among the `ON_ENTER` bindings on one cell (the tile and the objects placed on
     it), the transition keys of quest children and claim obligations are pairwise distinct. The
     same holds for `ON_LEAVE`. The `ON_ENTER` bindings on one cell hold at most one
     `RewardClaim` child.
   - (c) **Leave and enter.** No transition key is a quest child or claim obligation of both an
     `ON_ENTER` and an `ON_LEAVE` binding, because one step leaves one cell and enters another.
   - (d) **Acting character only.** A quest child names only the root's acting character.
   - (f) **No claim on leave.** In v1 an `ON_LEAVE` binding names no `RewardClaim` child. A step
     then has claims only from the cell it enters, and (b) bounds those to one. This is a declared
     v1 limit.
   - (e) **Placement.**
     - A placed object that carries a trigger is not movable by `ITEM_MOVE_INTENT`.
     - A floor-change cell carries no `ON_ENTER`, because a step never enters it.
     - Objects created by an overlay have no placement and carry no trigger.
4. **Plan check (defence in depth).** The trigger runtime builds the plan before the root commits,
   where `QUESTGATE0-RL-03` is checked. If a plan holds two quest children or claim obligations
   with an equal (character, transition_key), or more than one `RewardClaim` child:
   - the root is refused before commit, as under RL-03. The `USE` changes nothing and the step is
     refused, with the existing `USE_INTENT` and step refusal results.
   - a content defect is recorded;
   - equal children are never collapsed into one.
5. **Recovery.** A quest child's receipt is its `COMMITTED` record.
   - A child whose request returned an outcome is settled: a receipt means `COMMITTED`; a refusal
     or any error other than an unknown commit outcome means `REJECTED`. A settled child is never
     requested again.
   - A child whose attempt has an unknown outcome is `PENDING`. It is resolved by looking up its
     receipt by (character, root CommandRef, transition_key) with
     `reconcile_character_quest_transition`. Only with no receipt, and in the same GameSession,
     is the same request sent again; it replays or commits once.
     In a replaced GameSession, a pending child with no receipt is `REJECTED`.
   - A child that is `UNSTARTED` in a retained plan runs once if its fences authorize it (§4),
     else it is `REJECTED`.
   - **Plan loss.** The plan lives in the server process (§16.4: durable plans are not decided).
     If the process is lost after the root commits, the plan cannot be reconstructed safely:
     its conditions read quest state that its own committed children may have changed, so
     re-evaluating them is the re-enumeration successor §6.2 forbids. Recovery takes §6.2's
     fail-closed branch, an explicit reconciliation of the root. It applies to a root that is
     discoverable through a durable child record keyed by its CommandRef (a quest receipt, a
     claim's MINT outcome or pending reservation, or a claim's obligation row). A root with no
     such record left no durable quest or claim effect: nothing reconciles or records it, and
     none of its children runs again. Recovery never needs to find it, and the ruling still adds
     no root durability. For a discoverable root:
     - a child with a durable record keyed by the root CommandRef is settled by that record and
       never rejected:
       - a quest child by its receipt (character, root CommandRef, transition_key), read with
         `reconcile_character_quest_transition`; it stays `COMMITTED`;
       - a `RewardClaim` child by its DUR-03 MINT outcome under the root CommandRef
         (`chest_use.rs`): a committed MINT stays `COMMITTED`. A reservation with no receipt
         cannot be replayed after a plan loss: its `RewardClaimMintCandidate` and request are
         process-local, the reservation keeps only the intent binding hash and the frozen
         identities, FND-02 never re-enqueues the reserved CommandRef, and a replacement
         GameSession fails `character_item_fence_is_current`. The reconciliation therefore
         retires it through a new DUR-03 operation in `reward_claim_mint.rs`, with a durable
         terminal record. DUR-03 §39.3 and D39 §4.1 carry this as amendments pending on
         acceptance of QUEST-GATE-0. D39 accepts successor §17 and §19.1: an ambiguous MINT stays
         pending on its DUR-03 transaction, and a new GameSession reconciles the old occurrence
         before a duplicate is allowed. The retirement is that reconciliation, on the same DUR-03
         transaction key (the root CommandRef) under its cause lock, so the D39 amendment changes
         how the occurrence becomes terminal, not when a duplicate may start. Until a receipt or a
         retirement row exists, the existing pending rule applies unchanged. A chest `USE` whose
         trigger plan was not lost keeps D39 as accepted. The rules below are the detail:
         - **Retirement record.** QUEST-TRIGGER-1 adds one migration (its version above the
           highest on `main` at authoring) with an insert-only table
           `game_reward_claim_mint_retirements`, keyed by the reservation's (game_session_id,
           command_id) and referencing it. The runtime role gets `SELECT, INSERT` only; a
           no-truncate trigger is added as for the 0012 tables. The 0012 reservation guard is
           unchanged.
         - **Retirement eligibility.** The 0012 reservation records no source, and the NPC-QUEST-1
           dialogue claim (§5.4) and a D39 chest `USE` claim reserve on the same MINT, so the
           same migration adds an insert-only table `game_reward_claim_mint_trigger_children`,
           keyed by the reservation's (game_session_id, command_id) and referencing it.
           `RewardClaimMintRequest` gains a closed origin: `TriggerPlanChild` for a trigger plan's
           `RewardClaim` child, `Other` for every other MINT. `freeze_reward_claim_mint` inserts
           the row in the transaction that inserts the reservation, and only for
           `TriggerPlanChild`. A guard trigger refuses the insert unless that reservation row was
           inserted by the current transaction, so no later call can make an existing
           reservation retirable. A later pass under the same CommandRef whose origin disagrees
           with the row's presence refuses with `ConflictingCause`. The runtime role gets
           `SELECT, INSERT` only, with a no-truncate trigger.
         - **Retirement.** The operation loads the reservation by the root CommandRef. It
           refuses with `NotRetirable` and writes nothing unless the reservation's
           `character_id` is the reconciled Character and its trigger-child row exists; a
           dialogue or D39 chest `USE` reservation is never retired and keeps its own rule.
           Then, in one transaction under the current recovery fence, it takes the commit pass's
           locks in their order (`lock_admission_relations`, then `lock_cause`, which waits for
           a pass already in flight) and reads the receipt, then the retirement row. With a
           receipt it writes nothing and returns the committed result. With a retirement row it
           writes nothing and returns `Retired`. With neither it inserts the retirement row in
           that transaction and returns `Retired`. It spends no RL-08 work unit, so a pass that
           already charged the final unit changes nothing.
         - **Every commit pass checks it.** `commit_reward_claim_mint_noticed` refuses with
           `CapacityExceeded`, under the cause lock and before any write, when a retirement row
           exists for its CommandRef. `freeze_reward_claim_mint` refuses the same way. The cause
           lock orders each pass against the retirement. A pass that locked first commits
           before the retirement reads, so the receipt settles the child. A pass that locks
           after the retirement sees the row and writes nothing, whatever its own or the stored
           work unit count. This holds when its charge reached the RL-08 maximum.
         - **Reconcile reports it.** `reconcile_reward_claim_mint` returns a closed
           `RewardClaimMintReconciliation` in place of its `Option`: `Committed` with the
           receipt, `Retired`, or `Pending` (no receipt and no retirement row; the old `None`, and
           the same candidate may be retried). It first reads the retirement row and returns
           `Retired` without charging a work unit, since the row is insert-only and final. Then
           it charges, and under the cause lock reads the receipt and, when there is none, the
           retirement row again. `Retired` is terminal: the caller settles the occurrence
           `REJECTED` and never retries that candidate, and a fresh `USE` is a new CommandRef.
         - **Pending.** A retirement row makes the reservation provably terminal (successor
           §17.2), so the MINT's pending check counts it as not pending and a fresh `USE` is not
           refused with `ClaimPending`.
         - **Settlement.** A receipt means `COMMITTED`, and the claim's obligation row exists. A
           retirement row means `REJECTED`: no item, no RewardClaim and no obligation, so the
           claim stays unclaimed and a fresh `USE` (a new CommandRef) can claim it. A rerun of
           the reconciliation finds the receipt or the retirement row and reads the same result.
           The `REJECTED` settlement is written only after the retirement row commits.
     - a committed claim's quest obligation is a durable QUEST-STATE-0 §5.4 row, not a plan
       child: it is requested again at admission and commits once under its `ClaimObligation`
       cause (§16.2.2 keeps its transition key distinct); the reconciliation never rejects or
       retires it;
     - every other child is `REJECTED` and none runs; nothing is re-enumerated, added or
       renumbered;
     - a relocation child that committed before the loss has no cause-keyed record: its
       position is a Character runtime-state projection (D37 "Timeout, cancellation and
       recovery"; CHAR-POSITION-0), which the periodic or final position write may already have
       persisted. Recovery never infers, re-runs or reverts it; the Character resumes at its
       last persisted position, as after any crash, with or without the relocation. This holds
       for a discoverable root and for one that is not. Its `REJECTED` marks only that the
       child never runs again, not that nothing moved;
     - the reconciliation is recorded once per discoverable root (character, root CommandRef,
       the committed transition keys and claim) as an operational diagnostic.
   - What a plan loss leaves behind is bounded: the overlay children's effects are channel
     overlay state, cleared at a channel restart (WORLD-INTERACTION-0 §9.1); a relocation or
     `after_quest` child that did not run moved nothing; a relocation that ran is kept only as
     far as CHAR-POSITION-0 persisted the position (up to 5 minutes of movement is lost by any
     crash); a quest child that committed stays
     committed; a committed claim keeps its items, and its obligation commits at a later
     admission. A fresh `USE` or step is a new CommandRef and a new root (successor §9.3). This
     partial outcome after process loss is a declared v1 behaviour, and the ruling adds no root
     durability.

### 16.3 Push roots

- A push moves the pushed character under the pushing player's command. QUEST-STATE-0 §5.3 refuses
  a cause from another GameSession.
- A `RewardClaim` child is fenced the same way: its MINT admission
  (`durability/reward_claim_mint.rs`, `admit`) refuses a CommandRef that is not the acting
  character's current item fence (`character_item_fence_is_current`, `AuthorityRejected`), and a
  push CommandRef belongs to the pusher. Run under it, the claim is refused or names the wrong
  actor's session.
- In v1, therefore, a push root requests no quest child and no `RewardClaim` child, so it creates
  no claim obligation and no claim reservation, and its children declared `after_quest` do not
  run. Its other relocation, overlay and presentation children run as declared. Content may bind
  a claim only on an `ON_ENTER` cell (§16.2.3(f) rejects one on `ON_LEAVE`); it fires on the
  character's own step into the cell and stays silent on a push.
- QUEST-TRIGGER-1 tests: a push onto a cell whose binding declares a quest child and a
  `RewardClaim` writes no quest receipt, no claim reservation, no MINT and no obligation, and runs
  the cell's other declared children.
- WORLD-INTERACTION-0 §7.1 (the pushed step's `ON_ENTER` and `ON_LEAVE` fire with the push command
  as root) and its PUSH-1 brief row are amended in place to carry this exception, so PUSH-1 and
  QUEST-TRIGGER-1 build to one rule.
- No push command is registered today, so this case is latent.
- This is a declared v1 difference: in Tibia, a pushed player's step-in fires.
- The push command's own decision may admit a cross-character quest or claim cause.

### 16.4 Not decided

Each of these needs a durable child discriminator: option B, a new cause kind with an explicit
encoding and a new receipt key migration. It is decided with the first accepted need.

- Commands that move more than one cell (autowalk, a path).
- Roots that are not commands (timers, world events, creature movement).
- Nested cascades.
- Quest or `RewardClaim` children that act for another character (party, push, a lever that acts
  on others).
- Durable trigger plans.

### 16.5 Decision test

- **Must decide now:** YES. QUEST-TRIGGER-1 is `ALLOCATION_BLOCKED` on it.
- **Blocked:** QUEST-TRIGGER-1, and the levers and step triggers of QUEST-CONTENT-2.
- **Not unblocked by this ruling alone:** QUEST-TRIGGER-1 still depends on QUEST-GATE-1 (brief
  row). QUEST-GATE-1 is allocated first, and Trigger is allocated against its merged seam.
- **Harder later:** nothing irreversible. Option B adds a kind and a key; existing receipts keep
  their kinds and are not rewritten.
- **Superseding evidence:**
  - content that needs two quest children or claims with one transition key under one root;
  - content that needs two `RewardClaim` children under one root, or a claim on `ON_LEAVE`;
  - an accepted nested cascade, multi-cell command, non-command root or cross-character cause;
  - official behaviour of quest step triggers on a push or relocation.
- **Deliberately not decided:** §16.4.

### 16.6 Rejected

- **Option B now.** A cause kind, a receipt key migration and a new binding version, for no v1
  content need.
- **Hashing the child reference into a UUIDv7, or packing it into `CommandId` or
  `cause_ordinal`.** This is the ad hoc identity that successor §18 forbids.
- **Merging equal quest children of one root at runtime.** It silently loses a declared child.

### 16.7 Before-freeze checklist

1. **Amendments:** §4 "Accepted successor sections", "Roots" and "Quest child", the header and
   the brief row, in place; QUEST-STATE-0 §4 "Request" carries a pointer; WORLD-INTERACTION-0
   §7.1 and its PUSH-1 brief row, in place; WORLD-INTERACTION-0 §3 durable trigger children
   recovery (§16.2.5), in place; DUR-03 §39.3 and D39 §4.1 carry the reward MINT retirement
   amendment (§16.2.5) in place, pending on acceptance of QUEST-GATE-0.
2. **Serialization:** unchanged. Quest children run in canonical order in one sequencer slot.
3. **Restart:**
   - Receipts and obligations are durable.
   - Trigger plans are not durable; recovery follows §16.2.5.
   - A process loss between the root and a quest child loses the plan. A root discoverable
     through a durable child record is reconciled explicitly; a root with none left no durable
     quest or claim effect. Children with a durable record (a quest receipt, a claim's MINT) keep
     it, a committed claim's obligation is requested again at admission, a claim reservation with
     no receipt, marked as a trigger child at reservation, is retired under the cause lock by a
     durable retirement row and then settled by
     its receipt or `REJECTED`, and every later commit pass refuses on that row, and every other child is `REJECTED` and never runs again. A relocation that ran is not re-run or reverted; the
     Character resumes at its last persisted position (CHAR-POSITION-0).
4. **Typed references:** the root CommandRef (GameSessionId, CommandId), `transition_key`, the
   binding's definition and `placement_key`, and the content revision.
5. **Wire:** none new. A root refused by the plan check uses the existing `USE_INTENT` and step
   refusal results.
6. **Atomic commit:** one transaction per quest child (QUEST-STATE-0 §5). The plan check runs
   before the root commits.
7. **QUEST-TRIGGER-1 tests for plan loss after a claim reservation:**
   - a process loss after the reservation and before the MINT: the reconciliation finds no
     receipt, inserts the retirement row and rejects the child; nothing is minted; a fresh `USE` claims the
     chest once and is not refused with `ClaimPending`;
   - a pass in flight that commits during the retirement: the receipt read under the cause lock
     settles the child `COMMITTED`, its obligation commits at a later admission, and no second
     item is minted;
   - a pass that charged its work unit before the retirement and reaches the cause lock after
     it: the retirement reads no receipt and settles `REJECTED`, the pass refuses with
     `CapacityExceeded` and writes nothing, so no item, RewardClaim or obligation exists, and a
     fresh `USE` claims the chest once. The test runs once with that charge as the first RL-08
     unit and once as the third and final unit, where the stored count equals the pass's own;
   - a reservation of another Character under the same CommandRef is refused and left unchanged;
   - a reservation written by an NPC-QUEST-1 dialogue claim or a D39 chest `USE` claim has no
     trigger-child row: the retirement refuses with `NotRetirable`, writes nothing, and the
     reservation keeps its own pending or replay rule; a trigger-child row inserted outside the
     transaction that inserts its reservation is refused by the guard; a pass whose origin
     disagrees with the row's presence refuses with `ConflictingCause`;
   - `reconcile_reward_claim_mint` after a retirement returns `Retired` without charging a work
     unit, and its caller settles `REJECTED` and never retries the candidate; with a receipt it
     returns `Committed`; with neither it returns `Pending`;
   - a rerun of the reconciliation changes nothing and reads the same result;
   - a freeze under a retired CommandRef refuses; the runtime role cannot update, delete or
     truncate a retirement row;
   - a replacement GameSession never resumes the old reservation.
8. **One claim per root (§16.2.2):**
   - QUEST-CONTENT-2 validation fails on each of these: two `RewardClaim` children on one `USE`
     binding set; a trigger claim beside the D39 chest claim of the same placement; two claims
     among one cell's `ON_ENTER` bindings; a claim on an `ON_LEAVE` binding;
   - QUEST-TRIGGER-1: a plan that holds two `RewardClaim` children is refused before the root
     commits, with no reservation, no MINT and a content defect recorded.
