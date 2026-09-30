# GUILD-0 Guilds and guildhalls

- Decision: `GUILD0-GUILDS-AND-GUILDHALLS-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence,
  economy, security and protocol) and protected integration. Owner questions G1 and G2 (§14) are
  open; §3.2 and §7 apply their recommended answers as reversible assumptions.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the owner's direction (2026-09-30, verbatim: "mozesz sie tez zajac gildami bo blokuej mi
  to wdrozenie guildhalli do domkow"); it fills the guild lifecycle that EXP-HOUSES-01 §9 defers and
  the guildhall and guild ACL gaps HOUSE-OWN-0 §3, §10 and §15 leave open
- Builds on: EXP-HOUSES-01 (owner-accepted) §3, §9, §12.3, §14, §16 and §17; HOUSE-OWN-0
  (`game_house_properties`, auction, rent, disposition, ACL, World jobs, the house `FeeBurnCause`
  variants of D238); HOUSE-CUSTODY-0 (`HouseInterior`, reclaim provenance); BANK-0 (balance,
  ledger, junior rule); MARKET-0 (`CharacterInbox`, the balance hard ceiling); CHAT-0 (the World
  relay); PREMIUM-DELIVERY-0 (`premium_current(account)`); the UUIDv7 identity baseline (`GuildId`);
  the social presence baseline (membership grants no location); the scope matrix guild rows;
  DUR-03 §15, §17, §18 and §34; D178; owner rule 5905825574 (Global parity)
- Amends, each pending on acceptance of GUILD-0, in this PR: EXP-HOUSES-01 §9 (pointer);
  HOUSE-OWN-0 §3 and §10 (guildhall rows, guild ACL entries); the House catalogue contract `kind`
  (pointer); BANK-0 §3 (the guild ledger kinds, §5 here).
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| GUILD-1 | hard, persistence and security review | guild, rank, member, invitation and account leadership tables; found, invite, join, leave, exclude, rank, resign and disband transactions; the formation and vice World jobs (§3, §4) | this decision; GUILD-RET-0 |
| GUILD-RET-0 | control plane routes; privacy review | the retention profile of the guild event (the 30-day activity log, §4.4) | this decision |
| GUILD-BANK-1 | hard, persistence and economy review | the guild balance and ledger, deposit and withdraw, the disband payout (§5) | GUILD-1; BANK-1 |
| GUILDHALL-1 | hard, persistence, economy and security review | guildhall property rows, the guild bid with split escrow, guild-first rent, the guild ban, the `GUILD_DISBAND` disposition, guild ACL entries (§6, §7) | GUILD-1; GUILD-BANK-1; HOUSE-1; HOUSE-ACL-1 |
| GUILD-WIRE-1 | impl, protocol review | capability `GUILD_V1`, the guild commands and domain, the nameplate emblem, the guildhall variants of `HOUSE_INTENT` (§9) | GUILD-1; GUILD-BANK-1; GUILDHALL-1 for the guildhall variants; HOUSE-WIRE-1 |
| GUILD-CHAT-1 | impl, security review | one guild room per guild on the CHAT-0 World relay, auto-joined; the guild message at login (§8) | GUILD-1; CHAT-2 |

GUILD-1, GUILD-RET-0 and GUILD-BANK-1 do not wait for houses. GUILDHALL-1 follows HOUSE-1 and
HOUSE-ACL-1, which follow the house interior runtime (HOUSE-CUSTODY-0 §4), so no guildhall is
auctioned before it can be entered. Later, each with its own decision: guild wars (GUILD-WAR-0,
after PARTY-PVP-0: war declaration, fees, kill score, the war emblem, the PZ and assistance rules),
applications, autorank, the guild board and events, the Platform web view of guilds, leader
election, name wildcards in ACL lists, and Rested in guildhalls.

## 1. Question

How does a guild exist, who belongs to it and in which rank, how does it hold gold, and how does
it rent and run a guildhall?

## 2. Facts

**PROVEN**

- EXP-HOUSES-01 §9 (owner-accepted): the Guildhouse owner is `GuildId`; it does not consume the
  personal (Account, World) slot; selling the leader Character does not transfer it; acquisition,
  succession, rank ACL, rent funding and dissolution wait for the guild system. §12.3 and §14:
  forgotten items are neither destroyed nor gifted. §17: no spell edits an access list.
- House catalogue: 66 `guildhall` records with `rent_gold`, tiles and doors.
- HOUSE-OWN-0 (on `main`): property rows for `private_house` and `shop` only ("Guildhalls get no
  row"); auction with escrowed proxy bids; rent from the bank every 30 days with a 7-day grace;
  the disposition under a database-enforced content fence, each item to its reclaim subject's
  Inbox; flat ACL lists ("Guild and wildcard patterns wait for guilds"); `HOUSE_PRICE` and
  `HOUSE_RENT` are burns under the house variants of `FeeBurnCause` (owner answer H1, D238).
- The scope matrix: guild membership is World-scoped, strong durable, the same on every channel;
  guild chat is a World room. The UUIDv7 baseline lists `GuildId` as a Game-issued UUIDv7.
- The social presence baseline: membership of the same guild does not by itself grant exact
  location; guild rosters must not reveal hidden alternate characters.
- BANK-0: one balance per (Account, World); a junior character cannot use the bank; credits stop
  at `BANK0-RL-01`; returns of value already owned may reach the hard ceiling
  9,000,000,000,000,000 (`HOUSEOWN0-RL-13`, `MARKET0-RL-10`).
- PREMIUM-DELIVERY-0: `premium_current(account)` is PREM-1's one gameplay read; PREM-1a is frozen
  (#1391), the snapshot client is not built; no Premium exists yet. HOUSE-OWN-0's owner answer H2a
  set the precedent: a Premium rule waits until Premium is delivered.
- `0022`: character names are one namespace keyed by `name_key`; roots are never deleted.

**CIPSOFT_OFFICIAL** (the Tibia manual, `guilds.md` §5.8 and `houses.md` §5.7)

- A character belongs to at most one guild. One leader; a new guild must have at least 4 vice
  leaders within 3 days of founding or it is disbanded; if an existing guild drops below 4 vices,
  the leader has 2 weeks to replace them. Up to 20 ranks: rank 1 is the leader, rank 2 the vice
  leaders, the rest are members.
- Only Premium accounts found a guild or hold leader or vice rank; one leader-or-vice position per
  Account, across Worlds; a lapse of Premium keeps the rank; fewer than 5 Premium players among
  leader and vices disbands the guild after 2 weeks.
- Guild names follow the character name rules, at most 29 letters, and never change.
- Leader and vices invite; the invited character accepts. Leader or vice excludes; anyone but the
  leader leaves. A member promotes or demotes only members ranked strictly below itself, and never
  up to its own rank. The leader resigns to a successor whose Account holds no other leader or
  vice position; a rented guildhall follows the new leader.
- Guild bank account: any member deposits; leader and vices withdraw; a guild activity log of 30
  days (joins, rank changes, bank). Disbanding clears the guildhall.
- Only a guild leader rents a guildhall, at most one, in addition to its own houses. A guildhall
  bid draws on the guild bank first, then the leader's bank. Every guildhall has a depot locker.
  ACL lists accept `*@guild` (all members) and `rank@guild` (one rank), and `!` excludes.

## 3. Guild and membership (GUILD-1)

### 3.1 Tables

- **Guild.** `game_guilds`: `guild_id` (UUIDv7, Game-issued), `world_id`, `name`, `name_key`
  (unique per World while not `DISBANDED`), state (`FORMING`, `ACTIVE`, `DISBANDING`,
  `DISBANDED`), `formation_deadline`, `vice_deficit_since`, `motd` (at most 255 characters, the
  CHAT-0 text rules), `revision`, `created_at`. A row is never deleted; `DISBANDED` frees the name.
- **Ranks.** `game_guild_ranks`: (guild, level 1 to `GUILD0-RL-06` (20), name). Level 1 is the
  leader, level 2 the vice leaders, levels 3 and above members. A new guild gets "Leader", "Vice
  Leader" and "Member".
- **Members.** `game_guild_members`: `character_id` (primary key: at most one guild per
  character), guild, rank level, `title` (at most 29 characters), `joined_at`. A deferred guard
  keeps exactly one level-1 member per guild that is not `DISBANDED`, and the member's World equal
  to the guild's.
- **Invitations.** `game_guild_invitations`: (guild, character), inviter, `expires_at` = +30 days
  (`GUILD0-RL-07`, `PARITY_PENDING`: the manual gives no expiry).
- **Leadership positions.** `game_account_guild_leadership`: `account_id` primary key, guild,
  character. Written with every move into or out of levels 1 and 2, so one Account holds at most
  one leader-or-vice position across all Worlds, as in Tibia.
- **Scope.** World, strong durable, one state on every channel (the scope matrix row). Game owns
  guild truth; Platform may later show it read-only.

### 3.2 Operations

Each is one transaction and one guild event (§4.4), replayed by its occurrence and a SHA-256
request binding, like HOUSE-OWN-0 §4 "Replay".

- **Found `{name}`.** The acting character is in no guild and not junior (BANK-0 §4.4); its
  Account holds no leadership position; the name passes the `0022` character name rules, has at
  most 29 letters, and its `name_key` is free in the World among guilds (guild and character names
  are separate namespaces). The guild starts `FORMING` with `formation_deadline` = +3 days
  (`GUILD0-RL-02`), the founder at level 1. **Premium** (assumption pending G1, answer a): not
  required until PREM-1 delivers `premium_current`; from then on founding and every move into
  levels 1 and 2 require it (`NOT_PREMIUM`), and a lapse keeps the rank.
- **Invite `{character}` / revoke.** By levels 1 and 2; the target is of the same World and not a
  member of this guild; at most `GUILD0-RL-08` (500) open invitations per guild.
- **Accept.** By the invited character, which is in no guild; it joins at the lowest level. At most
  `GUILD0-RL-05` (2,000, `PARITY_PENDING`: Tibia has no limit) members per guild; a further join is
  `GUILD_FULL`. Accepting removes the character's other invitations.
- **Leave.** Any member but the leader.
- **Exclude `{character}`.** By levels 1 and 2, of a member at a strictly lower level.
- **Set rank `{character, level}`.** The actor's level `a`, the target's current level `t` and new
  level `n` satisfy `a < t` and `a < n`; so only the leader sets level 2. A move to level 2 writes
  the target's Account leadership row and needs it free (`ACCOUNT_HAS_POSITION`).
- **Edit ranks `{names}` / set title / set message.** Leader only for ranks and titles (3 to 20
  names; removing a level moves its members to the new lowest level); levels 1 and 2 set the
  message.
- **Resign `{successor}`.** The leader names a vice of the guild that is not junior (so the
  leader's bank may fund guildhall costs, §6-§7); the two swap levels 1 and 2, the
  leadership rows follow (the successor's Account already holds this guild's position). Immediate:
  the manual's "next server save" is a web-admin artifact (declared difference). A guildhall stays
  the guild's; its owner rights follow the new leader (§7.3).
- **Disband.** Leader only, with a confirmation; the state becomes `DISBANDING` and §3.4 runs.
- **Effect time.** Every change applies at once on every channel. The manual's "an online member
  stays until logout" is a web-admin artifact (declared difference).

### 3.3 Formation and vice rules (World jobs)

- `FORMING` becomes `ACTIVE` in the transaction that brings the guild to at least 4 vices
  (`GUILD0-RL-03`). A `FORMING` guild past `formation_deadline` is disbanded by a World job.
- In `ACTIVE`, a transaction that leaves fewer than 4 vices sets `vice_deficit_since`; one that
  restores 4 clears it. A World job disbands a guild whose deficit is older than 14 days
  (`GUILD0-RL-04`).
- The Premium rule (fewer than 5 Premium players among leader and vices for 14 days disbands) waits
  for G1 and PREM-1: from then on a daily World job reads `premium_current` per leadership Account
  and keeps `premium_deficit_since` the same way.
- Jobs follow HOUSE-OWN-0 §9: candidates without a lock, then locked and re-checked; at most
  `GUILD0-RL-09` (100) guilds per pass; idempotent per key.

### 3.4 Disband (World job steps)

1. If the guild owns a guildhall or holds a guildhall bid, the §7.4 disposition or the bid release
   runs first; the guild waits in `DISBANDING`.
2. The guild balance is paid out (§5.3).
3. Steps remove members and invitations, at most 100 per step, keyed by (guild, step).
4. The last step frees the leadership rows and sets `DISBANDED`; the name is free.

### 3.5 Characters and Accounts

- A future deletion workflow refuses to delete a guild leader until it resigns or disbands (leader
  election is a later decision). A deleted member leaves first.
- The Character Bazaar decision must move or refuse the leadership row when a leader or vice
  changes Account; the guildhall never moves with the character (EXP-HOUSES-01 §9).
- No `CharacterRevision` advance: guild rows are not Character state (composition rule 1).

## 4. Locks, visibility and evidence (GUILD-1)

### 4.1 Lock order

The operation occurrence; the acting Character's session fence and root FOR UPDATE (composition
rule 2); the guild row; the member rows by CharacterId; the invitation rows; the leadership rows by
AccountId; the guild balance row; then the bank balance rows by `account_id` (BANK-0 §4.1). A
target character's root is read without a row lock: its Account and World cannot change. World
jobs take only the recovery fence and admission relations, then the guild row onward.

### 4.2 Roster and presence

- Any character of the World sees a guild's name, ranks, member names, levels, vocations and an
  online flag (as the Tibia guild page); never a channel or position (the social presence
  baseline). Members see titles and the guild message.
- A member's other characters are never linked: the roster lists member characters only.

### 4.3 Nameplates

Each visible character carries an emblem relative to the viewer: none, own guild, other guild
(GUILD-WIRE-1). The war emblem belongs to GUILD-WAR-0.

### 4.4 Guild event and activity log

One guild event per operation (join, leave, exclude, rank change, found, resign, disband, bank
entry), in a guild outbox. Members read the last 30 days (`GUILD0-RL-10`); retention is GUILD-RET-0's
profile (purpose `GUILD_ACTIVITY`, bank entries also `ECONOMY_LEDGER`).

## 5. Guild bank (GUILD-BANK-1)

### 5.1 Storage

- `game_guild_bank_balances`: one row per guild (0 to `BANK0-RL-01`), `last_entry_id`.
- `game_guild_bank_entries`: immutable, the BANK-0 entry shape keyed by guild instead of Account:
  kinds `GUILD_DEPOSIT`, `GUILD_WITHDRAW`, `GUILDHALL_BID_RESERVE`, `GUILDHALL_BID_RELEASE`,
  `GUILDHALL_PRICE`, `GUILDHALL_RENT`, `GUILD_DISBAND_PAYOUT`; amount, before and after, acting
  character, the counterpart bank entry where there is one.
- BANK-0's guards apply the same way (chain by `last_entry_id`; the pair of a deposit or withdrawal
  commits together with equal amounts).
- The bank ledger gains kinds `GUILD_DEPOSIT` and `GUILD_WITHDRAW` on the member's (Account, World)
  (the BANK-0 §3 amendment); an entry references a bank, fee, Market, house or guild operation.

### 5.2 Operations

- **Deposit `{amount}`.** Any member, from its Account's bank balance to the guild balance: two
  `TRANSFER` value lines in one transaction. Not from coins: coins go to the bank first (BANK-0).
- **Withdraw `{amount}`.** Levels 1 and 2, from the guild balance to the actor's Account bank
  balance; refused above either balance or above `BANK0-RL-01` on the receiver (`BALANCE_LIMIT`).
- **Junior.** A junior character neither deposits nor withdraws (BANK-0 §4.4): the guild bank moves
  gold between Accounts, which the junior rule bars.
- Both are transfers of owned value, not a new value source: no D178 decision is needed.

### 5.3 Disband payout

After any guildhall release (§3.4), the whole guild balance moves to the leader's (Account, World)
bank balance (`GUILD_DISBAND_PAYOUT`, `TRANSFER`). It is value already owned, so it may exceed
`BANK0-RL-01` up to the hard ceiling (`HOUSEOWN0-RL-13`). The manual is silent;
`PARITY_PENDING`.

## 6. Guildhall property and auction (GUILDHALL-1)

### 6.1 Rows

- Every `guildhall` of the active catalogue gets a `game_house_properties` row, `VACANT`, with
  `owner_kind` `GUILD` (private houses and shops `CHARACTER`) and `owner_guild_id`. A guard: a
  `GUILD` row has an owner guild and no owner Character or Account when owned, uses no housing
  slot, and at most one guildhall is `OWNED`, `MOVE_OUT_PENDING` or `DISPOSITION` per guild.
- HOUSE-OWN-0's tile table, catalogue revision rules, fence, disposition, ban, job and ACL
  machinery apply unchanged except as below.

### 6.2 Bid

- `guildhall_bid {house, max}` by the guild leader only. Eligible at bid and at settlement: the
  guild is `ACTIVE`, owns no guildhall and holds no other guildhall bid, and has no guild ban; the leader is not junior (its bank
  may fund the bid). The leader's level, slot and Premium do not count: the guild is the future
  owner.
- **Split escrow.** `escrow_gold` = max + snapshotted rent, drawn first from the guild balance
  (`GUILDHALL_BID_RESERVE`), the rest from the leader's Account bank balance
  (`HOUSE_BID_RESERVE`, the Account fixed on the bid as `funding_account_id`). The bid row keeps
  `escrow_guild_gold` and `escrow_account_gold`; a guard keeps their sum equal to HOUSE-OWN-0's
  escrow rule. Raising the max draws the same way; lowering returns account gold first.
- **Settlement.** The winner's price and rent burn from the guild part first, then the account
  part (the house `FeeBurnCause` variants, under G2); the rest returns to each source. Releases
  return each part to its own source. If the leader changed after the bid, the account part still
  returns to `funding_account_id`.

## 7. Guildhall rent, ownership and disposition (GUILDHALL-1)

### 7.1 Rent

- Due as HOUSE-OWN-0 §5: the guild balance is debited first, then the current leader's Account bank
  balance for the rest (`GUILDHALL_RENT` and `HOUSE_RENT`, one burn line each); a junior leader's
  bank is never used. This resolves the
  manual's open question the same way as the bid.
- An insufficient total starts the grace; after it, eviction bans the **guild** from guildhall bids
  for 30 days (`game_guild_house_bans`); no Account is banned.
- **Price and rent as gold sinks** (assumption pending G2, answer a): the H1 house variants cover
  guildhalls.

### 7.2 Moving out

The leader sets a move-out date as HOUSE-OWN-0 §6.

### 7.3 Owner rights

The OWNER role of a guildhall is the guild's current level-1 member, resolved at check time; the
ACL rows stay with the house when the leader changes. Subowners and guests are edited as
HOUSE-OWN-0 §10.

### 7.4 Disposition

HOUSE-OWN-0 §7 with a new cause `GUILD_DISBAND` (no ban) next to `MOVE_OUT`, `EVICTION` and
`CATALOGUE_RETIREMENT`. Each item goes to its reclaim subject's Inbox, as for every house: the
manual sends portable items to the leader and leaves furniture behind, which EXP-HOUSES-01 §12.3
and §14 forbid (declared difference).

### 7.5 Depot locker

The guildhall's depot locker is map content; its use (DEPOT-0, the user's own depot and Inbox)
belongs to the house interior runtime, which admits only characters with house access.

## 8. Guild chat (GUILD-CHAT-1)

- One room per guild on the CHAT-0 World relay, sealed and bounded the same way; members only,
  auto-joined at login; the guild message is shown at login. Leader and vice names render
  distinctly.
- Inviting non-members into the guild room waits for private chat channels (CHAT-0 later list).

## 9. Wire (GUILD-WIRE-1)

- **Capability `GUILD_V1`**; its number, two command types and one domain are reserved on #162 at
  allocation.
- **`GUILD_QUERY`:** the acting character's guild (ranks, roster §4.2, titles, message, balance,
  invitations, activity log) and another guild by name (the public view).
- **`GUILD_INTENT`** (a oneof; an empty oneof is `REJECTED`): `found`, `invite`, `revoke_invite`,
  `accept`, `leave`, `exclude`, `set_rank`, `edit_ranks`, `set_title`, `set_message`, `resign`,
  `disband`, `deposit`, `withdraw`. Results: `OK`, `NOT_ALLOWED`, `NAME_TAKEN`, `NAME_INVALID`,
  `ALREADY_IN_GUILD`, `ACCOUNT_HAS_POSITION`, `NOT_PREMIUM`, `GUILD_FULL`, `NOT_INVITED`, `JUNIOR`,
  `INSUFFICIENT_FUNDS`, `BALANCE_LIMIT`, `STALE_REVISION`, plus the common results. Edits carry
  the expected guild `revision`.
- **Domain:** the viewer-relative emblem of §4.3 per visible character, and the member's own guild
  name and rank.
- **Houses:** HOUSE-WIRE-1's `HOUSE_INTENT` gains `guildhall_bid {house, max}` and guildhall
  `move_out`; `acl_set` and `door_set` accept guild entries (§10).

## 10. Guild entries in ACL lists (GUILDHALL-1, every house)

- An ACL or door list entry is a character, or a guild entry `{guild, min_level}` meaning every
  member at that level or above (`*@guild` is `min_level` = the lowest level, `rank@guild` one
  level range), or an exclusion of one character (`!name`). Exclusions win.
- Entries apply to private houses, shops and guildhalls alike, of the same World only; at most
  `HOUSEOWN0-RL-12` (200) entries per list, guild entries included.
- The house interior runtime resolves a guild entry against the membership at check time; no
  membership change edits an ACL row. A disbanded guild's entries grant nothing and are removed by
  the house job.
- Name wildcards (`*`, `?`) wait for a later decision: they match names across Accounts and need a
  privacy reading.

## 11. Rows (registered by the children before implementation)

| Row | Value |
|---|---|
| `GUILD0-RL-01` guild name length | at most 29 letters, character name rules |
| `GUILD0-RL-02` formation deadline | 3 days |
| `GUILD0-RL-03` vice leaders required | 4 |
| `GUILD0-RL-04` vice or Premium deficit grace | 14 days |
| `GUILD0-RL-05` members per guild | 2,000 (`PARITY_PENDING`) |
| `GUILD0-RL-06` ranks per guild | 3 to 20 |
| `GUILD0-RL-07` invitation lifetime | 30 days (`PARITY_PENDING`) |
| `GUILD0-RL-08` open invitations per guild | 500 |
| `GUILD0-RL-09` guilds per job pass | 100 |
| `GUILD0-RL-10` activity log window | 30 days |
| `GUILD0-RL-11` Premium leaders and vices required | 5, after G1 and PREM-1 |
| Guild bank deposit or withdraw | 0 items, 2 value lines, 1 event |
| Guildhall bid, raise or lower | 0 items, up to 4 value lines (two sources and two escrow parts), 1 event |
| Guildhall rent charge | 0 items, up to 2 burn lines, 1 event |
| Disband member step | 100 members, 0 value lines, 1 event |

## 12. Rejected options

- **A guild as a special Account.** A guild is World-scoped Game truth, not a Platform identity.
- **Guildhalls in the personal slot.** EXP-HOUSES-01 §9.
- **Leader-owned guildhalls.** EXP-HOUSES-01 §9: the owner is `GuildId`.
- **Guild administration only on the web.** Game owns guild truth and the Platform web view does
  not exist; in-game commands deliver guilds now and a later web view reads the same state.
- **Evicted items to the leader.** EXP-HOUSES-01 §12.3 and §14.
- **Guild wars now.** They need PvP (PARTY-PVP-0) first.
- **Exclusion at logout.** It exists only because Tibia administers guilds on the web.

## 13. Owner-rule applications (Global parity, 5905825574)

Kept as in Tibia: one guild per character; ranks 1 to 20; 4 vices within 3 days; 14-day vice grace;
one leader-or-vice position per Account; invite and exclude rights; promotion only below oneself;
guild bank rights; one guildhall per guild, bid from the guild bank then the leader's bank; guild
ACL entries. Declared differences: immediate effect of changes and resignation; per-item reclaim
on guildhall disposition; bounded members and invitations; the disband payout.

## 14. Owner questions (open)

**G1. Guilds before Premium exists?** Tibia requires Premium to found a guild and to be leader or
vice, and disbands a guild with fewer than 5 Premium leaders and vices; the Game has no Premium yet
(PREM-1 is in review). a) Not required until Premium is delivered; then the rules apply, and a
lapse keeps the rank (recommended: the same as your house answer H2a, guilds can start now);
b) no guilds until Premium exists.

**G2. Admit the guildhall price and rent as gold sinks?** D178 needs an owner decision for every
new fee source; the house price and rent were admitted as H1. a) Yes, the same as houses, drawn
from the guild bank first and then the leader's bank, as in Tibia (recommended); b) no.

## 15. Decision test

- **Must decide now:** YES. The owner asked for guilds because guildhalls block house work.
- **Minimum sufficient:** one guild, rank, member, invitation and position table; one guild
  balance; guildhall rows in the existing property table; guild entries in the existing ACL lists.
- **Superseding evidence:** owner answers; GUILD-WAR-0 may add war states; a Platform web view may
  move administration surfaces, not truth.
- **Deliberately not decided:** guild wars, applications, autorank, board, events, leader election,
  name wildcards, Rested in guildhalls, the Platform web view.

## 16. Before-freeze checklist

1. **Contract amendments:** EXP-HOUSES-01 §9, HOUSE-OWN-0 §3 and §10, the House catalogue `kind`,
   BANK-0 §3, each written "pending on acceptance of GUILD-0".
2. **Serialization:** §4.1's lock order; World jobs lock rows before re-checking them.
3. **Restart:** every state and step is durable and keyed; ambiguous outcomes reconcile by key.
4. **Typed references:** GuildId, HouseId, AccountId, WorldId, CharacterId, occurrence,
   TransactionId.
5. **Wire:** §9, capability `GUILD_V1`.
6. **Split work:** one guild per transaction; at most 100 members or bids per step.
