# PROD-ENTITLEMENTS-01 — Oteryn-v2 Game Consumer / Enforcement Contract Candidate

- DecisionStatus: `CANDIDATE`
- DeliveryStatus: `OPEN`
- ImplementationStatus: `NOT_STARTED`
- Date: 2026-08-17
- Gate: `PROD-ENTITLEMENTS-01`
- Oteryn-v2 issue: #115
- Worker task: `OTV2-20260817-prod-entitlements-115-consumer-contract`
- Worker merge authority: `ARCHITECTURE_COORDINATOR_ONLY`
- Runtime / activation authority: **NONE**
- Exact producer baseline: `blakinio/Oteryn-Platform@afaa6d1d8340e44b1152b62d6d27e5fd1649804a`
- Exact producer contract: `docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md` at that merge
- Existing Oteryn-v2 dependency baseline: `docs/architecture/PROD-ENTITLEMENTS-01_PLATFORM_GAME_ENFORCEMENT_DEPENDENCY.md`
- Consumes without redefining: FND-04, DUR-02, DUR-03, ANL-01 and Platform commercial-entitlement authority

> This is a worker **candidate**, not canonical acceptance. It authorizes no Premium/VIP/game-consumed entitlement runtime, product activation, Platform mutation, persistence schema, protocol endpoint, production rollout or payment operation.

## 1. Problem

Platform owns commercial entitlement lifecycle; Oteryn-v2 must eventually apply or deny gameplay effects derived from that lifecycle without becoming payment/order authority and without allowing an old `active` decision to survive indefinitely through outage, replay, reconnect, restart, rollback or unsafe clocks.

The producer side is already repaired and immutable at the pinned Platform merge. The remaining Oteryn-v2 problem is to define one executable game-consumer boundary that answers, deterministically:

1. which producer evidence Oteryn-v2 may accept;
2. how accepted evidence is fenced durably against rollback/resurrection;
3. when a Profile-B benefit is currently usable;
4. what happens at fresh admission, reconnect/recovery and during an already-running session;
5. how game-affecting delivery retries remain idempotent and reconcilable;
6. how producer and consumer revisions can be rolled out or rolled back without reintroducing unbounded authority.

## 2. Constraints and accepted authority split

### 2.1 Platform remains commercial authority

Oteryn-v2 consumes but does not reinterpret these producer-owned facts:

- `EntitlementId` and commercial entitlement lifecycle;
- product and product version;
- grant/activation/expiry/revocation decisions;
- producer lifecycle and authority revisions;
- producer-issued `effective_from`, `effective_until`, `authority_issued_at` and finite `authority_valid_until`;
- the product/version finite authority policy, including maximum authority lease, refresh point, maximum clock skew and whether bounded stale use is permitted.

A game-side cache, projection, persistence row, timer or session is **evidence about** Platform authority. It never becomes a second commercial authority.

### 2.2 Oteryn-v2 remains gameplay authority

Oteryn-v2 owns:

- whether current authoritative gameplay may use an entitlement-derived capability;
- game-state eligibility for a grant/service effect;
- authoritative gameplay mutation and durable game result;
- game-side idempotency, concurrency, fencing and reconciliation;
- stricter fail-closed consumer policy where it does not extend Platform authority.

Payment success, an active entitlement, a delivery request and a gameplay receipt remain different facts.

### 2.3 FND-04 remains session/admission authority

This contract does not redefine `GameSessionId`, `CharacterLease`, reconnect proof, recovery grants, `connection_generation`, PREPARE/COMMIT or actor continuity. Entitlement evidence is not a session credential and a session credential is not entitlement evidence.

The accepted exact four-second defensive PvE reconnect protection is unrelated to commercial entitlement grace and may never be reused as such.

## 3. Options considered

### Option A — Stateless fresh query before every entitlement use

Benefits:

- minimal local entitlement state;
- conceptually simple while Platform is healthy.

Costs / rejection reason:

- cannot safely preserve a monotonic high-water fence across process restart/rollback;
- makes every gameplay decision synchronously dependent on Platform availability;
- still needs replay, duplicate and clock handling;
- does not satisfy the accepted durable anti-resurrection obligation.

**Disposition: REJECTED.**

### Option B — Local cache with receipt-time TTL

Benefits:

- cheap availability during short outages.

Costs / rejection reason:

