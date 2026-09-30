# PARTY-PVP-0 Parties and PvP

- Decision: `PARTYPVP0-PARTIES-AND-PVP-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence,
  combat, security, privacy and protocol) and protected integration. Owner question P1 (§16) is
  answered: the PvP type is per-World configuration; Optional and Open PvP are both delivered; the
  first World launches as Optional PvP (confirmed by the owner, §16). Owner question P2 (§16) is
  answered: Hardcore PvP stays a v1 engine value that no World uses.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the owner's direction (2026-09-30): build parties and PvP now, full Tibia Global
  parity. It is the PvP decision that ATTACK-0 §3, the first player death decision §4.1 (D60) and
  CONDITIONS-0 §7 wait for, and the party system that D3 (D112) and D118 wait for.
- Builds on: the PartyId, solo viable party rewarded, PvP secondary-pillar and social presence
  baselines (owner-accepted); the scope matrix party and PvP rows; GAME-CHANNEL-01 §8 and §25;
  ATTACK-0; CONDITIONS-0; GAME-ABILITY-01; the first player death decision and DEATH-0; D3; D109
  and D118; CHAT-0; GUILD-0 (branch `claude/arch-guild-0`); the composition decision rules 1 and
  2; QUEST-STATE-0 §5.2 (CHAR-REV-SEQ-1); the spell `party_buff` behaviour (S27); owner rule
  5905825574 (Global parity)
- Amends, each pending on acceptance of PARTY-PVP-0, in this PR: ATTACK-0 §3 and §4 (PvP targets,
  the kill block); the first player death decision §4.1 and §4.5 (PvP deaths, black skull
  respawn); DEATH-0 §3.1 (PvP fields of the death receipt); D3 §4.4 (party loot right); the
  composition decision (rule 1 covers party and PvP rows); the scope matrix (party and PvP rows).
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| PARTY-1 | hard, persistence, security and privacy review | party, member, invitation and consent tables; invite (with the consent check), accept, decline, revoke, leave, succession, pass leadership, shared-XP toggle, block, party-invite and channel-visibility settings, invitation expiry; the node `PartyView` cache, full refresh and revision check, and the relay change hint; the cleanup job (§3, §4) | this decision; CHAT-2 |
| PARTY-XP-1 | hard (combat), combat review | shared-experience eligibility and split on the D118 XP slice; party immunity in area effects; the party loot right; the `PartyView` query for `party_buff` and the monk party rules (§5) | PARTY-1; the D118 XP lane; D3-4 |
| PARTY-CHAT-1 | impl, security review | one party room per party on the CHAT-0 World relay (§4.3) | PARTY-1; CHAT-2 |
| PVP-1 | hard, persistence and security review | PvP state, unjustified point and revenge mark tables; skull evaluation in the death transaction; the World cleanup job; the `rulesets/pvp/` and `rulesets/party/` rows, with `pvp_type` `OPTIONAL` for the first World (§6, §9) | DEATH-1; PARTY-1 |
| PVP-RT-1 | hard (combat), combat and security review | PvP legality in the GAME-ABILITY-01 legality stage; aggression relations; white and yellow skulls; logout, PZ and kill blocks and their durable write-ahead and restore; the damage factor; the PvP damage ledger and its durable snapshot; kill classification; Join Aggression; friendly fire (§7, §8) | ATTACK-1; COND-1; PVP-1 |
| PVP-DEATH-1 | hard (persistence), persistence review | the PvP variants of the death outcome: PvP death, red and black skull loss, Twist of Fate, Adventurer's Blessing, black skull respawn (§10) | DEATH-1; DEATH-3; PVP-1 |
| PVP-BLOCK-1 | impl, movement review | walking through characters and expert-mode blocking (§7.5) | PVP-RT-1; SPEED-1 |
| PVP-WIRE-1 | impl, protocol review | capabilities `PARTY_V1` and `PVP_V1`, the party and PvP commands and domains, VIS-2 skull, shield and frame fields (§11) | PARTY-1; PVP-RT-1; VIS-2; ATTACK-WIRE-1 |

Every PvP child builds and tests both `OPTIONAL` and `OPEN` (owner answer P1, §16). The `HARDCORE`
branches it touches are built and unit-tested with them, but no World is configured `HARDCORE`
(owner answer P2, §16). PvP goes live
on a World only when PVP-1, PVP-RT-1, PVP-DEATH-1 and PVP-WIRE-1 have landed; until then
ATTACK-0's "creatures only" stays. The first World goes live as `OPTIONAL` under that gate: rule 1
(§7.2) then refuses every character target until GUILD-WAR-0 fills `war_between`. A World
configured `OPEN` goes live under the same gate with no further decision. Later, each with its own decision: GUILD-WAR-0 (fills the
§12 hooks), arenas and PvP zones (`rulesets/pvp/arena/`), the Party Finder, the Party Hunt
Analyser, Retro Open and Retro Hardcore PvP (§6.1), Death Redemption (D64).

## 1. Question

How do characters form parties and share their benefits, and when may a character attack another
character, with which marks, blocks and death consequences?

## 2. Facts

**PROVEN**

- PartyId baseline (owner-accepted): a Game-issued UUIDv7 scoped by `WorldId`, one World-level
  authority; members may be on different channels; leader change, joins and channel moves keep it;
  knowing a PartyId grants nothing.
- Solo viable, party rewarded and PvP secondary pillar (owner-accepted): party play gives a real
  advantage without duplicating rewards; PvP is correct where a World enables it and follows the
  parity target in Reference.
- Social presence baseline (owner-accepted): a party invitation is a consent operation; party
  members may see a member's channel; nothing reveals hidden alternates; invite rate limits.
- Scope matrix: party membership World, strong and ordered; shared experience channel-local, only
  colocated members; PvP combat channel-local; combat lock Character + World, lease-visible, blocks
  channel change; skull and frag state Character + World, strong durable; ruleset World, immutable
  revision. GAME-CHANNEL-01 §8 and §25: a switch cannot clear or reduce PvP consequences.
- ATTACK-0: player targets refused until this decision; `secure` carried with no effect; a 60 s
  in-fight deadline that refuses logout and survives a same-GameSession reconnect; "the 15-minute
  block after a player kill belongs to PARTY-PVP-0".
- First player death decision: D60 deferred Twist of Fate, the unfair-fight reduction, skulls,
  Adventurer's Blessing and Retro rules; PvP-contributed deaths keep the D54 floor until this
  decision; D62 ladder, D63 full HP and mana, D65. DEATH-0: one death receipt per revision,
  `game_character_blessings`, the death writer on the Character sequencer (CHAR-REV-SEQ-1).
- D3: a corpse is exclusive for 10 s to its top-damage character, checked on the corpse receipt;
  D112 defers the party part; D121: "or its party"; player corpses are exempt. D118 and D109:
  shared experience as in Global, activity window and remainder UNKNOWN, up to 50 principals.
- CHAT-0: one sealed relay per World, at most once. Composition rules 1 and 2: non-revision rows
  and the acting Character's session fence. The map bundle has protection-zone and no-logout tile
  flags (MAP-WIRE-1). `rulesets/pvp/` and `rulesets/party/` exist, `READY_UNPOPULATED`. The spell
  `party_buff` behaviour (S27) needs party members in an area.

**CIPSOFT_OFFICIAL** (the Tibia manual: `combat.md` §5.3.7, §5.3.11-§5.3.13; `characters.md`
§5.1.11; `controls_communication.md` §4.2; `world.md`)

- Parties: the inviter leads; one party per character; only the leader invites; pass leadership;
  no leaving under a logout block; logout removes a member; a leaving leader passes to the
  earliest-invited member; party chat auto-joined; area spells spare members.
- Shared experience: as D118 (§5.1), the leader without a "battle sign".
- World types Hardcore, Open (skulls) and Optional (guild wars only); PvP from level 8, never in a
  protection zone; 10 s post-login immunity; 50% PvP damage unless the target is black-skulled;
  party and guild members spare each other except friendly fire; secure mode spares unmarked
  characters.
- Kills of marked victims are justified; others unjustified; assists count less; at most 5 full
  contributors over 5 minutes. Skulls (Open): white while the block runs; red at 3/5/10 kills per
  24 h/7 d/30 d ("rule of thumb"), 30 days; black at 6/10/20, 45 days, no attacks on unmarked
  characters, full PvP damage taken, respawn at 40 HP and 0 mana; private yellow and orange (7
  days or avenged).
- Blocks: logout 60 s; PZ block 60 s, not for party members or retaliation; a kill 15 minutes.
- Death: PvP death at 5% PvP damage in 5 minutes or a PvP final blow; red and black lose all
  equipment and regular blessings; Twist of Fate; 5+ blessings protect in PvP; Adventurer's
  Blessing to level 20 on Open; a reduced loss when attackers outlevel the victim (no formula);
  PvP kills give no XP except on Retro Hardcore.

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b51`, as cited in ATTACK-0 §2): an attacker in a protection
zone cannot attack (`combat.cpp:327-329`); in-fight lasts 60 s, refreshed by hits
(`player.cpp:4487-4503, 6518-6525`).

