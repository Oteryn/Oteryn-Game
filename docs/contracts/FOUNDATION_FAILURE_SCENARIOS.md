# Foundation Failure Scenario Catalogue

Status: normative scenario names; owning contracts define exact timings and expected codes.

Each foundation contract must mark every applicable scenario as `PASS`, `NOT_APPLICABLE`, `BLOCKED` or `DEFERRED_BY_ACCEPTED_GATE` with named evidence.

| ID | Scenario | Minimum invariant |
|---|---|---|
| `FS-PLATFORM-UNAVAILABLE` | Platform unavailable during login/admission | No alternate credential authority or unbound session is created. |
| `FS-GATEWAY-AFTER-REDEEM` | Gateway fails after one-time ticket redemption | No second candidate or silent downgrade; result is bounded and auditable. |
| `FS-POSTGRES-UNAVAILABLE` | Game PostgreSQL unavailable | No unfenced durable mutation; admission/runtime policy is explicit. |
| `FS-LEASE-RENEW-TIMEOUT` | Character lease renewal deadline expires | Stale owner stops authoritative durable writes before another generation can commit. |
| `FS-DUPLICATE-LOGIN` | Two admissions race for one character | At most one authoritative session and generation wins atomically. |
| `FS-STALE-GENERATION` | Old session/node submits command or save | Rejected with no partial mutation. |
| `FS-DUPLICATE-COMMAND` | Same `CommandId` is replayed | Deterministic prior result or explicit duplicate outcome; no duplicated effect. |
| `FS-CHANNEL-SPLIT-OWNER` | Two nodes believe they own one channel | Fencing prevents dual authoritative commits. |
| `FS-CHANNEL-DRAIN` | Channel drains during active gameplay | New work stops in order; admitted state reaches a documented safe boundary. |
| `FS-QUEUE-SATURATION` | Bounded inbound/outbound/work queue fills | Defined backpressure/rejection; no silent loss or unbounded growth. |
| `FS-SLOW-CLIENT` | Client cannot consume outbound state | Bounded memory and explicit disconnect/resync behavior. |
| `FS-CLOCK-SKEW` | Wall clocks disagree or move | Monotonic deadlines remain safe; signed timestamps use bounded skew policy. |
| `FS-KEY-ROTATION` | Signing/verification key rotates during login | Valid overlap and revocation behavior; no acceptance outside policy. |
| `FS-REVISION-MISMATCH` | Protocol/ruleset/content revisions disagree | Fail closed with no implicit downgrade or mixed authoritative state. |
| `FS-SNAPSHOT-DELTA-MISMATCH` | Snapshot and delta revisions diverge | Deterministic resync; no partial application. |
| `FS-DB-OUTBOX-BOUNDARY` | Crash occurs around durable mutation/outbox publish | Transaction contract prevents lost or duplicated externally visible effects. |
| `FS-WORLD-BUNDLE-CORRUPT` | Bundle checksum/version/decompression is invalid | Reject before unsafe allocation or partial world activation. |
| `FS-CLIENT-CUTOVER-ROLLBACK` | Migrated client workspace fails acceptance | Development source of truth and exact rollback path remain unambiguous. |
| `FS-ANALYTICS-TELEMETRY-OVERFLOW` | Best-effort gameplay telemetry queue reaches its registered bound | Gameplay follows the explicit fail-open/drop policy; loss is counted, bounded and never misrepresented as complete evidence. |
| `FS-AUDIT-OUTBOX-BACKLOG` | Durable audit/outbox consumer is unavailable or falls behind | Authoritative records are not silently discarded; backlog, recovery and owning transaction behavior follow the accepted durable contract. |
| `FS-EVENT-DUPLICATE-DELIVERY` | One event is delivered more than once | Consumer deduplication returns one analytical/provenance effect and never replays gameplay mutation. |
| `FS-EVENT-OUT-OF-ORDER` | Related events arrive outside causal or transaction order | Consumer buffers, rejects or reconciles deterministically without inventing state or enforcement. |
| `FS-AUDIT-MUTATION-MISMATCH` | Authoritative mutation and required audit/outbox evidence disagree | The mismatch cannot commit or remain silent; recovery produces named evidence and preserves authoritative invariants. |
| `FS-ANALYTICS-PRIVACY-POLICY` | Event/dataset lacks an accepted purpose, privacy class, retention or access policy | Production collection/projection fails closed for that class with no unclassified player-linked data. |
| `FS-DETECTOR-FALSE-POSITIVE` | Analytics detector raises an incorrect security anomaly | No automatic sanction or mutation occurs; evidence, detector version, human review and disposition remain auditable. |
| `FS-INVESTIGATION-MUTATION-ATTEMPT` | Investigation or AI component attempts a runtime/database mutation | Least-privilege credentials make the mutation impossible and the attempt is audited. |

New scenarios receive stable IDs and are added here rather than being named differently by each package.
