# Oteryn Premium panel v1 — read-only native projection

T165 / cg_account. The dedicated native Premium overview is reached from
Characters → Account services → Premium overview. It displays status, expiry,
remaining time and the current Oteryn feature catalogue, with explicit refresh.
It contains no purchase, grant or payment control. No CipSoft services are used.

## Authority and transport

Platform remains the commercial/entitlement authority under
CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md and the accepted PREMIUM-DELIVERY-0
snapshot contract. Game continues to use its existing Premium snapshot client,
durable entitlement fence and action eligibility checks. This UI projection
cannot be submitted as entitlement evidence and never enables gameplay locally.
It does not replace or expose the service-to-service mutual-TLS snapshot endpoint.

The Platform owner must mount this native read-only projection over its existing
Premium read model before deployment. No writable Platform repository is allocated
to this Game lane; this document is the exact remaining Platform prerequisite.
The Game client implements its actual typed HTTP consumer and the test-only HTTP
producer; this is not a claim that the Platform endpoint has been deployed.
No new Game persistence or migrations are required.

`GET /api/v1/native-account/premium` at the configured Oteryn Platform origin.
Authenticate using the existing separate `game:account-services` OAuth token.
Derive the account from that token; accept no client-supplied account ID. Never
reuse a gameplay ticket as an account-services bearer. Return HTTP 200 with JSON;
401/403 refuse authentication/authorization, 503 means unavailable. Apply the
existing native-account transport deadline, cancellation, no redirects/proxies,
TLS policy and 65536-byte reply cap. No mutation endpoint exists for this panel.

## JSON reply

Every member is required, including nullable `expires_at_unix`. Unknown members,
invalid enums, unsupported versions and duplicate feature IDs are rejected.

```json
{
  "version": 1,
  "authority_revision": 17,
  "generated_at_unix": 1790000000,
  "valid_for_seconds": 60,
  "state": "active",
  "expires_at_unix": 1790086400,
  "features": [
    {
      "id": "private_channels",
      "title": "Private channels",
      "description": "Create a private conversation channel where this world permits it.",
      "enabled": true
    }
  ]
}
```

- `authority_revision`: positive monotonic revision of this account's complete
  presentation, including feature policy. Bump when state, expiry or features
  change. Never reuse a revision for contradictory data.
- `generated_at_unix`: server UTC integer seconds in 1..253402300799.
- `valid_for_seconds`: 1..300, bounded by the upstream entitlement evidence lease
  and its refresh policy. Platform must not advertise stale cached entitlements
  as current. A failed authority refresh must refuse or return non-enabled status.
- `state`: `none`, `active`, `expired`, `revoked`. `none` requires null expiry;
  `active` requires expiry greater than generation time; `expired` requires expiry
  no later than generation time. `revoked` may retain its original expiry or null.
  Non-active replies must disable all feature rows.
- `expires_at_unix`: null or UTC seconds in the same range. Client shows the exact
  UTC epoch expiry plus a days/hours/minutes countdown based on server generation
  time and monotonic elapsed time, independent of the workstation wall clock.
- `features`: at most 32 rows. Unique nonempty `id` up to 64 UTF-8 bytes, `title`
  up to 80 and `description` up to 512, all free of control characters. `enabled`
  is a boolean. Features and wording come from current Oteryn policy, never an
  invented static set of official-client benefits. An empty list is explicit.

Client starts freshness before the request, refuses backwards authority revisions
or generation times and contradictory content at the same revision. Failed or
invalid refresh retains the last snapshot only as stale information. Elapsed
freshness or expiry suppresses every available indicator, including while the
native window remains open. Cancellation closes the panel through the existing
account-services lifecycle. Closing returns to the existing account-services
view; normal gameplay authorization is separately refreshed by that flow.

Platform qualification: test payer authorization, revoked/expired/none states,
feature-policy revision updates, upstream lease expiry and unavailable authority.
Reuse existing durable entitlement state rather than introducing another grant
store. Game's mock tests cover the wire contract, stale/refusal handling and the
real selector → scoped login → services → Premium → refresh → return path.

## Existing-data qualification (2026-11-08)

T165 is **PARTIAL for live integration**, rather than DONE against an existing
native server/protocol projection. The UI, typed consumer and tests are present;
mock success is not proof that the live producer exists.

Read-only inspection of Platform main
[`bc6f88346e6aaf1347f0a5a798b33a5ede00bc04`](https://github.com/Oteryn/Oteryn-Platform/blob/bc6f88346e6aaf1347f0a5a798b33a5ede00bc04/portal-rs/crates/payments/src/account.rs#L185)
confirms the existing Premium account route is `/{lang}/panel/premium`, enabled
by `PANEL_PREMIUM_PAYMENTS`, authenticated with `BrowserSession`, and rendered as
HTML from `oteryn_store::account::premium`. It is not this native scoped JSON
projection. Do not scrape that page, forward browser cookies, embed service mTLS
credentials in the client, or infer account expiry from `premium_only` channel
flags, `requires_premium` offers or Premium-required refusal codes.

The remaining producer work is to expose the existing Premium read model through
the authenticated projection above, supply actual feature policy, and qualify
it against the existing entitlement lease/revocation rules. Until then, HTTP
404/501 displays an explicit unavailable-service state, marks retained data stale,
and confirms no cached benefits. Transient errors remain retryable through Refresh.
