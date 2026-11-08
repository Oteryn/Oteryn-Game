# Party accounting query

Command 735 requires selected capabilities 10 and 42. Membership comes from the fenced durable party owner; the caller cannot supply member identities. All counters concern each member's current analyser connection interval. Retiring/replacing the connection resets its counters. Offline or unselected member measurements are explicitly unavailable. Counters are advisory and never move currency or items.

## Request

Little endian: price count u16 (0..2048), then unique nonzero item u32, flags u8 (1=loot price, 2=supply price, 3=both), and each present price as u64 in loot/supply order. Maximum 43010 bytes. Item references must exist in the requester's pinned generation. Currency denomination comes from the observed member's pinned item index; currency loot is already included in gold. Custom prices apply only when both generations agree on exact item definition and revision. Zero is an explicit price.

## Reply

Member count u8 (0..50). Each member: opaque party handle u64, UTF-8 name length u8 and 1..64 name bytes, flags u8 (1=measurements available, 2=incomplete), followed by twelve u64 fields: elapsed seconds, kills, gained XP, raw XP, looted item count, valued loot including gold, supply count, supply cost, unpriced loot count, unpriced supply count, damage dealt, healing given. Each row ends with needed-price count u8 (0..32), followed by unique item u32 and flags u8 (1=loot, 2=supply, 3=both). Refreshing after pricing reveals the next missing items if more remain. These references are returned only when the requester and member generations agree. Maximum 16551 bytes. Handles are connection-scoped existing party handles; character/account/session identities never enter this payload.

Unknown flags, invalid UTF-8/control characters, duplicate references, truncation, trailing bytes and bounds violations fail closed. Unavailable members have zero counters and no incomplete flag. Each live stream retains at most 16384 distinct loot references and 16384 supply references. Counter overflow, omitted detail or incompatible item identities marks measurements incomplete. The client must not suggest settlements when any member is unavailable, incomplete or unpriced. Pending fact queue loss does not discard these independent server counters. Splits are local suggestions requiring explicit player transfers; rounding conserves the total balance exactly. Integer remainders follow character-name order, so different viewers agree despite their different opaque handles.
