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
