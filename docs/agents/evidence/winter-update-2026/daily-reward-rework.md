# Winter Update 2026: Daily Reward Rework

Units: WU26-03 to WU26-07. Ledger rows: P09 (daily wall, streak, jokers, reward cards, XP
boost), P10 (resting-area bonuses).

## 1. Official rules (thread 565, Programmer first post)

- **Cosmetic Tokens.** New character-bound currency earned from daily rewards; 1 token has
  the purchasing value of 1 Tibia Coin; spendable in the Store on mounts, outfits, house
  decorations, furniture, beds and hireling dresses. Only the earning character can use
  them. Store purchases are disabled on the test server.
- **Seven-day cycle with fixed rewards**: Cosmetic Tokens, Prey Wildcards, a temporary
  temple teleport scroll and XP boosts. One-click claim; no choice of runes, potions or
  training weapons any more.
- **Bounty Reroll Token**: every daily reward includes one; the board's own daily claim is
  removed (see `bounty-elite-platinum.md`).
- **XP boosts.** A Daily Reward XP boost *adds* its percentage to an active Store XP boost
  instead of extending its duration (two overlapping 50% boosts give +100%). Claiming
  another Daily Reward XP boost *replaces* any remaining Daily Reward XP boost.
- **Streak bonus.** Each consecutive daily claim raises the chance of double Weapon
  Proficiency XP by one percentage point, up to 7%; Free and Premium alike. Client: "Claim
  your daily reward once between two regular server saves to keep your streak rising.
  Your streak grants a chance of up to 7% ...".
- **Resting areas no longer depend on the streak.** Premium: double HP and mana
  regeneration plus stamina and soul regeneration. Free: HP and mana regeneration.

The per-day table is not published in the CM post. Player posts in the same thread (no CM
confirmation) mention day 3 = 1 temple teleport scroll for Premium too, day 4 = 25% XP
boost, and 5 / 10 Cosmetic Tokens per week (Free / Premium).

## 2. Client evidence

New strings: `Cosmetic Tokens`, `$%1 Cosmetic Tokens`, "You do not have enough Cosmetic
Tokens.", `%1x Prey Wildcards`, `1 Temple Teleport Scroll`, `%1 minutes XP Boost %2%`,
`- Daily Reward XP boost: +%1% (%2 h remaining)` / `(paused)`, `- Store XP boost: +%1% (%2 h
remaining)` / `(paused)`, `Reward Streak Bonus`, "Your reward streak currently grants you a
%1% chance to gain double weapon proficiency XP.", "You currently have no Reward Streak
Bonus.", `Claim`, `Bounty Reroll Token`, Premium hint strings. Removed: the resting-area
streak bonus texts ("If you have reached a reward streak of at least %1 ..."), the
Free/Premium reward table HTML, the single `- XP boost: +%1%` line, "%1 minutes 50% XP
Boost", "Already claimed today.".

## 3. What Oteryn has today

Most of the rework already exists (DAILY30, owner decision; contract
`docs/contracts/protocol-oteryn/v1/daily-reward-v2.md`).

- Reward table: `content/interactions/daily-rewards.json` (schema v2, **30 days**). Days 1-7
  free / premium: 2/4 Cosmetic Tokens; 1/2 Prey Wildcards; 1/1 temple scroll; 10%/25% XP
  boost 30 min; 3/6 tokens; 1/2 wildcards; 20%/50% XP boost 30 min; one Bounty Reroll Token
  every day. Days 8-30 repeat with milestone extras on 14, 21, 28 and 30. This matches the
  player-reported week (5/10 tokens, day 3 scroll, day 4 25%). Loader:
  `crates/gameplay/content/src/content/daily_reward.rs`; cycle constant
  `crates/gameplay/progression/src/daily_reward.rs:6` (`CYCLE_DAYS = 30`).
