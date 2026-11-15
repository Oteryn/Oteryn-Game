# Highscores catalogue V1

Capability 502 requires 9. It is additive; query 14 is unchanged.

## Request

Command 6241 has an empty request. Command 6242 has a World type byte followed
by the unchanged HighscoresQueryV1 (at most 46 bytes total). Type 0 means any,
1 Optional PvP, 2 Open PvP, 3 Hardcore PvP, 4 Retro Open PvP, 5 Retro Hardcore
PvP. Other values are invalid. The World selection, type, vocation and category
are conjunctive. An absent World retains query 14's admitted-World meaning.
Category 13 still requires 498. A requested type requires the selected World
to appear in the current catalogue with that type. With no type filter, the
existing public World query semantics are retained, including a World whose
type has not been published.

## Reply

6241: count:u16le (0..256), then count entries of WorldId (16 bytes) and type:u8
(1..5), ordered by WorldId, strictly unique, with no trailing bytes. The server
projects Channel scopes through the independently validated current
admission guards and control-published field World policies, including offline
Worlds. This metadata grants no live PvP authority. Missing policies
supply no invented type; conflicting current Channel types fail closed.
The current gameplay rules publish Optional, Open and Hardcore PvP. Retro types
are reserved for Worlds whose owning policies implement those rules.
World labels may be resolved from the login catalogue; ids remain authoritative.
6242 returns the existing HighscoresResultV1 with its persisted snapshot, rank,
page and own row semantics. Queries retain the admitted session's authority.
Both commands are refused without 502. No persistent data is written.
