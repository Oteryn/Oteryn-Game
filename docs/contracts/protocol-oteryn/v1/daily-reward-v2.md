# DAILY_REWARD_V2

DAILY30 O1-O5: capability 182 carries the redesigned daily claim. Commands 354 (claim) and 355
(query) are selected only with capability 182. Neither command takes client
choices, epochs, amounts or tier. V1 is never offered by the v2 owner.

## claim

Empty payload. The server resolves the committed reset, 30-day lane and current
Premium tier inside the fenced transaction. A replay returns the retained result.

## query

Empty payload; read-only, requires no shrine to view.

## wall

Protobuf varints: 1 lane_day (1..30), 2 streak (0..100000), 3 jokers
(0..3), 4 claimed (boolean), 5 claim_enabled (boolean). The former field 6
seven-day choice plan is absent. Maximum 128 bytes.

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
temple scrolls, XP percent, XP minutes, bounty tokens); history count u8 (0..30);
then each receipt: epoch u64 LE, reset day i64 LE, lane day u8, streak u32 LE,
jokers spent u8, own-character boolean u8. Records are newest first. No trailing
bytes. Maximum 1104 bytes. The claim remains empty and server-owned.
