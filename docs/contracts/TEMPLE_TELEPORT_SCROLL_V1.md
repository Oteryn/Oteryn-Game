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
4. No home temple resolves for the Character, or the temple landing is not possible
   now (unwalkable, closed door, occupied, or closed by the PvP block): `NOT_POSSIBLE`.
5. An Instance runtime scope, a dead actor, a pending or reserving spell, the shared
   item-use gate (one use in flight, `ITEMUSE0-RL-01`) or a reserved source: the
   existing native USE refusals.

The scroll has no cooldown of its own.

## Atomic consume

The native item-use writer burns exactly one scroll as `ItemUseCause` 481
`TempleScroll`, shape 1 (burn), with its one Type 2 `item_use` event, and inserts one
immutable `game_temple_scroll_uses` row in the same transaction under the holder's
current item fence. The CharacterRevision does not advance. One record per CommandRef
is kept forever: an exact replay answers from the record and its row, burns nothing and
teleports nothing; the same CommandRef with another request conflicts. Migration 2762
admits cause 481 only as a burn and only together with its row, and the row only with
its burn of a carried scroll of the same Character in the same transaction.

## Move to the temple

Only a known commit whose row says `expired = false` moves the actor. The Channel owner
then teleports the actor to the home temple (`home_temple`, the citizenship temple of the
World's hometown catalogue; the fixture start on a World without one) through the same
callback teleport proof the citizenship and quest teleports use, under the runtime and
spell-state guards in their established order. The position then persists through the
ordinary position saves. A landing that becomes impossible between PREPARE and the commit
(another creature stepped onto the temple cell) leaves the scroll consumed and the actor
in place; PREPARE's landing check makes that window one lock round trip.

## Seven-day deadline

The scroll's deadline is a TIMED deadline in the sense of TIMED-ITEM-0B §10.3 (database
time at which the item expires) with the `Expire {Deadline}` cause of RT-1c, fixed once at
delivery and never moved: no `SetDeadline`, `ClearDeadline` or `PutOut` follows it.

- Delivery. The daily reward claim that mints the scroll stamps its immutable
  `game_character_daily_reward_lines` row (kind `TEMPLE_SCROLL`, naming the minted
  instance) with `effect_started_at` = the daily effect clock (`game_daily_effect_now()`)
  and `effect_expires_at` = `effect_started_at` + 168 hours (604,800,000 ms, the
  `remaining_ms` ceiling of the TIMED rows). Migration 2762 admits exactly that interval
  for this kind and requires it on every newly delivered scroll line.
- Why not `game_item_timed_states`. Its write admission (D360) binds a deadline to a lit
  continuous item on a tile and requires a pinned timed definition; a carried scroll with
  one charge and an unchanging deadline has neither. The delivery line is already the
  immutable, per-instance, transactional record of the delivery, so the deadline is kept
  there and read by instance, not copied into a mutable row.
- Expiry. The deadline is applied by the item's owner when the holder next uses it: the
  insert of the `game_temple_scroll_uses` row stamps the daily effect clock and the
  instance's deadline and records `expired = (deadline <= clock)`. An expired scroll is
  burnt by the same atomic consume, the actor is not moved, the result is `COMMITTED` and
  the actor receives the status text `Your temple teleport scroll has expired.` The
  database computes `expired`; the caller cannot choose it.
- No deadline. A scroll without a delivered daily line (a scroll delivered before
  migration 2762, or minted by any other owner) carries no deadline and never expires
  under this contract.

Not in this contract: a background sweep that removes an expired scroll the holder never
touches again, in the Inbox, a depot or a container. Such a scroll stays visible until it
is used, and it can never teleport after its deadline. A sweep belongs to the RT-1c
deadline scheduler when that owner exists. The Inbox delivery itself is unchanged: a
scroll is used after it is moved to the carried tree.

## Persistence and rollback

Migration 2762 widens `daily_effect_interval` with the 168-hour scroll interval, adds
`daily_temple_scroll_deadline` (`NOT VALID`, so lines delivered before it keep their null
interval), indexes lines by item instance, admits cause 481 in
`game_item_use_records_cause_kind_check` and `game_item_use_cause_shape` (each previous
rule kept verbatim and widened), and creates `game_temple_scroll_uses` with its stamp, its
immutability and its deferred pairing checks. Applied migrations are immutable: a rollback
is a successor migration that stops new writes (the dispatch branch and the INSERT grant)
and keeps every record as evidence of a consumed scroll.

## Tests

Unit: the refusal order and dispositions, the cause and lifetime constants. PostgreSQL
(route-less: returns `Ok(())` without `OTERYN_TEST_POSTGRES_ADMIN_URL`), in
`crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/temple_scroll.rs`:
`temple_scroll_teleports_home_and_is_consumed`, `temple_scroll_refused_with_logout_block`
and `temple_scroll_expires_after_seven_days`.
