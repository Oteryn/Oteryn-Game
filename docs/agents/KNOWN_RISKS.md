# Known architectural and delivery risks

- Greenfield repository paths and tooling may not yet exist; plans are not implementation evidence.
- Existing Oteryn Platform, Otheryn and otclient documents may still describe Canary-compatible or dual-protocol architecture that conflicts with Oteryn v2 ADR-0001.
- Character leases, stale-writer fencing and item transactions are prerequisite safety contracts, not later optimizations.
- A singleton world or process-global mutable game state would make multichannel retrofitting unsafe and expensive.
- Shared world services can create duplication/loss bugs at channel boundaries, especially loot, trade, market, rewards, PvP and houses.
- House topology is deliberately unresolved beyond one authoritative state per world and anti-duplication invariants.
- Reusing Lua/content without scoped execution can leak state across channels.
- Proprietary asset provenance may block redistribution even when technical conversion is possible.
- Client/server/protocol work can drift when exact producer/consumer revisions and golden fixtures are not pinned.
- CI green status without real two-channel, crash-recovery and user-observable E2E evidence is insufficient for production claims.
