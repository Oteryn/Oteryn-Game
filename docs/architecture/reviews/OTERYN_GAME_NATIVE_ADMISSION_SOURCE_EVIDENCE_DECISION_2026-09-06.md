# Native admission source evidence — Game consumer decision

- Decision: `FND-NATIVE-SOURCE-EVIDENCE-V1`
- Status: **CANDIDATE; Game acceptance requires independent review and protected integration**
- Source: Issue #328, prerequisite inventory #319, coordinator #162
- Base: Game `93f31ba05972d3b96afb0d9ea08e2c6753507d8c`
- External evidence: Platform `3b2ea1c7392187d5d22488673073dc8f8305a374`
- `MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`

## Resolution packet

```yaml
classification: ARCHITECTURE_RESOLUTION
repository: Oteryn/Oteryn-Game
main_sha: 93f31ba05972d3b96afb0d9ea08e2c6753507d8c
source_escalation: 328
blocking_question: Which authenticated source transport, observation envelope and bootstrap may register native security/trust evidence for Game admission?
facts:
  proven:
    - FND04 grant profile sections 10 and 13 defer evidence transport/schema while fixing source freshness and anti-rollback
    - Platform ADR 0028 already fixes native AccountId as Platform-issued immutable UUIDv7
    - inspected Platform revocation action owns transactional generation state while its redeem DTO/routes remain Canary integer-bound
  derived:
    - Game requires an authenticated native producer contract before actual source registration
  unknown:
    - counterpart acceptance, producer implementation, bootstrap material, deployed connectivity and availability
  conflict: []
accepted_decision: FND-NATIVE-SOURCE-EVIDENCE-V1, conditional Game consumer acceptance only
rejected_options: [credential_or_receipt_as_source, token_directed_trust_fetch, local_reaging, direct_Platform_database_access, unbounded_push_bus_for_first_slice]
affected_contracts: [FND-04_PRE_ADMISSION_GRANT_PROFILE_V1, FND-DUR-FRESH-ADMISSION-V1]
affected_paths:
  - docs/architecture/reviews/OTERYN_GAME_NATIVE_ADMISSION_SOURCE_EVIDENCE_DECISION_2026-09-06.md
  - docs/agents/tasks/active/OTV2-20260906-native-source-contracts-328.md
implementation_owner: Game authenticated ingestion under fresh allocation; Platform counterpart separately authorized and accepted
implementation_scope: bounded private evidence client, sealed registration and acknowledged guard publication
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
supersedes: [only_Game_consumer_deferral_of_evidence_transport_schema_and_bootstrap_in_profile_sections_10_and_13]
required_validation: source_authentication_ordering_restart_and_adoption_matrix_below
required_independent_review: exact-head authentication/provenance review
next_action: Work independently qualifies this Game candidate for protected integration and retains the external counterpart dependency.
```

## Problem, constraints and timing

The existing `Fnd04EvidenceAuthority` and sealed publication ports are capabilities, not actual producers. Game must not fill them with grants, directory projections, caller-created facts or old receipts. Platform's `app/Identity/Actions/RevokeIdentityGameAuthorizations.php` provides reusable transactional security state; `app/GameAuth/Tickets/RedeemedGameLoginTicket.php` and `routes/internal.php` do not provide native observations. These are exact-source findings, not statements about live deployment.

**Must decide now: YES:** actual Child C evidence ingestion and Server Seam admission are blocked. Incorrect bootstrap or cache re-aging would bake an authentication/rollback bypass into durable guards. A bounded request/response contract avoids a new broker and keeps latency off FND-03's writer. Cost: source availability and round-trip latency can close admission; fail-closed rejection is preferable to admitting a revoked account.

Supersession requires reviewed evidence that another transport preserves independent source authentication, ordering, freshness and restart behavior with lower measured cost or necessary failure isolation. Deliberately excluded: grant crypto/claims, existing five-second source-age limit, issuer identity ownership, AccountId semantics, production keys/endpoints, deployment, native ticket workflow redesign, resource values, and character creation policy. Foundation #326 and Child B proceed independently.

## Selected private transport and trust root

Select **request/response HTTPS with TLS 1.3 mutual service authentication and strict versioned JSON**, with no redirect following, public discovery, bearer-grant authentication of the service, or token-directed endpoint/key fetch. This is a Game consumer compatibility requirement proposed to Platform, not unilateral acceptance of Platform's producer obligations.

