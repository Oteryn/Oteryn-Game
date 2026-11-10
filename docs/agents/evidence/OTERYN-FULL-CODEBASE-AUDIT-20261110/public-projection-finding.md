# SEC-003 — valid V2 public projections exceed the transport envelope

Base: `e2e5ced306040113dc9c395251a802f50eb5776a`.

- **Severity:** P2. This is a functional transport/publication defect, not an established security exploit.
- **Status:** CONFIRMED by static production call-path and bounds evidence.
- **Origin:** session/protocol worker's earlier report; lead independently opened and traced the relevant code after that worker became unavailable.
- **Runtime limitation:** no actual mTLS exchange, native codec test or PostgreSQL publisher test was executed.

## Verified path and wrong behavior

`crates/gameplay/native-admission-source/src/descriptor.rs:188-189` permits request bodies up to 32,768 bytes for `PublishCharacterDeathsV2` and 393,216 bytes for `PublishGuildV2`. The V2 encoder at `public_projections/v2.rs:369-385` validates and serializes against that exact operation limit. Its valid state space includes up to 20 deaths with up to 16 killers each (`225-249`) and up to 2,000 guild members (`251-279`); these state bounds can produce ordinary valid JSON bodies above 10,240 bytes.

The shared HTTP implementation at `http1_mtls.rs:119-143` instead falls through to `REQUEST_ENVELOPE_BYTES = 10_240` for both operations. Neither appears in the expanded-cap match arms. `exchange_status` accepts the operation-specific body bound (`169-172`), establishes TCP/TLS, then constructs the request and returns `SourceError::CapacityExceeded` when head plus body exceeds 10,240 (`189-194`). It has not written the request at this point. Every body longer than 10,240 is rejected regardless of valid header length; slightly smaller bodies also fail when the header crosses that total.

This is a real composed path:

1. `apps/game-server/src/node/serve.rs:1219-1256` creates the actual `Publisher` using `PublicProjectionsStore` and `MtlsPublicSink` when the configured public publisher is enabled.
2. `public_projections/v2.rs:51-57` enables Guild and CharacterDeaths as ordinary V2 families independently of the optional newer families.
3. `crates/gameplay/durability/src/durability/public_projections/v2.rs:24-46` fetches the current durable snapshot and materializes the matching V2 body.
4. `public_projections/v2.rs:513-527` passes the encoded snapshot to `deliver(family.operation(), ...)`.
5. `public_projections.rs:699-711` uses the shared `http1_mtls::exchange_with_status` and maps its failure to `NotDelivered::Unavailable`.
6. `public_projections/v2.rs:531-548` acknowledges only success; the failed report remains pending and retries with backoff. The unchanged oversized report cannot succeed on retry.

**Impact:** larger valid guild/death public snapshots cannot be delivered, so public views stay stale. The repeated unavailable result can keep the oldest report at the head of its family until its data changes. The audit did not establish production frequency, a gameplay outage, loss of authoritative game data, or an actual live backlog.

## Counterguards and reproduction boundary

Operation body validation does not protect against this mismatch because it intentionally permits more bytes. The generic request cap correctly bounds memory; the wrong part is failing to allocate the declared per-operation envelope allowance. Small guild/death snapshots remain unaffected. Public projection configuration can disable the entire route; deployment configuration was not inspected.

To reproduce in an isolated existing fixture, construct a valid sorted guild snapshot with enough members to serialize above 10,240 and below 393,216 bytes, invoke the native V2 encoder, and route it through the real mTLS test sink. Expected: successful request write and acknowledgement. Current: `CapacityExceeded` before request write, retained outbox row and retry. Add below-bound and exact-bound controls and the analogous death snapshot. This integration scenario is specified, not reported as executed.

## Proposed repair direction

Give both V2 operations their declared body cap plus the fixed request-head allowance, with native encoder-to-transport boundary cases. Preserve the bounded envelope and the success-only outbox acknowledgement. No source repair was made in this audit.
