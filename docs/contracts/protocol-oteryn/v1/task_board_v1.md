# Full Task Board on TASK_BOARD_V1

## Authority and rollout

TASKBOARD-FULL-1 replaces the fixed TASKBOARD-1 gameplay choices for P04/P05.
Capability **72** and command **89** remain the owning route; no capability,
command or mechanic number is added; the existing mechanic gains offer order 560.
The expanded mechanic is initially offered
only in `BUNDLE_WORLD_OFFERED_CAPABILITIES`. An active native gameplay catalogue,
the existing composed admission checks and the current Character/session-generation
fence remain mandatory.

Crystal `201189f871d23597b82caf88fd611454a2ddefb8` is the behavioral reference:
`src/server/network/protocol/protocolgame.cpp:3437-3588` owns the donor action
set. This contract uses Oteryn action numbers, not the donor's opcode framing.

## Request

The existing protobuf field tags remain action=1 and selection=2. Legacy actions
0..3 contain only those two explicitly present fields and retain their existing
32-byte codec. Full actions 4..18 also contain field 3, a UTF-8 race key (explicitly
present, empty unless assigning a preferred/unwanted creature). Their strict
canonical codec requires fields in order, minimal varints, no duplicate or
unknown fields, and at most **192 bytes**.

| Action | Operation | Selection | Race |
| --- | --- | --- | --- |
| 0 | Legacy board projection | 0 | absent |
| 1 | Retired single-task selection; refused | old 1..5 task | absent |
| 2 | Retired weekly explicit claim; refused | old 1..5 task | absent |
| 3 | Upgrade talisman through the full durable owner | old 1..4 path | absent |
| 4 | Query full board and shop | 0 | empty |
| 5 | Start this week's board | difficulty 0..3 | empty |
| 6 | Deliver a weekly task | slot 0..8 | empty |
| 7 | Open bounty offers | 0 | empty |
| 8 | Select a bounty | slot 0..2 | empty |
| 9 | Claim completed bounty | 0 | empty |
| 10 | Change bounty difficulty | difficulty 0..3 | empty |
| 11 | Reroll bounty offers | 0 | empty |
| 12 | Retired; rejected by the request codec | — | — |
| 13 | Assign/clear preferred creature | slot 0..4 | canonical race or empty |
| 14 | Assign/clear unwanted creature | slot 0..4 | canonical race or empty |
| 15 | Unlock next preferred/unwanted list slot | 0 | empty |
| 16 | Buy task-shop offer | offered zero-based index | empty |
| 17 | Buy Bounty Talisman | 0 | empty |
| 18 | Upgrade talisman | path 0..3 | empty |

A syntactically legal index is not purchase or selection authority. The durable
owner checks the actual generated board, enabled offers, current resources,
cooldowns and ownership. Neither time, level, progress nor reward amounts are
accepted from a client.

Difficulty order is Beginner, Adept, Expert, Master. Talisman path order is
damage, life leech, loot, Bestiary.

## Result

The request action discriminates the response. Actions 0..3 retain
`TaskBoardReplyV1`, including the 1024-byte/five-row legacy projection and refusal
flag. Actions 4..18 return `TaskBoardFullReplyV1` from `task_board_v1.proto`.
The full reply is bounded by **16384 bytes**, nine kill rows (including the
any-creature row), nine delivery rows, three bounty offers, five preference
slots, exactly four talisman paths and 32 shop rows. Bounds never truncate a
reply. Canonical decoding rejects duplicated scalars, unknown fields, invalid
booleans, unknown refusal codes, balances outside their owner ranges, kill/bounty
progress above its requirement and overlong/non-canonical race keys.

The full response includes the week identity, all three balances, weekly
progress and pending rewards, bounty offers/active progress, the historical
`daily_reroll_at` field (always zero), preferred/unwanted lists, talisman levels/bonuses/next prices and
shop offer data. Item identifiers in delivery/shop rows identify donor catalogue
entries; the server resolves them against the active canonical Item catalogue
before any custody write.