- cache write/read/restart can accidentally reset the TTL;
- receipt time can incorrectly become a new commercial start or lease origin;
- an old `active` snapshot may outlive a newer revoke/expiry after rollback.

**Disposition: REJECTED.**

### Option C — Durable monotonic consumer fence + producer-issued finite authority interval

Oteryn-v2 stores enough crash-consistent high-water evidence to reject old revisions, evaluates only producer-issued absolute authority bounds using conservative trusted time, and separates the current entitlement classification from transport health.

Benefits:

- survives reconnect/restart/cache replay/projection rollback;
- permits bounded degraded availability without infinite grace;
- preserves Platform commercial authority and Oteryn gameplay authority;
- supports deterministic security testing and safe rollout.

Costs:

- requires a durable fence implementation and explicit product/version compatibility policy;
- can intentionally deny paid benefits when authority/time cannot be proven safely;
- rollout/rollback is stricter than ordinary cache configuration.

**RECOMMENDATION: Option C.** It is the only option consistent with the accepted producer contract and Oteryn-v2 security dependency.

## 4. Exact producer baseline and compatibility identity

The first consumer implementation may target only the producer semantics accepted at:

```yaml
producer_repository: blakinio/Oteryn-Platform
producer_merge: afaa6d1d8340e44b1152b62d6d27e5fd1649804a
producer_contract: docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md
producer_repair_pr: 968
producer_repair_final_head: 27414684ceb77700c7bbf7c6a047c6f3c0c79ad9
historical_security_finding: OPA-SEC-0007 / issue 944
```

A later producer revision is not automatically compatible merely because it has a newer Git commit. Before Oteryn-v2 accepts a different producer semantic/profile revision, a compatibility record must bind:

```text
producer semantic/profile revision
+ exact immutable producer repository revision
+ supported Oteryn-v2 consumer semantic/profile revision
+ affected product/version set
+ compatibility classification
+ rollout order
+ rollback order
```

Unknown required producer semantics fail closed for entitlement-derived benefit. A Git SHA is immutable provenance, not a numeric semantic ordering field.

## 5. Producer authority representation consumed by Oteryn-v2

For Profile B, Oteryn-v2 requires the producer-defined semantic fields equivalent to:

```text
AccountId
EntitlementId
product_id
product_version
entitlement_state
lifecycle_revision
authority_revision
effective_from
effective_until
authority_issued_at
authority_valid_until
```

Before such evidence can affect gameplay it must also be bound, directly or through authenticated integration context, to:

- authenticated Platform producer/source identity and authorized purpose/scope;
- one supported producer semantic/profile revision and exact compatible producer revision;
- the exact product/version authority policy;
- canonical target/account scope;
- bounded schema/size/version validation;
- replay-resistant transport/security context when a transport exists.

Exact wire encoding, transport, storage schema and cryptographic primitive remain deliberately deferred.

## 6. Game-side consumer fence

### 6.1 Fence identity

The durable logical fence is keyed by the canonical entitlement identity and its immutable target/provenance binding, at minimum:

```text
AccountId + EntitlementId
```

The first accepted evidence binds the expected product/product_version/target semantics for that entitlement. A later authenticated representation that reuses the same `EntitlementId` with contradictory immutable provenance is conflicting authority evidence and fails closed.

### 6.2 Ordered high water

Within one entitlement, accepted producer evidence is ordered by:

```text
lifecycle_revision
then authority_revision within that lifecycle revision
```

Rules:

1. a higher valid `lifecycle_revision` supersedes every lower lifecycle revision;
2. within one lifecycle revision, a higher `authority_revision` supersedes every lower authority revision;
3. byte/semantic-equivalent replay of the exact same ordered authority is idempotent;
4. the same ordered revision with contradictory state, target, product/version, interval or cutoff is an authenticated conflict and entitlement-derived benefit fails closed until a strictly newer valid producer decision or explicit recovery reconciliation resolves it;
5. older evidence never lowers the durable high-water fence;
6. a strictly newer authenticated Platform lifecycle decision may supersede an older restrictive decision only because Platform itself issued the newer commercial truth; Oteryn-v2 never synthesizes reactivation or revision advancement locally.

### 6.3 Fence-before-authorize invariant

A permissive authority transition must not become authoritative gameplay input until Oteryn-v2 has crash-consistently advanced the high-water fence enough that restart, snapshot restore or projection rollback cannot resurrect an older state.

