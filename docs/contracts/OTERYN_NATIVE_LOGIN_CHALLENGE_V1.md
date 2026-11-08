# Oteryn native login challenge v1

Game-client implementation contract, ACCOUNT-SERVICES, 2026-11-08.
Platform implementation and deployment remain required. This is Oteryn's own
challenge; no CipSoft, Cloudflare or other external challenge service is contacted.
Identity/security policy stays in Platform under
[the Character boundary](CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md).

## Client flow

The existing authorization-code + PKCE S256 flow is unchanged for ordinary login.
Before issuing an access token, `POST /oauth/token` may return HTTP 403 with exactly:

```json
{"error":"oteryn_challenge_required","challenge_id":"01890a5d-ac96-774b-bcce-b302099a8057","expires_in":120}
```

`challenge_id` is canonical lowercase UUIDv7; `expires_in` is an integer 1–300
seconds. Unknown/duplicate fields, wrong types and invalid bounds fail closed.
Bodies retain the existing 16 KiB OAuth bound. No access token, native ticket or
Game session exists yet. The code must remain usable for this specific retry.

The client opens the configured Platform origin's `/oauth/challenge` page in the
system browser with `client_id`, `challenge_id`, a **new** ephemeral IPv4 loopback
`redirect_uri` and independent 256-bit random `state`. It never opens a URL
supplied by the challenge response. The page should say “Oteryn sign-in check”
and explain completing the check and returning to the game. OAuth code, PKCE
verifier and access tokens are never placed in this URL.

After completion, Platform redirects to that exact loopback URI with
`code=<opaque challenge proof>&state=<exact state>`. `code` here is the existing
callback envelope's field name, not an OAuth authorization code. Proof is visible
ASCII using only letters, digits, `-`, `_` and `.` (unpadded base64url segments),
nonempty, at most 2048 bytes. This alphabet needs no query escaping and keeps
proof plus state within the existing 4096-byte callback query budget.
Existing callback validation rejects wrong
state, duplicate parameters, bad paths and malformed callbacks. Timeout is the
lesser of configured callback timeout and remaining challenge lifetime.

The client resubmits the **same** OAuth form (`grant_type`, `client_id`, original
OAuth `redirect_uri`, `code_verifier`, OAuth `code`) with two extra form fields:
`challenge_id` and `challenge_token`. A second challenge/refusal ends the attempt;
there is no automatic challenge loop. Browser failure, cancellation, wrong state
or expiry fails closed. Proof is redacted in Debug and exists only in memory.
Only a successful token response continues to character selection/ticket/admission.
The legacy `exchange_code` API fails closed on challenge responses.

## Exactly what Platform must implement

1. Count failed sign-ins and suspicious rates at the identity owner, atomically
   across replicas. An initial proposed policy is N=5 failures per identity in
   10 minutes or 20 attempts per source in one minute. Platform's owner must
   approve thresholds, trusted proxy/source handling and retention. Avoid account
   enumeration. A client counter or Game command cannot satisfy this requirement.
2. Durably bind the issued challenge to the pending authorization code, client,
   PKCE S256 challenge, original redirect URI, authenticated identity/context,
   expiry and policy version. Return 403 before code consumption; do not extend
   authorization-code expiry. Reject a retry after either expiry.
3. Serve `/oauth/challenge` with Oteryn-owned challenge content, accessibility
   alternative, rate limits and no third-party resources. Validate its client,
   challenge and loopback return URI against the pending flow; never allow an
   arbitrary redirect. Bind the return state/URI to this browser attempt.
4. Issue a short-lived, audience-bound signed proof after successful completion.
   Provision and rotate signing keys outside the repository; include a one-use
   nonce. Bind proof to the full pending flow above. Never accept a public test
   token, unsigned proof, response URL, client assertion or elapsed time as success.
5. On retry verify signature, audience, binding, nonce, expiry and current policy;
   consume proof and OAuth code and issue token in one transaction. Concurrent
   or replayed retries cannot issue another credential. Refuse invalid proofs
   with a generic OAuth error; no bypass after challenge expiry. Keep existing
   native-ticket scope/consumption and Gateway protocol unchanged.
6. Add Platform durable tests: N−1/N failures and suspicious rate; cross-replica
   counters; invalid/mismatched/expired/signature-invalid proof; concurrent
   consume, replay and rollback; no credential before completion; safe redirect
   validation; no secret logging. Deploy this extension before advertising it.

## Evidence in Game

`crates/platform-client/src/native_login/challenge.rs` owns bounded decoding and
proof-bearing exchange. `apps/client/src/native_entry/challenge.rs` owns the
browser/loopback/retry flow, called by production `authorize`.
Focused tests cover decoding, expiry, cancellation, redaction, repeated challenge,
browser failure and wrong callback state. The local Platform mock binds the exact
OAuth form and consumes its deterministic proof once; it is compiled only for
tests and is not a cryptographic verifier or a deployable identity service.
The admission test traverses real HTTP, loopback callback and TLS Game admission.
There are no Game SQL changes and no applicable Game PostgreSQL cases.
