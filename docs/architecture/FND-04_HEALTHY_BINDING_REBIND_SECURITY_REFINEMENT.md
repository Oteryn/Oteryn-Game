# FND-04 — Rebind Security, Decision Timing and Failure Progression Refinement

- Status: Candidate normative FND-04 refinement; canonical when the owning FND-04 delivery merges
- Date: 2026-08-08
- Gate: `FND-04`
- Refines: `FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md`, especially Sections 4, 14, 15, 20, 27, 28, 30 and 33
- Applies to: same-GameSession transport replacement, FND-04 decision timing and contract-owned cross-component failure progression
- Does not authorize: runtime/protocol implementation, transport migration feature, Platform writes or production traffic

## 1. Normative precedence

This document is part of the final FND-04 package, not an optional note.

For the subjects it owns below, it is the **single normative refinement** of the main FND-04 contract:

- Sections 2–5 below own healthy-binding non-preemption and PREPARE→COMMIT revalidation semantics;
- Section 6 is the canonical FND-04 decision-timing matrix;
- Section 7 is the canonical FND-04 cross-component error progression table;
- Section 8 owns the additional PREPARE→COMMIT eligibility-change failure-scenario disposition and required evidence.

The canonical main FND-04 contract and `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` must reference this refinement explicitly. For the subjects above, this refinement supersedes duplicated candidate wording in the main contract when the two differ. In particular, Section 7 below is the authoritative retry/terminal/idempotency/public mapping; a different public mapping in main-contract Section 27 is non-authoritative transitional duplication and must not be implemented.

This precedence rule removes ambiguity without changing the stable symbolic error names or Foundation Error Vocabulary categories already defined by the main contract.

## 2. Security problem and healthy current binding

A reconnect secret is a high-entropy bearer proof. A reauthenticated recovery grant is a stronger Platform-authenticated attempt proof. Neither one, by itself, permits eviction of a healthy current playable transport.

When a GameSession has a current `TransportBinding` whose current `connection_generation` still has accepted sufficient playable-control evidence, an unsolicited PREPARE from another transport MUST be rejected even if the newcomer presents the current reconnect secret, a valid reauthenticated recovery grant, the correct GameSessionId/AccountId/CharacterId and a fresh ReconnectAttemptRef.

A rejected contender:

- creates no prepared authority;
- receives no successor authority;
- does not advance `connection_generation`;
- does not invalidate incumbent proof;
- does not fence/close the incumbent as an authority effect;
- does not create `ControlLossEpoch` or defensive re-entry protection.

An unsolicited recovery PREPARE becomes eligible only after server-authoritative state proves replacement eligibility, including the accepted unexpected playable-control-loss / `RECONNECTABLE` state. Socket closure alone is never authority proof.

## 3. COMMIT revalidates authority; PREPARE is never authorization escrow

A successful PREPARE reserves one candidate transition. Possession of the prepared successor secret proves only possession of that candidate; it does not freeze the authority facts that made PREPARE eligible.

Immediately before and atomically with any authority switch, COMMIT MUST revalidate all applicable current facts:

1. prepared transition exists, is unexpired and is bound to the exact GameSessionId, ReconnectAttemptRef, prepared TLS transport and candidate `connection_generation`;
2. the current predecessor `connection_generation` is exactly the generation from which the transition was prepared;
3. GameSession remains eligible for this exact rebind and has not become `TERMINATING`/`TERMINAL` or entered an incompatible takeover/handoff transition;
4. AccountPresenceClaim still denotes the same CharacterId and no newer account-presence revision supersedes it;
5. CharacterLease generation/current authority remains compatible;
6. RuntimeScopeAuthority, placement and FND-02 command/session/reconciliation state still permit same-session continuation;
7. incumbent current generation has not regained sufficient playable-control authority after PREPARE;
8. same-session grace remains valid where required;
9. no newer fence/ownership/takeover/handoff transition supersedes the candidate.

If the prepared transition itself reaches its bounded expiry before COMMIT, that exact candidate returns the stable `RECONNECT_PREPARED_EXPIRED` progression. It commits no authority mutation and cannot be resumed. When same-session grace and all current authority/loss facts still permit another same-session attempt, the caller may start a **new PREPARE** only after fresh current-state/proof evaluation; this is intentionally distinct from `RECONNECT_GRACE_EXPIRED`, which forbids same-session retry.

### 3.1 Fast reconnect-secret path

Fast reconnect remains game-domain continuity. COMMIT does not invent synchronous Platform dependency or treat a later Platform account-security change as implicit post-admission revocation; that remains a separate fenced game-domain control concern.

It still revalidates all current game-domain facts above.

### 3.2 Reauthenticated recovery-grant path

When PREPARE used `oteryn-reauth-recovery-v1`, COMMIT additionally revalidates within the same authority-changing boundary:

- recovery JWT remains inside its accepted time/skew window;
- RecoveryGrantNonce remains eligible for this exact idempotent transition and has not been consumed by another successful transition;
- current trusted Platform-security evidence is authenticated and within the accepted `<= 5s` freshness bound;
- account remains admissible and `account_security_generation` is not below the accepted minimum/current floor;
- authenticated recovery signing-key/profile trust/revocation evidence is current within the accepted `<= 5s` freshness bound and still accepts the exact grant `kid`, issuer, purpose and profile;
- current AccountId→CharacterId ownership still matches;
- the signed `compatibility_revision` remains supported by the current protocol-major/runtime/content/ruleset/GameSession/reconciliation boundary required for the same-session continuation.

