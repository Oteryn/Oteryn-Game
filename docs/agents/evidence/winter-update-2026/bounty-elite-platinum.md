# Winter Update 2026: Elite and Platinum Bounty Tasks

Units: WU26-01, WU26-02. Ledger rows: P04 (Task Board / bounty), P05 (talisman).

## 1. Official rules (thread 560, Product Manager first post and reply #60983)

- **Elite toggle.** With Elite enabled every *new* task rolls as an Elite task. Target
  creatures deal +100% damage to the player and take 50% less damage from the player.
  Completing an Elite task gives +100% XP and dust by difficulty. Dust goes to Premium
  accounts only (client string). Difficulty and Elite can be changed at any time; a change
  applies to the next task only (client string).
- **Dust examples** (only numbers given): regular-grade Beginner Elite task 5 dust; Silver
  Adept 30; Platinum Master 560. With the existing grade multipliers (x1 / x2 / x4 / x8)
  this gives a per-difficulty base of Beginner 5, Adept 15, Master 70. Expert is not
  given (see section 5).
- **Platinum grade.** Twice the rewards of a Gold task (Gold is x4 of base, so Platinum
  x8).
- **Rarity build-up.** Each time a creature is offered and not selected, its chance to
  appear as Silver, Gold or Platinum rises. Selecting or rerolling raises the chance of the
  creatures not taken. A creature must have been rerolled at least once before it can roll
  Platinum. Caps: Silver 75%, Gold 20%, Platinum 5% (sum 100%, so a long-ignored creature
  always appears at least Silver).
- **Reroll tokens.** The board's separate daily free claim is removed; one token comes with
  every daily reward instead (thread 565). At most 10 tokens can be stored (client
  string). Reply #60983: no further token sources are planned for this update.
- Difficulty pools (client strings): Beginner = easy creatures; Adept = easy and medium;
  Expert = medium and hard; Master = hard and challenging.

## 2. Client evidence

