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

### 3.1 Fast reconnect-secret path

Fast reconnect remains game-domain continuity. COMMIT does not invent synchronous Platform dependency or treat a later Platform account-security change as implicit post-admission revocation; that remains a separate fenced game-domain control concern.

It still revalidates all current game-domain facts above.

### 3.2 Reauthenticated recovery-grant path

When PREPARE used `oteryn-reauth-recovery-v1`, COMMIT additionally revalidates within the same authority-changing boundary:

- recovery JWT remains inside its accepted time/skew window;
- RecoveryGrantNonce remains eligible for this exact idempotent transition and has not been consumed by another successful transition;
- current trusted Platform-security evidence is authenticated and within the accepted `<= 5s` freshness bound;
- account remains admissible and `account_security_generation` is not below the accepted minimum/current floor;
- key/profile revocation state still accepts the grant;
- current AccountId→CharacterId ownership still matches.

RecoveryGrantNonce is consumed atomically with the successful authority transition. PREPARE alone never converts an expiring or revoked recovery grant into durable replacement authority.

### 3.3 Failed COMMIT

If any required COMMIT-time condition fails before the authority switch:

```text
connection_generation does not advance
predecessor/current authoritative binding remains current
successor reconnect secret never becomes current proof
prepared candidate becomes aborted/terminal (or expires) under one stable state
no successful RecoveryGrantNonce consumption is recorded
no partial AccountPresence/lease/runtime/session authority mutation commits
no ControlLossEpoch/protection is manufactured
```

The same ReconnectAttemptRef may return an already-committed result when COMMIT previously succeeded, or its stable aborted/expired result when it did not. An aborted candidate is never reinterpreted as fresh authority.

COMMIT revalidation and the authority switch form one linearization boundary against competing reconnect, recovery, takeover, handoff and fencing transitions.

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

`YES` means the semantic choice must be frozen before FND-04 acceptance. `DEFERRED` means the value/mechanism remains intentionally owned by a later evidence gate.

| Material choice | Decide now? | Concrete downstream work blocked | Evidence required for later supersession |
|---|---|---|---|
| Platform attempt authorization vs Oteryn-v2 final gameplay authority | `YES` | native admission/session implementation; Platform producer rollout | owner-approved cross-repository authority ADR + security/migration proof |
| Separate AccountPresenceClaim, CharacterLease, GameSession, TransportBinding and RuntimeScopeAuthority | `YES` | DUR session/lease persistence, runtime recovery, duplicate login | fault-injection/formal concurrency evidence preserving every fence/presence invariant |
| Atomic fresh admission with no externally visible partial authority | `YES` | FND-02 admission messages, DUR transaction design, E2E | equivalent linearizability/reconciliation proof |
| Mutually exclusive fresh-entry vs reauthenticated-recovery credentials | `YES` | producer profiles and game validators | independent security/interoperability evidence for replacement profile |
| Fully specified JOSE `Ed25519`; deprecated `EdDSA` rejected | `YES` | cross-language fixtures and key policy | standards/security/interop evidence plus coordinated profile migration |
| 30s max grant lifetime, 5s verifier skew, <=5s Platform-security evidence age | `YES` for v1 ceilings | producer/consumer acceptance and revocation behavior | measured timing/distribution evidence + threat-model review |
| Fresh-entry route/runtime observation + owner-generation binding | `YES` | Gateway failover routing/admission integration | failover/security proof for safe carry-forward across owner generation |
| AdmissionAttemptRef distinct from GrantNonce/RecoveryGrantNonce | `YES` | producer issuance reconciliation and replay store | cross-system transaction proof with equivalent ambiguity/replay safety |
| PREPARE/COMMIT with COMMIT-time revalidation | `YES` | reconnect messages, session runtime, crash recovery | fault/concurrency proof of alternative with no lost-response/stale-authority takeover |
| Healthy binding non-preemptible by bearer reconnect/recovery proof | `YES` | reconnect/takeover implementation | explicit product/security decision + current-generation migration proof |
| Accepted 2s loss / 5s cleanup / 15s same-session grace / 4s per ControlLossEpoch protection | `YES` | session timers, reconnect UX, gameplay protection | measured fairness/abuse/liveness evidence + owner-approved gameplay revision |
| Post-grace recovery attaches fresh GameSession to same PRESENT_UNCONTROLLED actor | `YES` | actor lifecycle, recovery locator, presence logic | gameplay/durability evidence preserving actor state without logout/reset exploit |
| Exact liveness probe cadence/hysteresis | `DEFERRED` | implementation acceptance only | measured latency/load/loss/scheduler/fault evidence |
| CharacterLease TTL/renew/safety margin | `DEFERRED` | lease implementation acceptance only | datastore/network/clock/failover measurement + split-owner fault injection |
| Prepared/replay/rate/resource hard limits | `DEFERRED` | implementation acceptance | resource/abuse/performance evidence + registry tests |
| Physical persistence/isolation primitive | `DEFERRED` to DUR | durable implementation | DUR transaction/rollback/migration/recovery evidence |
| Concrete crypto/JWT library, KMS/HSM/vendor | `DEFERRED` | implementation/deployment | maintenance/interoperability/security/operations evidence |
| Healthy-session seamless migration protocol/UX | `DEFERRED` | optional future feature only | product need + current-generation authorization/abuse/concurrency evidence |