If current recovery signing-key/profile trust/revocation evidence is stale, unavailable, unauthenticated, contradictory or cannot prove current trust, COMMIT fails before the authority switch as `RECOVERY_GRANT_SECURITY_EVIDENCE_STALE`; RecoveryGrantNonce is not consumed, no session/lease/runtime/transport authority changes and whatever authority state is current at revalidation remains unchanged. If fresh current trust evidence explicitly no longer accepts the exact grant, including emergency revocation after PREPARE, COMMIT fails before the authority switch as `RECOVERY_GRANT_AUTHENTICATION_FAILED` with the same no-nonce/no-authority outcome.

If the signed recovery compatibility requirement is unsupported, superseded or changed after PREPARE, COMMIT fails before the authority switch as `RECOVERY_GRANT_REVISION_UNSUPPORTED`; RecoveryGrantNonce is not consumed and no session/lease/runtime/transport authority changes.

RecoveryGrantNonce is consumed atomically with the successful authority transition. PREPARE alone never converts an expiring, revoked, trust-stale or compatibility-stale recovery grant into durable replacement authority.

### 3.3 Failed COMMIT preserves the authority state that is actually current

A failed stale candidate is **non-mutating** with respect to gameplay authority. It never rolls authority back to the PREPARE predecessor and never revives a predecessor that was already fenced, superseded, handed off or made terminal by another valid transition.

If any required COMMIT-time condition fails before this candidate's authority switch:

```text
the candidate connection_generation does not become current
whatever TransportBinding / GameSession / lease / runtime ownership state is current at revalidation remains unchanged
if the PREPARE predecessor is still current, it remains current; if it was already superseded or no current transport exists, it is not revived
successor reconnect secret never becomes current proof
prepared candidate becomes aborted/terminal (or expires) under one stable state
no successful RecoveryGrantNonce consumption is recorded for this candidate
no authority mutation is committed by this failed candidate
no ControlLossEpoch/protection is manufactured by this failed candidate
```

The same ReconnectAttemptRef may return an already-committed result when this candidate previously succeeded, or its stable aborted/expired result when it did not. An aborted candidate is never reinterpreted as fresh authority. Reconciliation reports the actual current authority state; it does not reconstruct PREPARE-time authority as a rollback target.

COMMIT revalidation and the candidate authority switch form one linearization boundary against competing reconnect, recovery, takeover, handoff and fencing transitions.

### 3.4 Post-grace recovery new-session commit is also a current-trust boundary

Post-grace recovery does not use the same-session PREPARE authority switch, but it still creates new control authority and therefore MUST NOT rely on an earlier recovery-validator decision as trust escrow.

Immediately before and atomically with a post-grace new-GameSession/control attachment, the current owner revalidates the recovery JWT time/nonce, authenticated recovery signing-key/profile trust/revocation evidence with accepted age `<= 5 seconds`, current recovery signing-key/profile trust, current Platform-security state, signed compatibility requirement, AccountId→CharacterId ownership, AccountPresenceClaim, CharacterLease/runtime placement, actor `PRESENT_UNCONTROLLED` state and absence of a current playable controller or superseding transition.

If current recovery trust/revocation evidence is stale, unavailable, unauthenticated, contradictory or cannot prove current trust, the post-grace commit fails as `RECOVERY_GRANT_SECURITY_EVIDENCE_STALE`, consumes no RecoveryGrantNonce, creates no GameSession/lease/runtime/transport authority and preserves whatever authority state is current at revalidation. Emergency recovery-key/profile revocation recorded by fresh current evidence after earlier validation but before that commit maps to `RECOVERY_GRANT_AUTHENTICATION_FAILED` with the same no-nonce/no-authority/current-authority-preservation outcome.

Recovery result classification is ordered: if authoritative state shows a healthy current playable controller, the attempt returns the dedicated `RECOVERY_HEALTHY_CONTROLLER_PRESENT` conflict progression before any generic recovery-target fallback is considered. Only after that dedicated conflict is excluded may authoritative state matching neither same-GameSession recovery nor the post-grace existing-actor transition — including an actor that has legally become `ABSENT` — fail as `RECOVERY_TARGET_NOT_ELIGIBLE`. Neither outcome consumes RecoveryGrantNonce or mutates authority, and the no-target fallback is never used to hide a healthy-controller conflict.

## 4. Healthy-session migration is a distinct future transition

FND-04 does not forbid a future seamless migration of a healthy session, but it must not be implemented as unsolicited bearer-secret reconnect.

Any future healthy-binding migration requires authorization rooted in the **current authoritative connection_generation**, for example a server-issued one-time migration challenge acknowledged by the current binding or another separately accepted equivalent proof.

Minimum invariants:

- current binding participates in or explicitly authorizes migration while authoritative;
- authorization binds GameSessionId, current generation, destination attempt and short lifetime;
- one attempt has at most one winner;
- PREPARE gives destination no command/liveness authority;
- COMMIT revalidates current-generation authorization and switches/fences atomically;
- stale authorization cannot preempt a later generation;
- healthy migration creates no ControlLossEpoch or four-second disconnect protection;
- knowing reconnect secret alone cannot manufacture migration authorization.

Exact protocol and UX are deliberately deferred.

## 5. Reconnect-secret theft consequence

A stolen reconnect secret may let an attacker race a legitimate reconnect **after** server-declared eligible loss. It cannot kick a healthy binding and cannot finish a prepared replacement after the incumbent regains sufficient current-generation control.

The one-prepared-rebind rule, COMMIT-time revalidation and one-current-generation invariant determine the post-loss race. A stale/losing proof cannot fence the winner.

