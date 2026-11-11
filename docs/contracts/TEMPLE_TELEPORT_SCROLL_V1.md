# Temple Teleport Scroll use and seven-day deadline (WU26-06)

Owner requirement: Winter Update 2026 daily reward rework, unit WU26-06
(`docs/agents/evidence/winter-update-2026/daily-reward-rework.md`, directed
2026-11-11). Reference: English TibiaWiki "Temple Teleport Scroll" and the official
Winter Update 2026 test-server thread 565, which call the daily scroll temporary. The
Channel owner keeps the actor's position, fight and zone facts; the native item-use
writer keeps the item. WorldId and ChannelId stay distinct, and every existing
session-generation, recovery, lease and scope fence stays in force.

## Item and wire

The scroll is item 25718 (`oteryn:item.tibia.i25718`, definition-r1), not stackable,
one charge. It is used with the existing command 2 USE under capability 15 from the
carried backpack tree, the same source the other native uses accept. No command,
capability, codec or resource ceiling is added: commands 6200-6209 stay unused by this
unit. Refusals use the existing `UseDisposition` values; a session without capability 11
receives `REJECTED` for `NOT_POSSIBLE` and `PZ_BLOCKED`, as the connection already
negotiates.

## PREPARE refusals

Each refusal burns nothing, moves nothing and writes nothing:

1. The actor's cell is NOLOGOUT (OTBM bit 3, `ZONES_NOLOGOUT_V1.md`), or its zone
   evidence is unreadable: `NOT_POSSIBLE`. The scroll never leaves a cell that the
   logout owner would refuse.
2. A logout block: the predicate of `LOGOUT_INTENT` refuses (a live PvP lock, or the
   PvE in-fight deadline, outside a qualified protection zone) after the durable PvP
   locks are refreshed: `NOT_POSSIBLE`. An unreadable lock row refuses (`REJECTED`).
3. A protection-zone block (the live PvP lock blocks protection-zone entry):
   `PZ_BLOCKED`.
4. No home temple resolves for the Character, or no temple landing (below) admits the
   actor now: `NOT_POSSIBLE`.
5. An Instance runtime scope, a dead actor, a pending or reserving spell, the shared
   item-use gate (one use in flight, `ITEMUSE0-RL-01`) or a reserved source: the
   existing native USE refusals.

The scroll has no cooldown of its own.

## Atomic consume

The native item-use writer burns exactly one scroll as `ItemUseCause` 481
`TempleScroll`, shape 1 (burn), with its one Type 2 `item_use` event, and inserts one
immutable `game_temple_scroll_uses` row in the same transaction under the holder's
current item fence, and moves the actor (below) before that transaction commits. The
CharacterRevision does not advance. One record per CommandRef
is kept forever: an exact replay answers from the record and its row, burns nothing and
teleports nothing; the same CommandRef with another request conflicts. Migration 2762
admits cause 481 only as a burn and only together with its row, and the row only with
its burn of a carried scroll of the same Character in the same transaction.

## Move to the temple

The landing is resolved as Crystal resolves a temple teleport: the home temple cell
(`home_temple`, the citizenship temple of the World's hometown catalogue; the fixture start
on a World without one) and, when it is taken, the closest free cell around it, in
Crystal's placement order (`Map::placeCreature`'s normal relative list, the
closest-free-tile search): north-west, north, north-east, west, east, south-west, south,
south-east, on the temple's floor. A landing admits the actor when the callback teleport
proof the citizenship and quest teleports use accepts it: walkable, no closed door, not
occupied, not closed by the PvP block.

The move is part of the consume. After the use row is stamped and before COMMIT,
the writer calls the Channel owner, which takes the runtime and spell-state guards in their
established order and teleports the actor to the first admitting landing. When no landing
admits the actor at that moment (every candidate was taken since PREPARE), the transaction
rolls back: the scroll is not consumed, no record or row exists and the result is
`NOT_POSSIBLE`. A scroll is therefore never consumed without the move. The position then
persists through the ordinary position saves. The move precedes COMMIT, so a COMMIT that
fails or whose outcome is unknown can leave the actor at the temple with the scroll kept;
an unknown outcome reserves the scroll as unspendable until reconciliation, as every native
use does.

## Seven-day deadline