New strings: `Elite`, `Task Difficulty`, the four difficulty descriptions, the Elite
explanation ("monsters deal 100% more damage and take 50% less ... 100% more XP and
additional dust based on difficulty. Dust is only awarded to Premium accounts."),
"changes only apply to your next task", the Silver/Gold/Platinum chance text, "Tasks can
be re-rolled using Reroll Tokens. One token can be claimed for free each day from the
Daily Reward Wall. You can store up to 10 Reroll Tokens at a time.", `Bounty Reroll
Token`. Removed: `Claim Daily` (board button), "No reroll tokens.", the old difficulty
sentence. No new appearance ids.

## 3. What Oteryn has today

The feature is mostly built (T201, #3075).

- Elite flag and per-offer stamping: `crates/gameplay/quests/src/quest/task_board_full.rs:790`-`:835`
  (`generate_bounty`): XP x2 when Elite, dust only when Elite, grade multiplier
  `1 << grade` (x1/x2/x4/x8) on XP, points and dust.
- Grade roll and per-creature miss counter: `crates/gameplay/quests/src/quest/task_board_elite.rs:46`-`:64`
  (`weights`, `grade`), `:76`-`:97` (`remember_offers`: every offer not chosen gains one miss;
  choosing an improved offer resets that creature).
- Numbers are content, marked provisional: `content/combat/bounty_elite_v1.json`
  (`PARITY_PENDING`; dust 1/2/3/4; base 500/100/0 bp; +100/+50/+10 bp per miss; caps
  2500/1000/200 bp = 25% / 10% / 2%). The loader requires cap sum <= 10000
  (`task_board_elite.rs:29`-`:34`).
- Elite combat: player damage x0.5 in `crates/gameplay/dispatch/src/gameplay_transport/attack/drain.rs:2079`
  (`boost_attack_components`); monster damage x2 in
  `crates/gameplay/dispatch/src/gameplay_transport/attack/player_block.rs:332`-`:339` and Elite
  DoTs, both through the session cache in
  `crates/gameplay/dispatch/src/gameplay_transport/bounty_talisman.rs:212`, `:268`
  (`elite_condition_damage`, `elite_bounty`).
- Dust is forge dust, Premium and task-board-eligible only:
  `crates/gameplay/durability/src/durability/party_kill_xp.rs:292`.
- Reroll: `task_board_full.rs:574`-`:588` (one free reroll every 20 h, `FREE_REROLL_SECONDS`
  at `:21`, otherwise one token); the old daily claim action is refused
  (`:589`, `RetiredDailyReroll`). Claiming a bounty grants +1 token (`:567`).
- Token balance: `crates/gameplay/durability/src/durability/task_board/custody.rs:145`-`:164`
  only checks `balance >= 0`; daily rewards credit one token per claim
  (`crates/gameplay/durability/src/durability/daily_reward_effects.rs:33`, `:96`-`:98`).
  **No 10-token cap.**
- Wire: command 89 actions 4-18 and command 6174 (Elite on/off)
  (`crates/protocol-oteryn/src/task_board_full.rs:7`); client panel with grade names and the
  Elite text: `crates/client-ui/src/features/0072-task-board/view.rs:64`-`:94`.
- Test: `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/task_board.rs:203`
  (`task_board_elite_real_dispatch_rarity_combat_xp_and_premium_dust`).

## 4. Gap as implementation units

### WU26-01 Bounty rarity and dust numbers to the test-server profile (class B, S)

- Owned files: `content/combat/bounty_elite_v1.json`,
  `crates/gameplay/quests/src/quest/task_board_elite.rs`, its tests, the elite case under
  `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/task_board/`.
- Change: caps to 7500 / 2000 / 500 bp; dust to 5 / 15 / E / 70 (E = Expert, owner value
  until confirmed, see section 5); keep Platinum base 0 so a creature needs at least one
  miss. Keep the per-miss increments (not published) and record them as provisional.
  The loader's sum check still holds (10000).
- No protocol or persistence change (dust and grade already on the wire and in the board
  JSONB).
- Acceptance tests (quests crate):
  `elite_dust_matches_published_examples` (Beginner x1 = 5, Silver Adept = 30, Platinum
  Master = 560); `rarity_caps_reach_75_20_5_and_always_at_least_silver` (a creature at the
  max miss count never rolls ordinary); `platinum_needs_one_miss` (misses = 0 never rolls 3
  for any roll 1..=10000).

### WU26-02 Reroll token storage cap of 10 (class B, S)

- Owned files: `crates/gameplay/durability/src/durability/task_board/custody.rs`,
  `crates/gameplay/durability/src/durability/daily_reward_effects.rs` (bounty token credit
  only), one PG case in `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/`
  (mounted directory, new file).
- Change: every credit (daily claim, bounty claim) records `requested` and credits
  `min(requested, 10 - balance)`, like the prey wildcard ledger does
  (`daily_reward_effects.rs:60`-`:63`). Existing balances above 10 are kept (no clawback)
  but receive nothing until spent below 10.
- Persistence: the entries table already stores before/after; a new migration is needed
  only if a `requested` column is added. Claim the number with
  `.claude/skills/oteryn-dev/scripts/next-free-numbers.py`.
- Acceptance: `bounty_reroll_tokens_never_exceed_ten_on_daily_claim`,
  `bounty_claim_at_ten_tokens_credits_zero_and_records_the_entry`.

## 5. Open questions

1. Expert Elite dust base (Beginner 5, Adept 15, Master 70 are derivable; Expert is not).
2. Per-miss increment of the Silver/Gold/Platinum chances and the starting chances (only
   caps are published).
3. Whether selecting an improved offer resets that creature's build-up (Oteryn does;
   the post does not say).
4. Whether Global keeps a free reroll every 20 h (Oteryn's `FREE_REROLL_SECONDS` comes
   from Crystal); the new client text names the Daily Reward Wall as the only free source.
