# House Podium V1

Capability 150 (`HOUSE_PODIUM_V1`) requires House V1 (49). It is not offered in production;
bundle Worlds only, and every mutation also requires the current session, node and house-scope
generation fences.

Display kinds are outfit `1`, mount `2` and boss `3`. Direction is `0..=3`. Keys are UTF-8,
`1..=160` bytes, prefixed with one length byte. The server accepts a key only when the content
owner knows it; numeric donor ids are never runtime identities.

## Query

Command 174 carries the podium item UUID (16 bytes).

## Set

Command 175 carries the item UUID (16 bytes), the expected revision (`u64`, big endian),
display kind, direction, key length and the key.

## Result

The reply to 175 is bounded (at most 9 bytes) and never exposes a display before its durable
write commits.

## Projection query

Command 176 carries the item UUID (16 bytes). Players other than the editor use it.

## Projection

Replies to 174 and 176 come from the same authoritative durable display read and are at most
4096 bytes.

Boss kind is reserved on the wire. It stays unavailable until an accepted boss-display item and
content mapping exist.