The client uses an operator-provisioned, non-token-derived service descriptor: exact HTTPS origin/path mapping, allowed Platform evidence service identity, trusted service PKI roots and authorized Game client identity. TLS validates certificate chain, expiry, service identity and the configured peer binding. Platform must independently authorize that client for the exact observation purpose/account request. Network location alone is not authentication. Admission Ed25519 keys are payload being authenticated; they are not this channel's trust root. Client/private key custody stays outside Game data/receipts and requires separate deployment authority.

A descriptor or configured root alone provides no source facts. Readiness requires an actual successful authenticated exchange, a compatible typed response and committed non-rollback publication. The service trust descriptor itself requires an authenticated installation/update and durable monotonic configuration revision; restored lower/unknown descriptor state leaves ingestion closed. Replacing trust roots or service identity is an explicit authorized bootstrap update, never recovery from a token or automatic trust-on-first-use. No credential, certificate issuance, root installation or live endpoint operation is authorized here.

Two private operations are selected: `ReadAccountSecurityV1(AccountId)` and `ReadFreshSigningTrustV1(fixed verifier scope, key_id)`. Their paths are deployment mapping, not game protocol IDs. The request contains only operation/version and the exact typed scope fields; no grant or reusable credential is sent. The trust scope comes exclusively from configured expected issuer/profile/purpose; `kid` is only a bounded index within it. The response is bound to the outstanding request by the authenticated HTTP exchange and exact returned operation/scope equality. No unauthenticated shared cache is accepted.

Alternative considered: separately signed observations through a relay. Deferred because no intermediary authenticity requirement has been demonstrated and it adds signer/distribution/bootstrap machinery. A later change needs explicit counterpart compatibility acceptance. Direct Platform DB reads and arbitrary push events are rejected for this first slice.

## Exact V1 semantic envelope and encoding

Use separate closed typed response families, not a nullable mega-event. JSON decoding rejects duplicate or unknown members, wrong types, noncanonical values and unsupported versions. Version is integer `1`; comparable revision/generation values are positive unsigned 64-bit decimal strings without signs/leading zeroes; Unix seconds are nonnegative decimal strings fitting signed 64 bits. UUIDs use lowercase hyphenated canonical full UUID strings and their existing semantic validation. Public keys use unpadded base64url of exactly 32 Ed25519 bytes. Existing fixed issuer/profile/purpose constants remain unchanged.

| Family | Required fields |
|---|---|
| Common success | `version`, `operation`, `result = "observed"`, `source_authority`, `source_revision`, `decision_identity`, `source_observed_at`, `clock_uncertainty_seconds` |
| Account security | Common plus `account_id`, fixed `purpose`/`scope`, `allowed` boolean, `minimum_valid_generation` |
| Fresh signing trust | Common plus fixed `issuer`/`profile`/`key_purpose`, `key_id`, `trusted` boolean, `public_key` |

`decision_identity` is the canonical decimal source revision scoped by `(source_authority, operation, exact subject/trust scope)`; its reuse denotes exactly the same complete response. For security, the revision namespace is per AccountId/security purpose; for signing trust it is per fixed issuer/profile/key-purpose set, shared across key IDs. Each queried key observation advances that set-wide ordering; a key ID cannot switch to a fresh namespace to evade a newer accepted profile revocation. Source authority is authenticated by the descriptor/peer mapping and must equal the response, not a caller-selected string. A missing/unknown account or unknown key returns a closed `NotFound` result without invented generation/key values; unavailable, unauthorized, malformed and unsupported responses are separate closed failures. Neither `NotFound` nor a transport failure creates an authoritative allow or resets a floor. Explicit denial/untrust observations with known state remain ordered durable decisions. Failure bodies contain exactly `version`, `operation` and `result`, where `result` is one of `not_found`, `unavailable`, `unauthorized`, `unsupported`; all other shapes reject. Transport failures have no trusted result body. No raw upstream diagnostics reach players.

The operation serves one subject, never an unbounded account or key dump. Existing applicable registry ceilings must govern body, identifiers, concurrency, queue and deadlines. A dimension with no applicable accepted finite ceiling remains an explicit implementation qualification prerequisite in `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`; no numeric value is invented here. Unsupported/oversized input fails before expensive decoding or queuing. Selecting a library and binding accepted limits is allocation work, not permission to silently widen this contract.

## Source ordering, observation and recovery

A **new observation** is a Platform-owned transaction: serialize against all mutations of that subject's security or key/profile trust state, read current state and trusted observation time, allocate the next durable observation revision, and retain the exact immutable response/decision through commit before sending success. This observation revision is separate from `game_auth_generation`: unchanged security generation can have a newly observed revision; unchanged revision can never carry a refreshed timestamp. Exact response replay retains the original timestamp and uncertainty. Overflow fails closed.