A newly observed restrictive fact may deny benefit immediately before its durable write completes, because fail-closed narrowing is safe. However, if the durable fence cannot be advanced/reconstructed, Oteryn-v2 must not later resume entitlement-derived benefit from older persisted `active` evidence. Recovery remains fail closed until monotonic authority is re-established.

No successful cache read, reconnect, process start or transport retry resets `effective_from`, `authority_valid_until`, lifecycle revision or authority revision.

### 6.4 Persistence failure

If the durable fence owner is unavailable:

- do not accept a new permissive authority state for gameplay use;
- continue to apply any already proven stricter state or finite cutoff;
- treat inability to prove the required fence after restart/recovery as entitlement-authority unavailable/conflicting for benefit purposes;
- never weaken the rule by minting a local replacement revision or lease.

Exact PostgreSQL/table/transaction design is a DUR-02/implementation concern, not frozen here.

## 7. Trusted time and conservative interval evaluation

Producer timestamps are Platform-authority-issued facts. Browser/client/device clocks have no authority.

For one accepted representation let trusted current server time be known only within:

```text
[now_lower, now_upper]
```

where uncertainty is finite and acceptable under the exact product/version `max_clock_skew` or an equivalently strong accepted authority-time mechanism.

Evaluation is deliberately asymmetric:

- **not-before:** `now_lower` must be at or after `effective_from`; otherwise state is `NOT_YET_EFFECTIVE`;
- **not-after:** if `now_upper` reaches/passes `effective_until` when finite, or reaches/passes `authority_valid_until`, authority is `EXPIRED`;
- known uncertainty only shrinks the usable interval; it never starts a benefit early or extends the cutoff;
- if time uncertainty exceeds the product/version maximum and no accepted bounded authority-time/monotonic anchor can safely evaluate both boundaries, entitlement-derived benefit fails closed;
- receipt time, process start time, cache refresh time and reconnect time never become a replacement commercial start or lease origin.

A future authority-time recovery mechanism must be separately evidenced. It may restore evaluability but may not rewrite Platform-issued absolute boundaries.

The FND-04 security-source `<=5s` freshness rule is not an entitlement lease and must not be copied into Profile-B authority duration.

## 8. Consumer authority classifications and precedence

The consumer exposes one typed/equivalent enforcement classification. Names may change in implementation, semantics may not.

### 8.1 Restrictive precedence

For already authenticated, schema-valid and provenance-compatible evidence:

```text
1. REVOKED
2. EXPIRED
3. NOT_YET_EFFECTIVE
4. INVALID_OR_CONFLICTING
5. CURRENT_AUTHORITY
6. STALE_WITHIN_BOUND
7. AUTHORITY_UNAVAILABLE
```

The ordering above is a decision procedure, not a claim that commercial states form a numeric enum.

Additional rules:

- malformed or unauthenticated input is rejected before it can alter an accepted state; it is an input-security failure, not Platform commercial truth;
- authenticated equal-revision contradiction or an unsupported/downgraded required semantic revision enters `INVALID_OR_CONFLICTING` and denies benefit;
- a previously known `REVOKED` or `EXPIRED` fact is not erased by later malformed, unavailable or older evidence;
- transport failure alone never manufactures `ACTIVE`, `REVOKED`, `EXPIRED` or `NOT_YET_EFFECTIVE` commercial truth.

### 8.2 `REVOKED`

A newer accepted Platform lifecycle decision explicitly revokes the entitlement. Deny new and continued Profile-B benefit. Older active evidence is rejected by the high-water fence.

### 8.3 `EXPIRED`

Use when either:

- known commercial `effective_until` has conservatively ended;
- producer lifecycle state is expired; or
- the finite `authority_valid_until` cutoff has conservatively ended.

An elapsed authority cutoff is `EXPIRED`, not `AUTHORITY_UNAVAILABLE`.

### 8.4 `NOT_YET_EFFECTIVE`

Accepted ordered active evidence exists, but conservative trusted time cannot yet prove that `effective_from` has been reached. Deny new and continued Profile-B benefit; outage or reconnect cannot start it early.

### 8.5 `INVALID_OR_CONFLICTING`

Use for an authenticated semantic conflict, unsupported required producer/consumer semantic revision, contradictory same-revision authority or loss of a required monotonic fence that prevents safe interpretation. Deny benefit and surface operator-visible reconciliation evidence.

