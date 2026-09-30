# PARTY-PVP-0 Parties and PvP

- Decision: `PARTYPVP0-PARTIES-AND-PVP-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence,
  combat, security, privacy and protocol) and protected integration. Owner question P1 (§16) is
  open; it chooses the first World's PvP type and blocks only PvP going live, not any child.
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
| PARTY-1 | hard, persistence, security and privacy review | party, member and invitation tables; invite, accept, decline, revoke, leave, pass leadership, shared-XP toggle; the node `PartyView` cache and the relay change hint; the cleanup job (§3, §4) | this decision; CHAT-2 |
| PARTY-XP-1 | hard (combat), combat review | shared-experience eligibility and split on the D118 XP slice; party immunity in area effects; the party loot right; the `PartyView` query for `party_buff` and the monk party rules (§5) | PARTY-1; the D118 XP lane; D3-4 |
| PARTY-CHAT-1 | impl, security review | one party room per party on the CHAT-0 World relay (§4.3) | PARTY-1; CHAT-2 |
| PVP-1 | hard, persistence and security review | PvP state, unjustified point and revenge mark tables; skull evaluation in the death transaction; the World cleanup job; the `rulesets/pvp/` and `rulesets/party/` rows (§6, §9) | DEATH-1; PARTY-1 |
| PVP-RT-1 | hard (combat), combat and security review | PvP legality in the GAME-ABILITY-01 legality stage; aggression relations; white and yellow skulls; logout, PZ and kill blocks; the damage factor; the PvP damage ledger; kill classification; Join Aggression; friendly fire (§7, §8) | ATTACK-1; COND-1; PVP-1 |
| PVP-DEATH-1 | hard (persistence), persistence review | the PvP variants of the death outcome: PvP death, red and black skull loss, Twist of Fate, Adventurer's Blessing, black skull respawn (§10) | DEATH-1; DEATH-3; PVP-1 |
| PVP-BLOCK-1 | impl, movement review | walking through characters and expert-mode blocking (§7.5) | PVP-RT-1; SPEED-1 |
| PVP-WIRE-1 | impl, protocol review | capabilities `PARTY_V1` and `PVP_V1`, the party and PvP commands and domains, VIS-2 skull, shield and frame fields (§11) | PARTY-1; PVP-RT-1; VIS-2; ATTACK-WIRE-1 |

PvP goes live on a World only when PVP-1, PVP-RT-1, PVP-DEATH-1 and PVP-WIRE-1 have landed; until
then ATTACK-0's "creatures only" stays. Later, each with its own decision: GUILD-WAR-0 (fills the
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
  - `game_party_invitations`: (party, invitee), `seq`, `created_at`. At most `PARTYPVP0-RL-02`
    (50) per party.
- **Party size** `PARTYPVP0-RL-01` (50, D109).
- **No `CharacterRevision` advance**: party rows are World social state (the scope matrix), not
  Character state (composition rule 1 amendment).
- **Lifetime.** A party exists from the first invitation to its end. It ends when it has no
  member but the leader and no invitation; its rows are deleted. A PartyId is never reused, and no
  party history is kept (Tibia keeps none).

## 4. Party operations, cache and chat (PARTY-1, PARTY-CHAT-1)

### 4.1 Operations

Each is one transaction through a SECURITY DEFINER function under the composition rule 2 session
fence of the acting Character, replayed by its occurrence and a SHA-256 request binding (as
HOUSE-OWN-0 §4). Lock order: the occurrence; the acting Character's fence; the party row; member
rows by CharacterId; invitation rows.

- **Invite `{target actor}`.** The target is a character visible to the actor (VIS-2, so on the
  same channel), not in a party. An actor without a party creates one and becomes leader. Only the
  leader invites. At most `PARTYPVP0-RL-03` (10) invitations per character per minute.
- **Revoke `{invitee}`** by the leader; **decline** by the invitee.
- **Accept `{party}`.** The invitee holds a valid invitation and is in no party; it joins with
  the invitation's `seq`. Accepting removes its other invitations. The members need not be on the
  same channel (PartyId baseline).
- **Leave.** Refused while the member's logout block runs (§8.1). A leaving leader passes to the
  remaining member with the lowest `seq`.
- **Pass leadership `{member}`** by the leader, to any member.
- **Shared experience `{on | off}`** by the leader.
- **Logout** removes the member in the logout transaction. A channel switch keeps membership: it is
  not a logout (PartyId baseline).
- **Cleanup job** (World job, recovery fence and admission relations only): removes a member
  whose Character has no session row in state 1 or 2 for `PARTYPVP0-RL-08` (60 s), which covers a
  node crash and outlasts a channel switch.

### 4.2 Node cache

- Each node keeps a `PartyView` per party with a local member: members, leader, shared-XP flag,
  `revision`. After a commit, the writer sends a sealed hint `{party_id, revision}` on the CHAT-0
  World relay; nodes re-read the rows. A node whose listener is down is not ready for parties:
  party commands answer `PARTY_UNAVAILABLE`, and shared experience is off for its members.
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

## 5. Party benefits (PARTY-XP-1)

All benefits are channel-local: only members on the same channel count (scope matrix).

### 5.1 Shared experience

- **Eligible** at a kill when the leader enabled it and, on the kill's channel: the leader is
  present; every member of the party is on that channel within `PARTYPVP0-RL-04` (30 tiles,
  Chebyshev, any floor) of the leader; the lowest level × 3 ≥ the highest level × 2
  (`PARTYPVP0-RL-05`); every member was active within `PARTYPVP0-RL-06` (2 minutes: healed a
  member or attacked an aggressive monster; the D118 UNKNOWN, `PARITY_PENDING`); the leader is not
  under a PZ block. The manual's "battle sign" is read as the PZ block, because the logout block
  runs whenever a party hunts (`PARITY_PENDING`).
- **Split** (D118): the bonus of `PARTYPVP0-RL-07` over the base XP, a summon's share first, then
  equal shares rounded up; a stamina-reduced share is removed without changing others. Each member's
  award is its own XP receipt on its own Character sequencer (CHAR-REV-SEQ-1); reward principals
  per death at most 50 (D109). Soul points as D118.
- **Off:** any failed condition turns it off for the kill, and XP goes by damage share. The
  level-up stickiness of the manual follows from re-evaluating at every kill.

### 5.2 Party immunity and buffs

- Party members and their summons are immune to each other's offensive effects and area damage
  (legality stage, §7.2), except friendly fire (§7.4).
- **Hook** `PartyView::members_in_area(caster, area, same_floor)` for `party_buff`, the monk
  Serene and virtue rules and any later party rule. It returns members on the caster's channel only.

### 5.3 Party loot right (D3 amendment)

- The corpse receipt also captures `top_damage_party_id`, the top-damage character's PartyId at
  death (from the runtime view). During the 10 s window, a pickup is also admitted for a character
  that is a current member of that party, read from `game_party_members` FOR SHARE. Player corpses
  stay exempt (D121).

## 6. World PvP type and PvP state (PVP-1)

### 6.1 World PvP type

- `pvp_type` is a World ruleset field (`rulesets/pvp/`), the same on every channel, bound into the
  ruleset revision. It changes only with a new ruleset revision at a planned reset; a change never
  clears durable PvP state.
- Values in v1: `OPTIONAL`, `OPEN`, `HARDCORE`. `RETRO_OPEN` and `RETRO_HARDCORE` are refused by
  ruleset validation until their rules have a source (retro frags, PvP XP formula, 6.31%
  blessings).

### 6.2 Tables

Character + World, strong durable, one state on every channel (the scope matrix row):

- `game_character_pvp_state`: `character_id` primary key, `world_id`, `skull` (`NONE`, `RED`,
  `BLACK`), `skull_until`, `kill_block_until`, `adventurer_forfeited`, `revision`.
- `game_character_unjustified_points`: (`character_id`, `death_occurrence_id`) primary key,
  `victim_character_id`, `points_milli` (1 to 1,000), `committed_at`.
- `game_character_revenge_marks` (orange): (`victim_character_id`, `killer_character_id`,
  `death_occurrence_id`), `expires_at` = +7 days (`PARTYPVP0-RL-17`), `avenged_at`.
- **No `CharacterRevision` advance** (composition rule 1 amendment): like the GAME-CHANNEL-01 §9
  anti-hopping guard, these are PvP-domain consequence rows keyed by a Character, not Character
  progression. They are written by death transactions (§9) and one runtime write (§10.4).
- **Effective skull** is read lazily: `skull` if `skull_until` is later than the database clock,
  else none. A World job deletes point rows older than `PARTYPVP0-RL-23` (45 days) and expired
  marks, at most 100 per pass.

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
   except friendly fire (§7.4).
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
  sees the blocker), leaving a party and a voluntary channel switch (GAME-CHANNEL-01 §8). The
  runtime blocks survive a same-GameSession reconnect with the actor (ATTACK-0); a node crash ends
  them; the kill block survives everything and is re-read at admission.

### 8.2 Aggression relations

- The channel runtime keeps, per actor, the set of characters it has aggression toward, each with
  its deadline (the aggressor's PZ block) and whether it was first. At most `PARTYPVP0-RL-24` (64)
  per actor; the oldest expired goes first; a full set refuses new aggression.
- **White skull:** a character with aggression toward an unmarked character, while its PZ block
  runs (Open only).
- **Yellow skull** (private, viewer-relative): shown to a viewer on a marked character that
  attacked the viewer first while that aggression runs.
- **Orange skull** (private): shown to the victim, and to the killer, on a killer with an
  unexpired, unavenged revenge mark toward the victim.
- **Red and black:** the effective durable skull (§6.2), shown to everyone.
- No green skull: the official manual has none; the party shield (§11) shows membership.

### 8.3 PvP damage ledger

Per character victim, runtime: each character contributor's damage (its summons' included) and
assists (a paralysis applied, a trap closed, a heal on an attacker) over the last `PARTYPVP0-RL-18`
(5 minutes), at most `PARTYPVP0-RL-25` (16) contributors (D3's rule: the 17th is not tracked), plus
the total damage taken from all sources in that window.

## 9. Kill classification and skulls (PVP-RT-1 computes; PVP-1 commits)

- **At a character death**, the runtime classifies each contributor: justified when the victim was
  marked toward it at death (white, red or black skull; yellow toward it; an orange mark toward
  it), when `war_between` is some, or on Hardcore; else unjustified (Open only). Friendly-fire
  damage never counts.
- **Points** per unjustified contributor: 1,000 milli for a damage contributor, 500 for an
  assist-only one (`PARTYPVP0-RL-19`, `PARITY_PENDING`); with more than 5 damage contributors
  (`PARTYPVP0-RL-18`), each gets `1,000 × 5 / n`, rounded down.
- **Commit:** the classification, points, skull changes, kill blocks and revenge marks are written
  in the **victim's death transaction** (§10), keyed by its `PlayerDeathOccurrence`. Lock order
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
  `shared_xp {enabled}`. Invitations are addressed by a per-session handle, never a PartyId or
  CharacterId. Results: `OK`, `NOT_LEADER`, `ALREADY_IN_PARTY`, `PARTY_FULL`, `NOT_INVITED`,
  `NOT_VISIBLE`, `LOGOUT_BLOCKED`, `RATE_LIMITED`, `PARTY_UNAVAILABLE`, plus the common results.
- **Domain `PARTY`:** members (§4.4), leader, shared-XP state (on, off, or the failing member),
  the character's pending invitations with the inviter's name.
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
| Party operation | 1 transaction, 0 items, 0 value lines, 1 relay hint |
| PvP additions to a death | at most 17 state rows, 16 point rows, 16 mark rows; no value lines |

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
2026-09-30. Declared differences: at most 50 members (D109) and bounded invitations; benefits only
for members on the same channel; the "battle sign" read as the PZ block; the 2-minute activity
window, the assist share and the lower-bound thresholds (`PARITY_PENDING`); no unfair-fight
reduction until a source; no Retro types yet; no green skull; runtime blocks end at a node crash
while the kill block survives.

## 16. Owner questions

**P1. Which PvP type does the first World use?** It decides whether players can attack each other
there at all; the engine supports all three (§6.1). a) Open PvP: skulls and the full rule set, the
most common Global type (recommended); b) Optional PvP: no PvP until guild wars exist;
c) Hardcore PvP: no skulls, no restrictions.

## 17. Decision test

- **Must decide now:** YES. The owner asked for parties and PvP now; ATTACK-0, the first player
  death decision, CONDITIONS-0, D3 and D118 wait for them.
- **Minimum sufficient:** three party tables, three PvP tables, rules in the existing legality
  stage, PvP fields in the existing death transaction, two wire capabilities.
- **Superseding evidence:** an official source for the thresholds, the activity window, the
  unfair-fight formula or the retro rules; owner answer P1.
- **Deliberately not decided:** guild wars, arenas and PvP zones, the Party Finder, the Party Hunt
  Analyser, Retro Worlds, Death Redemption, blessing sales (DEATH-4).

## 18. Before-freeze checklist

1. **Contract amendments:** ATTACK-0 §3 and §4, the first player death decision §4.1 and §4.5,
   DEATH-0 §3.1, D3 §4.4, the composition decision and the scope matrix, each written "pending on
   acceptance of PARTY-PVP-0".
2. **Serialization:** §4.1's and §9's lock orders; World jobs lock rows before re-checking them.
3. **Restart:** party and PvP rows are durable and keyed; runtime blocks follow ATTACK-0; the kill
   block and skulls survive every restart.
4. **Typed references:** PartyId with WorldId, CharacterId, `PlayerDeathOccurrence`, actor ids on
   the wire; never a PartyId or CharacterId from a client as authority.
5. **Wire:** §11, capabilities `PARTY_V1` and `PVP_V1`.
6. **Split work:** one party per transaction; PvP rows only inside the victim's death transaction;
   at most 100 rows per cleanup pass.
