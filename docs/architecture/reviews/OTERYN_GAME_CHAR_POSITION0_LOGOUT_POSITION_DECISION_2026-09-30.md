# CHAR-POSITION-0 Durable logout position

- Decision: `CHAR-POSITION0-LOGOUT-POSITION-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review
  (persistence) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the gap DEATH-0 §3.4 ("persisting a general Character position for ordinary logins
  remains a separate lane") and ADR-0021 §4.4 ("a later logout position"); the architect
  programme plan (#162 5910870596, M2)
- Builds on: DEATH-0 §3.4 (pending respawn), NPC-0 §6 and §6.1 (pending arrival, placement
  fallback; PR #1332, a candidate), the Character and item composition decision §3 (fence),
  ADR-0001 §7 and §12 (shared base, no channel change through a shared house), ADR-0021 §4.2 and
  §4.7 (bundle digest, world reset), PREMIUM-ACTIVATION-V1 §4.5 (login relocation, D119),
  ATTACK-0 §4 (in-fight deadline; PR #1347, a candidate)
- Amends: the composition decision §3 (a named class of runtime-state projection) and
  `MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md` (the player position row)
- Runtime, migration and production authority: NONE. The child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

- **CHAR-POSITION-1** (`oteryn-hard-worker`, persistence review):
  - one migration (next free lease at allocation);
  - the writes of §3.2, the terminal one inside the terminal release;
  - the admission read of §3.3;
  - `CHARPOS0-RL-01` in the resource registry (this PR), and tests.
  It depends on DEATH-1's admission consumption path, which it extends, and on MAP-LOAD-1 for the
  placeability check. HOME-TOWN replaces the `entry_start` fallback when it ships (§3.3).

## 1. Question

Where does a character appear when it logs in again after an ordinary logout, or after a server
restart?

## 2. Facts

**PROVEN**

- Every new runtime actor is placed at the pinned content `entry_start`
  (`runtime_actor_carrier.rs`, `gameplay_transport/mod.rs` first-entry path). No home town exists
  yet; the HOME-TOWN lane depends on this one (D118-D128 decision). A resumed session keeps its
  still-present actor and position (`gameplay_transport/resume.rs`).
- Terminal release sets `session_state = 3` (`fresh_admission.rs`, `release`), and the actor is
  removed only after it commits. A write fenced on `session_state IN (1,2)` after that commits
  nothing.
- DEATH-0 keeps an immutable pending respawn row, created by its death and consumed at placement;
  NPC-0 adds a pending arrival with the same pattern. Both are obligations inside DUR-03 or death
  transactions. The position row here is different: a mutable upsert with no cause.
- Composition rule 1 covers only item locations and DUR-03 cause records; rule 6 needs its own
  decision for any other non-XP Character write. The scope matrix lists player position as
  channel-local runtime state.
- PREMIUM-ACTIVATION-V1 §4.5: at login, a non-Premium character in a Premium area goes to its home
  temple, or to Thais if the home is a Premium city.
- ADR-0001 §12 forbids changing channel through a shared house.
- ADR-0021: a base map revision activates only at a planned world reset, under a new scope
  ownership generation; durable ground items survive a crash.
- The Tibia manual (`combat.md` §5.3.12.c): closing the client while logout-blocked leaves the
  character in the world. Tibia logs a character in where it logged out.

## 3. Decision

### 3.1 Storage

- `game_character_last_positions`: one row per Character (primary key `character_id`) with
  `world_id`, the native `WorldTilePosition`, the frame and map markers of
  `pinned_position_context`, the bundle digest it was written under, the session generation that
  wrote it, and a `write_sequence`.
- Foreign keys to the Character root and World, with a guard that `world_id` is the root's World,
  as the death guard does.
- The channel is not stored: every channel of a World shares the base map (ADR-0001 §7), and
  admission chooses the channel.
- The position encoding is one typed form shared with the pending respawn and pending arrival
  rows (World, `WorldTilePosition`, markers), replacing opaque bytes when those rows are next
  touched.
- The row is never deleted. No Character deletion workflow exists; when one is designed it must
  delete this row.
- Grants: `oteryn_game_runtime` SELECT, INSERT and UPDATE, never DELETE; `oteryn_game_control`
  SELECT.

### 3.2 Writes

This is a Character runtime-state projection (composition §3 amendment): outside the revision
chain, with no cause and no replay.

- **Fence.** The composition rule 2 session checks (reconnect-session row, runtime-scope
  assignment, admission guards), without a cause lock. It takes no `character_root` row lock: it
  reads no inventory, occupancy or claim. A stale session writes nothing.
- **Order.** `write_sequence` increases per write within one session generation. An upsert
  applies only when (session generation, `write_sequence`) is greater than the stored pair, so an
  in-flight periodic write never overwrites the final one.
- **At logout.** The final write runs inside the terminal release transaction, before
  `session_state` becomes 3, as H-1 writes before the lease is released.
- **Logout-blocked actor.** When the client closes during the ATTACK-0 in-fight deadline, the actor
  stays in the world; the final write runs in the terminal release that ends it.
- **Periodically:** every 5 minutes (`CHARPOS0-RL-01`) while the position has changed since the
  last write, so a crash loses at most that much movement.
- **Not written:**
  - while a pending respawn or pending arrival exists: those obligations decide the next
    placement;
  - in an instance scope (`runtime_scope_kind = 2`): the row keeps the last position outside,
    which is the instance's entry side;
  - at channel transfer, which does not exist yet; its decision adds the write.
- **World reset.** Step 2's new scope ownership generation fences late writes out, so the last
  periodic row is used and re-checked by §3.3.

### 3.3 Admission

The read runs only when a new runtime actor is created, in the first-entry path under the channel
owner's lock; it replaces `entry_start` as the input to `initialize_position`. Attaching to a
still-present actor (resume) reads nothing.

The character is placed at the first of:

1. a pending respawn (DEATH-0), consumed;
2. a pending arrival (NPC-0), consumed;
3. the last position, if it is valid;
4. the home temple once HOME-TOWN exists, else the pinned `entry_start`. A first login (no row)
   uses this step too.

Then:

- **Premium.** If the chosen tile is in a Premium area and Premium is not current, the
  PREMIUM-ACTIVATION-V1 §4.5 relocation applies (home temple, or Thais for a Premium home city;
  `entry_start` until HOME-TOWN exists).
- **Both pending rows.** A pending respawn and a pending arrival at once cannot arise (a death
  deletes the arrival). If both exist, admission fails closed and reports an integrity fault.
- **Valid** means: same World; the tile is walkable in the active bundle; and it is not a house
  tile. House tiles are invalid until HOUSE-OWN-0 decides entry; they place at step 4. The bundle
  digest is kept for diagnostics only; a matching digest does not skip the check, because durable
  ground items can block a tile.
- **Occupied or blocked tile:** the NPC-0 §6.1 fallback (nearest free walkable tile within 3,
  then step 4).
- After a planned world reset with a new bundle, positions are re-checked by these rules; nothing
  is migrated.

## 4. Rejected options

- **A column on the Character root with a revision advance.** A position changes constantly;
  advancing `CharacterRevision` for it would flood the receipt chain (DUR-02 rule 2).
- **A receipt per write (as stance does).** The position is a projection with no semantic value,
  and the latest row is all admission needs.
- **Storing the channel.** The base is shared by all channels of a World; the channel is an
  admission choice.
- **Writing on every step.** The periodic write bounds crash loss at a fraction of the cost.
- **Writing after terminal release.** The fence has already closed.

## 5. Decision test

- **Must decide now:** YES. Without it every login starts at `entry_start`, which is not how Tibia
  plays.
- **Minimum sufficient:** one table, one fenced upsert, one admission rule.
- **Superseding evidence:** a measured write cost that needs a longer period.
- **Deliberately not decided:** logout-block durations after a player kill (PARTY-PVP-0), house
  entry rules (HOUSE-OWN-0), channel transfer, the home town (HOME-TOWN).

## 6. Before-freeze checklist

1. **Contract amendments:** composition §3 (runtime-state projection class) and the scope matrix
   player position row, both in this PR. DEATH-0 and NPC-0 obligations keep precedence.
2. **Serialization:** the session fence and the write sequence per write; the final write inside
   terminal release.
3. **Restart:** at most 5 minutes of movement lost; placement is always defined.
4. **Typed references:** World, native tile position and markers; the bundle digest.
5. **Wire:** none.
6. **Split work:** one upsert per write.