## 3. Party state (PARTY-1)

- **Tables**, World-scoped, strong durable, ordered by one row lock per party:
  - `game_parties`: `party_id` (UUIDv7, Game-issued), `world_id`, `leader_character_id`,
    `shared_xp_enabled`, `next_seq`, `revision`, `created_at`.
  - `game_party_members`: `character_id` primary key (one party per character), `party_id`,
    `seq` (the invitation order), `joined_at`. A deferred guard keeps the leader a member and the
    member's World equal to the party's.
  - `game_party_invitations`: (party, invitee), `seq`, `created_at`, `expires_at` = `created_at`
    + `PARTYPVP0-RL-30` (5 minutes). At most `PARTYPVP0-RL-02` (50) per party and
    `PARTYPVP0-RL-29` (20) open per invitee. An expired invitation is invalid from `expires_at`
    (accept answers `NOT_INVITED`) whether or not the cleanup job has deleted it yet.
- **Consent rows** (the social baseline's block list and privacy control; Character + World,
  strong durable, until a later contacts contract takes them over unchanged):
  `game_character_social_blocks` (`character_id`, `blocked_character_id`) primary key, at most
  `PARTYPVP0-RL-28` (100) per character; `game_character_social_settings`: `character_id` primary
  key, `party_invites` (`ANYONE`, the Global default, or `NOBODY`) and `channel_visibility`
  (`PARTY`, the social baseline's default, or `HIDDEN`, the observed player hiding its exact channel
  from party members, §4.4). A missing settings row reads as `ANYONE` and `PARTY`. They advance no `CharacterRevision` (composition rule 1 amendment).
- **Party size** `PARTYPVP0-RL-01` (50, D109).
- **No `CharacterRevision` advance**: party rows are World social state (the scope matrix), not
  Character state (composition rule 1 amendment).
- **Lifetime.** A party exists from the first invitation to its end. It ends when it has no
  member but the leader and no invitation, or when its last member leaves or is removed (leave,
  logout or the cleanup job, §4.1); its invitations and rows are deleted in that transaction. A
  PartyId is never reused, and no party history is kept (Tibia keeps none).
- **Invariant, checked by the deferred guard at commit:** a party row exists only with its leader
  as a member; no transaction commits a party without a leader member or an invitation without its
  party.

## 4. Party operations, cache and chat (PARTY-1, PARTY-CHAT-1)

### 4.1 Operations

