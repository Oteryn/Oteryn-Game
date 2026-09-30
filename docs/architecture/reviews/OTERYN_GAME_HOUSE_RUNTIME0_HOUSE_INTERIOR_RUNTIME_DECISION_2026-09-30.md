# HOUSE-RUNTIME-0 House interior runtime

- Decision: `HOUSE-RUNTIME0-HOUSE-INTERIOR-RUNTIME-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (security,
  persistence, protocol) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the owner's direction (2026-09-30, verbatim: "zacznij 1,2,3,4,5,7,6,8,9,10,11", item 1
  being this decision); the house interior runtime that HOUSE-CUSTODY-0 §3.4 and §4, HOUSE-OWN-0
  (its implementation brief and §10) and GUILD-0 §7.5 wait for; ADR-0001 §11's open house topology
- Builds on: EXP-HOUSES-01 (owner-accepted) §4.1, §5.1-§5.4, §14, §16, §17, §19 and §20;
  HOUSE-CUSTODY-0 (`HouseInterior`, provenance, §3.4 direction, §3.5 closure); HOUSE-OWN-0
  (property, tiles, ACL lists, disposition fence); GUILD-0 (#1395: guild ACL entries, guildhall
  owner rights, depot locker); ADR-0001 §10-§12; ADR-0003 (admission on every transition);
  migrations `0001`, `0003`, `0006` (runtime scope kind 2, scope assignments); CHAR-POSITION-0 §3;
  ATTACK-0 (in-fight deadline); MAP-WIRE-1 (snapshots, map item handles); DEPOT-0; the Tibia
  manual `houses.md`; owner rule 5905825574 (Global parity)
- Amends, pending on acceptance of HOUSE-RUNTIME-0, in this PR: ADR-0001 §11 (house topology
  pointer); HOUSE-CUSTODY-0 §3.4 (the direction becomes binding here) and §4; CHAR-POSITION-0 §3.2
  and §3.3 (house scope position); HOUSE-OWN-0 §11 (`kick` and `leave`).
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| SCOPE-HANDOFF-1 | hard, security and durability review | the session transition between a Channel scope and a house scope (§4): the house scope kind in scope assignments and sessions (the migration HOUSE-CUSTODY-0 §3.4 names), origin routing metadata, the transition protocol, refusal rules; built so the later channel change reuses it | this decision |
| HOUSE-RUNTIME-1 | hard, security and persistence review | the interior runtime (§3, §5-§8): activation and unload, presence and movement on house tiles, doors and ACL checks, kick and leave, revalidation, the three `HouseInterior` transfer shapes with provenance, the disposition quiesce, login into a house | SCOPE-HANDOFF-1; HOUSE-CUSTODY-1; HOUSE-1 and HOUSE-ACL-1 (for owned houses) |
| HOUSE-VIEW-1 | impl, protocol review | house scope snapshots and deltas on MAP-WIRE-1; the read-only projection of `HouseInterior` items into Channel views (windows) (§9) | HOUSE-RUNTIME-1; MAP-WIRE-1 children |

Order: SCOPE-HANDOFF-1 and HOUSE-CUSTODY-1 first; HOUSE-RUNTIME-1 can land before HOUSE-1 and be
tested on an operator-owned test house (§10); HOUSE-1 then opens auctions (HOUSE-CUSTODY-0 §4),
GUILDHALL-1 opens guildhalls. Later, each with its own decision: beds, Rested and offline training
(the EXP-HOUSES-01 §18 numbers); Residence; containers placed in houses (HOUSE-CUSTODY-0 §3.1
admits only items without contents).

## 1. Question

Where does a house interior run, how does a character enter and leave it, who may do what inside,
and how do house items change?

## 2. Facts

**PROVEN**

- EXP-HOUSES-01 (owner-accepted): §5.1 one logical authoritative World-scoped interior runtime
  per active house, so characters from different Channels converge on one presence and item
  state; §5.2 instance primitives may be reused, `HouseId` stays the identity; §5.3 entry is an
  explicit handoff that keeps origin-Channel routing, and exit returns through it; entry and exit
  cannot bypass combat restrictions, admission, fencing, protected transactions or duplicate
  session prevention; §5.4 generation fencing on recovery; §16 OWNER, SUBOWNER, GUEST with
  revalidation at entry and mutation boundaries; §17 only a GUI edits access; §19 one authoritative
  location, nothing destroyed.
- ADR-0001 §10: a channel change is a session transition (checkpoint, close, fresh admission,
  fresh `GameSessionId`); §11 leaves the house topology to a house decision; §12 forbids changing
  channel through a shared house or instance.
- Migrations `0001`, `0003`, `0006`: sessions and scope assignments already carry
  `runtime_scope_kind` 1 (Channel) and 2 (Instance, with an instance id). No channel change or
  instance entry is implemented.
- HOUSE-CUSTODY-0: `HouseInterior` and provenance tables with no runtime grant (§3.5); §3.4 gives
  as direction a house scope reusing the Instance kind, and three shapes (inventory or equipment
  to house, house to inventory, tile to tile).
- HOUSE-OWN-0: property rows, ACL lists with `acl_revision`, a disposition fence enforced by a
  trigger, `HOUSEOWN0-RL-14` 2,000 items per interior; "the panel works anywhere".
- CHAR-POSITION-0 §3.2: no last-position write in an instance scope; the row keeps the last
  position outside.
- The House catalogue: tiles, doors (a door is its house key and position) and `entrance`, the tile
  a character is placed on when leaving or being moved out.

**CIPSOFT_OFFICIAL** (the Tibia manual `houses.md`)

- A house interior is a protection zone: no attacks, and no health or mana regeneration.
- Only invited characters enter; door rights restrict inner doors; the owner always passes.
- Owner or subowner kicks a character; anyone can leave; a kicked character goes outside.
- Guildhalls have a depot locker; any invitee uses its own depot, Inbox and the Market there.

## 3. Topology (HOUSE-RUNTIME-1)

- **House scope.** Each active house is one runtime scope of kind 2 whose key is the `HouseId`
  (the `\x02`-tagged `scope_key` of HOUSE-CUSTODY-0 §3.4), hosted by one node of the World and
  fenced by its scope ownership generation. The instance id is derived from the `HouseId`; it is
  runtime identity only (EXP-HOUSES-01 §5.2).
- **Activation.** The first entry, or a login into the house (§6.3), acquires the assignment on
  the node that runs the entering character's Channel if it has capacity, else any node of the
  World. Only one assignment is live; a second activation waits for the first.
- **Unload.** With no character inside for `HOUSERT0-RL-01` (5 minutes), the runtime releases the
  assignment. Items are durable; nothing is lost.
- **Map.** The house scope's map is the house's tiles from the active bundle, with their walls and
  doors, plus the `HouseInterior` items. Channels treat house tiles as not enterable: the only way
  in is a door handoff (§4).
- **Rules inside.** The interior is a protection zone (no attack, no regeneration, as the
  manual). No creature exists there. Summons are dismissed at entry.

## 4. Entry and exit (SCOPE-HANDOFF-1)

### 4.1 Entry

- A character moves onto a house's front door tile from outside (the door position in the
  catalogue), or uses the door.
- The Channel runtime checks, at the current `acl_revision`:
  - access (§5.1);
  - no ATTACK-0 in-fight deadline, PZ lock or combat lock; no direct trade, unresolved item
    mutation or pending operation (ADR-0001 §10);
  - the house is not in `DISPOSITION`.
- A refusal leaves the character outside with a typed reason (`NO_ACCESS`, `IN_COMBAT`, `BUSY`,
  `HOUSE_CLOSED`); nothing is written.
- The Channel check is a pre-check only. The admission commit (below) re-reads, in its own
  transaction and under a FOR SHARE lock on the property row, which every content-fence and ACL
  write updates (HOUSE-OWN-0 §9, §10), the property state, the content fence, and the exact
  `acl_revision` the pre-check used. When the grant comes from a guild entry (GUILD-0 §10), the
  same transaction also takes FOR SHARE on each guild row whose revision the grant depends on and
  on the character's membership row, and re-reads that exact guild revision. Architect ruling: a
  membership change that can end a guild entry's grant advances the guild revision under FOR
  UPDATE on the guild row (§5.4), so it conflicts with these locks. Any difference refuses
  (`NO_ACCESS` or `HOUSE_CLOSED`) and the character stays in its source session. So a
  revocation, a membership change or a disposition either commits first and the admission
  refuses, or commits after and finds the character inside (§5.4, §7). Every admission into a
  house scope, including login (§6.3), uses this commit.
- On success the transition runs ADR-0001 §10 (checkpoint, close the Channel session, fresh
  admission into the house scope, fresh `GameSessionId`) as a recoverable handoff:
  1. **Prepare.** One transaction records a handoff row (CharacterId, source `GameSessionId`,
     destination `HouseId` and scope generation, origin ChannelId) and reserves the destination
     while the source session stays live and fenced; the character takes no further action in
     the source.
  2. **Commit.** One transaction, with the revalidation above, makes the source session terminal
     and admits the destination session with its fresh `GameSessionId`, the origin Channel
     recorded on it, and marks the handoff committed. Where source and destination cannot share
     one transaction, the durable handoff row is the commit point and each side reconciles to it;
     the source never becomes terminal before that point.
  3. **Abort.** A refusal or failure before the commit deletes the reservation and resumes the
     source session; nothing else is written.

  A crash at any point leaves the character in exactly one admitted scope: before the commit
  point the source, after it the destination. Restart reconciles every open handoff row to that
  outcome before either scope admits or resumes the character (§4.3). The character is placed on
  the tile inside the door.
- The client receives a snapshot (MAP-WIRE-1's teleport trigger). The player sees one walk
  through a door; the transition target is `HOUSERT0-RL-02` (500 ms at the 99th percentile),
  measured before activation.

### 4.2 Exit

- Walking out through the front door, leaving (§5.3), a kick, a revocation (§5.4), a property
  transfer, a disposition (§7) or a house scope shutdown moves the character to the house's
  `entrance` tile on the recorded origin Channel, through the same transition (§4.1) in reverse.
- Every exit's commit transaction also writes the last-position row (CHAR-POSITION-0 §3.2): it
  clears the house columns (§6.3) and records the tile actually placed under the destination
  session's order key. The destination Channel resolves that tile at the commit: the `entrance`
  if it is free and walkable, otherwise the CHAR-POSITION-0 §3.3 fallback (the nearest free
  walkable tile within 3, in that order). An exit is never refused for an occupied or blocked
  entrance. A disconnect right after any exit therefore never returns the character inside.
- If the origin Channel is draining or unavailable, admission picks another Channel of the World
  under its normal rules (EXP-HOUSES-01 §5.3 fallback). A house is never a way to change channel
  by choice: the exit always targets the origin first (ADR-0001 §12).

### 4.3 Recovery

- A crash of the house node follows the ordinary reconnect and recovery rules for sessions in scope
  kind 2; a new assignment has a higher generation, so the old runtime writes nothing.
- An open handoff row (§4.1, either direction) is reconciled at restart before the character is
  admitted or resumed anywhere: committed, the destination holds; not committed, the source holds
  and the reservation is released.
- A character whose house scope cannot be recovered is placed at the `entrance` on its origin
  Channel at the next admission, and that admission transaction clears the house columns of the
  last-position row and writes the entrance position, exactly as for an exit (§4.2, §6.3), so a
  disconnect before the next periodic or terminal write cannot leave the stale house position
  authoritative.

## 5. Access inside (HOUSE-RUNTIME-1)

### 5.1 Who may enter

- The owner (for a guildhall, the guild's level-1 member, GUILD-0 §7.3), subowners and guests,
  including guild entries (GUILD-0 §10), and minus exclusions.
- Inner doors with a door list admit only that list and the owner (the manual).
- A house with no owner admits nobody but an operator test (§10).

### 5.2 What each role may do

| Action | Owner | Subowner | Guest |
|---|---|---|---|
| Enter, move, use objects | yes | yes | yes |
| Place an item from own inventory or equipment | yes | yes | yes |
| Move an item between tiles of the house | yes | yes | yes |
| Take an item into own inventory | any item | any item | only items it placed |
| Kick a character | yes | guests only | no |

- "Placed" means the item's provenance names the acting character (HOUSE-CUSTODY-0 §3.2). Taking
  or moving an item never changes who may reclaim it at a disposition, except that taking it out
  ends its provenance, as HOUSE-CUSTODY-0 §3.2 says.
- Guests taking others' items is a declared difference from Tibia: EXP-HOUSES-01 §16.3 grants a
  guest only explicit capabilities, and storage permission never implies authority over items
  placed by somebody else (§16.4).

### 5.3 Leave

Any character may leave at will: walking out, or `leave` in the House Management panel.

### 5.4 Revalidation

- The runtime reads the ACL at its `acl_revision`; a new revision triggers a re-check of every
  character inside; one that lost access is moved out (§4.2). A membership change that ends a guild
  entry's grant counts the same way (GUILD-0 §10), checked when the guild's revision changes; that
  change advances the guild revision in its own transaction under FOR UPDATE on the guild row, so
  it serializes with admission (§4.1).
- Every item write re-checks the role at the write (EXP-HOUSES-01 §16.5). Architect ruling: the item
  transaction takes FOR SHARE on the property row and, for a role derived from a guild entry, on the
  granting guild and membership rows (the §4.1 admission locks), and re-reads the exact `acl_revision`
  and guild revisions the authorization used; any change refuses the write with a typed reason
  and nothing is moved, so a revocation cannot commit between the role check and the item commit.

### 5.5 Kick

`kick {character}` in the House Management panel (HOUSE-WIRE-1 gains the variant; no spell,
EXP-HOUSES-01 §17). It needs the kicker to be inside the house or to own it.

## 6. Items and positions (HOUSE-RUNTIME-1)

### 6.1 Transfer shapes

The three HOUSE-CUSTODY-0 §3.4 shapes are binding, each one item and one transaction with its
provenance, run by the house runtime under its generation fence and the acting character's
`character_root` lock. The same transaction also takes the acting character's normal session fence
(composition decision rule 2: its reconnect-session row with the current `GameSessionId`, lease
generation and session generation, and the runtime-scope assignment of this house scope). Both
fences are checked in the one commit, so a command from an older session of the character writes
nothing even when the house scope generation is still current:

- from `CharacterInventory` or `CharacterEquipment` to `HouseInterior`;
- from `HouseInterior` to `CharacterInventory`;
- between tiles of the same house.

Each refuses: a tile not of this house; `HOUSEOWN0-RL-14` reached (`HOUSE_STORAGE_FULL`); a role
without the right (§5.2); a set disposition fence (the database trigger refuses it too); an item
with contents (HOUSE-CUSTODY-0 §3.1). Stack merge and split, containers and cross-house moves stay
out. The runtime role gets EXECUTE on the shape functions only, which is how HOUSE-CUSTODY-0 §3.5's
closure opens.

### 6.2 Map items and use

House tiles carry map-authored items (doors, the depot locker, beds, decorations) that are never
pickupable (ADR-0021 §4.4). `USE` follows ITEM-USE-0 inside the house scope. The depot locker opens
the character's own depot for the house's town (DEPOT-0; GUILD-0 §7.5). Beds are inert until the
beds decision.

### 6.3 Logout and login

- **Position.** In a house scope the last-position row (CHAR-POSITION-0) records the `HouseId` and
  the tile inside, in two new nullable columns, instead of skipping the write. Every exit clears
  them in its commit (§4.2).
- **Login.** When the row names a house, admission into the house scope runs the §4.1 admission
  commit: access and the property state are revalidated under the same property row, ACL and guild
  revision locks, so a concurrent revocation, membership change or disposition either commits
  first and the login refuses, or commits after and finds the character inside (§5.4, §7). The
  saved tile is revalidated under the active bundle and the current `HouseInterior` items: it must
  be a tile of this house, walkable, not blocked by an item and not occupied. The house runtime,
  the single writer per `HouseId`, selects and reserves the tile, so item commands and other
  actor placements serialize with it; the placement commit revalidates the reserved tile and, if
  it was taken, repeats the selection. If it is not valid, the
  CHAR-POSITION-0 §3.3 fallback applies inside the house (the nearest free walkable tile of the same
  house within 3, in that fallback's order); if none, the house is refused as below. On success the
  character is admitted into the house scope at the chosen tile (activating it if needed), with
  the Channel admission chooses recorded as origin.
- **Rejected house position.** If access, the property state or the tile fallback refuses, the
  character is placed at the house's `entrance` (or its §4.2 fallback tile) on a Channel admission
  chooses, and that admission transaction clears the house columns and writes the tile actually
  chosen under the new session's order key, as every exit does (§4.2). A later change of access or owner never returns the character to
  the stale interior tile. As in Tibia, a character logs in where it logged out when it still
  may.

## 7. Disposition quiesce (HOUSE-RUNTIME-1)

When HOUSE-OWN-0 §7 sets the content fence, the runtime moves every character out (§4.2) and
refuses new entries (`HOUSE_CLOSED`). The disposition steps then run with no one inside; each step
that changes `HouseInterior` advances the house content revision in its own transaction (§9).
Release (the state `VACANT`) reopens entry for the next owner.

## 8. Rows (registered by the children before implementation)

| Row | Value |
|---|---|
| `HOUSERT0-RL-01` idle unload | 5 minutes with nobody inside |
| `HOUSERT0-RL-02` door transition target | 500 ms at the 99th percentile, measured before activation |
| `HOUSERT0-RL-03` characters inside one house | 200 (`PARITY_PENDING`: Tibia has no stated limit) |
| `HOUSERT0-RL-04` active house scopes per node | set from measurement before activation |
| Item placement or take | 1 item, 1 location line, 1 provenance write or delete, 1 event |

## 9. Views (HOUSE-VIEW-1)

- **Inside.** House scope snapshots and deltas use MAP-WIRE-1 unchanged: the house tiles, walls,
  doors and `HouseInterior` items as map items with their handles.
- **Outside.** Channels project the `HouseInterior` items of houses in view, read-only, from a house
  content revision. Every authoritative `HouseInterior` mutation advances it in the same
  transaction: the three shapes (§6.1), each HOUSE-OWN-0 disposition job step (§7), and any write
  made while the house runtime is unloaded. A mutation that does not advance it is refused, so a
  vacant house never keeps a stale projection. So furniture is seen through
  windows, as in Tibia. Characters inside a house are not seen from outside (declared difference:
  one presence per house cannot be shown in every Channel).

## 10. Test house

Before HOUSE-1, HOUSE-RUNTIME-1 is tested on one house that an operator fixture marks owned by a
test Character in a test World only (never a production World). The fixture is test code, not a
migration.

## 11. Rejected options

- **House interiors as ordinary Channel tiles.** Each Channel would have its own presence and
  item copy (EXP-HOUSES-01 §4.2, §5.1).
- **A dedicated home Channel.** It is a Channel change by the back door (ADR-0001 §12).
- **Showing inside characters in every Channel.** Presence would need a cross-Channel projection
  of moving actors; items are enough to keep houses readable from outside.
- **Guests take any item.** EXP-HOUSES-01 §16.3 and §16.4.
- **Always logging in at the entrance.** Tibia returns the character inside when it still has
  access; the admission check makes that safe.

## 12. Owner-rule applications (Global parity, 5905825574)

Kept as in Tibia: the door is the way in; invited characters only; door rights; kick and leave;
the interior is a protection zone without regeneration; login where one logged out; furniture seen
through windows; the guildhall depot locker. Declared differences: guests take only their own
items; characters inside are not seen from outside; a bounded number of characters per house.

## 13. Decision test

- **Must decide now:** YES. House auctions, guildhalls and every house item wait for it, and the
  owner put it first.
- **Minimum sufficient:** one house scope per active house on the existing Instance kind; one
  transition used both ways; three item shapes; one read-only outside projection.
- **Superseding evidence:** measured transition latency above `HOUSERT0-RL-02`; the channel
  change decision may generalize SCOPE-HANDOFF-1.
- **Deliberately not decided:** beds, Rested, offline training, Residence, containers in houses,
  cross-house moves.

## 14. Before-freeze checklist

1. **Contract amendments:** ADR-0001 §11 pointer; HOUSE-CUSTODY-0 §3.4 and §4; CHAR-POSITION-0
   §3.2 and §3.3; HOUSE-OWN-0 §11, each written "pending on acceptance of HOUSE-RUNTIME-0".
2. **Serialization:** one live generation per `HouseId`; item shapes under the house fence, the
   acting character's session fence and its `character_root` lock; every admission, login
   included, revalidates access and property state under a lock on the property row and on the
   granting guild and membership rows; every `HouseInterior` mutation advances the content
   revision in its transaction.
3. **Restart:** items and positions are durable; a lost house scope sends characters to the
   entrance; transitions are fresh admissions through a durable handoff reconciled at restart;
   every exit, and every login that rejects a saved house position, commits the outside position
   actually placed (entrance with the §3.3 fallback, never refused); a saved interior tile is
   reserved by the house runtime and revalidated at the placement commit with a deterministic
   fallback.
4. **Typed references:** HouseId, WorldId, ChannelId (origin), CharacterId, GameSessionId, scope
   generation.
5. **Wire:** MAP-WIRE-1 snapshots; HOUSE-WIRE-1 gains `kick` and `leave` (HOUSE-OWN-0 §11 pointer).
6. **Split work:** one item per transaction; one character per transition.
