# DAILY-REWARD-0 Daily Reward System

- Decision: `DAILYREWARD0-DAILY-REWARD-SYSTEM-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence,
  value and protocol) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers:
  - EXERCISE-0 R3 ("training weapons: DAILY-REWARD-0, which reuses §4 and §5 with a 50-charge
    definition");
  - PREY-0's implementation brief (Daily Rewards as a wildcard source);
  - PLAYER-TRADE-0 §3 and MAIL-0 §1 ("Store-bound items ... their decision adds the refusal") for
    reward items;
  - control-plane allocation D294 (#1622).
- Builds on:
  - ADR-0021 and BOSS-RAID-0 §2 (a durable planned World reset with an epoch is the Tibia server
    save);
  - MARKET-0 §5 (the `CharacterInbox` family; deliveries are never refused);
  - EXERCISE-0 §3-§5 (exercise definitions, training, the checkpoint);
  - TIMED-ITEM-0 §4 (charges);
  - PREY-0 §4 (the Account wildcard balance and ledger);
  - DUR-03 §5.5, §11.3, §14 and §38 (binding, planned outputs, mint, the rewards row);
  - WO-0 and DEPOT-0 §4.1 (a world object USE as a non-durable view open);
  - PREMIUM-ACTIVATION (Premium evidence, fail closed);
  - SUPPLY-STASH-0 §4 (the Store-sourced marker);
  - owner rule 5905825574.
- Amends, each pending on acceptance of DAILY-REWARD-0 and written by its children in their own
  docs commits:
  - PREY-0 §4 (a `DAILY_REWARD` wildcard entry kind);
  - DUR-03 §14-§15 and the §38 rewards row (`DailyRewardCause`);
  - PLAYER-TRADE-0, MARKET-0 §3.1, MAIL-0, NPC-0, SUPPLY-STASH-0 §4 and ITEM-MOVE-WIRE-1 (the
    bound-item refusal, §6).
- Runtime, migration and production authority: NONE. Each child needs its own #162/#1622
  allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| DAILY-CONTENT-1 | content lane | reward shrine bindings in temples and depots (§3.1); the per-vocation rune and potion list (§4.2); 8 training weapon definitions (§4.3); the Gold Converter and Temple Teleport scroll definitions, inert (§4.4) | WO-1; EXERCISE-CONTENT-1; RUNE-USE content |
| DAILY-1 | hard (persistence), persistence and value review | the reward state, joker, claim and line tables (§5); the claim transaction (§3); the item binding table and its writer (§6); line delivery to the Inbox and the wildcard balance (§4) | MARKET-1 (`CharacterInbox`); PREY-1 (wildcard balance); the World reset epoch (ADR-0021) |
| DAILY-BIND-1 | impl | the bound-item refusals at each listed site (§6) | DAILY-1 |
| DAILY-WIRE-1 | impl, protocol review | capability `DAILY_REWARD_V1`, the reward wall view, the claim request (§7) | DAILY-1; WORLDINT-USE-1 |

Later, each with its own decision or amendment:
- the resting-area bonuses (RESTING-AREA-0: the tile flag and the regeneration effects; it reads
  the streak of §3.3);
- the XP boost (XP-BOOST-0: the modifier and its activation; it delivers the pending day-7 lines);
- the Gold Converter and Temple Teleport scroll use;
- Instant Reward Access and house reward shrines (the Store and house lanes);
- the Store Inbox (gap register §32);
- Double Daily Reward events.

## 1. Question

How does a character collect a daily reward, keep a streak and receive Tibia's reward items?

## 2. Facts

**PROVEN**

- ADR-0021 and BOSS-RAID-0 §2: the planned World reset has a durable epoch and is Tibia's server
  save; its time is operational. A crash restart is not a reset.
- MARKET-0 §5: `CharacterInbox {character_id, ordinal}` (Character + World) is a custody family;
  its out-shapes run at the locker; a delivery is never refused, its bound is its source's bound.
- PREY-0 §4: `game_account_prey_wildcards` per (Account, World), cap `PREY0-RL-05` = 52, with an
  immutable ledger whose kinds are closed.
- EXERCISE-0 §3-§5: exercise weapons are definitions with `charges` and an `exercise` fact; R3
  gives training weapons to this decision.
- PLAYER-TRADE-0 §3 and MAIL-0 §1: no item binding exists; "Store-bound items do not exist yet;
  their decision adds the refusal".
- The D118-D128 batch: no XP boost source exists; only a later decision adds an XP modifier.

**CIPSOFT_OFFICIAL** (the Tibia manual)

- `interface.md` lines 157-160: a pickup needs standing next to a reward shrine or an Instant
  Reward Access (Store); the reward goes to the Store inbox; the streak grows with each consecutive
  pickup window and a streak of 2 or more grants resting-area bonuses; a daily reward joker covers
  a missed day; "1 joker granted per account on the 1st of each month; max 3 jokers held at once".
- `characters.md` lines 58 and 185: the resting area is a subset of protection zones (houses,
  temples, depots); Premium players with a sufficient streak regenerate stamina and soul there
  while online.
- `products.md` line 187: Instant Reward Access claims from anywhere. Line 97: house reward
  shrines.

**TIBIAWIKI_STRUCTURED** (Daily Reward System rev 905700)

- Reward Shrines are in the temples and depots of all major cities. One reward between two server
  saves, whatever the time of the last claim. The wall can also be opened from the inventory
  button.
- The lane has 7 positions; each claim moves it on, and after 7 it returns to 1. Missing days does
  not move it.

  | Day | Free | Premium |
  |---|---|---|
  | 1 | choose 5 runes or potions | choose 10 |
  | 2 | choose 5 runes or potions | choose 10 |
  | 3 | 1 Prey Wildcard | 3 Prey Wildcards |
  | 4 | choose 10 runes or potions | choose 20 |
  | 5 | a temporary Gold Converter (100 charges) | a temporary Temple Teleport scroll and a temporary Gold Converter |
  | 6 | 1 training weapon | 2 training weapons |
  | 7 | 10 minutes of 50% XP boost | 30 minutes of 50% XP boost |

- Rewards are not tradeable and go to the Store inbox. Only runes and potions the vocation can use
  are offered, with no level filter. Day-7 rewards expire after 7 days if not used.
- Streak bonuses in a resting area: day 2 HP regeneration, day 3 mana regeneration, day 4 stamina,
  day 5 double HP, day 6 double mana, day 7 soul. Days 4-7 are Premium only.
- A missed day resets the streak to 1 at the next claim unless reward jokers cover it; jokers are
  consumed automatically at the claim. A server reset keeps the streak.
- Exercise Weapons rev 1126820: training weapons have 50 charges (1 min 40 s of training).

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b5`, `data/modules/scripts/daily_reward/daily_reward.lua`)