Future sender-constrained/PoP reconnect credentials may reduce stolen-bearer risk further but are not required for FND-04 v1 acceptance.

## 6. Mandatory architecture decision timing

Every material FND-04 choice below records all five timing dimensions required by repository architecture discipline: whether the choice must be decided now, concrete downstream work blocked, what becomes harder or impossible if deferred or chosen incorrectly, evidence that may justify supersession, and what is deliberately not decided here.

`YES` means the semantic choice must be frozen before FND-04 acceptance. `DEFERRED` means the value/mechanism remains intentionally owned by a later evidence gate rather than being guessed here.

| Material choice | Decide now? | Concrete downstream work blocked | What becomes harder or impossible later | Evidence required for later supersession | Deliberately not decided here |
|---|---|---|---|---|---|
| Platform attempt authorization vs Oteryn-v2 final gameplay authority | `YES` | native admission/session implementation; Platform producer rollout | moving final authority after deployed credentials/sessions exist would require cross-repository credential/session migration and risks dual security authorities | owner-approved cross-repository authority ADR + security/migration proof | exact API transport, deployment topology and implementation language/library details |
| Separate AccountPresenceClaim, CharacterLease, GameSession, TransportBinding and RuntimeScopeAuthority | `YES` | DUR session/lease persistence, runtime recovery, duplicate login | collapsing scopes/generations later would force persistence and recovery rewrites and could make stale-writer/split-control provenance ambiguous | fault-injection/formal concurrency evidence preserving every fence/presence invariant | physical tables, lock primitives, cache layout and service/process placement |
| Atomic fresh admission with no externally visible partial authority | `YES` | FND-02 admission messages, DUR transaction design, E2E | exposing partial authority first would make rollback/retry semantics observable and can create duplicate presence/session/lease state that is expensive to unwind | equivalent linearizability/reconciliation proof | physical transaction/isolation mechanism, database schema and concrete commit primitive |
| Mutually exclusive fresh-entry vs reauthenticated-recovery credentials | `YES` | producer profiles and game validators | a shared deployed bearer format would couple routing and recovery authority and make later purpose separation a credential migration/downgrade problem | independent security/interoperability evidence for replacement profile | concrete JWT library, key-distribution implementation and future additional credential purposes |
| Fully specified JOSE `Ed25519`; deprecated `EdDSA` rejected | `YES` | cross-language fixtures and key policy | changing deployed algorithm identifiers/key semantics later requires coordinated producer/consumer/key rollover and risks algorithm-confusion compatibility branches | standards/security/interop evidence plus coordinated profile migration | implementation crypto library, KMS/HSM/vendor, key rotation cadence and storage technology |
| 30s max grant lifetime, 5s verifier skew, <=5s Platform-security evidence age and <=5s signing-key/profile trust/revocation-evidence age | `YES` for v1 ceilings | producer/consumer acceptance and revocation behavior | loosening/tightening deployed windows later changes replay/revocation exposure and can break clients/operators relying on old timing behavior | measured timing/distribution evidence + threat-model review | actual shorter producer lifetime, refresh cadence, cache implementation, key-distribution transport and operational SLOs inside these ceilings |
| Fresh-entry route/runtime observation + owner-generation binding | `YES` | Gateway failover routing/admission integration | allowing grants to float across owner generations would embed failover semantics in bearer credentials and make stale-owner rejection harder to retrofit safely | failover/security proof for safe carry-forward across owner generation | concrete runtime-status transport/cache, NodeId placement details and future safe carry-forward mechanism |
| AdmissionAttemptRef distinct from GrantNonce/RecoveryGrantNonce | `YES` | producer issuance reconciliation and replay store | conflating producer idempotency with game consume identity would couple two transaction domains and make ambiguous issuance/consume recovery unsafe to disentangle | cross-system transaction proof with equivalent ambiguity/replay safety | physical encoding/storage/index of replay records beyond profile-required values and retention minima |
| PREPARE/COMMIT with COMMIT-time revalidation | `YES` | reconnect messages, session runtime, crash recovery | rotate-and-forget or PREPARE-time-only authority would make lost-response/crash and stale-authorization races part of deployed semantics, requiring protocol/session migration to fix | fault/concurrency proof of alternative with no lost-response/stale-authority takeover | exact prepared-state persistence, wrapping/encryption and storage technology |
| Healthy binding non-preemptible by bearer reconnect/recovery proof | `YES` | reconnect/takeover implementation | permitting bearer-only eviction would create a security/player-fairness contract that is difficult to revoke without breaking established reconnect behavior | explicit product/security decision + current-generation migration proof | whether seamless healthy-session migration is added and its exact protocol/UX |
| Accepted 2s loss / 5s cleanup / 15s same-session grace / 4s per ControlLossEpoch protection | `YES` | session timers, reconnect UX, gameplay protection | changing player-visible recovery/protection windows later alters combat fairness, abuse surface and compatibility fixtures | measured fairness/abuse/liveness evidence + owner-approved gameplay revision | probe cadence, hysteresis, scheduler margins and other measured implementation timing inside the accepted semantics |
| Post-grace recovery attaches fresh GameSession to same PRESENT_UNCONTROLLED actor | `YES` | actor lifecycle, recovery locator, presence logic | implementing logout/respawn/reset first would make preserving the original authoritative actor later require gameplay/persistence migration and could create exploit-compatible legacy paths | gameplay/durability evidence preserving actor state without logout/reset exploit | recovery-locator transport, persistence schema and exact client UX |
| Exact liveness probe cadence/hysteresis | `DEFERRED` | implementation acceptance only | freezing an unmeasured cadence now can cause false loss classification, excess traffic or later compatibility churn; deferring beyond implementation would leave safety unproven | measured latency/load/loss/scheduler/fault evidence | exact probe interval, missed-probe threshold, STABLE_ACTIVE hysteresis and scheduler margins |
| CharacterLease TTL/renew/safety margin | `DEFERRED` | lease implementation acceptance only | guessed values can create split-writer windows or unnecessary fail-stop; postponing past implementation would make fencing correctness unknowable | datastore/network/clock/failover measurement + split-owner fault injection | exact TTL, renew cadence, safety margin, fail-safe deadline and datastore-specific mechanics |
| Prepared/replay/rate/resource hard limits | `DEFERRED` | implementation acceptance | unbounded/default limits invite memory/abuse failures, while arbitrary early numbers create unnecessary compatibility/operations constraints | resource/abuse/performance evidence + registry tests | exact byte/count/rate/retention maxima and per-scope quotas until measured and registered |
| Physical persistence/isolation primitive | `DEFERRED` to DUR | durable implementation | selecting it here would couple semantic authority to storage prematurely; leaving it undecided after DUR implementation begins would force schema/transaction rework | DUR transaction/rollback/migration/recovery evidence | database schema, isolation/locking strategy, prepared-state/replay tables and recovery encoding |
| Concrete crypto/JWT library, KMS/HSM/vendor | `DEFERRED` | implementation/deployment | premature vendor/library lock-in adds migration/supply-chain cost; deferring through implementation would leave interoperability/key operations unproven | maintenance/interoperability/security/operations evidence | exact library, KMS/HSM, secret-management product, deployment and rotation tooling |
| Healthy-session seamless migration protocol/UX | `DEFERRED` | optional future feature only | designing it now would enlarge v1 attack/protocol surface; adding it later remains possible only because current-generation authorization boundary is preserved | product need + current-generation authorization/abuse/concurrency evidence | whether feature ships at all, message schema, device/path UX and migration-token construction |