Each is one transaction through a SECURITY DEFINER function under the composition rule 2 session
fence of the acting Character, replayed by its occurrence and a SHA-256 request binding (as
HOUSE-OWN-0 §4). Lock order: the occurrence; the acting Character's fence; a `character_root` row
where named below (the invite target's, or the acting Character's own for a consent change); the
party row; member rows by CharacterId; invitation rows; consent rows.

- **Invite `{target actor}`.** The target is a character visible to the actor (VIS-2, so on the
  same channel), not in a party. An actor without a party creates one and becomes leader. Only the
  leader invites. At most `PARTYPVP0-RL-03` (10) invitations per character per minute.
  Re-inviting an invitee this party has already invited renews its `expires_at` and writes no new
  row.
- **Invite consent check** (architect ruling; the social baseline's block list and privacy
  control). Right after the visibility check and before any other target check or write, the
  transaction locks the target's `game_character_roots` row FOR UPDATE (a stable row every
  Character has, so the lock holds even when the target has no settings or block row; no revision
  moves) and then reads the target's consent rows (§3): when the target has blocked the actor or
  its `party_invites` is `NOBODY`, the invite is refused with `NOT_VISIBLE`, the same result, the
  same empty payload and the same path (no party created, no row written, the rate-limit counter
  charged as for any refusal) as a target that is not available, so a refusal never tells the actor
  that it is blocked or which setting applies. The same lock serializes the invitee cap: a target
  with `PARTYPVP0-RL-29` (20) unexpired open invitations is refused on the same `NOT_VISIBLE` path.
  FOR UPDATE rather than FOR SHARE (architect ruling) because two concurrent invites must not both
  pass the cap.
- **Block `{target actor}`, unblock `{handle}`, party invites `{anyone | nobody}`, channel
  visibility `{party | hidden}`** by the character on its own consent rows. Each first locks the
  character's own `game_character_roots` row FOR UPDATE (no revision moves), so it serializes with
  every invite to it whether or not a consent row exists yet; an invite that committed first is
  seen, and one that commits later sees the change. A block also deletes, in the same transaction,
  an open invitation to the blocker from a party the blocked character leads. A channel-visibility
  change of a party member also advances its party's `revision` (party row locked after the
  `character_root`), so every `PartyView` refreshes (§4.2, §4.4).
- **Revoke `{invitee}`** by the leader; **decline** by the invitee.
- **Accept `{party}`.** The invitee holds a valid invitation and is in no party; it joins with
  the invitation's `seq`. Accepting removes its other invitations. The members need not be on the
  same channel (PartyId baseline).
- **Leave.** Refused (`LOGOUT_BLOCKED`) while the member's combat lock runs (§8.1: the logout
  block, the PZ block or the kill block). A leaving leader passes by the succession rule.
- **Succession rule** (architect ruling), for a leader that leaves, logs out or is removed by the
  cleanup job: leadership passes to the longest-standing online remaining member, the one with a
  session row in state 1 or 2 and the lowest `seq` (the manual's earliest invited); without an
  online member, to the remaining member with the lowest `seq`. With no remaining member the party
  ends: the same transaction deletes its invitations and its party row (§3 lifetime), so the leader
  guard is never violated.
- **Pass leadership `{member}`** by the leader, to any member.
- **Shared experience `{on | off}`** by the leader.
- **Logout** removes the member in the logout transaction, a leader by the succession rule. A
  channel switch keeps membership: it is not a logout (PartyId baseline).
- **Cleanup job** (World job, recovery fence and admission relations only): removes a member
  whose Character has no session row in state 1 or 2 for `PARTYPVP0-RL-08` (60 s), a leader by the
  succession rule, which covers a node crash and outlasts a channel switch. It also deletes expired
  invitations, oldest `expires_at` first, at most 100 per pass, one party per transaction (party
  row locked, then re-checked); a party left with only its leader and no invitation ends in that
  transaction (§3 lifetime).

### 4.2 Node cache

- Each node keeps a `PartyView` per party with a local member: members, leader, shared-XP flag,
  `revision`. After a commit, the writer sends a sealed hint `{party_id, revision}` on the CHAT-0
  World relay. The hint is advisory and at most once: it only makes a node re-read sooner; no rule
  depends on receiving it.
- **Full refresh** (architect ruling): at node start, at every relay (re)connect, and when a hint
  or a party-chat line (§4.3) shows a revision the node did not apply in order, the node re-reads
  from the durable rows every local character's `game_party_members` row and each such party's
  rows at their current `revision`, and replaces its views.
- **Bounded staleness:** a node also re-reads the `revision` of all its cached parties in one query
  every `PARTYPVP0-RL-26` (5 s) and refreshes any view that differs, so a hint lost to a writer
  crash or a dropped listener is caught. A view not confirmed within twice that interval is stale.
- **Party readiness:** a node is ready for parties only after its full refresh has completed and
  while its views are confirmed. While a node or a view is not ready, it fails toward less
  disclosure and no shared benefit: party commands answer `PARTY_UNAVAILABLE`; shared experience
  is off (XP by damage share); `members_in_area` (§5.2) returns no member; the `PARTY` domain and
  the party shield send no member channel or health; and PvP legality refuses a character target
  whose party relation to the actor comes from a stale view (`PVP_REFUSED {PARTY_STATE}`; an area
  effect skips it), so neither a stale immunity nor a stale non-membership decides harm.
- **Irreversible decisions** (architect ruling): even a confirmed view can miss a membership change
  committed since its last check, so a decision that cannot be undone never rests on the view
  alone. PvP legality between two characters (§7.2 rule 5, with the PZ-block exemption of §8.1 and
  friendly fire of §7.4, which use the relation it confirms) first reads, in one indexed query, the
  `game_party_members` rows of the actor and the target and the `revision` of each party found; an
  area effect batches this read over all its character targets. When a party found, or its
  `revision`, differs from the view, the view is stale: the harm is refused (`PVP_REFUSED
  {PARTY_STATE}`; an area effect skips that target) and the view is refreshed. A failed read
  refuses the same way. The relation is taken at that read; a change committed after it applies
  from the next action.
- Durable checks (loot, §5.3) read rows, never the cache.

### 4.3 Party chat (PARTY-CHAT-1)

- One room per party on the CHAT-0 World relay, sealed and bounded the same way; auto-joined on
  joining; members only. Each line carries the party `revision` at send; a receiving node
  refreshes a view older than it before delivery and delivers only to current members. Access ends
  at leaving.

### 4.4 Presence

- The party list shows each member's name, level, vocation and channel (the social baseline
  allows the channel to party members); health and mana percentages only for members on the
  viewer's channel. Never a position. Membership never names another character of the Account.
