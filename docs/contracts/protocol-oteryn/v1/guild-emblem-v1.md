# Guild emblem V1

Optional capability 509 requires visibility 6. It carries the observer-relative
guild emblem Tibia Global draws beside a player, in the order of the five icons
of Tibia's `/images/creatures/hud/flags/guild.png`: 1 green (ally), 2 red
(enemy), 3 blue (neutral), 4 member, 5 other.

The server sends it as a header-only domain-1 type-2 visibility delta whose
only other field is optional field 8 (length-delimited, at most 12,288 bytes):
a replacement list of at most 256 entries, each a message with field 1 the
16-byte entity identity, field 2 the nonzero actor generation and field 3 the
emblem, each exactly once. Identities are unique and name visible players only;
a player without an entry shows no emblem. An empty list clears every emblem.
A delta with field 8 has no entity entries and nothing after field 8.

Relation (Tibia's): a viewed player without a guild has no emblem. If the
viewed player's guild is at no active war, the emblem is 4 when it is the
observer's guild and 5 otherwise. If it is at an active war, the emblem is 1
when it is the observer's guild, 2 when it is at war with the observer's
guild, and 3 otherwise. Guild membership is `game_guild_members` of a guild
that is not disbanded; an active war is a `game_guild_wars` row of the World
with state 2 (the same war state guild-war kills and PvP rules use).

The server computes the list from the Channel's positioned players, keeps the
entries the session currently sees, and sends a new list only when it differs
from the last one sent on this connection. An unavailable read sends nothing.
Without the capability the field is refused and nothing is sent; unselected
payloads are unchanged.
