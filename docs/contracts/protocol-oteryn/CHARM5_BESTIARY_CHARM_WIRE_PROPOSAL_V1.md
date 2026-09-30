# CHARM-5 Bestiary and Charm wire — protocol amendment proposal V1

- Date: 2026-09-29; revised 2026-09-30 after the independent protocol review of `a7ceccaa` (PR #1301) and owner
  decisions D168 to D170
- Status: **IDs ACCEPTED by the protocol owner** (Sol ruling, #162 comment 5907282001): capability 1
  `BESTIARY_CHARMS_V1`, command types 4 and 5, state domains 4 and 5, bounds 21504 and 488 bytes, and
  `CHARM5-RL-01` to `CHARM5-RL-05`. The charms bound is now 490 bytes (§3); Sol must acknowledge that change on
  #162 before the registry PR. Nothing is registered yet (§8).
- Authority: none. This document changes no protocol or resource registry, proto file, DDL, content or production
  state. The codecs that implement it (`crates/protocol-oteryn/src/{bestiary,charm}.rs`) are not reachable from any
  session or connection until the IDs are registered.
- Task: `OTV2-20260929-charm5-protocol-client` (#162).
- Parent: CHARM-0 decision packet
  (`docs/architecture/reviews/OTERYN_GAME_CHARM0_BESTIARY_CHARM_PROGRESSION_DECISION_PACKET_2026-09-29.md`, PR #1295):
  slice CHARM-5, §4.1, §4.2 and the owner answers in §7.
- Pattern: FIRST-CONTROL-WIRE-V1 and the spell cast wire
  (`docs/architecture/OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md`):
  - typed proto3 payloads inside FND-02 envelopes;
  - zero or unknown enum values and unknown or repeated fields fail closed;
  - a small byte bound per payload;
  - a result reports an outcome, never state;
  - state travels only through a state domain with its snapshot and delta.

## 1. Scope

The client needs to see and act on the Bestiary and Charm state that CHARM-2 and CHARM-3 own:

- a Bestiary view: per race, the kill count and thresholds, and the derived unlocked stage;
- a Charm view: every charm with its unlocked stage and assignment, and the derived Charm Points and Minor Charm
  Echoes available;
- two commands: unlock the next stage of a charm, and assign a charm to a race.

There is **no unassign or reset command in this proposal.** D170 supersedes the scope of CHARM-0 answer 3c: unassign
(level × 100 gp, 25% refund with the Charm Expansion) and the full charm reset are planned as in Tibia, as task
CHARM-6, which depends on `GAME-ITEM-01`/`DUR-03`. CHARM-6 proposes its own command types for them, and player-facing
charm release is gated on CHARM-6 (§8).

Out of scope: unassign and reset (CHARM-6), Charm Upgrade, the Store expansion, Bosstiary, creature names or descriptions on the wire,
and every rule decision (CHARM-2 and CHARM-3 own them).

## 2. Proposed IDs, identifiers and limits

The protocol owner assigned these IDs in the Sol ruling (#162 comment 5907282001). They are registered only by the
acceptance PR, which also checks that they are still free (§8).

| ID | Name | Kind | Payload | Result |
|---|---|---|---|---|
| 4 | `CHARM_UNLOCK_STAGE_INTENT` | command type | `CharmUnlockStageIntentV1` | `CharmUnlockStageResultV1` |
| 5 | `CHARM_ASSIGN_INTENT` | command type | `CharmAssignIntentV1` | `CharmAssignResultV1` |
| 4 | `CHARACTER_BESTIARY` | state domain; delta type 1, snapshot type 1 | `BestiaryViewV1` | — |
| 5 | `CHARACTER_CHARMS` | state domain; delta type 1, snapshot type 1 | `CharmViewV1` | — |

**Capability-gated (D168 precedent, Sol ruling).** Command types 4 and 5 and domains 4 and 5 belong to capability
1 `BESTIARY_CHARMS_V1`:

- Without the capability selected, the server sends no snapshot or delta of domains 4 and 5, and refuses command types
  4 and 5 as unsupported.
- It is consistent with PROFICIENCY: D168 accepted the PROFICIENCY-0 wire as a capability-gated snapshot and delta.
- It doubles as the D170 release gate. The server does not offer the capability until CHARM-6 ships, so no client
  sees or sends charm traffic before unassign and reset exist, and no composition flag of its own is needed.

Identifiers:

- **Race index:** 1-based into the Bestiary race list of the loaded content generation, ordered by Creature
  definition key (bytewise). It is not a Canary race ID (registry invariant: Canary IDs are never imported).
- **Charm index:** 1-based into the charm list of the loaded content generation (`content/charms/`, CHARM-1),
  ordered by charm key.
- Both follow SPELL-D1: an index, not a key string, so there is no free text on the wire and the server looks it up
  in O(1). The client resolves names from the same content generation (owner answer 8a, §7).
- **The content generation is fixed per connection.** Both peers derive the indices once, from the content generation
  bound at admission, and neither re-derives them while the connection lives. A change of content generation reaches
  the client only through a new connection, whose snapshots of domains 4 and 5 are re-derived. Hot content reload
  inside one connection is out of scope.

Proposed resource limits (for `RESOURCE_LIMITS_REGISTRY.json` on acceptance):

| ID | Resource | Hard maximum | Reason |
|---|---|---|---|
| `CHARM5-RL-01` | Bestiary races in one view (and the largest race index) | 1024 | 833 Bestiary races in the captured staticdata (CHARM-0 §2) |
| `CHARM5-RL-02` | Largest Bestiary kill threshold | 100000 | The captured definitions top out at 5000 |
| `CHARM5-RL-03` | Charms in one view (and the largest charm index and slot limit) | 32 | 25 charms in the candidate catalogue |
| `CHARM5-RL-04` | Largest single charm stage cost | 100000 | Either currency |
| `CHARM5-RL-05` | Largest available balance of either currency | 1000000 | Earned Charm Points are at most 1024 × 100 |

## 3. Schema

Proposed file on acceptance: `docs/contracts/protocol-oteryn/v1/charm_bestiary_v1.proto` (not created).

### 3.1 Bestiary view

```proto
// One race of the character's Bestiary. Only races with at least one credited kill appear.
message BestiaryRaceProgressV1 {
  uint32 race = 1;            // race index, 1..=1024
  uint32 kill_count = 2;      // 1..=final_threshold: counters saturate there (CHARM-0 §4.1)
  uint32 first_threshold = 3; // 1 <= first < second < final <= 100000
  uint32 second_threshold = 4;
  uint32 final_threshold = 5;
}

// StateDelta.payload of domain 4 delta type 1 and StateDomainSnapshot.payload of snapshot type 1.
// At most 21504 bytes: 1024 entries of at most 21 bytes each.
message BestiaryViewV1 {
  repeated BestiaryRaceProgressV1 races = 1; // strictly ascending by race, at most 1024
}
```

- The **snapshot** is the full set of counted races. A **delta** upserts only the listed races and leaves the others
  unchanged. Counters never decrease in this version, so a delta never removes a race.
- The **unlocked stage is not on the wire.** It is derived as the number of thresholds the kill count has reached,
  0 to 3; 3 is a complete entry. CHARM-0 §4.1 derives it and never stores it, and sending it too would only add a
  way for the two to disagree. Thresholds are sent so that the client can show progress without a content lookup.

### 3.2 Charm view

```proto
enum CharmKind {
  CHARM_KIND_UNSPECIFIED = 0;
  CHARM_KIND_MAJOR = 1; // stages cost Charm Points
  CHARM_KIND_MINOR = 2; // stages cost Minor Charm Echoes
}

message CharmStateV1 {
  uint32 charm = 1;           // charm index, 1..=32
  CharmKind kind = 2;
  uint32 unlocked_stage = 3;  // 0 (locked) to 3
  uint32 assigned_race = 4;   // race index; 0 = unassigned; only when unlocked_stage >= 1
  uint32 next_stage_cost = 5; // in the kind's currency; 0 exactly when unlocked_stage == 3
}

// StateDelta.payload of domain 5 delta type 1 and StateDomainSnapshot.payload of snapshot type 1.
// At most 490 bytes (Sol accepted 488 before the slot limit; the 490 needs Sol's acknowledgement on #162): 32 entries of at most 15 bytes, two balances of at most 4 bytes, and the slot
// limit of at most 2 bytes.
message CharmViewV1 {
  repeated CharmStateV1 charms = 1;      // strictly ascending by charm, at most 32
  uint32 charm_points_available = 2;     // derived, never stored; at most 1000000
  uint32 minor_charm_echoes_available = 3; // derived, never stored; at most 1000000
  uint32 assignment_slot_limit = 4;      // D169: 2 free, 6 Premium; 0 = no limit (Charm Expansion); at most 32
}
```

- The view is small, so **every delta replaces the whole view**.
- The balances are derived by CHARM-3 (earned minus spent, CHARM-0 §4.2 and §7 answer 2a). The wire carries only
  what is available, which is what the client can act on.
- One race holds at most one major and one minor charm at a time. CHARM-3 enforces it (migration 0020, #1307); the view
  does not re-check it, so two charms may name the same race.
- **Slots (D169).** `assignment_slot_limit` is the number of charms the character may hold assigned: 2 for a free
  account, 6 with Premium, and no limit with the Charm Expansion, sent as 0. CHARM-3 derives the entitlement in the
  command's transaction (`CharmSlotEntitlement`) and enforces the limit there; the view field is a display hint. The
  slots in use are the charms with `assigned_race != 0`, so they are not sent; sending them too would only add a way
  for the two to disagree. The view does not require the slots in use to be within the limit, because a lapsed
  Premium can leave more charms assigned than the free limit.

### 3.3 Unlock the next stage

```proto
// ClientCommand.payload of command type 4. At most 8 bytes (canonical worst case 4).
message CharmUnlockStageIntentV1 {
  uint32 charm = 1;          // charm index, 1..=32
  uint32 expected_stage = 2; // the stage the client saw, 0..=2; the command unlocks expected_stage + 1
}

enum CharmUnlockDisposition {
  CHARM_UNLOCK_DISPOSITION_UNSPECIFIED = 0;
  CHARM_UNLOCK_DISPOSITION_UNLOCKED = 1;
  CHARM_UNLOCK_DISPOSITION_NOT_ENOUGH_CHARM_POINTS = 2;
  CHARM_UNLOCK_DISPOSITION_NOT_ENOUGH_MINOR_CHARM_ECHOES = 3;
  CHARM_UNLOCK_DISPOSITION_STAGE_MISMATCH = 4; // not at expected_stage: stale view, or already at stage 3
  CHARM_UNLOCK_DISPOSITION_UNKNOWN_CHARM = 5;  // no charm has this index in the loaded content generation
  CHARM_UNLOCK_DISPOSITION_REJECTED = 6;       // malformed intent, ineligible actor, lost fence, unavailable
}

// CommandResult.payload of command type 4: outcome only. At most 4 bytes.
message CharmUnlockStageResultV1 {
  CharmUnlockDisposition disposition = 1;
}
```

`expected_stage` lets the server refuse a command sent from a stale view. It is a check by the port before the
commit, not a guarantee under the Character root lock: CHARM-3's `UnlockNextStage` carries no expected stage, and
`commit_charm_command` unlocks whatever stage is next when it holds the lock. The port reads the stored stage,
answers `STAGE_MISMATCH` if it differs from `expected_stage`, and otherwise commits. The check is exact while one
session serializes its charm commands, which the session-generation fence and FND-02 ordered command ingress
provide; this proposal claims nothing more. Moving the check under the root lock needs an expected stage on the
CHARM-3 command, which is a CHARM-3 change outside this proposal.

A resent command is not a second command: the port commits under a CHARM-3 occurrence derived from the FND-02
command identity (§5), so CHARM-3 returns the retained `AlreadyCommitted` outcome instead of unlocking again.

### 3.4 Assign a charm to a race

```proto
// ClientCommand.payload of command type 5. At most 8 bytes (canonical worst case 5).
message CharmAssignIntentV1 {
  uint32 charm = 1; // charm index, 1..=32
  uint32 race = 2;  // race index, 1..=1024
}

enum CharmAssignDisposition {
  CHARM_ASSIGN_DISPOSITION_UNSPECIFIED = 0;
  CHARM_ASSIGN_DISPOSITION_ASSIGNED = 1;
  CHARM_ASSIGN_DISPOSITION_CHARM_LOCKED = 2;       // unlocked_stage == 0
  CHARM_ASSIGN_DISPOSITION_ALREADY_ASSIGNED = 3;   // the charm holds a race; no unassign in this version
  CHARM_ASSIGN_DISPOSITION_RACE_STAGE_TOO_LOW = 4; // major: complete entry; minor: stage 2 (CHARM-0 §4.2)
  CHARM_ASSIGN_DISPOSITION_RACE_CHARM_LIMIT = 5;   // the race holds as many charms as the rule allows
  CHARM_ASSIGN_DISPOSITION_UNKNOWN_CHARM = 6;
  CHARM_ASSIGN_DISPOSITION_UNKNOWN_RACE = 7;
  CHARM_ASSIGN_DISPOSITION_REJECTED = 8;           // malformed intent, ineligible actor, lost fence, unavailable
  CHARM_ASSIGN_DISPOSITION_ASSIGNMENT_SLOTS_FULL = 9; // every assignment slot is in use (D169)
}

// CommandResult.payload of command type 5: outcome only. At most 4 bytes.
message CharmAssignResultV1 {
  CharmAssignDisposition disposition = 1;
}
```

- `ALREADY_ASSIGNED` covers re-assigning a charm to the same race too. Moving an assigned charm needs the CHARM-6
  unassign (D170), which is not part of this proposal.
- `ASSIGNMENT_SLOTS_FULL` is CHARM-3's `AssignmentSlotsFull`: the character already holds as many assigned charms as
  its slot entitlement allows (D169). CHARM-3 checks it after the race's per-category limit.
- The dispositions are stable and closed. A new reason needs a new enum value through this contract, never free
  text.

## 4. Behaviour

- **Authority.** The client proposes; CHARM-3 decides. Each command is one Character transaction under the
  session-generation fence (`CurrentCharacterGameplayFence`), validated against the derived balance. A result is
  sent only after that transaction committed or was refused. The new state then arrives through domain 5.
- **Domain revisions.** The revision of domains 4 and 5 is the **CharacterRevision**, not a per-domain counter.
  Delta continuity never crosses a connection (Sol ruling):
  - Every admission, reconnect, resume and transfer starts with a full snapshot of both domains. Its
    `StateDomainSnapshot.revision` is the CharacterRevision the view was read at, in the same transaction as the view.
  - A `StateDelta` is sent only when a commit changes that domain's view. Its `base_revision` is the last revision the
    client received for the domain on this connection, and its `new_revision` is the CharacterRevision the new view was
    read at. A commit that does not change the domain, such as an XP award, sends no delta.
  - Why the CharacterRevision: every writer of this state advances it under the Character root lock (CHARM-2 for kill
    counters, CHARM-3 exactly once for each command, #1307). It is therefore monotonic, durable and never reused, as
    FND-02 §15 requires, with no new counter to store. The existing domains follow the same rule where the state has
    a durable authority revision: `WORLD_OBJECT_OVERLAY` uses the committed entry's revision and `ACTOR_VITALS` the
    actor state's revision.
  - Because other Character writes also advance the CharacterRevision, consecutive domain revisions may jump. FND-02
    needs only monotonic revisions and `base_revision` equal to the client's applied revision.
  - The port reads each view and its CharacterRevision in one transaction. CHARM-3's `read_character_charm_state`
    returns no revision today; the composition task adds that read.
- **Malformed intents.** As for the other commands, a payload that does not decode gets the `REJECTED` disposition
  and changes nothing.
- **Bounds.** The encoder refuses every value the decoder would refuse, as a server (or client) fault, before any
  byte is emitted. A view that CHARM-2 or CHARM-3 cannot fit in the bounds is a server fault, never truncated.

## 5. Server boundary

`apps/game-server/src/gameplay_transport/charm.rs` is a thin adapter behind one narrow trait, `CharmProgressionPort`:
the Bestiary and Charm views of the one fenced character the port is bound to, and the two commands. CHARM-2 and
CHARM-3 implement the port; composition binds it to the session's character and routes command types 4 and 5 and
domains 4 and 5. The adapter only decodes, derives the occurrence, calls the port, and encodes, with test doubles for
the port. No command type or domain is dispatched until the IDs are registered.

- **Asynchronous.** Every port method is asynchronous, because CHARM-3's `commit_charm_command` and its reads run
  through the durability root.
- **Occurrence.** A command method takes `(occurrence, intent)`. The occurrence is CHARM-3's
  `CharmCommandOccurrence`, derived from the FND-02 command identity `(GameSessionId, CommandId)`: CommandId is
  strictly increasing within the GameSession and survives an eligible reconnect, so a resent command keeps it. The
  derivation follows the spell-cast occurrence (`gameplay_transport/actor_spell.rs`):
  `SHA-256("oteryn:charm-command-occurrence:v1" || GameSessionId (16 bytes) || CommandId (u64 big-endian))`,
  truncated to 16 bytes, with the UUID version nibble set to 7 and the variant bits to `10`, which is the form
  `CharmCommandOccurrence::from_bytes` requires. Its first 48 bits are hash bits, not a timestamp. A resent command
  therefore returns CHARM-3's retained `AlreadyCommitted` outcome, and a reused CommandId with a different intent
  gets `ConflictingOccurrence`, which the port reports as `REJECTED`.
- **Unlock.** The port maps `UnlockNextStage` with the `expected_stage` pre-check of §3.3.
- **Dispositions.** The port maps CHARM-3's `CharmRuleError` one to one: `InsufficientBalance` to the kind's
  `NOT_ENOUGH_*`, `FinalStageReached` to `STAGE_MISMATCH`, `CharmLocked`, `CharmAlreadyAssigned`,
  `BestiaryStageTooLow`, `RaceCapacityReached` and `AssignmentSlotsFull` to their assign dispositions, and
  `UnknownCharm` to `UNKNOWN_CHARM`. An index with no charm or race in the connection's content generation is
  `UNKNOWN_CHARM` or `UNKNOWN_RACE` before CHARM-3 is called. Every other refusal is `REJECTED`.

## 6. Client

`apps/client/src/cyclopedia.rs` holds the pure client state: the Bestiary and Charm views with snapshot and delta
application, the derived stage and progress, the local preconditions for the two commands, and one line of player
feedback per disposition, including the slot limit and the slots in use (D169). Session wiring (sending the commands
and routing domains 4 and 5) follows registration.

## 7. Owner answers (2026-09-30, in session)

Recorded in the CHARM-0 packet §8 (PR #1295); decision-register numbers are assigned by the coordinator batch on
`#162`.

| Question | Answer | Effect |
|---|---|---|
| 8. Names on the client | **a** | The client resolves race and charm names from a client content export of the same content generation. No text is sent on the wire. |
| 9. State-domain owner | **a** | Domains 4 and 5 are owned by the Character Authority. |
| 10. Races with no kills | **a** | Only counted races are sent. The client lists the other races of a class from the content export, shown as unknown. |
| 11. Wire identifiers | **a** | Race and charm indices follow SPELL-D1 (§2): 1-based and derived per content generation. They are never stored; durable state keeps the Creature and charm keys. A display-name change moves no index. An added or removed race moves indices only in a new content generation, which both peers derive again. |

## 8. On acceptance

1. Register the command types and domains the protocol owner assigns in `PROTOCOL_OTERYN_V1_REGISTRY.json` with the
   byte bounds of §3, capability 1 `BESTIARY_CHARMS_V1` of §2, and `CHARM5-RL-01` to `CHARM5-RL-05` in
   `RESOURCE_LIMITS_REGISTRY.json`. That PR adds the test that the IDs are free or registered under these names.
2. Move §3 into `docs/contracts/protocol-oteryn/v1/charm_bestiary_v1.proto` and point the codec tests at the registry
   entries, as `actor_spell` does.
3. Compose: route the commands in `gameplay_transport/connection.rs` through the adapter, bind the port to CHARM-2 and
   CHARM-3, and add the session calls and domain routing for the client. **Composition stays off until CHARM-6
   ships (D170):** no server routes command types 4 and 5 or sends domains 4 and 5 to a player before unassign and
   reset exist. With the capability, that means the server does not offer it until then.
