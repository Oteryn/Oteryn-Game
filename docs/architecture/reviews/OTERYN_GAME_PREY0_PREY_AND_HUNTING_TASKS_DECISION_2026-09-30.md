# PREY-0 Prey and Hunting Tasks

- Decision: `PREY0-PREY-AND-HUNTING-TASKS-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence,
  economy, combat and protocol) and protected integration. R1-R4 (§14) are architect rulings;
  owner questions Y1-Y3 (§15) are open and block only the parts they name.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the owner direction of 2026-09-30 (build Prey and Hunting Tasks now, full Tibia Global
  parity). It is the owning semantic gate DUR-02 names for "Hunting Task/Prey state" (persistence
  baseline §13) and fills `rulesets/progression/prey/` of the content tree.
- Builds on: CHARM-0 (Bestiary facts, the 5-minute credit window, answers 2, 8-11) and CHARM-2
  (`0019`, `bestiary_progress.rs`); QUEST-STATE-0 §5.2 (CHAR-REV-SEQ-1, the death chain); the
  composition decision rules 1-6; the gold fee decision (D174-D178, §4.3 and §4.4) and BANK-FEE-0;
  D208; PREMIUM-ACTIVATION-V1 §4.1 and PREMIUM-DELIVERY-0; HOUSE-OWN-0 H2a; the Store catalogue
  owner decision and gap register §32; D47 and D49; D118 (XP and stamina); D109; D3 and the loot
  plan (`combat/loot_plan.rs`); GAME-ABILITY-01 (typed contribution stages); SIM-DETERMINISM-01
  §10-§11 (RNG purposes); ADR-0021 (reset epoch); BOSS-RAID-0 (branch `claude/arch-boss-raid-0`,
  the Bosstiary in the death chain); A13 and CHAR-BUILD-1a (#1393); owner rule 5905825574
- Amends, each pending on acceptance of PREY-0, in this PR: QUEST-STATE-0 §5.2 (the death chain
  gains task kill credit); PREMIUM-ACTIVATION-V1 §4.1 (Prey slot 2 is a Premium surface); the
  content tree's Prey section (the Task Board ruleset).
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| PREY-CONTENT-1 | content lane | `rulesets/progression/prey/` and `rulesets/progression/task-board/`: bonus grade table, list bands, the `preyable` fact, task tiers and tables, shop catalogue (§3, §8) | this decision |
| PREY-1 | hard, persistence review | prey slot state, the resource balance, prey commands and receipts, list and bonus draws, hunting-time checkpoints, expiry with auto reroll and lock (§4-§6) | CHAR-REV-SEQ-1; PREY-CONTENT-1 |
| PREY-FEE-1 | hard, persistence and economy review | the gold list reroll as `FeeBurnCause::PreyListReroll` (§5.3) | PREY-1; GOLD-FEE-1b; owner answer Y1 |
| PREY-EFFECT-1 | combat lane, combat review | damage and reduction contributions, the XP multiplier, the extra loot roll, the hunting clock (§7) | PREY-1; the XP lane (D118); SPELL-TARGET-1 (ATTACK-0) |
| TASKBOARD-1 | hard, persistence review | Bounty and Weekly kill tasks, task kill receipts in the death chain, token and point balances, the weekly settlement (§8, §9) | PREY-1; CHAR-REV-SEQ-1; owner answer Y3 |
| TASKBOARD-DELIVERY-1 | hard, persistence and economy review | Weekly delivery tasks: the `TaskDeliveryCause` BURN and its DUR-03 amendment (§8.3) | TASKBOARD-1; ITEM-USE-1 (item selection) |
| TASKSHOP-1 | hard, persistence and economy review | the Hunting Task Shop: point debit with an item MINT, a D47 unlock or promotion points (§8.4) | TASKBOARD-1; owner answer Y3 |
| PREY-WIRE-1 | impl, protocol review | capability `PREY_V1`: the Prey and Task Board domains and intents (§11) | PREY-1; TASKBOARD-1 |

Later, each with its own decision: the Bounty Talisman upgrades and effects, the Soulpit (Soulseal
use), Daily Rewards (a wildcard source in Global), the Store delivery of wildcards and permanent
unlocks (gap register §32, Y2), party sharing of loot boosts.

## 1. Question

How does a character hold three Prey slots and a Task Board, with draws, bonuses, hunting time,
wildcards, fees and task rewards that are durable, replay-safe, the same on every channel, and
admitted as value sources only by the owner?

## 2. Facts

**PROVEN**

- DUR-02 persistence baseline §13: Hunting Task and Prey state use dedicated typed relations; a
  generic JSON, KV or blob escape hatch is forbidden. GAME-CHAR-01 Stage B: permanent Prey and
  Hunting Task slots and Hunting Task Points are character-specific product state.
- CHARM-0: Bestiary facts sit on Creature definitions (`authoring.profile.bestiary`, 686 records,
  difficulty in stars); kill credit is the 5-minute damage window (answer 6); wire names come from
  the client content export and indices follow SPELL-D1 (answers 8 and 11). `0019` keeps per
  (Character, race) kill receipts and advances `CharacterRevision` (`bestiary_progress.rs`).
- QUEST-STATE-0 §5.2: every revision-advancing write goes through one sequencer per Character
  (CHAR-REV-SEQ-1); a creature death chain holds the slot for XP, then Bestiary, then quest.
  BOSS-RAID-0 §12 adds Bosstiary beside Bestiary.
- The gold fee decision: one transaction carries the Character change and the BURN lines, one
  receipt, one `CharacterRevision` advance (D177); a closed `FeeBurnCause` variant per fee source,
  each admitted by the owner (D178); coins first, then the bank (BANK-FEE-0). D208: a new value
  source needs its own owner decision.
- Store catalogue owner decision: the Tibia Coin balance, payment and purchase ledger are
  Platform's; Store delivery ownership, entitlement lifecycle and refunds are open (gap register
  §32). D47: Store purchases belong to the account; a purchased item is claimable on a compatible
  world of the profile family (D49).
- PREMIUM-ACTIVATION-V1 §4.1 and PREMIUM-DELIVERY-0: Premium is Platform's entitlement; Game reads
  `premium_current(account)`; losing Premium never blocks login. HOUSE-OWN-0 H2a: Premium is not
  required until the Premium consumer contract is delivered.
- The loot plan (`loot_plan.rs`): `IndependentBernoulliPpm`, draws derived from the death key.
  SIM-DETERMINISM-01 §11: each random decision has a stable, versioned purpose identity.
- No Prey, Task Board, stamina or wildcard code, table or message exists on `main`.

**CIPSOFT_OFFICIAL** (the Tibia manual, capture 2026-09-28)

- `combat.md` §5.3.9 Prey Dialog: needs the character to have left Rookgaard or Newhaven; 9
  creatures per slot; one free list reroll per 20 hours, more for gold scaling with level; 5
  wildcards pick any eligible creature; bonus types damage boost, damage reduction, bonus XP and
  improved loot; a wildcard bonus reroll never lowers the value unless at the maximum; **2 hours
  of hunting time, ticking only while actively hunting**; a new list pick or bonus roll resets it
  to 2 h; Automatic Bonus Reroll costs 1 wildcard per trigger and Lock Prey 5, each switching off
  when wildcards run out; slot 1 for everyone, slot 2 free with Premium or bought permanently,
  slot 3 bought permanently in the Store.
- `combat.md` §5.3.10 Task Board: Bounty Tasks (1 of 3 offers; 4 tiers Beginner to Master; a
  preferred list; 1 free reroll token a day, at most 10 held; silver and gold bounties; rewards XP,
  tokens and Bounty Points; the Bounty Talisman at 5,000 gp); Weekly Tasks (6 kill and 6 delivery
  tasks a week, +3/+3 with the Store expansion; rewards every Monday after server save: XP, Hunting
  Task Points 25 per kill task and 75 per delivery task, 1 Soulseal each; multipliers x2, x3, x5, x8
  at 4, 8, 12, 16 tasks); the Hunting Task Shop (outfits, mounts, items, trophies, decorations,
  promotion points).
- `products.md`: Prey Wildcard (at most 52 owned), Permanent Prey Slot and Permanent Weekly Task
  Expansion are Store products.

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b5`, read-only)

