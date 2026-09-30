# VIP-0 The VIP list

- Decision: `VIP0-ACCOUNT-WORLD-VIP-LIST-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (protocol,
  persistence, security and privacy) and protected integration. Owner question V1 (§14) is
  answered: c) consent first, plus a "hide my status" setting (§5.1); VIP-1 and VIP-2 are
  unblocked (architect ruling R3, §13). Owner question V2 (§14) is answered: only the inviting
  character sees the status (§3).
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the owner's direction (2026-09-30): build the VIP list now, full Tibia Global parity.
  CHAT-0 and PARTY-PVP-0 left the VIP list to its own decision.
- Builds on: the social presence and contact consent baseline (owner-accepted, 2026-08-06); the
  scope matrix Presence row; CHAT-0 (the sealed World relay, `NOT_ONLINE` equal answers, the
  private message by name); PARTY-PVP-0 (the relay change hint and node cache pattern, branch
  `claude/arch-party-pvp-0`); `0001` (session rows), `0005` (roots, `account_id`), `0022`
  (global name namespace); BANK-0 (Account + World rows); the composition decision rules 1 and 2;
  PREMIUM-DELIVERY-0 (`premium_current`, the switch-over) and HOUSE-OWN-0 (owner answer H2a);
  FND-02 §10 (identities on the wire); owner rule 5905825574 (Global parity)
- Amends, each pending on acceptance of VIP-0, in this PR: the composition decision (rule 1 covers
  VIP rows, one paragraph before its §6); the scope matrix (a VIP list paragraph after the BANK-0
  one).
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker; review | Builds | Depends on |
|---|---|---|---|
| VIP-1 | impl; persistence, protocol, privacy | store, receipts, operations, the consent lifecycle and `vip_status` setting, caps, wire (§3-§7, §9) | VIP-0; PARTY-1 |
| VIP-2 | hard; security, privacy | presence on the World relay, gated by consent and `vip_status` (§8) | VIP-1; CHAT-2 |

VIP-1 builds the tables, the command receipts, the write functions under the rule 2 fence, add by
name with its invitation, accept, reject and revoke, the `vip_status` column on PARTY-1's
`game_character_social_settings` row, groups, the Premium switch-over, the rate bucket and the
`VIP_V1` wire, not yet offered to clients. VIP-2 builds the presence hints, the node watcher
index, the reconcile, status deltas and login notices, each showing a status only as §8.1 allows.

Later, each with its own decision: the rest of the consent-based contact layer of the social
baseline (exact channel, account-wide contacts), exercise-training status, character
rename and deletion (they call the hooks of §10), "Report Name".

## 1. Question

How does a player keep a list of other characters, with notes, icons and groups, and see when they
are online, without learning more than Tibia shows and without breaking the social baseline?

## 2. Facts

**PROVEN**

- Social baseline (owner-accepted): exact `ChannelId`, instance, node and position are never
  public; "a normal non-contact may see at most a coarse state such as `online on this world`,
  subject to the observed player's privacy settings"; exact channel only for party members and
  mutually accepted contacts. It also says "the legacy unilateral VIP-list model is rejected for
  Oteryn social relationships" and that the VIP window becomes a consent-based surface. Adding one
  character never reveals other characters of its Account. Stale presence fails toward less
  disclosure. Rate limits are mandatory. Cross-World scope, storage and wire are left open.
- Scope matrix: Presence is owned per World, "eventual with lease truth", "subject to privacy".
- `0001`: `game_durability_reconnect_sessions` holds `account_id`, `character_id`, `world_id`
  and `session_state` (1 in grace, 2 active); one row per character in state 1 or 2
  (`game_durability_one_nonterminal_session_per_character`).
- `0005`, `0022`: a root's Account and World never change; roots are never deleted; names are one
  global namespace by `name_key`; there is no rename or deletion yet.
- CHAT-0: one sealed `NOTIFY` relay per World, at most once; a node whose listener is down is not
  ready; the game nodes of a World are one trust boundary; a private message to a name not online
  on this World, including a name of another World, answers `NOT_ONLINE`.
- PARTY-PVP-0: a commit sends a sealed `{id, revision}` hint on the relay; nodes re-read rows.
  Party invites by name were rejected; the party list may show a member's channel.
- Composition rules 1 and 2: rows outside the Character revision chain advance no
  `CharacterRevision`; a player command's transaction takes the acting session's fence.
- PREMIUM-DELIVERY-0: every Premium check reads `premium_current(account, now)` once PREM-1 is
  delivered; its activation record names the switch-over date. HOUSE-OWN-0 H2a: a Premium
  requirement is not applied until then.

**CIPSOFT_OFFICIAL** (the Tibia manual)

- `controls_communication.md` §4.2.6: per entry, Edit (one icon, a free-text description,
  "Notify on login", assignment to custom groups), Remove, Message to [Name] (not when offline),
  Add New VIP (also "Add to VIP list" on any character); groups: 3 predefined (Enemies, Friends,
  Trading Partners) plus up to 5 custom; sort, show or hide offline and groups on the client.
  **Capacity 20 (free), 100 (Premium).** A violet name marks exercise training.
- `interface.md` §3.6.4: the same capacity; the list shows the online status of the entries.
- §4.2.5: "Allow VIPs to message you" fills the whitelist from the VIP list.

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b5`)

