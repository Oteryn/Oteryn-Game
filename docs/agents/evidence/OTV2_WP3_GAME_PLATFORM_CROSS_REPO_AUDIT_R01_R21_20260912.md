# OTV2 WP3 Game + Platform cross-repository audit R01-R21

Date: 2026-09-12
Status: retained audit evidence; no implementation or architecture-acceptance authority
Game snapshot: `489e3e390a1bce1ce3439c66521ab75f8a826cd8`
Platform snapshot: `2271fea9db1e202cacb206dab289efa4cefcec76`

This artifact consolidates the cross-repository findings produced while auditing the WP3 -> WP4/Child B -> WP5/source-readiness -> G0 -> Server Seam dependency chain. It does not replace the canonical WP3 Q01-Q75 matrix or historical PR #588 evidence. Runtime tests, real PostgreSQL qualification, real mTLS, production restore drills and end-to-end failure injection were not executed as part of this audit unless explicitly stated below.

Evidence language follows repository convention: `PROVEN`, `DERIVED`, `UNKNOWN`, `RECOMMENDATION`.

## R01 — Platform native evidence producer now exists

`PROVEN`: the Platform producer counterpart for the four accepted operations exists on protected Platform main. Historical Game handoffs that say the producer is absent are stale on that factual point.

`RECOMMENDATION`: treat older producer-missing handoffs as historical evidence while preserving any still-valid allocation semantics.

## R02 — producer is not the whole credential issuer

`PROVEN`: the native evidence producer serves account-security/signing-trust source material. Platform also has separate OAuth/Game Login Ticket concepts.

`UNKNOWN`: this audit did not establish a complete current native credential/pre-admission issuer flow using the same final native generation semantics all the way into gameplay admission.

`RECOMMENDATION`: do not make a full web/Gateway journey an artificial prerequisite to local WP3/S1 where the accepted G0 contract does not require it, but do not claim full login-to-gameplay completion without issuer -> grant -> source -> FND-04 qualification.

## R03 — WP3 root extends beyond SQLx decoder bytes

`PROVEN`: the real Child B/RuntimeBackend path includes pool/bootstrap/schema/custody work and semantic operations outside a narrow receive-buffer accounting seam.

`RECOMMENDATION`: superseding WP3-v2 must decide the root/pool/executor ownership boundary before further dependency instrumentation.

## R04 — four distinct network profiles must not be conflated

`PROVEN`: the programme spans at least PostgreSQL transport, gameplay TCP+TLS 1.3, Platform native-source HTTP/1.1+TLS 1.3+mTLS and client-side HTTPS to Platform.

`RECOMMENDATION`: qualify each profile independently and compose only exact compatible candidates.

## R05 — Platform feature tests do not prove real mTLS

`PROVEN`: Platform native-evidence tests inject `SSL_PROTOCOL`, `SSL_CLIENT_VERIFY` and `SSL_CLIENT_S_DN` into Laravel request state. They validate application behavior for trusted metadata but do not perform a real TLS handshake/terminator trust-boundary test.

`RECOMMENDATION`: require real authenticated Platform<->Game transport qualification before WP5 S3.

## R06 — Platform application limits do not automatically prove ingress-to-response bounds

`PROVEN`: Platform has bounded application-side request/response/JSON/header profiles and two application in-flight slots, but request decoding and response serialization include layers outside the application slot proof.

`UNKNOWN`: exact ingress/FastCGI/TLS buffering and end-to-end response concurrency bounds in the intended deployment topology.

`RECOMMENDATION`: qualify the real ingress topology rather than treating Laravel limits as proof of the entire server envelope.

## R07 — witness availability affects both evidence serving and security mutation availability

`PROVEN`: native security/trust high-water witnesses are independent from relational state; activated revocation fails closed when required witness state is unavailable or inconsistent.

`DERIVED`: witness topology/restore semantics are therefore part of both source availability and security-mutation availability.

`RECOMMENDATION`: qualify DB+witness restore ordering and failure/recovery, not only successful reads.

## R08 — source age is a composition constraint

`PROVEN`: accepted source evidence has bounded age/clock uncertainty semantics.

`DERIVED`: queueing, mTLS fetch, publication and DB waiting can consume that freshness budget even when each component is individually correct.

`RECOMMENDATION`: test freshness at the final authority-use point, including saturation and recovery.

## R09 — possible DFR/native-source wait-cycle requires proof

`DERIVED`: when source publication and Child B share constrained database/executor resources, a wait-for cycle is possible in principle unless ownership/order proves otherwise.

`UNKNOWN`: no such cycle was reproduced by this audit.

`RECOMMENDATION`: publish an explicit wait-for/resource graph and saturation/recovery test; do not solve uncertainty by silently adding slots or a second executor.

## R10 — AccountId migration is intentionally multi-phase

`PROVEN`: Platform adds canonical UUIDv7 AccountId while retaining nullable rollout compatibility until pre-AccountId writers are drained; later backfill+NOT NULL is a separate phase.

`RECOMMENDATION`: merge of the producer implementation must not be described as completed production cutover.

## R11 — HTTP transport result and domain result are separate

`PROVEN`: the Platform producer may return bounded domain results such as `unavailable` in an HTTP-success response while transport/authentication/shape failures use transport status/failure behavior.

`RECOMMENDATION`: Game ingestion must classify transport and domain outcome separately and fail closed according to the accepted contract.

## R12 — local gates are not exact cross-repository proof