This classification does not convert untrusted garbage into commercial state; unauthenticated/malformed input is simply rejected.

### 8.6 `CURRENT_AUTHORITY`

Latest accepted active evidence is usable, conservative start is reached, no conservative end/cutoff is reached, trusted time is safe and refresh is not known to be overdue.

### 8.7 `STALE_WITHIN_BOUND`

Refresh is due/failed or Platform is temporarily unreachable, but all of the following remain true:

- latest accepted evidence is active and ordered;
- conservative start has been reached;
- commercial end/cutoff has not been reached;
- trusted time remains safe;
- exact producer product/version policy permits bounded stale use;
- exact Oteryn-v2 consumer surface policy permits stale use for the requested surface.

The cutoff is unchanged. No reconnect/restart/cache event restarts it.

### 8.8 `AUTHORITY_UNAVAILABLE`

Current Platform authority cannot provide acceptable evidence and no already accepted fact produces a more specific `REVOKED`, `EXPIRED` or `NOT_YET_EFFECTIVE` result. This includes stale use that is not permitted for the requested consumer surface.

Deny entitlement-derived benefit without inventing a commercial revocation.

## 9. Product/version consumer enforcement policy

Before any Profile-B product/version may activate, Oteryn-v2 must have an explicitly versioned consumer policy bound to the exact Platform product/version and supported producer semantic revision.

The policy must state, at minimum:

```text
producer product/version authority policy binding
consumer policy revision
fresh-admission surface: REQUIRE_CURRENT | ALLOW_PRODUCER_STALE
reconnect/recovery surface: REQUIRE_CURRENT | ALLOW_PRODUCER_STALE
running-session surface: REQUIRE_CURRENT | ALLOW_PRODUCER_STALE
product-specific entitlement-loss/degraded behavior owner
```

Rules:

- all fields are explicit before activation; missing configuration is not an implicit allow;
- `ALLOW_PRODUCER_STALE` is valid only when the producer product/version policy itself permits `STALE_WITHIN_BOUND`;
- Oteryn-v2 may be stricter than Platform authority availability policy, never more permissive;
- local policy cannot extend `effective_from`, `effective_until` or `authority_valid_until`;
- a future stricter local maximum interval may narrow producer authority only when anchored to producer-issued absolute time/revision semantics; cache receipt time cannot create that interval;
- increasing stale/lease permissiveness is an authorization-policy change requiring explicit compatibility/rollout review, not a cache-tuning change.

This contract does not choose any actual Premium/VIP product or numeric value.

## 10. Fresh admission boundary

FND-04 fresh admission remains independently valid or invalid under its own authority rules. Profile-B state does not become part of `PreAdmissionGrant` by implication.

After authoritative game admission/session state exists, an entitlement-derived capability may be enabled only if its consumer surface policy and current classification authorize it:

```text
CURRENT_AUTHORITY
or
STALE_WITHIN_BOUND when explicitly allowed for fresh-admission surface
```

`NOT_YET_EFFECTIVE`, `EXPIRED`, `REVOKED`, `AUTHORITY_UNAVAILABLE` and `INVALID_OR_CONFLICTING` deny the entitlement-derived capability.

Generic `PROD-ENTITLEMENTS-01` does **not** reject the player's base game login merely because a Premium/VIP benefit is unavailable. A future product that makes an entitlement a prerequisite for admission requires a separate explicit product/gameplay decision; it may not be inferred here.

## 11. Reconnect and recovery boundary

FND-04 reconnect/recovery may restore authoritative control independently of entitlement availability.

A reconnect credential, recovery JWT, `GameSessionId`, `connection_generation` or same-session continuity state does not carry forward commercial authority.

At/after the authoritative reconnect/recovery boundary, entitlement-derived capabilities are evaluated against the same current consumer fence, trusted-time state and product/version policy as any other gameplay use.

Consequences:

- reconnect cannot resurrect a benefit revoked/expired while the client was absent;
- `STALE_WITHIN_BOUND` may be used only if both producer and consumer reconnect policies permit it and the finite cutoff remains safe;
- after cutoff, reconnect succeeds or fails according to FND-04, but Profile-B benefit remains denied until acceptable fresh authority with safe time evaluation exists;
- same-session recovery never resets a lease, start time or revision high water.

## 12. Already-running sessions

Commercial authority loss and GameSession termination are separate decisions.

Baseline behavior:

