# Full Task Board on TASK_BOARD_V1

## Authority and rollout

TASKBOARD-FULL-1 replaces the fixed TASKBOARD-1 gameplay choices for P04/P05.
Capability **72** and command **89** remain the owning route; no capability,
command or mechanic number is added. The expanded mechanic is initially offered
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
| 12 | Claim daily reroll | 0 | empty |
| 13 | Assign/clear preferred creature | slot 0..4 | canonical race or empty |
| 14 | Assign/clear unwanted creature | slot 0..4 | canonical race or empty |
| 15 | Unlock preferred/unwanted list slot | slot 0..4 | empty |
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
booleans, progress above its requirement and overlong/non-canonical race keys.

The full response includes the week identity, all three balances, weekly
progress and pending rewards, bounty offers/active progress, daily-reroll
timestamp, preferred/unwanted lists, talisman levels/bonuses/next prices and
shop offer data. Item identifiers in delivery/shop rows identify donor catalogue
entries; the server resolves them against the active canonical Item catalogue
before any custody write.

Each command receipt binds the session command identity to its original action.
A replay returns the stored result snapshot and does not repeat any durable
effect. A different payload under the same command identity is refused.
Refused commands do not spend currency or consume items.

The calendar week is derived from committed World reset epochs owned by the
server-save/reset owner. A connection or node wall clock cannot advance it.
Kill credit uses the memoized death occurrence per eligible Character.

## Compatibility

The old five-task rule is retired; its command 1/2 operations are explicitly
refused rather than maintaining a second gameplay state. Existing quest tracks
remain the balance/talisman truth during migration. The full durable owner serves
both codecs. Native-client feature registration remains explicitly unsupported;
this change adds the complete server command contract and production consumers,
not a native Task Board view.

## Resource limits

`TASKBOARD1-RL-01` to `TASKBOARD1-RL-03` own the expanded request/reply/task bounds.
The legacy subcodec keeps its smaller limits. Boundary tests live in
`crates/protocol-oteryn/src/task_board_full_tests.rs`.