- The list belongs to the **Account**, shared by its characters: `account_viplist (account_id,
  player_id, description varchar(128), icon, notify)`, unique per pair, deleted with the target
  character (`schema.sql:207-222`); groups per Account, the three fixed ones created with the
  Account (`:225-262`). The name shown is the target's current name (`iologindata.cpp:365`).
- Caps: 100 Premium, 20 free (`player_vip.cpp:24-32`); groups 8 with Premium (5 custom and 3
  fixed), none without (`:34-39`); an entry may be in several groups (`:106-127`).
- Add by name: names over 25 characters are ignored; an unknown name answers "A player with this
  name does not exist."; a hidden (ghost) target is added as offline (`game.cpp:7058-7096`).
- Icons 0-10 (`protocolgame.cpp:2858`). Statuses: offline, online, pending (dead, not yet back),
  training (`creatures_definitions.hpp:783-788`; `player.cpp:3279-3286`, `:4353-4355`).
- Login and logout are pushed to every online watcher, with the "has logged in." or "has logged
  out." message and a sound only when the entry notifies (`player_vip.cpp:41-61`,
  `player.cpp:4512-4524`).

## 3. What a VIP entry is (the reading of the baseline)

- A VIP entry is a watch-list entry of one Account on one World. It names one target character.
  Adding it sends the target character a **VIP invitation** (owner answer V1 c, §14); the entry
  shows a status only after the target accepts (§5.1), and never while the target hides its status.
- An accepted entry shows the target's name and a **coarse status: online on this World, or
  offline**. It never shows `ChannelId`, instance, node, position, level, vocation, guild or the
  target's Account. An entry without consent shows only `UNCONFIRMED` (§7).
- **Who sees it** (owner answer V2, §14): the target accepted an invitation that named the
  inviting character, so the status is shown only to sessions of that character, not to the other
  characters of the watcher's Account that share the list.
- Accepting grants only this coarse status. The rest of the baseline's consent-based contact layer
  (exact channel, account-wide contacts) is a later, separate layer; a VIP entry never becomes
  one silently, and a legacy import never creates an entry or a consent.

## 4. Storage (VIP-1)

- **Per (Account, World)** (architect ruling R1, §13). The list is shared by the Account's
  characters on that World, as Canary shares it per Account. Another World has its own list.
- `game_vip_lists`: one row per (`account_id`, `world_id`): `revision`, `next_entry_no`,
  `rate_tokens`, `rate_at`. Created on the first write.
- `game_vip_entries`: (`account_id`, `world_id`, `character_id`) primary key; `entry_no`
  (unique per list, never reused within it); `description`; `icon`; `notify`; `added_at`;
  `inviter_character_id` (the acting character of the add); `consent` (`NONE`, `PENDING`,
  `ACCEPTED`); `invite_expires_at`; indexed by (`character_id`, `consent`) for the target's view.
  The target root must be on the same World (checked in the write function against `0005`).