- Cosmetic Tokens: earned per claim into `game_character_cosmetic_tokens` (migration 0547)
  by `crates/gameplay/durability/src/durability/daily_reward_effects.rs`; spent through
  capability 479, commands 6187 / 6188 (`crates/protocol-oteryn/src/store/cosmetic_tokens.rs`,
  contract `docs/contracts/protocol-oteryn/v1/store-cosmetic-tokens-v1.md`, owner decision
  2026-11-09). Platform coins are not touched. Eligible categories: outfits, mounts, house
  decorations and furniture (beds and hireling dresses are not named; see section 5).
- Streak bonus: `crates/gameplay/progression/src/daily_reward.rs:17`
  (`proficiency_bonus_chance`: 1-7% for streak 1-7, then **7/8/9/10%** at 8/14/21/28),
  applied in `crates/gameplay/dispatch/src/gameplay_transport/kill_reward.rs:3430` and
  `:3454` (`daily_proficiency_points`).
- XP boost model: the daily grant starts at the latest expiry of either the Store or the
  daily boost and appends its minutes
  (`crates/gameplay/durability/src/durability/daily_reward_effects.rs:72`-`:77`, citing
  Crystal); the active percent is the **max** of both sources
  (`crates/gameplay/durability/src/durability/hunting_xp.rs:40`); the XP formula accepts only
  0/10/20/25/50 (`crates/gameplay/characters/src/hunting_experience.rs:21`). The wire already
  reports the two sources separately (`crates/protocol-oteryn/src/potion_durations.rs:7`-`:28`,
  source 3 Store fixed at 50%, source 4 daily). No production writer of
  `game_character_store_xp_boosts` exists (Platform-owned Store).
- Resting area: HP from streak 2, mana from 3, doubled at 5 / 6, Free capped at an effective
  streak of 3 (`crates/gameplay/spell/src/spell/food_regeneration.rs:98`-`:128`); Premium
  stamina from streak 4 and soul from streak 7
  (`crates/gameplay/dispatch/src/gameplay_transport/resting_area.rs:131`-`:154`).
- Item picker still present beside the one-click claim: commands 6122 / 6123
  (`crates/protocol-oteryn/src/daily_reward/picker.rs:2`-`:3`,
  `crates/gameplay/dispatch/src/gameplay_transport/daily_reward/picker.rs`,
  `crates/gameplay/progression/src/daily_reward/picker.rs`), table `game_daily_reward_picks`
  (migration 1711), client entry `apps/client/src/features/0182-daily-reward/native.rs:152`-`:191`.
- Temple teleport scroll (item 25718) is minted by the claim, but no use handler exists:
  the only production callers of `home_temple` are death respawns
  (`crates/gameplay/dispatch/src/gameplay_transport/composed_admission/death.rs:206`,
  `promotion_death.rs:57`); the item record keeps `temporal` UNKNOWN
  (`content/items/definitions/items-14500-14999.json`, key `oteryn:item.tibia.i25718`).

## 4. Gap as implementation units

### WU26-03 XP boosts: additive Store + daily, daily replaces daily (class B, M)

- Owned files: `crates/gameplay/durability/src/durability/daily_reward_effects.rs` (XP boost
  branch), `crates/gameplay/durability/src/durability/hunting_xp.rs`,
  `crates/gameplay/characters/src/hunting_experience.rs`,
  `crates/gameplay/durability/src/durability/item_use/concoction.rs` (same reader),
  `crates/protocol-oteryn/src/potion_durations.rs` only if `paused` is added, a new PG case.
- Change: a daily grant starts now and ends the previous daily boost (set its
  `effect_expires_at` to now in the same transaction, receipt kept); active percent =
  daily percent + Store percent; widen the accepted set to the sums (10..100).
- Persistence: no new table; the update of the previous line must be an append-only
  correction if the lines table is immutable (check its trigger first).
- Acceptance: `daily_xp_boost_adds_to_store_boost_percent` (50 + 50 = 100 in
  `hunting_experience`), `second_daily_xp_boost_replaces_the_first`,
  `daily_xp_boost_no_longer_extends_store_timer` (PG).

