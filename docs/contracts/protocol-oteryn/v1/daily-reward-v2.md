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