- `game_vip_groups`: custom groups only, (`account_id`, `world_id`, `group_no` 4-8), `name`,
  unique per list. Groups 1-3 are the fixed Enemies, Friends and Trading Partners: constants,
  never stored, never renamed or removed.
- `game_vip_group_members`: (`account_id`, `world_id`, `character_id`, `group_no`); deleted with
  its entry or its custom group.
- `game_vip_command_receipts`: one immutable row per VIP command, keyed by (`account_id`,
  `world_id`, `command_ref`); the acting `character_id`; the operation kind; a SHA-256 digest of
  the canonical request; the outcome (result code, `entry_no` or `group_no`, `EXHAUSTED` seconds,
  the list `revision` after the command); `committed_at`. The BANK-0 §3 pattern.
- No writer role writes these tables directly. One SECURITY DEFINER function per operation first
  asserts the current recovery fence (composition decision rule 2, replay step: a stale recovery
  fence returns no outcome), then reads the receipt of its CommandRef: a receipt with the same
  binding (character, kind, digest) returns its stored outcome **before** the live session,
  lease and runtime checks and any lock, cap or rate check, writes nothing and advances nothing;
  a receipt with a different binding is `REJECTED`. With no receipt, it checks
  the acting session's fence (composition rule 2: session row in state 1 or 2, generations, the
  runtime-scope guard), that the session's Account and World own the list, then locks the list
  row, re-reads the receipt under that lock, checks caps and the rate bucket, writes, advances
  `revision`, and inserts the receipt with the outcome **in the same transaction**. A target's
  consent operation (§5.1) is receipted in the acting Account's own list (created on first write)
  and also locks, after its own `game_character_roots` row, the watcher's list row it changes. **Lock
  order:** a transaction that needs two list rows (a consent operation: the acting Account's and
  the watcher's) takes the `game_character_roots` rows first, then both list rows in ascending
  (`account_id`, `world_id`) order, before any read under lock, so reciprocal accepts cannot
  deadlock; a single-list operation takes one row. Every outcome
  is receipted, refusals included (an `UNKNOWN_NAME` add has consumed a token), so a commit
  followed by a node failure replays the first result instead of executing again.
- Receipts are deleted with their list; their retention bound is reserved as `VIP0-RL-12`.
- **Composition rule 1:** VIP rows and the `vip_status` setting are Account + World and Character +
  World social rows, not Character state. A VIP write advances no `CharacterRevision` and touches
  no item. Amendment to the composition decision.

## 5. Operations (VIP-1)

- **Add `{name}`.** The name is checked against the `0022` grammar, then resolved by `name_key`
  to a root **on the session's World**. A name that does not exist, is not on this World, or
  (later) belongs to a deleted character all answer the same `UNKNOWN_NAME`, so the answer never
  reveals where a name exists (as CHAT-0 `NOT_ONLINE`). Adding the acting character itself is
  `REJECTED`. Adding the Account's own other characters is allowed, as in Tibia, and needs their
  consent like any other. A listed target is `ALREADY_LISTED`. A full list is `LIST_FULL` (§6). On
  success the entry is created with an invitation (§5.1) and sent as `UNCONFIRMED`.
- **Remove `{entry_no}`**, with its group memberships; it also withdraws the entry's invitation
  or consent. Adding the name again sends a new invitation.
- **Edit `{entry_no, description, icon, notify, groups[]}`**: replaces all four. The description is
  UTF-8, 0 to `VIP0-RL-04` characters, no control characters, trimmed. The icon is 0 to
  `VIP0-RL-05`. The groups are fixed ones and existing custom ones, each once; an unknown group is
  `NOT_FOUND`.
- **Group add `{name}`**, **rename `{group_no, name}`**, **remove `{group_no}`**: custom groups
  only; a fixed group is `REJECTED`. The name is 1 to `VIP0-RL-06` characters, unique in the list
  (case-insensitive), else `NAME_TAKEN`. Too many is `GROUP_LIMIT` (§6).
- Every operation is idempotent by its CommandRef through its receipt (§4): replay returns the
  first result, even after a commit whose answer was lost; a changed request is `REJECTED`.
- An unknown `entry_no` or `group_no` is `NOT_FOUND`. It names only the caller's own list.

### 5.1 Consent and "hide my status" (owner answer V1 c, §14)

- **Invite.** An add first locks the target's `game_character_roots` row FOR UPDATE (no revision
  moves; the PARTY-PVP-0 §4.1 pattern), then the list row. The entry gets `consent` `PENDING`,
  `inviter_character_id` the acting character and `invite_expires_at` = the database clock +
  `VIP0-RL-14`. When the target has blocked the inviting character (PARTY-PVP-0 §3
  `game_character_social_blocks`) or already has `VIP0-RL-13` unexpired pending invitations, the
  entry is written with `consent` `NONE` instead: the add answers `OK` and the entry shows
  `UNCONFIRMED` either way, so the watcher never learns that it was blocked or capped.