- `src/io/ioprey.cpp:41-54`: a bonus reroll draws the grade uniformly from [grade + 1, 10] (10 at
  9 or more; at 10 only the type changes, `:29-35`); values damage `2g+5`, reduction `2g+10`, XP and
  loot `3g+10` percent.
- `ioprey.cpp:57-128`: the list draws 9 creatures from the Bestiary map by star bands that depend
  on level (`<100`: 3/3/2/1 of 1-4 stars; `<300`: 1/3/3/2; `<500`: 1/2/3/3; else 1/1/3/4), without
  XP-less, non-preyable or prey-exclusive creatures, and without races already listed or selected.
- `ioprey.cpp:254-296` and `data/events/scripts/player.lua:103-133`: prey time falls with the
  stamina clock (60 s per hunting minute); on expiry auto reroll or lock spends wildcards, else the
  bonus is erased and a new list drawn. `config.lua.dist:144-150`: 200 gp per level per paid list
  reroll, 5 wildcards to select or lock, 1 to reroll the bonus.
- `combat.cpp:887-908`: damage boost and reduction scale primary and secondary damage;
  `player.lua:568-572`: the XP bonus multiplies creature XP;
  `ondroploot_prey.lua`: improved loot is a chance of one extra loot roll into the corpse.
- Canary has the older 12.x Hunting Task slots (`ioprey.cpp:130-250`), not the Task Board.

