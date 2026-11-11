# House Podium V1

Capability 150 (`HOUSE_PODIUM_V1`) requires House V1 (49). The composed native authority
offers it (mechanic `house_podium`, W18); a fixture World never does. Every command also
requires the current House session, node and house-scope generation fences.

The podiums are the catalogue items renown `oteryn:item.tibia.i35973`/`i35974`, vigour
`i38707` and tenacity `i42367`/`i42368` (Crystal 201189f8 `data/items/items.xml:67245-67252,
71414-71416, 75107-75114`; migration 0561). Display kinds are outfit `1`
(`oteryn:content.outfit.*`), mount `2` (`oteryn:content.mount.*`) and creature `3`
(`oteryn:creature.*`). Renown shows an outfit or a mount the Character may wear (starter or
earned; Crystal `game.cpp:4603-4636, 4692-4699`); vigour shows a Bosstiary boss and tenacity a
Bestiary race (`game.cpp:11149`). Direction is `0..=3`. Keys are UTF-8, `1..=160` bytes,
prefixed with one length byte. The server accepts a key only when the content owner knows it;
numeric donor ids are never runtime identities.

## Query

Command 174 carries the podium item UUID (16 bytes).

## Set

Command 175 carries the item UUID (16 bytes), the expected revision (`u64`, big endian),
display kind, direction, key length and the key.

## Result

The reply to 175 is `0` and the committed revision (`u64`, big endian), or one refusal byte:
`1` stale session, `2` no House access, `3` not an item of this House, `4` not a podium, `5`
display kind not shown by this podium, `6` key unknown to content, `7` outfit or mount not
owned, `8` revision changed, `9` invalid input. It is at most 9 bytes and never exposes a
display before its durable write commits.

## Projection query

Command 176 carries the item UUID (16 bytes). Players other than the editor use it.

## Projection

Replies to 174 and 176 come from the same authoritative durable display read under the current
House session fence: `0` for an empty podium, or `1`, podium kind (`1` renown, `2` vigour, `3`
tenacity), display kind, direction, revision (`u64`, big endian), key length and key (at most
174 bytes, within the 4096-byte bound).

An unknown command payload ends the connection; a missing House session or an unavailable
store ends the command without a reply.

## Visibility flags (capability 489)

Capability 489 `HOUSE_PODIUM_FLAGS_V1` requires 150. It follows Crystal's
podium flags `PODIUM_SHOW_PLATFORM`, `PODIUM_SHOW_OUTFIT` and
`PODIUM_SHOW_MOUNT`. When a session selected it:

- command 175 is the V1 request followed by `show_platform:u8, show_outfit:u8,
  show_mount:u8`, each exactly 0 or 1; a request without them is rejected;
- the replies to commands 174 and 176 for a shown display are the V1 projection
  followed by the same three bytes; an empty podium stays `0`.

The flags are stored with the direction in the display variant (no new
storage). A display stored without flags, including every display set by a
session without 489, reads as all three shown. Flags are presentation only:
they grant no ownership and do not change the content or ownership checks.
Sessions without 489 send and receive byte-identical V1 payloads. The command
175 registry bound is 190 bytes: a V1 request is at most 187 bytes and the
flags add 3.