- **Accept `{invitation}`**, by the target character on its own session: a `PENDING`, unexpired
  entry naming it becomes `ACCEPTED`. A target with `VIP0-RL-15` accepted entries answers
  `LIST_FULL`. **Reject `{invitation}`**: `PENDING` becomes `NONE`. Anything else is `NOT_FOUND`.
- **Revoke `{watcher}`**, by the target: an `ACCEPTED` entry naming it becomes `NONE`, and the
  watcher sees `UNCONFIRMED` from its next status evaluation. The watcher revokes by Remove.
- **Expiry.** A `PENDING` entry past `invite_expires_at` reads as `NONE` whether or not a row has
  been rewritten; no job is needed.
- **Hide my status `{shown | hidden}`**, by the target on its own consent rows: the
  `vip_status` column of `game_character_social_settings` (PARTY-PVP-0 §3; a missing row reads
  `SHOWN`; the same row carries `vip_consent_revision`, §5.1 below). The write first locks its own `game_character_roots` row FOR UPDATE and, after its
  commit, sends a presence hint for the character (§8.2). While it is `HIDDEN`, every watcher,
  accepted ones included, sees `OFFLINE`, with no login or logout announcement.
- Invitations and accepted watchers are addressed by a per-session handle and shown by the
  inviting character's current name, never by CharacterId, AccountId or list.
- Each consent operation is one transaction under the target's rule 2 fence, receipted and
  rate-limited like any VIP operation (§4, §9); it advances the watcher list's `revision`, whose
  list hint (§7) refreshes the watcher's sessions.
- **Target-side consent view.** The target's view (pending invitations, accepted watchers) is
  derived from other Accounts' rows, so it has its own revision: `vip_consent_revision` on the
  target's `game_character_social_settings` row (created on first write). Every change to what
  that view shows advances it in the same transaction, under the target's `game_character_roots`
  lock already held: an add that writes `PENDING`, accept, reject, revoke, a watcher's Remove of a
  `PENDING` or `ACCEPTED` entry, and the deletion of a list. An add written as `NONE` (blocked or
  capped) changes nothing the target sees and advances nothing. After the commit the writer sends
  a relay hint `{kind: vip_consent, character_id, revision}` (advisory, bounded like any relay
  line); a node holding a session of that character re-reads the consent view and revision from
  the rows and sends delta 5, dropping a hint whose revision it has already sent. Expiry rewrites
  no row, so the client hides an invitation at its `expires_at` and the reconcile (§8.2) re-sends
  the view when the revision or an expiry changed. A lost hint is corrected at the reconcile.

## 6. Caps and Premium (VIP-1)

- Entries: `VIP0-RL-01` (20) free, `VIP0-RL-02` (100) Premium, per list (manual).
- Custom groups: `VIP0-RL-03` (5) with Premium, 0 without (manual; free 0 from Canary).
- **Until the switch-over** named by PREM-1's activation record, the Premium caps apply to every
  Account (architect ruling R2, §13, applying owner answer H2a). From the switch-over, the caps
  read `premium_current(account, now)` at each write.
- **Over the cap, nothing is deleted.** An Account over its current cap (Premium ended, or a free
  Account after the switch-over) keeps every entry and group, sees and edits them, and can remove
  them; it cannot add an entry or a custom group, or rename a custom group, until it is under the
  cap (`LIST_FULL`, `GROUP_LIMIT`). `PARITY_PENDING`: Global's exact behaviour when Premium ends.

