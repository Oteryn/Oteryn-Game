# TRAVEL-CONTENT-1: Travel route admission and fare evidence

Current NPC pages, Boat and Magic Carpet evidence are added as a sidecar admission catalogue. Admission stays false until CP resolves exact instance origins, destination tiles, conditions and conflicts. In particular, Anderson currently advertises 10 gp to Folda/Vega, while the existing route candidates say 20 gp.

Source registry: [sources.json](sources.json). Facts link to exact revision IDs or immutable repository blobs. `PROVEN` means the cited source states the fact; `DERIVED` means the stated transformation is reproducible; `UNKNOWN` and `CONFLICT` must remain held. Community evidence is not controlled Global telemetry. Official characters/combat/trading manual sources, where cited, were re-read live on 2026-10-18 and retain response digests. Pinned client assets carry their own version/continuity qualification.

These files prepare Reference data for CP. They add no runtime admission, Rust change, generated content regeneration, or interaction/world-pin mutation. `POPULATED` describes the evidence packet, not complete gameplay readiness. The packet schema is an evidence envelope, not a newly accepted runtime schema.

- Coverage is every existing route across all 56 services, not a claim that these are every Global route. Existing audit holds, scripted routes, discounts and Premium booleans are preserved; newly found price disagreements are explicit CONFLICTs.

Service data placement: **STAGED_FOR_COORDINATOR**. The World Bundle fingerprint includes every tracked `content/services/` file. Updating pins/regenerating content is forbidden in this task. Therefore the proposed files are under `target-files/`, with their intended canonical paths and SHA-256 in [service-apply-plan.json](service-apply-plan.json). CP/coordinator applies them together with qualified regeneration; no current generated Service declaration is changed. This is a concrete data handoff, not a claim that the canonical Service family is populated.

## Research completion and coverage

All 56 original travel services and 195 route declarations are covered. Current NPC fares, directional carpet fares, explicit quest conditions, Free mainland boat eligibility and Premium carpet rules are extracted separately. Native historical candidates remain visible. Multiple NPC placements, omitted/obsolete route keywords and undocumented arrival tiles remain explicitly held. An NPC standing tile is never substituted for a destination arrival tile.

The requested data/evidence preparation is delivered with explicit evidence holds, as required by the owner instruction “If a value cannot be proven, write it with classification UNKNOWN and a note.” A held fact is not runtime-ready. [research-audit.json](research-audit.json) enumerates every UNKNOWN/CONFLICT field path, observed candidate, source and limitation so CP can distinguish unavailable evidence from a missing packet. No runtime admission or integration is authorized.
