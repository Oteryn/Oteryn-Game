# Existing-actor recovery evidence transport amendment

- Decision: `FND-RECOVERY-SOURCE-TRANSPORT-V2`
- Status: **CANDIDATE; conditional Game consumer acceptance only**
- Escalation #336; source inventory #319; coordinator #162
- Base: `b8ae4c965cc7f686b89b4d5c0ba2bc04af6e07fd`
- `MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`

## Resolution packet

```yaml
classification: ARCHITECTURE_RESOLUTION
repository: Oteryn/Oteryn-Game
main_sha: b8ae4c965cc7f686b89b4d5c0ba2bc04af6e07fd
source_escalation: 336
blocking_question: How may the accepted native evidence transport register ExistingActorRecovery sources without relabeling FreshAdmission observations?
facts:
  proven:
    - recovery profile sections 12 and 13 already require authenticated source provenance, freshness and anti-rollback
    - sections 18 and 19 require current evidence again at recovery mutation
    - accepted 330 selected operations and trust scope are fresh-admission scoped
    - legacy recovery compatibility DTOs are not a complete authenticated source capability
  derived: [pure_typed_recovery_semantics_follow_existing_authority, actual_transport_requires_explicit_versioned_consumer_compatibility]
  unknown: [Platform_counterpart_acceptance_implementation_bootstrap_and_availability]
  conflict: []
accepted_decision: FND-RECOVERY-SOURCE-TRANSPORT-V2, conditional Game consumer acceptance only
rejected_options: [relabel_fresh_evidence, trust_legacy_DTO_as_source, token_selects_trust_scope, reuse_fresh_signing_key_purpose, unilateral_Platform_acceptance]
affected_contracts: [FND-NATIVE-SOURCE-EVIDENCE-V1, FND-04_REAUTHENTICATED_RECOVERY_GRANT_PROFILE_V1]
affected_paths: [this_amendment, OTV2-20260906-native-source-bounds-recovery-336_task]
implementation_owner: separately allocated Game recovery adapter; separately authorized Platform producer counterpart
implementation_scope: explicit recovery observation operations and sealed typed ingestion
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
supersedes: [only_330_fresh_only_operation_allowlist_and_its_extension_to_shared_strict_security_currentness_for_explicit_V2_consumers]
required_validation: version_scope_provenance_freshness_namespace_and_current_authority_matrix
required_independent_review: exact-head authentication_and_compatibility_review
next_action: Work independently qualifies this Game amendment and retains external counterpart acceptance as a separate dependency.
```

## Timing and preserved authority

**Must decide now: YES** for actual recovery transport/producer registration. A pure sealed semantic `ExistingActorRecovery` source/result can already be allocated under the accepted recovery profile without a new gameplay decision. It must retain authenticated provenance and derived deadlines; the old compatibility DTO cannot supply missing facts. This amendment prevents an implementation from silently extending fresh service operations to a different trust purpose.

Preserve recovery profile crypto, identity, fixed issuer/profile/purpose, source freshness and error precedence. No new grace, lease, credential lifetime, protection rule or post-grace implementation is selected. #334 is an independent candidate at this admission base and is not treated as accepted authority or an implicit dependency for ordinary recovery evidence. B and pure recovery semantics continue independently.

Trade-off: two explicit operations and an additional fixed trust registration cost some plumbing, but make cross-purpose substitution detectable. Alternate option—one generic operation with caller-selected scope—is rejected because it broadens the attack surface and invites accidental fresh/recovery reinterpretation. Future consolidation requires accepted compatibility evidence, not a convenience refactor. Supersession requires reviewed proof of equivalent scope isolation, ordering and current-authority checks. Deployment/bootstrap availability and character operation authorization remain undecided dependencies.

## Additive versioned operations

Keep #330's existing V1 operations and response shapes unchanged. Add exactly two V2 operations, each mapped explicitly in the authenticated descriptor; no fallback from unknown V2 to V1:

| Operation | Exact request fields | Successful response additions to common V2 envelope |
|---|---|---|
| `ReadRecoveryAccountSecurityV2` | `version = 2`, `operation`, `account_id`, `purpose = "platform_security"`, `scope = "existing_actor_recovery"` | Same exact account/purpose/scope, `allowed` boolean, positive `minimum_valid_generation` |
| `ReadRecoverySigningTrustV2` | `version = 2`, `operation`, fixed `issuer`, `profile`, `key_purpose`, bounded `key_id` | Same exact issuer/profile/key-purpose/key-id, `trusted` boolean, Ed25519 `public_key` |

