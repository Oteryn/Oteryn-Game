# ARENA-RULES-CONTENT-1: Tibiadrome scoring, rewards and PvP arena status

Current Tibiadrome scoring and reward costs are separated from the retired PvP Duelling Arena. The current Wiki marks duelling arenas permanently removed. No pve-catalog.json, creature statistics or arena-owner data is duplicated from occupied PR #2179.

Source registry: [sources.json](sources.json). Facts link to exact revision IDs or immutable repository blobs. `PROVEN` means the cited source states the fact; `DERIVED` means the stated transformation is reproducible; `UNKNOWN` and `CONFLICT` must remain held. Community evidence is not controlled Global telemetry. Where cited, the official characters/combat/trading manual was re-read live on 2026-10-18; exact response digests and current URLs are in the source registry.

These files prepare Reference data for CP. They add no runtime admission, Rust change, generated content regeneration, or interaction/world-pin mutation. `POPULATED` describes the evidence packet, not complete gameplay readiness. The packet schema is an evidence envelope, not a newly accepted runtime schema.

## Research completion and coverage

Current Tibiadrome admission, scoring, rotations, rewards and combat behavior are captured without duplicating the occupied arena catalog. Removed Duelling Arena behavior is labeled historical; whether Oteryn reintroduces it is an architecture decision outside this task.

The requested data/evidence preparation is delivered with explicit evidence holds, as required by the owner instruction “If a value cannot be proven, write it with classification UNKNOWN and a note.” A held fact is not runtime-ready. [research-audit.json](research-audit.json) enumerates every UNKNOWN/CONFLICT field path, observed candidate, source and limitation so CP can distinguish unavailable evidence from a missing packet. No runtime admission or integration is authorized.
