# Oteryn Game Guilds v1

- Status: **CANDIDATE**, revision 1 (GUILD-0 contract), 2026-10-10. It authorizes no runtime,
  migration, wire allocation, Platform, Atlas or production work. Acceptance needs exact-head
  validation, independent review (persistence, protocol, privacy and security) and owner or
  architect acceptance relayed by the control plane. Each implementing child needs its own #1622
  allocation.
- Contract ID: `oteryn-game-guilds-v1`
- Coordination: #1622 (control plane); #162 (history of the GUILD-0 decision and its packets).
- Producer and authority: `Oteryn/Oteryn-Game`. Consumers: the Oteryn client (through
  `protocol-oteryn`), the CHAT-0 World relay and the Platform public read model (through
  `oteryn-game-public-projections-v1`, read-only). Atlas is not a consumer of v1.
- Builds on, and never re-decides:
  - GUILD-0 (`docs/architecture/reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md`,
    `GUILD0-GUILDS-AND-GUILDHALLS-V1`), accepted by ACCEPT-SOCIAL-MAP-0
    (`docs/architecture/reviews/OTERYN_GAME_ACCEPT_SOCIAL_MAP0_ACCEPTANCE_DECISION_2026-10-04.md`).
    Every rule, row (`GUILD0-RL-*`) and lock order (`GUILD0-LO-01`) cited here is GUILD-0's;
  - SOCIAL-MAP packets §1.2 and §2.2 (`docs/architecture/reviews/OTERYN_GAME_SOCIAL_MAP_PACKETS_2026-10-04.md`):
    the GUILD-1 packet, its owned paths and acceptance tests;
  - `docs/contracts/CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md`: Platform owns Accounts and
    Character ownership; Game owns Character gameplay state; no Platform direct database write;
  - `docs/contracts/OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md` §4.2 and §4.4 (candidate): the public
    `guild` and `character_profile` families;
  - CHAT-0 §5 with its GUILD-0 §8 amendment (the guild-room line);
  - `GUILD_ACTIVITY_RETENTION_V1` in `docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json`;
  - the multichannel scope matrix guild rows, the social presence owner baseline and the UUIDv7
    identity baseline (`GuildId`).
- A conflict between this contract and GUILD-0 is resolved in favour of GUILD-0 and fixed here
  before acceptance. After acceptance, any change to a command, result, wire member, bound or
  projection is a new `contract_version`.

## 1. Purpose and scope

GUILD-0 decided how guilds work. This contract fixes the integration surface that the
implementing children build against and that reviewers test: who is authority, which durable
state exists and who writes it, the command set and its results, what each consumer sees, how it
fails and which conformance cases prove it.

**In scope (the playable guild path):** found, invite, revoke, accept, decline, leave, exclude,
set rank, edit ranks, set title, set message, resign, disband; the formation, vice and Premium
World jobs; the guild room in chat; the guild shown in creature information; the members' own
guild view; the public `guild` projection.

**Out of scope here** (each by its GUILD-0 child or later decision): the guild bank and disband
payout (GUILD-BANK-1), guildhalls and guild ACL entries (GUILDHALL-1), guild wars (GUILD-WAR-0),
applications, autorank, board, events, leader election, a Platform administration surface.
Their commands are reserved in the wire oneof (§5) only so that they are added without renumbering.

## 2. Authority and ownership

| Fact | Owner and truth | Writer | Others |
|---|---|---|---|
| Guild, ranks, members, invitations, guild state and deadlines | Game | Game guild transactions and Game World jobs only | Platform: public read model only; client: display only |
| Account leadership position (`game_account_guild_leadership`) | Game | the same guild transactions | Platform never reads or writes it |
| `AccountId` of a Character | Platform (Character Authority boundary) | never written by guild code | Game reads the committed Character root binding |
| `WorldId`, World registry | Platform Registry | never minted by Game | Game copies it |
| `GuildId` | Game (UUIDv7) | the found transaction | opaque everywhere else |
| Premium (`premium_current`) | PREM-1 seam | not written by guild code | read by the founding rule and the daily Premium job only |
| Junior (BANK-0 §4.4) | BANK-1 predicate | not written by guild code | read by found and resign |
| Guild activity events | Game event outbox | guild transactions | member read only, `GUILD_ACTIVITY_RETENTION_V1` |

