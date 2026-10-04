# MAP-OVERLAY-1a channel overlay budget (ADR-0021 §4.4, §4.8)

- Task: MAP-OVERLAY-1a (`docs/architecture/reviews/OTERYN_GAME_SOCIAL_MAP_PACKETS_2026-10-04.md` §2.5)
- Date: 2026-10-04
- Row: `MAP01-CHANNEL-OVERLAY-BYTES` (`docs/contracts/RESOURCE_LIMITS_REGISTRY.json`)
- Result: **confirmed at 64 MiB (67,108,864 B); no revision.**

## Accounting

`apps/game-server/src/map/overlay.rs` gates on accounted bytes, not on an allocator query. Each
charge is a deterministic, conservative bound of the heap the change can hold:

| Charge | Bytes | Bound |
|---|---|---|
| overlay fixed | 4,096 | minimum allocations and group padding of the tables |
| tile record | 3 × (slot + 1) + 4 × `AddedEntry` | hash-table slot at 7/8 load with power-of-two growth; the minimum 4-entry `Vec` |
| added entry | 2 × `AddedEntry` + payload capacity | `Vec` doubling; attribute bytes, or the boxed `GroundItemInstance` and its strings and byte vectors |
| expiry slot | 4 × key (24 B) | B-tree nodes at least half full |
| Ground index slot | 3 × (slot + 1) | hash-table slot, as for a tile |
| hidden origin | 8 | one bitmask bit, never free |

A volatile entry or freeze-time hide is admitted only if `used + cost <= budget`, computed before
anything is allocated or mutated, so a refusal leaves the overlay unchanged. Durable Ground items
and rebuild re-hides are always admitted; an admission that lands over the budget increments the
alarm counter. Removal, unhide and expiry refund exactly what was charged.

## Measurement

`map_overlay_budget_measure` (`apps/game-server/tests/map_overlay_channel.rs`, ignored, run by
hand: `cargo test --release --locked -p oteryn-game-server --test map_overlay_channel
map_overlay_budget_measure -- --ignored --nocapture`). One overlay at the default budget over the
fixture's 1,024-tile sector; volatile entries with 0-23 attribute bytes round-robin over every
tile, one in three with a decay deadline, until the first refusal. Resident growth is `VmRSS`
after the fill minus before. x86_64, 4 vCPU, release.

| Run | Entries (decaying) | Accounted | Resident growth | Ratio | Fill | Expire all |
|---|---|---|---|---|---|---|
| 1 | 389,018 (129,672) | 67,108,695 B | 49,180,672 B | 0.733 | 92 ms | 170 ms |
| 2 | 389,018 (129,672) | 67,108,695 B | 49,180,672 B | 0.733 | 113 ms | 185 ms |
| 3 | 389,018 (129,672) | 67,108,695 B | 49,180,672 B | 0.733 | 116 ms | 158 ms |

- The next volatile entry is refused 169 B short of 64 MiB, and the resident heap stays at 73% of
  the accounted bytes in this fill.
- Then 1,024 durable re-hides and 250 Ground items are all admitted over the full budget; the
  overlay reports `over_budget` and 1,253 alarms.
- About 172 accounted bytes per volatile entry, against the ADR-0021 §3 estimate of 45-80 B per
  dropped item. The difference is the conservative bound; at it, 64 MiB still holds about 390,000
  volatile entries per channel.

Removal, unhide and expiry also give back capacity: a tile's `Vec` of added entries is shrunk once
it holds more than the 4 + 2 per entry slots charged for it, and the tile and Ground tables once
they hold more than 2 per element plus 16 (the remainder is in the fixed charge). The B-tree frees
its nodes itself. `ChannelOverlay::capacity_within_charge` checks this, so after any sequence of
removals `used_bytes` still bounds what the overlay retains, not only its peak.

## Tests

`cargo test --locked -p oteryn-game-server map_overlay` covers the packet acceptance: two
channels share one base with separate overlays; hide and add up to ordinal 63, ordinal 64 refused,
a 65th base entry refused at load; no merge with a base stack; expiry within 1 s of the decay and
never before; atomic volatile refusal at the budget; capacity given back when a hidden tile is filled to
the budget and emptied tile after tile, and when tile records are released; durable admission over it with the alarm;
the Ground rebuild of every item, failing closed on a `map_revision` mismatch, another World or
Channel, a bad or unmapped position, or a duplicate.
