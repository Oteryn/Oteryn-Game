# Highscores Loyalty V1

N014. Optional capability 498 requires highscores capability 9 and is offered only in
`BUNDLE_WORLD_OFFERED_CAPABILITIES`. It adds no command and no byte layout: a session that
selected it may name category 13 (Loyalty) in `HIGHSCORES_QUERY_V1` (command 14). Without it a
category 13 query stays `NOT_SUPPORTED` (1200), as before.

## Source and ranking

The highscore job publishes category 13 with the other categories in the same cut. Its source
is the account loyalty projection `game_account_loyalty_projections` (migration 0406), which
Game's puller fills with Platform's `loyalty_points` from the loyalty-snapshot stream
(migration 2803, `docs/contracts/PLATFORM_SANCTION_LOYALTY_EXPORT_V1.md`). Eligible
Characters are those of categories 1-9 (live, not detached by account closure, vocation other
than none) whose account has more than zero projected loyalty points. Each such Character
ranks with its account's points: `value_primary` is the points, `value_progress` is 0. No row
is fabricated for an account without a projection or with zero points.

Category 13 is a Game snapshot for Game clients only. It is never queued to the public
projection (`OTERYN_GAME_PUBLIC_PROJECTIONS_V1`), whose highscore domain stays
1..=12 | 14 | 16 | 17; Platform publishes loyalty itself.