- **Channel visibility** (the social baseline lets the observed player hide its exact channel):
  for a member whose `channel_visibility` is `HIDDEN`, other members get only "online on this
  World": no channel, and no health or mana (which would reveal a shared channel). The setting is
  carried in the `PartyView` at its `revision`; a view not confirmed (§4.2) sends no member channel
  or health. Characters on the same channel still see each other on the map (VIS-2).

## 5. Party benefits (PARTY-XP-1)

All benefits are channel-local: only members on the same channel count (scope matrix). Members on
another channel are excluded, never failing participants: they neither receive a benefit nor turn
it off for the colocated members.

### 5.1 Shared experience

- **Participants** of a kill are the party's members on the kill's channel; members on other
  channels are excluded (above) and get no share.
- **Eligible** at a kill when the leader enabled it and, on the kill's channel: the leader is
  present; every participant is within `PARTYPVP0-RL-04` (30 tiles, Chebyshev, any floor) of the
  leader; over the participants, the lowest level × 3 ≥ the highest level × 2 (`PARTYPVP0-RL-05`);
  every participant was active within `PARTYPVP0-RL-06` (2 minutes: healed a member or attacked
  an aggressive monster; the D118 UNKNOWN, `PARITY_PENDING`); the leader is not under a PZ block. The manual's "battle sign" is read as the PZ block, because the logout block
  runs whenever a party hunts (`PARITY_PENDING`).
- **Split** (D118), among the participants only: the bonus of `PARTYPVP0-RL-07` (vocations counted
  over the participants) over the base XP, a summon's share first, then equal shares rounded up; a stamina-reduced share is removed without changing others. Each member's
  award is its own XP receipt on its own Character sequencer (CHAR-REV-SEQ-1); reward principals
  per death at most 50 (D109). Soul points as D118.
- **Off:** any failed condition of a participant turns it off for the kill, and XP goes by damage
  share. The
  level-up stickiness of the manual follows from re-evaluating at every kill.

### 5.2 Party immunity and buffs

- Party members and their summons are immune to each other's offensive effects and area damage
  (legality stage, §7.2), except friendly fire (§7.4).
- **Hook** `PartyView::members_in_area(caster, area, same_floor)` for `party_buff`, the monk
  Serene and virtue rules and any later party rule. It returns members on the caster's channel only.

### 5.3 Party loot right (D3 amendment)

- The corpse receipt also captures `top_damage_party_id`, the top-damage character's PartyId at
  death, read from that character's `game_party_members` row FOR SHARE in the corpse receipt's
  transaction, never from the runtime view (architect ruling: the value is irreversible once
  written). When that read cannot be made in the transaction, no party is recorded, so only the
  top-damage character holds the window (the D3 default). During the 10 s window, a pickup is also admitted for a character
  that is a current member of that party, read from `game_party_members` FOR SHARE. Player corpses
  stay exempt (D121).

## 6. World PvP type and PvP state (PVP-1)

### 6.1 World PvP type

- `pvp_type` is a World ruleset field (`rulesets/pvp/`), the same on every channel, bound into the
  ruleset revision. It changes only with a new ruleset revision at a planned reset; a change never
  clears durable PvP state.
- Values in v1: `OPTIONAL`, `OPEN`, `HARDCORE` (kept as an unused value, owner answer P2, §16). `RETRO_OPEN` and `RETRO_HARDCORE` are refused by
  ruleset validation until their rules have a source (retro frags, PvP XP formula, 6.31%
  blessings).
- **First World** (owner answer P1, §16): `OPTIONAL`. `OPEN`, with the full skull system, is
  delivered with it and available to any World configured with it.

### 6.2 Tables

Character + World, strong durable, one state on every channel (the scope matrix row):

- `game_character_pvp_state`: `character_id` primary key, `world_id`, `skull` (`NONE`, `RED`,
  `BLACK`), `skull_until`, `kill_block_until`, `logout_block_until`, `pz_block_until`,
  `white_skull_until`, `adventurer_forfeited`, `ledger_total_buckets`, `revision`.
- `game_character_pvp_ledger` (§8.3): (`victim_character_id`, `contributor_character_id`) primary
  key, `buckets` (at most 31 entries of bucket start, damage, assist; 10 s buckets), `last_at`; at most `PARTYPVP0-RL-25` (16) per victim.
- `game_character_unjustified_points`: (`character_id`, `death_occurrence_id`) primary key,
  `victim_character_id`, `points_milli` (1 to 1,000), `committed_at`.
- `game_character_revenge_marks` (orange): (`victim_character_id`, `killer_character_id`,
  `death_occurrence_id`), `expires_at` = +7 days (`PARTYPVP0-RL-17`), `avenged_at`.
- **No `CharacterRevision` advance** (composition rule 1 amendment): like the GAME-CHANNEL-01 §9
  anti-hopping guard, these are PvP-domain consequence rows keyed by a Character, not Character
  progression. They are written by death transactions (§9) and two runtime writes (§8.1, §10.4).
- **Effective skull** is read lazily: `skull` if `skull_until` is later than the database clock,
  else none. A World job deletes point rows older than `PARTYPVP0-RL-23` (45 days), expired
  marks and ledger rows whose `last_at` is older than `PARTYPVP0-RL-18` (5 minutes), at most 100
  per pass.

