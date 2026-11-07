# Mechanics status v1

CL6 owner authorization (2026-11-06): additive read-only projections for C20c,
S13c, W09 and P19. Capability 384, command 669; mechanic 642, offer 639.
No domain, migration, new gameplay rule or durable state. Existing capabilities
and reply formats remain unchanged. Offered in bundle Worlds with native gameplay.
The native client opens Character effects with Ctrl+Alt+Shift+U and refreshes once
per second while open; statuses are observations, never settlement authority.

## Request

Empty. Unselected capability, malformed request, unavailable owner or stale
Character/session/connection/scope fence returns Rejected with empty payload.
Existing command ingress retains exact replies for duplicate CommandRefs; a new
query observes current owners. No target identifier is accepted from the client.

## Reply

Accepted payload is little-endian, at most 65536 bytes:

- i64 database observation Unix milliseconds, u32 current dust, u32 dust limit.
- u8 death count (0..5), newest first. Each: i64 committed Unix milliseconds,
  u8 consumed blessing count (0..32), then length-prefixed UTF-8 blessing keys.
  Consumption is the immutable receipt's before-minus-after, never a comparison
  of current blessings against an earlier read. Empty means no blessing consumed
  by that recorded death. Twist of Fate is explicitly displayed as consumed or
  not consumed for each receipt. No history is distinct from an empty consumption.
- u8 timer count (0..16), keys strictly sorted. Each: length-prefixed UTF-8
  production/effect key, u64 positive remaining milliseconds, u8 online-only flag.
  Wealth Duplex uses the existing online clock and latest activation. Other timed
  potions use active source-qualified installed conditions. Instant Antidote and
  Kooldown Aid have no active duration; their reuse cooldown is not an effect timer.
- u8 party member count (0..50). Each: 16-byte Character identity, UTF-8 name,
  u8 known flag. If known: u16 condition-payload byte length and an unchanged
  condition-status-v1 payload (including permanent -1 and exact sub-identities).
  Each member then carries five u32 remaining milliseconds (Train, Protect,
  Enchant, Heal, Enlighten Party). Then five u32 values: damage reduction, auto attack, spell damage, healing
  percentages, and additive Guiding Presence Mantra points. Unknown members have
  zero in all ten words.
  Unknown means no qualified live owner in this Channel, not no effects. An empty
  known condition list means no active effects. Empty roster means no party.

Text has a u8 byte length, 1..128, valid UTF-8 and no control characters. Flags
are exactly 0/1. Duplicate members, unsorted/duplicate consumed keys or timers,
zero timers, invalid condition payloads, dust above limit, future death timestamps,
truncation and trailing bytes are rejected. The limits bound all allocations.
Client state changes only after accepted, fully decoded replies; refresh errors
retain confirmed data with an error. Every session end clears the panel.

## Existing owners and observation boundary

Own durable values use the existing recovered authority, node and current item
fence in a bounded semantic transaction. Dust reads the actual balance owner,
including its documented absent-row default, never Forge history. Deaths read
committed receipts. Wealth Duplex reuses game_character_online_us.
Party membership comes from the World party owner's shared observation, under its
World advisory lock; no viewport, cached party shield or client-supplied target
confers membership. Live member sessions are qualified by current admission and
Character lease/scope, then resolved again under the actual runtime owner. Runtime
and spell-state guards are released before database awaits. Member conditions and
local potion conditions are sampled under their locks after the durable observation;
this is a bounded composition of owner observations, not a cross-owner transaction.
Remote/offline members retain their names but their conditions are unknown.

## Donor evidence and presentation differences

Crystal 201189f871d23597b82caf88fd611454a2ddefb8, OTS_HYPOTHESIS_ONLY:

- src/creatures/players/player.cpp:4212-4248: regular versus Twist of Fate
  consumption; src/server/network/protocol/protocolgame.cpp:5411-5480: blessings.
- src/server/network/protocol/protocolgame.cpp:5305-5314: active concoction IDs
  and time; data/libs/systems/concoctions.lua:126-152,179-199 and config.lua.dist:
  310-312: existing Wealth Duplex online clock and instantaneous Kooldown Aid.
- data/scripts/actions/items/potions.lua:1-31,52-63,125-147: installed timed potion
  conditions. Oteryn's existing condition_potions owner records exact source keys.
- data/scripts/spells/party/protect_party.lua:6-10,26-38 and sibling party spell
  scripts: existing member conditions. The accepted Enlighten Party duration
  difference remains documented in party-effects-v1.md; no duration is changed.
- src/creatures/monsters/monster.cpp:2765-2773: influenced 1..5, fiendish 15 with
  hidden stack count; protocolgame.cpp:3657-3676: actual dust and dust limit reads.

Canary 47dfd51f45280a59a1d3e50ba7edd573d7234446 is the companion audit donor;
its party.cpp:308-350 documents the separately owned Guiding Presence bonus.
The new wire groups existing observations in one bounded native query. Native
monster labels use original text glyphs (Influenced N / Fiendish) instead of
redistributing proprietary icons; fiendish never displays 15. TibiaWiki MediaWiki
Criaturas Modificadas revision400948 independently confirms both classifications.
No missing source or wiki check is used to infer a gameplay rule.
