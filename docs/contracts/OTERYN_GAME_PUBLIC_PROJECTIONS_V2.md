# Game public projections V2

Status: **ACCEPTED**, owner 2026-11-07, PUBPROJ (“1 tak, 2 zgoda”), for
character_deaths, guild and house. Implementation qualification and activation
remain separate; the publisher defaults disabled. Game owns all source facts;
Platform owns its read models and serving privacy floor. No account linkage,
ChannelId, position, session or authority token crosses these public surfaces.

This profile reconciles Platform main's
`docs/contracts/OTERYN_V2_PUBLIC_GAME_FAMILIES_EXTENSION_CONTRACT_PROPOSAL.md`
revision 2 with Game V1 §§2–8. It replaces only V1's three candidate families;
accepted V1 routes and its closed watermark enum retain their schemas.
Skills, achievements, equipment, outfit, journal, atlas and catalogue adapters
are outside this acceptance.

## Common wire

Reuse V1 canonical UTF-8 JSON, no whitespace, escapes, BOM, null, duplicate or
unknown members; nesting at most five containers. Times are decimal-string Unix
seconds, source milliseconds floored. UUIDs are canonical lowercase non-nil
UUIDv7. Revisions/epochs are positive decimal-string uint64. Names follow V1's
printable ASCII producer subset; keys are 1–128 `[a-z0-9._:-]` bytes.

Ordered envelope: `contract_version` (integer 2), `operation`, `source_authority`,
subject members, `projection_epoch`, `projection_revision`, `source_observed_at`,
body. Observation is revision assignment time and remains identical on retry.

| Family | Operation | POST path | Bytes | Subject | Body |
|---|---|---|---:|---|---|
| character_deaths | PublishCharacterDeathsV2 | /internal/v2/game-public/character-deaths | 32768 | character_id | deaths |
| guild | PublishGuildV2 | /internal/v2/game-public/guild | 393216 | guild_id | guild |
| house | PublishHouseV2 | /internal/v2/game-public/house | 2048 | world_id, house_key | house |

The purpose-bound public-projection mTLS identity and external epoch fence are
shared with V1. One publication per family is in flight. Connect ≤1s, handshake
≤2s, whole exchange ≤3s, response ≤256 bytes. V2 acknowledgements are exactly
`{"contract_version":2,"result":"accepted"}` or
`{"contract_version":2,"result":"superseded"}`. HTTP 200 without a valid body
never clears work. Definite 400/401/409 holds and transient backoff follow V1 §6.

## Deaths and stable killer identity

`deaths` contains at most 20 entries, newest `(occurred_at, death_occurrence_id)`
first. Each entry in order: `death_occurrence_id`, `occurred_at`, `level`, `pvp`,
`killers`. Receipt occurrence identity is immutable; level is `level_before`.
Platform serves only the last 30 days, and only under an eligible current
profile. Hidden/deleted victims publish `deaths: []`.

Owner amendment 2026-11-07 supersedes Platform proposal's name-only killer array:
`killers` holds at most 16 tagged objects. Player object exact ordered members:
`kind: "player", character_id, name`; creature object:
`kind: "creature", creature_key, name`. Identity is the committed CharacterId
or native creature content key, never a name, sprite ID or account. Historical
name at death commit is presentation only; never resolve a reused historical
name to a current holder. Environment/unknown causes are an empty list and
are never inferred as creatures. A stable creature key requires native source
capture; legacy receipts lacking it remain unknown.

Preserve PARTY-PVP0 classification ordering: final blow first when recorded,
then damage contributors and assists, with name-byte ordering inside each
group. Suppress an ineligible player's entire identity/name object; never
replace the id with a guessed/current name. V1 §7 applies to both fields:
internal IDs never become public URLs/API identity, account joins or an
unfiltered kill history. Platform enforces current killer visibility as well.
Death consequence data, unjustified flags, skulls and revenge are excluded.

## Guild

`guild` is 0..1 records. Exact ordered fields: `world_id, name, state,
founded_at, ranks, members`. State FORMING/ACTIVE/DISBANDING; disbanded or
unclassifiable guilds publish `[]`. Ranks are 3..20 unique ascending
`level` (integer 1..20), `name`. Members are at most 2000, each ordered
`character_id, name, rank_level, joined_at`, sorted `(rank_level, character_id)`.
Member ids are unique, ranks must exist, times are positive decimal strings.
Exclude ineligible profiles and private guild memberships. A valid guild with
no visible members retains its record and empty members. CharacterId joins
replace name joins; title, MOTD, balances, invitations and ACL remain private.

## House

The body and exact status-dependent fields are V1 §4.5, unchanged. Static
fields come from the active Game house catalogue; retired/absent entries
publish `house: []`. Unknown/hidden owner or disposition publishes UNAVAILABLE.
Owner name is display only. Preserve the V1 §7.3 name handoff barrier before a
new holder can be joined by name. No bids, maxima, accounts, owner IDs or ACL.

## Persistence, watermark and recovery

Source changes, per-subject revision and outbox are one transaction under their
existing authority. Rejected writes publish nothing. Durable snapshots preserve
exact content for a revision across retries and restarts; an ACK clears only
through its acknowledged pair, retaining concurrent successors.

V2 watermark operation `PublishPublicProjectionWatermarkV2`, POST
`/internal/v2/game-public/watermark`, at most 512 bytes. Exact ordered fields:
`contract_version, operation, source_authority, family, projection_epoch,
complete_through, observed_at`. The V2 family enum includes character_deaths,
guild, house, world_daily, character_online_sessions and world_daily_kills.
Do not send these via the V1 watermark. Completeness cannot pass unresolved
source facts, pending outbox rows, resync work or privacy withdrawals.

Operator `public-projection resync --family` accepts each implemented family;
`all --raise-epoch true` uses the same external F_pub fence as V1. The node
must be stopped for restore/resync; Platform serves only fresh, privacy-fenced
read models and never queries Game during page rendering.
