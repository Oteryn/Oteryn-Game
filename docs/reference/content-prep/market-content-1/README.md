# MARKET-CONTENT-1: Market wares and sourced category evidence

Current own-item Wiki templates were revalidated against the latest API revisions, including seven changed templates and separately fetched missing item pages. Category assignment is an explicit conservative DERIVED crosswalk; unrecognized types remain UNKNOWN. Existing service prices and item definitions are preserved.

Source registry: [sources.json](sources.json). Facts link to exact revision IDs or immutable repository blobs. `PROVEN` means the cited source states the fact; `DERIVED` means the stated transformation is reproducible; `UNKNOWN` and `CONFLICT` must remain held. Community evidence is not controlled Global telemetry. Official manual observations retain their 2026-09-28 capture and unverified current continuity.

These files prepare Reference data for CP. They add no runtime admission, Rust change, generated content regeneration, or interaction/world-pin mutation. `POPULATED` describes the evidence packet, not complete gameplay readiness. The packet schema is an evidence envelope, not a newly accepted runtime schema.

- Staged link evidence covers all 324 existing trade services and their exact offered Item keys; 2076 unique wares are covered. NPC saleability does not prove marketability. Unsupported categories, additional restrictions and 1H/2H filtering remain UNKNOWN.

Service data placement: **STAGED_FOR_COORDINATOR**. The World Bundle fingerprint includes every tracked `content/services/` file. Updating pins/regenerating content is forbidden in this task. Therefore the proposed files are under `target-files/`, with their intended canonical paths and SHA-256 in [service-apply-plan.json](service-apply-plan.json). CP/coordinator applies them together with qualified regeneration; no current generated Service declaration is changed. This is a concrete data handoff, not a claim that the canonical Service family is populated.