- Per-character storages for lane day, next reward time, jokers and streak; a per-vocation rune
  and potion list. Its lane differs from TibiaWiki on days 3 and 5 (R6).

## 3. Claims (DAILY-1)

### 3.1 Shrines and the wall

- A reward shrine is a `container_fixture`-like world object bound by DAILY-CONTENT-1 in each
  temple and depot. A USE on it while adjacent (Chebyshev 1, same floor) opens the reward wall as a
  non-durable view (the DEPOT-0 §4.1 idiom), with the claim enabled.
- The wall can also be opened anywhere, view only, to show the lane, the streak and the jokers.

### 3.2 The claim window

- One claim per character per World reset epoch: the claim is keyed by (`character_id`,
  `reset_epoch`) and is unique. A second claim in the same epoch is `ALREADY_CLAIMED`.
- A crash restart is not a reset, so it neither opens a new window nor breaks a streak.

### 3.3 Lane, streak and jokers

- **Lane.** `lane_day` 1..7 is the next reward. A claim takes it and moves it to the next day,
  wrapping after 7. Missed epochs do not move it.
- **Streak.** Let `gap` = current epoch − `last_claim_epoch` − 1 (0 for consecutive epochs).
  - No earlier claim: the streak becomes 1.
  - `gap` = 0: the streak grows by 1.
  - `gap` > 0 and the jokers cover it: `gap` jokers are spent and the streak grows by 1.
  - Otherwise the streak becomes 1 and no joker is spent.
- **Jokers.** Held per Account + World, 0 to 3 (R3). They are granted lazily: at a claim, the
  balance gains one per calendar month (UTC, `PARITY_PENDING`) that has had a reset epoch since
  the last grant month, capped at 3, then pays the gap.
- **Premium.** The Premium lane and the Premium streak bonuses use PREMIUM-ACTIVATION evidence,
  read at the claim, fail closed: until PREM-3 every claim takes the Free column.

### 3.4 The transaction

