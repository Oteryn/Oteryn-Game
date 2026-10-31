# Team Finder leader v1

Capability 200 (`TEAM_FINDER_LEADER_V1`), owner decision S14a. Only bundle-world nodes offer
it. The leader edits one listing keyed by their character; it is kept across logout and
relog. Browse/join (S14b) and accept/deny (S14c) are separate later capabilities.

Source: Crystal 201189f8 `src/server/network/protocol/protocolgame.cpp:2973-3036`
(`createLeaderTeamFinder`) and `:3060-3121` (`parseLeaderFinderWindow`).

## Upsert

Command 390. At most 64 bytes of varint fields (wire type 0), each exactly once:

| field | meaning | bound |
|---|---|---|
| 1 | minimum level | 1..=65535, not above field 2 |
| 2 | maximum level | ..=65535 |
| 3 | vocation mask | 1..=255 |
| 4 | team slots | 1..=65535 |
| 5 | free slots | 0..=field 4 |
| 6 | party members join | 0 or 1 |
| 7 | client timestamp | u32 |
| 8 | team type | 1 boss, 2 hunt, 3 quest |
| 9 | boss id, hunt type or quest id | u16 |
| 10 | hunt area | u16; present only for type 2 |

Any other field, a repeated or missing field, or a value outside its bound is a refusal.
The upsert replaces the whole listing.

## Remove

Command 391. The payload is empty. Removing when no listing exists succeeds.

## Reply

The reply payload is empty; the command status is Accepted or Rejected. A replayed command
id returns its first result.