Common successful fields are exactly `version = 2`, `operation`, `result = "observed"`, `source_authority`, `source_revision`, `decision_identity`, `source_observed_at`, `clock_uncertainty_seconds`. All common and operation-specific fields are mandatory; duplicate/unknown/missing members reject. Failure body is exactly `version = 2`, `operation`, `result`, with the unchanged closed results `not_found | unavailable | unauthorized | unsupported`. An unknown account/key supplies no invented generation/key data. Transport failure has no authenticated body.

The recovery trust values are fixed by the existing profile: issuer `urn:oteryn:platform:game-recovery`, profile `oteryn-reauth-recovery-v1`, and distinct key-purpose `existing_actor_recovery`. Their typed scope is `Fnd04EvidenceScope::ExistingActorRecovery`. The configured expected verifier context chooses them before parsing an untrusted credential. `key_id` remains only an index within that configured recovery set. No fresh/OAuth/service key purpose is accepted in its place.

Reuse #330's exact canonical encodings, including full UUID identity, positive uint64 decimal strings, signed-64 nonnegative source timestamps, unsigned-64 uncertainty strings allowing zero with checked arithmetic, and 32-byte public keys. The wire envelope version is a service-compatibility version, not a change to the recovery JWT profile. No grant, reusable credential, GameSession claim or actor placement is sent to the evidence service. Service authentication/authorization is independent of the user's recovery credential.

Use the same independently bootstrapped mutually authenticated TLS 1.3 service trust boundary as #330. An authorized descriptor may explicitly register these recovery operations for a compatible peer; the peer must independently authorize the Game consumer for them. If the companion resource decision is accepted and registered, its HTTP/1.1 first-slice envelope and four-operation/two-exchange process budget apply across V1 and V2 together, not once per version. Without an accepted complete limit mapping, actual adapter acceptance remains blocked. This amendment is separable from a particular numeric resource choice.

## Ordering and cross-purpose isolation

Each returned observation is a newly authenticated owner transaction under the existing #330 ordering/commit-before-response rule, or exact immutable replay retaining its original time. Operation/version/scope is part of the immutable response binding. A fresh response cannot be reserialized as recovery at the same revision, and a legacy compatibility DTO cannot be enriched by inventing source revision, decision identity, time or accepted floor.

Platform account enabled state and security-generation floor remain one owner state. V1 fresh and V2 recovery account observations use the **same comparable account-security source revision namespace for the same authenticated authority and AccountId**, while retaining their explicit consumer operation/scope. Every new observation, of either operation, advances that shared ordering; exact revision replay retains exactly the original complete response. Game must enforce the shared account-security high-water/deny floor across both consumers. A newer accepted recovery denial cannot be bypassed by an older fresh allow, or conversely. Sharing the ordering does not let one consumer manufacture the other's typed observation or change its provenance.

**Explicit currentness and availability disposition:** strict source revision equality to the currently accepted shared account-security floor is retained. If fresh observation N is followed by accepted recovery observation N+1, fresh N becomes stale even when enabled state/generation did not change. A later fresh N+2 similarly makes recovery N+1 stale. Neither a compatibility helper nor a same-decision-content exception may authorize the lower revision. The higher response retains its own operation/purpose provenance and cannot fill the other typed slot. This is deliberate cross-purpose invalidation, not simultaneous fresh/recovery availability.

The first slice requests security evidence only for the currently selected Game-authorized admission/recovery path; it must not fetch both purposes as mandatory prerequisites for one authorization. Existing independently resolved Game ownership/presence/controller dispatch selects the account-security path: fresh entry requires the accepted idle/no-incumbent conditions, whereas existing-actor recovery requires the accepted current actor/presence and recovery eligibility. A peer request or unverified profile does not choose that authority. Fixed-context signing-trust lookup and credential verification retain their accepted ordering; these Game eligibility checks do not replace final security/ownership revalidation. This amendment introduces no user-selectable authority switch. With a stable selected path and no intervening newer observation, its newly published evidence can authorize within existing freshness bounds. Competing cross-purpose requests may make either candidate stale and must return that existing fail-closed outcome. There is no automatic alternating refresh/retry loop or guarantee of simultaneous success under contention. The bounded pipeline/queue limits constrain retained work. This availability cost is accepted to preserve strict anti-rollback and avoid weakening #330/profile currentness; any future multi-purpose observation or safe concurrent-use design requires a separate explicit contract/producer compatibility decision.