The scroll expires through the game's existing Item expiration owner, the absolute Item
deadline owner (`game_spell_item_temporal_schedules`, drained by
`drain_due_items_in_transaction` on the Channel's Item deadline pass, about once a second),
which already makes spell- and action-minted Items with a duration vanish. The scroll is a
third original source of that owner; no new clock, sweep or table is introduced.

- Delivery. The daily reward claim that mints the scroll into the Inbox inserts, in the
  same transaction, the scroll's schedule: `duration_millis` = 604,800,000 (168 hours),
  `expires_at_unix_ms` = the transaction's database start + that duration, no decay target
  (the scroll vanishes rather than transforms), `source_transaction_id` = the claim and
  `source_ordinal` = 1. The deferred schedule proof (migration 2762) admits exactly that
  row: the live scroll of definition `definition-r1`, its `TEMPLE_SCROLL` delivery line
  delivered by that claim, the exact duration, deadline and content digest, and the current
  transaction. The deadline is fixed once and never moved.
- Why not `game_item_timed_states`. Its write admission (D360) binds a deadline to a lit
  continuous item on a tile and requires a pinned timed definition, and the item loop
  refuses items with a timed row; a carried scroll has neither.
- Movement. The scroll moves with its immutable schedule through the item loop (Inbox
  retrieval, carried tree, depot), as an action-minted Item does; every other scheduled
  Item keeps its movement guard. Owners that refuse every scheduled Item (supply stash,
  House containers, market) refuse the scroll too.
- Expiry applies in all custodies. When the deadline has passed, one Item deadline pass
  removes the scroll wherever it is, for an online or an offline Character, exactly as it
  removes every other expiring Item: one immutable receipt, predecessor capture, audit
  envelope, the Item retired (`lifecycle` 2, quantity 0) and its custody removed. Location
  never decides whether a due scroll expires:
  - Ground, and the carried tree (main backpack and nested containers) and equipment of a
    Character whose session is live in the passing Channel: the pass's existing session-bound
    removal (receipt custody 1, 2, 3).
  - Every other custody the scroll can occupy, through the one seam
    `game_temple_scroll_expire_due` (migration 2762) on the same pass, for this scroll only,
    under no session: the Character Inbox root (custody 4), a depot box or locker (custody 5,
    the box's committed count decremented as a depot transfer does), a House floor tile
    (custody 6, its reclaim provenance removed with it), a corpse lying on Ground (custody 7),
    a container rooted in the Inbox, a depot, Ground, a House tile or a backpack dropped at
    death (custody 2), and the carried tree (custody 2) and equipment (custody 3, the
    equipment revision advanced) of an offline Character. An offline Character's scroll does
    not wait for login. Ground-rooted custody (corpses, containers on Ground) is retired by the
    pass of the Channel that owns that Ground.
  - The seam's window holds only scrolls it may retire, earliest deadline first, so due scrolls
    the session-bound removal owns never hold back stored, offline or world scrolls. A scroll
    whose removal a native guard refuses at that moment (a House under disposition, a busy
    lock) stays for the next pass without stopping the others.
  - Bazaar custody is never changed: a due scroll in bazaar escrow, or of a Character locked for
    a bazaar transfer, stays until the transfer completes or is cancelled, and the next pass
    after that removes it through the offline-Character path.
  - The Supply Stash holds stackable balances only and refuses every scheduled Item, as do
    House containers and the market, so the scroll can never be in them.

  No status text is sent; the Item simply disappears.
- Defensive refusal. A use racing the deadline pass (the scroll is due but not yet
  removed) is refused: the writer reads the schedule under the scroll's row lock and
  returns `NOT_POSSIBLE` with nothing burnt, written or moved. The use row's stamp records
  the schedule's deadline and the database clock, and its CHECK (`observed < expires`)
  refuses a burn that crosses the deadline after that read.
- No deadline. A scroll delivered before migration 2762 has no schedule, carries no
  deadline and never expires under this contract.

## Persistence and rollback

Migration 2762 indexes daily lines by item instance; admits cause 481 in
`game_item_use_records_cause_kind_check` and `game_item_use_cause_shape` (each previous
rule kept verbatim and widened); creates `game_temple_scroll_uses` with its stamp, its
immutability and its deferred pairing checks; admits the scroll's schedule provenance in
`game_spell_item_temporal_schedule_proven`, its movement in `game_item_loop_line_admit`,
the session-less scroll receipt (custody 2 to 7 with no session; 2, 6 and 7 also without a
Character) in both receipt CHECKs, its predecessor capture in
`game_spell_item_temporal_capture`, its outcome in `game_spell_item_temporal_receipt_proven`
and the Inbox, depot, House tile and corpse entry removals in `game_character_inbox_remove`,
`game_character_depot_remove`, `game_house_location_proven` and
`game_item_corpse_entry_removal_proven`, each as a prepended arm that keeps every previous
arm; and creates the seam `game_temple_scroll_expire_due` with its custody resolver
`game_temple_scroll_custody`.
Applied migrations are immutable: a rollback is a successor migration that stops new
writes (the dispatch branch, the claim's schedule insert and the INSERT grant) and keeps
every record as evidence of a consumed or expired scroll.

## Tests

Unit: the refusal order and dispositions, the landing order, the cause, revision and
lifetime constants (the lifetime fits the deadline owner's `duration_millis`). PostgreSQL
(route-less: returns `Ok(())` without `OTERYN_TEST_POSTGRES_ADMIN_URL`), in
`crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/temple_scroll.rs`:
`temple_scroll_teleports_home_and_is_consumed`, `temple_scroll_refused_with_logout_block`,
`temple_scroll_kept_when_the_temple_landing_is_taken_at_commit` (the writer with every
landing occupied at commit: nothing is burnt or written) and
`temple_scroll_vanishes_through_the_item_deadline_owner` (online: a due scroll's use is
refused with nothing written; one Item deadline pass then retires the due carried scroll
under the session and the due scrolls in the Inbox, in a container in the Inbox and in a
depot box under no session, receipt custody kinds 2, 2, 4 and 5, and removes their custody
and the depot box count, while the fresh scroll stays and still teleports) and
`temple_scroll_vanishes_in_every_custody_of_an_offline_character` (offline: one pass
retires the due scrolls carried, worn, in the Inbox, in a container in the Inbox, in a depot
box, on a House tile, in a corpse and in a backpack on Ground, receipt custody kinds 2, 2, 2,
3, 4, 5, 6 and 7, all under no session) and
`temple_scroll_backlog_of_online_scrolls_never_holds_back_stored_ones` (501 earlier-due
scrolls carried by the live Character do not fill the seam's window: a one-scroll seam call
retires a stored scroll and leaves them to the session-bound pass).
