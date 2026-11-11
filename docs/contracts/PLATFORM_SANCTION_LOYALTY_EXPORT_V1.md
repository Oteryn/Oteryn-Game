# Platform sanction and loyalty export V1

Status: Game-owned consumer contract, 2026-11-11. Programme items N009 (own sanction record,
capability 497) and N014 (Loyalty highscores, capability 498).

## Authority

Platform owns identity, commercial state, sanction decisions and loyalty earning
(`CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md`, ADM-01 §6.7). Game consumes a normalized export
and keeps a read-only projection for its own clients. Game never decides, enforces, edits,
expires or revokes a sanction from this projection, and never awards or changes loyalty. A
missing projection row means "nothing exported", never "no sanction" in Platform.

## Intake route

Both families use the existing Platform-to-Game projection route of loyalty migrations
0406/0925: Platform's control-plane writer calls a revisioned, idempotent `SECURITY DEFINER`
function with the `oteryn_game_control` group. `oteryn_game_runtime` has no `EXECUTE`
on either function and only reads the projection tables. No new transport, endpoint, poller
or credential is introduced. Each call is one record. Platform sends a stream's records in
ascending revision order; Game tolerates replays and reordering as below.

## Sanction record

Function `game_account_sanction_project` (migration 2800), one call per decision revision of
one sanction stream. Platform sends only records that the affected player may see about their
own account. Staff-only notations, staff notes, the deciding staff identity, evidence,
reports and other accounts are never exported.

| Argument | Type | Rule |
|---|---|---|
| `p_sanction_stream_id` | UUID | UUIDv7, ADM-01 `sanction_stream_id`; one row per stream |
| `p_decision_revision` | BIGINT | 1..=2^63-1, ADM-01 `decision_revision`; strictly increases per stream |
| `p_account_id` | UUID | the Game account (`game_character_account_guards`), unchanged per stream |
| `p_character_id` | UUID or NULL | a Character of that account; NULL for an account-wide record; unchanged per stream |
| `p_kind` | SMALLINT | 1 mute, 2 jail, 3 character_ban, 4 account_ban, 5 namelock, 6 notation; unchanged per stream |
| `p_reason_code` | INTEGER | 1..=65535, Platform's player-facing reason category id |
| `p_reason_text` | TEXT | 1..=120 UTF-8 bytes, no control characters; the player-facing reason category text |
| `p_starts_at_ms` | BIGINT | > 0, Unix milliseconds when the decision took effect |
| `p_ends_at_ms` | BIGINT or NULL | > `p_starts_at_ms`; NULL means permanent (bans) or not applicable (namelock, notation) |
| `p_revoked` | BOOLEAN | true once Platform revoked or lifted the decision (ADM-01 unmute, release, revoke) |

Shape rules (table checks): kinds 1, 2, 3 and 5 name a Character; kind 4 names none; kind 6 may
name one. Kinds 1 and 2 need an end; kinds 5 and 6 have none; kinds 3 and 4 have an end or are
permanent. A Platform lift is a higher revision of the same stream with `p_revoked = true`, not
a second stream.

Result `SMALLINT`:

- `1` applied: a new stream, or a higher revision replacing the stored one;
- `2` replay: the same revision with identical content, nothing changes;
- `3` stale: a lower revision, the stored row is kept.

Refusals change nothing: an equal revision with other content, or a revision that moves the
stream to another account, Character or kind, is `23000`; a Character outside the account or
an unknown account is `23503`; a check violation is `23514`; a non-positive revision is `22003`.

## Loyalty record

Unchanged migration 0925 function `game_account_loyalty_project_premium_days`, one call per
account revision.

| Argument | Type | Rule |
|---|---|---|
| `p_account_id` | UUID | the Game account |
| `p_spent_premium_days` | BIGINT | 0..=2147483647, the absolute account-wide spent-Premium-day total |
| `p_platform_revision` | BIGINT | > 0, strictly increases per account |

Game derives loyalty points 1:1 from the total (0925), stores them in
`game_account_loyalty_projections` and returns them. The same revision with the same total is
a replay; the same revision with another total is `23000`; a lower revision is `40001` and
changes nothing. A higher revision may lower the total (refund, chargeback, correction).

## Game consumers

- Capability 497, command 6240 (`protocol-oteryn/v1/sanction-record-v1.md`): the admitted
  Character reads its own account-wide records and its own Character records, newest first,
  at most 32.
- Capability 498 (`protocol-oteryn/v1/highscores-loyalty-v1.md`): highscores category 13
  ranks the World's eligible Characters by their account's projected loyalty points.
  Category 13 is never republished through `OTERYN_GAME_PUBLIC_PROJECTIONS_V1`.
