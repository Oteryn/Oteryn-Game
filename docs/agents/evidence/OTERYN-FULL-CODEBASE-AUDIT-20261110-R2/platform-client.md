# oteryn-platform-client — source audit

Base `4f0b6c62a789489fca6c37dd36d5917d096f328e`; status **REVIEWED** for all tracked package source, manifests and tests. Native HTTP/TUF/TLS integration tests were inspected, not run. A candidate functional timing defect awaits independent verification; no new confirmed P0/P1 is claimed.

## Entrypoints and invariants

`PlatformClient::fetch_directory/parse_directory` applies a 1 MiB response cap both on advertised and actually read bytes, bounded world/channel/character arrays, typed references and recursive refusal of route/credential fields. The public directory conveys availability and labels, never a gameplay credential. HTTP(S) configuration rejects non-loopback plaintext; redirects and proxies are disabled on directory/native login clients. `NativeLoginClient` has no revealing request-error or secret formatting. The raw directory uses serde_json Value, while sensitive native success/error messages use strict typed decoders rejecting duplicate/unknown/missing members.

`authorization_url/exchange_code[_challenged]/issue_native_ticket/gateway_login` preserves the OAuth → one-time ticket → Gateway → admission division. The selected attempt UUID is canonical, random and created once by the app. Gateway retries reuse the same serialized body, share a six-request budget per NativeTicket, stop at the ticket monotonic deadline and retry only contract retryable rows or a lost response. Native grant response binds the echoed attempt, typed WorldId/ChannelId text, TLS name, endpoint, ALPN/profile and compact-token grammar. The server remains the signature/admission authority. Cancellation wraps the whole request/backoff future; no automatic credential replay is added to device rotations.

Challenge flow uses only the configured Platform challenge route, bounded 1–300 second challenge lifetime and query-safe proof bytes. Proof may be submitted once after a challenge; a second challenge with a proof refuses. The app creates independent callback state, and the server verifies code + PKCE proof binding.

Account-service functions validate bounded schemas, identifier and amount ranges, quote/receipt/request/character binding, permitted state combinations and exact sandbox checkout origin/path. No local callback grants coins; receipt and balance values remain Platform presentation. Premium presentation does not confer gameplay entitlement.

`device_sessions` validates canonical 48-byte credentials, redacts Debug, binds vault namespace to canonical HTTPS origin/client and verifies family/absolute expiry continuity with changed successor credentials. Session-class enrollment proof signs the lexically ordered contract object. Only the explicit original remembered variant is unbound. HTTPS is mandatory for device operations even on loopback; rotation/revocation requests have exact bodies and sensitive Authorization headers, bounded response bytes and no internal retry. The application owns cross-process serialization and commit-before-use of returned successors.

`UpdateClient::check` starts at the build-pinned trust root, isolates channel state, loads TUF metadata with explicit size/root-update bounds and persists verified roots. It requires consistent snapshots, exactly one terminating channel delegation, channel-contained target paths, signed policy revision monotonicity and explicit authorization for release-sequence rollback. `OriginTransport` allows exact HTTPS origins and maps hashed package paths to the download host; TUF still verifies length/hash. Downloads stream to a create-new partial file, sync then rename after successful verification. The actual updater caller `apps/client/src/updater.rs:55-107` holds an OS file lock across metadata/download/install and clears a stale partial before a new download. Thus abandoned partial files do not permanently block real updater recovery. Install/archive verification is outside this package and delegated to the client lane.

## Hypotheses checked

- Directory plaintext exception is restricted to parsed loopback hosts; unrelated hosts cannot borrow it through string prefixes.
- Native Gateway fallback does not alter ticket/attempt identity on retry; an expired or budget-spent ticket is never sent again by the bounded flow.
- Unknown/raw security error rows do not create retry authority: HTTP status, public class and code must match one row.
- Arbitrary checkout URL or wrong order/quote/request identity is rejected before application use.
- Device response shape cannot add nullable optional fields to an enrollment response: exact member counts and typed required fields jointly enforce the endpoint-specific shape.
- Updater stale partial hypothesis is refuted by the actual caller's cleanup under the install lock; unsigned/incorrect target bytes are not activated by the package API.

## Candidate awaiting verification

PS-R2-001 is a P2 functional candidate concerning a remembered-device login failure. It remains LIKELY; concrete producer timing and runtime validation are outstanding. Details are retained privately while the lead independently verifies the path.

## Validation and limits

Existing tests were analyzed for strict schemas, duplicate members, error row mapping, secrets, UUID shapes, request-byte identity, bounded retries/expiry, OAuth-to-ticket request order, challenge replay, sandbox origin binding, device fixture signatures and TUF corruption/root rejection. No package native tests executed. The HTTP test fixture is scripted loopback HTTP; production device HTTPS tests live in the app. No game state/database or live account was mutated. Full tracked package files have exact SHA-256/ranges in `ledger.json`; remaining package files: none. Cross-module app device lifecycle was inspected at the specific ranges cited by the candidate, not claimed as a full app review.
