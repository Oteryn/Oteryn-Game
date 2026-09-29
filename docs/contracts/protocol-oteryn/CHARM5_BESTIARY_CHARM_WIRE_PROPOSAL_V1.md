# CHARM-5 Bestiary and Charm wire — protocol amendment proposal V1

- Date: 2026-09-29
- Status: **PROPOSAL**. It needs protocol-owner assignment of the IDs, owner acceptance and independent protocol
  review before any registry, proto file, session or server composition change.
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

There is **no unassign command** in this version (CHARM-0 §7 answer 3c). Unassign ships later, with its gold fee,
after `GAME-ITEM-01`/`DUR-03` prove the Character and Item boundary.

Out of scope: charm reset, Charm Upgrade, the Store expansion, Bosstiary, creature names or descriptions on the wire,
and every rule decision (CHARM-2 and CHARM-3 own them).

## 2. Proposed IDs, identifiers and limits

The protocol owner assigns IDs. On `main@4ea220fa` the next free ones are command types 4 and 5 and state domains 4
and 5. The names below use those numbers only as proposals. A unit test fails if another writer registers one of
them under a different name.

| ID | Name | Kind | Payload | Result |
|---|---|---|---|---|
| 4 | `CHARM_UNLOCK_STAGE_INTENT` | command type | `CharmUnlockStageIntentV1` | `CharmUnlockStageResultV1` |
| 5 | `CHARM_ASSIGN_INTENT` | command type | `CharmAssignIntentV1` | `CharmAssignResultV1` |
| 4 | `CHARACTER_BESTIARY` | state domain; delta type 1, snapshot type 1 | `BestiaryViewV1` | — |
| 5 | `CHARACTER_CHARMS` | state domain; delta type 1, snapshot type 1 | `CharmViewV1` | — |

Identifiers:

- **Race index:** 1-based into the Bestiary race list of the loaded content generation, ordered by Creature
  definition key (bytewise). It is not a Canary race ID (registry invariant: Canary IDs are never imported).
- **Charm index:** 1-based into the charm list of the loaded content generation (`content/charms/`, CHARM-1),
  ordered by charm key.
- Both follow SPELL-D1: an index, not a key string, so there is no free text on the wire and the server looks it up
  in O(1). The client resolves names from the same content generation (open question 1).

Proposed resource limits (for `RESOURCE_LIMITS_REGISTRY.json` on acceptance):

| ID | Resource | Hard maximum | Reason |
|---|---|---|---|
| `CHARM5-RL-01` | Bestiary races in one view (and the largest race index) | 1024 | 833 Bestiary races in the captured staticdata (CHARM-0 §2) |
| `CHARM5-RL-02` | Largest Bestiary kill threshold | 100000 | The captured definitions top out at 5000 |
| `CHARM5-RL-03` | Charms in one view (and the largest charm index) | 32 | 25 charms in the candidate catalogue |
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
// At most 488 bytes: 32 entries of at most 15 bytes and two balances of at most 4 bytes.
message CharmViewV1 {
  repeated CharmStateV1 charms = 1;      // strictly ascending by charm, at most 32
  uint32 charm_points_available = 2;     // derived, never stored; at most 1000000
  uint32 minor_charm_echoes_available = 3; // derived, never stored; at most 1000000
}
```

- The view is small, so **every delta replaces the whole view**.
- The balances are derived by CHARM-3 (earned minus spent, CHARM-0 §4.2 and §7 answer 2a). The wire carries only
  what is available, which is what the client can act on.
- How many charms one race may hold is still `UNKNOWN` (CHARM-0 §7). The view does not constrain it: two charms may
  name the same race.

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

`expected_stage` makes the command safe against a stale view: a player who pressed "unlock stage 2" never pays for
stage 3. FND-02 command sequencing already de-duplicates a resent command; this guards the other case, two
different commands sent from the same view.

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
}

// CommandResult.payload of command type 5: outcome only. At most 4 bytes.
message CharmAssignResultV1 {
  CharmAssignDisposition disposition = 1;
}
```

- `ALREADY_ASSIGNED` covers re-assigning a charm to the same race too. Without an unassign command, an assignment
  is final in this version; this is the listed, reversible deviation of answer 3c.
- The dispositions are stable and closed. A new reason needs a new enum value through this contract, never free
  text.

## 4. Behaviour

- **Authority.** The client proposes; CHARM-3 decides. Each command is one Character transaction under the
  session-generation fence (`CurrentCharacterGameplayFence`), validated against the derived balance. A result is
  sent only after that transaction committed or was refused. The new state then arrives through domain 5.
- **Malformed intents.** As for the other commands, a payload that does not decode gets the `REJECTED` disposition
  and changes nothing.
- **Bounds.** The encoder refuses every value the decoder would refuse, as a server (or client) fault, before any
  byte is emitted. A view that CHARM-2 or CHARM-3 cannot fit in the bounds is a server fault, never truncated.

## 5. Server boundary

`apps/game-server/src/gameplay_transport/charm.rs` is a thin adapter behind one narrow trait, `CharmProgressionPort`:
the Bestiary and Charm views of the one fenced character the port is bound to, and the two commands. CHARM-2 and
CHARM-3 implement the port; composition binds it to the session's character and routes command types 4 and 5 and
domains 4 and 5. The adapter only decodes, calls the port, and encodes, with test doubles for the port. No command
type or domain is dispatched until the IDs are registered.

## 6. Client

`apps/client/src/cyclopedia.rs` holds the pure client state: the Bestiary and Charm views with snapshot and delta
application, the derived stage and progress, the local preconditions for the two commands, and one line of player
feedback per disposition. Session wiring (sending the commands and routing domains 4 and 5) follows registration.

## 7. Open questions

1. **Names on the client.** The wire carries indices. How does the client resolve race and charm names?
   - a) A client content export of the same content generation (recommended; no text on the wire).
   - b) A bounded name string in each view entry.
2. **State-domain owner.** Is the owner of domains 4 and 5 recorded as the Character Authority (recommended; it is
   Character state), or as the current ChannelRuntime like the other domains?
3. **Races with no kills.** Only counted races are sent. Should the view also list every Bestiary race with zero
   kills (recommended: no; the client lists the rest from the content export of question 1)?

## 8. On acceptance

1. Register command types 4 and 5 and domains 4 and 5 in `PROTOCOL_OTERYN_V1_REGISTRY.json` with the byte bounds of
   §3, and `CHARM5-RL-01` to `CHARM5-RL-05` in `RESOURCE_LIMITS_REGISTRY.json`.
2. Move §3 into `docs/contracts/protocol-oteryn/v1/charm_bestiary_v1.proto` and point the codec tests at the registry
   entries, as `actor_spell` does.
3. Compose: route the commands in `gameplay_transport/connection.rs` through the adapter, bind the port to CHARM-2 and
   CHARM-3, and add the session calls and domain routing for the client.