A later contract supersedes a row only explicitly; historical FND-04 remains provenance. Deferred rows are not permission for implementation defaults: their named evidence must be accepted before the affected implementation claim can pass.

## 7. Canonical contract-owned failure progression

This section is the sole normative FND-04 progression under `FOUNDATION_ERROR_VOCABULARY.md`.

- `RETRYABLE` — bounded retry only under the exact authority rule in the table;
- `TERMINAL` — current semantic attempt/proof cannot be retried as if still authoritative;
- `SECURITY_TERMINAL` — rejected credential/proof must not be blindly retried/reinterpreted;
- `NO_AUTHORITY_MUTATION` — no new gameplay/session/lease authority committed;
- `COMMITTED_OR_RECONCILE_REQUIRED` — prior success may already exist; reconcile before independent retry;
- `ISSUANCE_OUTCOME_RECONCILE_REQUIRED` — one Platform capability may already have been issued for the same producer attempt reference; reconcile or deterministically retire that attempt before any independent new issuance; no gameplay authority is implied by the ambiguity;
- `BOUNDED_RENEWAL_ONLY` — retry can preserve only already-current authority before fail-safe deadline and never grants replacement.

For FND-04 v1, any shorthand such as `after nbf` or `post-nbf` means **after trusted server time enters the profile's accepted `nbf` skew window**, never literal `now >= nbf`. Both signed v1 profiles use the verifier boundary `now + 5s >= nbf`; before that boundary the grant is `*_NOT_YET_VALID`. The exact profile time equations govern every retry rule and fixture.