## 7. PvP legality (PVP-RT-1)

### 7.1 Where

Every PvP rule runs in the GAME-ABILITY-01 legality stage, the same for melee, spells, runes,
fields and condition ticks: no second combat path. A summon's actions are its owner's.

### 7.2 Rules, in order

1. **World type:** `HARDCORE` allows; `OPEN` follows the rules below; `OPTIONAL` allows only when
   `war_between(a, b)` (§12) is some; until GUILD-WAR-0 no PvP happens on an Optional World.
2. **Level:** both at level `PARTYPVP0-RL-09` (8) or more.
3. **Protection zone:** neither actor on a protection-zone tile.
4. **Post-login immunity:** an actor admitted less than `PARTYPVP0-RL-10` (10 s) ago cannot start
   aggression; it may answer an aggressor. Re-entry protection stays PvE only.
5. **Party and guild:** members of the same party or guild (GUILD-1) cannot harm each other,
   except friendly fire (§7.4). The relation is confirmed against the durable member rows and
   party `revision` before deciding (§4.2 irreversible decisions); a stale `PartyView` or a failed
   read refuses (`PARTY_STATE`).
6. **Secure mode** (ATTACK-0 `secure`, always off on Hardcore): no offensive effect on an unmarked
   character; an area effect skips it.
7. **Black skull:** a black-skulled attacker cannot harm an unmarked character.

A refused single target answers `PVP_REFUSED {reason}`; an area effect skips the actor.

### 7.3 Damage factor

PvP damage (source a character or its summon, target a character) is multiplied by
`PARTYPVP0-RL-11` (50%; 100% when the target is black-skulled) after the damage draw and before
mitigation (`PARITY_PENDING`). The same applies to each tick of a PvP-sourced condition.

### 7.4 Friendly fire and Join Aggression

- **Friendly fire:** while two party or guild members both have aggression toward the same enemy,
  their area effects may hit each other. It creates no aggression, skull, block or points.
- **Join Aggression `{member}`:** the actor takes aggression toward every character that member
  is fighting, as if it attacked each; with secure mode on, only if all are marked.

### 7.5 Blocking and expert mode (PVP-BLOCK-1)

On Open and Optional Worlds characters walk through each other, except as the expert mode
(Dove, White Hand, Yellow Hand, Red Fist) makes a character block; blocking counts as an attack.
Party and guild members never block each other. Hardcore: everyone blocks, blocking is not an
attack, the mode is locked to Red Fist. A black skull cannot select Red Fist.

## 8. Aggression, marks and blocks (PVP-RT-1)

### 8.1 Blocks

- **Logout block:** ATTACK-0's 60 s in-fight deadline (`ATTACK0-RL-03`).
- **PZ block:** a runtime deadline of 60 s (`PARTYPVP0-RL-12`), set and refreshed by aggression
  toward a character that is not a party member and not an earlier aggressor (retaliation), and by
  casting a field rune; it bars stepping onto a protection-zone tile and logout.
- **Kill block:** a kill (as a direct or assisting contributor) sets `kill_block_until` = the
  database clock + 15 minutes (`PARTYPVP0-RL-13`), durable, on every World type. It bars logout
  and stepping onto a protection-zone tile. Later violence refreshes the 60 s blocks, which can
  outlast it.
- **Combat lock** (the scope matrix row) is the union of the three. It blocks logout (FND-ID-01
  sees the blocker), leaving a party and a voluntary channel switch (GAME-CHANNEL-01 §8).
- **Durable PvP deadlines** (architect ruling; the scope matrix makes the combat lock Character +
  World, strong, and GAME-CHANNEL-01 §8 and §25 forbid reducing a PvP consequence). A PvP-sourced
  logout block (a hit dealt to or taken from a character or its summon), a PZ block and a white
  skull are written ahead to `game_character_pvp_state` (`logout_block_until`, `pz_block_until`,
  `white_skull_until`): when the runtime would set or refresh one of them past its stored value,
  one transaction under the acting Character's rule 2 session fence first writes the database
  clock + 60 s + `PARTYPVP0-RL-27` (10 s) for the attacker's and the target's affected columns
  (rows in CharacterId order, missing rows inserted, never lowered), and only then does the action
  take effect; a failed write refuses the action. A stored deadline is therefore never earlier than
  the runtime one, and a character writes at most once per `PARTYPVP0-RL-27` while fighting.
- **Admission and restart:** every admission, readmission after a node crash, reconnect and
  channel entry reads the row and restores the runtime logout block, PZ block and white skull to
  the stored deadlines still in the future, with the kill block, and the PvP damage ledger from its
  durable snapshot (§8.3). A crash can lengthen a block by
  at most `PARTYPVP0-RL-27`, never shorten or clear one. Aggression relations (§8.2) stay runtime:
  after a crash a restored white skull marks the character toward everyone, and yellow skulls and
  retaliation rights are rebuilt only by new aggression, which can add consequences to a
  retaliating character but never removes the aggressor's. A PvE-only in-fight deadline stays as
  ATTACK-0 defines it.

### 8.2 Aggression relations

