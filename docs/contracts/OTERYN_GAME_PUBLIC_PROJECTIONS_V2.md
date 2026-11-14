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
The P3 wire extension for skills, achievements, equipment and outfit is accepted
against Platform revision 3 at `f5f91a6e81bc8b89b51cbb77517253cf549dc4fa`
(#1854), owner 2026-11-13. Parts A/B supply wire and durable capture; publisher activation remains
separate. Journal, atlas and catalogue adapters stay outside
this acceptance. The existing Game death amendment below remains unchanged.

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
| character_skills | PublishCharacterSkillsV2 | /internal/v2/game-public/character-skills | 4096 | character_id | skills |
| character_achievements | PublishCharacterAchievementsV2 | /internal/v2/game-public/character-achievements | 262144 | character_id | achievements |
| character_equipment | PublishCharacterEquipmentV2 | /internal/v2/game-public/character-equipment | 8192 | character_id | equipment |
| character_outfit | PublishCharacterOutfitV2 | /internal/v2/game-public/character-outfit | 2048 | character_id | outfit |

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

## P3 character views (revision 3, wire only)

Each body is 0..1 records: an empty outer list suppresses earlier data.
All records start with `world_id, character_revision`. Skills are public;
achievements, equipment and outfit are owner-only despite the ingest prefix.
Current-owner authorization and Character Authority's ownership-transition
revision floor are prerequisites for the owner-only views; watermark freshness
alone cannot establish them.

Skills appends, in order: `level, total_experience, magic_level, fist_level,
club_level, sword_level, axe_level, distance_level, shielding_level, fishing_level`.
Level is positive decimal-string uint32; XP is decimal-string 0..i64::MAX.
Magic is integer 0..1000; the seven other skills are integers 10..1000.
Values exclude temporary modifiers; level and XP may decrease.

Achievements appends `catalogue_revision, earned`. Revision keys match
`[A-Za-z0-9][A-Za-z0-9._:-]{0,127}`. Earned rows contain exactly `key, earned_at`,
strictly ascending unique key, positive Unix seconds up to i64::MAX.
Achievement keys are `oteryn:achievement/` plus 1..141 lowercase letters,
digits or underscores (total at most 160 bytes), resolved against that exact
catalogue. More than 1024 rows refuses the whole snapshot, never truncates.
These are the current owner's account facts viewed through the Character;
no previous owner's facts may survive an ownership transition.

Equipment appends `slots` (0..10). Rows contain `slot, item_key, item_revision`,
strictly ordered and unique: Head, Necklace, Container, Armor, RightHand,
LeftHand, Legs, Feet, Ring, Ammo. Only occupied primary slots from committed
custody qualify; a reserved two-handed slot is not a second item.

Outfit appends `content_revision, outfit_key, colours, addons, mount`.
Colours are exactly four integers 0..65535 in head/body/legs/feet order;
addons is integer 0..255. Mount is 0..1 records with only `mount_key`.
The source must export compatible DisplayedLook, not selection/unlocks;
deferred mount display emits no mount. Keys use the common key grammar.

Four independent `[platform.public_projections]` flags default false, missing
means false: `character_skills_enabled`, `character_achievements_enabled`,
`character_equipment_enabled`, `character_outfit_enabled`. The existing global
`enabled` prerequisite remains unchanged. Publisher selection and the physical mTLS boundary exclude these four families
even when flags are true until Part C enables qualified delivery. Migration
2834 supplies their durable sources. No new environment variable is defined.

## Persistence, watermark and recovery

Source changes, per-subject revision and outbox are one transaction under their
existing authority. Rejected writes publish nothing. Durable snapshots preserve
exact content for a revision across retries and restarts; an ACK clears only
through its acknowledged pair, retaining concurrent successors.

V2 watermark operation `PublishPublicProjectionWatermarkV2`, POST
`/internal/v2/game-public/watermark`, at most 512 bytes. Exact ordered fields:
`contract_version, operation, source_authority, family, projection_epoch,
complete_through, observed_at`. The V2 watermark family enum is closed: character_deaths,
guild, house, character_skills, character_achievements, character_equipment,
character_outfit, character_online_sessions, world_daily_kills, world_daily.
Do not send these via the V1 watermark. Completeness cannot pass unresolved
source facts, pending outbox rows, resync work or privacy withdrawals.

Operator `public-projection resync --family` accepts each implemented family;
`all --raise-epoch true` uses the same external F_pub fence as V1. The node
must be stopped for restore/resync; Platform serves only fresh, privacy-fenced
read models and never queries Game during page rendering.


## Character-view source capture (P3 Part B)

Migration 2834 reuses v2 heads, monotonic counters, outbox and epoch fencing.
Progression/build/root, earned-account facts, ownership, committed equipment
(including the main backpack), item changes and appearance changes touch the
source in the same transaction. Deferred capture observes the final cut.
Missing/unreconciled progression suppresses skills; missing build uses only its
established seed. Missing/invalid equipment custody suppresses equipment.

Achievement catalogue identity is `sha256:` followed by the lowercase SHA-256
of all validated embedded achievement shards in their existing filename order.
The Game-only synchronization method installs that exact catalogue; readers
refuse unavailable or incompatible metadata. A bounded 1025-row scan counts and
aggregates in one snapshot; overflow or unknown provenance creates an
undeliverable head and retains its outbox. Ownership changes rebind the earned
view to the new account and current CharacterRevision under the source fence.

The appearance writer captures DisplayedLook only after its existing content,
sex and earned-unlock validation. Its proof is invalid for another owner, sex,
content or appearance occurrence. A raw historical selection without proof is
suppressed. The displayed mount list is always empty. This adds no unlock or
appearance mutation authority.

Full operator resync includes all four families and all subjects, including
suppression rows. Baseline completeness is false until that scan completes.
Unknown/poison rows remain queued. Delivery acknowledgements continue to clear
only through the acknowledged epoch/revision; a concurrent successor remains.