A later contract supersedes a row only explicitly; historical FND-04 remains provenance.

## 7. Canonical contract-owned failure progression

This section is the sole normative FND-04 progression under `FOUNDATION_ERROR_VOCABULARY.md`.

- `RETRYABLE` — bounded retry only under the exact authority rule in the table;
- `TERMINAL` — current semantic attempt/proof cannot be retried as if still authoritative;
- `SECURITY_TERMINAL` — rejected credential/proof must not be blindly retried/reinterpreted;
- `NO_AUTHORITY_MUTATION` — no new gameplay/session/lease authority committed;
- `COMMITTED_OR_RECONCILE_REQUIRED` — prior success may already exist; reconcile before independent retry;
- `BOUNDED_RENEWAL_ONLY` — retry can preserve only already-current authority before fail-safe deadline and never grants replacement.

| Internal code | Category | Disposition | Retry authority | Mutation / idempotency outcome | Public class |
|---|---|---|---|---|---|
| `ADMISSION_GRANT_MALFORMED` | `INVALID_INPUT` | `TERMINAL` | never same malformed grant; obtain newly issued valid capability | `NO_AUTHORITY_MUTATION` | `RETRY_LOGIN` |
| `ADMISSION_GRANT_AUTHENTICATION_FAILED` | `AUTHENTICATION_FAILED` | `SECURITY_TERMINAL` | never same credential; restart authenticated issuance | `NO_AUTHORITY_MUTATION` | `AUTHENTICATION_REQUIRED` |
| `ADMISSION_GRANT_EXPIRED` | `SESSION_REJECTED` | `TERMINAL` | fresh Gateway/issuer attempt + new grant | `NO_AUTHORITY_MUTATION` | `RETRY_LOGIN` |
| `ADMISSION_GRANT_REPLAYED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | never reuse grant; reconcile prior admission first, then fresh attempt only if no current authority | `COMMITTED_OR_RECONCILE_REQUIRED` | `SESSION_UNAVAILABLE` |
| `ADMISSION_GRANT_SECURITY_STATE_REVOKED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | wait for Platform security authority to permit a newly authenticated attempt | `NO_AUTHORITY_MUTATION` | `AUTHENTICATION_REQUIRED` |
| `ADMISSION_GRANT_SECURITY_EVIDENCE_STALE` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same unconsumed grant only if still valid and other bindings remain current after fresh evidence; else new grant | `NO_AUTHORITY_MUTATION` | `TEMPORARILY_UNAVAILABLE` |
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
| `RECONNECT_GRACE_EXPIRED` | `SESSION_REJECTED` | `TERMINAL` | same-session retry forbidden; use eligible post-grace recovery | `NO_AUTHORITY_MUTATION` | `SESSION_UNAVAILABLE` |
| `RECOVERY_GRANT_MALFORMED` | `INVALID_INPUT` | `TERMINAL` | never same malformed recovery grant; perform new authenticated recovery issuance | `NO_AUTHORITY_MUTATION` | `AUTHENTICATION_REQUIRED` |
| `RECOVERY_GRANT_AUTHENTICATION_FAILED` | `AUTHENTICATION_FAILED` | `SECURITY_TERMINAL` | never same credential/profile/signature; perform new Platform-authenticated recovery | `NO_AUTHORITY_MUTATION` | `AUTHENTICATION_REQUIRED` |
| `RECOVERY_GRANT_EXPIRED` | `SESSION_REJECTED` | `TERMINAL` | never same expired grant; obtain a new recovery grant if actor/session remains recovery-eligible | `NO_AUTHORITY_MUTATION` | `AUTHENTICATION_REQUIRED` |
| `RECOVERY_GRANT_REPLAYED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | never reuse grant; reconcile prior recovery before new authenticated recovery | `COMMITTED_OR_RECONCILE_REQUIRED` | `SESSION_UNAVAILABLE` |
| `RECOVERY_GRANT_SECURITY_STATE_REVOKED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | wait for Platform security authority to permit a new authenticated recovery; never reinterpret as fresh-entry grant | `NO_AUTHORITY_MUTATION` | `AUTHENTICATION_REQUIRED` |
| `RECOVERY_GRANT_SECURITY_EVIDENCE_STALE` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same unconsumed grant only while still within time/profile bounds and after fresh trusted security evidence; otherwise obtain a new recovery grant | `NO_AUTHORITY_MUTATION` | `TEMPORARILY_UNAVAILABLE` |
| `RECOVERY_GRANT_REVISION_UNSUPPORTED` | `UNSUPPORTED_REVISION` | `TERMINAL` | compatible producer/client/consumer recovery profile only; no downgrade or fresh-entry reinterpretation | `NO_AUTHORITY_MUTATION` | `CLIENT_UPDATE_REQUIRED` |
| `RECOVERY_HEALTHY_CONTROLLER_PRESENT` | `CONFLICT` | `TERMINAL` | no bearer-proof takeover; retry only after authoritative loss or separately authorized migration | `NO_AUTHORITY_MUTATION` | `CHARACTER_ALREADY_ACTIVE` |
| `RECOVERY_PLACEMENT_UNAVAILABLE` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same unconsumed grant only while time/security valid; else fresh recovery grant | `NO_AUTHORITY_MUTATION` | `TEMPORARILY_UNAVAILABLE` |
| `RECOVERY_STATE_UNSAFE` | `INTERNAL_UNAVAILABLE` | `TERMINAL` | no same transition retry until server reconciliation establishes safe state | `NO_AUTHORITY_MUTATION` | `SESSION_UNAVAILABLE` |
| `CHARACTER_LEASE_STALE` | `STALE_GENERATION` | `TERMINAL` | stale holder never renews/replaces authority; reconcile current owner/session | `NO_AUTHORITY_MUTATION` | `SESSION_UNAVAILABLE` |
| `CHARACTER_LEASE_RENEW_TIMEOUT` | `TIMEOUT` | `RETRYABLE` | bounded same-current-lease renewal before fail-safe deadline; then fail safe | `BOUNDED_RENEWAL_ONLY` | `TEMPORARILY_UNAVAILABLE` |
| `CHARACTER_LEASE_DEPENDENCY_UNAVAILABLE` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | bounded same-current-lease renewal/reconciliation while safety deadline remains | `BOUNDED_RENEWAL_ONLY` | `TEMPORARILY_UNAVAILABLE` |
| `SESSION_TAKEOVER_NOT_ALLOWED` | `CONFLICT` | `TERMINAL` | fresh takeover only after authoritative eligibility change + fresh authorization | `NO_AUTHORITY_MUTATION` | `CHARACTER_ALREADY_ACTIVE` |

