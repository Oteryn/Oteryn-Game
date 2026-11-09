# Spellbook details V1

Capability 449 requires 287 and adds command 733. It is offered in bundle World
when native gameplay is active. Existing catalog snapshots and deltas are unchanged.
The query is read-only, bound to the admitted exact actor and current Game session.
The current spell book and the existing fenced Character known-spell reader supply
all facts; client labels and imported settings cannot supply eligibility.

## Request

Version byte `1`, page `u16`, vocation `u8`, premium filter `u8`, cooldown group
`u8`, minimum level `u32`, maximum level `u32`, then unprefixed UTF-8 search text.
Integers use little endian. Search is at most 64 bytes without control characters.
The request is 14–78 bytes. Pages are 0–31, each with at most 32 entries.
Maximum level zero means unlimited; otherwise minimum must not exceed maximum.
Vocation zero means any; 1–11 select none, druid, elder druid, sorcerer,
master sorcerer, knight, elite knight, paladin, royal paladin, monk, exalted monk.
Premium zero means any, one means free, two means premium. Group zero means any;
1–16 are existing server cooldown-group ordinals. Search matches name or words,
case-insensitively. Filters combine with AND. Out-of-range pages are refused;
page zero of an empty result is valid.

## Reply

At most 65,536 bytes: version byte `1`, page `u16`, matching total `u16`, catalog
byte length `u32`, that many bytes of the existing ActorSpellCatalogV1 codec,
then seven metadata bytes per entry, in the exact catalog order: vocation bitmask
`u16`, premium boolean `u8`, required rune magic level `u32`. Integers use little
endian. Vocation bits 0–10 correspond to request values 1–11; the mask is nonzero
and has no higher bits. Premium is strictly 0 or 1. Instant spells have magic
level zero. Catalog entry and total limits remain 1,024. Page entry count must
match the total and page size exactly. Trailing bytes are invalid.

All non-alias spells in the current book can appear, including other vocations;
learned and available remain projections of the current actor's actual state.
Availability describes eligibility, never target, pools, equipment or cooldown
readiness. Casting still goes through the existing authoritative cast protocol.
Each query observes current state independently; a page is not a durable snapshot
of all pages. No database writes or credentials are introduced.