- **Game is the only guild authority.** No Platform route, web page, Atlas export or client
  message changes guild state except the Game commands of §5. A read model that disagrees with
  Game is wrong.
- **No Platform guild command in v1.** The manual's web administration is delivered in game
  (GUILD-0 §12). A later Platform view reads the same state; it is a new contract.
- **Platform obligations that touch guilds** (stated here so Platform contracts can cite them):
  a Character transfer between Accounts (Bazaar) or a deletion of a leader or vice is refused or
  routed through Game first (GUILD-0 §3.5); Platform never infers guild facts from its own data.
- **Scope.** World-scoped, strong durable, one state on every Channel. No guild row, command,
  event or projection carries a `ChannelId`, node, instance or position.
- **CharacterRevision.** Guild rows are not Character state: no guild write advances it
  (GUILD-0 §3.5).

## 3. Durable state

The tables are GUILD-0 §3.1, built by GUILD-1 in one migration. This contract fixes their
invariants as testable statements. Each is enforced by the database (constraint, unique index or
deferred guard) and not only by application code.

| Id | Invariant | Source |
|---|---|---|
| GI-01 | A Character is a member of at most one guild (`character_id` primary key of members). | §3.1 |
| GI-02 | A guild that is not `DISBANDED` has exactly one level-1 member; every member's World equals the guild's World. | §3.1 |
| GI-03 | `name_key` is unique per World among guilds not `DISBANDED`; guild and character names are separate namespaces. | §3.1 |
| GI-04 | Ranks: 3 to 20 per guild (`GUILD0-RL-06`), levels contiguous from 1, (guild, rank `name_key`) unique. | §3.1 |
| GI-05 | An Account holds at most one leader-or-vice position across all Worlds, and the leadership rows equal the set of level-1 and level-2 members. | §3.1 |
| GI-06 | Invitations: at most 500 per guild (`GUILD0-RL-08`), at most 50 per target character counting expired rows (`GUILD0-RL-17`), never for a member of the same guild. | §3.2 |
| GI-07 | Members per guild at most 2,000 (`GUILD0-RL-05`). | §3.2 |
| GI-08 | State moves only `FORMING`→`ACTIVE`, `FORMING`/`ACTIVE`→`DISBANDING`→`DISBANDED`; a guild row is never deleted. | §3.1, §3.4 |
| GI-09 | `DISBANDED` holds no member, invitation, leadership row, balance, guildhall or open guildhall bid (fail closed; the step retries). | §3.4 |
| GI-10 | Every guild transaction writes exactly one guild event (§4.4) in the same transaction; a refused or fenced-out command writes none. | §4.4 |
| GI-11 | Every guild `revision` advances in the same committed transaction as the change; a public projection revision advances in that transaction only when a field it projects changes (PUBLIC-PROJ §5). | PUBLIC-PROJ §2.3, §5 |

- **Session-generation fence.** Every player command takes the acting Character's session fence
  and root FOR UPDATE in its transaction (composition rule 2, `GUILD0-LO-01` position 2). A stale
  generation, lease or node commits nothing. An invite also locks the target's root FOR UPDATE
  without a write and without a session fence.
- **World jobs** take the recovery fence and admission relations, no session fence and no
  Character root, and start at `GUILD0-LO-01` position 3 (GUILD-0 §4.1).
- **Lock order.** `GUILD0-LO-01` binds every guild transaction. A transaction that would take a
  class out of order rolls back and retries; it never waits out of order.
- **Replay.** Each command carries an occurrence and a SHA-256 request binding. The same
  occurrence with the same binding returns the stored outcome and writes nothing; with another
  binding it is `REJECTED`.

## 4. Lifecycle summary

```text
found ─► FORMING ──(4 vices before formation_deadline)──► ACTIVE
            │                                               │
            │ deadline passed (World job)                   │ vice deficit > 14 d, Premium deficit > 14 d
            ▼                                               ▼ (World job), or disband by the leader
        DISBANDING ◄────────────── disband by the leader ───┘
            │ keyed steps: guildhall release, payout, members and invitations (100 per step)
            ▼
        DISBANDED (name free, row kept)
```