Recovery-profile parser/header/claim/UUID/profile/purpose failures map to `RECOVERY_GRANT_MALFORMED` unless cryptographic/key/trust validation fails, which maps to `RECOVERY_GRANT_AUTHENTICATION_FAILED`. Time expiry maps to `RECOVERY_GRANT_EXPIRED`; account-security revocation/generation denial maps to `RECOVERY_GRANT_SECURITY_STATE_REVOKED`; stale/unavailable-but-recoverable trusted security evidence maps to `RECOVERY_GRANT_SECURITY_EVIDENCE_STALE`; incompatible mandatory profile/protocol semantics map to `RECOVERY_GRANT_REVISION_UNSUPPORTED`. These recovery codes never inherit fresh-entry actions such as obtaining a Gateway route unless a later independent fresh-entry attempt is separately authorized.

No public mapping exposes raw credential validity, security generation, private fence/lease data or combat-sensitive internals. Numeric wire allocation remains later FND-02 registry work and cannot weaken this progression.

## 8. Failure-scenario disposition and implementation evidence

The catalogue scenario:

```text
FS-RECONNECT-PREPARE-COMMIT-ELIGIBILITY-CHANGE
```

is **`PASS` at FND-04 contract level**: Sections 2–3 require COMMIT to atomically revalidate current authority/security and require failed revalidation to leave the predecessor/current authority unchanged with no generation advance, successful recovery-nonce consumption or partial authority mutation.

