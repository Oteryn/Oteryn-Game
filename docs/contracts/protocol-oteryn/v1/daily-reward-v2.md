# DAILY_REWARD_V2

DAILY30 O1-O5: capability 182 carries the redesigned daily claim. Commands 354 (claim) and 355
(query) are selected only with capability 182. Neither command takes client
choices, epochs, amounts or tier. V1 is never offered by the v2 owner.

## claim

Empty payload. The server resolves the committed reset, seven-day lane and current
Premium tier inside the fenced transaction. A replay returns the retained result.

Winter Update 2026 (WU26-07, owner decision 2026-11-11) replaces the DAILY30 30-day lane
with a seven-day cycle of fixed rewards (`content/interactions/daily-rewards.json`, seven
days). Consecutive claims walk days 1..7 and wrap to day 1; a gap the jokers cannot cover
restarts the streak and the cycle at day 1. A lane stored before the rework on day 8..30
resumes on the same weekday, `1 + ((lane_day - 1) % 7)`.
Migration 2763 admits the wrap in `game_daily_reward_projection_guard`: after a day 7
claim the stored lane day is 1; states written before it (lane day 8 after day 7) still prove.

## query

Empty payload; read-only, requires no shrine to view.

## wall

Protobuf varints: 1 lane_day (1..7), 2 streak (0..100000), 3 jokers
(0..3), 4 claimed (boolean), 5 claim_enabled (boolean). The former field 6
seven-day choice plan is absent. Maximum 128 bytes.

The client presents the weapon proficiency XP bonus from this server-confirmed
streak: streak 0 has 0% chance, each streak day adds 1%, capped at 7% from streak
7 onward (WU26-07), for Free and Premium alike. Each eligible proficiency XP award
can be doubled; this is a chance to double an award, not a guaranteed XP rate
increase. This mirrors `gameplay/progression/src/daily_reward.rs`; Game rolls
and awards the bonus in `gameplay_transport/kill_reward.rs`. The panel explains
the 7% cap and asks the player to refresh after claiming, since a claim
result contains no updated streak. No client-supplied chance enters a command.

## result

Protobuf varint field 1 disposition (1..7), bytes field 2 UUIDv7 only for
CLAIMED (1). Other dispositions retain DAILY_REWARD_V1 result meanings.
Maximum 20 bytes. No new snapshot domain is introduced.

## details

T144 adds read-only command 738 to the existing capability 182. Empty request;
no shrine interaction and no economic mutation. The reply describes all thirty
Free/Premium catalogue rows and the latest thirty durable claims for the current
account and World, including whether each belongs to this character. Recovery,
node, character, scope, session and connection-generation fences apply to the read.

Bounded binary `DRD1`: four magic bytes; current reset epoch u64 LE; thirty days,
each Free then Premium tier (six u8 values: cosmetic tokens, prey wildcards,
temple scrolls, XP percent, XP minutes, bounty tokens; since WU26-07 only days 1..7
carry the cycle and days 8..30 are all zero); history count u8 (0..30);
then each receipt: epoch u64 LE, reset day i64 LE, lane day u8, streak u32 LE,
jokers spent u8, own-character boolean u8. Records are newest first. No trailing
bytes. Maximum 1104 bytes. The claim remains empty and server-owned.

## item picker (T144, owner decision B 2026-11-09), retired

Retired by WU26-05 (Winter Update 2026 test server, thread 565: one-click claim, no
choice of runes, potions or exercise weapons). Commands 6122 and 6123 left capability
182 and the registry: no capability owns them, Dispatch routes them nowhere and refuses
them as unregistered (`Rejected`, empty payload) without reading their bytes. Both
numbers stay retired and are never reused. The one-click claim is the
only daily claim; its request binding is unchanged. No picked item is minted and
`game_daily_reward_picks` (migration 1711) is history only. The rules below describe
the retired behaviour.


Reference: Canary `99902524e052f37574194466c2949c576e4ab269`,
`data/modules/scripts/daily_reward/daily_reward.lua:68-75,107-150,409-479`.
Vocation-specific potions/runes are available on reference days 1, 2 and 4:
Free quantities 5, 5, 10; Premium 10, 10, 20. Day 6 offers the eight listed
exercise weapons, Free 1/Premium 2, each with 50 charges. Other reference days
have no item choices. Promotions use their base vocation. Selected quantities
must be positive, unique by item, from the current catalogue, and sum to no more
than the server's allowance. Delivery is to the character Inbox, atomically with
the daily receipt; full Inbox, stale fence, invalid choice or changed day rolls
back the entire claim. Premium is rechecked inside the existing Platform-backed
gate, never supplied by the client. A replay cannot mint again.

Owner decision B 2026-11-09, reference silent: retain the accepted thirty-day
reward table and empty one-click claim, and overlay the reference seven-day item
schedule using `1 + ((lane_day - 1) % 7)`. An empty one-click claim opts out of
physical items; the pick claim delivers selected physical items plus the same
existing daily rewards, and consumes the same daily entitlement. At most nineteen
unique item selections keep the complete mint, including a possible temple scroll,
within the existing twenty-output Inbox source bound. Exercise quantities are
separate weapon instances, each with its admitted fifty-charge definition.

Existing capability 182 gains commands 6122 (picker query, empty request) and
6123 (pick claim). Query replies `DRP1`, epoch u64 LE, lane day u8, Premium bool
u8, quantity allowance u8, and exactly 32 u32 LE item IDs (zero-padded). The
server resolves vocation and tier under the current fence. Claim payload `DRC1`,
epoch u64 LE, lane day u8, line count u8 (1..19), then strictly increasing item
u32 LE/count u8 pairs. No trailing bytes. Epoch/day are optimistic guards, never
authority. The reply is the existing daily result. The binding includes the full
canonical choice payload. No new capability, commercial balance, or snapshot.