`PROVEN`: Platform feature tests use framework-level request injection and default test datastore behavior; Game unit/wire tests separately prove codec behavior.

`RECOMMENDATION`: final S3/G0 evidence must identify the exact Game head, exact Platform head, actual database/transport profile and real interoperability run.

## R13 — downstream product modules are compatibility constraints, not blanket WP3 scope

`PROVEN`: Platform owns web identity/commercial/control-plane concerns while Game owns gameplay/character authority according to accepted boundaries. Atlas consumes Game-owned exports rather than becoming Game truth.

`RECOMMENDATION`: inspect downstream contracts for compatibility but do not pull Bazaar/payments/content/Atlas implementation into WP3 without an actual blocking dependency.

## R14 — G0, Server Seam readiness and G1 are different outcomes

`PROVEN`: #364 distinguishes G0/source-admission composition from later real command->durable effect->projection G1. #247 separately owns Server Seam delivery.

`RECOMMENDATION`: `SERVER_SEAM_READY_FOR_INTEGRATION` must not be reported as full programme G1 or production enablement.

## R15 — witness write path can succeed without `fsync` when the runtime lacks it

`PROVEN`: Platform `NativeEvidenceHighWaterWitness::writeFloor()` performs `fsync` only when the PHP function exists; without it, the application path can still complete after `fflush`/rename.

`UNKNOWN`: whether the intended production PHP/filesystem profile ever lacks the required synchronization primitive or supplies equivalent guarantees below this layer.

`RECOMMENDATION`: make the durability prerequisite explicit and fail closed where the deployment cannot provide the accepted witness guarantee.

## R16 — current rate-limit admission is not a strict concurrent reservation proof

`PROVEN`: `ThrottleNativeEvidencePeer` calls Laravel `RateLimiter::attempt()`. In the pinned framework implementation, `attempt()` checks `tooManyAttempts`, executes the callback, then records a hit.

`DERIVED`: multiple concurrent requests can pass the pre-check before their hits are recorded; this mechanism therefore does not by itself prove a strict simultaneous admission ceiling.

`RECOMMENDATION`: decide whether the control is an approximate throughput limiter or a hard admission bound. A hard bound needs atomic reservation before expensive work.

## R17 — witness advancement can interact with larger security transactions

`PROVEN`: password change/reset and recovery flows call `RevokeIdentityGameAuthorizations` inside larger transactions. Native generation fencing advances witness state before the relational generation mutation is finally committed.

`DERIVED`: a later outer-transaction failure can leave witness state ahead of rolled-back relational state, intentionally forcing fail-closed behavior.

`UNKNOWN`: complete operator recovery procedure and end-to-end qualification for every outer security workflow.

`RECOMMENDATION`: test through final commit/failure/restart/reconciliation for password, recovery, MFA/email and termination families as applicable; do not weaken anti-rollback by simply moving the witness update after commit.

## R18 — empty writable witness directory is not sufficient provenance by itself

`PROVEN`: witness configuration checks path existence/readability/writability and source bootstrap can create initial floor state when no local witness/history is observed.

`DERIVED`: deployment restore procedures must distinguish a genuinely first activation from accidental loss/replacement of an independently retained witness volume.

`UNKNOWN`: whether the actual deployment tooling already proves such volume identity/continuity.

`RECOMMENDATION`: make witness-volume provenance/restore continuity an explicit deployment qualification.

## R19 — trust-profile revocation is semantically stronger than key rotation

`PROVEN`: the Platform signing-trust registry permits key publication/revocation and profile revocation; a revoked profile cannot accept a newly trusted key through the normal publish path.

`RECOMMENDATION`: separately specify ordinary key rotation, key revocation and terminal profile revocation/recovery. Do not clear revoked state or invent a replacement profile outside an accepted contract.

## R20 — evidence reads create durable source history and consume peer throughput

`PROVEN`: successful new observations advance source revision, persist immutable response JSON and update witness state. Throughput limiting is keyed by authenticated peer identity and defaults to a configured per-minute value.

`DERIVED`: source reads have write/storage/fsync cost and competing operations from the same peer share the service budget. At 120 successful new observations/minute, a constant-rate upper-bound scenario would create 172,800 observation rows/day before retention/reuse behavior is considered.

`UNKNOWN`: real steady-state request rate, reuse behavior, storage growth and latency in target deployment.

`RECOMMENDATION`: benchmark login/reconnect/restart traffic and retention semantics before tuning limits; never delete history required for ordering/rollback correctness.

## R21 — Child B relation locking strongly serializes protected writes

`PROVEN`: the audited Child B head obtains `EXCLUSIVE` relation locks across the current admission ledger before domain advisory keys. This is deliberately conservative and does not claim write concurrency.

`DERIVED`: two active executor slots do not imply two admission transactions can progress concurrently when they require conflicting relation locks; one connection may wait behind another.

`RECOMMENDATION`: choose PgPool vs explicit connection actors using real mixed-operation latency/contention evidence, while preserving all correctness-required locks. Do not infer that one connection is sufficient solely from serialization.

## Programme disposition

These findings support moving from broad discovery to one WP3-v2 superseding architecture decision plus bounded Platform hardening. They do not support continuing indefinite broad SQLx/rustls/Tokio ownership expansion, nor do they justify weakening existing security, resource, durability or lock semantics.

The multi-agent execution split and dependency gates are recorded in `docs/agents/programs/OTV2_WP3_V2_MULTI_AGENT_DELIVERY_PROGRAMME.md`.