`PASS` means a contract invariant exists; executable proof remains mandatory before implementation acceptance.

Required implementation evidence includes at minimum:

1. healthy current generation + correct reconnect secret from second transport → PREPARE rejected, incumbent unaffected;
2. healthy current generation + valid recovery grant → PREPARE rejected, incumbent unaffected;
3. concurrent healthy contenders → none prepares without separately authorized healthy migration;
4. eligible declared loss → one contender may PREPARE and exactly one eligible contender may COMMIT;
5. incumbent regains sufficient current-generation control after PREPARE → COMMIT rejected, incumbent remains authoritative;
6. recovery JWT expires/is revoked or Platform-security generation/freshness invalidates after PREPARE → COMMIT rejected with no authority switch;
7. CharacterLease/runtime/session/reconciliation authority changes after PREPARE → stale candidate cannot COMMIT;
8. failed COMMIT leaves predecessor proof/generation/current authority unchanged and candidate non-revivable;
9. crash/lost COMMIT response resolves to exactly predecessor-current or successor-current, never both;
10. healthy migration, if later implemented, uses current-generation authorization and grants no disconnect protection;
11. stale migration authorization from generation N cannot affect generation N+1;
12. stolen predecessor reconnect secret after successful COMMIT cannot regain authority/fence successor;
13. malformed/bad-signature/expired/revoked/stale-security/unsupported recovery-grant cases each follow the recovery-specific Section 7 progression and never silently fall into fresh-entry retry behavior;
14. every Section 7 failure code follows its frozen disposition/retry/idempotency/public mapping in positive, negative and ambiguous-result fixtures.

## 9. Concise rule

```text
healthy current binding
+ reconnect secret / recovery JWT elsewhere
-> NOT replacement authority
-> reject unsolicited PREPARE

server-proven eligible loss
-> PREPARE may reserve one candidate
-> PREPARE grants no authority escrow
-> COMMIT atomically revalidates current authority/security

incumbent recovered
OR grant/security invalidated
OR lease/runtime/session/reconciliation changed
-> no authority switch
-> candidate terminal/aborted
-> predecessor/current authority unchanged

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
