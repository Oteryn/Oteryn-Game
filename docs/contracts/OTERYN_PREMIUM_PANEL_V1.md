# Oteryn Premium overview v1 (T165)

The supported native overview runs in an authenticated Game session. Open it with
Ctrl+Alt+Shift+P; the Store advertises this shortcut. The dedicated window shows
last accepted account Premium state, actual subscription expiry, countdown,
Oteryn feature descriptions and conservative current availability, with Refresh.
There is no purchase, grant, browser login or payment control in this flow.

## Existing protocol and authority

Use existing capability 97 and command 113 (Character information), with the
single-byte operation payload `[5]`. No new capability, command ID, capability
file or grant authority is introduced. Operations 0–4 and their bytes retain
their meanings. An old server rejects operation 5 and the client reports that
this overview is unavailable; an old client never sends the new operation.
The request contains no character or account selector. Extra bytes are rejected.

The actual handler resolves the authenticated account from the current Character
root under recovery, node-incarnation, session, connection-generation, Character
lease and scope fences. It checks the runtime actor and durable account binding
before and after observing Premium. A stale lease, detached account or changed
owner refuses the query. QueryVisible cannot retrieve another account's Premium.

The existing Game Premium fence is the only subscription metadata source; its
consumer/coordinator and registered trusted clock determine benefit availability.
The subscription end comes from `PremiumEvidence.effective_until_us`. It must
**not** be replaced with `CurrentPremium.benefit_until_us`: that is the shorter
minimum of subscription end, authority lease and refresh deadline. No client
receives Platform service credentials, account IDs or entitlement identifiers.
Game's normal Premium refresh and activation rules remain authoritative.

This is an observation, not grant evidence. A last accepted Active state may be
unconfirmed or expired by the time it is displayed. Only a current, delivered
Premium gate whose revision matches the durable observation can enable feature
indicators. Missing evidence, clock loss, conflicts, stale leases and revocations
confirm no benefits. All gameplay actions still recheck their own eligibility.

## Bounded result profile

CommandResult Accepted/DuplicateReplay payload is `premium_overview::Overview`:

| Field | Encoding |
| --- | --- |
| version | u8, 1 |
| state | u8: 0 unavailable, 1 none, 2 active, 3 expired, 4 revoked, 5 scheduled |
| authority_revision | u64 LE; zero only for unavailable |
| observed_at_us | nonnegative i64 LE, conservative trusted Unix microseconds |
| expires_at_us | positive i64 LE; zero encodes no confirmed subscription end |
| valid_for_us | u64 LE, 0..60000000, bounded by the existing current authority read |
| feature count | u8, at most 16 |
| each feature | enabled u8 (0/1), title and description as u16 LE byte length + UTF-8 |

Title is 1..80 UTF-8 bytes, description 1..512; control characters and duplicate
titles are rejected. Total result is at most 16384 bytes. Missing/trailing bytes,
unknown versions/states, invalid booleans, inconsistent expiry/revision and enabled
features without current validity are rejected. Unavailable has revision zero,
no expiry and zero validity. Active/expired/scheduled require an expiry. Positive
validity is only legal for Active and cannot outlast the actual subscription.
The existing Character information command limits and dispatch gates still apply.

The server describes implemented Premium-gated channels, promotion benefits,
spells, private channels and house services. Descriptions explicitly preserve
world, Character and service-specific requirements. Indicators are observations
of the Premium prerequisite, not assertions that every other rule is satisfied.

The client starts its monotonic freshness clock before queuing the request,
so queue/network time reduces validity. It refuses regressing authority revisions,
backwards observation times and contradictory same-revision subscription facts.
Invalid, rejected and unavailable replies invalidate indicators while retaining
last known metadata only as unconfirmed. The native window recomputes countdown
and freshness while open. Session end clears the panel and closes its window.

## Qualification

`premium_game_real_controller_sends_existing_command_and_refuses_stale_data`
drives the actual PlayLink/controller command path. Codec tests cover bounds and
canonical decoding. `t165_premium_overview_real_dispatch_account_privacy_expiry_and_leases`
uses real PostgreSQL and the production admitted dispatcher: another account's
Premium never supplies this account's observation; actual subscription end is
separate from a shorter refresh deadline; refresh expiry and revocation disable
indicators; stale Character leases and payloads with account-selection bytes fail.
No new persistence schema, financial ledger, commercial policy or payment keys
are required.

## Earlier pre-login projection

The optional scoped Platform SDK endpoint `/api/v1/native-account/premium` and
its mock tests remain a separate, unqualified pre-login integration seam; the
default native Bazaar window no longer offers that entry. It is not used by the
supported in-game overview and is not evidence of live Platform deployment.
Platform's existing Premium page at main `bc6f88346e6aaf1347f0a5a798b33a5ede00bc04`
is browser-session HTML. Do not scrape it or expose service mTLS credentials.
See ACCOUNT_SERVICES_LIVE_DECISIONS.md for commercial/Platform ownership. Payment
remains sandbox/disabled and no CipSoft service is contacted.

## Account-service overview retained

The authenticated account-service surface below remains available from Characters / Account services. It is a separate read-only projection; neither surface grants Premium or bypasses Game eligibility checks.

### Account-service Premium panel v1

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
