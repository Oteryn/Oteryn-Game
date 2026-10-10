# Oteryn Game Public Projections v1

- Status: **CANDIDATE**, revision 1 (GAME-PUBLIC-PROJ-0), 2026-10-08. It authorizes no runtime,
  migration, Platform, Atlas or production work. Acceptance needs exact-head validation,
  independent review (protocol, privacy, persistence) and owner or architect acceptance relayed by
  the control plane. Each implementing child needs its own #1622 allocation.
- Contract ID: `oteryn-game-public-projections-v1`
- Coordination: #1622 (P5); Oteryn/Oteryn-Platform#1476 (Platform mirror; P8 DECANARY-PUBLIC-1
  consumes this contract).
- Control-plane rulings: **Q3=A**, Game pushes public projections into Platform read models in
  the LCFA style (push, mTLS, watermark), with no live query on page render; **D965 Q3=B**, guilds
  and house rent are in the core set (CP relay, #1622).
- Producer and authority: `Oteryn/Oteryn-Game`. Consumer: `Oteryn/Oteryn-Platform`, read-only.
  Atlas is not a consumer of v1.
- Builds on:
  - ADR-0004 §6 and §7: the Portal reads game data through a versioned Game query API or
    projection; public projections are Game-owned and read-only for Platform;
  - `docs/contracts/OTERYN_GAME_LIST_CHARACTERS_FOR_ACCOUNT_PROJECTION_V1.md` (LCFA, accepted for
    `testing` and `preproduction`, D828): transport, envelope, ordering, outbox, epoch fence,
    watermark, response vocabulary and the name wire rule, reused here by reference;
  - `docs/architecture/SOCIAL_PRESENCE_AND_CONTACT_CONSENT_OWNER_BASELINE.md` (owner-accepted):
    the privacy rules of §7;
  - HIGHSCORES-0 (`docs/architecture/reviews/OTERYN_GAME_HIGHSCORES0_WORLD_HIGHSCORES_DECISION_2026-10-03.md`,
    candidate): this contract is the Platform export profile that HIGHSCORES-0 deferred;
  - GUILD-0 (`docs/architecture/reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md`,
    candidate) §3, §4.2 and §9;
  - HOUSE-OWN-0 (`docs/architecture/reviews/OTERYN_GAME_HOUSE_OWN0_HOUSE_OWNERSHIP_DECISION_2026-09-30.md`,
    candidate), EXP-HOUSES-01 and `docs/architecture/OTERYN_HOUSE_CATALOGUE_OWNER_CONTRACT_V1.md`;
  - DEATH-0 (`docs/architecture/reviews/OTERYN_GAME_DEATH0_CHARACTER_DEATH_RECEIPT_DECISION_2026-09-28.md`,
    candidate) and PARTY-PVP0 §9 and §10 (candidate);
  - the multichannel scope matrix (`docs/architecture/MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md`).
- Several sources are candidates. A field taken from a candidate source is marked with that
  source. If the source is superseded before this contract is accepted, the field follows the
  accepted source, and the change is made here before acceptance. After acceptance, any change to
  a member, path, bound or response is a new `contract_version`.

## 1. Purpose

Platform's public web pages (highscores, character pages, guilds, deaths, houses, who is online)
read Canary tables directly today. This contract replaces that with six Game-owned public
projections that Game pushes into Platform read models. Platform renders pages from its read
models only and never queries Game while rendering a page.

The projections are public by design. Each one carries only data that the owner baseline allows
anyone to see, and never any account linkage.

## 2. Common semantics

### 2.1 Families

| Family | Subject (one snapshot each) | Source | §  |
|---|---|---|---|
| `highscores` | (`world_id`, `category`, `filter`) | HIGHSCORES-0 snapshot rows | 4.1 |
| `character_profile` | `character_id` | Character root, progression, build state, guild membership | 4.2 |
| `character_deaths` | `character_id` | DEATH-0 receipts and PARTY-PVP0 classification | 4.3 |
| `guild` | `guild_id` | GUILD-0 guild, ranks and members | 4.4 |
| `house` | (`world_id`, `house_key`) | HOUSE-OWN-0 property row and the active house catalogue | 4.5 |
| `world_online` | `world_id` | committed Game session state | 4.6 |

- **Snapshot, not delta.** Every publication carries the subject's complete current public
  state. Platform replaces its entry for the subject, so a lost or duplicated publication never
  corrupts the read model.
- **Absence.** A subject that must not be public, or no longer exists, is published as a snapshot
  with an empty body list (§4). Platform then deletes or hides its entry. Nothing becomes public by
  default when its state is unknown.

### 2.2 Multichannel: WorldId only, never ChannelId

- `WorldId` and `ChannelId` are distinct and never derived from each other. Every family scoped to
  a World carries `world_id`.
- **No projection carries a `ChannelId`,** nor any instance, node, map, position, last Channel or
  per-Channel count. `world_online` reports only "online on this World", aggregated over every
  Channel of the World (baseline: exact placement is never public).
- Highscores, guilds and houses are World-scoped with one state on every Channel (scope matrix).
  Moving between Channels of one World changes no projection.

### 2.3 Session-generation fencing

- Character writes stay session-generation fenced. No projection carries a session generation,
  lease, owner token, fencing value or connection identity. Nothing in a projection can be used to
  take, extend, compare or forge a fence.
- Every revision (§5) is advanced in the same committed transaction as the change it reflects. A
  write rejected by its fence commits nothing, so it advances nothing. Projections only ever
  reflect committed, fenced writes. `world_online` reads committed session state only, never
  in-memory runtime state.
- The publisher never writes Character, guild, house or session state and never takes a lease or
  session. Its only writes are outbox deletions after an acknowledgement and, for `world_online`,
  its own snapshot rows.

### 2.4 Owner and authority rules

| Fact | Owner and truth | Platform role |
|---|---|---|
| Characters, names, level, vocation, deaths, guilds, houses, presence | Game, only | public read model; never proof of anything |
| `projection_epoch`, `projection_revision`, outbox, resync | Game | orders and invalidates; never sets or requests them in v1 |
| `WorldId`, World names and Registry | Platform Registry | Game copies `world_id`; never mints it |
| `AccountId`, account state, loyalty | Platform | never present in any public projection |

- Platform never writes Game state from a read model and never offers a write through a public
  page. A dispute between a read model and Game is always resolved in favour of Game.
- **Platform must never join a public projection with the LCFA read model or any account data in
  anything it serves publicly.** Public pages must not reveal, directly or by grouping, ordering or
  co-listing, that two characters share an account.

## 3. Transport and identity

Transport, TLS and the HTTP client follow LCFA §3. The differences are:

- **Identity.** A dedicated public-projection client certificate, separate from the LCFA
  projection, native-evidence and runtime-status certificates. Only the public-projection
  publisher holds it, on Game authority hosts whose database role may read the source tables.
  Platform accepts these routes only from its configured public-projection identities, and refuses
  those identities on every other route.
- **Operations** (closed set, compiled paths):

| Operation | Path |
|---|---|
| `PublishWorldHighscoresV1` | `POST /internal/v1/game-public/highscores` |
| `PublishCharacterProfileV1` | `POST /internal/v1/game-public/character-profile` |
| `PublishCharacterDeathsV1` | `POST /internal/v1/game-public/character-deaths` |
| `PublishGuildV1` | `POST /internal/v1/game-public/guild` |
| `PublishHouseV1` | `POST /internal/v1/game-public/house` |
| `PublishWorldOnlineV1` | `POST /internal/v1/game-public/world-online` |
| `PublishPublicProjectionWatermarkV1` | `POST /internal/v1/game-public/watermark` |

- **Bounds.** Connect 1 s, handshake 2 s, exchange 5 s. The request byte limits are in §8. Every
  response is at most 256 bytes.
- **In-flight.** At most one publication in flight per family per publisher, so a large
  highscores snapshot never delays `world_online`. `world_online` is per-World: the publisher
  sends the changed Worlds of one scan concurrently, one publication in flight per World, up to
  `PUBPROJ-ONLINE-WORLDS` (16) Worlds. Each exchange keeps its own 5 s bound, so a scan of up to
  16 changed Worlds still fits `PUBPROJ-ONLINE-SCAN-DEADLINE` (5 s); a deployment with more
  Worlds than that is not enabled until the deadline is re-derived for the fanout.

## 4. Wire (v1)

### 4.0 Common envelope and encoding

Every snapshot request is one JSON object with an exact member set. It has no unknown, duplicate
or `null` members and nests at most 5 levels. It uses raw UTF-8 with no string escapes and no
insignificant whitespace (LCFA §4).

| Member | Rule |
|---|---|
| `contract_version` | `1` |
| `operation` | the family's operation (§3) |
| `source_authority` | the configured Game authority namespace, 1..128 of `[A-Za-z0-9._:/-]` |
| subject members | as listed per family below |
| `projection_epoch`, `projection_revision` | canonical non-zero decimal uint64 strings (§5) |
| `source_observed_at` | canonical decimal Unix seconds at which the revision was assigned; identical on every retry |
| body | the family's list, as listed per family below |

Encoding rules:

- **Ids.** Every `*_id` is a canonical lowercase non-nil UUIDv7. `character_id` and `guild_id` are
  internal join keys of the read model (§7.3).
- **Names.** Every character, guild, rank and house name follows the LCFA name wire rule: 1..64
  bytes, NFC, no Cc or Cf characters, no `"` or `\`. Game emits only names its own name policies
  allow, a subset of the wire rule. The consumer validates the wire rule only.
- **Integers.** `category`, `filter`, `vocation`, `rank_level` and guild `ranks[].level` are JSON integers in canonical
  form (no sign, no leading zero, no exponent), in the ranges given per family. Levels, ranks,
  values, gold amounts and times are canonical decimal uint64 strings, with `0` allowed only where
  stated.
- **Vocation.** `vocation` is the build-state vocation id (A13, as stored in HIGHSCORES-0 rows),
  0..255. Platform maps it for display. An id it does not know is shown as unknown and never
  refuses the snapshot.
- **Content.** The content of a snapshot is its subject members plus its body. For
  `highscores`, `computed_at` is content as well, so an equal pair with a different
  `computed_at` is `409`. The envelope, `source_observed_at` and the epoch and revision pair are
  not content (LCFA §4 ruling). An equal
  pair with equal content is an idempotent `200 accepted`, and the consumer keeps its first stored
  `source_observed_at`.

Responses are exactly LCFA §4's: `200` `{"contract_version":1,"result":"accepted"}` or
`{"contract_version":1,"result":"superseded"}`. Failures have empty bodies: `400` malformed or
over a bound, `401` unauthenticated or wrong identity for the route, `409` equal pair with
different content, `429` rate limited, `503` unavailable.

### 4.1 `highscores` (HIGHSCORES-0 export profile)

```json
{"contract_version":1,"operation":"PublishWorldHighscoresV1","source_authority":"oteryn:game:primary","world_id":"01934f10-7c02-7001-805b-3b1122334401","category":1,"filter":0,"projection_epoch":"1","projection_revision":"17","source_observed_at":"1790000000","computed_at":"1790000000","rows":[{"rank":"1","character_id":"01934f10-7c04-7001-805b-3b1122334401","name":"Aldric","vocation":4,"level":"412","value":"11400000000"}]}
```

- Subject: `world_id`, `category` 1..11 (the HIGHSCORES-0 §3 ids that have a Game source; 13
  Loyalty is Platform's and never published by Game), and `filter` 0..5 (0 = all, otherwise a base
  vocation, HIGHSCORES-0 §5).
- `computed_at` is the snapshot's `computed_at`. `rows` are the snapshot's rows for that filter in
  `position` order: 0..1,000 rows (`HIGHSCORES0-RL-02`). `rank` is the competition rank (1, 2, 2,
  4). `character_id` is the row's character, at most once per snapshot. It is an internal join
  key and never appears on a public page (§7.3). `name` is the name read at the cut. `value` is
  the category value; `0` is allowed.
- A row carries no `AccountId` or position. Category 10 (Achievement Points)
  appears only under the character HIGHSCORES-0 selects (highest-level live character). Platform
  must not add any other character of that account.
- One HIGHSCORES-0 snapshot of (World, category) is published as six snapshots, one per filter, all
  with the same revision (§5). Who is listed, and every exclusion, follows HIGHSCORES-0 §5. The
  deletion, sale and sanction exclusions it leaves open apply here once they are decided.
- **Profile join (fail closed, D607).** Platform shows a highscores row only while it serves a
  `character_profile` entry for the row's `character_id` in the same `world_id`, and shows it
  under that entry's current name. Otherwise the row is left out and the other rows keep their
  published `rank`. A character that becomes hidden, locked, pending deletion or deleted
  therefore drops out of every highscores page when its `profile: []` is stored, before the next
  HIGHSCORES-0 job. Names are mutable and may be reused (FND-ID-01), so the join never uses the
  name: a reused name never inherits another character's row. While `character_profile` is stale
  (§6), no highscores row is shown.

### 4.2 `character_profile`

```json
{"contract_version":1,"operation":"PublishCharacterProfileV1","source_authority":"oteryn:game:primary","character_id":"01934f10-7c04-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"9","source_observed_at":"1790000000","profile":[{"world_id":"01934f10-7c02-7001-805b-3b1122334401","name":"Aldric","vocation":4,"level":"412","guild":[{"guild_id":"01934f10-7c06-7001-805b-3b1122334401","rank_level":3}]}]}
```

- `profile` has 0 or 1 entries. It has 1 entry only for a live Character root (`lifecycle = 1`);
  a character without a vocation is published with its "none" vocation id. Every other state
  publishes `[]`: deleted, pending deletion,
  retired, transfer, locked, any sanction that hides the character once that decision exists, and
  any state the publisher cannot classify.
- `guild` has 0 or 1 entries: the GUILD-0 membership (`guild_id`, rank level 1..20). Guild and rank
  names come from the `guild` family, so a rename of either never touches profiles.
- **Deliberately absent:** `AccountId`, other characters of the account, account status,
  achievement points, last login or logout time, online state (only `world_online` carries
  presence), `ChannelId`, position, sex, comments, house (the `house` family names its owner),
  creation time. Adding a field is a new `contract_version`.

### 4.3 `character_deaths`

```json
{"contract_version":1,"operation":"PublishCharacterDeathsV1","source_authority":"oteryn:game:primary","character_id":"01934f10-7c04-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"3","source_observed_at":"1790000000","deaths":[{"occurred_at":"1789990000","level":"413","pvp":true,"killers":["Beren","Mira"]}]}
```

- `deaths` holds the character's most recent committed deaths, newest first: at most
  `PUBPROJ-DEATHS` (20). It is `[]` whenever `character_profile` would publish `[]`.
- `occurred_at` is the receipt's `committed_at` converted from Unix milliseconds, as stored by
  `game_character_death_receipts`, to Unix seconds: `occurred_at = floor(committed_at / 1000)`.
  The wire value is whole Unix seconds, like every other time on these routes. `level` is `level_before`, the level at death
  (DEATH-0 §3). `pvp` is the receipt's `pvp_death` (PARTY-PVP0 §10 amendment).
- `killers` lists 0..16 character names (`PUBPROJ-KILLERS`), consistent with PARTY-PVP0 §9
  (candidate). The final-blow character comes first when there is one. Then follow the other
  contributors recorded by the death's classification, damage contributors before assist-only
  ones, each group in name order. Names are the names at the death commit, not current names:
  the producer stores them in its derived projection rows in the victim's death transaction, so a
  later rename never changes a published death.
- **Deliberately absent:** creature killers and other non-player causes (no durable source in
  DEATH-0; U-PP3), experience lost, blessings, skulls, unjustified-kill classification, revenge
  marks and kill points. Platform hides entries older than `PUBPROJ-DEATHS-WINDOW` (30 days) at
  read time, so aging needs no publication.

### 4.4 `guild` (consistent with GUILD-0, candidate)

```json
{"contract_version":1,"operation":"PublishGuildV1","source_authority":"oteryn:game:primary","guild_id":"01934f10-7c06-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"5","source_observed_at":"1790000000","guild":[{"world_id":"01934f10-7c02-7001-805b-3b1122334401","name":"Red Rose","state":"ACTIVE","founded_at":"1789000000","ranks":[{"level":1,"name":"Leader"},{"level":2,"name":"Vice Leader"},{"level":3,"name":"Member"}],"members":[{"name":"Aldric","rank_level":1,"joined_at":"1789000000"}]}]}
```

- **This is the GUILD-0 §4.2 public view, minus the presence data.** If GUILD-1 or a guild wire
  contract fixes a different public shape before acceptance, this section follows it (the header rule on candidate sources).
- `guild` has 0 or 1 entries. `DISBANDED`, or a guild the publisher cannot classify, publishes
  `[]`. `state` is `FORMING`, `ACTIVE` or `DISBANDING`. `founded_at` is `created_at`.
- `ranks`: 3..20 entries (`GUILD0-RL-06`), `level` ascending, unique.
- `members`: 0..2,000 entries (`GUILD0-RL-05`), ordered by `rank_level` then name (GUILD-0 §9
  keyset order is rank level then CharacterId; names are used here because `character_id` is not
  published per member). A member whose character would publish `profile: []` is left out. A guild
  whose members are all left out publishes an empty `members` list, never `guild: []` (D245).
- Vocation, level and online state of members are not repeated here. Platform shows them by
  joining with `character_profile` and `world_online` on (`world_id`, name). That way one
  revocation path governs presence (§7.2), and a member's level-up never touches the guild.
- **Members only, never published:** titles, the guild message (motd), balance and escrow,
  invitations, applications, the activity log and every guildhall ACL. A member's other characters
  are never linked (GUILD-0 §4.2).

### 4.5 `house` (consistent with HOUSE-OWN-0, candidate)

```json
{"contract_version":1,"operation":"PublishHouseV1","source_authority":"oteryn:game:primary","world_id":"01934f10-7c02-7001-805b-3b1122334401","house_key":"oteryn:content.house.example-street-1","projection_epoch":"1","projection_revision":"12","source_observed_at":"1790000000","house":[{"name":"Example Street 1","kind":"private_house","town":"oteryn:content.area.city.example","size_sqm":"24","beds":"2","rent_gold":"5000","status":"OWNED","owner_kind":"CHARACTER","owner_name":"Aldric","paid_until":"1792000000"}]}
```

- Subject: `world_id` and the catalogue `house_key` (`identity.key`, 1..128 of `[a-z0-9._:-]`).
  Together they form the `HouseId`.
- `house` has 0 or 1 entries. A `RETIRED` house, or one absent from the active catalogue, publishes
  `[]`. `name`, `kind` (`private_house`, `shop`, `guildhall`), `town` (the `Area` key), `size_sqm`,
  `beds` and `rent_gold` come from the active catalogue record. `beds` may be `"0"` (shops in
  the active catalogue have no beds).
- `status` and the extra members of each status (an exact member set per status):

| Property state | `status` | Extra members |
|---|---|---|
| `VACANT` | `VACANT` | none |
| `AUCTION` | `AUCTION` | `current_bid` (the public current price; `0` allowed), `auction_ends_at` |
| `OWNED` | `OWNED` | `owner_kind`, `owner_name`, `paid_until` |
| `MOVE_OUT_PENDING` | `MOVE_OUT_PENDING` | `owner_kind`, `owner_name`, `paid_until`, `move_out_at` |
| `DISPOSITION`, unknown | `UNAVAILABLE` | none |

- `owner_kind` is `CHARACTER` (`owner_name` = the owner character's current name) or `GUILD` (a
  guildhall; `owner_name` = the guild name, pending GUILD-0 acceptance). An owner whose character
  would publish `profile: []` gives `UNAVAILABLE`.
- **Never published:** owner `CharacterId` or `AccountId`, the housing slot, bidder identities,
  bid maxima and bid history (HOUSE-OWN-0: maxima are never shown), `grace_until`, ACL entries,
  interior items and any payment source.

### 4.6 `world_online`

```json
{"contract_version":1,"operation":"PublishWorldOnlineV1","source_authority":"oteryn:game:primary","world_id":"01934f10-7c02-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"8812","source_observed_at":"1790000000","characters":[{"name":"Aldric","vocation":4,"level":"412"}]}
```

- `characters` lists the World's characters that are online on the World (a committed live game
  session on any of its Channels), whose `character_profile` would publish a present entry (§4.2),
  and whose presence may be public (§7.2). A character hidden by a sanction or any other
  visibility decision is never listed. There are 0 to
  `PUBPROJ-ONLINE` (4,096) entries, sorted by name in byte order, unique.
- If more characters qualify than the bound, the publisher publishes nothing for that World, and
  the World's feed goes stale (§5.1, §6). The list is never truncated, because a truncated list
  would hide arbitrary players and still look complete.
- **Deliberately absent:** `ChannelId` and per-Channel counts, login time, session duration,
  `character_id`, guild and party data. A total count is not published separately (Platform counts
  the list).

## 5. Revision, ordering and delivery

Ordering, outbox, resync and the epoch fence follow LCFA §5, applied per family and subject:

- **Ordering.** Platform orders each subject's snapshots by (`projection_epoch`,
  `projection_revision`). A lower pair is `superseded`. An equal pair with equal content is
  `accepted`. An equal pair with different content is `409` and not stored.
- **Revisions** are per subject, monotonic, and advanced in the same transaction as every change
  of a projected field, wherever that field's source row lives:

| Family | Revision advanced by |
|---|---|
| `highscores` | each committed HIGHSCORES-0 job run of (World, category): one revision for all six filters. A visibility removal does not touch it; the profile join removes the row (§4.1) |
| `character_profile` | create; rename; lifecycle change; world transfer; level or vocation change; guild join, leave or rank change; any visibility decision |
| `character_deaths` | each committed death of the character; any change that flips its profile between `[]` and present |
| `guild` | any change to guild state, name, ranks, roster or a member's rank; a member's rename; any change that flips a member's profile between `[]` and present |
| `house` | any property state or revision change; any bid placement, raise or lowering that changes `current_bid` or `auction_ends_at`; a catalogue activation that changes the record; a rename of the owner character or guild; any change that flips the owner's profile |
| `world_online` | the publisher's own scan of every World, which starts every `PUBPROJ-ONLINE-INTERVAL` (10 s); a World gets a revision when its qualifying list differs from its last snapshot. Its watermark follows the scan, not the outbox (§5.1) |

- **Transactional outbox.** Each revision touch also records the subject in that family's outbox,
  in the same transaction. The publisher reads the current subject state and revision in one read
  statement, sends it, and clears outbox rows up to the sent revision on `accepted` or
  `superseded`. Delivery is at least once, and duplicates are harmless.
- **Epoch.** `projection_epoch` is raised only after a restore or rollback of the Game store. A
  raise forces a full resync of every subject of every family. Platform invalidates every entry of
  every family below the highest epoch it has seen. Raises use LCFA §5's persisted external
  high-water fence, with a fence of this feed's own (`F_pub`): a raise is strictly above `F_pub` or
  it is refused, and the publisher publishes nothing while the store's epoch is below `F_pub`.
  Whether the epoch row is shared with LCFA is an implementation choice of the producer child
  (U-PP5).
- **Resync.** An operator command republishes every subject of one family or of all families
  (initial fill, Platform read-model loss, epoch raise). Platform cannot request a resync in v1.

### 5.1 Liveness watermark

```json
{"contract_version":1,"operation":"PublishPublicProjectionWatermarkV1","source_authority":"oteryn:game:primary","family":"world_online","projection_epoch":"1","complete_through":"1790000000","observed_at":"1790000003"}
```

- One watermark per (`source_authority`, `family`), sent at least every 10 s
  (`PUBPROJ-WATERMARK-GAP`). `complete_through` follows LCFA §5.1: no committed change at or
  before it is still undelivered, and during a full resync it stays below the resync start. Its
  validation and refusal are LCFA §5.1's, with Platform's `clock_uncertainty` of 1 s.
- For `world_online`, online changes (login, logout, level or vocation, a presence setting) are
  not recorded in an outbox, so the watermark is held by the scan instead (D607). A scan starts
  every `PUBPROJ-ONLINE-INTERVAL` and reads the committed session state of every World at one
  cut time. `complete_through` is the cut time
  of the last scan that completed and whose changed Worlds were all `accepted` or `superseded`
  within `PUBPROJ-ONLINE-SCAN-DEADLINE` (5 s) of that cut, and it never passes that cut. A scan
  that misses the deadline counts as failed, even if its snapshots are accepted later. A stopped, late or failed scan therefore stalls the watermark,
  and so does a World whose snapshot is refused for its bound (§4.6) or whose publication failed.
  The whole family goes stale rather than showing a list that is wrong for one World.
- A family is stale when `platform_now - complete_through + clock_uncertainty > S`, when the
  stored `complete_through` is more than `clock_uncertainty` after `platform_now`, or when the
  watermark epoch is not the highest seen. S = 30 s for `testing` and `preproduction` (U-PP1).
  Platform checks staleness each time it serves, including from a cache, so no cached entry
  outlives the moment its family goes stale.

## 6. Error handling

**Consumer (Platform).**

| Case | Response | Read-model effect |
|---|---|---|
| malformed, unknown member, over a bound, bad name or id | `400` | nothing stored |
| wrong identity or route | `401` | nothing stored |
| equal pair, different content | `409` | first stored snapshot kept |
| overload | `429` or `503` | nothing stored |

| Family state | What Platform serves |
|---|---|
| `world_online` stale | no online list and no online markers anywhere (fail toward less disclosure) |
| any other family stale | none of its entries; the family's pages say the data is unavailable. There is no stale display window, because an undelivered change may be a visibility removal (fail closed, D607) |
| entry below the highest epoch | not served until its new-epoch snapshot arrives |
| profile `[]` | the character's profile, deaths and highscore links are not found (§7.3) |

**Producer (Game).**

- `429`, `503`, timeouts and transport errors: retry with exponential backoff from 1 s to 60 s.
  The outbox row stays queued.
- `400`, `401` and `409` are producer or configuration defects. The row stays queued, retries for
  that subject drop to at most once per 5 minutes, and an operator alarm counter is raised. A row
  is never dropped or rewritten to make it pass. The watermark stalls, so the family goes stale
  and fails toward less disclosure.
- A snapshot that would exceed a §8 bound is refused before any exchange, and handled like `400`.
- Publication failure never affects gameplay, admission, sessions or the change that caused it.

## 7. Privacy (reviewed against the social presence baseline)

### 7.1 Baseline review

| Baseline rule | How v1 keeps it |
|---|---|
| exact Channel, instance, node and map placement are never public | no family carries `ChannelId`, position or a per-Channel figure (§2.2) |
| a non-contact sees at most "online on this world", subject to the player's settings | `world_online` is World-level only and honours the presence setting (§7.2) |
| party, guild or World membership grants no exact location | guild rosters carry no presence; Platform joins only to the coarse `world_online` |
| alternate characters hidden by default; no account linkage | no `AccountId` anywhere; no "other characters"; no LCFA join (§2.4); achievement points only under the HIGHSCORES-0 character |
| revocation and setting changes invalidate caches promptly and fail toward less disclosure | §7.2; a stale `world_online` is hidden; unknown states publish `[]` |
| no enumeration through sequential ids, timing, search or error differences | §7.3 |
| the client and consumers never infer hidden state from stale caches | Platform serves read models only, under the §6 staleness rules |

### 7.2 Presence setting

- No privacy-preference contract exists yet. Until one is accepted, every online character of the
  World whose profile would be present (§4.6) qualifies for `world_online`. This is the baseline's maximum for non-contacts, and GUILD-0
  §4.2 already shows a coarse online flag to the whole World. It is a candidate default (U-PP2).
- When a privacy-preference contract adds a presence setting, a character whose setting hides
  presence, or whose setting cannot be read, is left out of `world_online`.
- **Revocation bound.** A revocation is any committed change that removes a character from
  `world_online`: a presence setting that hides it, a sanction or other visibility decision, a
  deletion, or a logout. Platform stops serving the character's online marker within
  `PUBPROJ-ONLINE-REVOKE-BOUND` (20 s) of the revocation commit, by one of two paths:

  | Step (worst case: the revocation commits just after a scan's cut) | Bound |
  |---|---|
  | the next scan starts and takes its cut (`PUBPROJ-ONLINE-INTERVAL`) | 10 s |
  | the scan reads its cut and every changed World's snapshot is `accepted` or `superseded` (`PUBPROJ-ONLINE-SCAN-DEADLINE`) | 5 s |
  | Platform stops serving the replaced snapshot, caches included, after returning `accepted` (`PUBPROJ-ONLINE-SERVE-DELAY`) | 5 s |
  | **total, scans on time (`PUBPROJ-ONLINE-REVOKE-BOUND`)** | **20 s** |

  If the next scan is late, fails or misses its deadline, `complete_through` stays at or before
  the last cut, which is before the revocation. The family is then stale, and every marker is
  hidden, once `platform_now - complete_through + clock_uncertainty > S` (§5.1, §6). That is
  within S − 1 s = 29 s of the revocation, because staleness is checked at serve time. S = 30 s
  is the hard upper bound on either path, and the on-time path is shorter than it.

### 7.3 Enumeration and identifiers

- `character_id` and `guild_id` are internal join keys of Platform's read model. Platform never
  puts them in a public URL, page, API or error. Public pages are addressed by name: a character
  by name, a guild by (World, name), a house by (World, `house_key`; see the name handoff rule
  below).
- An unknown name, a hidden or deleted character, and a profile published as `[]` get the same
  public response, with the same status and body and no timing difference that depends on which
  case applies. Platform rate-limits public lookups.
- A rename makes the old name not found at once. Old names are never kept as public aliases.
- **Name handoff.** Game permits a name to be reused after a rename or deletion, and the two
  profiles publish under independent `character_id` subjects, so Platform can briefly hold two
  profiles for one name. The rule is namespace-level and has two halves. Game's publisher never
  publishes the new holder of a (World, character name) or (World, guild name) until the
  publication that vacates it (the old holder's rename, deletion or visibility change) is
  `accepted` or `superseded`; the publisher enqueues the vacating publication first. Platform
  resolves a public name only when exactly one current, non-stale profile or guild holds it in
  that World: a name held by two or more is answered as an unknown name (§7.3 first rule) until a
  single holder remains, and never shows either holder's data. House names are presentation, not
  identity: a house is addressed by (World, `house_key`), and a reused house name is never a
  routing key.

### 7.4 Logging

The publisher logs only the family, the operation, the result class and timings, never names,
ids or bodies. Platform does the same for these routes. The publication carries no credential or
secret.

## 8. Proposed limits

These are the worst-case sizes, assuming 64-byte names and maximum-length decimals. They are
registered in `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` by the producer child together with
the implementation, as LCFA's were.

| Limit | Value | Basis |
|---|---|---|
| `PUBPROJ-HS-REQUEST-BYTES` | 262,144 | 1,000 rows × at most 212 bytes + envelope |
| `PUBPROJ-PROFILE-REQUEST-BYTES` | 2,048 | at most 650 bytes in total |
| `PUBPROJ-DEATHS` | 20 deaths | most recent first |
| `PUBPROJ-KILLERS` | 16 names per death | final blow first |
| `PUBPROJ-DEATHS-REQUEST-BYTES` | 32,768 | 20 deaths × at most 1,156 bytes + envelope |
| `PUBPROJ-DEATHS-WINDOW` | 30 days | Platform read-time filter |
| `PUBPROJ-GUILD-REQUEST-BYTES` | 262,144 | 2,000 members × at most 127 bytes + 20 ranks + envelope: at most 256,339 bytes |
| `PUBPROJ-HOUSE-REQUEST-BYTES` | 2,048 | at most 1,100 bytes in total |
| `PUBPROJ-ONLINE` | 4,096 entries per World | wire bound, not a capacity; 8 Channels × 500 players (D128) |
| `PUBPROJ-ONLINE-REQUEST-BYTES` | 524,288 | 4,096 × at most 112 bytes + envelope |
| `PUBPROJ-ONLINE-INTERVAL` | 10 s | fixed start interval of `world_online` scans: the minimum gap between revisions of one World, and the maximum age of a healthy scan |
| `PUBPROJ-ONLINE-WORLDS` | 16 | changed Worlds published concurrently in one scan, one publication in flight per World |
| `PUBPROJ-ONLINE-SCAN-DEADLINE` | 5 s | from a scan's cut to the last changed World's `accepted` or `superseded`; a later scan counts as failed |
| `PUBPROJ-ONLINE-SERVE-DELAY` | 5 s | from Platform returning `accepted` to the replaced snapshot no longer being served, caches included |
| `PUBPROJ-ONLINE-REVOKE-BOUND` | 20 s | derived: 10 + 5 + 5; at most S (§7.2) |
| `PUBPROJ-WATERMARK-BYTES` | 512 | |
| `PUBPROJ-WATERMARK-GAP` | 10 s | maximum gap between watermarks of one family |
| `PUBPROJ-INFLIGHT` | 1 per family per publisher | |

## 9. Required tests

- Each family: exact wire fixtures shared with the Platform consumer, including a house with
  `beds: "0"`; unknown, duplicate and `null`
  members refused; every bound at its limit accepted and one past it refused.
- Ordering: a lower pair is superseded; an equal pair with different content is `409`; a higher
  epoch invalidates every family below it until resync; `source_observed_at` alone never causes a
  `409`.
- Revisions: each row of the §5 table advances in the same transaction as its change, including
  the cross-family touches (a rename touches profile, guild and house; a death touches deaths).
- Absence: deleted, pending-deletion, retired and unclassifiable characters publish `profile: []`
  and `deaths: []`, and are left out of guild rosters, house owners, highscores and
  `world_online`.
- Privacy: no family contains `AccountId`, `ChannelId`, a position or a member title; an
  unknown name and a hidden character give identical public responses; ids never appear on a
  public page.
- Presence: a stale `world_online` is hidden; a World over `PUBPROJ-ONLINE` publishes nothing and
  goes stale; a revocation committed just after a scan's cut stops being served within
  `PUBPROJ-ONLINE-REVOKE-BOUND` (20 s) when the next scan is on time, and within S when it is late
  or misses `PUBPROJ-ONLINE-SCAN-DEADLINE`, because the family goes stale; a scan whose snapshots
  are accepted after the deadline does not advance the watermark.
- Deaths: killer names stay the names at the death commit after a rename; a receipt with
  `committed_at` 1789990000999 (milliseconds) publishes `occurred_at` `"1789990000"` (seconds,
  rounded down).
- Watermark: `complete_through` never passes an undelivered change; a future-dated watermark is
  refused; a stopped watermark makes the family stale and unserved; a stopped `world_online` scan
  stalls its watermark even when no outbox row is pending; a scan that does not start within
  `PUBPROJ-ONLINE-INTERVAL` of the last one stalls it too.
- Content: a highscores snapshot with an equal pair and a different `computed_at` is `409`; a
  guild with every member hidden publishes an empty `members` list; a highscores row whose
  character publishes `profile: []` is not shown before the next HIGHSCORES-0 job, and no row is
  shown while `character_profile` is stale; a highscores row is joined by `character_id`, so a
  name reused by another character never shows the old row under it; a bid that changes
  `current_bid` or `auction_ends_at` advances the house revision; an online character whose
  profile is `[]` (for example after a sanction) is not listed in `world_online`; a scan with 16
  changed Worlds whose exchanges all take 4 s is accepted within the deadline, because the
  Worlds are published concurrently.
- Name handoff: a character renamed from `Aldric` while another character takes `Aldric` never
  has both profiles resolve; the new holder is not published before the rename is `accepted` or
  `superseded`, and if both reach Platform the name answers as unknown until one holder remains;
  the same holds for a reused guild name.
- Identity: only the public-projection certificate is accepted on these routes, and it is refused
  elsewhere.

## 10. Unknowns

| ID | Testing and preproduction (proposed) | Release entry |
|---|---|---|
| U-PP1 | S = 30 s; no stale display window (D607) | open: measured S |
| U-PP2 | coarse online is public by default (§7.2) | open: the privacy-preference contract and its default |
| U-PP3 | no creature or environment killers | open: a durable killer-cause field (DEATH-0 amendment), then a new `contract_version` |
| U-PP4 | public-projection identity from the test stack's development CA, one per publisher host | open: Platform PKI and the production host list |
| U-PP5 | the producer child chooses whether the epoch row is shared with LCFA; `F_pub` is separate | open: the production restore runbook |
| U-PP6 | deletion, sale and sanction exclusions follow their decisions as they land (HIGHSCORES-0 §11) | open |
| U-PP7 | Atlas does not consume v1 | open: an Atlas export profile |

## 11. Implementation status

Nothing is implemented. Producer children (Game) and the Platform consumer (P8 DECANARY-PUBLIC-1,
Oteryn/Oteryn-Platform#1476) each need their own allocation after acceptance. Until the consumer
is enabled in a stack, Platform's current public pages are unchanged by this contract.

## 12. Acceptance

Pending. Acceptance freezes revision 1 as the v1 wire for `testing` and `preproduction`. After
acceptance, any change to a member, path, bound or response is a new `contract_version`.