| Internal code | Category | Disposition | Retry authority | Mutation / idempotency outcome | Public class |
|---|---|---|---|---|---|
| `ADMISSION_GRANT_MALFORMED` | `INVALID_INPUT` | `TERMINAL` | never same malformed grant; obtain newly issued valid capability | `NO_AUTHORITY_MUTATION` | `RETRY_LOGIN` |
| `ADMISSION_GRANT_AUTHENTICATION_FAILED` | `AUTHENTICATION_FAILED` | `SECURITY_TERMINAL` | never same credential; restart authenticated issuance | `NO_AUTHORITY_MUTATION` | `AUTHENTICATION_REQUIRED` |
| `ADMISSION_GRANT_NOT_YET_VALID` | `SESSION_REJECTED` | `RETRYABLE` | retry the same still-unconsumed grant only after trusted server time enters the accepted `nbf` window and only while `exp`, Platform-security, signing-key/profile trust freshness, route/runtime-generation and all other admission bindings remain current; otherwise obtain a new grant | `NO_AUTHORITY_MUTATION`; GrantNonce is not consumed and no presence/lease/session/transport authority changes | `TEMPORARILY_UNAVAILABLE` |
| `ADMISSION_GRANT_EXPIRED` | `SESSION_REJECTED` | `TERMINAL` | fresh Gateway/issuer attempt + new grant | `NO_AUTHORITY_MUTATION` | `RETRY_LOGIN` |
| `ADMISSION_GRANT_REPLAYED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | never reuse grant; reconcile prior admission first, then fresh attempt only if no current authority | `COMMITTED_OR_RECONCILE_REQUIRED` | `SESSION_UNAVAILABLE` |
| `ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same AdmissionAttemptRef reconciliation/status recovery only; do not mint a second capability or begin an independent new attempt merely because issuance outcome is unknown; after the registered attempt deadline, a new attempt requires deterministic retirement of the old attempt and proof that any possibly issued capability is no longer acceptable | `ISSUANCE_OUTCOME_RECONCILE_REQUIRED`; game authority is unchanged by the ambiguous producer outcome | `TEMPORARILY_UNAVAILABLE` |
| `ADMISSION_GRANT_SECURITY_STATE_REVOKED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | wait for Platform security authority to permit a newly authenticated attempt | `NO_AUTHORITY_MUTATION` | `AUTHENTICATION_REQUIRED` |
| `ADMISSION_GRANT_SECURITY_EVIDENCE_STALE` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same unconsumed grant only if still valid and other bindings remain current after fresh authenticated Platform-security and signing-key/profile trust/revocation evidence; else new grant | `NO_AUTHORITY_MUTATION`; GrantNonce is not consumed | `TEMPORARILY_UNAVAILABLE` |
| `ADMISSION_GRANT_ROUTE_STALE` | `STALE_GENERATION` | `TERMINAL` | fresh Gateway route + new grant; never retarget old grant | `NO_AUTHORITY_MUTATION` | `RETRY_LOGIN` |
| `ADMISSION_GRANT_RUNTIME_GENERATION_STALE` | `STALE_GENERATION` | `TERMINAL` | fresh current-owner evidence + new grant | `NO_AUTHORITY_MUTATION` | `RETRY_LOGIN` |
| `ADMISSION_GRANT_REVISION_UNSUPPORTED` | `UNSUPPORTED_REVISION` | `TERMINAL` | compatible producer/client/consumer revision only; no downgrade | `NO_AUTHORITY_MUTATION` | `CLIENT_UPDATE_REQUIRED` |
| `ADMISSION_ACCOUNT_CHARACTER_CONFLICT` | `CONFLICT` | `TERMINAL` | new attempt only after authoritative ownership/lifecycle change | `NO_AUTHORITY_MUTATION` | `SESSION_UNAVAILABLE` |
| `ADMISSION_INCUMBENT_PROTECTED` | `CONFLICT` | `TERMINAL` | never reuse same grant as takeover; new attempt only after incumbent eligibility changes | `NO_AUTHORITY_MUTATION` | `CHARACTER_ALREADY_ACTIVE` |
| `ADMISSION_CAPACITY_EXCEEDED` | `CAPACITY_EXCEEDED` | `RETRYABLE` | bounded backoff; same unconsumed grant only on same current route while valid, else fresh route/grant | `NO_AUTHORITY_MUTATION` | `TEMPORARILY_UNAVAILABLE` |
| `RECONNECT_PROOF_INVALID` | `AUTHENTICATION_FAILED` | `SECURITY_TERMINAL` | never blind-retry invalid proof; use valid proof or reauthenticated recovery | `NO_AUTHORITY_MUTATION` | `AUTHENTICATION_REQUIRED` |
| `RECONNECT_PROOF_REPLAYED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | reconcile current GameSession/binding; stale proof never reusable | `COMMITTED_OR_RECONCILE_REQUIRED` | `SESSION_UNAVAILABLE` |
| `RECONNECT_SESSION_TERMINAL` | `SESSION_REJECTED` | `TERMINAL` | same GameSession never retries; use eligible fresh-session actor recovery/new login | `NO_AUTHORITY_MUTATION` | `SESSION_UNAVAILABLE` |
| `RECONNECT_GENERATION_STALE` | `STALE_GENERATION` | `TERMINAL` | reconcile current generation; stale generation/proof cannot retry as authority | `NO_AUTHORITY_MUTATION` | `SESSION_UNAVAILABLE` |
| `RECONNECT_ATTEMPT_CONFLICT` | `CONFLICT` | `RETRYABLE` | reconcile current prepared/committed attempt; same ReconnectAttemptRef may fetch stable result; competing attempt waits | `NO_AUTHORITY_MUTATION` or stable prior result | `TEMPORARILY_UNAVAILABLE` |
| `RECONNECT_PREPARED_EXPIRED` | `TIMEOUT` | `TERMINAL` | never resume the expired prepared candidate; if same-session grace and current authority/loss eligibility still permit, create a new ReconnectAttemptRef/PREPARE only after fresh current-state and proof evaluation | `NO_AUTHORITY_MUTATION`; expired candidate cannot advance generation or become current proof | `TEMPORARILY_UNAVAILABLE` |
| `RECONNECT_GRACE_EXPIRED` | `SESSION_REJECTED` | `TERMINAL` | same-session retry forbidden; use eligible post-grace recovery | `NO_AUTHORITY_MUTATION` | `SESSION_UNAVAILABLE` |
| `RECOVERY_GRANT_MALFORMED` | `INVALID_INPUT` | `TERMINAL` | never same malformed recovery grant; perform new authenticated recovery issuance | `NO_AUTHORITY_MUTATION` | `AUTHENTICATION_REQUIRED` |
| `RECOVERY_GRANT_AUTHENTICATION_FAILED` | `AUTHENTICATION_FAILED` | `SECURITY_TERMINAL` | never same credential/profile/signature; perform new Platform-authenticated recovery | `NO_AUTHORITY_MUTATION` | `AUTHENTICATION_REQUIRED` |
| `RECOVERY_GRANT_NOT_YET_VALID` | `SESSION_REJECTED` | `RETRYABLE` | retry the same still-unconsumed recovery grant only after trusted server time enters the accepted `nbf` window and only while `exp`, current Platform-security evidence, signing-key/profile trust freshness and the current recovery/session/actor eligibility remain valid; otherwise obtain a new authenticated recovery grant | `NO_AUTHORITY_MUTATION`; RecoveryGrantNonce is not consumed and no rebind/session/lease/runtime authority changes | `TEMPORARILY_UNAVAILABLE` |
| `RECOVERY_GRANT_EXPIRED` | `SESSION_REJECTED` | `TERMINAL` | never same expired grant; obtain a new recovery grant if actor/session remains recovery-eligible | `NO_AUTHORITY_MUTATION` | `AUTHENTICATION_REQUIRED` |
| `RECOVERY_GRANT_REPLAYED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | never reuse grant; reconcile prior recovery before new authenticated recovery | `COMMITTED_OR_RECONCILE_REQUIRED` | `SESSION_UNAVAILABLE` |
| `RECOVERY_ATTEMPT_RECONCILIATION_REQUIRED` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same recovery `attempt_ref` reconciliation/status recovery only; do not mint a blind second recovery grant or begin an independent recovery attempt merely because issuance outcome is unknown; after the registered recovery-attempt deadline, a new attempt requires deterministic retirement of the old attempt and proof that any possibly issued recovery capability is no longer acceptable | `ISSUANCE_OUTCOME_RECONCILE_REQUIRED`; game authority is unchanged by producer ambiguity | `TEMPORARILY_UNAVAILABLE` |
| `RECOVERY_GRANT_SECURITY_STATE_REVOKED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | wait for Platform security authority to permit a new authenticated recovery; never reinterpret as fresh-entry grant | `NO_AUTHORITY_MUTATION` | `AUTHENTICATION_REQUIRED` |
| `RECOVERY_GRANT_SECURITY_EVIDENCE_STALE` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same unconsumed grant only while still within time/profile bounds and after fresh authenticated Platform-security and recovery-key/profile trust/revocation evidence; otherwise obtain a new recovery grant | `NO_AUTHORITY_MUTATION`; RecoveryGrantNonce is not consumed | `TEMPORARILY_UNAVAILABLE` |
| `RECOVERY_GRANT_REVISION_UNSUPPORTED` | `UNSUPPORTED_REVISION` | `TERMINAL` | compatible producer/client/consumer recovery profile only; no downgrade or fresh-entry reinterpretation | `NO_AUTHORITY_MUTATION` | `CLIENT_UPDATE_REQUIRED` |
| `RECOVERY_TARGET_NOT_ELIGIBLE` | `SESSION_REJECTED` | `TERMINAL` | this recovery transition cannot retry or reinterpret the grant as fresh-entry authority; if fresh login is legally permitted, it requires a separate newly authorized fresh-entry attempt | `NO_AUTHORITY_MUTATION`; RecoveryGrantNonce is not consumed and authoritative absence/non-recovery state remains unchanged | `SESSION_UNAVAILABLE` |
| `RECOVERY_HEALTHY_CONTROLLER_PRESENT` | `CONFLICT` | `TERMINAL` | no bearer-proof takeover; retry only after authoritative loss or separately authorized migration | `NO_AUTHORITY_MUTATION` | `CHARACTER_ALREADY_ACTIVE` |
| `RECOVERY_PLACEMENT_UNAVAILABLE` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same unconsumed grant only while time/security valid; else fresh recovery grant | `NO_AUTHORITY_MUTATION` | `TEMPORARILY_UNAVAILABLE` |
| `RECOVERY_STATE_UNSAFE` | `INTERNAL_UNAVAILABLE` | `TERMINAL` | no same transition retry until server reconciliation establishes safe state | `NO_AUTHORITY_MUTATION` | `SESSION_UNAVAILABLE` |
| `CHARACTER_LEASE_STALE` | `STALE_GENERATION` | `TERMINAL` | stale holder never renews/replaces authority; reconcile current owner/session | `NO_AUTHORITY_MUTATION` | `SESSION_UNAVAILABLE` |
| `CHARACTER_LEASE_RENEW_TIMEOUT` | `TIMEOUT` | `RETRYABLE` | bounded same-current-lease renewal before fail-safe deadline; then fail safe | `BOUNDED_RENEWAL_ONLY` | `TEMPORARILY_UNAVAILABLE` |
| `CHARACTER_LEASE_DEPENDENCY_UNAVAILABLE` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | bounded same-current-lease renewal/reconciliation while safety deadline remains | `BOUNDED_RENEWAL_ONLY` | `TEMPORARILY_UNAVAILABLE` |
| `SESSION_TAKEOVER_NOT_ALLOWED` | `CONFLICT` | `TERMINAL` | fresh takeover only after authoritative eligibility change + fresh authorization | `NO_AUTHORITY_MUTATION` | `CHARACTER_ALREADY_ACTIVE` |