- **State gate** (`GUILD0-RL-15`): only `FORMING` and `ACTIVE` admit commands; `DISBANDING` and
  `DISBANDED` refuse with `GUILD_DISBANDING`, except the disband steps and the disband claim.
- **Deadline gate** (`GUILD0-RL-18`): past `formation_deadline` in `FORMING`, or with
  `vice_deficit_since` older than 14 days in `ACTIVE`, no accept or rank change may activate or
  cure the guild (`GUILD_DEADLINE_PASSED`).
- **Premium** (owner answer G1 a, PREMIUM-ACTIVATION-0 §1.2 and §2.2): only `NotActivated`
  bypasses the rule; then no Premium rule applies and the Premium job writes nothing. Once
  activated, founding and every move into levels 1 and 2 require `Current`; `NotCurrent`,
  including an unavailable source or clock, is refused `NOT_PREMIUM` and counts as a deficit for
  the Premium job. A lapse keeps the rank.
- **Effect time.** Every committed change applies at once on every Channel (declared difference
  from the manual's logout and server-save rules, GUILD-0 §3.2).

## 5. Commands

All commands are `GUILD_INTENT` variants (a oneof; empty is `REJECTED`) under capability
`GUILD_V1`, except the query (§6.1). The capability number, the two command type numbers and the
state domain number are leased by the control plane at GUILD-WIRE-1 allocation; this contract
fixes names and semantics, not numbers. Edits that change ranks, titles or the message carry the
expected guild `revision` (`STALE_REVISION` on mismatch).

| Intent | Actor | Preconditions (each refusal writes nothing) | Effect |
|---|---|---|---|
| `found {name}` | any Character | in no guild (`ALREADY_IN_GUILD`); not junior (`JUNIOR`); Account holds no position (`ACCOUNT_HAS_POSITION`); name valid, ≤ 29 letters (`NAME_INVALID`); `name_key` free in the World (`NAME_TAKEN`); Premium once activated (`NOT_PREMIUM`) | guild `FORMING`, deadline +3 d, default ranks, founder level 1, leadership row |
| `invite {character}` | level 1 or 2 | target of the same World and not a member of this guild (`NOT_ALLOWED`); limits GI-06 (`INVITATION_LIMIT`) | invitation, expiry +30 d (`GUILD0-RL-07`) |
| `revoke_invite {character}` | level 1 or 2 | an invitation exists (`NOT_INVITED`) | invitation deleted |
| `accept {guild}` | invited Character | in no guild; invitation present and unexpired (`NOT_INVITED`); `GUILD_FULL`; state and deadline gates | joins at the lowest level; all its invitations deleted (≤ 50) |
| `decline {guild}` | invited Character | an invitation exists (`NOT_INVITED`) | its invitation deleted |
| `leave` | member, not level 1 | member (`NOT_ALLOWED`) | membership and, for level 2, the leadership row removed |
| `exclude {character}` | level 1 or 2 | target at a strictly lower level (`NOT_ALLOWED`) | as leave, for the target |
| `set_rank {character, level}` | member at level `a` | target level `t`, new level `n`: `a < t` and `a < n` (`NOT_ALLOWED`); into level 2 needs the target's Account free (`ACCOUNT_HAS_POSITION`) and Premium once activated; deadline gate | level moved; leadership rows follow; may activate or set or clear the vice deficit |
| `edit_ranks {names[3..20]}` | level 1 | names valid and pairwise distinct by `name_key` (`NAME_TAKEN`); revision | ranks replaced; members of a removed level move to the new lowest level |
| `set_title {character, title}` | level 1 | target is a member; title ≤ 29 characters, CHAT-0 text rules (`NAME_INVALID`) | title set |
| `set_message {text}` | level 1 or 2 | ≤ 255 characters, CHAT-0 text rules (`NAME_INVALID`); revision | message set |
| `resign {successor}` | level 1 | successor is a level-2 member that is not junior (`NOT_ALLOWED`, `JUNIOR`) | levels 1 and 2 swap; leadership rows follow |
| `disband {confirm}` | level 1 | `confirm` equals the guild name (the GUILD-0 §3.2 confirmation, `NOT_ALLOWED` otherwise) | state `DISBANDING`; the disband job runs |
| `deposit`, `withdraw`, `claim_disband_payout` | — | reserved for GUILD-BANK-1 | — |

- **Results** (closed): `OK`, `NOT_ALLOWED`, `NAME_TAKEN`, `NAME_INVALID`, `ALREADY_IN_GUILD`,
  `ACCOUNT_HAS_POSITION`, `NOT_PREMIUM`, `GUILD_FULL`, `NOT_INVITED`, `JUNIOR`,
  `INSUFFICIENT_FUNDS`, `BALANCE_LIMIT`, `STALE_REVISION`, `GUILD_DISBANDING`,
  `GUILD_DEADLINE_PASSED`, `INVITATION_LIMIT`, plus the common foundation results. A refusal
  never reveals more than the actor may read (§6): an invite of an unknown or other-World name is
  `NOT_ALLOWED`, never "no such character".
- **`decline`** is added to the GUILD-0 §9 oneof. It is the invited Character deleting its own
  invitation row; the GUILD-1 packet already lists it. It grants nothing and needs no new rule.
- **Junior** follows BANK-0 §4.4 as BANK-1 delivers it; until that fact exists for a Character it
  is not junior.

## 6. Projections

### 6.1 Own guild (`GUILD_QUERY`, members and the invited)

Paged per `GUILD0-RL-16`: sections `SUMMARY`, `ROSTER`, `INVITATIONS`, `ACTIVITY`,
`DISBAND_CLAIMS` (reserved for GUILD-BANK-1) and `PUBLIC` (another guild by name). A list page has
at most 100 entries of at most 512 encoded bytes each, excluding each entry's field tag and
length prefix (a page is bounded by 100 × 512 B plus a 1,024 B header for that framing, the cursor
and the section fields), and a keyset `next_cursor`; a malformed cursor or
one of another guild or section is `REJECTED`.

| Section | Reader | Contents |
|---|---|---|
| `SUMMARY` | member | name, state, ranks, message, own rank and title, deadlines |
| `ROSTER` | member | member names, levels, titles, vocations, online flag |
| `INVITATIONS` | member; the invited Character sees its own | guild name, invited name, expiry |
| `ACTIVITY` | member | own guild's entries within 30 days (`GUILD0-RL-10`) |
| `PUBLIC` | any Character of the World | name, state, ranks, member names, levels, vocations, online flag |

- Never in any section: a `ChannelId`, position, Account, another Character of the same Account,
  or another guild's activity entry (social presence baseline, GUILD-0 §4.2).
- The online flag is World-level only (online on this World), as `world_online`.

### 6.2 Creature information (guild on visible characters)

The client shows, for each visible player character, whether and how it belongs to a guild.

- A new state domain `GUILD_BADGES` under `GUILD_V1` carries, per visible player entity of the
  session's `WORLD_SPATIAL_VISIBILITY` set, one badge keyed by the entity `identity`:
  `emblem` (`NONE`, `OWN_GUILD`, `OTHER_GUILD`; GUILD-0 §4.3), and, unless `NONE`, `guild_name`
  and `rank_name` (§12 Q1). `WorldSpatialEntityV1` is unchanged, so its 128-byte bound and every
  existing session without `GUILD_V1` are unaffected.
- Each badge message is at most 96 encoded bytes, excluding its field tag and length prefix; one
  snapshot covers at most the 256 visibility entities (MOVE-RL-11), so at most 25,600 bytes
  (256 × 96 B + 1,024 B header for the per-badge tags and length prefixes, 2 B each, and the
  snapshot's own fields), as `world_spatial_v1.proto` budgets visibility; deltas follow the
  visibility enter, update and leave sets under the same per-badge bound.
- **Source.** The Channel runtime derives a badge from the committed membership read, never from
  client data. A committed membership, rank-name or state change makes the World guild owner
  notify each Channel of the World; each Channel refreshes the badges of the affected visible
  characters in its next delta. A badge is display only: it never grants access, chat delivery,
  ACL or PvP standing.
- **Fail closed.** A badge that cannot be resolved is sent as `NONE`. A `DISBANDING` guild shows
  its badge until `DISBANDED`.
- The title and the guild message are not in badges (members-only, §6.1).

### 6.3 Public read model

`PublishGuildV1` and the guild member of `character_profile` follow
`OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md` §4.4 and §4.2 unchanged. A guild transaction advances the
`guild` revision only for a change in its §5 row (state, name, ranks, roster, a member's rank),
and a member's `character_profile` revision only for a join, leave or rank change, in its
transaction (GI-11). Invite, revoke, decline, set title and set message change no projected
field and advance no public revision. `DISBANDED` publishes `guild: []`.

### 6.4 Activity log

One guild event per transaction (GI-10), in `GuildEventV1` (`docs/contracts/game-events/v2/guild_event.proto`,
added by GUILD-1), retained under `GUILD_ACTIVITY_RETENTION_V1`. Expiry never touches guild
tables.

## 7. Guild chat

GUILD-0 §8 and the CHAT-0 §5 amendment, unchanged: one guild room per guild on the World relay;
members are auto-joined at login and shown the guild message; a line carries the sender's
`GuildId` and rank level, taken from its committed membership at send (not a member:
`NOT_ALLOWED`); a receiving node delivers only to sessions whose Character is, by a committed read
at delivery, a member of the payload's `GuildId`; a failed read drops the line (counted); leader
and vice names render distinctly. Bounds: plaintext ≤ 1,188 bytes, sealed ≤ 1,217, base64 ≤ 1,624,
under `CHAT0-RL-06`.

## 8. Failure modes

| Failure | Required behaviour |
|---|---|
| Stale session generation, lease or node on a command | nothing commits; the client gets the common fence result |
| Two commands race on one guild | serialized by the guild row lock (`GUILD0-LO-01` position 3); the loser re-checks under the lock |
| Two guilds found the same name | the unique index decides; the loser gets `NAME_TAKEN` |
| Two guilds invite one target past its 50-row limit | serialized on the target root (position 2); one gets `INVITATION_LIMIT` |
| Accept races exclude, disband or the deadline | the state and deadline gates under the guild lock decide; nothing partial commits |
| A World job crashes mid-disband | each step is keyed (guild, step) and idempotent; the next pass resumes; `DISBANDED` refuses while GI-09 does not hold |
| A late World job | it re-checks under the guild lock; the deadline gate keeps a due guild due |
| Command replayed after an ambiguous result | the stored outcome by occurrence; another binding is `REJECTED` |
| Premium not activated | Premium rules do not apply (G1 a); the Premium job writes nothing |
| Premium activated, source or clock unavailable | `NotCurrent` (PREMIUM-ACTIVATION-0 §1.2): founding and moves into levels 1 and 2 are `NOT_PREMIUM`; the Premium job counts the deficit; ranks held are kept |
| Relay node cannot read membership | the guild line is dropped and counted; never delivered on stale data |
| Badge resolution fails | `NONE` (§6.2) |
| Public publisher down | Platform keeps its last snapshot; the next snapshot replaces it (PUBLIC-PROJ §2.1) |
| Platform moves a leader or vice to another Account | refused until Game releases the position (§2) |

## 9. Conformance cases

Each case changes one fact and keeps the rest valid. GUILD-1 owns GC-01 to GC-22, GUILD-WIRE-1
GC-23 to GC-28, GUILD-CHAT-1 GC-29 to GC-31, the public publisher GC-32.

| Case | Given | Expect |
|---|---|---|
| GC-01 | a valid found | `OK`; `FORMING`, deadline +3 d, 3 ranks, founder level 1, leadership row, one event |
| GC-02 | found by a member of another guild | `ALREADY_IN_GUILD`, nothing written |
| GC-03 | found by a Character whose Account leads or vices on another World | `ACCOUNT_HAS_POSITION` |
| GC-04 | found with a name taken by a live guild of the World | `NAME_TAKEN`; the same name on another World or of a `DISBANDED` guild is `OK` |
| GC-05 | found with 30 letters or an invalid character | `NAME_INVALID` |
| GC-06 | found while Premium not activated, then after activation without Premium, then after activation with the Premium source unavailable | `OK`, then `NOT_PREMIUM`, then `NOT_PREMIUM` |
| GC-07 | invite by level 3 | `NOT_ALLOWED` |
| GC-08 | the 501st invitation of a guild; the 51st row for one target | `INVITATION_LIMIT` |
| GC-09 | accept of an expired invitation | `NOT_INVITED` |
| GC-10 | accept that would make member 2,001 | `GUILD_FULL` |
| GC-11 | accept deletes the Character's other invitations | none remain |
| GC-12 | leave by the leader | `NOT_ALLOWED` |
| GC-13 | exclude of an equal-level member | `NOT_ALLOWED` |
| GC-14 | set_rank by level 2 to level 2 | `NOT_ALLOWED`; by level 1 to level 2 of a target whose Account has a position: `ACCOUNT_HAS_POSITION` |
| GC-15 | the fourth vice in `FORMING` before the deadline | `ACTIVE` in the same transaction |
| GC-16 | the fourth vice after `formation_deadline` | `GUILD_DEADLINE_PASSED`; the next job pass disbands |
| GC-17 | `ACTIVE` drops to 3 vices, then 14 d pass | `vice_deficit_since` set; the job disbands after `GUILD0-RL-04` |
| GC-18 | edit_ranks with two names of one `name_key` | `NAME_TAKEN`, nothing written; removing a level moves its members to the lowest level |
| GC-19 | resign to a level-3 member or a junior vice | `NOT_ALLOWED` or `JUNIOR`; to a valid vice, levels and leadership rows swap |
| GC-20 | any command on a `DISBANDING` guild | `GUILD_DISBANDING` |
| GC-21 | disband with 250 members; crash after the first step | `DISBANDED` after resumption, no member, invitation or leadership row left, name free |
| GC-22 | any command with a stale session generation | nothing commits, no event |
| GC-23 | replay of one occurrence with the same binding; with another binding | the stored outcome; `REJECTED` |
| GC-24 | `GUILD_QUERY` `ROSTER` with 250 members | three pages of ≤ 100, keyset order rank level then CharacterId |
| GC-25 | a cursor of another guild or section | `REJECTED` |
| GC-26 | `PUBLIC` of another guild | no titles, message, invitations, activity, Account or Channel data |
| GC-27 | a visible member of the viewer's guild, of another guild, of none | badges `OWN_GUILD`, `OTHER_GUILD`, `NONE`; name and rank per §12 Q1 |
| GC-28 | a member is excluded while visible | its badge becomes `NONE` in the next delta on every Channel of the World |
| GC-29 | a guild line from a member | delivered to members on every Channel of the World only |
| GC-30 | sender leaves before delivery | not delivered to the sender's new guild; receivers check the payload `GuildId` |
| GC-31 | the receiving node's membership read fails | dropped and counted |
| GC-32 | a guild becomes `DISBANDED` | the next `PublishGuildV1` carries `guild: []` |

## 10. Reference evidence

- `docs/reference/tibia-manual/guilds.md` (capture 2026-09-28): §5.8.1 one guild per character;
  §5.8.2 leader, at least 4 vices, Premium for leader and vice, unlimited members; §5.8.3 found,
  invite and accept, exclude, leave, edit ranks, resign, disband; §5.8.4 wars (out of scope).
- `docs/reference/tibia-manual/houses.md` §5.7 (guildhalls, out of scope here).
- Declared differences (GUILD-0 §13): immediate effect of changes and resignation; bounded
  members and invitations; in-game administration instead of the web. Legacy server sources
  (Canary, Crystal) are reference evidence only; no legacy schema, packet or name is carried over.

## 11. Implementation order

1. GUILD-1 (§3, §4, §5 durability; GC-01 to GC-22), as SOCIAL-MAP packets §2.2.
2. GUILD-WIRE-1 (§5 wire, §6.1, §6.2; GC-23 to GC-28) and GUILD-CHAT-1 (§7; GC-29 to GC-31), in
   parallel after GUILD-1.
3. The public `guild` publisher (§6.3; GC-32) with the public projection children.
4. GUILD-BANK-1, then GUILDHALL-1, unchanged from GUILD-0.

## 12. Open questions for the control plane

**Q1. What does creature information show for another character's guild?** GUILD-0 §4.3 fixes
only the emblem; the playable path asks for the guild name. The guild name and rank names are
already public (GUILD-0 §4.2 public view); titles are members-only.
a) emblem, guild name and rank name, no title (recommended: no new privacy reading, the
manual's look text minus the title);
b) emblem only, as GUILD-0 §4.3;
c) emblem, guild name, rank name and title (needs a privacy ruling, since titles are
members-only).

Until answered, §6.2 is written for a.
