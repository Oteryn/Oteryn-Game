# Potion durations V1 (NX-8 W09)

Capability 381, command 662, read-only admitted session query. Bundle-world offer.

## Request

Empty. A nonempty request, unavailable owner or stale exact actor/session/lease fence is rejected. The server proves the live actor both before and after reading durable effects. The result contains only the admitted character's active effects.

## Reply

Accepted result: one count byte (0–32), followed by that many 15-byte entries:
source u8, item id u32 LE, bonus percent u16 LE, remaining milliseconds u64 LE.
No trailing bytes. A positive duration is required. Source 1 identifies Drome online-time effects by donor item id (36724–36742, except instant Stamina Extension 36725); source 2 identifies timed potion conditions (7439, 7440, 7443, 49271, 35563); source 3 is store XP boost (item 0, 50%); source 4 is daily reward XP boost (item 0, 10/20/25/50%). Other sources/identities are malformed.

Wealth Duplex uses the same one-hour online-time calculation as the loot owner, never wall expiry. XP sources use their active database-clock intervals. Winter Update 2026 (thread 565): the Store XP boost (source 3) and the Daily Reward XP boost (source 4) are at most one entry each, and their percentages add in the XP formula (a Store 50% and a daily 50% give +100%). A daily claim starts its boost at the claim and replaces the remainder of any earlier daily boost: source 4 reports only the newest daily claim's boost, and a daily claim never extends the Store timer. Source 3 reports the highest active Store percentage with the latest active expiration. The wire is unchanged. Queued and expired grants are excluded. Potion conditions use the live condition owner's remaining semantic duration. Zero active entries clears earlier timers. The client starts its monotonic countdown when the reply arrives, hides expired entries, and refreshes while the native panel is open; a refusal does not restart old timers.

Kooldown Aid and Stamina Extension are instantaneous and have no ongoing effect timer. The catalogue can decode every timed Drome identity. At this base only Wealth Duplex has an active durable Drome owner; the other identities have no fabricated timer or activation. NX-8 adds display/projection only; it does not invent missing combat effects.