- expiry/revocation/unsafe authority evaluation does not automatically force logout;
- entitlement-derived authorization is checked at authoritative gameplay decision boundaries, not trusted from a session-start snapshot;
- no authoritative action after the conservative cutoff may rely on the expired Profile-B capability;
- newer accepted revocation stops future entitlement-derived authorization as soon as the restrictive evidence is authoritative to Oteryn-v2;
- UI/projection cleanup may lag only as presentation; it may not grant authoritative gameplay rights;
- session continuity is never a hidden grace period.

If a product's benefit cannot be removed/degraded safely through ordinary authoritative capability checks, that product must provide a separately accepted gameplay transition contract before activation. The generic entitlement gate does not invent inventory relocation, stat recalculation, teleport, combat, logout, housing or other product-specific mechanics.

Durable value already correctly granted under Profile C/D is not retroactively treated as a Profile-B session capability.

## 13. Game-affecting delivery identity and reconciliation

For Profile C/D or another separately accepted gameplay-mutating delivery, Oteryn-v2 consumes one stable semantic delivery operation identity from the Platform orchestration contract.

Minimum game-side invariants:

1. exact retries of one semantic fulfilment reuse the same `delivery_operation_id`;
2. the operation is bound to exact entitlement, product/version, target and intent/revision;
3. conflicting reuse of the same operation identity fails closed;
4. duplicate/replayed delivery cannot duplicate authoritative gameplay value;
5. timeout/lost response after possible commit is reconciled under the same operation identity; it never creates a blind replacement operation solely because the response was lost;
6. Oteryn-v2 emits/retains a durable game result/receipt sufficient for Platform reconciliation without asserting payment truth;
7. character-service mutation routes through accepted Character Authority semantics;
8. item/currency/value conservation remains owned by DUR-03 and is not weakened by entitlement orchestration.

This candidate does not approve any concrete Profile-C/D product, value grant, reversal or compensation policy.

## 14. Rollout and rollback contract

### 14.1 First activation against the current producer

The current producer prerequisite is already merged, so the first safe sequence is:

```text
1. producer semantic baseline remains pinned at afaa6d1...
2. accepted Oteryn-v2 consumer contract exists
3. consumer implementation deploys dark/disabled
4. durable fence + time + negative-path + mixed-version evidence passes
5. exact product/version producer policy and Oteryn consumer surface policy are registered
6. product/version activation is explicitly authorized
```

Architecture acceptance alone does not perform steps 3-6.

### 14.2 Future producer/consumer evolution

A new producer semantic revision may be deployed before it becomes required, provided old compatible semantics remain emitted to consumers that need them. A new required semantic cannot be emitted to an Oteryn-v2 consumer that has not explicitly declared compatibility.

A new consumer may accept an older supported producer baseline only when its compatibility record says so; absence of a compatibility record is fail closed.

### 14.3 Rollback invariants

Rollback must never:

- lower the entitlement lifecycle/authority high-water fence;
- restore an older `active` snapshot over a newer restrictive decision;
- move `effective_from` earlier;
- move a cutoff later or restart an expired lease;
- switch to a consumer/producer revision that cannot understand the current fence/validity semantics while Profile-B activation remains enabled;
- blindly replay an ambiguous gameplay mutation through a fallback path.

If the rollback target cannot safely consume current authority/fence semantics, affected game-consumed entitlement activation is disabled/fail closed until a compatible state is re-established. Existing base game sessions need not be terminated merely to enforce that disablement.

## 15. Observability and audit

Implementation must provide bounded operator evidence for security and reconciliation while preserving privacy.

At minimum expose typed metrics/events for:

- accepted/rejected producer semantic revisions;
- lifecycle/authority high-water advancement and rejected rollback/out-of-order evidence;
- current classifications and transitions, including `NOT_YET_EFFECTIVE`, `STALE_WITHIN_BOUND`, `AUTHORITY_UNAVAILABLE`, `EXPIRED`, `REVOKED` and conflicts;
- conservative remaining authority interval without redefining it;
- refresh due/failure age;
- trusted-time/skew fail-closed and recovery events;
- durable fence persistence/recovery failures;
- duplicate/conflicting delivery operations and reconciliation age;
- entitlement capability enable/disable reason at game authority boundaries;
- producer/consumer/product-policy revision mismatch.