For fresh-entry grants, a valid signature/profile whose trusted-server time is still before the accepted `nbf` window maps to `ADMISSION_GRANT_NOT_YET_VALID`; it is neither malformed nor consumed. Expiry maps to `ADMISSION_GRANT_EXPIRED`.

If Platform fresh-entry issuance may already have succeeded but the producer cannot recover that exact outcome, the attempt maps to `ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED`. Only same-AdmissionAttemptRef reconciliation is retryable; ambiguity never authorizes blind second issuance. A new independent attempt is allowed only after deterministic retirement of the old attempt and proof that any possibly issued capability can no longer be accepted.

If Platform recovery-grant issuance may already have succeeded but the producer cannot recover that exact outcome, the recovery attempt maps to `RECOVERY_ATTEMPT_RECONCILIATION_REQUIRED`. Only reconciliation/status recovery for the same recovery `attempt_ref` is retryable. A blind second recovery grant or independent recovery attempt is forbidden until deterministic retirement of the old attempt and proof that any possibly issued recovery capability is no longer acceptable; the condition never grants gameplay authority or permits recovery-to-fresh-entry reinterpretation.

Fresh-entry signing-key/profile trust/revocation evidence and recovery signing-key/profile trust/revocation evidence have the same `<= 5s` accepted-age ceiling. If freshness cannot be proven or evidence is stale/unavailable/unauthenticated/contradictory, use the purpose-specific `*_GRANT_SECURITY_EVIDENCE_STALE`. If fresh authenticated evidence explicitly marks the exact key/profile unknown/revoked/not trusted, use the purpose-specific `*_GRANT_AUTHENTICATION_FAILED`. Neither class consumes the relevant nonce or mutates authority.