- The channel runtime keeps, per actor, the set of characters it has aggression toward, each with
  its deadline (the aggressor's PZ block) and whether it was first. At most `PARTYPVP0-RL-24` (64)
  per actor; the oldest expired goes first; a full set refuses new aggression.
- **White skull:** a character with aggression toward an unmarked character, while its PZ block
  runs (Open only); after a readmission, until the restored `white_skull_until` (§8.1).
- **Yellow skull** (private, viewer-relative): shown to a viewer on a marked character that
  attacked the viewer first while that aggression runs.
- **Orange skull** (private): shown to the victim, and to the killer, on a killer with an
  unexpired, unavenged revenge mark toward the victim.
- **Red and black:** the effective durable skull (§6.2), shown to everyone.
- No green skull: the official manual has none; the party shield (§11) shows membership.

### 8.3 PvP damage ledger

Per character victim, runtime: each character contributor's damage (its summons' included) and
assists (a paralysis applied, a trap closed, a heal on an attacker) over the last `PARTYPVP0-RL-18`
(5 minutes), at most `PARTYPVP0-RL-25` (16) contributors, plus the total damage taken from all
sources in that window. Architect ruling (replaces D3's "the 17th is not tracked", which would let an
untracked attacker escape §9): a PvP damaging or assisting action by a character with no ledger slot
while all 16 are taken is refused (`PVP_REFUSED {LEDGER_FULL}`; an area effect skips that target)
before any effect, so every accepted attacker is tracked and the contributor count in §9 and §10.1
is complete. Slots free as contributors age out of the window.

**Durable snapshot** (architect ruling; §9 and §10.1 must not lose contributors to a node crash):
the ledger is written ahead with the §8.1 deadlines. Every durable deadline write that touches a
victim upserts its `game_character_pvp_ledger` rows from the runtime ledger (the acting contribution
as a row with the amounts known before it) and sets `ledger_total_buckets` in its state row; amounts
are kept per 10 s time bucket (`PARTYPVP0-RL-27`), each hit or assist adding to the bucket of its time; a hit or assist by a contributor with no durable row for that victim always
triggers such a write before it takes effect (a failed write refuses the action). So every
contributor is durable before it contributes, and the stored amounts are at most `PARTYPVP0-RL-27`
(10 s) behind. At every admission and readmission (§8.1) the runtime ledger is restored from the
buckets that are still within the window, each bucket aging out at its end + the window, so a hit
near the start of the window expires on time even when a later hit by the same contributor is
current, and a restored amount is never lower than the live one (at most 10 s longer); the stored
total ages out per bucket the same way. A contributor row with no live bucket is deleted. The victim's death transaction (§9) deletes its rows; the World job deletes expired
ones (§6.2). A crash can lose at most 10 s of amounts, never a contributor
(`PARTYPVP0-RL-31`).

## 9. Kill classification and skulls (PVP-RT-1 computes; PVP-1 commits)

- **At a character death**, the runtime classifies each contributor: justified when the victim was
  marked toward it at death (white, red or black skull; yellow toward it; an orange mark toward
  it), when `war_between` is some, or on Hardcore; else unjustified (Open only). Friendly-fire
  damage never counts.
- **Points** per unjustified contributor: 1,000 milli for a damage contributor, 500 for an
  assist-only one (`PARTYPVP0-RL-19`, `PARITY_PENDING`); with more than 5 damage contributors
  (`PARTYPVP0-RL-18`), each gets `1,000 × 5 / n`, rounded down.
- **Commit:** the classification, points, skull changes, kill blocks and revenge marks are written
  in the **victim's death transaction** (§10), keyed by its `PlayerDeathOccurrence`, which also
  deletes the victim's ledger rows (§8.3). Lock order
  after DEATH-0's (`character_root` of the victim): the `game_character_pvp_state` rows of the
  victim and every contributor in CharacterId order, then point and mark rows. A missing state row
  is inserted. Killers' `character_root` rows are not locked: no revision moves.
- **Skull evaluation** per contributor that gained points, over its rows including the new one:
  windows of 24 h, 7 d and 30 d; red when any window reaches 3,000 / 5,000 / 10,000 milli
  (`PARTYPVP0-RL-14`), black at 6,000 / 10,000 / 20,000 (`PARTYPVP0-RL-15`); a red or black skull
  gets `skull_until` = now + 30 or 45 days (`PARTYPVP0-RL-16`), reset by any new points. A skull
  never goes down by evaluation; it ends at `skull_until`. The thresholds are the manual's lower
  bounds (`PARITY_PENDING`).
- **Revenge:** an unjustified kill writes a mark (victim, killer); a kill of that killer by that
  victim avenges its oldest open mark.
- **XP:** a PvP kill gives no XP.
- **Assist hook** (`on_player_kill`, §12): the committed classification with contributors and
  assists is published to GUILD-WAR-0 once it exists.

## 10. Death interplay (PVP-DEATH-1)

### 10.1 PvP death

A death is a PvP death when PvP damage is at least `PARTYPVP0-RL-20` (5%) of the damage taken in
the last 5 minutes, or the final blow came from a character or its summon. It commits through
DEATH-1's transaction and replaces the D54 floor for PvP-contributed deaths.

### 10.2 Outcome

- **Red or black skull at death:** every equipped item and the backpack are lost; blessings and an
  Amulet of Loss do not protect; regular blessings are consumed, PvP death or not; Twist of Fate
  neither helps nor is consumed. D65 (no loss up to level 8) still applies.
- **Other PvP deaths:** the D62 ladder and the Amulet of Loss as in PvE. Twist of Fate, if held,
  keeps the regular blessings and is consumed; without regular blessings it is kept.
- **Adventurer's Blessing** (Open only): a character at level 20 or less (`PARTYPVP0-RL-22`) with
  `adventurer_forfeited` false loses no XP, item or blessing in a PvP death.
- **Unfair fight:** the reduction when the attackers' level sum exceeds the victim's level has no
  sourced formula; the receipt records 0 until one is found (`PARITY_PENDING`, never guessed).
  Attackers at war with the victim are excluded from the sum (`war_between`).
- **Black skull respawn:** 40 HP and 0 mana (`PARTYPVP0-RL-21`) instead of D63's full values.
- **XP loss:** D58, D59 and D68 unchanged.

### 10.3 Receipt

