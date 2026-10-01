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

## 3. Bounds (PROF-WIRE-1 registers them)

| Row | Value |
|---|---|
| `PROFWIRE0-RL-01` `ACTOR_PROFICIENCY` snapshot payload | 12,288 bytes (666 tracks × 17 B, about 11.3 KB, plus framing), sent through `SnapshotChunk` |
| `PROFWIRE0-RL-02` delta payload | 32 bytes per changed track; one delta message carries at most 16 tracks |
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

- The server advertises capability 2 only when PROF-1 (durability) and PROF-2 (selection legality)
  are both live; before that no session can negotiate it.
- The PROFICIENCY-1 operations (refine, reshape, Catalysts, Lunar Orb) take the next free command
  types at their own acceptance and join capability 2; capability 2 is not advertised before they
  are decided or explicitly excluded, so adding them is not a breaking change.

## 6. Owner questions

None (D168 settles the capability choice).

## 7. Before-freeze checklist

1. **Contract amendments:** PROFICIENCY-0 §4.4 (accepted). Applied in this PR.
2. **Wire:** one capability, one command, one domain; bounds above.
3. **Split work:** none.
