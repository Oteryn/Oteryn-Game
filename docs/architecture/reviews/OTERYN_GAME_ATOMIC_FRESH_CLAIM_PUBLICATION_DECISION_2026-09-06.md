# Oteryn Game — Atomic Fresh Claim Publication Decision

- Decision ID: `FND-DUR-FRESH-CLAIM-PUBLICATION-V1`
- Date: 2026-09-06
- Status: **CANDIDATE — acceptance requires independent review and protected integration**
- Parent: accepted `FND-DUR-FRESH-ADMISSION-V1`, Issue #313 / PR #317
- Source escalation: [Issue #324](https://github.com/Oteryn/Oteryn-Game/issues/324)
- Decision base: `main@12143ca171832e6c8ff341e266d126cb486515c8`
- `MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`

## 1. Resolution packet

```yaml
classification: ARCHITECTURE_RESOLUTION
repository: Oteryn/Oteryn-Game
main_sha: 12143ca171832e6c8ff341e266d126cb486515c8
source_escalation: 324
blocking_question: How does fresh COMMIT acquire Game claims with owning-source provenance without Durability inventing source metadata?
facts:
  proven:
    - FreshAdmissionCommitRequestV1 contains only an authorization with expected preclaim guards
    - changed acquired Account and Character guards require newer publication and Game source revisions
    - positive fixture commit_sources advances revisions and decision identity outside the production request
    - owning publication construction is sealed but has no atomic fresh-request binding
    - Child A is integrated and released; B and C remain unallocated
  derived:
    - a narrow additive Foundation handoff is necessary before B can implement the accepted atomic boundary
  unknown:
    - actual production owning-source registration and bootstrap readiness tracked separately by Issue 319
  conflict: []
accepted_decision: FND-DUR-FRESH-CLAIM-PUBLICATION-V1, conditional on protected integration
rejected_options:
  - SQL adapter fabricates source revision, decision identity or remote observation time
  - weaken equal-source-revision contradiction rejection
  - independently commit claim publication before or after canonical session effects
  - require complete production source bootstrap before independently useful A-followup and B work
  - redesign accepted fresh linearization or add a durable fresh PREPARE

affected_contracts: [FND-DUR-FRESH-ADMISSION-V1]
affected_paths: see_section_6
implementation_owner: Foundation followup, then Durability B, then Foundation integration C under fresh Work allocations
implementation_scope: sealed conditional claim transition plus its atomic persistence and registered owner consumption
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
supersedes:
  - only the unresolved claim-advance handoff in FND-DUR-FRESH-ADMISSION-V1 sections 5, 6.1 and 10
required_validation: see_section_7
required_independent_review: admission, provenance and durable lifecycle boundary; exact candidate required
next_action: Work independently qualifies and integrates this candidate through the normal protected lifecycle.
```

The existing parent decision is preserved unchanged as history and remains binding, including no fresh durable PREPARE, final guarded decision `L`, commit-before-publish, source anti-rollback, one transport namespace, forward migration and independent adoption. This decision fills its API gap, not a new source-authority or product policy.

## 2. Evidence and timing

**PROVEN** at the decision base: `fresh_admission_durability.rs` defines `FreshAdmissionCommitRequestV1` with just `authorization`; `FreshAdmissionDurabilityFlowV1::begin` constructs that request. `validate_current_guards` rejects changed acquired rows retaining either expected publication revision or expected source revision. `fresh_equal_game_source_revision_cannot_publish_acquired_state` preserves this rule. The positive `commit_sources` fixture manually advances the first two rows. `AdmissionAuthorityPublicationV1::prepare` requires an owning publisher, but no existing request carries its proposed claim changes into the same fresh transaction.

Live Issue #324 and its empty comments were read independently. Live `main` and the allocated architecture branch both resolved to the decision base. Protected closeout #323 and the current allocation checkpoint establish that #321 delivered A and released its custody; they do not allocate B/C. Live #313 is closed completed. Live #319 records unresolved actual source readiness rather than a reason to re-open the accepted atomic design.

**Must decide now: YES.** Child B cannot produce valid adopted acquired guards from the current request without seizing source ownership. Server Seam #247 is transitively blocked.

**Future cost:** allowing unsealed SQL metadata generation or separately committed claims would couple persistence to authentication and create restart/double-owner repair obligations. An additive typed capability costs a small Foundation followup and keeps that dependency explicit.

**Supersession evidence:** an independently reviewed implementation proving equivalent owner-controlled conditional advancement, one atomic session/claim effect set and restart-safe anti-rollback through a smaller boundary may replace the precise handoff. A happy-path fixture or green unrelated CI is insufficient.

**Not decided:** upstream transport/bootstrap topology, new Platform authority, production availability, takeover/transfer policy, resource values, retention horizons, schema beyond the existing forward-migration scope, or new wire IDs. Player benefit is safe single-winner admission and recovery without added network work in the writer; producer cost is a narrow typed handoff rather than a second claim service.

## 3. Selected Foundation capability

Introduce a sealed `FreshAdmissionClaimTransitionV1` (name illustrative, semantics mandatory) prepared by a registered Game owning-source capability and paired with `FreshAdmissionCommitAuthorizationV1` in the production request. The caller cannot construct it from raw publication fields, authorization/audit/receipt data, or implement its owning-source seal. A verified authorization supplies an expected candidate binding; it does not itself confer source registration.

The owning adapter prepares exactly two conditional successor publications: the existing Account guard and Character guard. It resolves independently accepted owner state and binds:

- the complete immutable fresh authorization binding, including replay key, candidate session, account/character/world/channel, acquired lease generation and transport;
- exact prior Account/Character guard keys, authority/purpose, accepted source revision/decision and publication revisions;
- successor outer Game source metadata, chosen by that owner, with a strictly newer source revision and a new decision identity bound to this conditional operation;
- the normal next publication CAS revision, checked for overflow; unchanged eligibility/ownership/security fields plus only the required presence/lease/holder acquisition.

The source decision identity is stable for retries of this proposal and cannot designate different effects. It is not a newly authenticated Platform decision. Source revision ordering is the owner's existing comparable ordering: a proposed successor is not accepted merely because it was reserved or computed. Competing proposals against one predecessor may exist, but only the successful guard CAS may activate one. No accepted source high-water mark is advanced during preparation. Owners must not later reuse an accepted revision for another decision.

The Account outer source remains the registered Game account/presence publisher with its existing authority and purpose. The nested Platform security observation remains unchanged: authority, purpose/scope, source and accepted-source revision, decision and accepted-decision identity, observation timestamp, uncertainty, minimum generation and allowed state retain their authenticated values. Only its local `publication_revision` wrapper is rebound to the new Account guard revision as already permitted by `same_security_observation`. Signing trust and Runtime publications are unchanged by acquisition. Preparation must not refresh either remote observation's age.

The capability is inert and usable only with its exact bound authorization in the typed fresh transaction. It cannot pass through ordinary standalone publication to acquire claims. `begin`/request construction must require the paired capability for a production fresh submit; no fallback constructs successor metadata from historical DTOs. Public getters may expose immutable effects to the trusted adapter without exposing a constructor or registration route.

At `L`, a pure bounded Foundation predicate validates both the original final authorization and the entire transition against the independently locked rows and trusted time. It returns the already owner-authored exact effects for B to persist. It performs no I/O, source call or externally visible mutation under the writer or transaction. The source observation timestamp is the actual owner preparation observation, not a SQL-generated assertion of remote freshness. All existing freshness/deadline rules still apply at `L`. If preparing sufficiently fresh owner evidence is impossible, admission fails closed.

## 4. One transaction and recovery

B acquires the parent decision's full guard, incumbent, replay, candidate and transport serialization protections before `L`. It validates the authorization and the sealed transition against the same locked preclaim state, then atomically persists the two successor guard publications/high-water marks, canonical ACTIVE generation-1 session, immutable fresh receipt and transport reservation. Both claim holders name that canonical session. It neither allocates nor increments source metadata by convenience. SQL can enforce comparisons and persist Foundation-provided values.

The durable receipt/reconciliation evidence must retain the exact accepted claim-transition binding (or an equivalently lossless typed durable representation) so retry cannot substitute different source decisions/effects. Historical restoration of that representation does not recreate a submit capability. Existing exact-binding replay returns the original committed result without applying a second transition; altered immutable transition effects conflict. A source refresh is not permission to replace an ambiguously committed proposal.

All tentative effects roll back together on rejection/abort. No success, owner projection, source high-water advancement, transport install or gameplay authority is published before durable COMMIT. A known abort permits a newly prepared eligible proposal; transaction retry reacquires and revalidates at a fresh `L`. Ambiguous response requires reconciliation of the original binding before another candidate. A conflict does not consume the grant. Source or publication overflow fails closed.

Owner activation is a later normalized completion backed by the committed receipt and independently current projection. Crash after COMMIT is recovered through durable current rows/high-water marks and original receipt, not by replaying an in-memory increment. Concurrent publisher-before-admission invalidates the expected CAS; admission-before-publisher makes the stale publisher lose its CAS. A commit followed by revocation, lease replacement or terminal state may retain historical success but cannot install a stale controller. Adoption retains all existing independently current checks.

## 5. Claim lifecycle completeness

The same ownership rule applies to every actual claim-changing entry point; this does not grant new lifecycle policy or a generic arbitrary mutation capability.

| Operation | Required boundary |
|---|---|
| Fresh acquisition | The sealed two-guard transition and canonical fresh effects commit together. |
| Control loss or reconnect retaining the same holder/lease | Preserve claims and their source metadata; validate the existing session fences. No gratuitous source revision is manufactured. |
| Accepted reconnect replacement, lease advance or holder change | An owner-authorized typed transition changes the affected claims atomically with the matching canonical session effects. Existing reconnect V1/V2 policy remains binding. |
| Terminal release | Owner-authorized clearing of the current matching presence/holder, monotonic source/publication marks and canonical terminal session effect commit together. Old session or generation cannot release a successor claim. |
| Transfer, handoff, denial or other publication changing claims | Use the corresponding accepted fenced session transaction; ordinary publication cannot detach claims from session effects. Unsupported paths fail closed until precisely allocated. |
| Security, eligibility or runtime publication without claim change | Existing sealed publication CAS remains available; it serializes with admission and preserves unchanged claim state. |

No release resets high-water marks, deletes tombstones for rebootstrap, reuses transport references or authorizes receipt-based reacquisition. Any operation affecting both Account and Character changes both atomically. B must audit its existing reconnect/session callers, and C must inventory public producer mutation callers, to close bypasses before qualification. If a needed Foundation lifecycle helper or caller lies outside an allocation, Work grants a narrow amendment first. This requirement must not silently turn the A followup into a whole reconnect redesign.

## 6. Ownership and sequencing

This candidate allocates **no runtime paths**. Work remains the unique mutating control plane and independently integrates the architect's work.

1. After this decision's protected acceptance, allocate a narrow **Foundation A followup** for `apps/game-server/src/foundation/admission_authority_publication.rs`, `apps/game-server/src/foundation/fresh_admission_durability.rs`, and `apps/game-server/src/foundation/fresh_admission_durability_tests.rs`, plus its exact task record. Define the sealed pairing, pure final predicate, immutable transition evidence/replay rules, production submit requirement and negative tests. No SQL, migration, Cargo or source bootstrap is required here. Additional facade/export/lifecycle paths require demonstrated need and a fresh explicit allocation.
2. Allocate **B** only from protected main containing the followup. Its prospective paths remain exactly parent Section 10: `apps/game-server/src/durability/fresh_admission.rs`, `admission_authority_guards.rs`, `admission_journal.rs`, `db.rs`, `mod.rs`, `schema.rs` (all under that Durability directory); `apps/game-server/migrations/0002_fresh_admission_authority.sql`; `apps/game-server/tests/durability_postgres.rs`; `apps/game-server/tests/support/postgres.rs`. Include affected existing lifecycle calls and lossless accepted transition evidence in that bounded adapter/migration work. Never edit released `0001`.
3. **C/source inventory #319** may continue read-only discovery while A followup and B execute. Real producer construction/registration and composition still require separate exact allocations. Parent Section 10 C paths remain `apps/game-server/src/admission_authority.rs`, `apps/game-server/src/lib.rs` (serialized composition export), `apps/game-server/src/foundation/admission_authority_publication.rs`, `fnd04_verifier.rs`, `admission_facade.rs` (the latter two in the same Foundation directory), `apps/game-server/tests/admission_authority_postgres.rs`, and `apps/game-server/tests/support/postgres.rs`. C binds the actual claim owner, proves restart/activation and closes all producer bypasses. Missing sources keep readiness closed; they do not block isolated A/B proof.
4. Only Work may resume preserved Server Seam `9370b254c6ac4f6529e069c1968ae6bfa1e1750e` after all dependencies and actual C readiness are protected-main proven.

The architecture author's only owned paths are this decision and `docs/agents/tasks/active/OTV2-20260906-atomic-fresh-claim-publication-324.md`. Historical #313/#318 records and shared allocation overlays remain with their owners. No external-repository, production, listener or merge authority is added.

## 7. Required qualification

Use `AuthorityInvariant × ConsumerBoundary × MutationOperator`, independently controlled current sources, and one changed invariant per negative case.

A followup must demonstrate sealed construction (including compile-fail raw/receipt reconstruction), wrong owner/key/purpose, missing or partial transition, changed candidate/replay/transport, stale predecessor, wrong holder/lease, source rollback/equal-revision contradiction, decision reuse with different effects, publication overflow, observation/deadline expiry, and remote security provenance substitution rejection. The positive production-shaped fixture must submit a real owner-prepared capability instead of mutating `commit_sources` after the fact. Preserve current equal-revision rejection, historical-only restart and independent adoption tests.

B requires real isolated PostgreSQL proof of all-or-nothing claim/source/session/receipt/reservation writes; rollback after each tentative effect; concurrent same-account and same-character proposals; publisher-before/admission-before races; exact and conflicting retries; lost COMMIT response and restart reload; fresh/reconnect transport collisions; unchanged claims on control loss; accepted holder/lease advance; stale-generation release rejection; terminal release followed by new admission; and historical replay after reconnect/terminal state without reacquisition. Source marks and accepted decisions survive restart and cannot be regenerated from receipt-only current facts. Existing reconnect V1/V2 and migration compatibility remain required.

Real PostgreSQL evidence must execute in the enforced PostgreSQL integration harness. The current PR/Merge Queue PostgreSQL step selects `--test durability_postgres`; internal `#[cfg(test)]` fixtures or ordinary workspace tests without PostgreSQL configuration do not by themselves satisfy this gate. The implementation allocation must identify the executed cases and exact run evidence; this decision does not select new harness mechanics.

C requires actual registered owner inputs and commit-before-publish activation/reconciliation, independently manipulated source negatives, and proof that every claim-changing entry point crosses the matching session transaction. Fixtures establish only fixture behavior, never production source availability.

For this documentation candidate: governance and diff checks, whole-candidate self-review, genuinely independent exact-head authority/provenance review, selected repository CI and normal protected integration. Runtime/E2E is not applicable to the documentation change; it remains mandatory for the assigned implementation boundaries. Green CI does not replace the independent architecture review.