## 7. Wire (VIP-1; offered to clients by VIP-2)

- **Capability `VIP_V1`**; its number, one command type and one state domain are reserved on
  #162 at allocation.
- **`VIP_INTENT`** (a oneof; empty is `REJECTED`): `add {name}`, `remove {entry_no}`,
  `edit {entry_no, description, icon, notify, groups}`, `group_add {name}`,
  `group_rename {group_no, name}`, `group_remove {group_no}`, `accept {invitation}`,
  `reject {invitation}`, `revoke {watcher}`, `vip_status {shown | hidden}`. Results: `OK`,
  `UNKNOWN_NAME`,
  `ALREADY_LISTED`, `LIST_FULL`, `GROUP_LIMIT`, `NAME_TAKEN`, `NOT_FOUND`, `EXHAUSTED {seconds}`
  (the rate bucket), `VIP_UNAVAILABLE`, plus the common results.
- **Identities:** entries by `entry_no` (a small number local to the list), never CharacterId or
  AccountId (FND-02 §10). The name is the target's current name.
- **Domain `VIP`.** Snapshot: the custom groups, and every entry `{entry_no, name, description,
  icon, notify, groups, status}`; the character's pending unexpired invitations (at most
  `VIP0-RL-13`, inviter name and expiry), its accepted watchers (inviter names) and its
  `vip_status`. Deltas: 1 entry upsert, 2 entry removed, 3 groups replaced, 4 status
  `{entry_no, status, announce}`, 5 consent view replaced. `status` is `OFFLINE`, `ONLINE` or
  `UNCONFIRMED` (no consent for this session, §8.1); `TRAINING` is
  reserved for the exercise-training decision; `PENDING` is not used (`PARITY_PENDING`: a dead
  character stays online until its session ends). `announce` is set only for a real login or
  logout of an entry with `notify`; the client shows "Name has logged in." or "has logged out."
  and plays the sound (Canary).
- The revision stays above any the session has seen across a reconnect (the NPC-0 pattern).
- Sort, show or hide offline, show or hide groups, "Message to" (the CHAT-0 private message by
  name) and "Allow VIPs to message you" (the whitelist) stay on the client, as in Tibia.
- A write answers after its commit. Other sessions of the same Account on the World get the change
  through the relay hint `{vip_list, account, world, revision}` (PARTY-PVP-0 pattern); a node whose
  listener is down answers writes `VIP_UNAVAILABLE`.

## 8. Presence (VIP-2)

### 8.1 What online means

- **Consent gate** (§3, §5.1), read with the session row at every status evaluation (snapshot,
  hint re-read, reconcile): a watcher session gets `UNCONFIRMED` unless the entry's `consent` is
  `ACCEPTED` and its character is the entry's `inviter_character_id`; then `OFFLINE` while the
  target's `vip_status` is `HIDDEN`. A failed or stale read sends no `ONLINE` (less disclosure).
- Otherwise a target is **online** while it has a session row of this World in state 1 or 2
  (`0001`), so a client drop in grace or a channel switch shows no change, and offline without
  one. A private message still follows CHAT-0 (state 2 only).
- The channel is **never** sent for a VIP entry, whatever the channel count (§3).

### 8.2 How changes reach nodes

- The writer of a session-row transition into or out of states 1-2 (admission, logout,
  teardown, the recovery that ends a crashed node's rows) sends, after its commit, one sealed hint
  on the CHAT-0 World relay: `{kind: presence, character_id, online, at}`. It is bounded like any
  relay line (`CHAT0-RL-06`) and counts toward `CHAT0-RL-09`. No new process, table or channel.
- Each node keeps a **watcher index** for its own sessions: target CharacterId to the local
  sessions whose list names it. It is built from the rows at admission, reconnect and channel
  transfer, and changed by VIP writes and list hints.
- **A hint is advisory.** Relay lines of two transitions can arrive out of order (each `NOTIFY` is
  sent after its own commit). On a presence hint for a watched target, the node ignores the hint's
  `online` value and re-reads the target's authoritative session row (state 1 or 2 on this World,
  one indexed query), as PARTY-PVP-0 nodes re-read rows. It sends a status delta only if that
  read differs from the status it last sent the watchers, with `announce` for `notify` entries
  only when the read confirms the change the hint names. A delayed login hint after a logout thus
  reads offline and emits nothing. Hints for unwatched targets are dropped.