The DEATH-0 receipt gains `pvp_death`, `skull_at_death`, `twist_of_fate_used`,
`adventurer_applied` and `unfair_fight_milli`, and the command binding also covers the
classification inputs of §9 (DEATH-0 §3.1 amendment). Blessing changes stay in the same
transaction. Twist of Fate is the blessing key `twist_of_fate`; its sale belongs to DEATH-4, whose
D178 question covers it.

### 10.4 Forfeiting Adventurer's Blessing

The first aggression of a character toward another character sets `adventurer_forfeited` in one
transaction under the rule 2 session fence, before the action takes effect; a failed write refuses
the action. Reaching level 21 needs no write.

## 11. Wire (PVP-WIRE-1)

- **Capabilities `PARTY_V1` and `PVP_V1`**; their numbers, two command types and two domains are
  reserved on #162 at allocation.
- **`PARTY_INTENT`** (a oneof; an empty oneof is `REJECTED`): `invite {actor}`, `revoke {name}`,
  `accept {invitation}`, `decline {invitation}`, `leave`, `pass_leadership {name}`,
  `shared_xp {enabled}`, `block {actor}`, `unblock {handle}`, `party_invites {anyone | nobody}`,
  `channel_visibility {party | hidden}`. Invitations are addressed by a per-session handle, never a PartyId or
  CharacterId. Results: `OK`, `NOT_LEADER`, `ALREADY_IN_PARTY`, `PARTY_FULL`, `NOT_INVITED`,
  `NOT_VISIBLE` (also a block, `party_invites` or invitee-cap refusal, §4.1), `LOGOUT_BLOCKED`
  (any combat lock, §4.1), `RATE_LIMITED`,
  `PARTY_UNAVAILABLE`, plus the common results.
- **Domain `PARTY`:** members (§4.4), leader, shared-XP state (on, off, or the failing member),
  the character's pending unexpired invitations (at most `PARTYPVP0-RL-29`) with the inviter's name
  and expiry, its own blocked list and its `party_invites` and `channel_visibility` settings.
- **`PVP_INTENT`:** `join_aggression {actor}`. **`FIGHT_MODES_INTENT`** gains `expert_mode` in a
  new revision behind `PVP_V1` (ATTACK-WIRE-1's command).
- **Domain `PVP`:** own effective skull and its end, points per window against both thresholds,
  kill block end, PZ block flag, Adventurer's Blessing.
- **VIS-2 entity entry** (new revision behind `PVP_V1`): viewer-relative skull (none, white,
  yellow, red, black, orange), party shield (leader, member, invited, inviting, shared-XP state),
  frame (none, yellow, orange, brown), and a war emblem slot that GUILD-WAR-0 fills.

## 12. Hooks for GUILD-WAR-0 (named only)

- `war_between(a, b) -> Option<WarId>`: none until GUILD-WAR-0 (§7.2, §9, §10.2).
- `on_player_kill(classification)`: the committed kill with contributors and assists (§9).
- `AssistLedger`: the §8.3 assists; war kills against guildmates never count as assisted.
- The VIS-2 war emblem slot (§11) and a war frame colour.

## 13. Rows (registered by the children before implementation)

| Row | Value |
|---|---|
| `PARTYPVP0-RL-01` members per party | 50 (D109) |
| `PARTYPVP0-RL-02` open invitations per party | 50 |
| `PARTYPVP0-RL-03` invitations sent per character | 10 per minute |
| `PARTYPVP0-RL-04` shared-XP range | 30 tiles from the leader, any floor, same channel |
| `PARTYPVP0-RL-05` shared-XP level spread | lowest × 3 ≥ highest × 2 |
| `PARTYPVP0-RL-06` shared-XP activity window | 2 minutes (`PARITY_PENDING`) |
| `PARTYPVP0-RL-07` shared-XP bonus | +20% (kills ≥ 20 XP); 2, 3, 4+ vocations +30%, +60%, +100% |
| `PARTYPVP0-RL-08` member without a live session | removed after 60 s |
| `PARTYPVP0-RL-09` PvP minimum level | 8 |
| `PARTYPVP0-RL-10` post-login PvP immunity | 10 s |
| `PARTYPVP0-RL-11` PvP damage factor | 50%; 100% on a black skull |
| `PARTYPVP0-RL-12` PZ block | 60 s, refreshed |
| `PARTYPVP0-RL-13` kill block | 15 minutes |
| `PARTYPVP0-RL-14` red skull thresholds | 3,000 / 5,000 / 10,000 milli per 24 h / 7 d / 30 d |
| `PARTYPVP0-RL-15` black skull thresholds | 6,000 / 10,000 / 20,000 milli |
| `PARTYPVP0-RL-16` skull duration | red 30 days, black 45 days, reset by new points |
| `PARTYPVP0-RL-17` revenge mark | 7 days or until avenged |
| `PARTYPVP0-RL-18` kill credit | 5 contributors in full; window 5 minutes |
| `PARTYPVP0-RL-19` assist-only share | 500 milli (`PARITY_PENDING`) |
| `PARTYPVP0-RL-20` PvP death | 5% PvP damage in 5 minutes, or a PvP final blow |
| `PARTYPVP0-RL-21` black skull respawn | 40 HP, 0 mana |
| `PARTYPVP0-RL-22` Adventurer's Blessing | level 20 or less, Open only |
| `PARTYPVP0-RL-23` point row retention | 45 days |
| `PARTYPVP0-RL-24` aggression relations per actor | 64 |
| `PARTYPVP0-RL-25` PvP contributors per victim | 16 |
| `PARTYPVP0-RL-26` `PartyView` revision check | every 5 s; stale after 10 s unconfirmed |
| `PARTYPVP0-RL-27` durable PvP deadline write-ahead | 10 s beyond the 60 s block |
| `PARTYPVP0-RL-28` blocked characters per character | 100 |
| `PARTYPVP0-RL-29` open invitations per invitee | 20 |
| `PARTYPVP0-RL-30` invitation lifetime | 5 minutes; expired rows deleted by the cleanup job, 100 per pass |
| `PARTYPVP0-RL-31` durable PvP ledger | at most 16 rows per victim, at most 10 s behind, deleted 5 minutes after `last_at` or at the victim's death |
| PvP legality party check | 1 indexed read per character target; 1 batched read per area effect |
| Party operation | 1 transaction, 0 items, 0 value lines, 1 relay hint |
| PvP additions to a death | at most 17 state rows, 16 point rows, 16 mark rows, 16 ledger rows deleted; no value lines |
| PvP deadline write | 1 transaction, at most 2 state rows and 16 ledger rows, per character at most once per 10 s plus once per new contributor |

## 14. Rejected options

- **A party held in one node's memory.** Members span channels and nodes (PartyId baseline).
- **Party rows as Character state.** Every invite would advance a revision; the scope matrix
  makes membership World state.
- **Invites by name.** A name lookup would reveal presence and hidden alternates (social
  baseline); an invite needs a visible target, as in Tibia.
- **Skulls and frags in runtime only.** A relog would clear a red skull (scope matrix: durable).
- **Advancing killers' `CharacterRevision`.** Killers issue no command, and one death would couple
  several revision chains; PvP rows are consequence rows like the anti-hopping guard.
- **One frag counter with decay jobs.** Rolling windows over point rows are exact and replayable.
- **A PvP damage path beside GAME-ABILITY-01.** The accepted gate forbids a second engine.
- **Shared experience across channels.** The scope matrix limits it to colocated members.
- **Guessing the unfair-fight formula or retro rules.** No source; guessing would fake parity.

## 15. Owner-rule applications (Global parity, 5905825574)

Kept as in Tibia: party forming, leadership, leaving and succession; shared experience and its
bonuses; party immunity and friendly fire; world types; level 8; PZ rules; 10 s login immunity;
50% PvP damage; justified and unjustified kills, assists and the 5-contributor rule; white,
yellow, red, black and orange skulls and their durations; logout, PZ and 15-minute kill blocks;
secure and expert modes; Join Aggression; red and black loss; Twist of Fate; Adventurer's
Blessing; black skull respawn; no XP for PvP kills. D60 is superseded by the owner's direction of
2026-09-30. The first World is Optional PvP (owner answer P1). Declared differences: at most 50 members (D109) and bounded invitations (20 per
invitee, expiring after 5 minutes); leaving a party is barred by the whole combat lock; benefits only
for members on the same channel; the "battle sign" read as the PZ block; the 2-minute activity
window, the assist share and the lower-bound thresholds (`PARITY_PENDING`); no unfair-fight
reduction until a source; no Retro types yet; no green skull; a block list and a party-invite
setting and a channel-visibility setting (social baseline); PvP party relations confirmed against
durable rows, so a just-changed membership can refuse an attack; a node crash can lengthen a PvP block by up to 10 s, never shorten it,
and ends yellow skulls and retaliation relations and can lose at most 10 s of PvP ledger amounts.

## 16. Owner questions

**P1. Which PvP type does the first World use?** It decides whether players can attack each other
there at all; the engine supports all three (§6.1). a) Open PvP: skulls and the full rule set, the
most common Global type (recommended); b) Optional PvP: no PvP until guild wars exist;
c) Hardcore PvP: no skulls, no restrictions.