For an undelivered task, delivery `current` is the recorded count available from
carried inventory, stash and depot, excluding Inbox. It may exceed `required`.
Crystal `201189f` `src/server/network/protocol/protocolgame.cpp:6210–6222` sums
three unsigned 32-bit counts; the displayed result preserves that modulo 2^32
behavior without clamping. Once delivered, `current` is exactly `required`, not
the remaining owned quantity. The count is part of the recorded reply snapshot;
replay does not recount the current inventory.

Each command receipt binds the session command identity to its original action.
A replay returns the stored result snapshot and does not repeat any durable
effect. A different payload under the same command identity is refused.
Refused commands do not spend currency or consume items.

| Refusal | Meaning |
| --- | --- |
| 0 | Success |
| 1 | Invalid rule input |
| 2 | Wrong board or bounty state |
| 3 | Level below the selected difficulty's minimum |
| 4 | Insufficient points, tokens, or other rule resources |
| 5 | Retained legacy cooldown refusal; no new bounty reroll emits it |
| 6 | Progression or list limit reached |
| 7 | Permanent offer already owned |
| 8 | Addon requires the base outfit |
| 9 | Invalid server random draw |
| 10 | Insufficient qualified creature or delivery catalogue |
| 11 | Checked rule arithmetic overflow |
| 12 | Retired legacy select/claim action |
| 13 | Insufficient deliverable items |
| 14 | Existing gold owner refused insufficient funds |
| 15 | Item or decoration-kit delivery is unavailable |
| 16 | Existing item owner refused capacity/change placement |
| 17 | Cosmetic entitlement lacks an admitted target/grant mapping |

Hunting task points use the existing nonnegative BIGINT range, through
`i64::MAX`. Bounty points and soulseals each range from zero through `u32::MAX`,
matching the donor's storage widths. The full wire fields remain uint64.
PR #2764 answer 6543406906 replaces the historical one-billion track bounds;
existing balances and definition pins are preserved. A reward exceeding a bound
records arithmetic-overflow refusal atomically and returns that refusal on replay.