- **Truth at the edges.** A snapshot reads statuses from the session rows (one indexed query over
  at most 100 CharacterIds). Every `VIP0-RL-09` (60 s), and at once after its listener reconnects,
  a node re-reads the statuses of all its watched targets in one query and sends the differences
  **without** `announce`. A lost hint is thus corrected within 60 s, and a stale login is never
  announced.
- **Fail closed on visibility.** Hints are at-most-once, so a lost `SHOWN` to `HIDDEN` hint must
  not keep an `ONLINE` on screen. A node sends or keeps `ONLINE` for a watcher only while it has
  confirmed the target's visibility (`vip_status`, session row and the entry's consent) in the last
  `VIP0-RL-16` (10 s): every `VIP0-RL-16` it re-reads these rows for its watched targets that it
  shows `ONLINE` (one indexed query, only those targets) and demotes to `OFFLINE` (hidden) or
  `UNCONFIRMED` (consent gone) at the first read that says so. The full reconcile of all watched
  targets stays at `VIP0-RL-09` (60 s).
- While a node's listener is down, it cannot confirm: it sends `UNCONFIRMED` for every entry it
  shows `ONLINE`, sends no other status change and answers writes `VIP_UNAVAILABLE`; the reconcile
  after reconnect restores truth (without `announce`).

### 8.3 Privacy

- A watcher learns only "online on this World" or "offline", and only for a target character that
  accepted its invitation and does not hide its status (§5.1, §8.1). The target sees who asked.
- The target sees only its own pending invitations and accepted watchers, by the inviting
  character's name; the relay carries no watcher identity, and a block, cap or rejection gives
  the watcher the same `UNCONFIRMED`.
- Nothing links Accounts: the target's Account is never read into a message; `UNKNOWN_NAME` is one
  answer; a list of one Account is never shown to another Account.
- The relay is read only by the World's game nodes (CHAT-0 trust boundary). Hints and VIP rows
  are not logged at ordinary levels, as FND-02 §20 treats private chat.
- The `vip_status` setting filters at the watcher's node before sending; a change to `HIDDEN`
  takes effect on the next status evaluation, at once on its presence hint and at the latest at
  the next `VIP0-RL-16` visibility confirmation (10 s), after which no stale `ONLINE` remains.

## 9. Rate limits (VIP-1)

- One bucket per list: `VIP0-RL-07` (20 writes, one more every 3 s), across all operations. A
  refused write is `EXHAUSTED {seconds}`. An `UNKNOWN_NAME` answer costs one token like any add,
  so name probing is bounded (baseline: no enumeration through search errors).
- The bucket lives in the list row, so it survives relog, channel switch and a second character.

## 10. Rename and deletion (hooks for their own decisions)

- Entries are keyed by CharacterId, so a later **rename** keeps every entry. The rename
  transaction sends a relay hint `{kind: renamed, character_id}`; nodes re-read the name and send
  an entry upsert to its watchers. Until the rename decision lands, a name changes only at the
  next snapshot. Showing the new name matches Global (Canary reads the current name).
- A later **terminal deletion** deletes the character's entries and group memberships in its own
  transaction (Canary: `ON DELETE CASCADE`) and sends an offline hint and a list hint for each
  touched list. The rows carry a foreign key to the root so that transaction is the only way out.
  A name freed after a rename or deletion names a new CharacterId; old entries never follow it.
- An Account's lists are removed with the Account under the Platform account lifecycle; not here.

## 11. Rows