This is a proposed external producer requirement requiring Platform acceptance. An implementation may optimize physical storage only if it proves equivalent lossless ordering and exact replay. Every relevant disable/revoke/generation/key mutation participates in that ordering; an observed denial cannot be undone by an old allow response. Source time is sampled while current state is protected, not after an unbounded delay; later response/storage time cannot make it fresher.

Game's privileged adapter authenticates the exchange, validates fields and request equality, then constructs sealed evidence. Publication persists the source revision/decision/time and Game high-water marks through B's guarded CAS before normalized completion can activate the projection. Pure verifier lookup reads that acknowledged projection and never blocks on HTTP/SQL. The accepted conservative source age, including uncertainty, must still be at most five seconds at the existing authorization boundaries. A delayed valid response may already be unusable. Evidence refresh is asynchronous and bounded, not a reason to hold the logical writer.

Lower revision rejects; equal revision permits only byte-equivalent semantic replay and never refreshes time. A newer denial fences admission through the same guard serialization as fresh `L`; the existing bounded unseen-revocation window remains unchanged. Game local publication revisions are not Platform source revisions. Outer Game presence changes under #325 preserve nested remote provenance exactly except its permitted local publication wrapper.

Bootstrap requires both authenticated live owner evidence and proof of the non-rollback floor. A genuinely new Game store may initialize only under an independently authorized fresh-store provenance record; absent rows alone are not proof of a fresh store. Restart restores persistent floors/tombstones and authenticates a current observation at or above them before enabling admission. Restoring a backup with uncertain floor requires reconciliation from the authoritative source's retained high-water state. If the source itself was rolled back and cannot prove its previous maximum, it must remain unavailable pending explicit owner recovery; a new service identity/configuration must not silently reset the revision namespace. This decision does not authorize that recovery or erasure of history.

## Delivery authority and qualification

Game acceptance enables allocation of its typed decoder/client/sealed adapter against a separately controlled test producer. It does not mean actual Platform acceptance or availability. Required external dependencies are: Platform-owned native AccountId implementation under accepted ADR 0028; explicit counterpart acceptance of this evidence wire/PKI/ordering/bootstrap contract; compatible native issuer/security/trust producer implementation; and separately authorized runtime provisioning/connectivity proof. Platform files, Issues, comments, contracts, keys and deployment remain read-only under this Game task.

Game ownership/world source implementation remains a separate exact allocation under `CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md`. No fixture or grant is privileged account/character seed truth. `DUR-02_PROFILE_NEUTRAL_CHARACTER_PERSISTENCE_OWNER_BASELINE.md` rule 3 explicitly makes its portfolio guard insufficient to prove Platform account existence or operation authorization. This read-only security/trust contract likewise does not authorize character creation or binding: an authenticated creation-intent contract proving AccountId existence and caller operation permission, plus Game name reservation/quota/policy and versioned starter context, remains the exact separate bootstrap dependency. Neither an `allowed` security observation nor possession of AccountId discharges it. The canonical Character root belongs in the normalized Game owner store, never reconstructed by repurposing B admission guards. This decision does not redesign those product policies or claim that #319 is fully resolved. Child C cannot qualify until real owners, their authorized initialization and every relevant mutation caller are proven. Proposed Game implementation surfaces are a dedicated evidence adapter, Foundation verifier/publication seams and integration tests; Work must resolve exact new filenames and disjoint leases before allocation. Nothing here extends Foundation #326 or Child B's current paths.

Qualification uses `AuthorityInvariant × ConsumerBoundary × MutationOperator`: wrong peer/root/client scope; wrong operation/account/key/purpose; malformed/duplicate/oversized JSON; numeric overflow/mixed identifiers; source rollback; equal revision changed state/time; expired/future/uncertain time; denial races; cache replay; root-descriptor rollback; lost response; Game/source restart and backup restoration; HTTP/SQL delay; and independently current post-commit adoption. Negative tests change one invariant while holding the others valid. Include separate-language golden fixtures, actual authenticated transport, real PostgreSQL guard publication and concurrent producer mutations. Mocks prove only mock behavior. Real PostgreSQL cases must execute in the configured integration harness, not only internal test modules without database configuration. Documentation validation is governance/whitespace/self-review plus independent exact-head review and selected CI; runtime/E2E is NOT_APPLICABLE to this candidate.
