# BOSS-RAID-0 Bosses, raids and the Bosstiary

- Decision: `BOSSRAID0-BOSSES-RAIDS-BOSSTIARY-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence,
  economy, security, concurrency and protocol) and protected integration. R1-R3 (§17) are
  architect rulings; owner question R4 (§17) is answered (2026-09-30, #162): the slot swap fee is
  a gold sink as in Tibia, the first change after a server save free (§10.3).
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the owner's direction (2026-09-30): build bosses, raids and the Bosstiary now, full
  Tibia Global parity. It is the "boss and raid default policies" the scope matrix leaves open,
  ADR-0001 follow-up 7 (rewards, bosses, raids and event scope), the boss loot D77 defers, the
  reward domain the encounter format D27 names, and the Bosstiary CHARM-0 §6 excludes.
- Builds on: ADR-0001 §7, §10 and §12; the scope matrix (boss, boss reward eligibility, raid,
  instanced dungeon rows; the reward anti-hopping contract); the instance baseline
  (`INSTANCE_SCOPE_AND_RUNTIME_OWNER_BASELINE.md`, owner-accepted); the encounter authoring format
  (D26, D27, E3); the monster schema D9 (`bosstiary`, `reward_boss`); GAME-AI-01's first creature
  slice (D55, D57, process-local respawn); DUR-03 A4 (creature death key, D52); D77, D78 and
  D109; D3 (D112, D132 contributors); D118, D119, D121; CHARM-0 (the Bestiary pattern, the credit
  window); QUEST-STATE-0 §5.2 (CHAR-REV-SEQ-1); the composition decision rules 1-6; ADR-0021
  (`WorldReset`, reset epoch); HOUSE-OWN-0 §9 (World jobs); HOUSE-RUNTIME-0 (branch
  `claude/arch-house-runtime-0`, SCOPE-HANDOFF-1); PARTY-PVP-0 (branch `claude/arch-party-pvp-0`,
  party loot right); MARKET-0 and DEPOT-0; the gold fee decision (D178) and BANK-FEE-0 (the bank
  part); owner rule 5905825574 (Global parity)
- Amends, each pending on acceptance of BOSS-RAID-0, in this PR: the scope matrix (boss and raid
  rows); the VSL combat resource rows decision (D77 boss loot, row 10); the encounter authoring
  format §10 (D27 consumer); the composition decision (rule 1 covers boss eligibility rows);
  CHARM-0 §6 (Bosstiary pointer).
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| RAID-CONTENT-1 | content lane | `content/encounters/raids/` from the 88 Canary raid files, checked against TibiaWiki (§4.1) | this decision; the encounter admission (E4) |
| RAID-1 | hard, persistence and concurrency review | raid schedule, firing and channel-run tables; the scheduler World job; channel raid runs and announcements (§4) | RAID-CONTENT-1; CHAT-1 |
| BOSS-1 | hard, persistence review | durable boss spawn clocks for open-world bosses (§5); the boss cooldown and eligibility tables (§6.3, §9) | this decision |
| BOSS-ROOM-1 | hard, security and durability review | lever admission into one activity instance per group; cooldown commit and compensation; instance lifetime, exit and recovery (§6) | BOSS-1; SCOPE-HANDOFF-1; the encounter runtime (§6.5) |
| BOSS-REWARD-1 | hard, persistence, economy and security review | contribution tracking for reward bosses; the per-participant reward draw; the `CharacterRewardChest` location, claim and expiry; its DUR-03 amendment (§7-§9) | BOSS-1; D3-3; PARTY-XP-1 |
| BOSSTIARY-1 | hard, persistence review | Bosstiary kill receipts and progress; boss slots and the slot swap fee; the Boosted Boss draw (§10, §11) | CHAR-REV-SEQ-1; BOSS-REWARD-1 (the credited set); GOLD-FEE-1b; GOLD-FEE-2 for the fee's bank part |
| BOSS-WIRE-1 | impl, protocol review | capability `BOSS_V1`: raid announcement, Bosstiary, boss slots, boss cooldowns, reward chest (§13) | BOSSTIARY-1; BOSS-REWARD-1; RAID-1 |

Order: BOSS-1 and RAID-CONTENT-1 first; RAID-1 can run raids of ordinary creatures before any
boss child lands. BOSS-REWARD-1 before BOSSTIARY-1, because the Bosstiary counts the credited set.
Later, each with its own decision: the encounter runtime (ENCOUNTER-RT-0, named in §6.5), World
Changes and Mini World Changes, the Tibiadrome, the Party Finder, the Boosted Creature (a Bestiary
feature), the Cyclopedia map raid warning, Hazard and the Soul War taint rewards.

## 1. Question

How do raids fire and recover on a multichannel World, how are bosses spawned, entered and
rewarded without cross-channel farming, and where does Bosstiary progress live?

## 2. Facts

**PROVEN**

- Scope matrix: "Creature and spawn runtime" is Channel. "Boss runtime" is "`ChannelRuntime` or
  explicit world event owner", "must declare scope", "no implicit default". "Boss reward
  eligibility" is Character/Account/World, strong durable, "prevent repeated farming across
  channels". "Raids and events" is World/Channel/Instance, "scope declared in event definition".
  The reward anti-hopping contract: "five independent boss copies ... does not imply five reward
  claims are allowed".
- ADR-0001 §12 makes duplicate boss rewards across channels and channel changes through an
  instance exploit classes to make impossible; §10 forbids a channel change during a protected
  encounter or an instance ownership transition.
- Instance baseline (owner-accepted): an instance is `WorldId + InstanceId`, never owned by a
  source channel; a lever is an admission request, not a teleport; all participants are validated
  atomically, and one failure fails the whole admission; each participant keeps an origin channel;
  entry is seamless, make-before-break. Party Finder and scheduled events use the same admission.
- Encounter format: D26 encounters run one instance per party by default, `channel_shared` is an
  explicit opt-in; D27 encounters only emit outcomes, and boss cooldowns, reward eligibility and
  reward rooms belong to the reward domain; E3 an encounter-bound creature is never activated
  without its encounter. 61 encounters are admitted (`content/encounters/definitions/`).
- Content: 1,479 Creature definitions; 202 carry a `bosstiary` block (56 bane, 73 archfoe, 73
  nemesis) with kills and points per level; 291 have `reward_boss`; 174 have both; none has both a
  Bestiary and a Bosstiary block. `content/encounters/raids/`, `content/creatures/bosstiary/` and
  `rulesets/progression/bosstiary/` are `READY_UNPOPULATED`. The client staticdata lists 447 bosses
  (`imports/cipsoft-staticdata/bosses/`).
- GAME-AI-01 slice: spawns are `EphemeralScopeReset`; respawn timers are process-local; a restart
  realizes every spawn again (D55). D57: 16 spawns and 64 creatures per scope.
- DUR-03 A4: the death key is (WorldId, ChannelId, ScopeOwnershipGeneration, ActorLocalId,
  ActorLocalGeneration); an InstanceRuntime uses its scope ref in place of ChannelId. D52:
  uncommitted loot and XP of a death are lost on a generation change, never duplicated.
- D77: 16 loot entries per death, "Bosses get a separate decision". D109: up to 50 reward
  principals per death. D3 D132: top-damage attribution is in-memory, at most 16 contributors
  per creature. D121: loot right to the top-damage character or party, then everyone.
- CHARM-0 and CHARM-2 (`0019`, `bestiary_progress.rs`): per (Character, race) kill receipts in a
  chain, stages derived; the Bestiary write is a death descendant after XP on the Character
  sequencer (QUEST-STATE-0 §5.2).
- ADR-0021: a durable planned World reset with an epoch is the Tibia server save; its time is
  operational.

**CIPSOFT_OFFICIAL** (the Tibia manual and the tibia.com snapshot `2026-09-28-160207Z`)

- `world.md` §5.2.1: "Monster raids": scripted or ad-hoc surprise group spawns that can assault
  travellers or a city. `controls_communication.md`: raid notices go to the Server Log.
- `interface.md` Bosstiary: silhouette until the first kill; tiers Bane, Archfoe, Nemesis; kills
  for Prowess 25/5/1, Expertise 100/20/3, Mastery 300/60/5.
- Boss Slots: need Prowess; one free swap per server save, further swaps the same day cost rising
  gold (formula not stated); +25% chance of an extra equipment loot set (capped-drop items
  excluded), +25% more at Mastery; Boss Points (per progress level) raise the bonus up to 182%;
  the second slot at 1,500 points.
- Boosted Boss: a random Archfoe daily; kills count 3x; a bonus loot chance; the cooldown is reset
  for every character at server save.
- `combat.md`: Bestiary excludes boss and raid monsters. tibia.com: "Boss Cooldowns ... how much
  longer you need to wait until you can fight a certain boss again".

**OTS_HYPOTHESIS_ONLY**

- Canary audit: 88 reference raid files (Crystal 152). The Crystal checkpoint: 55 raid monster
  files; Morgaroth is split into a monster, a raid XML (announcements, timing, place) and a spell.
- Monster schema D9: arena, phases, cooldowns and the reward chest belong to the Encounter; the
  `rewardBoss` registration is an approved omission because reward tracking is the reward domain.

**UNKNOWN** (captured by the named child before it freezes; `PARITY_PENDING` until then)

- The reward contribution formula, the reward chest expiry and item limits (BOSS-REWARD-1).
- The boss slot bonus curve and the swap fee formula (BOSSTIARY-1).

## 3. Scope

| Thing | Owner | Scope | Rule |
|---|---|---|---|
| Raid schedule and firing | Raid World job | World | one durable firing per draw (§4) |
| Raid run | `ChannelRuntime` | Channel | one run per (firing, channel) (§4.4) |
| Open-world boss spawn | `ChannelRuntime` + durable clock | Channel | independent per channel (§5) |
| Boss room fight | `InstanceRuntime` | Instance | one instance per admitted group (§6) |
| Boss cooldown | reward domain | Character | strong durable, one per (Character, boss) (§6.3) |
| Reward eligibility | reward domain | Character + World | strong durable, anti-hopping (§9) |
| Reward chest | DUR-03 location | Character + World | strong durable (§8) |
| Bosstiary | Character Authority | Character | CharacterRevision on the sequencer (§10) |
| Boosted Boss | reset activation | World per reset epoch | derived from the date (§11) |

## 4. Raids (RAID-1)

### 4.1 Definition (content)

A raid is an Encounter with `encounter_type` Raid and scope `channel_shared`, plus:

- `schedule`: `random {mean_interval_s, min_gap_s}` or `fixed {weekday or day of month, offset
  after reset}`;
- `announcements[]`: `{delay_s, text, kind}`, kind `warning` or `advance`;
- `waves[]`: `{delay_s, entries[] {creature, count, area anchor}}`;
- `despawn_after_s` (optional; otherwise raid creatures stay until the next reset, as in Tibia);
- `channels`: every live channel (ruling R1, §17).

Validation refuses a raid above `BOSSRAID0-RL-03` creatures or `BOSSRAID0-RL-04` waves. A raid's
creatures and anchors must be admitted (E4). A boss wave entry with `reward_boss` uses §7-§9.

### 4.2 Schedule and firing

- `game_world_raid_schedules`: one row per (World, raid key): `last_fired_at`, `next_check_at`,
  `check_ordinal`, `next_firing_ordinal`. Created for each raid of the active bundle at reset
  activation; a retired raid's row stays, marked retired.
- **Check** (World job, every `BOSSRAID0-RL-01`): for a due `random` raid, the draw uses the
  simulation-determinism RNG seeded by (WorldId, raid key, `check_ordinal`), with the chance
  `check interval / mean_interval_s`. It fires only if `now - last_fired_at >= min_gap_s`, no
  firing of the raid is `RUNNING`, and, for a raid that omits `despawn_after_s` (a persistent raid,
  whose creatures stay until the next reset, §4.1), no firing of the raid exists in the current
  reset epoch. A persistent raid therefore fires at most once per reset epoch, and its closed
  firing (§4.4) still blocks a second set while the first set's creatures remain; the next
  `WorldReset` removes them and ends the guard. The ordinal advances whether or not it fires, so a retry
  replays the same draw. A `fixed` raid fires at its slot once, under the same persistent-raid guard.
- **Firing.** `game_world_raid_firings`: identity (WorldId, raid key, firing ordinal), with a
  server UUIDv7 `firing_id`, `fired_at`, the `reset_epoch` it fired in, the bundle digest, the target channel set (the channels
  whose scope was assigned and admitting at fire time) and state `RUNNING`, `DONE` or
  `CANCELLED`. The check and the firing commit in one transaction.
- **Downtime.** Missed checks are never caught up: after a restart, `next_check_at` is the next
  interval from now. `min_gap_s` counts from the durable `last_fired_at`, so a restart never
  fires a raid early.

### 4.3 Announcements

Each targeted channel shows each announcement to its own players at `fired_at + delay_s`, as a
raid message in the Server Log and on screen (BOSS-WIRE-1). A channel never announces a raid it
does not run.

### 4.4 Channel run and recovery

- A channel owner claims `game_world_raid_channel_runs` (firing, ChannelId) with its scope
  ownership generation before its first wave. The claim is unique; a second claim returns the
  existing row.
- Wave creatures are channel-local actors. Their spawn source is (firing_id, wave, entry,
  index), so one generation never spawns an entry twice.
- **Restart.** A new generation that finds a run claimed by an older generation marks it `LOST`
  and does not re-run it: no duplicate boss, no repeated announcement (D52's rule, applied to raid
  creatures). A channel that activates after the firing does not join it.
- A firing is `DONE` only when a terminal run row exists for every channel in its durable target
  channel set (a channel with no row is not terminal, so an empty run table never completes a
  firing), or at the close deadline: the last wave's time plus `despawn_after_s`, or the last
  wave's time itself when `despawn_after_s` is omitted (architect ruling: creatures that persist
  do not keep a firing open or a channel claim pending; the §4.2 per-epoch guard, not the firing
  state, stops a persistent raid from firing again while they remain). At the deadline a target channel with no row is recorded `LOST` and
  the firing closes, so `min_gap_s` (§4.2) can apply. A `WorldReset` cancels running firings; raid creatures go with the overlay.

## 5. Open-world boss spawns (BOSS-1)

- A spawn whose creature has a `bosstiary` block or `reward_boss` is a **boss spawn**. Its content
  gives `respawn {min_s, max_s}` or `at_reset {chance}`.
- `game_boss_spawn_clocks`: (World, ChannelId, spawn key) with `cycle`, state `DUE`, `ALIVE` or `INACTIVE`,
  `reset_epoch`, `due_at`, and `owner_generation`: the scope ownership generation that realized the `ALIVE`
  boss. Each channel keeps its own clock (the scope matrix spawn row).
- At activation, under its new scope ownership generation, the channel realizes a boss spawn
  when:
  - its clock is `DUE` and `due_at` has passed: it sets `ALIVE` with its own generation; or
  - its clock is `ALIVE` with an `owner_generation` that has ended (not the activating
    generation): the boss actor was lost with that owner, so the channel realizes it again at full
    health and sets `owner_generation` to its own generation, by compare-and-set on the old
    generation (a failed compare realizes nothing). The lost fight state is not restored (D52).

  A later due time is kept as an ordinary owner timer. An `ALIVE` clock of the live generation is
  never realized twice.
- A committed boss death advances the clock as one more death descendant (DUR-03 A4 live-generation
  rule). For a `respawn` spawn: `cycle + 1`, `DUE`, `due_at = death + draw(min_s, max_s)`, the draw
  seeded by (clock, cycle). For an `at_reset` spawn, which has no `min_s` or `max_s`: `cycle + 1`
  and `INACTIVE`, keeping the current `reset_epoch`, so nothing realizes it again in this epoch and
  only the next epoch's draw (below) can reactivate it. If the generation ends first, the death was not committed, so the clock is still `ALIVE`
  under the ended generation and the next owner realizes the boss again (above). A restart therefore never gives a free boss and never loses a dead one's
  timer.
- `at_reset`: the first activation of a channel in a new reset epoch performs the draw once per
  (clock, epoch), by compare-and-set on the clock's old `reset_epoch`, and the transition
  overwrites whatever state the clock had (`DUE`, `ALIVE` of an ended generation, or `INACTIVE`):
  a true draw sets `DUE` with `due_at` = the reset time, a false draw sets `INACTIVE`. Both
  record the new `reset_epoch`. Activation realizes only `DUE` and an `ALIVE` clock of an ended
  generation in the same epoch, never `INACTIVE`, so a failed draw cannot spawn the boss until the
  next reset epoch's draw (an `ALIVE` clock from a previous epoch is overwritten, not recovered).

## 6. Boss rooms (BOSS-ROOM-1)

### 6.1 Topology

A boss room is an activity instance (instance baseline; D26 `instance_per_party`): `WorldId +
InstanceId`, its map from the encounter's activity template, one `InstanceRuntime`. The lever
room stays ordinary channel map. The instance uses scope kind 2 and the SCOPE-HANDOFF-1
transition of HOUSE-RUNTIME-0, extended from a house scope to an activity instance: a fresh
InstanceId per admission, the origin channel recorded per participant.

### 6.2 Admission by lever

1. A player uses the lever (GAME-INTERACTION-01 `USE`). The channel collects the characters on
   the encounter's entry tiles: 1 to the encounter's `max_participants` (at most
   `BOSSRAID0-RL-07`).
2. One transaction, the admission occurrence first: lock each participant's fence and
   `character_root` FOR UPDATE in CharacterId order; check each one's cooldown (§6.3), level and
   quest predicates (QUEST-PRED-1), and the ADR-0001 §10 gates (no combat lock, trade, pending
   item mutation); write the instance allocation row (`ADMITTING`) and each cooldown row.
3. Any failure refuses the whole group with a typed reason (`ON_COOLDOWN`, `NOT_ELIGIBLE`,
   `BUSY`, `FULL`) and writes nothing.
4. Each participant then transitions (SCOPE-HANDOFF-1). The allocation becomes `ACTIVE` with the
   first arrival. If no participant arrives within `BOSSRAID0-RL-08`, a compensation step keyed by
   the admission deletes its cooldown rows and marks it `ABANDONED`. A participant who fails to
   transfer while others arrive keeps its cooldown, as in Tibia.

### 6.3 Cooldowns

- `game_character_boss_cooldowns`: (CharacterId, boss encounter key) -> `available_at`, the
  admission that set it and its reset epoch. `available_at = now + cooldown_s` from the encounter
  content (at most `BOSSRAID0-RL-19`).
- They are reward-domain rows, not Character progression: no `CharacterRevision` advance (§12).
- **Boosted Boss:** a row set in an earlier reset epoch than the current one does not block the
  Boosted Boss of the current epoch (§11). Nothing is rewritten.

### 6.4 Lifetime, exit and recovery

- The instance runs until its encounter resets (no player inside for `reset_after_ms`) or its
  `time_limit_s` ends (at most `BOSSRAID0-RL-09`). Characters inside then leave to the
  encounter's exit anchor on their origin channel (fallback: another admitting channel of the
  World, as HOUSE-RUNTIME-0 §4.2). A death inside respawns at the home temple (D119).
- No channel change happens through an instance: exit targets the origin first (ADR-0001 §12).
- **Login** into a retired instance places the character at the exit anchor. **Crash:** the
  instance is lost; its participants are placed at the exit anchor at the next admission;
  cooldowns stay spent (Tibia never refunds them). Rewards follow R3 (§17).

### 6.5 Encounter runtime

Boss mechanics run the encounter vocabulary (format §4-§6). Its runtime is ENCOUNTER-RT-0, a
separate decision this one names. By E3, a boss whose creature is encounter-bound waits for it;
bosses without an encounter (plain spawns, most raid bosses) do not.

**Amendment (pending on acceptance of ENCOUNTER-RT-0; `reviews/OTERYN_GAME_ENCOUNTER_RT0_ENCOUNTER_RUNTIME_AND_BOSS_LEVERS_DECISION_2026-10-01.md`).** The lever
is WORLD-INTERACTION-0's `BOSS_ENTRY` child (ENCOUNTER-RT-0 §7); the boss room's Ground items are
in InstanceRuntime custody and retire by `InstanceRetire` when the instance ends (§8); the instance
allocation row gains the states `RETIRING` and `RETIRED`, and BOSS-ROOM-1's recovery finishes them.

## 7. Death identity and contribution (BOSS-REWARD-1)

- A boss death uses the DUR-03 A4 key, with the instance scope ref inside an instance. No new
  death identity is created.
- **Contribution** is tracked for a `reward_boss` actor in owner memory, as D132: per
  CharacterId, damage dealt to the boss, damage taken from the boss and healing done to other
  contributors, weighted by the formula BOSS-REWARD-1 captures (`PARITY_PENDING`).
- **Bounded top contributors (architect ruling, not D132's first-arrival overflow).** Scores
  accumulate first and are selected only at death. The accumulator holds the cumulative score of
  at most `BOSSRAID0-RL-10b` (200) distinct characters, four times the credited bound; every
  contribution by a tracked character adds to its total. Entries are ordered by cumulative score
  descending, then first-contribution tick ascending, then CharacterId ascending. A contribution
  by an untracked character enters a free entry; when all 200 are taken, it enters only if its
  score ranks above the lowest entry under that order, and the lowest entry is then evicted with
  its total discarded (a later contribution re-enters from zero). Otherwise that one contribution
  is not counted; the character fights normally, and its later contributions accumulate once it
  is admitted. Arrival order alone never admits or excludes anyone.
- **Residual (stated precisely).** With at most 200 distinct contributors in one boss life, every
  total is exact and the selection below is the true top 50. Only above 200 can a contribution be
  dropped: a character loses a contribution only while its total ranks below 200 others, so
  low-contribution taggers cannot hold credit against a higher cumulative contributor unless
  more than 200 distinct characters contribute. The accumulator stays bounded in owner memory.
- Each accumulator entry also keeps its **last qualifying damage time** (damage to the boss; for
  the Bosstiary rule, other score components do not refresh it).
- **Presence at death (owner amendment, 2026-09-30, #162; Tibia Global reward chest).** A
  contributor is credited if its character is in the game when the boss dies: a dropped
  connection keeps the character in the world, and a character that died in the fight but has not
  logged out is still credited. At the death commit, a contributor is **present** when its
  character is in the boss's World in one of two states, each with the fence that already governs
  that actor's own gameplay writes (damage, death) in that state; BOSS-REWARD-1 adds no authority:
  - **(a) Admitted GameSession, not terminal:** the reconnect-session row in `session_state`
    1 or 2 (in-grace disconnect or active), including after the character's own death while the
    session is not terminal (death screen, before logout), and a logout-blocked actor whose client
    closed and whose terminal release has not run (CHAR-POSITION0 §3.2). Fence: the composition
    rule 2 session checks in the STARTER-BACKPACK-0 server-originated variant (no CommandRef), the
    same session checks DEATH-0 §3.5's `commit_character_death` takes for the character's own death.
  - **(b) Post-grace `PRESENT_UNCONTROLLED` actor:** the old GameSession is terminal but the same
    actor is still present (FND-04B §21; DUR-02 §5.9). Fence: the one FND-04B §21 names for that
    actor, its current CharacterLease generation (DUR-02 §5.2) and current RuntimeScopeAuthority
    owner generation and placement (FND-04B §22), with the recovery fence and admission-relation
    locks, exactly as the writer of that actor's own death takes it in that state. Until an
    accepted Game writer commits a post-grace actor's own death under that fence (today DEATH-0 and
    composition rule 2 check `session_state IN (1,2)`, and post-grace authority is the candidate
    FND-DUR-POST-GRACE-TIMING-V1), such a contributor fails closed: not credited. Crediting never
    runs ahead of the death writer: a character that cannot durably die in a state cannot be
    credited in it.

  In both states the death composition claims the eligibility row (§9) and writes the Bosstiary
  receipt under that fence; §9's serialization is unchanged.
- **Absent at death (Tibia-faithful).** A character that has legally become `ABSENT` (logged out or
  released, FND-04 §4) before the death commit is credited with nothing (no reward, no Bosstiary),
  as in Tibia. Offline crediting is not a Tibia behaviour and is not built.
- At the death commit, the credited set is the top `BOSSRAID0-RL-10` (50, D109) accumulator
  entries with score > 0 by the order above that are present (previous bullets), and each score is
  fixed in the death record (§8.1). A selected contributor whose fence fails at the commit is
  skipped without replacement. Draws and MINTs of a credited character complete in its live or next
  admitted session under its own fence (§8.1).
- Non-reward bosses keep D121 corpse loot with the D112 window and the party right (PARTY-PVP-0
  §5.3). Their Bosstiary credit uses CHARM-0's 5-minute damage rule, up to 50 principals: at the death
  commit an entry whose last qualifying damage is older than 5 minutes is excluded before the top
  50 are chosen, so a fight longer than 5 minutes credits only recent damagers. Tracked by
  the same bounded accumulator (damage only): the 200-entry `BOSSRAID0-RL-10b` accumulator applies
  to every boss spawn actor, `reward_boss` or `bosstiary`, not to reward bosses alone.

## 8. Reward chest (BOSS-REWARD-1)

### 8.1 Death record and draw

- `game_boss_reward_deaths`: one row per reward-boss death key, with the credited set and scores,
  the loot table ref (key and revision) and the content generation, a **snapshot of the resolved
  loot table** taken from the active content generation at the death commit, and the Boosted Boss
  flag. Each snapshot entry holds its entry key, the exact typed item definition reference that
  DUR-03 MINT identity uses (`family`, `production_key`, `revision_ref`, as
  `OneItemTypedDefinitionRevisionV1`), its count range, its chance, and the draw-relevant flags
  the bonus rules read (capped-drop, and membership of the equipment loot set that the §10.3 slot
  bonus adds). The resolved Boosted Boss bonus chance is recorded
  with the flag. The snapshot holds at most the table's entries, each key and revision within the
  DUR-03 512 B bound (`DUR04-FIRST-PROD-KEY-BYTES`). The record is written by the death composition (ruling R3:
  committed with the death; its draws and MINTs resume after a restart, keyed so they never
  duplicate, in each credited character's own admitted session, below).
- **Bonus snapshot.** The same death composition writes, per credited character, whether the
  dead boss was in one of its boss slots and the resolved boss slot bonus multiplier (from its
  boss points), read from the character's committed state at the death commit. Later slot changes
  or boss point gains never change a recorded death's bonus.
- Each credited character gets its own draw from the boss loot table: at most
  `BOSSRAID0-RL-11` (16) entries, chances scaled by its share (`PARITY_PENDING`), plus the boss
  slot bonus from the snapshot and the Boosted Boss bonus from the recorded flag (§10.3, §11).
  Every draw, including a resumed or retried one, reads only the death record (the snapshotted
  table, never the live content definition), never live slot or Bosstiary state, and each MINT
  uses the snapshotted typed reference exactly, never a newer revision of the same key. A snapshot
  entry whose exact typed definition revision is no longer mintable is skipped and recorded in its
  step, never replaced by another revision or other content. The RNG is seeded by (death key, CharacterId).
- Each item is its own one-item DUR-03 MINT, cause `(death key, CharacterId, loot table ref,
  entry key, draw ordinal)`, in steps of `BOSSRAID0-RL-12` MINTs keyed by (death, CharacterId, step).
- **Session-fenced continuation (architect ruling).** The death record keeps, per (death key,
  CharacterId), a pending-draw marker that the step completing that character's draw clears. A
  draw or MINT step for a credited character is written only under that character's own current
  admitted session: the STARTER-BACKPACK-0 server-originated variant, with that session's
  `CurrentCharacterItemFence`, whose Character and World must equal the death record's. A
  character online in the World completes its steps in its live session. After a restart, or
  while the character is offline, its steps stay pending; its next admitted session in that World
  completes them under its own fence; a character credited as a post-grace actor (§7 (b)) has no
  live session and completes them after its post-grace recovery or next admission. Nothing is minted under an ended generation's or session's
  authority, and no step runs for a character without an admitted session. The (death, CharacterId, step) and
  item cause keys make the continuation exactly-once: a step committed before a restart is found
  by its key and never repeated. The §8.2 expiry of an item counts from its MINT.

### 8.2 Location, claim and expiry

- MINTs go to `CharacterRewardChest` (CharacterId, WorldId, reward id): a new DUR-03 location
  family BOSS-REWARD-1 adds by its own DUR-03 amendment. It has no weight or slot capacity. It
  holds at most `BOSSRAID0-RL-14` items; a draw past that goes to `CharacterInbox` (MARKET-0).
- **Claim:** a reward chest map object (every depot, and boss rooms) or the boss corpse shows the
  character's own rewards; taking one is a TRANSFER to inventory under composition rule 1 and the
  B3 capacity rules. Nobody else sees them.
- **Expiry:** an unclaimed reward expires after `BOSSRAID0-RL-13` (7 days, `PARITY_PENDING`). A
  World job retires it with a `DECAY_RETIRE` cause `RewardExpiry {reward, item}`, in steps of 100.

## 9. Anti-hopping eligibility (BOSS-REWARD-1)

- `game_character_boss_eligibility`: (CharacterId, boss key, eligibility ref) unique, for raid
  firings and boss rooms.
- **Raid firing:** the ref is the `firing_id`. A character is credited by at most one channel's
  copy of a firing's boss. A second copy's kill credits it with nothing (no reward, no
  Bosstiary).
- **Boss room:** the cooldown already bounds it; the ref is the admission.
- **Open-world boss spawn** (ruling R2, §17): no derived window ref.
  `game_character_boss_open_world_eligibility` is one singleton row per (CharacterId, boss key)
  with `next_eligible_at` and `last_credited_epoch`. The death composition inserts it if absent and locks it FOR UPDATE; it
  credits the character only if the death commit time is at or after `next_eligible_at`, and then
  sets `next_eligible_at = death commit time + BOSSRAID0-RL-18` in the same transaction. An
  `at_reset` spawn has no `min_s`: its death instead credits only if the row's
  `last_credited_epoch` is older than the current reset epoch, and sets it to the current epoch,
  so a character is credited once per boss key per reset epoch across channels. Two
  concurrent deaths on different channels serialize on that row: the second sees the advanced
  time and credits nothing (no reward, no Bosstiary). No bucket boundary exists.
- The row is written or claimed in the death composition, before the reward draw, under the
  credited character's §7 fence (state (a) or (b)). With it, a channel change gains nothing, so no extra
  channel-change gate is needed beyond ADR-0001 §10.

## 10. Bosstiary (BOSSTIARY-1)

### 10.1 Progress

- `game_character_bosstiary_kill_receipts` and `game_character_bosstiary_progress`:
  (CharacterId, boss key) -> `kill_count`, the `0019` pattern. The boss key is the Creature
  definition key of the Bosstiary entry.
- Levels (Few, Prowess, Expertise, Mastery) are derived from `kill_count` and the definition's
  thresholds, never stored. The count saturates at the Mastery threshold.
- **Write:** one death descendant per credited character (§7, §9), after XP and before quest
  transitions, on the Character sequencer (CHAR-REV-SEQ-1). The increment is 1, or 3 for the
  Boosted Boss (`BOSSRAID0-RL-15`). The receipt is memoized per (death, character).

### 10.2 Boss points

Derived, never stored: the sum over bosses of the points of every reached level (the
definition's `*_points`).

### 10.3 Boss slots

- `game_character_boss_slots`: (CharacterId, slot) -> boss key. Slot 1 is open from the start;
  slot 2 opens at 1,500 boss points (`BOSSRAID0-RL-16`).
- `game_character_boss_slot_changes`: one row per CharacterId with the reset epoch of the last
  change and the changes in that epoch; one character-wide counter across all slots, locked in
  the change transaction.
- Equip needs Prowess on that boss. Commands: equip and clear. The first change by a character in
  a reset epoch, on any slot, is free (`BOSSRAID0-RL-17`); every later one in that epoch costs the
  swap fee (owner R4a), as the one free swap per server save (§2, §16).
- **Swap fee** (owner R4a, D178). A gold sink as in Tibia, by the formula BOSSTIARY-1 captures
  before it freezes (§2). It is taken as the gold fee decision takes fees, coins first and then
  the bank (BANK-FEE-0), under a new `FeeBurnCause` variant for the slot change keyed by its
  command occurrence, with the Character receipt of the change as the one receipt (gold fee §4.3,
  as `CharmUnassign`). BOSSTIARY-1 writes the variant's amendments of DUR-03 §39.3 and the gold
  fee decision §4.4. Too little gold refuses the change as `INSUFFICIENT_FUNDS`; nothing is
  written and the free-change counter does not move.
- Effect: in the character's own reward draw of a boss equipped at the death commit (the §8.1
  snapshot), the bonus chance of one extra
  loot set, capped-drop items excluded; the curve by boss points up to 182% is captured before
  freeze (`PARITY_PENDING`).
- Each change advances `CharacterRevision` on the sequencer; the fee, if any, burns (and debits
  the bank part) in the same transaction, as the charm writer does (`item_fee_burn.rs`).

## 11. Boosted Boss (BOSSTIARY-1)

- `game_world_boosted_bosses`: (World, reset epoch) -> boss key, written by reset activation
  (ADR-0021 step 4) in the same transaction.
- The boss is drawn from the active bundle's Archfoe entries, seeded by the UTC date of the reset
  and the content generation. Worlds on the same content and date draw the same boss, as in
  Tibia's one boss for all worlds.
- Effects: Bosstiary kills count 3; the bonus loot chance in §8.1; the cooldown rule in §6.3.

## 12. Revision, fences and locks

- **CharacterRevision** advances only for Bosstiary receipts and boss slot changes, each on
  CHAR-REV-SEQ-1. The death chain is XP, Bestiary or Bosstiary, then quest, each taking the
  revision the previous one committed.
- **No advance** for cooldown, eligibility and reward chest rows: composition rule 1 covers them
  (amendment in this PR). They take rule 2's fence; a server-originated death descendant takes the
  STARTER-BACKPACK-0 variant (the admitted session's fence, no CommandRef), or for a post-grace
  present actor the §7 (b) fence. A reward draw or MINT step takes the credited character's own current session fence (§8.1), never a dead generation's.
- **Lock order:** the occurrence (admission, death step, firing); the Character roots in
  CharacterId order; cooldown and eligibility rows by (CharacterId, key); items by
  ItemInstanceId; the World rows (schedule, firing, run) by key.
- **World jobs** (raid check, firing close, reward expiry, admission compensation): HOUSE-OWN-0
  §9's pattern; the recovery fence and admission relations, no session fence; at most
  `BOSSRAID0-RL-05` rows per pass; idempotent per key.

## 13. Wire (BOSS-WIRE-1)

- **Capability `BOSS_V1`**; its number, command types and domains are reserved on #162 at
  allocation.
- **Raid message:** a text message kind for raid warnings (Server Log and screen).
- **`BOSSTIARY` domain:** counted bosses with `kill_count`; boss points; slots; the Boosted Boss.
  Names and thresholds come from the client content export (CHARM-0 answer 8); indices follow
  SPELL-D1.
- **Boss cooldowns:** the character's rows with `available_at`, as the tibia.com Boss Cooldowns
  view.
- **Reward chest:** open (own rewards grouped by death) and take (the TRANSFER of §8.2).
- **`BOSS_INTENT`:** `slot_equip {slot, boss}`, `slot_clear {slot}`. Results: `OK`,
  `NOT_PROWESS`, `SLOT_LOCKED`, `INSUFFICIENT_FUNDS`, plus the common results.

## 14. Rows (registered by the children before implementation)

| Row | Value |
|---|---|
| `BOSSRAID0-RL-01` raid check interval | 60 s |
| `BOSSRAID0-RL-02` running firings per raid | 1; a raid without `despawn_after_s`: 1 firing per reset epoch (§4.2) |
| `BOSSRAID0-RL-03` raid creatures per firing per channel | 200 (`PARITY_PENDING`; content validation refuses more) |
| `BOSSRAID0-RL-04` waves and announcements per raid | 32 waves, 16 announcements |
| `BOSSRAID0-RL-05` rows per World job pass | 100 |
| `BOSSRAID0-RL-06` boss spawn clocks per channel | 1,024 |
| `BOSSRAID0-RL-07` participants per lever admission | 15 (content sets up to it) |
| `BOSSRAID0-RL-08` admission arrival window | 30 s |
| `BOSSRAID0-RL-09` boss room time limit | content, at most 2 h |
| `BOSSRAID0-RL-10` contributors per reward boss | 50 (D109), the top contributors by score (§7) |
| `BOSSRAID0-RL-10b` contribution accumulator per boss life (reward or Bosstiary) | 200 distinct characters; lowest cumulative total evicted when full (§7) |
| `BOSSRAID0-RL-11` reward entries per character per death | 16 (D77 per character) |
| `BOSSRAID0-RL-12` reward MINTs per step | 100 |
| `BOSSRAID0-RL-13` reward expiry | 7 days (`PARITY_PENDING`) |
| `BOSSRAID0-RL-14` items in one reward chest | 1,000 (`PARITY_PENDING`) |
| `BOSSRAID0-RL-15` Bosstiary increment | 1; 3 for the Boosted Boss; saturates at Mastery |
| `BOSSRAID0-RL-16` boss slots | 2; slot 2 at 1,500 boss points |
| `BOSSRAID0-RL-17` free slot changes | 1 per character per reset epoch, all slots |
| `BOSSRAID0-RL-18` open-world eligibility interval | the spawn's `min_s` (R2 a), from the last credited death commit; an `at_reset` spawn: one credit per reset epoch (§9) |
| `BOSSRAID0-RL-19` boss cooldown | content, at most 30 days |
| Lever admission | 0 items, 0 value lines, 15 cooldown rows, 1 event |
| Reward step | 100 items, 0 value lines, 1 event |
| Bosstiary receipt | 0 items, 1 receipt, 1 event |
| Boss slot change | 1 receipt, 1 revision; with the swap fee, the gold fee shape rows (BANK-FEE-0 §4.3 for the bank part) |

## 15. Rejected options

- **Raid state only in memory.** A restart would re-fire raids and break `min_gap_s`.
- **One global boss shared by all channels.** Presence across channels needs a cross-channel
  actor; the scope matrix keeps creatures channel-local.
- **Boss rooms as channel map areas.** One arena per channel serializes groups; D26 and the
  instance baseline choose instances.
- **Boss spawns as ordinary spawns.** A restart would realize every boss again.
- **Cooldowns as Character progression.** A lever group would need one revision per participant
  in one transaction; the scope matrix puts eligibility in the reward domain.
- **Corpse loot for reward bosses.** It rewards only the top damage dealer; modern Tibia gives each
  contributor a personal reward.
- **Storing boss points.** They are derived, as Charm Points (CHARM-0 answer 2).

## 16. Owner-rule applications (Global parity, 5905825574)

Kept as in Tibia: random and fixed raids with announcements; long boss respawns; levers with
per-character cooldowns; personal contribution rewards in a reward chest with expiry; Bosstiary
tiers, thresholds and points; boss slots with one free swap per server save and a gold fee for
later swaps (R4a); a daily Boosted
Archfoe for all worlds with 3x kills, bonus loot and a cooldown reset. Declared differences: a
boss room is an instance per group (D26), not one shared arena; raid copies on several channels
(R1) and the anti-hopping eligibility (§9); bounded contributors, raid creatures and reward chest
items; lost boss rooms after a crash do not refund cooldowns (as in Tibia) and follow R3 for
rewards.

## 17. Architect rulings (owner rule 5905825574) and owner question

R1-R3 are architecture and Global-parity applications, which owner rule 5905825574 gives to the
architect; each is **ruled a), a) and b)** as recommended below. Only R4 (a D178 fee) is the
owner's.

**R1. Where does a raid happen?** (blocks RAID-1) A World has several channels, each a full copy
of the map with its own creatures. a) Every channel live at fire time runs its own copy of each
firing; one character is rewarded by one copy only (§9) (recommended: each channel has about one
Tibia world's players, D128, so supply per player stays as in Tibia); b) one random channel per
firing, announced to the whole World; c) per raid: ordinary raids on every channel, unique raid
bosses on one random channel.

**R2. Open-world bosses and channel hopping?** (blocks BOSS-REWARD-1) Each channel has its own
boss spawn clock, so one character could kill the same rare boss on every channel. a) One reward
and Bosstiary credit per (character, boss) per `BOSSRAID0-RL-18` across channels, claimed on one
locked (character, boss) row (§9) (recommended, ADR-0001 §12); b) no limit: every channel copy rewards fully.

**R3. Boss rewards after a crash?** (blocks BOSS-REWARD-1) D52 drops a death's uncommitted rewards
on a restart. a) Keep D52 for bosses too; b) a reward boss's death record (§8.1) is committed with
the death, and its draws and MINTs resume after a restart, keyed so they never duplicate
(recommended: a boss can cost a 20-hour cooldown, and reward-boss deaths are few; this is the
upgrade path DUR-03 A4 §4.4 keeps open).

**R4. Admit the boss slot swap fee as a gold sink?** (blocks the fee in BOSSTIARY-1 only) D178
needs an owner decision for each new fee source. a) Yes, from the formula BOSSTIARY-1 captures,
taken as the gold fee decision takes fees (recommended, Global parity); b) no fee: only the free
change per reset epoch.
Owner answer (2026-09-30, #162): a — yes, as in Tibia; the first change after a server save is
free (§10.3).

**Owner amendment (disconnect credit, 2026-09-30, #162).** The owner asked what happens if a player
killed the boss and their connection dropped. The round-4 ruling (only an admitted session is
credited) is replaced by §7's presence rule: as in Tibia, a character still in the game at the
death is credited, including a disconnected or dead one; only a logged-out one is not.

## 18. Decision test

- **Must decide now:** YES. The owner asked for bosses, raids and the Bosstiary now, and the scope
  matrix forbids an implicit default.
- **Minimum sufficient:** one World job and three raid tables; one clock per boss spawn; boss rooms
  on the accepted instance model and SCOPE-HANDOFF-1; one reward location; Bosstiary on the
  Bestiary pattern; derived points.
- **Superseding evidence:** captured contribution, expiry, slot and swap fee formulas;
  ENCOUNTER-RT-0; measured raid sizes above `BOSSRAID0-RL-03`.
- **Deliberately not decided:** the encounter runtime; World Changes; the Tibiadrome; the Party
  Finder; the Boosted Creature; Hazard; spectators.

## 19. Before-freeze checklist

1. **Contract amendments:** the scope matrix, the VSL resource rows decision, the encounter format
   §10, the composition decision and CHARM-0 §6, each written "pending on acceptance of
   BOSS-RAID-0". BOSS-REWARD-1 writes its own DUR-03 amendment (location family, expiry cause).
2. **Serialization:** §12's lock order; one claim per (firing, channel); one admission per lever
   use; World jobs lock before re-checking.
3. **Restart:** firings, runs, clocks, cooldowns, eligibility and rewards are durable and keyed;
   an `ALIVE` clock of an ended generation is realized again (§5); resumed draws read the §8.1
   snapshot and complete in the credited character's next admitted session under its own fence;
   raid runs and boss rooms are lost, never duplicated; R3 for rewards.
4. **Typed references:** WorldId, ChannelId, InstanceId, CharacterId, raid, encounter and boss
   keys, `firing_id`, death key, reset epoch.
5. **Wire:** §13, capability `BOSS_V1`.
6. **Split work:** one firing per transaction; at most 100 rows or MINTs per step; one
   participant per transition.