## 3. Content (PREY-CONTENT-1)

- `rulesets/progression/prey/`: slots (3), list size (9), the level-band table, bonus types and the
  grade table (grades 1-10, value per type and grade), hunting time (7,200 s), free reroll interval
  (20 h), wildcard costs (1, 5, 1, 5), the paid reroll price rule (Y1). All values not in the
  manual are captured against TibiaWiki before PREY-1 freezes (`PARITY_PENDING`).
- **Preyable pool:** a Creature is preyable when it has a Bestiary block, positive XP, and no
  `prey: excluded` fact (new, boss and event exceptions). The pool is the active content
  generation's preyable Creatures in key order. Bosses have no Bestiary block, so they are out.
- **Prey loot budget (ruling R4):** a preyable Creature's loot table has at most `PREY0-RL-16`
  entries (8), so it plans in at most 16 RNG draws: half of `COMBAT01-LOOT-PLAN-ENTRIES` (16) and
  `COMBAT01-LOOT-RNG-DRAWS` (32). The base roll and the improved-loot roll of §7 together fit the
  existing D77 ceilings (16 entries, 16 items, 32 draws, `COMBAT01-LOOT-PLAN-BYTES`) with no
  amendment. The content compiler checks it for every Creature that would be preyable and **fails
  the compile closed** (`PREY_LOOT_TABLE_TOO_LARGE`, naming the Creature); it never drops the
  Creature from the pool silently. Content that needs a larger table marks it `prey: excluded`.
- `rulesets/progression/task-board/`: Bounty tiers (Bestiary difficulty per tier), kill counts,
  rewards and silver/gold chances; Weekly task tables; the shop catalogue. Each is
  `PARITY_PENDING` until captured.

## 4. Prey state (PREY-1)

### 4.1 Tables

- `game_character_prey_slots`: (CharacterId, slot 1-3) -> `state` (`LOCKED`, `SELECTION`,
  `ACTIVE`), the drawn list (up to 9 Creature keys), selected Creature key, bonus type, grade,
  `hunting_s_left`, option (`NONE`, `AUTO_REROLL`, `LOCK`), `free_reroll_at`, `slot_epoch`
  (raised by each list draw and expiry). Keys, never wire indices (CHARM-0 answer 11).
- `game_character_prey_resources`: CharacterId -> typed columns `prey_wildcards`,
  `bounty_reroll_tokens`, `bounty_points`, `hunting_task_points`, `soulseals`, each non-negative
  with its cap (§12). Typed columns, not a KV table (DUR-02 §13).
- `game_character_prey_unlocks`: (CharacterId, unlock kind, target) for `PREY_SLOT_PERMANENT`
  with target slot 2 or 3, and `WEEKLY_TASK_EXPANSION` with target `NONE`, with the source (Y2).
  Each (kind, target) is a separate write-once fact, so one purchase unlocks exactly one slot.
  Empty until the Store delivery contract exists.
- `game_character_prey_receipts`: one per revision for Prey and Task Board writes, keyed by the
  occurrence, binding the command, its drawn outcome and each balance change with its cause (§10).

### 4.2 Scope and revision

- Prey and Task Board state is **Character** state: the same on every channel and World admission
  of that Character. The owning channel runtime keeps a copy for effects and ticks.
- Every write advances `CharacterRevision` once on CHAR-REV-SEQ-1, with its receipt, as the charm
  writer does. PREY-1 and TASKBOARD-1 each admit their receipt kind by replacing the `0020`
  consistency guard in their own migration (as QUEST-STATE-1 does).
