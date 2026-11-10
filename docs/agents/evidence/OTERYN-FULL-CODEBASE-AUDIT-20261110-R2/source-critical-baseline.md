# Source critical baseline — REVIEWED

Base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. All two tracked package files (145 lines) substantively reviewed.

Entry points are `SourceCriticalBaseline::qualify`, `fingerprint_parts`, and `draw`. The closed key/chance table rejects unknown nonzero profiles. Zero has a qualified 1% default only for auto attacks. Draws bind root, occurrence, purpose and source index, and the result explicitly has zero bonus; this is an accepted qualification boundary rather than a missing implemented bonus. Integer threshold conversion is exact for every allowed chance.

Cross-module check: `combat/src/creature_auto_attack.rs:351-369` resolves a current typed creature profile before qualification; `1187-1208` dispatches the owner-issued occurrence/index and preserves the zero bonus, with a separate Soulpit Overpower path. The event participates in hazard handling at `1257-1271`. Existing RNG-001 remains a separate root-secrecy question; this review does not claim to settle it.

Hypotheses rejected: arbitrary caller-supplied chance (closed constructor); replay draws producing new results (pure keyed draw); baseline unintentionally changing ordinary damage (explicit zero bonus). Existing tests check deterministic repeats, qualified keys, 1% and 10% sample ranges. Native tests were inspected, not executed. No new confirmed finding. No package files remain; production combat/persistence and statistical runtime qualification are outside this package completion.