- One PostgreSQL transaction with one TransactionId; planned outputs fixed before the first
  attempt (DUR-03 §11.3).
- Checks: the open wall from an adjacent shrine; the request's picks (§4.2, §4.3).
- **Fence.** Composition rule 2 with the `character_root` lock; then the joker row FOR UPDATE.
- **Writes.** The reward state row, the joker row and its entries, the claim and its lines, and
  the delivery of every line whose sink exists (§4.5).
- A refusal writes nothing.

## 4. Rewards (DAILY-CONTENT-1, DAILY-1)

### 4.1 Lines

A claim has one line per reward part: kind (`ITEM`, `PREY_WILDCARD`, `XP_BOOST`), definition,
quantity and status (`DELIVERED` or `PENDING`). At most `DAILYREWARD0-RL-02` (20) item outputs
per claim.

### 4.2 Runes and potions (days 1, 2 and 4)

- The request picks definitions and counts that sum to exactly N (5, 5 or 10 Free; 10, 10 or 20
  Premium). Each definition must be on DAILY-CONTENT-1's list for the character's vocation. There
  is no level filter.
- Each pick becomes stacks of at most 100 in the Inbox.

### 4.3 Training weapons (day 6)

- DAILY-CONTENT-1 adds 8 definitions, one per EXERCISE-0 kind (sword, axe, club, wraps, bow, wand,
  rod, shield), each with `charges {count: 50, show_count: true}` and the `exercise` fact.
- They train exactly as EXERCISE-0 §4 and §5 say. The request picks the kind (1 Free, 2 Premium,
  repeats allowed). They are never sold by NPCs and are bound (§6).

### 4.4 Gold Converter and Temple Teleport scroll (day 5)

DAILY-CONTENT-1 defines both as bound items: the Gold Converter with `charges {count: 100}`, and
the Temple Teleport scroll with one use. Until their use decisions, a USE on them is
`NOTHING_TO_USE` (the EXERCISE-0 precedent). Their "temporary" expiry is `PARITY_PENDING` and
decided with their use.

### 4.5 Delivery and pending lines

- **Items.** Each `ITEM` line mints fresh items into the character's `CharacterInbox` (MARKET-0
  §5), with their binding rows (§6), under the closed `DailyRewardCause {claim_id, line}`. The
  claim's own bound (20 outputs) is the Inbox bound.
- **Wildcards.** A `PREY_WILDCARD` line credits the Account wildcard balance with the new entry
  kind `DAILY_REWARD`, capped at `PREY0-RL-05`; the uncredited excess is recorded on the line
  (`PARITY_PENDING`).
- **XP boost.** An `XP_BOOST` line (10 or 30 minutes) stays `PENDING`. XP-BOOST-0 delivers it, and
  its 7-day expiry starts at that delivery.
- **Pending.** A line whose sink is not built (MARKET-1, PREY-1, XP-BOOST-1) stays `PENDING`. That
  sink's child delivers it later, exactly once, keyed by (claim, line) (R4).

## 5. Storage (DAILY-1)

- `game_character_daily_rewards`: one row per (`character_id`, `world_id`): `lane_day` 1..7,
  `streak` (0 to `DAILYREWARD0-RL-01`), `last_claim_epoch`.
- `game_account_daily_reward_jokers`: one row per (`account_id`, `world_id`): `jokers` 0..3,
  `last_grant_month`, `last_entry_id`; with an immutable entry ledger (`GRANT`, `SPEND`), as in
  BANK-0 §3.
- `game_character_daily_reward_claims`: one row per (`character_id`, `reset_epoch`), unique:
  TransactionId, a SHA-256 binding of the request (picks), lane day, streak before and after,
  jokers spent, the Premium flag read.
- `game_character_daily_reward_lines`: (claim, line), kind, definition, quantity, status, and the
  delivering TransactionId.
- Scope: Character + World, and Account + World for jokers; no channel. `character_id` references
  the Character root with RESTRICT. Never touched by death, `WorldReset` or channel changes.
- Grants: `oteryn_game_runtime` SELECT and INSERT; UPDATE only on the state, joker and line status
  columns; never DELETE.

## 6. Bound items (DAILY-1, DAILY-BIND-1)

- **Binding.** `game_item_bindings`: one row per bound item: item, World, `character_id`, cause
  (`DAILY_REWARD`). It is written in the mint transaction and goes only with the item's
  retirement. It is the first DUR-03 §5.5 binding and the Store-sourced marker that SUPPLY-STASH-0
  §4 reads; the Store decision reuses it.
