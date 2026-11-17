# PARTY_LIST_V1 (461)

Reference: `docs/reference/tibia-manual/interface.md` §3.6.3 (lines 122–125).
Canary reference: `src/creatures/players/grouping/party.cpp`,
`showPlayerStatus`, `updatePlayerStatus`, `updatePlayerHealth`, `updatePlayerMana`
(https://github.com/opentibiabr/canary/blob/main/src/creatures/players/grouping/party.cpp).

Requires PARTY_V1 (42). Uses its existing query (46), membership authority,
opaque member handles and session fence. No new command or persistence.

Selection adds repeated length-delimited field 6 to each PartyEntry. Without
selection the field is omitted, preserving the legacy wire. Invitations never
carry it. The first row represents the member, followed by at most two current
player summons. Members outside the local runtime remain in the membership list
without live display data. Every refresh replaces the projection, including
summon removal and unavailable vitals.

Each row has the following canonical byte layout, with integers big endian:

| Bytes | Value |
| --- | --- |
| 16 + 8 | scoped entity identity and nonzero generation |
| 1 + 1 | vocation (0 none, 1 knight, 2 paladin, 3 druid, 4 sorcerer, 5 monk), summon boolean |
| 4 | horizontal Chebyshev distance (summons use their owner’s distance) |
| 1 + 1 + 1 | health percent, mana percent, observer-relative skull; 255 means unavailable |
| 1 + N | UTF-8 name length (1–64 bytes), name |
| 2 + N | optional Appearance Binding length and canonical binding bytes |
| 12 | head/body/legs/feet RGB values resolved from the authoritative cosmetics palette |

Health/mana are 0–100, rounded up; skull is 0–6. Beyond 30 fields, health, mana and skull
are all unavailable. Binding entity must match the row. Rows are at most 512
bytes, entries at most 1536 bytes, and PartyResult at most 65536 bytes only
when capability 461 is selected. Without 461, the peer-aware server encoder
enforces the legacy 8192-byte limit and rejects Party List fields.
Cosmetics enrichment uses the existing appearance owner; unavailable artwork
never prevents membership operations or fabricates a portrait.

The client retains display-time ordering while an entity remains listed. Its
Party List controls rename the window, toggle players, the five vocation groups
and summons independently, and sort by time, distance, HP or case-insensitive
name. Clicking the current sort reverses it. Unknown HP sorts after known HP
in ascending order. These preferences are local to the open play session.
Summon rows cannot be used as membership management targets.