The reroll balance uses uint64 on the wire and the existing nonnegative BIGINT
durable owner, through `i64::MAX`. The free daily token is granted only by Daily Reward; completed
bounties retain their earned token reward. The Winter Update 2026 client stores at most 10
tokens: every credit (daily claim or completed bounty) adds `min(requested, 10 - balance)`.
A completed bounty claimed at 10 or more pays its XP and points and grants 0 tokens (the
receipt's `reroll_tokens_granted` is 0); a daily claim at the cap writes a zero ledger
entry. A historical balance above 10 is kept and only spending lowers it (migration 2760).
Every bounty offer reroll consumes exactly one
token; a zero balance is refusal 4 (insufficient resources), regardless of elapsed time.
The retired free-reroll deadline in saved board JSON is ignored when loading old boards.
The maximum talisman level remains 355, following the accepted
repository profile in `docs/reference/NEWS-HIST-TALISMAN-1.md:15–27` and the PR's
coordinator decisions. No saved level is converted.

The calendar week is derived from committed World reset epochs owned by the
server-save/reset owner. A connection or node wall clock cannot advance it.
Kill credit uses the memoized death occurrence per eligible Character.

## Compatibility

The old five-task rule is retired; its command 1/2 operations are explicitly
refused rather than maintaining a second gameplay state. Existing quest tracks
remain the balance/talisman truth during migration. The full durable owner serves
both codecs. The native Task Board uses the same authoritative state and command
queue for weekly tasks, bounty offers, and the Elite checkbox. Original capability-72
clients omit the capability-474 Elite fields and display Platinum as their highest
supported Gold grade; the frozen reward amounts remain unchanged.

## Resource limits

`TASKBOARD1-RL-01` to `TASKBOARD1-RL-03` own the expanded request/reply/task bounds.
The legacy subcodec keeps its smaller limits. Boundary tests live in
`crates/protocol-oteryn/src/task_board_full_tests.rs`.

## Weekly results

T184 uses capability **474**, `TASK_BOARD_WEEKLY_RESULTS_V1`, requiring 72,
and command **6172**. Its request is the canonical full query (action 4,
selection 0, empty race); other actions on 6172 are rejected. It shares the
existing fenced Task Board owner, receipt binding and full reply bounds.
Only clients selecting 474 receive optional full-reply field 10,
`TaskBoardWeeklyResultsV1`. Original capability-72 clients keep the original
reply shape. The completed board's period and already credited task points and
Soulseals remain readable until a successful action 5 starts the next board.
A query never pays those rewards twice. Failed difficulty choices preserve the
results. Starting uses the existing four difficulty and level checks and replaces
the old tasks atomically.

The client queries on admission through the play queue, separately from opening
the Task Board. After Monday's server save, a settled board opens the dedicated
Weekly Results window. The player selects Beginner, Adept, Expert or Master and
starts the next cycle. Dismissing the window does not spend or claim anything;
it can appear again on the next admission until the next board is started.
The server's committed reset period is the only calendar authority.

Reference: TibiaData v4 news 8567, fetched 2026-11-09
(`https://api.tibiadata.com/v4/news/id/8567`, `OTHER_STRUCTURED_TIBIA_DATA`),
reports a special Monday reward window displaying rewards and choosing the next
difficulty. News 8998 (same fetch date and source class) retires the daily-token
claim from the Bounty Board; Daily Reward supplies it instead. The owner's T195
request in PR #3091 (2026-11-10) also removes the timed free offer reroll.
Earned completed-bounty tokens retain their existing rules.

## Elite bounty tasks (T201)

Owner decision 2026-11-09, news 9004: command **6174** belongs to capability
474 and accepts full action 4 (query) or 19 (selection 0/1 disables/enables Elite).
The checkbox affects newly generated offers; it never changes a selected task or
already displayed offers. Each generated task freezes its Elite flag and reward.
Elite task creatures deal twice the damage to their task owner and receive half
that owner's damage. The generated bounty XP reward doubles. Silver, Gold and
Platinum multiply the usual task rewards by 2, 4 and 8, respectively.

The single bounded content table `content/combat/bounty_elite_v1.json` is
`PARITY_PENDING`. The Winter Update 2026 test server (thread 560) publishes the
caps 75%/20%/5% and the Elite dust examples Beginner 5, Silver Adept 30 and
Platinum Master 560, so dust by difficulty is 5/15/3/70 and the caps are
7500/2000/500 basis points (sum 10000: a creature at the caps always rolls at
least Silver). Expert dust (3), the base weights 500/100/0 and the per-miss
increments 100/50/10 are unpublished owner-approved provisional values, not
assertions of measured Global probabilities. Platinum base 0 means a creature
must have been passed over at least once before it can roll Platinum. A single 1..10000 draw partitions
Platinum, Gold, Silver, then ordinary outcomes. Selecting an improved offer resets
that race's history; selecting an ordinary task retains its history. Other offers
advance once on selection, or all offers advance once when rerolled. Counters
saturate when all content-defined caps have been reached. The existing Character
board and immutable receipts retain at most 4096 race counters, bounded by 1 MiB
JSON; migration 2724 widens the existing storage checks and records the completion
dust selection key in the existing kill receipt. No probability is
accepted from the client. The runtime parses the content table with no fallback.

Elite dust is paid once on the completing kill, with the existing kill XP/dust
transaction and its death receipt, only for a Premium recipient. A monotonic
selection serial binds the dust ledger occurrence; a distinct death arriving
before task credit drains cannot award dust again. Each kill receipt retains
that key for replay even after another task is selected. Task reward XP
and Bounty Points retain the existing claim owner. Dust uses the frozen difficulty
and grade of the selected offer; account state is resolved by the existing Premium
kill owner. Dust overflow keeps the existing dust limit/lost-amount ledger rules.
