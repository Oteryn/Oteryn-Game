# BLESSINGS-CONTENT-1: Blessing catalogue, prices and loss evidence

Eight normal/PvP blessing entries plus the distinct Inquisition bundle and Adventurer protection are recorded. The Spark of the Phoenix requires two ordered NPC visits. Prices are transcribed from the current Wiki with the above-120 official-source disagreement held. Death-related arithmetic uses official evidence where available and never silently equates item protection with XP protection.

Source registry: [sources.json](sources.json). Facts link to exact revision IDs or immutable repository blobs. `PROVEN` means the cited source states the fact; `DERIVED` means the stated transformation is reproducible; `UNKNOWN` and `CONFLICT` must remain held. Community evidence is not controlled Global telemetry. Where cited, the official characters/combat/trading manual was re-read live on 2026-10-18; exact response digests and current URLs are in the source registry.

These files prepare Reference data for CP. They add no runtime admission, Rust change, generated content regeneration, or interaction/world-pin mutation. `POPULATED` describes the evidence packet, not complete gameplay readiness. The packet schema is an evidence envelope, not a newly accepted runtime schema.

Service data placement: **STAGED_FOR_COORDINATOR**. The World Bundle fingerprint includes every tracked `content/services/` file. Updating pins/regenerating content is forbidden in this task. Therefore the proposed files are under `target-files/`, with their intended canonical paths and SHA-256 in [service-apply-plan.json](service-apply-plan.json). CP/coordinator applies them together with qualified regeneration; no current generated Service declaration is changed. This is a concrete data handoff, not a claim that the canonical Service family is populated.

## Research completion and coverage

The seven ordinary blessings, Twist of Fate, Inquisition bundle and Adventurer protection are covered with NPCs/locations and available price rules. Reduction tables are derived from the live official manual. Conflicting above-120 regular prices, Retro Hardcore community wording and unsupported promotion-specific purchase discounts remain explicit.

The requested data/evidence preparation is delivered with explicit evidence holds, as required by the owner instruction “If a value cannot be proven, write it with classification UNKNOWN and a note.” A held fact is not runtime-ready. [research-audit.json](research-audit.json) enumerates every UNKNOWN/CONFLICT field path, observed candidate, source and limitation so CP can distinguish unavailable evidence from a missing packet. No runtime admission or integration is authorized.

## Owner pricing ruling (2026-10-18)

The owner ruled the blessing prices on 2026-10-18 (PR #2194, PREM-2b+PREM-5). The ruled facts carry the classification `PROVEN_BY_OWNER` and an `owner_ruling` object. Their sources are the ruling (`owner:ruling:2026-10-18`) and the owner-provided TibiaMonk calculator cross-reference, fetched 2026-10-18. That page computes prices client-side, so it is not an independent source.

- The five regular blessings cost 2000 up to level 30, 2000+200·(L−30) for 30 < L < 120, and 20000+75·(L−120) from level 120. This resolves the above-120 `CONFLICT` for the Wiki formula. The official manual says regular prices stay flat past 120, and that wording is not selected.
- Blood and Heart of the Mountain cost 2600 up to level 30, 2600+260·(L−30) for 30 < L < 120, and 26000+100·(L−120) from level 120.
- Twist of Fate costs 2000 up to level 30, 2000+200·(L−30) for 30 < L < 270, and 50000 from level 270. This is data only.
- A promotion has no blessing price effect.
- The Store prices in Tibia Coins are recorded as data only:
  - all seven: 130;
  - Twist of Fate: 8;
  - Blood or Heart of the Mountain: 25 each;
  - The Spark of the Phoenix: 20;
  - each of the other four regular blessings: 15.

The Wiki revision `wiki:1630:1113782` states the same formulas. Its `200*(L-20)` and `260*(L-20)` equal the ruled forms.

PREM-5 admits only the NPC prices of The Wisdom of Solitude, Blood of the Mountain and Heart of the Mountain.

The official Premium features page (`official:premiumfeatures:2026-10-18`) agrees that character promotion is Premium only, and it lists no blessing. It also attributes the 30% death penalty reduction to Premium, while this packet attributes it to the Premium-gated promotion. That difference is recorded, not resolved.

The staged Service files and the World Bundle declarations are unchanged.