The narrow #330 extension is that its fresh-only per-account security currentness now participates in this shared strict V1/V2 floor for registrations enabling recovery. Fresh-only deployments remain unchanged. Tests must show both invalidation directions on unchanged allow state, newer denial/generation rejection, and a successful single selected-purpose path. No claim is made that the current implementation already has this arbitration or shared-floor enforcement.

Signing trust remains a **distinct recovery issuer/profile/key-purpose namespace**, shared across key IDs within that recovery set, separate from the fresh signing set. A key ID cannot reset the recovery set's high-water mark. No assertion is made that revoking an unrelated fresh key revokes a recovery key; global service bootstrap revocation is enforced independently by the authenticated channel configuration. These namespace rules preserve #330's source ownership and the accepted separation of key purposes.

Ingestion verifies peer, exact operation/version/scope, source identity, monotonic revision/decision and actual observation time/uncertainty before constructing the sealed recovery capability. It persists acknowledged source/floor state before exposing a current projection; the pure verifier does not wait on HTTP/SQL in the logical writer. A normalized completion may update current evidence only with authentic owning-source provenance. Publication cannot turn receipt/cache timestamps into source time.

The existing conservative ≤5-second freshness applies to each source observation. Revalidation at PREPARE/final COMMIT and independently current adoption remains required by the owning recovery flow; earlier signature success is not escrow. Checked source deadline derivation may reject an encoded u64 uncertainty that cannot safely convert or fit the accepted freshness window. New evidence never rewrites an old attempt's immutable binding/deadline. This transport amendment does not define that attempt's timing representation.

## Restart, migration and counterpart dependencies

Retain durable shared security and recovery-trust high-water marks, denials/tombstones and authenticated configuration revision. Missing or regressed floors fail closed; restart authenticates current owner observations and reconciles prior trusted state before capability activation. A new V2 adapter starts from the shared existing security floor, not a fresh zero namespace. Unknown recovery-trust history requires authoritative bootstrap/non-rollback evidence, not absence of a local row. Do not purge historical V1 observations to enable V2.

Game compatibility rollout is additive: accept/implement readers and explicit registration, prove producer compatibility, then enable the specific V2 operations. Unsupported old peers fail closed for recovery while unchanged V1 behavior remains usable where independently valid. Rollback disables V2 ingestion without deleting its floors or translating its observations to fresh; same-account V1 authorization must still honor any newer shared security floor learned through V2.

Game docs cannot make Platform's side accepted or authorize its code, Issues, contracts, PKI or deployment. External requirements remain: native AccountId producer under accepted ADR 0028; explicit Platform acceptance of V2 operations/shared-security ordering/recovery trust/bootstrap; compatible implementation; and separately authorized real provisioning/connectivity proof. Pure semantic tests or configured descriptors do not establish these facts.

Game character creation/binding still needs authenticated operation permission/account-existence evidence and its normalized owner state, name/quota/policy/starter context. An allowed recovery security observation is not that permission. C readiness and Server Seam release remain dependent on actual registered owners and independent qualification, not this amendment alone.

## Qualification and implementation custody

Work must allocate exact Game semantic/adapter/publication/tests and any forward storage changes after current ownership/schema readback. Existing Foundation/B/#334 paths are not granted here. Legacy compatibility remains non-production where it lacks the required evidence; no broad weakening of fresh seals or generic source constructor is allowed.

Use `AuthorityInvariant × ConsumerBoundary × MutationOperator`: fresh-to-recovery and recovery-to-fresh substitution; wrong operation/version/account/purpose/key-set/peer; duplicate/malformed/oversized inputs; shared security rollback across V1/V2; N/N+1/N+2 purpose alternation yielding explicit stale rather than a refresh loop; stable selected-purpose success; recovery key-ID namespace switching; equal revision changed scope/time; stale/future/uncertain evidence; authenticated deny; missing restart floor; rollback from V2 retaining security restrictions; and state changes between verification, PREPARE, COMMIT and adoption. One changed invariant per negative case. Prove exact replay without re-aging and current typed sources without DTO reconstruction. Include independent cross-language wire fixtures, actual mTLS transport, real configured PostgreSQL integration tests and actual producer mutations. Runtime/E2E is NOT_APPLICABLE to this docs-only candidate; independent exact-head review and canonical CI remain required.
