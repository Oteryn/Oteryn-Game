# Social — complete package source review

Base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`.

Status: **REVIEWED**. All **56 tracked package files / 11,871 lines** were read substantively, including the manifest, all implementations, literal rules and original test bodies. `social-coverage-validation.json` verifies complete recorded ranges against immutable Git blobs and the unchanged source checkout at **2026-11-10 16:40:03 UTC**. Byte and range validation preserves evidence of the preceding review; it does not replace substantive review.

## Chat, relay, party and guild

Chat review covered typed text bounds, Unicode greeting-word matching, local whisper and yell projection, source-specific spam policy, room membership and bounded egress. The queue reserves the in-flight item, prioritizes room/local/private traffic and tracks discard versus delivery. The actual connection writer at `crates/gameplay/dispatch/src/gameplay_transport/connection/serve/refresh.rs:120–188` awaits the frame write before marking a real chat item delivered and requesting the next item. The room acceptance adapter rechecks the current Guild/Party binding; a denied queued item is discarded without being presented as delivered text.

All relay implementation and test files were read. Encryption binds the channel and key identity, payload identity and text forms are closed, and retained key overlap is explicit. The replay window is bounded and refuses saturation without dropping unexpired entries. Database-time high-water, restart floor and the stored lease are separate barriers. The actual dispatch path waits for the durable monotone lease update before marking a lease stored or opening relay payloads. A pure lease helper alone was not treated as durable authorization.

The Party view requires complete node observations for the current generation. Withdrawals invalidate the view; duplicate, cross-World or inconsistent membership facts fail closed. Polling, stale observation and clock regression rules were reviewed with all original tests. Missing cache knowledge is not silently interpreted as a solo player. The view does not export other members' private vitals or Channel identity.

Guild maintenance uses an explicit World job and sequential war, deadline and disband owners. Cursor advancement follows successful work, and the World enumeration is bounded. This package review does not independently resolve the separately retained question about processing more than one page of active guild wars.

## Premium

The consumer treats Platform snapshots as the Premium authority. It validates the bound account and fresh nonce, complete nullable schema, interval and lifecycle semantics, strict supported-version forms and bounded body before accepting evidence. A well-formed unsupported version is distinguished from a malformed response. Durable high-water and conflicts are monotone; a loaded snapshot does not authorize a new process until freshly proven. Pull-ticket ordering prevents an older successful request from clearing a newer failure.

Trusted time includes a bounded uncertainty interval and rejects unsynchronized or wholly obsolete readings. Classification uses conservative boundaries at effective start, refresh and expiry. Actual entitlement end is separate from a failed pull or an expired authority lease. The production relocation caller first settles activation delivery and loads the durable fence, then requires the entitlement-ended result; a temporary failure by itself does not qualify a citizenship change.

The HTTP adapter uses an explicitly configured HTTPS endpoint, pinned trust roots, mutual TLS, no proxy or redirects, a total deadline and an enforced streaming body limit. Configuration is all-or-none. One refresh task per online account serializes pulls, uses a new nonce for each request and has bounded retry and Retry-After scheduling. Wall-clock scheduling is not the authorization clock. The test producer creates its own temporary PKI and loopback server and remains test-only.

## Spectate and Cast

Spectate and Cast are distinct owners. Spectate review covered explicit capacity, grant codes and revisions, exact observer identity, rolling admission rate limits, reserved terminal audit capacity and lifecycle invalidation before output. Its projection whitelists public spatial and presentation data and has bounded output queues. Reading this package does not establish deployment of the future Spectate admission adapter or a production durable audit sink for that owner.

All ten Cast files were read. Production admission uses an opaque proof from an online purpose-separated Platform consume response. Node, cast, epoch, access revision, account, reservation and expiry are bound together. The proof is capped from the monotonic time sampled before network IO. Local account, cast and node limits supplement the global reservation; keepalive cannot extend that reservation. Password policy remains in Platform preferences.

The worker owns Cast state outside the game tick. It registers and refreshes the durable consent-worker lease, processes revocation barriers, binds captures to a privacy version and one-second lifetime, persists audit prefixes and closes casts on unavailable authority. Consent withdrawals purge both queued and already shown scenes. Scene and media output use the same consent and delay rules. Final draining rechecks consent revision and freshness; reserved terminal journal space remains available when admission is backpressured.

Actual production integration was checked, rather than inferred from comments:

| Edge | Exact source evidence |
| --- | --- |
| Node configuration | `apps/game-server/src/node/serve.rs:3390–3420` supplies `CastRuntimeConfig::from_env` using the current node proof. |
| Explicit activation | `crates/gameplay/dispatch/src/gameplay_transport/cast.rs:643–685` requires the explicit enable flag, valid capacity and secure mTLS files. Deployment values were not inspected. |
| Service construction | `crates/gameplay/dispatch/src/gameplay_transport/serving.rs:31–43` constructs and installs `CastService`. |
| Periodic capture | `connection/serve/refresh.rs:1–20` invokes the selected-capability path; `cast.rs:433–491` obtains the permit, reads the visible map and consent facts, rechecks exact authority after awaits, then publishes or slates. |
| Final network write | `cast_observer.rs:127–152` submits an awaited worker callback with checked sequence, encoding and synchronous complete-frame write/flush. Partial or pending output fails the viewer under that same owner barrier. |

Paths in the last two rows are relative to `crates/gameplay/dispatch/src/gameplay_transport/`. Complete dispatch-file review is delegated and recorded in the client-support ledger; the root independently read the listed integration ranges. The existing central Cast finding is retained once and is not duplicated by this review.

## Achievements and chest use

The achievement catalogue delegates parsing to the previously reviewed progression parser. Earned facts alone are displayed; retired entries contribute zero points. Total points and fact count are checked, rows sort deterministically, and pagination uses checked arithmetic. An unknown earned key fails the complete response with an escaped operator event. No row is silently omitted or truncated. The real wire route derives the account from the admitted controller, while notification reads check exact current authority again after awaits.

Chest use resolves the placement, claim, reward, quantity and optional achievement/quest/key facts from current Content. The child occurrence binds the session, command, chest, claim, semantic revisions and typed edge. The current backpack is read before the existing durable freeze and commit path. Ambiguous storage retries retain the same request; replay after ambiguity uses an unknown grant notice rather than inventing a second earned notification.

The actual entry-chest caller at `crates/gameplay/dispatch/src/gameplay_transport/composed_admission/objects.rs:218–324,349–398` checks World, physical action eligibility, alive actor and reach, and supplies the admitted item fence. Bundle and source-world object routes are separate. Historical comments claiming that no production caller exists are not accepted as present activation evidence. The synthetic entry clone is explicitly distinct from a pinned world-content publication.

## Findings and verification limits

**No additional confirmed finding** was established by this package pass. The central register retains previously qualified cross-module findings and separately tracked open questions. Optional Cast deployment, live TLS behavior and database behavior remain unverified by execution.

One original, unchanged rolling-rate-window test passed with a direct `rustc --test` harness; exact source digest, command and captured output are in `social-rate-window-unchanged-tests.json`. All other test bodies were read without execution. No full Cargo build, PostgreSQL test, live client/server session, deployment or source modification was performed. Supplementary content referenced by this package is not credited as fully reviewed merely because it was embedded, parsed or hashed. The package is **REVIEWED**, not **VERIFIED**.
