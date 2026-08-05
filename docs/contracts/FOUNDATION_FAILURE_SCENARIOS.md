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

New scenarios receive stable IDs and are added here rather than being named differently by each package.