Logs/traces/audit must not contain bearer credentials, payment/provider secrets, voucher plaintext, complete provider payloads or unnecessary private game/account state. Credential-free correlation and canonical IDs may be retained only under the applicable access/privacy/audit policy.

Analytics remains observational. ANL-01/ANL-02/ANL-03 evidence never becomes commercial or gameplay authorization merely because it observed an event.

## 16. Mandatory negative-path/security acceptance matrix

These are **future implementation requirements**, not runtime PASS claims for this architecture-only PR.

| Scenario | Required consumer result | Required security property |
| --- | --- | --- |
| Valid active authority; conservative start reached; refresh current | `CURRENT_AUTHORITY` | Benefit only inside producer interval |
| Pre-issued active; `now_lower < effective_from` | `NOT_YET_EFFECTIVE` | No early paid benefit |
| Clock uncertainty straddles commercial start | `NOT_YET_EFFECTIVE` | Uncertainty delays, never advances start |
| Platform outage after start but before cutoff; producer + consumer allow stale | `STALE_WITHIN_BOUND` | Bounded benefit; unchanged cutoff |
| Same outage but consumer surface requires current | `AUTHORITY_UNAVAILABLE` | Stricter local deny; no invented revoke |
| Platform outage at/after authority cutoff | `EXPIRED` | Continued and new benefit denied |
| Commercial `effective_until` occurs before lease cutoff | `EXPIRED` | Commercial end wins |
| Newer revoke arrives during partition/recovery | `REVOKED` | Immediate restrictive fence; no resurrection |
| Delayed older active after newer revoke | retain newer restrictive state; reject old evidence | High-water anti-rollback |
| Higher authority refresh then older refresh replay | retain higher authority revision | No cutoff rollback/reset |
| Same ordered revision with contradictory authenticated payload | `INVALID_OR_CONFLICTING` | Fail closed; operator reconciliation |
| Unauthenticated/malformed authority input | reject input; do not mutate accepted authority | Input cannot manufacture commercial state |
| Process restart with cached active evidence | reconstruct original absolute times + high water before benefit | Restart does not reset lease |
| Projection/snapshot rollback to older active state | fail closed until monotonic fence is proven | Restore cannot resurrect authority |
| Durable fence store unavailable while newer permissive authority arrives | do not authorize new permissive state | Fence-before-authorize |
| Durable fence store unavailable while newer revoke arrives | deny immediately; block later resumption until fence reconciles | Restrictive evidence may only narrow |
| Trusted time uncertainty exceeds product bound | deny entitlement benefit | Unsafe clock cannot extend/start authority |
| Fresh authority arrives while time remains unsafe | continue fail closed unless exchange establishes accepted bounded time anchor | Fresh state alone is insufficient |
| Fresh admission with stale evidence | only per explicit fresh-admission consumer surface policy | No implicit stale admission benefit |
| Reconnect inside stale bound | only per explicit reconnect consumer surface policy | Reconnect does not reset lease |
| Reconnect after cutoff | FND-04 may restore session; entitlement state `EXPIRED` | Session continuity is not grace |
| Running session crosses cutoff | session may remain; future entitlement-derived authorization denied | No benefit beyond cutoff |
| Duplicate/replayed gameplay grant | same operation result, at most one effect | No double grant |
| Lost response after possible gameplay commit | reconcile same operation identity | No blind replacement mutation |
| Producer semantic revision unknown to consumer | `INVALID_OR_CONFLICTING` / fail closed for benefit | No compatibility guessing |
| Consumer rollback target lacks current fence semantics | keep product disabled/fail closed | Rollback cannot weaken authority |
| Mixed compatible producer/consumer revisions | behavior follows explicit compatibility record | No hidden downgrade |

Future implementation evidence must name exact producer, consumer, product-policy, runtime, persistence and test revisions plus fault/clock mode. A synthetic unit test alone does not prove cross-repository activation safety.

## 17. Security invariants

1. **Finite authority:** no Profile-B game benefit survives beyond the conservatively evaluated producer-grounded authority/commercial interval.
2. **No early authority:** uncertainty and pre-issued evidence cannot begin benefit before conservatively proven `effective_from`.
3. **Monotonic restriction:** newer producer authority fences older evidence across delivery, restart, reconnect and restore.
4. **Fence before permissive use:** crash recovery cannot regress a newly accepted permissive revision.
5. **Transport is not truth:** availability/failure affects evidence availability, not commercial lifecycle facts.
6. **Session is not truth:** login/reconnect continuity cannot mint or extend commercial authority.
7. **No local commercial authority:** game policy may narrow, never extend, producer rights.
8. **Idempotent mutation:** ambiguous or duplicate delivery cannot double-apply gameplay value.
9. **Explicit compatibility:** producer/consumer semantic mismatch never silently downgrades security.
10. **Secret-safe evidence:** observability supports diagnosis without leaking bearer/payment/private payloads.