- **Refused** (`NOT_TRADEABLE`, or the site's own refusal): player trade, a Market offer, mail, the
  Stash, an NPC sale, and a drop to the Ground (`PARITY_PENDING`).
- **Allowed:** Inbox and depot moves, moves within the character's own containers, use and
  training.

## 7. Wire (DAILY-WIRE-1)

- **Capability `DAILY_REWARD_V1`**, number reserved by the control plane before DAILY-WIRE-1.
- The reward wall view shows the lane day, the 7 rewards of the Free or Premium column, the
  streak, the jokers and whether this epoch's claim is taken.
- **Claim.** `DailyRewardClaimV1 {picks: [(definition, count)]}`: rune and potion picks for days
  1, 2 and 4, and weapon kinds for day 6. Results: `ALREADY_CLAIMED`, `INVALID_PICK`,
  `NOT_IN_REACH` and the existing ones.

## 8. Rows (registered by DAILY-1 and DAILY-WIRE-1)

| Row | Value |
|---|---|
| `DAILYREWARD0-RL-01` streak ceiling | 100,000 (an engineering bound) |
| `DAILYREWARD0-RL-02` item outputs per claim | 20 |
| Jokers held per (Account, World) | 3 |
| Claims per character per reset epoch | 1 |

## 9. Rejected options

- **Building the Store Inbox now.** Its ownership is a Game and Platform question (gap register
  §32); the Inbox delivers today (R1).
- **Unbound rewards.** Rewards would become a tradeable value source, which Tibia does not allow
  (R2).
- **A wall-clock 24-hour window.** Tibia counts server saves, and a crash would move the window.
- **Waiting for every sink before shipping.** The lane, streak and items work now; pending lines
  lose nothing (R4).
- **Deciding resting-area bonuses here.** They need a tile flag and regeneration effects that
  belong to their own decision (R5).

## 10. Architect rulings (owner rule 5905825574)

- **R1, delivery: a) the `CharacterInbox` until a Store Inbox decision** (declared difference: the
  Store inbox); b) the main backpack; c) a Store Inbox now. Recommendation and ruling: a).
- **R2, binding: a) a `game_item_bindings` row refused at the §6 sites** (Tibia: not tradeable);
  b) unbound. Recommendation and ruling: a).
- **R3, jokers: a) per Account + World** (official manual, PREY-0 Y2 c precedent); b) per Character
  (TibiaWiki, Canary). Recommendation and ruling: a).
- **R4, missing sinks: a) durable `PENDING` lines, delivered once when the sink ships**; b) no
  claims until every sink exists. Recommendation and ruling: a).
- **R5, resting area: a) RESTING-AREA-0 later, reading the streak stored now**; b) here.
  Recommendation and ruling: a).
- **R6, lane: a) TibiaWiki rev 905700** (Global documentation); b) Canary. Recommendation and
  ruling: a), `PARITY_PENDING`.

## 11. Owner questions

None. Every choice above is a reversible architect ruling under owner rule 5905825574.

## 12. Decision test

- **Must decide now:** YES. Control-plane allocation D294; EXERCISE-0 R3 and PREY-0 point here.
- **Blocked without it:** training weapons, the wildcard source, reward items, and RESTING-AREA-0's
  streak input.
- **Harder later:** YES. Streak and lane history cannot be rebuilt after the fact, and unbound
  rewards would leak into trade.
- **Supersede if:** official evidence for per-character jokers, the current lane table, the
  joker month boundary or the bound-item drop rule; a Store Inbox decision (R1).
- **Deliberately not decided:** resting-area bonuses, XP boost, Gold Converter and scroll use,
  Instant Reward Access, house shrines, the Store Inbox, Double Daily Reward events.

## 13. Before-freeze checklist

1. **Contract amendments:** PREY-0 §4, DUR-03 §14-§15 and §38, and the §6 refusal sites; each
   pending on acceptance, written by DAILY-1 and DAILY-BIND-1.
2. **Serialization:** one transaction per claim, Character fence, root lock, joker row lock.
3. **Restart:** claims, lines and state are durable; a replayed claim returns its first outcome;
   a pending line is delivered once.
4. **Typed references:** CharacterId, AccountId, WorldId, reset epoch, definition keys.
5. **Wire:** §7, capability `DAILY_REWARD_V1`.
6. **Split work:** at most 20 item outputs per claim; one claim per epoch.
