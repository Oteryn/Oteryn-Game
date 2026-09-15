# Oteryn Game Next-Wave First-Slice Resource Limits Decision — 2026-08-25

Status: **ACCEPTED CANDIDATES FOR SERIALIZED REGISTRATION**

Authority: Issues #128, #131 and #133. This packet selects reversible first-slice safety ceilings only; it does not select production sizing, gameplay balance, Reference parity, ports, secrets, deployment, or implementation behavior.

## Decision summary

The bounded evidence task accepts 24 finite candidate limits for the exact Ability, Interaction, AI and TCP/TLS admission/backpressure first slices. Thirty-three inventory rows that are unreachable in those slices are explicitly `NOT_APPLICABLE_TO_FIRST_SLICE` and must fail closed. The Durability child is intentionally journal-only, so all eight DUR03 transaction-cardinality rows are excluded until a later transaction-capable child exists. Movement remains unallocated; Issue #139 owns its non-current remaining rows.

- Evidence: `docs/agents/evidence/OTV2-20260825-next-wave-limit-evidence.json`
- Evidence SHA-256: `2d6db30655c33408b8e48f80b827f864e4c7efd331ebc5cc37437a1d1851d0da`
- Source main / worker base: `86653375231febbf81623b4c6984a6ff1263bdc2` (allocation PR #137 merge)
- Harness: `tools/next-wave-limit-evidence/main.rs` (Rust/std only)
- TDD: observed compile/test RED before each generic boundary, overflow, Ability, Interaction, AI, listener, exclusion, metadata, margin, JSON and stress capability; final suite is 11/11 GREEN under `rustc -D warnings`.
- Optimized stress observation on Molehill-PC: 64 iterations, 1,536 accepted max cases, 1,536 rejected max+1 cases, 1,048,576-byte peak single fixture allocation, deterministic checksum `14695981039346656037`, observed wall time `72.756 ms`. Timing is evidence about the fixture only and is not a production SLO or tuning default.
- Every candidate JSON record carries `allocation_impact`, `client_visible`, explicit `boundary_tests`, and `evidence_basis`, so the serialized registry worker must copy rather than reinterpret the decision.

## Frozen first slices

- **Ability:** one explicit target, one immediate typed damage/heal occurrence and one bounded staged plan; no area/chain/retarget/future/conditions/reactions/cross-domain/scripts.
- **Interaction:** one root with immediate bounded children only; no grandchildren, foreign delegated owner operations, or automatic retries.
- **AI:** fixed acquire-or-idle authored representation with bounded perception and at most one path proposal per actor; no spawn/timers/memory/repath/controlled backlog/scripts.
- **Durability:** Foundation authority-journal/reconciliation receipt substrate only; no item/value/transform/container/workflow transaction entry point exists in this child.
- **Server Seam:** TCP/TLS admission, bounded handshake/auth concurrency, outbound queue/backpressure and drain accounting only; no bind/port/certificate/key/deployment/listener implementation.

## Accepted candidate limits

| Candidate ID | Inventory row | Domain | Resource | Unit | Representative | Hard maximum | Margin | Max cost (retained B / work) |
|---|---|---|---|---|---:|---:|---:|---:|
| `ABILITY01-TARGET-CANDIDATES` | `AB-RL-01` | Ability | target candidates examined by one explicit resolution step | candidate entities/positions | 1 | **2** | 2x | 128 / 3 |
| `ABILITY01-RESOLVED-TARGETS` | `AB-RL-02` | Ability | resolved targets for one immediate ability occurrence | targets | 1 | **2** | 2x | 128 / 5 |
| `ABILITY01-EFFECT-PLAN-ENTRIES` | `AB-RL-05` | Ability | typed entries in one staged Effect Plan | typed plan entries | 1 | **2** | 2x | 768 / 10 |
| `ABILITY01-EFFECT-PLAN-BYTES` | `AB-RL-06` | Ability | retained encoded/in-memory bytes for one staged Effect Plan | bytes | 1024 | **4096** | 4x | 4096 / 8 |
| `ABILITY01-CALC-STAGES` | `AB-RL-07` | Ability | typed calculation/contribution stages per occurrence | stages | 4 | **8** | 2x | 640 / 17 |
| `INTERACTION01-CASCADE-DEPTH` | `GI-RL-01` | Interaction | child ancestry levels per interaction root | child ancestry levels | 1 | **2** | 2x | 192 / 3 |
| `INTERACTION01-CHILD-FANOUT` | `GI-RL-02` | Interaction | immediate child occurrences per parent | child occurrences | 4 | **8** | 2x | 1152 / 17 |
| `INTERACTION01-ROOT-WORK` | `GI-RL-03` | Interaction | total immediate descendant work per root | descendant work units | 4 | **8** | 2x | 1280 / 33 |
| `INTERACTION01-TRIGGER-CANDIDATES` | `GI-RL-06` | Interaction | eligible trigger/edge candidates per source occurrence | candidates | 8 | **16** | 2x | 1088 / 33 |
| `INTERACTION01-RETAINED-CHILD-LIFECYCLES` | `GI-RL-07` | Interaction | retained child lifecycle entries per root | child lifecycle entries | 4 | **8** | 2x | 1664 / 9 |
| `AI01-ACTIVE-ACTORS` | `AI-RL-01` | AI | active AI actors per authoritative scope safety envelope | actors | 128 | **256** | 2x | 65536 / 256 |
| `AI01-AUTHORED-UNITS` | `AI-RL-02` | AI | authored acquire-or-idle representation units | authored units | 2 | **4** | 2x | 320 / 5 |
| `AI01-EVALUATION-WORK` | `AI-RL-03` | AI | semantic AI evaluation work per resolution | deterministic work units | 4 | **8** | 2x | 640 / 17 |
| `AI01-PERCEPTION-CANDIDATES` | `AI-RL-05` | AI | perception/target candidates per decision | candidates | 32 | **64** | 2x | 2176 / 129 |
| `AI01-PATH-REQUESTS-PER-ACTOR` | `AI-RL-07` | AI | queued/in-flight path requests per actor | requests per actor | 1 | **2** | 2x | 512 / 2 |
| `AI01-PATH-SEARCH-WORK` | `AI-RL-08` | AI | path-search nodes/work units per request | nodes/work units | 512 | **1024** | 2x | 49408 / 1024 |
| `AI01-ROUTE-STEPS` | `AI-RL-09` | AI | route steps retained in one path proposal | route steps | 64 | **128** | 2x | 4160 / 128 |
| `AI01-ROUTE-BYTES` | `AI-RL-09` | AI | aggregate retained route bytes in one path proposal | bytes | 2048 | **4096** | 2x | 4096 / 4 |
| `NET03-PREADMISSION-CONNECTIONS` | `NET03-RL-01` | Server Seam | concurrent pre-admission TCP/TLS connections | connections | 128 | **256** | 2x | 524288 / 256 |
| `NET03-HANDSHAKE-AUTH-WORK` | `NET03-RL-02` | Server Seam | concurrent TLS handshake/authentication work | concurrent handshakes | 32 | **64** | 2x | 1048576 / 256 |
| `NET03-OUTBOUND-QUEUE-ENTRIES` | `NET03-RL-04` | Server Seam | outbound queued entries per admitted session | entries per session | 32 | **64** | 2x | 4224 / 65 |
| `NET03-OUTBOUND-QUEUE-BYTES` | `NET03-RL-05` | Server Seam | outbound queued bytes per admitted session | bytes per session | 524288 | **1048576** | 2x | 1048576 / 4 |
| `NET03-PENDING-WRITES` | `NET03-RL-06` | Server Seam | pending transport writes per session | writes per session | 4 | **8** | 2x | 1088 / 17 |
| `NET03-DRAIN-TASKS` | `NET03-RL-07` | Server Seam | connection/task shutdown and drain work per batch | tasks per drain batch | 128 | **256** | 2x | 16512 / 257 |

Each row is accepted only as a hard safety ceiling for the frozen first slice. Configuration may be narrower but may not exceed the listed maximum. `max` is accepted, `max+1` returns `CAPACITY_EXCEEDED` before unchecked allocation or partial mutation, and cumulative retained/work arithmetic is checked for overflow.

## Exact inherited listener resource

`NET03-RL-03` does not create a new limit. It consumes the exact existing `FND02-WIRE-FRAME-BYTES` maximum of **1048576 bytes** because first listener retains at most one already FND-02-bounded WireEnvelope frame per connection and introduces no second application assembly buffer. A second listener-local application assembly buffer would invalidate this equivalence and require a new decision.

## Explicit fail-closed exclusions

| Inventory row | Domain | Disposition | Reason |
|---|---|---|---|
| `AB-RL-03` | Ability | `NOT_APPLICABLE_TO_FIRST_SLICE` | explicit-target slice performs no geometry or spatial candidate query |
| `AB-RL-04` | Ability | `NOT_APPLICABLE_TO_FIRST_SLICE` | dynamic retargeting is disabled in the first slice |
| `AB-RL-08` | Ability | `NOT_APPLICABLE_TO_FIRST_SLICE` | multi-hit and multi-target sub-occurrences are disabled |
| `AB-RL-09` | Ability | `NOT_APPLICABLE_TO_FIRST_SLICE` | channel and periodic future occurrences are disabled |
| `AB-RL-10` | Ability | `NOT_APPLICABLE_TO_FIRST_SLICE` | no future ability work may be enqueued |
| `AB-RL-11` | Ability | `NOT_APPLICABLE_TO_FIRST_SLICE` | catch-up scheduling is unreachable without future work |
| `AB-RL-12` | Ability | `NOT_APPLICABLE_TO_FIRST_SLICE` | conditions are outside the first slice |
| `AB-RL-13` | Ability | `NOT_APPLICABLE_TO_FIRST_SLICE` | scheduled condition work is outside the first slice |
| `AB-RL-14` | Ability | `NOT_APPLICABLE_TO_FIRST_SLICE` | post-commit reactions are disabled |
| `AB-RL-15` | Ability | `NOT_APPLICABLE_TO_FIRST_SLICE` | reaction descendants are disabled |
| `AB-RL-16` | Ability | `NOT_APPLICABLE_TO_FIRST_SLICE` | aggregate reaction/future work is unreachable |
| `AB-RL-17` | Ability | `NOT_APPLICABLE_TO_FIRST_SLICE` | cross-domain proposals are disabled |
| `AB-RL-18` | Ability | `NOT_APPLICABLE_TO_FIRST_SLICE` | variable diagnostic payloads are disabled; fixed counters only |
| `AB-RL-19` | Ability | `NOT_APPLICABLE_TO_FIRST_SLICE` | script-backed mechanics are disabled |
| `GI-RL-04` | Interaction | `NOT_APPLICABLE_TO_FIRST_SLICE` | foreign delegated owner operations are disabled |
| `GI-RL-05` | Interaction | `NOT_APPLICABLE_TO_FIRST_SLICE` | automatic reconciliation and retry execution are disabled |
| `AI-RL-04` | AI | `NOT_APPLICABLE_TO_FIRST_SLICE` | threat, stimulus and memory collections are disabled |
| `AI-RL-06` | AI | `NOT_APPLICABLE_TO_FIRST_SLICE` | AI timers and delayed operations are disabled |
| `AI-RL-10` | AI | `NOT_APPLICABLE_TO_FIRST_SLICE` | repath and retry windows are disabled |
| `AI-RL-11` | AI | `NOT_APPLICABLE_TO_FIRST_SLICE` | spawn sources and population mutation are disabled |
| `AI-RL-12` | AI | `NOT_APPLICABLE_TO_FIRST_SLICE` | spawn placement search is disabled |
| `AI-RL-13` | AI | `NOT_APPLICABLE_TO_FIRST_SLICE` | postponed occupancy retries are disabled |
| `AI-RL-14` | AI | `NOT_APPLICABLE_TO_FIRST_SLICE` | controlled-actor command backlog is disabled |
| `AI-RL-15` | AI | `NOT_APPLICABLE_TO_FIRST_SLICE` | script-backed AI is disabled |
| `AI-RL-16` | AI | `NOT_APPLICABLE_TO_FIRST_SLICE` | variable replay/diagnostic payloads are disabled; fixed counters only |
| `DUR03-RL-01` | Durability | `NOT_APPLICABLE_TO_FIRST_SLICE` | journal-only first slice exposes no item/value transaction entry; transaction work fails closed |
| `DUR03-RL-02` | Durability | `NOT_APPLICABLE_TO_FIRST_SLICE` | journal-only first slice exposes no item/value transaction entry; transaction work fails closed |
| `DUR03-RL-03` | Durability | `NOT_APPLICABLE_TO_FIRST_SLICE` | journal-only first slice exposes no item/value transaction entry; transaction work fails closed |
| `DUR03-RL-04` | Durability | `NOT_APPLICABLE_TO_FIRST_SLICE` | journal-only first slice exposes no item/value transaction entry; transaction work fails closed |
| `DUR03-RL-05` | Durability | `NOT_APPLICABLE_TO_FIRST_SLICE` | journal-only first slice exposes no item/value transaction entry; transaction work fails closed |
| `DUR03-RL-06` | Durability | `NOT_APPLICABLE_TO_FIRST_SLICE` | journal-only first slice exposes no item/value transaction entry; transaction work fails closed |
| `DUR03-RL-07` | Durability | `NOT_APPLICABLE_TO_FIRST_SLICE` | journal-only first slice exposes no item/value transaction entry; transaction work fails closed |
| `DUR03-RL-08` | Durability | `NOT_APPLICABLE_TO_FIRST_SLICE` | journal-only first slice exposes no item/value transaction entry; transaction work fails closed |

No excluded row may silently become reachable. A child that adds the excluded behavior must first obtain an accepted finite hard maximum (or a newly justified explicit exclusion) and, where a new resource exists, a separately serialized registry mutation.

## Movement disposition

Issue #139 is the dedicated non-current successor for `MOVE-RL-02..11`, `MOVE-RL-16`, and `MOVE-RL-17`. It grants no Movement implementation authority. Existing exact Foundation rows `MOVE-RL-01`, `MOVE-RL-12`, `MOVE-RL-13`, `MOVE-RL-14`, and `MOVE-RL-15` remain inherited unchanged. Movement stays blocked on an exact child plan plus its non-resource predecessors (including compatible Interaction, Client, and real QA evidence).

## Registry handoff

This packet is not the canonical registry. A later one-writer registry task must copy the 24 accepted candidate values exactly, preserving units, hard maxima, configurable ranges, `CAPACITY_EXCEEDED` failure semantics, pre-allocation rejection, client visibility, and max/max+1/overflow boundary obligations. It must not invent a production default, gameplay value, wire/error numeric identifier, or value for any excluded row.

## Blocker effect after registration

- #93: Ability/Interaction/AI resource decision portion becomes satisfiable after exact registration; Movement-only remainder is durably moved to non-current #139.
- #116: listener resource decision becomes satisfiable after exact registration plus the inherited FND-02 equivalence above; no listener implementation is authorized.
- #123: the exact first Durability child remains journal-only and all DUR03 transaction rows are explicitly fail-closed excluded, so no transaction hard maximum is exercised by that child. A later transaction-capable child must reopen applicable DUR03 decisions before allocation.

## Validation posture

Decision acceptance requires the checked-in harness/evidence/task/packet diff to pass local governance, architecture checks, `git diff --check`, whole-diff self-review and exact-head GitHub CI including `game-gate`. Until that PR merges, these values remain evidence candidates and must not be consumed as registered maxima.