- Expected revision: a player command's binding excludes the revision, so a mismatch reloads the
  cursor and retries once (the quest and Bestiary rule, QUEST-STATE-0 §5.2).
- Fence: the composition rule 2 fence for commands; the STARTER-BACKPACK-0 variant (the admitted
  session's fence, no CommandRef) for server-originated writes (initialization, checkpoints,
  expiry, settlement).

## 5. Prey commands (PREY-1)

### 5.1 Slots and unlock

- The Prey dialog opens only for a character with a vocation (it has left the starter island,
  A13; `PARITY_PENDING` for Dawnport specifics). Else `NOT_ELIGIBLE`.
- Slot 1: every eligible character. Slot 2: Premium current at use, or `PREY_SLOT_PERMANENT
  {slot 2}`; until PREM-1 is live, open to all (ruling R2). Slot 3: `PREY_SLOT_PERMANENT {slot 3}`
  only (Y2).
- **Permanent slot claim (architect ruling, Global order):** a `PreyStoreClaim` for a permanent
  slot binds its target slot in the claim and the receipt: the lowest of slots 2 and 3 that has no
  `PREY_SLOT_PERMANENT` fact, chosen at claim time and stored. A claim when both facts exist is
  refused `ALREADY_UNLOCKED` and writes nothing; the delivery stays unclaimed for Platform's refund
  path (gap register §32). A replay returns the stored target.
- A slot that loses its unlock (Premium lapses) keeps its durable row, applies no bonus, spends no
  time and refuses commands with `SLOT_LOCKED` until the unlock returns (`PARITY_PENDING`).

### 5.2 Commands

Each is one sequencer write, keyed by its CommandRef. A refusal writes nothing. Every command
carries a `slot` (1 to `PREY0-RL-01`); the server validates it before any other check (out of
range: `INVALID_SLOT`; not unlocked for this character now: `SLOT_LOCKED`; no row yet: §6.5) and
binds it in the command binding and the receipt, so a replay under the same CommandRef with a
different slot is a conflict, never a second write. Client focus never supplies the slot.

| Command | Needs | Effect |
|---|---|---|
| `list_reroll {slot}` | `free_reroll_at` passed, else the fee (§5.3) | draws a new list (§6.1); a free use sets `free_reroll_at` = now + 20 h; an active bonus type and grade are kept for the next pick, the creature and time cleared |
| `select {slot, index}` | state `SELECTION`; index in that slot's list | selected creature; if no bonus yet, draws type and grade (§6.2); time = 7,200 s; `ACTIVE` |
| `select_any {slot, creature}` | 5 wildcards; creature preyable | as `select`, from the whole pool |
| `bonus_reroll {slot}` | `ACTIVE`; 1 wildcard | new type and grade (§6.2); time = 7,200 s |
| `set_option {slot, option}` | 1 wildcard held for `AUTO_REROLL`, 5 for `LOCK` | stores the option; nothing is spent until expiry |

- A creature selected or listed in another slot of the character cannot be picked (`DUPLICATE`).
- Wildcards are debited in the same receipt. Too few: `NOT_ENOUGH_WILDCARDS`.

### 5.3 The gold list reroll (PREY-FEE-1, Y1)

- The price is `level x 200` gp (Canary; the manual says "scales with level";
  `PARITY_PENDING`), frozen in the receipt binding.
- Paid by the gold fee path: one transaction carries the prey change, the BURN lines of coins then
  bank, and one receipt, with a new closed variant `FeeBurnCause::PreyListReroll {slot,
  occurrence}` (D177, D178, BANK-FEE-0). PREY-FEE-1 amends the gold fee decision §4.4 and DUR-03
  §39.3 for this variant only.
- Until Y1 is answered `a`, a list reroll before `free_reroll_at` is refused
  `FREE_REROLL_NOT_READY`.

## 6. Draws and time (PREY-1)

### 6.1 List draw

- Purpose `oteryn.prey.list.v1`, seeded by the command occurrence, for an expiry redraw by
  (CharacterId, slot, `slot_epoch`), or for the first list by (CharacterId, slot, `INIT`) (§6.5). The draw takes 9 distinct Creatures from the pool by the level
  bands of §3, excluding those listed or selected in the character's other slots.
