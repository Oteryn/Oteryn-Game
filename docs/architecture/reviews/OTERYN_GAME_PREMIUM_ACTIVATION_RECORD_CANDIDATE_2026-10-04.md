# Premium activation record (candidate)

- Record: `PREM-ACTIVATION-RECORD-1`
- Status: **CANDIDATE. NOT ACTIVATED.** This record names no switch-over date and changes no
  configuration. Premium stays off: production configures no snapshot client, so every account
  reads Free, and the `PremiumActivation` gate of PREMIUM-ACTIVATION-0 §1.2 is not set.
- Task: `PREM-E2E-1` (owner D540, control plane #1622)
- Required by: PREMIUM-DELIVERY-0 §6 ("Switch-over"), §10.3 and §12.5; `PROD-ENTITLEMENTS-01`
  §6.6 and §7; PREMIUM-ACTIVATION-0 §1.2 and §3
- Cross-repository coordination id: `OTV2-PREMIUM-DELIVERY`
- Authority: NONE for production, certificates, secrets, Platform writes or activation. Each
  item in §4 needs its own authority.

## 1. Preconditions (PREMIUM-DELIVERY-0 §10.3)

| Precondition | State | Evidence |
|---|---|---|
| PREM-1b merged | met | Game #1678 (client, refresh, test producer, migration 0057) |
| PREM-P accepted and merged | met for code, open for closeout | Platform #1432 (contract) and #1433 (producer, `/admin/premium`). Platform Issue #1431 is still open |
| Path, form and values reconciled | met | PREMIUM-DELIVERY-0 §12, this task's PR |
| Cross-repository end-to-end test, Game half | met when this PR merges | `apps/game-server/tests/premium_platform_fixtures.rs` and `premium::tests::platform_fixtures_classify_as_the_contract_requires`, against Platform's fixtures pinned at `71bbe6c5cffc29d430195fa286b3c6941af4abb8` |
| Real-endpoint run against a deployed PREM-P | **open** | §4 item 2 |
| PREM-P live (deployed and enabled) | **open** | §4 item 3 |
| `PROD-ENTITLEMENTS-01` §6.6 evidence | candidate (§2). The revisions are open | this record |
| Owner authority and switch-over instant `S` | **open** | §4 item 5 |

## 2. Rollout evidence (`PROD-ENTITLEMENTS-01` §6.6)

### 2.1 Exact revisions

- **Platform producer revision: open.** It is the `PLATFORM_BUILD_REVISION` of the deployment
  that serves the real-endpoint run, read from `producer_revision` in its responses (40
  lowercase hexadecimal characters). That deployment must contain Platform #1433. The contract
  and fixtures this record was checked against are at Platform
  `71bbe6c5cffc29d430195fa286b3c6941af4abb8`.
- **Game consumer revision: open.** It is the `main` commit that contains this task's PR, or a
  later one, as deployed. A consumer older than this PR must not be activated (§2.3).

### 2.2 Contract and profile revision (the compatibility pair)

| Item | Value |
|---|---|
| Snapshot schema | `oteryn.premium_snapshot.v1` |
| Request schema | `oteryn.premium_snapshot_request.v1` |
| Producer profile | `oteryn.entitlement.profile_b.v1` |
| Product | `oteryn.premium_time`, version `1` |
| Product policy | lease 3,600 s, `refresh_after` at two thirds of the lease, clock skew 5 s, stale use denied (PREM-P §3) |
| Consumer surface policy | `premium-surfaces-1`: every surface `REQUIRE_CURRENT` |
| Endpoint | `POST /internal/v1/products-entitlements/premium-snapshots/read`, mutual TLS 1.3 |

Both sides record this pair: PREM-P in its contract §1, §3 and §5, Game in `premium::{SNAPSHOT_SCHEMA,
PRODUCER_PROFILE, PRODUCT_ID, PRODUCT_VERSION}`. The end-to-end test checks that they are equal.

### 2.3 Mixed-version compatibility rules

- Platform changes the wire only with a new `schema` id and a new fixture set (PREM-P §7).
  Game accepts only the pair in §2.2. Any other well-formed `schema`, `producer_profile`,
  `product_id` or `product_version`, or a lease above 3,600 seconds, is a durable
  `INVALID_OR_CONFLICTING`. It denies Premium for the account, records an audit row, and nothing
  clears it in V1 (PREMIUM-DELIVERY-0 §3.1).
- A Platform without PREM-P has no route. Any non-200 is a failed pull, and Premium is denied.
- A Game consumer older than this PR calls `/v1/premium/snapshot`. Platform has no such route,
  so every pull fails and Premium is denied (fail closed). A PREM-1b-era consumer that is pointed
  at the PREM-P path does not know two things. It would keep benefit until
  `authority_valid_until` instead of `refresh_after`, and it would accept bodies that break the v1
  rules. That is a weaker validity reading, so **activation requires a consumer at or after this
  PR**. §2.1 records that revision.
- Platform's 404 (an unknown AccountId), 400, 401, 429 and 503 are each a failed pull. None of
  them is Free by `NONE`.

### 2.4 Rollout classification: producer first

1. Platform deploys PREM-P with `PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_ENABLED` unset. Every
   request then gets a `503`.
2. Operations issue the Game server's client certificate and configure Platform's dedicated
   subject (PREM-P §4.2). Game deploys a consumer at or after this PR, with
   `OTERYN_PREMIUM_SNAPSHOT_URL`, `OTERYN_PREMIUM_CLIENT_IDENTITY_PEM` and
   `OTERYN_PREMIUM_PLATFORM_CA_PEM`. Every pull still fails, so Premium is denied.
3. Platform enables the endpoint in a non-production environment, and the real-endpoint run
   (§4 item 2) is recorded.
4. Production repeats steps 1 to 3. The owner then sets `PremiumActivation` with `S`
   (PREMIUM-ACTIVATION-0 §1.2).

Each half fails closed on its own, so the order needs no atomic switch. The order matters only
for step 4, which comes last.

### 2.5 Rollback order

- **Stop Premium without a deploy.** Disable Platform's endpoint (`503`) or revoke the Game client
  certificate. Every pull then fails, and each online account loses Premium at its next pull at
  the latest. Kept evidence never outlives its `refresh_after`, which is at most 40 minutes after
  issue. Login is unaffected.
- **Game consumer rollback** goes only to a revision at or after this PR (§2.3). An older one
  fails closed on the old path, but it must not be configured for the PREM-P path.
- **Platform producer rollback** with its database intact needs no Game action, because
  revisions keep rising. Restoring the Platform database (PREM-P §6.5) makes Game reject
  re-issued low revisions as stale. Premium then fails closed for the affected accounts until
  Platform issues higher revisions, and recovery is operator reconciliation against Game's
  retained high water.
- The PREMIUM-ACTIVATION-0 latch is irreversible by design. A rollback after `S` stops Premium
  through the two levers above, never by unsetting the latch.

### 2.6 Evidence issued before a rollback

Game's fence keeps its high water and its durable conflict markers. Older revisions are stale.
Kept evidence stops granting benefit at its own `refresh_after` and expires at
`authority_valid_until`. A restart needs a fresh proof before any benefit. Nothing extends a
cutoff or moves a start earlier: there is no stale grace, no cache restore and no fail-open
path (PREM-P §8.4; PREMIUM-DELIVERY-0 §3.1).

### 2.7 When one side does not understand the validity semantics

| Case | Deterministic outcome |
|---|---|
| Unknown or downgraded schema, profile, product or version, or a lease above 3,600 s, in a well-formed body | durable `INVALID_OR_CONFLICTING`: Premium denied, audit row written, not cleared in V1 |
| Supported body that breaks a v1 rule, or malformed, oversized, unbound (nonce or account) or wrong content type | failed pull (`AUTHORITY_UNAVAILABLE`) until a later pull succeeds |
| Any non-200, timeout, TLS failure or redirect | failed pull |
| Trusted time missing or uncertainty above 5 s | not current |
| Upper trusted time at or after `refresh_after` without newer evidence | `STALE_WITHIN_BOUND`, denied |

No outcome restores unbounded stale authority.

## 3. Mandatory scenarios (`PROD-ENTITLEMENTS-01` §7)

| # | Scenario | Game evidence |
|---|---|---|
| 1 | Current active entitlement within the bound | `platform_fixtures_classify_as_the_contract_requires` (active fixture current) |
| 2 | Platform outage before the bound expires | a failed pull denies at once (`premium_failed_pulls_deny_until_a_later_success`, `only_proven_active_evidence_inside_its_interval_is_current`) |
| 3 | Platform outage after the bound expires | stale from `refresh_after`, expired from the cutoff (same unit tests) |
| 4 | Effective interval ends while Platform is unreachable | `restrictive_facts_win_and_start_is_conservative` (the commercial end wins) |
| 5 | Newer revoke after cached active | revoked fixture denies. Higher lifecycle revisions advance the fence (`premium_fence_is_monotonic_and_idempotent`) |
| 6 | Delayed old active after a newer revoke | stale below the high water (`premium_fence_is_monotonic_and_idempotent`) |
| 7, 8 | Reconnect with stale or expired evidence | admission and reconnect pull before any read. Stale and expired evidence deny (`premium_admission_pulls_without_waiting_one_at_a_time`) |
| 9 | Node restart with cached active | `premium_restart_reproves_before_benefit` |
| 10, 11 | Cache rollback, out-of-order revisions | high-water fence and ticket ordering (`premium_fence_rows_cannot_be_rolled_back`, the racing-ticket cases) |
| 12 | Equal revision, contradictory state | `premium_equivocation_fails_closed_and_sticks` |
| 13 | Unsafe local clock | `TrustedNow` refuses uncertainty above 5 s. PREMIUM-ACTIVATION-0 §1.1 owns the system clock |
| 14 | Not yet effective under skew | `restrictive_facts_win_and_start_is_conservative` |
| 15 | Ambiguous durable game delivery, then retry | not applicable to Profile B: Premium is checked at use and never delivered into game state |
| 16 | Version mismatch during rollout and rollback | §2.3 and §2.7. `premium_unsupported_is_durable_and_never_cleared`, `every_invalid_fixture_is_refused_with_the_required_outcome` |

## 4. Still open before activation (each needs its own authority)

1. **Platform closeout.** Platform Issue #1431 is closed by Platform's coordinator. Game writes
   nothing there.
2. **Real-endpoint run** in a non-production environment, against a deployed PREM-P with a
   Platform-issued test client certificate. The run records `producer_revision` and the Game
   consumer revision. It records one response of each kind: `NONE` for a new known account,
   `ACTIVE` after an operator test grant, `REVOKED` after a revocation, `404` for an unknown
   AccountId and `503` while disabled. Each response must be classified as §2.7 says.
3. **PREM-P live**: the production deployment, the enabled flag, the certificate and the subject
   configuration.
4. **Game configuration**: the three environment variables of §2.4 step 2, as deployment secrets
   (none committed).
5. **Owner authority** for production activation, and the switch-over instant `S`
   (PREMIUM-ACTIVATION-0 §1.2 and §3). This record becomes `ACCEPTED` only when items 1 to 4 are
   recorded here and the owner names `S`.
