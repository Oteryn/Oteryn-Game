# MARKET-CONTENT-1: Market wares and sourced category evidence

Current own-item Wiki templates were revalidated against the latest API revisions, including seven changed templates and separately fetched missing item pages. Category assignment is an explicit conservative DERIVED crosswalk; unrecognized types remain UNKNOWN. Existing service prices and item definitions are preserved.

Source registry: [sources.json](sources.json). Facts link to exact revision IDs or immutable repository blobs. `PROVEN` means the cited source states the fact; `DERIVED` means the stated transformation is reproducible; `UNKNOWN` and `CONFLICT` must remain held. Community evidence is not controlled Global telemetry. Where cited, the official characters/combat/trading manual was re-read live on 2026-10-18; exact response digests and current URLs are in the source registry.

These files prepare Reference data for CP. They add no runtime admission, Rust change, generated content regeneration, or interaction/world-pin mutation. `POPULATED` describes the evidence packet, not complete gameplay readiness. The packet schema is an evidence envelope, not a newly accepted runtime schema.

- Staged link evidence covers all 324 existing trade services and their exact offered Item keys; 2076 unique wares are covered. NPC saleability does not prove marketability. Unsupported categories, additional restrictions and 1H/2H filtering remain UNKNOWN.

Service data placement: **STAGED_FOR_COORDINATOR**. The World Bundle fingerprint includes every tracked `content/services/` file. Updating pins/regenerating content is forbidden in this task. Therefore the proposed files are under `target-files/`, with their intended canonical paths and SHA-256 in [service-apply-plan.json](service-apply-plan.json). CP/coordinator applies them together with qualified regeneration; no current generated Service declaration is changed. This is a concrete data handoff, not a claim that the canonical Service family is populated.

## Research completion and coverage

All 324 original trade services and 2076 distinct offered Item identities are covered. Current own-item Wiki observations and the Market rules are preserved; 1868 items also have positive raw category/market metadata from the pinned official 15.30 client asset. That asset is explicitly not represented as a fresh live-client capture. Missing flags never become false. Global instance exclusions are linked on every item; unidentified current filters and exhaustive item-instance restrictions remain held.

The requested data/evidence preparation is delivered with explicit evidence holds, as required by the owner instruction “If a value cannot be proven, write it with classification UNKNOWN and a note.” A held fact is not runtime-ready. [research-audit.json](research-audit.json) enumerates every UNKNOWN/CONFLICT field path, observed candidate, source and limitation so CP can distinguish unavailable evidence from a missing packet. No runtime admission or integration is authorized.