- A band with too few candidates is filled from the next band down, then up (`PARITY_PENDING`); a
  pool under 27 Creatures disables Prey on that content generation (Canary refuses under 36).
- The list is stored in the receipt and the slot row. A replay returns the stored list; a content
  change never redraws a stored list.

### 6.2 Bonus draw

- Purpose `oteryn.prey.bonus.v1`, seeded as §6.1 plus the command kind. The type is uniform over
  the four types. The first grade is uniform in [2, 10] (Canary; `PARITY_PENDING`). A reroll draws
  uniform in [grade + 1, 10], 10 from grade 9 up; at 10 it draws a different type and keeps 10.

### 6.3 Hunting time

- `hunting_s_left` falls only on the **hunting clock**: one tick per 60 s of the session in which
  the character dealt damage to, or took damage from, a creature (not a player). It is the stamina
  signal of the XP lane (D118); PREY-EFFECT-1 owns it until stamina lands (`PARITY_PENDING`).
- A tick lowers every `ACTIVE` unlocked slot at once. Offline time never counts.
- **Checkpoint:** consumed time is written in one receipt per character at most every
  `PREY0-RL-07` (300 s of hunting), and always before logout, channel transfer, any prey command
  and expiry. A crash loses at most that much consumption, in the player's favour (ruling R3).
- **Checkpoint occurrence:** (GameSessionId, `checkpoint_seq`). GameSessionId is the admitted
  session's durable, never-reused identity (the STARTER-BACKPACK-0 fence); `checkpoint_seq` starts
  at 1 in each session and rises by 1 per checkpoint, and is stored in the receipt. Recovery reads
  the highest stored `checkpoint_seq` of that session before the next checkpoint, so a restarted
  runtime never reissues a used occurrence; a new session starts a new sequence under a new
  GameSessionId. A replay of a stored occurrence returns its receipt; the same occurrence with a
  different binding is a conflict and writes nothing.

### 6.4 Expiry

- When `hunting_s_left` reaches 0, one server-originated write keyed by (CharacterId, slot,
  `slot_epoch`):
  - `AUTO_REROLL` with at least 1 wildcard: debit 1, bonus draw (§6.2), time 7,200 s;
  - `LOCK` with at least 5 wildcards: debit 5, same creature and bonus, time 7,200 s;
  - otherwise (or too few wildcards): option cleared, bonus and creature erased, a new list drawn
    free (Canary), `SELECTION`, and a status message.

### 6.5 First-list initialization

- A slot has no row until it is initialized. At the first admission of the Character at which it
  is Prey-eligible (§5.1) and a slot is unlocked with no row, one server-originated write per such
  slot, keyed by (CharacterId, slot, `INIT`), creates the row in `SELECTION` with a list drawn by
  §6.1, `slot_epoch` 1, no bonus, option `NONE` and `free_reroll_at` = now. A slot unlocked later
  (Premium, a permanent claim) is initialized at the next admission the same way.
- The occurrence is idempotent: a replay returns the stored row and list; an existing row is never
  re-initialized, so a locked-then-unlocked slot keeps its row (§5.1). There is no migration
  backfill and no first-read side effect; until initialized, the `PREY` snapshot shows the slot as
  `LOCKED` or pending and every command on it is refused `SLOT_LOCKED`.

## 7. Effects (PREY-EFFECT-1)

- The runtime copy gives, per (character, Creature key), the active bonus of an unlocked `ACTIVE`
  slot with time left. It is read at the event and frozen into it.
- **Damage boost:** a typed pre-commit contribution (GAME-ABILITY-01 §10) on damage a character
  deals to that Creature, primary and secondary, rounded up. **Damage reduction:** the same on
  damage that Creature deals to the character. Order beside charms and resistances is fixed by
  PREY-EFFECT-1 with the combat lane (`PARITY_PENDING`).
- **Bonus XP:** a multiplier on that character's creature XP share, applied in the XP award before
  stamina, with the grade frozen in the XP binding. EligibleRawXP (the multichannel checkpoint)
  stays the pre-bonus value.
- **Improved loot:** for the loot owner (D121) with the bonus on that Creature, a chance equal
  to the bonus percentage of one extra roll of the Creature's loot table into the same corpse,
  purpose `oteryn.prey.loot.v1` from the death key (ruling R4). The §3 prey loot budget makes the
  two-roll worst case (16 entries, 32 draws) fit the existing D77 ceilings; the planner still
  checks them and refuses the whole plan closed on a breach, which is unreachable for compiled
  content.