### WU26-04 Resting-area bonuses independent of the streak (class B, S)

- Owned files: `crates/gameplay/spell/src/spell/food_regeneration.rs`,
  `crates/gameplay/dispatch/src/gameplay_transport/resting_area.rs`,
  `crates/gameplay/dispatch/src/gameplay_transport/composed_admission/vitals.rs`, tests,
  `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/daily_resting.rs`.
- Change: Free = HP and mana resting regeneration (today's streak-3 tier); Premium = doubled
  HP and mana plus stamina and soul (today's streak-7 tier); streak no longer read.
- Acceptance: `resting_free_account_gets_hp_and_mana_without_streak`,
  `resting_premium_gets_double_and_stamina_soul_without_streak`, PG
  `daily_resting_zero_streak_premium_regenerates_soul`.
- Ledger: P10 is DONE today; this reopens it as a change, not a defect.

### WU26-05 Retire the daily item picker (class B, S)

- Owned files: `crates/protocol-oteryn/src/daily_reward/picker.rs`,
  `crates/gameplay/dispatch/src/gameplay_transport/daily_reward/picker.rs`,
  `crates/gameplay/progression/src/daily_reward/picker.rs`, the `selection` plumbing in
  `crates/gameplay/durability/src/durability/daily_reward_delivery.rs`, client
  `apps/client/src/features/0182-daily-reward/picker_native.rs` and its entry in `native.rs`.
- Protocol: commands 6122 / 6123 refused (or capability withdrawn); contract note in
  `docs/contracts/protocol-oteryn/v1/daily-reward-v2.md`. Persistence: migration 1711 stays
  (history); no new writes.
- Acceptance: `daily_picker_commands_are_refused_after_rework`, client fixture without the
  picker button.

### WU26-06 Temple teleport scroll: use and 7-day expiry (class C, M)

- Owned files: a new item-use handler under
  `crates/gameplay/dispatch/src/gameplay_transport/item_use/`, reuse of
  `character_home_town.rs:124` (`home_temple`), item record for 25718.
- Rules (TibiaWiki "Temple Teleport Scroll"; thread 565 says "temporary"): use teleports to
  the home temple; refused in a no-logout zone, with a logout block or a protection-zone
  block; consumed on use; expires 7 days after delivery.
- Persistence: expiry needs a per-instance expiry time (use the existing item temporal model
  if present, otherwise a migration; claim the number).
- Acceptance: `temple_scroll_teleports_home_and_is_consumed`,
  `temple_scroll_refused_with_logout_block`, `temple_scroll_expires_after_seven_days` (PG).

### WU26-07 Seven-day cycle and 7% streak cap (class B, S; owner decision first)

- Official text: seven-day cycle, chance capped at 7%. Oteryn: 30-day DAILY30 lane and
  chance up to 10%. This is an owner choice (DAILY30 was an owner decision), not a defect.
- If the owner adopts the test-server rule: `content/interactions/daily-rewards.json` to 7
  days, `CYCLE_DAYS = 7`, `proficiency_bonus_chance` capped at 7, contract text in
  `daily-reward-v2.md`, client panel text. Persistence: migration 0547 allows lane 1..30;
  7 fits, no migration.
- Acceptance: `streak_bonus_caps_at_seven_percent`, `daily_lane_wraps_after_day_seven`.

## 5. Open questions

1. Exact seven-day table and Premium multipliers (not published by CipSoft; Oteryn's days
   1-7 agree with the player reports).
2. Whether the 30-day DAILY30 lane and the 8-10% chance stay (owner).
3. When an XP boost shows `(paused)` (client string exists; the condition is not described).
4. Whether beds and hireling dresses must be added to the Cosmetic Token categories
   (thread 565 lists them; the Oteryn contract names outfits, mounts and house
   decorations / furniture).
