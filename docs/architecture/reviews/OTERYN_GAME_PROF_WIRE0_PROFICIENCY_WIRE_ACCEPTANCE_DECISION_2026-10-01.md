# PROF-WIRE-0 Weapon Proficiency wire acceptance

- Decision: `PROF-WIRE0-PROFICIENCY-WIRE-ACCEPTANCE-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent protocol review and
  protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3), protocol owner
- Answers: PROFICIENCY-0 §4.4 ("contract candidate, owner acceptance required"); the control
  plane's request after owner answer 5b (D281, #162) to accept the wire so the IDs can be reserved.
  The capability choice follows owner precedent D168 (capability-gated); the rest is protocol-owner
  detail, as in the charm wire ruling (#162 5907282001).
- Builds on: PROFICIENCY-0 §4.2-§4.4; FND-02 §13 and §15; the charm wire ruling 5907282001 (the soft
  reservation of capability 2, command 6 and domain 6, and the Character Authority revision rule);
  A13 §4.1 (every build change is one CharacterRevision); `PROTOCOL_OTERYN_V1_REGISTRY.json`.
- Runtime, migration and production authority: NONE. PROF-WIRE-1 lands the registry rows.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Accepted IDs

| Kind | Id | Name |
|---|---|---|
| capability | 2 | `WEAPON_PROFICIENCY_V1` |
| command type | 6 | `PROFICIENCY_SELECT_PERK` |
| state domain | 6 | `ACTOR_PROFICIENCY` |

These are the IDs reserved by 5907282001 and still free in the registry. PROFICIENCY-0 §4.4's
"first registry capability" is stale: capability 1 exists, and PROF-WIRE-1 follows its registry row
shape (id, name, owner decision, gated message types).

## 2. Revision semantics

`ACTOR_PROFICIENCY` is a Character Authority domain projected by the current ChannelRuntime. It
follows the binding rule of 5907282001: the domain revision is the durable CharacterRevision of the
commit that last changed the proficiency projection (every PROF-1 receipt advances it, A13 §4.1);
revisions are monotonic and not contiguous; a delta carries `base_revision` and `new_revision` and
applies only on an exact match, else the client resyncs (FND-02 §15); every admission, reconnect,
resume and transfer starts with a full snapshot; deltas are emitted only after the durable commit.

- **Per-track revision.** Every track entry in a snapshot or delta carries the track's
  `committed_character_revision` (PROFICIENCY-0 §4.2), the CharacterRevision of the receipt that
  last changed that track. The client sends it back as `expected_revision`, and the server's stale
  check compares it with the row's current value. Tested: a select with the value from the latest
  delta is accepted; one with an older value is refused `STALE_REVISION`.
- **Large commits.** A commit that changes more than `PROFWIRE0-RL-05` (16) tracks (a training
  checkpoint or a migration can touch up to 666) is not sent as a delta. The server sends a full
  snapshot at `new_revision` instead, which replaces the client's whole domain state atomically.
  Tested: a 17-track checkpoint yields one snapshot replacement and no delta.

## 3. Bounds (PROF-WIRE-1 registers them)

| Row | Value |
|---|---|
| `PROFWIRE0-RL-01` `ACTOR_PROFICIENCY` snapshot payload | 32,768 bytes, sent through `SnapshotChunk` |
| `PROFWIRE0-RL-02` delta payload | 640 bytes (at most 16 track entries of 36 B, plus header) |
| `PROFWIRE0-RL-05` tracks per delta before a snapshot replacement | 16 |

**Derivation (worst-case protobuf).** One track entry: the entry's own tag and length (3 B); item id
(tag + varint ≤ 4 B); definition revision (tag + varint ≤ 3 B); progress up to 2³⁵ (tag + varint
≤ 6 B); `committed_character_revision` (tag + varint ≤ 11 B); 7 selections as a packed repeated
field (tag + length + 7 B = 9 B). That is 36 B; 666 entries are 23,976 B, plus a header (content
generation, domain revision) of at most 32 B, about 24,008 B. RL-01 leaves a 36% margin.
PROFICIENCY-1's modification rows add to the entry and are re-measured by PROF-SHAPE-WIRE-1 before
its commands ship.
| `PROFWIRE0-RL-03` `PROFICIENCY_SELECT_PERK` payload | 24 bytes |
| `PROFWIRE0-RL-04` command result payload | 16 bytes |

A track count above 666 in a later content generation needs a new value for RL-01 before it ships.

## 4. Command result

`PROFICIENCY_SELECT_PERK {item_id, level, perk_index | clear, expected_revision}` returns one of
`ACCEPTED`, `NOT_UNLOCKED`, `NOT_IN_PROTECTION_ZONE`, `UNKNOWN_TRACK`, `UNKNOWN_PERK` or
`STALE_REVISION` (when `expected_revision` differs from the track's committed revision). These are
command results, not protocol errors. An unnegotiated command keeps PROFICIENCY-0's
`CAPABILITY_MISMATCH` (1007, `SESSION_FATAL`).

## 5. Advertising and later commands

- **One offer gate:** the server advertises capability 2 only when PROF-1 (durability) and PROF-2
  (accrual and selection legality) are both live. PROFICIENCY-1 is not part of the gate.
- The PROFICIENCY-1 operations take the next free command types at their own acceptance and join
  capability 2 as additive commands. Until an operation is admitted, its command is answered with
  PROFICIENCY-1's typed `NOT_ADMITTED` result, never a protocol error, so adding or admitting them
  is not a breaking change.

## 6. Decision test

- **Must decide now:** YES. PROF-1 and PROF-2 are allocated (D281), and PROF-WIRE-1 cannot reserve
  registry rows or build the client view without accepted IDs, revision semantics and bounds.
- **Blocked:** PROF-WIRE-1 (registry rows, `.proto`, client view) and the release of Weapon
  Proficiency to players.
- **Harder later:** capability 2, command 6 and domain 6 become permanent registry identities that
  cannot be reused; the per-track `committed_character_revision` and the snapshot-replacement rule
  become client compatibility obligations; the bounds RL-01 to RL-04 constrain later additions
  (PROFICIENCY-1's rows must re-measure RL-01).
- **Superseding evidence:** a measured snapshot above RL-01, or a client need the delta model cannot
  meet.
- **Deliberately not decided:** PROFICIENCY-1's command numbers and payloads (§5), Mastery display.

## 7. Owner questions

None (D168 settles the capability choice).

## 8. Before-freeze checklist

1. **Contract amendments:** PROFICIENCY-0 §4.4 (accepted). Applied in this PR.
2. **Wire:** one capability, one command, one domain; bounds above.
3. **Split work:** none.