## 8. Task Board (TASKBOARD-1, TASKBOARD-DELIVERY-1, TASKSHOP-1)

Global's current Task Board replaces the 12.x Hunting Task slots (ruling R1).

### 8.1 Bounty Tasks

- `game_character_bounty_tasks`: CharacterId -> tier, the 3 offers (Creature key, kills, reward,
  rarity), the active task (at most one) with `kills_done`, and the preferred list (at most
  `PREY0-RL-13` Creature keys).
- Offers are drawn with purpose `oteryn.taskboard.bounty.v1` from Bestiary Creatures of the tier's
  difficulties, weighted to the preferred list (`PARITY_PENDING`), seeded by the occurrence.
- Commands: `set_tier` (applies at the next pick), `pick {offer}`, `reroll_offers` (1 token),
  `set_preferred`, `claim`. One free token per reset epoch day, at most 10 held.
- `claim` on completion credits Bounty Points and tokens in its receipt; its XP is an XP award
  descendant with the XP source `TaskReward`, keyed by the claim.

### 8.2 Weekly kill tasks

- `game_character_weekly_tasks`: (CharacterId, week) -> 6 kill and 6 delivery tasks (+3/+3 with
  `WEEKLY_TASK_EXPANSION`), drawn lazily at the first Task Board read after the weekly reset with
  purpose `oteryn.taskboard.weekly.v1` seeded by (CharacterId, week).
- The week starts at the first World reset epoch (ADR-0021) on or after Monday 00:00 in the World's
  reset time zone.

### 8.3 Delivery tasks (TASKBOARD-DELIVERY-1)

- Delivering N of an item is one transaction: the task change and receipt, and BURN lines of the
  delivered stacks under a closed cause `TaskDeliveryCause {week, task, occurrence}`, as the gold
  fee composition (D177): one TransactionId, one revision advance. TASKBOARD-DELIVERY-1 amends
  DUR-03 §15 and §39.3 for this shape only.

### 8.4 Weekly settlement and the shop

- At the first admission after a weekly reset, one server-originated write keyed by (CharacterId,
  week) settles the finished week: Hunting Task Points (25 per kill task, 75 per delivery task,
  times the multiplier of §2), 1 Soulseal per task, and an XP award descendant (`TaskReward`).
- **Shop (TASKSHOP-1):** `buy {offer}` debits Hunting Task Points in the Character receipt and
  delivers in the same transaction: an item as a DUR-03 MINT (cause `TaskShopPurchase {offer,
  occurrence}`) into the main backpack or `CharacterInbox`; an outfit or mount as a D47 account
  unlock (write-once, gameplay-earned); promotion points fail closed until the Wheel of Destiny
  child. The shop catalogue is content.
- Soulseals accrue now; spending them waits for the Soulpit decision.

## 9. The death chain (TASKBOARD-1)

- The order becomes: **XP, then Bestiary or Bosstiary, then task kill credit, then quest**, each
  taking the revision the previous one committed, in one sequencer slot (QUEST-STATE-0 §5.2, as
  amended).
- Task kill credit is one receipt per (death, character), written only when the Creature matches
  the character's active Bounty task or an open weekly kill task; else nothing is written.
- Credit follows the Bestiary rule: damage within 5 minutes before the death (CHARM-0 answer 6),
  up to 50 principals (D109). It never blocks or rolls back XP, loot or Bestiary.
- Prey writes nothing in the death chain: its XP and loot effects are frozen into those writes.

## 10. Durable writes

| Write | Class | Cause | Revision |
|---|---|---|---|
| Prey command, wildcard debit | Character receipt | `PreyCommand {occurrence}` | +1 |
| Gold list reroll | receipt + BURN (+ change MINT) | `FeeBurnCause::PreyListReroll` (Y1) | +1 |
| Hunting-time checkpoint | Character receipt | `PreyTimeCheckpoint {GameSessionId, checkpoint_seq}` | +1 |
| Expiry, auto reroll, lock | Character receipt | `PreyExpiry {slot, slot_epoch}` | +1 |
| First-list initialization | Character receipt | `PreyInit {slot}` (§6.5) | +1 |
| Wildcard credit, permanent unlock | Character receipt | `PreyStoreClaim {delivery}`, binding the target slot (Y2) | +1 |
| Bounty and weekly commands | Character receipt | `TaskBoardCommand {occurrence}` | +1 |
| Task kill credit | Character receipt | `TaskKill {death key}` | +1 |
| Delivery | receipt + BURN | `TaskDeliveryCause` (Y3) | +1 |
| Settlement, bounty XP | receipt, then XP award | `TaskSettlement {week}`, `TaskReward` | +1 each |
| Shop purchase | receipt + MINT or D47 unlock | `TaskShopPurchase` (Y3) | +1 |

