# Account Achievement completion filter v1

Capability 612 enables command 6387. The existing command 10 remains the earned
achievements query. Both commands use the admitted controller's account and the
server's activated achievement catalogue.

## Request

A single filter byte followed by the existing `AccountAchievementsQuery`
payload. The filter is 0 for completed or 1 for incomplete. The page defaults to
0. The maximum encoded size is 7 bytes. Invalid filters and malformed page
payloads are rejected by the existing command path.

## Result

The result uses `AccountAchievementsResult`. Completed is identical to command
10, including earned secret and retired rows. Incomplete shows only public,
earnable catalogue entries without an account fact. Their
`earned_at` field is absent (decodes as 0), and their points are the catalogue
value if earned. Secret and retired entries do not appear in incomplete results.

`fact_count` and `total_points` always describe all earned facts, as in command
10. Filtering precedes sorting by grade, name and key and paging to 64 rows.
`has_more` describes the filtered result. A stored unknown fact key is an
account-integrity failure for either filter.

Crystal sends unlocked achievements and reveals secret metadata after unlocking
(`src/server/network/protocol/protocolgame.cpp:4489–4508` at `201189f8`). The
public versus secret visibility also appears in the Tibia Fandom archive
[Achievements](https://tibia.fandom.com/wiki/Achievements).