Owner answer (2026-09-30, #162): b and a — the PvP type is per-World configuration; Optional PvP
and Open PvP (with the full skull system) are both delivered; the first World launches as Optional
PvP, and Open PvP is available for a World configured with it. The owner confirmed this reading of
"b i a" ("b and a") on 2026-09-30 (#162, A1).

**P2. Should Hardcore PvP be dropped from the v1 engine values?** a) drop it; b) keep it as an
unused value.

Owner answer (2026-09-30, #162): b — `HARDCORE` stays a v1 value with the rules of §7; no World is
configured with it (§6.1, §7).

## 17. Decision test

- **Must decide now:** YES. The owner asked for parties and PvP now; ATTACK-0, the first player
  death decision, CONDITIONS-0, D3 and D118 wait for them.
- **Minimum sufficient:** three party tables, two consent tables, four PvP tables, rules in the existing legality
  stage, PvP fields in the existing death transaction, two wire capabilities.
- **Superseding evidence:** an official source for the thresholds, the activity window, the
  unfair-fight formula or the retro rules.
- **Deliberately not decided:** guild wars, arenas and PvP zones, the Party Finder, the Party Hunt
  Analyser, Retro Worlds, Death Redemption, blessing sales (DEATH-4).

## 18. Before-freeze checklist

1. **Contract amendments:** ATTACK-0 §3 and §4, the first player death decision §4.1 and §4.5,
   DEATH-0 §3.1, D3 §4.4, the composition decision and the scope matrix, each written "pending on
   acceptance of PARTY-PVP-0".
2. **Serialization:** §4.1's and §9's lock orders; World jobs lock rows before re-checking them.
3. **Restart:** party and PvP rows are durable and keyed; PvP logout and PZ blocks, the white
   skull, the kill block, skulls and the PvP ledger's contributors survive every restart (§8.1,
   §8.3); a PvE-only in-fight deadline
   follows ATTACK-0; a node rebuilds its `PartyView`s before party readiness (§4.2).
4. **Typed references:** PartyId with WorldId, CharacterId, `PlayerDeathOccurrence`, actor ids on
   the wire; never a PartyId or CharacterId from a client as authority.
5. **Wire:** §11, capabilities `PARTY_V1` and `PVP_V1`.
6. **Split work:** one party per transaction; PvP rows only inside the victim's death transaction;
   at most 100 rows per cleanup pass; a leader-only party ends in the leader's leave, logout or
   cleanup transaction.
7. **World type:** the PvP children build and test `OPTIONAL` and `OPEN`; the first World's
   ruleset has `pvp_type` `OPTIONAL` (owner answer P1).