| Row | Value |
|---|---|
| `VIP0-RL-01` entries per list, free | 20 (manual) |
| `VIP0-RL-02` entries per list, Premium | 100 (manual) |
| `VIP0-RL-03` custom groups, Premium / free | 5 / 0 (manual; free from Canary) |
| `VIP0-RL-04` description | 128 characters, 512 bytes (`PARITY_PENDING`, Canary column) |
| `VIP0-RL-05` icon | 0 to 10 (`PARITY_PENDING`, Canary) |
| `VIP0-RL-06` custom group name | 1 to 25 characters (`PARITY_PENDING`) |
| `VIP0-RL-07` VIP writes per list | a bucket of 20, one per 3 s (Oteryn choice) |
| `VIP0-RL-08` `VIP` snapshot | at most 100 entries and 5 custom groups; bytes measured by VIP-1 |
| `VIP0-RL-09` presence reconcile | 60 s, and at listener reconnect |
| `VIP0-RL-10` presence hints per World per second | measured by VIP-2 inside `CHAT0-RL-09` |
| `VIP0-RL-11` status delta after a login, p99 | measured by VIP-2 |
| `VIP0-RL-12` VIP command receipt retention | at least the CommandRef replay window; set by VIP-1 |
| `VIP0-RL-13` pending VIP invitations per target | 20 (Oteryn choice, as `PARTYPVP0-RL-29`) |
| `VIP0-RL-14` VIP invitation lifetime | 7 days (Oteryn choice) |
| `VIP0-RL-15` accepted VIP watchers per target | 100 (Oteryn choice, the Premium list cap) |
| `VIP0-RL-16` visibility confirmation for a shown `ONLINE` | 10 s (architect ruling R4) |
| VIP operation | 1 transaction, 1 receipt, 0 items, 0 value lines, 0 `CharacterRevision`, 1 relay hint |

## 12. Rejected options

- **A list per Character.** Global shares the list across the Account's characters (Canary).
- **A list per Account across Worlds.** Presence is World-scoped and the relay is per World; an
  entry from another World could show nothing but a false offline, and would count against a cap
  the player cannot see.
- **Showing the channel.** A VIP entry is not a mutually accepted contact (baseline).
- **Status without consent (V1 a, b).** The owner chose consent first (V1 c).
- **Telling the watcher of a block, cap or rejection.** It would reveal the target's settings.
- **"This player does not exist" for other Worlds.** It would reveal where a name lives.
- **CharacterId or a guid on the wire.** FND-02 §10; a list-local `entry_no` is enough.
- **A presence service or broker.** The CHAT-0 relay and the session rows are enough.
- **Hints only, no reconcile.** The relay is at most once; a lost logout would show online forever.
- **Announcing reconcile corrections.** A late "has logged in." would be false.
- **Acting on a hint's `online` value.** Relay lines can be reordered; a stale login hint would
  announce a character that has logged out.
- **Idempotency without a stored receipt.** A lost answer after a commit would execute again:
  `ALREADY_LISTED`, `NOT_FOUND`, a second token and a second revision.
- **Deleting entries over the cap.** Premium ending must not destroy player data.
- **Advancing `CharacterRevision`.** A note on a friend is not Character state (rule 1).

## 13. Architect rulings (owner rule 5905825574)

**R1. Storage per (Account, World).** Global shares one list per Account (Canary). Oteryn Worlds
have separate presence and relays, and CHAT-0 treats another World's names as unknown. a) Account
+ World (recommended: parity inside a World, no cross-World leak, a visible cap); b) Account
across Worlds: entries that can never show a status; c) per Character: not Global. **Ruled a)**,
the BANK-0 shape.

**R2. The Premium caps before Premium is delivered.** a) Everyone has the Premium caps until the
switch-over, then `premium_current` decides and nothing is deleted (recommended: the owner's H2a
answer for houses, and no data loss at the switch); b) the free caps for everyone until then (the
CHAT-0 gate shape): players lose Global's Premium room now for nothing. **Ruled a)**. It is not a
Game-side "everyone is Premium" switch (D69): only these caps read it, and only until PREM-1.

**R3. VIP-1 waits for V1.** a) VIP-1 now, VIP-2 after V1: under V1 c) the store gains an invitation
table and the wire an invitation flow, and unilateral rows written before the answer would exist
under an unaccepted model; b) VIP-1 after V1 (recommended: fail closed, no redesign, no row under
an unaccepted model). **Ruled b)**. V1 is answered (§14), so VIP-1 may be allocated; the
invitation state lives on the entry row (§4), with no new table.