Recovery-profile parser/header/claim/UUID/profile/purpose failures map to `RECOVERY_GRANT_MALFORMED` unless cryptographic/key/trust validation fails, which maps to `RECOVERY_GRANT_AUTHENTICATION_FAILED`. Trusted-server time before the accepted `nbf` window maps to `RECOVERY_GRANT_NOT_YET_VALID`; time expiry maps to `RECOVERY_GRANT_EXPIRED`; account-security revocation/generation denial maps to `RECOVERY_GRANT_SECURITY_STATE_REVOKED`; stale/unavailable-but-recoverable Platform-security or recovery-key/profile trust/revocation evidence maps to `RECOVERY_GRANT_SECURITY_EVIDENCE_STALE`; incompatible mandatory profile/protocol semantics maps to `RECOVERY_GRANT_REVISION_UNSUPPORTED`. A healthy current playable controller maps first to `RECOVERY_HEALTHY_CONTROLLER_PRESENT`; only after that conflict is excluded does authoritative state matching neither legal recovery transition map to `RECOVERY_TARGET_NOT_ELIGIBLE`. These recovery codes never inherit fresh-entry actions such as obtaining a Gateway route unless a later independent fresh-entry attempt is separately authorized.

Prepared-transition expiry is not same-session grace expiry. `RECONNECT_PREPARED_EXPIRED` terminalizes only that prepared candidate and may permit a new PREPARE after fresh evaluation while grace remains valid; `RECONNECT_GRACE_EXPIRED` ends same-session retry eligibility.

No public mapping exposes raw credential validity, security generation, private fence/lease data or combat-sensitive internals. Numeric wire allocation remains later FND-02 registry work and cannot weaken this progression.

## 8. Failure-scenario disposition and implementation evidence

The catalogue scenario:

```text
FS-RECONNECT-PREPARE-COMMIT-ELIGIBILITY-CHANGE
```

is **`PASS` at FND-04 contract level**: Sections 2–3 require COMMIT to atomically revalidate current authority/security/compatibility and require a failed stale candidate to leave the authority state that is actually current at revalidation unchanged, without candidate-generation advance, successful recovery-nonce consumption or partial authority mutation.

`PASS` means a contract invariant exists; executable proof remains mandatory before implementation acceptance.

Required implementation evidence includes at minimum:

