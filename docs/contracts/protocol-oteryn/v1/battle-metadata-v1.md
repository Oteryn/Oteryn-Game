# Visible battle filter metadata

O014 adds read-only command 736 to existing capability 17 ATTACK_V1; no capability is added.
Only currently visible, generation-matching players are enriched. The caller is fenced before
and after the read. Unknown, unavailable or stale facts remain unknown, never a guessed vocation
or a negative Guild relationship. No new persistent state is written.

## Request

A count byte (0–16), followed by that many 24-byte EntityRefs: 16 identity bytes and an unsigned,
nonzero, eight-byte big-endian generation. References must be distinct; trailing bytes are invalid.
Maximum request length: 385 bytes.

## Reply

A count byte followed by 26-byte rows. Each row has the same EntityRef, one vocation-family byte,
and one own-Guild relation byte. Vocation families are 0 untrained, 1 sorcerer, 2 druid, 3 paladin,
4 knight, 5 monk, and 255 unknown. Promotions retain their base family.
Guild relation is 0 unknown, 1 outside the viewer's Guild, 2 same Guild. A viewer without a Guild
has no same-Guild players. Other Guild IDs, ranks and membership are never disclosed. The same
fenced native Guild reader used for the viewer's own roster authorizes this comparison.
Maximum reply length: 417 bytes. Duplicate references, unknown codes and trailing bytes are invalid.