## 18. Cross-domain findings — `REPORT_ONLY`

```yaml
cross_domain_finding:
  id: ENT-CDF-01
  observed_in_domain: PROD-ENTITLEMENTS-01
  target_owner: DUR-02 / future entitlement persistence implementation
  severity: P1
  evidence: Platform producer contract requires durable lifecycle/authority high-water fencing across restart/rollback
  conflict_or_gap: Exact physical storage, transaction and restore mechanism for the entitlement high-water fence is intentionally not selected by this paper contract.
  required_before: Profile-B consumer implementation can claim crash/restart/rollback proof
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: ENT-CDF-02
  observed_in_domain: PROD-ENTITLEMENTS-01
  target_owner: product-specific gameplay capability/effect owner
  severity: P1
  evidence: Running sessions may remain connected but no Profile-B benefit may remain authoritative after the conservative cutoff.
  conflict_or_gap: Any entitlement benefit that cannot be removed through ordinary capability checks needs an explicit safe gameplay transition/degraded-state contract.
  required_before: activation of that product/version
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: ENT-CDF-03
  observed_in_domain: PROD-ENTITLEMENTS-01
  target_owner: future Platform↔Oteryn entitlement integration/transport owner
  severity: P1
  evidence: Consumer acceptance requires authenticated producer identity/purpose, bounded schema/versioning, replay protection and exact producer semantic revision binding.
  conflict_or_gap: Exact API/event/query/command transport, serialization and cryptographic service-authentication mechanism remain unselected.
  required_before: executable cross-repository entitlement integration
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: ENT-CDF-04
  observed_in_domain: PROD-ENTITLEMENTS-01
  target_owner: runtime/security trusted-time implementation owner
  severity: P1
  evidence: Producer contract requires conservative not-before/not-after evaluation and fail-closed behavior when clock uncertainty exceeds the product bound.
  conflict_or_gap: Exact trusted-time source and bounded authority-time/monotonic recovery mechanism are intentionally deferred.
  required_before: Profile-B implementation and clock fault evidence
  worker_action: REPORT_ONLY
```

No foreign-owner file is modified by these findings.

## 19. Risks and trade-offs

### Security risk — false allow after rollback

Mitigation: durable high-water fence, fence-before-authorize, fail closed when fence cannot be reconstructed.

### Player-experience risk — temporary paid benefit denial

Conservative clocks, Platform outage or persistence failure can produce false negatives. This is deliberate where authority cannot be proven. Product/version stale policy may preserve bounded availability, and base gameplay session continuity is not automatically terminated.

### Operational risk — high-cardinality authority state

Entitlement projections/fences can grow with issued entitlements. Physical retention, compaction and indexing are implementation decisions, but they may not discard monotonic evidence required to prevent resurrection.

### Rollout risk — producer/consumer drift

Mitigation: exact compatibility records, dark deployment, fail-closed unknown revisions and product/version activation only after mixed-version evidence.

### Product-design risk — non-revocable benefits mislabeled Profile B

Mitigation: Profile B is for continuously enforced account capability. Irreversible/durable value belongs in separately approved delivery profiles and DUR/gameplay contracts.

## 20. Decision timing

