# TRAVEL-CONTENT-1: Travel route admission and fare evidence

Current NPC pages, Boat and Magic Carpet evidence are added as a sidecar admission catalogue. Admission stays false until CP resolves exact instance origins, destination tiles, conditions and conflicts. In particular, Anderson currently advertises 10 gp to Folda/Vega, while the existing route candidates say 20 gp.

Source registry: [sources.json](sources.json). Facts link to exact revision IDs or immutable repository blobs. `PROVEN` means the cited source states the fact; `DERIVED` means the stated transformation is reproducible; `UNKNOWN` and `CONFLICT` must remain held. Community evidence is not controlled Global telemetry. Official characters/combat/trading manual sources, where cited, were re-read live on 2026-10-18 and retain response digests. Pinned client assets carry their own version/continuity qualification.

These files prepare Reference data for CP. They add no runtime admission, Rust change, generated content regeneration, or interaction/world-pin mutation. `POPULATED` describes the evidence packet, not complete gameplay readiness. The packet schema is an evidence envelope, not a newly accepted runtime schema.

- Coverage is every existing route across all 56 services, not a claim that these are every Global route. Existing audit holds, scripted routes, discounts and Premium booleans are preserved; newly found price disagreements are explicit CONFLICTs.

Service data placement: **STAGED_FOR_COORDINATOR**. The World Bundle fingerprint includes every tracked `content/services/` file. Updating pins/regenerating content is forbidden in this task. Therefore the proposed files are under `target-files/`, with their intended canonical paths and SHA-256 in [service-apply-plan.json](service-apply-plan.json). CP/coordinator applies them together with qualified regeneration; no current generated Service declaration is changed. This is a concrete data handoff, not a claim that the canonical Service family is populated.

## Supplemental source research — 2026-10-18

All 56 native travel services and 195 routes remain covered. Supplemental current sources establish Premium-city access prerequisites, Gray Island fares, the 24-hour honeymoon window, local-ferry transport, placement-specific Junkar fares/world-change conditions and Hawkhurst/Tonar/Gnomprona access unlocks. Source-described NPC standing tiles are never used as arrival tiles. Four Junkar table/NPC fare disagreements remain explicit CONFLICT. Quest-start level 250 for Ingol is retained separately from an unproven post-unlock ferry level check. Historical Kendra anniversary travel is marked historical.

This is a sourced reference handoff, not a claim of complete gameplay knowledge. The field-level [research audit](research-audit.json) enumerates all remaining UNKNOWN/CONFLICT fields. Runtime admission stays blocked pending CP qualification.
