# CHAR-VITALS-0 Durable health and mana

- Decision: `CHAR-VITALS0-DURABLE-VITALS-V1`
- Status: **ACCEPTED**. Accepted by the owner (`blakinio`) on 2026-10-13 via the control plane
  (`ACCEPT-CHAR-VITALS-0`, `OTERYN_GAME_ACCEPT_CHARVITALS0_ACCEPTANCE_DECISION_2026-10-13.md`),
  effective only on exact-head review, CI and protected integration; no rule text changes.
  Original gate: the control plane accepts it in a separate PR. Acceptance needs exact-head
  validation, independent review (persistence) and protected integration.
- Input: owner decision of 2026-10-13: "persist character HP and mana (restore on login/restart)".
- Answers: the HP/mana part of CHAR-POSITION-1, blocked there pending this decision (task record
  `OTV2-20261012-char-position-1`, open question 1).
- Builds on: CHAR-POSITION-0 (ACCEPTED, `OTERYN_GAME_CHAR_POSITION0_LOGOUT_POSITION_DECISION_2026-09-30.md`)
  §3.1 to §3.3; the composition decision §3 amendment (Character runtime-state projections) and
  rule 6; the player spell cast wire and vitals contract
  (`OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md`) SPELL-D2 and SPELL-D8.
- Amends:
  - SPELL-D2: health and mana stop being non-durable. They are saved as part of the CHAR-POSITION-0
    projection row, not through a DUR-02 receipt (§4).
  - CHAR-POSITION-0 §3.1: two columns on `game_character_last_positions` (§3.1 below).
- Wire and protocol: no change. The restored values reach the client through the existing vitals
  messages.
- Runtime, migration and production authority: NONE beyond the implementing child
  (CHAR-VITALS-1, migration lease `0084`).
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Question

How do a Character's current health and mana survive a logout, a reconnect after grace expiry and
a node restart?

## 2. Facts

**PROVEN**

- Health and mana live only in the runtime actor's spell state; a new actor starts at its
  maxima (SPELL-D2).
- CHAR-POSITION-0 already keeps one fenced, ordered, never-deleted row per Character, written at
  the terminal release, periodically (at most every 300 s, `CHARPOS0-RL-01`) and at node shutdown,
  and read under the session fence before the first-entry owner step.
- A live death leaves a pending respawn; the respawned actor returns at its maxima (first player
  death decision §4.5, D63; `PlayerSpellState::respawned`).

**ASSUMED**

- Players accept the loss of at most the last 300 s of health and mana changes on a node crash,
  as they already do for the position.

## 3. Decision

The saved health and mana are part of the CHAR-POSITION-0 runtime-state projection. Under
composition rule 6 this decision names them as part of that projection: they advance no
`CharacterRevision`, write no root, progression or receipt row, and take the same session fence
and the same write order as the cell.

### 3.1 Storage

`game_character_last_positions` gains `health INTEGER NULL` and `mana INTEGER NULL`.

- Both are NULL or both are set. A set pair is a living actor: health is 1 to `MAX_VITAL_POOL`
  (2^28 - 1, SPELL-D8) and mana 0 to `MAX_VITAL_POOL`.
- NULL means "start at the maxima": an actor saved at both maxima, a respawn placement, and every
  row written before the migration.
- The existing guards cover the columns: an UPDATE must strictly raise
  (`character_lease_generation`, `write_sequence`) and keep the Character and its World; no
  TRUNCATE; the same grants.

### 3.2 Writes

- The vitals ride every CHAR-POSITION-0 §3.2 write of the cell, in the same upsert and under the
  same key: the terminal release, the periodic write, the shutdown flush. A pass that would write
  nothing for the cell now also writes when only the vitals changed.
- They are captured from the actor in the same owner step as its cell. A dead actor is never
  captured (CHAR-POSITION-0). An actor at both maxima writes NULL.
- A respawn placement writes NULL: the respawned actor starts at its maxima.
- Nothing is written while a pending respawn exists or in an instance scope (unchanged). An
  instance actor's vitals are therefore not saved; the next admission restores the last row
  written outside it.

### 3.3 Admission

- The admission reads the row with the cell (CHAR-POSITION-0 §3.3, same read, same fence).
- The vitals are restored only when the first-entry step creates the actor's spell state and the
  outcome is a positioned admission from a stored row of the same World. A pending respawn, a
  missing row, a row of another World, a NULL pair or an existing spell state (a reconnect inside
  grace keeps its live actor) starts or keeps the actor as before.
- Each value is capped at the actor's current maximum. A restore never kills: health 0 is not
  restored.
- The restore advances no revision and sends nothing beyond the existing vitals messages.

## 4. Rejected options

- **DUR-02 receipt per change (SPELL-D2's "under DUR-02" route):** every hit and every cast would
  advance the `CharacterRevision` chain and write a receipt. That floods the chain and its replay
  with values that carry no semantic authority, and races the guard re-issue of the monk state.
- **A separate table:** a second row with its own order key and writer would need its own fence
  and could disagree with the cell after a crash. The same row keeps one order for both.
- **Restore on every first entry (including a live reconnect):** would overwrite a live actor's
  current values with an older save.

## 5. Decision test

- A wounded actor that logs out and logs in again starts with the saved health and mana, capped
  at its current maxima.
- After a node restart the stored values are restored on the next admission.
- A stale connection, lease or scope cannot overwrite the saved vitals; a lower or equal key is
  superseded.
- A respawned actor starts at its maxima.
- A failed terminal release writes neither the cell nor the vitals.

## 6. Risks

- Crash loss of at most 300 s of vitals changes (`CHARPOS0-RL-01`), the same as the cell.
- An instance actor's vitals are not saved (no instance-scope write).