**R4. Hiding must fail closed.** a) Accept up to 60 s of stale `ONLINE` after a lost hint: rejected,
it discloses after the target used a privacy control; b) a 10 s visibility confirmation for shown
`ONLINE` entries plus `UNCONFIRMED` while the listener is down (recommended: bounded, cheap, no
cross-node acknowledgement); c) a synchronous cross-node acknowledgement of every hide: new
infrastructure. **Ruled b)**. Owner question raised in the report: is a residual window of up to
10 s acceptable, or must hiding wait for a cross-node acknowledgement?

## 14. Owner questions

**V1. May the VIP list show online status without the target's consent?** The social baseline
(2026-08-06) rejects "the legacy unilateral VIP-list model" and makes the VIP window a
consent-based surface; the direction of 2026-09-30 asks for full Global parity, where anyone can
add a name and see it online. Only the owner can reconcile two owner statements.

- a) **Global VIP list as a private watch list** (§3): add any name of the World, see only
  online/offline, never the channel; the consent-based contact layer comes later and alone grants
  the channel. The baseline's presence rules are all kept.
- b) As a), plus a per-character "appear offline in VIP lists" setting, default off (a deviation
  from Global; the baseline's "subject to the observed player's privacy settings").
- c) Consent first: an entry shows status only after the target accepts an invitation (the
  baseline as written; not Global).

**Recommended: a).** It is exactly Global and the presence it shows is the baseline's own ceiling
for non-contacts; b) can be added later without changing the store. VIP-1 is the same under a) and
b); under c) VIP-1 gains an invitation table. VIP-1 and VIP-2 both start only after the answer (R3).

Owner answer (2026-09-30, #162): c — consent first, plus a "hide my status" setting: status is
visible only after the watched character accepts the VIP invitation, and the watched character
can hide its status even from accepted watchers (§3, §5.1, §8.1). A declared difference from
Global.

**V2 (answered). Does one acceptance cover the whole watcher list?** The list is shared by the
watcher Account's characters on the World (R1), but the target accepted an invitation that named
one character. a) Only the inviting character sees the status (the fail-closed rule applied until
answered, §3); b) every character of the watcher's Account on the World sees it, which the target
cannot know. The store keeps `inviter_character_id`, so b) needs no store change.

Owner answer (2026-09-30, #162): only the inviting character sees the status. This is option a)
above; on #162 it was asked with the letters reversed and answered as "1b". The rule of §3 is
binding and is no longer a fail-closed placeholder.

## 15. Decision test

- **Must decide now:** YES. The owner asked for the VIP list now; CHAT-0 and PARTY-PVP-0 left it
  here.
- **Minimum sufficient:** five tables (four list tables and the command receipts), one write
  function per operation, one command, one domain, one presence hint kind on the existing relay, one
  reconcile query per node per minute.
- **Superseding evidence:** official Global values for description length, icons, group names,
  over-cap behaviour after Premium ends, and pending or training status.
- **Deliberately not decided:** exact channel for contacts, account-wide contacts, privacy
  settings beyond `vip_status`, exercise training, rename and deletion themselves, "Report Name",
  legacy import.

## 16. Before-freeze checklist

1. **Contract amendments:** the composition decision (rule 1 covers VIP rows); the scope matrix
   (VIP list paragraph). Each is written "pending on acceptance of VIP-0". The capability, command
   and domain numbers are reserved at allocation.
2. **Serialization:** the list row lock orders all writes of one list; rule 2 fence; replay by
   the CommandRef receipt written in the mutation's transaction; presence hints re-read the session
   row; presence is eventual, corrected by the reconcile.
3. **Restart:** entries, groups, receipts and the rate bucket are durable; watcher indexes and
   statuses are rebuilt from rows.
4. **Typed references:** AccountId, WorldId, CharacterId (server only); `entry_no` and `group_no`
   (clients); names.
5. **Wire:** §7, capability `VIP_V1`; offered only when VIP-2 lands.
6. **Privacy review:** §3, §5, §5.1, §8.1 and §8.3 against the social baseline and owner answer
   V1 c (recorded on #162, 2026-09-30); no status without an accepted invitation; `HIDDEN` shows
   `OFFLINE` to every watcher.