- No gold is minted anywhere. The only gold change is the Y1 fee.
- Test grants of resources exist only outside production (`PreyTestGrant`), refused by a
  production World's ruleset, as the D69 operator grants are for Premium.
- Lock order: the occurrence; `character_root` with the expected revision; the prey and task rows;
  then items by ItemInstanceId and the bank row, as the gold fee path.

## 11. Wire (PREY-WIRE-1)

- **Capability `PREY_V1`**; its number, state domains and command types are reserved on #162 at
  allocation.
- **`PREY` domain:** per slot state, list (Creature indices, SPELL-D1), bonus type and grade,
  time left, option, `free_reroll_at`, the paid reroll price; the resource balances. **`TASK_BOARD`
  domain:** bounty offers and task, weekly tasks and progress, shop offers. Names come from the
  client content export (CHARM-0 answer 8).
- **`PREY_INTENT`** (§5.2, each command with its `slot`) and **`TASK_BOARD_INTENT`** (§8).
  Results: `OK`, `NOT_ELIGIBLE`, `INVALID_SLOT`, `SLOT_LOCKED`, `DUPLICATE`, `NOT_ENOUGH_WILDCARDS`, `FREE_REROLL_NOT_READY`,
  `INSUFFICIENT_FUNDS`, `NOT_ENOUGH_POINTS`, `NO_ROOM`, plus the common results.

## 12. Rows (registered by each child before implementation)

| Row | Value |
|---|---|
| `PREY0-RL-01` prey slots | 3 |
| `PREY0-RL-02` creatures per list | 9 |
| `PREY0-RL-03` bonus hunting time | 7,200 s |
| `PREY0-RL-04` free list reroll interval | 20 h per slot |
| `PREY0-RL-05` wildcards held | 52 (`products.md`) |
| `PREY0-RL-06` wildcard costs | bonus reroll 1, select any 5, auto reroll 1, lock 5 |
| `PREY0-RL-07` hunting-time checkpoint | every 300 s of hunting at most, and at each boundary |
| `PREY0-RL-08` grades | 1-10 |
| `PREY0-RL-09` prey and task commands per character | 1 in flight (the sequencer) |
| `PREY0-RL-10` bounty offers; tokens | 3; 1 per day, 10 held |
| `PREY0-RL-11` weekly tasks | 6 kill and 6 delivery; +3/+3 with the expansion |
| `PREY0-RL-12` Hunting Task Points per task; multipliers | 25, 75; x2, x3, x5, x8 at 4, 8, 12, 16 |
| `PREY0-RL-13` preferred list | 32 Creatures (`PARITY_PENDING`) |
| `PREY0-RL-14` balances | u32 each, 0 floor; points 10,000,000 cap (`PARITY_PENDING`) |
| `PREY0-RL-15` delivery lines | at most 20 input stacks per delivery |
| `PREY0-RL-16` preyable Creature loot table | at most 8 entries, 16 RNG draws per roll (half of D77's 16 and 32) |
| Prey command | 0 items, 1 receipt, 1 event |
| Gold list reroll | at most 20 inputs and 2 change outputs (D178) |
| Shop purchase | at most 1 MINT item, 1 receipt, 1 event |

## 13. Rejected options

- **Prey time on the wall clock.** Global counts hunting time only.
- **A revision per hunting minute.** It multiplies Character writes by the number of hunters; the
  checkpoint bounds both the write rate and the crash loss.
- **Wildcards as items.** Global wildcards are an untradable character balance.
- **Wildcards on the account.** Global binds them to the character; D47 is kept by the account
  owning the purchase and the claim choosing the character (Y2).
- **Platform writing the balance.** Platform is commercial authority, not Game truth; Game credits
  a Platform delivery by an idempotent claim.
- **Derived Hunting Task Points.** They are spent in a shop; a balance with receipts is simpler.
- **The 12.x Hunting Task slots (Canary).** Global replaced them with the Task Board (R1).

## 14. Architect rulings (owner rule 5905825574)

R1-R4 are Global-parity and architecture applications, each **ruled a)**.

