# T143: cross-World Highscores and Show Me

Owner decision B, 2026-11-09. This amends HIGHSCORES0-WORLD-HIGHSCORES-V1
§6 and supersedes its §9 R5 own-World-only ruling. Snapshot production,
retention, competition ranks, category ownership and vocation filters stay unchanged.

Reference: `docs/reference/tibia-manual/interface.md:217–218` requires game-World,
vocation and skill/category filters; https://tibia.fandom.com/wiki/Highscores
(HIGHSCORES0's captured revision 1190424) describes 50-row pages. Show Me selects
the page containing the asking character, using the stored **position** in the
selected list, never the global competition rank: ties and vocation filters make
rank and position different. A character absent from that capped list has no jump.

## Public reads and authority

Capability 9, command 14 remain the only route. Optional query field 5 is a
16-byte UUID-v7 `WorldId`; absence means the admitted World. It is public read
selection DATA, never a `ChannelId`, actor World switch or permission proof.
The durable current session must still be non-terminal and belong to the served,
admitted World; its CharacterId cannot come from the payload. Read the selected
World's existing committed snapshot with the existing bounded indexed queries.
A foreign snapshot token cannot select another World/category. Unknown/unpublished
Worlds refuse without fabricating scores. No writes or new source of authority.

Optional query field 6 (`details`, bool) opts in to result field 8 (`own_page`,
1..20, only with an own row) and field 9 (the selected typed WorldId). Without
`details`, preserve legacy result bytes. The query bound becomes 45 bytes; the
result bound becomes 9899 bytes. Unknown/repeated fields, malformed UUIDs and
non-boolean flags fail closed as before. No new capability, command or migration.

The own page comes from the own-row `position` in the **same repeatable-read
snapshot transaction** as the page. Only the admitted World returns an own row
or own page; another World's public read cannot choose another character.
Changing World/category/vocation resets to page 1 with token 0; page navigation
and Show Me retain the confirmed snapshot and filters. A refusal preserves the
last confirmed page; it never supplies a fabricated success.

Owner decision B 2026-11-09, reference silent: Oteryn has no Game-owned directory
of friendly World names. The native World selector accepts the canonical WorldId
(and blank for the current World), rather than inventing a directory or deriving
World identity from ChannelId. World-type/BattlEye filters are outside T143.