1. healthy current generation + correct reconnect secret from second transport → PREPARE rejected, incumbent unaffected;
2. healthy current generation + valid reauthenticated recovery grant → `RECOVERY_HEALTHY_CONTROLLER_PRESENT`, incumbent unaffected, no fallback to `RECOVERY_TARGET_NOT_ELIGIBLE`;
3. current generation healthy + multiple concurrent contenders → none can create prepared state without current-binding migration authorization;
4. server-declared eligible loss → one valid reconnect contender may PREPARE and exactly one may COMMIT;
5. PREPARE accepted after eligible loss, then incumbent regains sufficient current-generation control before COMMIT → COMMIT rejected/candidate terminalized, incumbent unaffected;
6. PREPARE using recovery grant, then grant expires/is revoked/security generation changes or recovery trust/revocation evidence exceeds the 5-second freshness ceiling before COMMIT → COMMIT rejected without candidate authority change; stale/unprovable trust evidence returns `RECOVERY_GRANT_SECURITY_EVIDENCE_STALE`, while fresh current evidence explicitly showing key/profile revocation returns `RECOVERY_GRANT_AUTHENTICATION_FAILED`; neither consumes RecoveryGrantNonce or mutates authority;
7. PREPARE using recovery grant, then signed `compatibility_revision` becomes unsupported/superseded by current runtime/content/ruleset/session/reconciliation compatibility before COMMIT → `RECOVERY_GRANT_REVISION_UNSUPPORTED`, no RecoveryGrantNonce consumption and no candidate authority switch;
8. PREPARE then CharacterLease/runtime/session/reconciliation state changes before COMMIT → stale candidate cannot switch authority;
9. PREPARE then another valid fencing/handoff/takeover/terminality transition supersedes the predecessor → stale COMMIT cannot revive the predecessor or overwrite the newer/no-current-transport authority state;
10. failed COMMIT revalidation leaves whatever authority state is current at revalidation unchanged and the stale candidate non-revivable;
11. PREPARE's own bounded expiry occurs while same-session grace remains valid → `RECONNECT_PREPARED_EXPIRED`; candidate remains non-mutating/non-revivable and only a freshly evaluated new PREPARE may proceed;
12. post-grace recovery passes earlier validation, then recovery signing-key/profile is emergency-revoked before the atomic new-GameSession attachment → `RECOVERY_GRANT_AUTHENTICATION_FAILED`, no RecoveryGrantNonce consumption, no new GameSession/control authority and current authority preserved;
13. post-grace recovery trust/revocation evidence exceeds the 5-second freshness ceiling or cannot be authenticated/proven before attachment → `RECOVERY_GRANT_SECURITY_EVIDENCE_STALE`, no RecoveryGrantNonce consumption, no new GameSession/control authority and current authority preserved;
14. a valid recovery grant resolves to an actor that is legally `ABSENT` or otherwise matches neither recovery transition after the healthy-controller conflict is excluded → `RECOVERY_TARGET_NOT_ELIGIBLE`, no RecoveryGrantNonce consumption/no authority mutation/no recovery-to-fresh-entry reinterpretation;
15. ambiguous Platform admission issuance response after capability creation may have succeeded → `ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED`; only same-AdmissionAttemptRef reconciliation may retry, no blind second capability may be minted, and a fresh independent attempt requires deterministic retirement plus proof that any possibly issued old capability is no longer acceptable;
16. ambiguous Platform recovery-grant issuance response/crash after capability creation may have succeeded → `RECOVERY_ATTEMPT_RECONCILIATION_REQUIRED`; only same-recovery-attempt-ref reconciliation may retry, no blind second recovery grant/independent recovery attempt may start, and a new attempt requires deterministic retirement plus proof that any possibly issued recovery capability is no longer acceptable;
17. both profiles accept authenticated signing-key/profile trust/revocation evidence at exact age `5s`, reject `>5s`/unavailable/unauthenticated/contradictory evidence as purpose-specific `*_GRANT_SECURITY_EVIDENCE_STALE`, and reject fresh explicit unknown/revoked/not-trusted key/profile as `*_GRANT_AUTHENTICATION_FAILED`;
18. pre-loss current-binding-authorized migration, if implemented, switches authority atomically without creating ControlLossEpoch/protection;
19. stale migration authorization from generation N cannot affect generation N+1;
20. stolen predecessor reconnect secret after successful COMMIT cannot regain authority or fence successor;
21. fresh-entry grant with valid signature/profile and `now + 5s < nbf` returns `ADMISSION_GRANT_NOT_YET_VALID` and consumes no GrantNonce; at the first accepted boundary `now + 5s >= nbf`, the same still-unconsumed grant may proceed only while expiry, Platform-security, key/profile trust freshness, route/runtime-generation and every other admission binding remain valid;
22. recovery grant with valid signature/profile and `now + 5s < nbf` returns `RECOVERY_GRANT_NOT_YET_VALID` and consumes no RecoveryGrantNonce; at the first accepted boundary `now + 5s >= nbf`, the same still-unconsumed recovery grant may proceed only while expiry, Platform-security, key/profile trust freshness, compatibility and recovery/session/actor eligibility remain valid;
23. malformed/bad-signature/not-yet-valid/expired/revoked/stale-security/unsupported/no-target/healthy-controller/recovery-issuance-ambiguity cases each follow the recovery-specific Section 7 progression and never silently fall into another recovery result or fresh-entry retry behavior;
24. every Section 7 failure code follows its frozen disposition/retry/idempotency/public mapping in positive, negative and ambiguous-result fixtures.

## 9. Concise rule

```text
healthy current binding
+ reconnect secret / recovery JWT elsewhere
-> NOT replacement authority
-> RECOVERY_HEALTHY_CONTROLLER_PRESENT for recovery
-> reject unsolicited PREPARE

ambiguous fresh-entry grant issuance
-> ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED
-> same AdmissionAttemptRef reconciliation only
-> no blind second capability

ambiguous recovery-grant issuance
-> RECOVERY_ATTEMPT_RECONCILIATION_REQUIRED
-> same recovery attempt_ref reconciliation only
-> no blind second recovery grant or fresh-entry reinterpretation

signing-key/profile trust or revocation evidence
-> authenticated and age <= 5s required
-> stale/unavailable/unprovable => purpose-specific *_GRANT_SECURITY_EVIDENCE_STALE
-> fresh explicit unknown/revoked/not-trusted => purpose-specific *_GRANT_AUTHENTICATION_FAILED

server-proven eligible loss
-> PREPARE may reserve one candidate
-> PREPARE grants no authority escrow
-> COMMIT atomically revalidates current authority/security/key-trust freshness/compatibility

prepared candidate expired
-> RECONNECT_PREPARED_EXPIRED
-> candidate cannot resume
-> new PREPARE only after fresh evaluation if same-session grace still permits

incumbent recovered
OR grant/security/key-trust/compatibility invalidated
OR lease/runtime/session/reconciliation changed
OR a newer valid transition superseded PREPARE
-> no candidate authority switch
-> candidate terminal/aborted
-> actual current authority state remains unchanged
-> no predecessor revival/rollback

post-grace recovery new-session commit
-> revalidate <=5s recovery-key/profile trust/revocation evidence and all current actor/session/security facts atomically
-> healthy controller => RECOVERY_HEALTHY_CONTROLLER_PRESENT
-> after healthy conflict excluded, no legal recovery target => RECOVERY_TARGET_NOT_ELIGIBLE

successful COMMIT
-> exactly one current generation
-> predecessor fenced only inside same atomic authority transition

healthy intentional migration
-> separate current-generation-authorized transition
-> never bearer-secret-only takeover
-> no disconnect protection

cross-component failure
-> stable internal code + foundation category
-> explicit RETRYABLE / TERMINAL / SECURITY_TERMINAL
-> exact retry authority
-> explicit mutation/idempotency outcome
-> bounded public class
```