**R1. Which task system?** a) The current Global Task Board: Bounty and Weekly Tasks and the
Hunting Task Shop (recommended: Global runs it now); b) Canary's 12.x Hunting Task slots; c) both.

**R2. Prey slot 2 before Premium is delivered?** a) Open to every eligible character until PREM-1
is live, then Premium or the permanent unlock is required, and a slot in use then follows §5.1
(recommended: the owner's H2a for houses); b) locked until PREM-1.

**R3. Crash loss of hunting time.** a) Checkpoint at most every 300 s; a crash returns up to that
much bonus time (recommended: bounded, in the player's favour, as Tibia's rollback to the last
save); b) a write per hunting minute.

**R4. Improved loot.** a) One extra loot roll at the bonus percentage chance, for the loot owner
only, within D77 by a compile-time budget of 8 entries per preyable table (recommended: the only
sourced reading, no ceiling amendment; party loot boosts wait for their own decision); b) scale
every entry's chance.

## 15. Owner questions (value and commerce)

**Y1. Admit the Prey list reroll as a gold sink?** (blocks PREY-FEE-1 only; D178) a) Yes:
`FeeBurnCause::PreyListReroll`, `level x 200` gp until the Global price is captured, coins then
bank (recommended, Global parity); b) no fee: one free reroll per slot every 20 hours only.

**Y2. How are Prey Wildcards and the permanent unlocks (slot 3, the Weekly Task Expansion)
credited?** (blocks their sources only) Tibia Coins, the Store and delivery are Platform's and
delivery ownership is open (gap register §32). a) Game owns a per-Character balance; the purchase
belongs to the account (D47) and is claimed onto one Character by an idempotent Store claim once
the Store delivery contract exists; until then they have no production source, and the features
that need them stay unused (recommended: no new Game value source, Global's character binding
kept); b) as a), plus a free Game ration of wildcards (for example 5 a week) until the Store
exists: a new value source; c) wildcards as an account balance shared by the account's characters.

**Y3. Admit the Task Board rewards as value sources?** (blocks TASKBOARD-1 rewards,
TASKBOARD-DELIVERY-1 and TASKSHOP-1; D208) a) Yes, at Global parity: task XP, Bounty Points,
tokens, Hunting Task Points and Soulseals as Character balances; delivery item burns; shop item
MINTs, D47 unlocks and promotion points (recommended); b) the tasks and balances, with a shop of
cosmetics only (no item MINT); c) defer the Task Board.

## 16. Decision test

- **Must decide now:** YES. The owner asked for Prey and tasks now; DUR-02 requires a semantic
  gate before any Prey or task table.
- **Minimum sufficient:** six Character tables, one balance row, the existing sequencer, gold fee
  path, XP writer, loot plan and ability stages; one capability; no new location family.
- **Superseding evidence:** owner answers Y1-Y3; captured Global values for grades, price, bands,
  task tables and the shop; the Store delivery contract; the stamina lane's clock.
- **Deliberately not decided:** the Bounty Talisman, the Soulpit, Daily Rewards, the Store
  delivery, party loot boosts, the Wheel promotion points.

## 17. Before-freeze checklist

1. **Contract amendments:** QUEST-STATE-0 §5.2, PREMIUM-ACTIVATION-V1 §4.1 and the content tree,
   each "pending on acceptance of PREY-0". PREY-FEE-1 amends the gold fee decision §4.4 and DUR-03
   §39.3; TASKBOARD-DELIVERY-1 and TASKSHOP-1 amend DUR-03 for their causes.
2. **Serialization:** every write on CHAR-REV-SEQ-1; the death chain order of §9; the §10 lock
   order; one receipt per occurrence.
3. **Restart:** slots, balances, tasks and unlocks are durable; the runtime copy is rebuilt at
   admission; at most `PREY0-RL-07` of hunting time is lost.
4. **Typed references:** CharacterId, Creature and item keys, slot, `slot_epoch`, week, death key,
   CommandRef, the Store delivery id, GameSessionId with `checkpoint_seq`.
5. **Wire:** §11, capability `PREY_V1`.
6. **Determinism:** five named RNG purposes, each seeded by a durable occurrence; draws stored.
7. **Content:** the prey loot budget (`PREY0-RL-16`) fails the content compile closed.
