# ARENA-1 Tibiadrome, the PvE arena and PvP arenas

- Decision: `ARENA1-TIBIADROME-AND-ARENAS-V1`
- Status: **CANDIDATE** (architect-authored). Acceptance needs exact-head validation, independent review (security, concurrency, persistence, economy, combat and protocol) and protected integration. Owner acceptance is the owner's to give; none is recorded here.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the mechanics completion plan row 106 (`OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md`, batch ARENA-1: "Tibiadrome and arenas"); the PvP arena moved here from SOCIAL-LATE-1 (`OTERYN_GAME_ACCEPT_BATCH_2026-10-15_ACCEPTANCE_DECISION.md` point 8: "SOCIAL-LATE-1: as in Tibia. No marriage. The PvP arena moves to ARENA-1."); the task record `docs/agents/tasks/ARENA-1.md`; BOSS-RAID-0 §18 and PARTY-PVP-0 (arenas and PvP zones deferred to their own decision); the gap register §20 (arenas).
- Builds on: BOSS-RAID-0 (accepted 2026-10-15: §6 boss rooms as activity instances, lever admission, cooldowns, exit and recovery; §12 lock order and World jobs); the instance baseline (`INSTANCE_SCOPE_AND_RUNTIME_OWNER_BASELINE.md`, owner-accepted); HOUSE-RUNTIME-0 §4 (SCOPE-HANDOFF-1 transition and exit fallback); ENCOUNTER-RT-0 (accepted 2026-10-14); PARTY-PVP-0 (accepted: §7 legality, §9 consequences, §10 PvP death); DEATH-0 (accepted: the death receipt); ADR-0001 §10 and §12; GAME-CHANNEL-01 §13 to §15; DUR-03 (MINT, ledgers); PREMIUM activation (accepted 2026-10-13); the content and ruleset tree; owner rule 5905825574 (Global parity).
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`
- Runtime, migration, wire and production authority: NONE until accepted and allocated. Each child needs its own #162 allocation.

## 1. Scope

Three arena kinds: the **Tibiadrome** (a group PvE wave battle with rotations, leaderboards and rewards), the **PvE arena** (the Svargrond arena, a character's sequence of duels against arena creatures) and the **PvP arena** (a lever-started match between characters). This brief decides their common instance model, entry, death outcome, rewards and durable state, and the Tibiadrome's rules. It does not decide wave scaling, drome monster stats or the reward potion list, which have no repository source (§5).

## 2. Reference grounding

- **Tibiadrome** (`docs/reference/tibia-manual/combat.md` §5.3.14):
  - Premium-only PvE practice zone; locations Ankrahmun, Kazordoon, Thais, Edron, Rathleton, entered by a demonhead-marked teleporter. Free accounts may spectate from the roof only.
  - Groups of 1 to 5 players, each level 50 or more. Run start: all participants on designated tiles, one pulls a lever, teleport, an 8-second preparation phase.
  - A battle is unlimited sequential waves of increasing difficulty with combat modifiers; each wave has a 120-second limit. Monsters per wave by group size: 1→4, 2→8, 3→11, 4→14, 5→17, from a pool of five drome-only monsters. Nine combat modifiers, one chosen at random per round, several marking 100 random tiles every 15 s.
  - The battle ends when a wave is not cleared in 120 s, the whole team dies, or the team leaves by the teleporter; if one member leaves, the others continue. Afterwards players return to the entry hall with HP and mana reset to their pre-entry values, and every participant waits 60 minutes before the next fight.
  - No death penalty (XP, skills, items, blessings). Mana and health potions, arrows, bolts and runes are not consumed; rings, amulets, food and imbuement charge time are. No skill or XP gain, no stamina drain. Soul-point spells are disabled. One group per drome instance; no PvP. Familiars and Chivalrous Challenge are usable.
  - Drome level and starting level: a run starts at the lowest participant's starting level; each beaten wave raises both by 1; after a battle the starting level drops by 5. A rotation lasts 2 weeks; at its end the drome level resets to 0 and the starting level drops by 5 (floor 0); an NPC lowers the starting level by 5 on request.
  - Rotation rewards: drome points = drome level × 10 (eligible after at least one cleared wave), spent on cosmetics at an entry-hall NPC; potions by rank (1st to 5th: 3, 6th to 10th: 2, 11th to 20th: 1), plus potions equal to 20% of the remaining participants raffled among rank 21 and below, weighted by drome level; a temporary title for the top 5 until the next rotation ends. Leaderboards for the current rotation and lifetime Highscores of cumulative drome levels.
  - Character trading keeps the levels but removes the character from the leaderboard during the auction.
  - The manual's own open question: no numbers for wave-to-wave scaling or drome monster values.
- **PvE arena.** `world.md:36`: Svargrond has a boss arena. The TibiaWiki achievements snapshot (`imports/tibiawiki/achievements/2026-09-29/tibiawiki-achievements-facts.json`) describes it as "one-on-one battles" in three tiers (Greenhorn, Scrapper, Warlord; the Barbarian Arena Quest), the Warlord tier being ten opponents in a row. Crystalserver (`imports/crystalserver/summer-update/source-tree.json`, paths only) has `data-global/lib/quests/svargrond_arena.lua`, `data-crystal/scripts/quests/arena/` (enter, pit, trophy and a dedicated `creaturescripts_player_death.lua`) and per-tier monster folders.
- **PvP arena.** Crystalserver has `data-global/scripts/actions/arena_pvp/arena_2x2.lua` and `arena_10x10.lua` (lever actions). No repository text gives their rules or death outcome.
- **Tree.** `content/encounters/tibiadrome/`, `content/encounters/arenas/` ("PvP rules remain ruleset"), `rulesets/encounters/tibiadrome/` (selection, scoring, reward) and `rulesets/pvp/arena/`, all `READY_UNPOPULATED`. The task record's owned path `content/arenas/` does not match (§5).

## 3. Rules

**A1. Every arena battle is an activity instance.** A Tibiadrome battle, a PvE arena run and a PvP arena match are each one activity instance (`WorldId + InstanceId`, scope kind 2, one `InstanceRuntime`), exactly as a BOSS-RAID-0 §6.1 boss room: a fresh InstanceId per admission, the origin channel recorded per participant, the SCOPE-HANDOFF-1 transition. The lobby, the entry hall, the teleporter and the roof are ordinary channel map. One group per instance (the manual's one group per drome instance). No channel change happens through an arena: the exit targets the origin channel first, with the HOUSE-RUNTIME-0 §4.2 fallback (ADR-0001 §12).

**A2. Entry is a lever admission, gated server-side.** BOSS-RAID-0 §6.2 applies unchanged: the channel collects the characters on the arena's entry tiles; one transaction locks each participant's fence and `character_root` in CharacterId order, checks every gate and writes the instance allocation and each cooldown row; any failure refuses the whole group (`ON_COOLDOWN`, `NOT_ELIGIBLE`, `BUSY`, `FULL`) and writes nothing; the `BOSSRAID0-RL-08` arrival window and compensation apply. Arena gates checked in that transaction:
- Tibiadrome: every participant Premium (the accepted Premium activation switch), level at least `ARENA1-RL-02`, group size 1 to `ARENA1-RL-01`, no drome cooldown;
- PvE arena: one participant; the tier's quest predicate (QUEST-PRED-1);
- PvP arena: the match definition's team sizes and level floor (content).
The client never chooses the instance, the team or the start wave.

**A3. An arena death is a defeat, not a DEATH-0 death.** An arena instance carries `death_policy`. `ARENA_DEFEAT` (Tibiadrome; PvP arena, A9): when an actor's HP would reach 0, the actor is not killed. It leaves the battle alive: no DEATH-0 receipt, no XP or skill loss, no item loss, no blessing change, no corpse or loot, no `CharacterRevision` advance, no PARTY-PVP-0 §9 consequence rows. It is moved to the arena's exit anchor on its origin channel with HP and mana restored to their pre-entry values (A5). The battle records the defeat. `STANDARD` (boss rooms) is DEATH-0 unchanged. This is a DEATH-0 §3.1 and PARTY-PVP-0 §10 amendment, pending on acceptance of this brief: the death classification first asks the scope's `death_policy`. The PvE arena's policy is an open point (§5); until it is sourced it is `STANDARD`.

**A4. Tibiadrome battle.**
- After every participant has arrived (or the arrival window closed with at least one), an 8-second preparation (`ARENA1-RL-04`), then waves from the group's start wave: the lowest participant's starting level.
- A wave spawns the monster count for the current group size (`ARENA1-RL-05`) from the drome pool, scaled by the wave number under the content curve (open point), and one combat modifier drawn per wave under the new RNG purpose `drome_modifier`, seeded by (InstanceId, wave). Modifier effects are ENCOUNTER-RT-0 actions; tile marks are at most `ARENA1-RL-09` per pulse.
- A wave is cleared when its monsters are dead within `ARENA1-RL-03`. The battle ends when a wave is not cleared in time, when no participant is left in the instance, or when every remaining participant leaves by the exit teleporter. A participant who leaves or is defeated leaves alone; the others continue.
- Battle duration is bounded: at most `ARENA1-RL-08` waves; reaching it ends the battle as if the team left.

**A5. Tibiadrome rules inside the instance** are instance ruleset values, applied by the owning mechanics (no second combat path):
- Consumption suppression: health and mana potions, arrows, bolts and runes take effect but are not retired or decremented (no DUR-03 retirement, no charge write). Rings, amulets, food and imbuement charge time are consumed normally. ITEM-USE and RUNE-USE amendment, pending on acceptance of this brief.
- No XP receipt, no skill try, no stamina drain; soul-point spells refuse with a typed reason (`NOT_ALLOWED_HERE`). Familiars and Chivalrous Challenge are allowed.
- No PvP: PARTY-PVP-0 §7.2 gains rule 0, "an arena instance without PvP refuses", pending on acceptance of this brief.
- On leaving (any end), HP and mana are set to the values recorded at the admission commit; nothing else of the vitals is restored.

**A6. Tibiadrome durable state.**
- `game_character_drome`: (CharacterId, WorldId) → `starting_level`, `lifetime_levels` (the Highscores value), `rotation_ordinal`, `drome_level` and `waves_cleared` in that rotation, `cooldown_until`.
- `game_drome_battles`: one row per admission (the BOSS-RAID-0 allocation is its occurrence): participants, start wave, waves cleared, state `RUNNING`, `ENDED` or `LOST`.
- **Per wave.** A cleared wave commits once, keyed (battle, wave): each participant still in the instance gains 1 drome level, 1 starting level and 1 lifetime level. A retry of the same key writes nothing.
- **Battle end.** One transaction, keyed by the battle: each participant's starting level drops by 5 (floor 0) and `cooldown_until = end + ARENA1-RL-06`; the battle becomes `ENDED`. **Crash** (architect ruling): the instance is lost; a recovery World job settles the battle exactly as an end (the per-wave commits already made stand; the drop and the cooldown apply), marks it `LOST`, and places participants at the exit anchor at their next admission. A server crash never preserves a starting level a normal end would have lowered.
- These are reward-domain rows like the BOSS-RAID-0 cooldowns: no `CharacterRevision` advance; BOSS-RAID-0 §12's lock order and fences apply.

**A7. Rotations and rewards, exactly once.**
- `game_drome_rotations`: (WorldId, ordinal) → `starts_at`, `ends_at = starts_at + ARENA1-RL-07`, state `OPEN`, `SETTLING`, `SETTLED`. The anchor is a World ruleset value aligned with a planned reset (its day and time are an open point).
- **Settlement** is a World job (HOUSE-OWN-0 §9, at most `BOSSRAID0-RL-05` rows per pass). At `ends_at` the rotation becomes `SETTLING` and new battles count toward the next ordinal. The job freezes the ranking from the per-character rows: drome level descending, then the time the level was reached, then CharacterId (deterministic; Tibia's tie rule is an open point). For each character with at least one cleared wave it writes one `game_drome_rotation_rewards` row keyed (WorldId, ordinal, CharacterId), unique: drome points (level × 10), potions by rank, the raffle result and the title flag. The raffle draws under the new RNG purpose `drome_raffle`, seeded by (WorldId, ordinal), weighted by drome level. Then it resets each character's drome level to 0 and lowers the starting level by 5 (floor 0).
- **Delivery.** Each reward row is consumed once: drome points credit a per-character balance with a ledger line keyed by the row (DUR-03 ledger discipline; balance at most `ARENA1-RL-10`); potions are DUR-03 MINTs into the character's inbox (INBOX-1) keyed by the row; the title is a timed grant ending with the next rotation. A retried step finds its key and writes nothing (the task's exactly-once criterion).
- **Spending** points at the entry-hall NPC is a burn with a ledger line and a cosmetic grant; it waits for an NPC currency exchange (§6).

**A8. PvE arena (Svargrond).** Each tier is an arena definition: an ordered list of opponents (content), one per pit. A run is one instance for one character; it advances pit by pit; dying or leaving ends the run. Completing a tier is a QUEST-STATE-0 step; its reward is quest content, granted once by the quest receipt. Opponents are encounter-bound creatures (ENCOUNTER-RT-0).

**A9. PvP arena.** A match is one instance for two teams, admitted by lever (A2) with the definition's team sizes (at most `ARENA1-RL-11` participants). Inside, PvP legality replaces PARTY-PVP-0 §7.2 rules 1, 4, 6 and 7 with "opposing teams only"; rule 2 (level), rule 3 (protection zone) and rule 5 (same party or guild cannot harm each other, applied within a team) hold. No aggression marks, skulls, unjustified points or revenge marks are written (§8 and §9 do not apply), so the World's `pvp_type` is untouched and the first World (Optional) can offer arena matches. `death_policy` is `ARENA_DEFEAT` (proposed; the reference outcome is open, §5). The match ends when one team has no participant left or at the definition's time limit (at most `BOSSRAID0-RL-09`). No reward in v1. This is a PARTY-PVP-0 §7.2 amendment, pending on acceptance of this brief.

**A10. Multiplicity.** Arena rewards are Character-scoped and keyed by World and rotation, so channel count changes nothing: rotation rewards are `WORLD_SCOPED_UNIQUE` per character (GAME-CHANNEL-01 §14). Drome monsters drop nothing. PvE arena rewards are quest-once. ChannelId never enters a reward key.

**A11. Spectating.** The Tibiadrome roof is ordinary channel map: a character there sees what its position shows, under ordinary visibility. It is not an observer permission and grants no view into an instance; observing an instance belongs to SPECTATE-1.

**A12. Rows (proposed; registered by the children before implementation).**

| Row | Value | Source |
|---|---|---|
| `ARENA1-RL-01` Tibiadrome group size | 1 to 5 | manual §5.3.14 |
| `ARENA1-RL-02` Tibiadrome minimum level | 50 | manual |
| `ARENA1-RL-03` wave time limit | 120 s | manual |
| `ARENA1-RL-04` preparation phase | 8 s | manual |
| `ARENA1-RL-05` monsters per wave by group size | 4, 8, 11, 14, 17 | manual |
| `ARENA1-RL-06` battle cooldown | 60 minutes | manual |
| `ARENA1-RL-07` rotation length | 14 days | manual ("every 2 weeks") |
| `ARENA1-RL-08` waves per battle | 1,000 | engineering ceiling |
| `ARENA1-RL-09` marked tiles per modifier pulse | 100 | manual |
| `ARENA1-RL-10` drome point balance | 100,000,000 | engineering ceiling |
| `ARENA1-RL-11` participants per PvP arena match | content, at most 20 | engineering ceiling; reference sizes open |
| `ARENA1-RL-12` active arena instances per node | set from measurement before activation, as `HOUSERT0-RL-04` | measurement |
| Lever admission | `BOSSRAID0-RL-07`, `BOSSRAID0-RL-08` (cited, unchanged) | BOSS-RAID-0 |
| Settlement pass | `BOSSRAID0-RL-05` (cited, unchanged) | BOSS-RAID-0 |
| Wave commit | 0 items, 0 value lines, 1 event | |
| Rotation reward delivery | at most 3 potions plus raffle potions, 1 ledger line, 1 event | |

## 4. Packets (proposed)

Allocated one by one by the control plane after acceptance; each its own PR.
1. **ARENA-CONTENT-1** (content): `content/encounters/tibiadrome/`, `content/encounters/arenas/`, `rulesets/encounters/tibiadrome/` and `rulesets/pvp/arena/` schemas with the drome monster pool, the nine modifiers, locations, PvE arena tiers and PvP arena definitions, with provenance. No runtime.
2. **ARENA-INSTANCE-1** (hard, security and concurrency review): the arena instance kind on the BOSS-ROOM-1 admission and the SCOPE-HANDOFF-1 transition, A2 gates, the A3 `death_policy` (DEATH-0 amendment), the vitals restore, exit and recovery. Needs BOSS-ROOM-1, SCOPE-HANDOFF-1, ENC-RT-1, DEATH-1.
3. **DROME-RT-1** (hard, combat review): A4 and A5 (waves, modifiers, `drome_modifier`, consumption and progression suppression, soul-point refusal, PvP rule 0). Needs ARENA-INSTANCE-1, LOOP-MONSTER-1, ITEM-USE-1c, PVP-RT-1 (rule 0 only).
4. **DROME-DATA-1** (hard, persistence and economy review): migration and writers for A6 and A7 (per-wave commits, battle end, crash settlement, rotations, settlement job, `drome_raffle`, point ledger, potion MINTs, title). Needs DROME-RT-1, D3-3, INBOX-1, PARTY-XP-1 (group membership at admission).
5. **DROME-WIRE-1** (protocol, Reg): battle HUD (wave, timer, modifier), leaderboards, Highscores and the point balance; its capability is reserved on #162 at allocation.
6. **ARENA-PVE-1** (impl, combat review): A8. Needs ARENA-INSTANCE-1, QUEST-STATE-1.
7. **ARENA-PVP-1** (hard, combat and PvP review): A9 (PARTY-PVP-0 §7.2 amendment). Needs ARENA-INSTANCE-1, PVP-RT-1.

Order: ARENA-CONTENT-1, ARENA-INSTANCE-1, then DROME-RT-1, ARENA-PVE-1 and ARENA-PVP-1 in parallel, then DROME-DATA-1, then DROME-WIRE-1.

## 5. Open points (not decided here)

- Wave-to-wave difficulty scaling and drome monster values (the manual's own open question); the reward potion list and effects; modifier periods other than those the manual gives.
- The rotation anchor (day and time), Tibia's ranking tie rule and the rounding of the 20% raffle count; until sourced, the content marks them `PARITY_PENDING`.
- The death outcome of the Svargrond arena (Crystalserver has a dedicated death script whose content is not in the repository) and of PvP arenas; A3 and A9 state the defaults.
- PvP arena team sizes and level floor (`arena_2x2` and `arena_10x10` may name team sizes or arena dimensions; not inferred).
- The task record's owned path `content/arenas/` should become `content/encounters/arenas/` and `content/encounters/tibiadrome/` (the content tree); the record is not edited here.

## 6. Split out, not decided here

- **Drome point spending** (cosmetics at the entry-hall NPC): needs the NPC exchange contract that CRAFT-1 §6 also splits out, plus outfit and mount ownership. Proposed DROME-SHOP-1, blocked on that decision.
- **Character trading and the leaderboard**: removal during an auction and the forfeit rule belong with the Character Bazaar (a Platform boundary).
- **Tournaments, ratings and matchmaking queues** (gap register §20): no reference requirement in this scope.
- **Observing an arena instance**: SPECTATE-1 (A11).

## 7. Handback

```yaml
result: CANDIDATE
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_ARENA1_DECISION_2026-10-15.md
resource_values_changed: false
production_authority_changed: false
implementation_may_resume: false
required_fresh_allocation: true
required_independent_review: "security, concurrency, persistence, economy, combat, protocol"
amends_pending_acceptance: "DEATH-0 §3.1 and PARTY-PVP-0 §7.2, §10 (A3, A5, A9); ITEM-USE and RUNE-USE consumption (A5)"
```