| Material decision | Must decide now? | Concrete work blocked | Harder later if wrong | Evidence that justifies supersession |
| --- | --- | --- | --- | --- |
| Platform commercial authority vs Oteryn gameplay authority | `YES` — already accepted producer baseline | any entitlement integration | split-brain commerce/game truth, unsafe direct mutation | explicit newer cross-repository ADR/contract accepted by both authorities |
| Durable lifecycle/authority high-water fence | `YES` | crash-safe consumer implementation, restart/rollback tests | migrating from permissive cache after activation can resurrect paid authority | equivalent formally reviewed monotonic mechanism with same restart/rollback guarantees |
| Conservative producer-issued time evaluation | `YES` | lease/expiry implementation and clock fault tests | local-clock grace can become compatibility behavior | stronger bounded authority-time mechanism proving no early/late authority |
| Typed state precedence | `YES` | deterministic enforcement/error/telemetry tests | inconsistent nodes could disagree about expiry/outage | new producer semantic model with explicit safe mapping and migration evidence |
| Explicit admission/reconnect/running-session stale policy | `YES` | FND-04 integration and product activation | implicit session grace becomes product behavior | product evidence supporting a safer stricter model or new accepted producer semantics |
| Stable game-delivery operation identity | `YES` | Profile C/D retry/reconciliation safety | duplicate value/mutation cleanup becomes expensive/unsafe | equivalent idempotency construction proving same ambiguous-commit guarantees |
| Exact storage schema/database layout | `NO` | nothing at architecture-candidate stage | premature schema freezes implementation | DUR-02 implementation spike, workload/concurrency/restore measurements |
| Exact entitlement API/event/query transport and serializer | `NO` | nothing until integration implementation is authorized | premature coupling to topology/library | bounded integration spike + security/resource/interoperability evidence |
| Exact Premium/VIP benefits and product catalogue | `NO` | generic consumer contract does not need them | freezes product design without player evidence | owner product decision/player evidence |
| Numeric lease/refresh/skew values | `NO` here, but mandatory per product before activation | only concrete product activation/tests | guessed values create bad availability/security trade-off | measured Platform/runtime latency, outage SLO, clock uncertainty and product risk evidence |
| Forced logout/UI transition on expiry | `NO` | authoritative entitlement denial is already defined | couples commerce loss to session UX | product/session UX evidence and explicit runtime policy |

## 21. Decisions deliberately not taken

This candidate does **not** select or claim:

- a payment provider, order system or Platform entitlement schema;
- exact game-side table/index/transaction/ORM design;
- REST/gRPC/event/broker/pull/push entitlement transport;
- protobuf/JSON/other wire encoding;
- mTLS/JWT/signature/KMS vendor or exact service-authentication primitive;
- real Premium/VIP existence, price, benefits or product catalogue;
- exact per-product `max_authority_lease`, `refresh_before`, `max_clock_skew` or stale policy values;
- exact trusted-time implementation or recovery anchor;
- forced disconnect policy, UI wording or client animation after authority loss;
- product-specific reversal/compensation mechanics;
- any durable item/currency/game-resource grant catalogue;
- implementation crate/service names, deployment topology, production capacity or rollout dates.

Each remains an extension point with a named future owner/evidence requirement rather than an accidental gap granting implementer discretion.

## 22. Future implementation evidence required before activation

Architecture acceptance, if later granted, is still insufficient for activation. A separately authorized implementation programme must prove on exact revisions:

- authenticated producer interoperability and semantic/profile version negotiation;
- crash-consistent durable high-water persistence and restore/rollback protection;
- trusted-time/skew faults, rollback and recovery;
- all matrix scenarios above, including outage/revoke/reconnect/running-session cases;
- product/version policy completeness and fail-closed missing/invalid config;
- delivery idempotency/reconciliation for any gameplay-mutating profile;
- resource limits, abuse/rate protection and bounded observability;
- cross-repository rollout and rollback with mixed supported/unsupported revisions;
- user-visible truthful entitlement/degraded presentation for any activated product;
- required Tier 1/2/3 evidence only where the actual implemented journey and client behavior make those tiers applicable.

No scenario is `PASS` in this candidate because no executable entitlement consumer exists yet.

## 23. Candidate disposition

```yaml
gate: PROD-ENTITLEMENTS-01
candidate_scope: Oteryn-v2 game consumer/enforcement contract
producer_prerequisite: SATISFIED
producer_revision_consumed: afaa6d1d8340e44b1152b62d6d27e5fd1649804a
consumer_decision_status: CANDIDATE
consumer_delivery_status: OPEN
consumer_implementation_status: NOT_STARTED
runtime_implementation: NOT_AUTHORIZED
premium_vip_activation: NOT_AUTHORIZED
issue_115_close_authority: NONE_FOR_WORKER
merge_authority: ARCHITECTURE_COORDINATOR_ONLY
independent_review_before_acceptance: REQUIRED
```

The candidate is acceptable for coordinator review only if the final PR proves exact scope, producer fidelity, security/failure-path self-review, ordinary exact-head repository CI and no unresolved material findings. Canonical acceptance and lifecycle reconciliation are separate coordinator/owner actions.
