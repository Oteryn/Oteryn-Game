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

The producer side is already repaired and immutable at the pinned Platform merge. The remaining Oteryn-v2 problem is to define one executable game-consumer boundary that answers deterministically:

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

Rejected because it cannot preserve a monotonic high-water fence across restart/rollback, makes gameplay synchronously dependent on Platform and still requires replay/clock handling.

### Option B — Local cache with receipt-time TTL

Rejected because receipt/restart time can accidentally become a new lease origin, and cache rollback can resurrect older `active` authority.

### Option C — Durable monotonic consumer fence + producer-issued finite authority interval

Oteryn-v2 stores enough crash-consistent high-water evidence to reject old revisions, evaluates only producer-issued absolute authority bounds using conservative trusted time, and separates entitlement classification from transport health.

**RECOMMENDATION: Option C.** It is the only option consistent with the accepted producer contract and Oteryn-v2 security dependency. The cost is stricter durability/rollout work and deliberate fail-closed loss of benefit when authority cannot be proven.

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

A later Git commit is not automatically a compatible producer semantic revision. Before Oteryn-v2 accepts different producer semantics, an explicit compatibility record must bind:

```text
producer semantic/profile revision
+ exact immutable producer repository revision
+ supported Oteryn-v2 consumer semantic/profile revision
+ affected product/version set
+ compatibility classification
+ rollout order
+ rollback order
```

Unknown required semantics fail closed for entitlement-derived benefit. A Git SHA is immutable provenance, not a numeric semantic ordering field.

## 5. Producer authority representation consumed by Oteryn-v2

For Profile B, Oteryn-v2 requires producer-defined semantic fields equivalent to:

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

Before evidence can affect gameplay it must also be bound, directly or through authenticated integration context, to:

- authenticated Platform producer/source identity and authorized purpose/scope;
- one supported producer semantic/profile revision and exact compatible producer revision;
- the exact product/version authority policy;
- canonical target/account scope;
- bounded schema/size/version validation;
- replay-resistant transport/security context when a transport exists.

Exact wire encoding, transport, storage schema and cryptographic primitive remain deliberately deferred.

## 6. Game-side consumer fence

### 6.1 Fence identity and immutable provenance

The durable logical fence is keyed at minimum by:

```text
AccountId + EntitlementId
```

The first accepted authority binds the expected product/product-version/target provenance. A contradictory reuse fails closed **unless** a strictly newer accepted Platform lifecycle decision is explicitly governed by a producer migration/replacement compatibility rule that authorizes that exact provenance transition. Product/version changes are never inferred from local game state or silently reinterpreted.

### 6.2 Ordered high water

Within one entitlement, accepted producer evidence is ordered by:

```text
lifecycle_revision
then authority_revision within that lifecycle revision
```

Rules:

1. a higher valid `lifecycle_revision` supersedes every lower lifecycle revision;
2. within one lifecycle revision, a higher `authority_revision` supersedes every lower authority revision;
3. exact semantic replay of the same ordered authority is idempotent;
4. the same ordered revision with contradictory authenticated state, target, product/version, interval or cutoff is a current-authority conflict and fails closed;
5. evidence below the accepted high water is stale and rejected without replacing current classification;
6. a strictly newer authenticated Platform lifecycle decision may supersede an older restrictive decision only because Platform issued newer commercial truth; Oteryn-v2 never synthesizes reactivation or revision advancement.

### 6.3 Crash-consistent consume-and-fence invariant

A newer producer authority representation is not **successfully consumed** until Oteryn-v2 has durably advanced the entitlement high-water fence together with enough producer-consumption/reconciliation progress to prove after restart that no acknowledged newer authority can be replaced by an older cached state.

Required semantics, independent of eventual transport/storage choice:

- a new permissive state cannot authorize gameplay until the durable high-water advancement is committed;
- a new restrictive state may deny gameplay immediately in memory, but the affected entitlement remains denied/quarantined until the restrictive high water is durably committed or the current producer authority is re-proven;
- producer event offset/cursor/receipt/acknowledgement, when such a mechanism exists, must not advance past authority evidence whose corresponding durable fence is not crash-consistently committed;
- if transport is query/snapshot based rather than event/ack based, restart must re-establish current source authority or an equivalently strong continuity proof before an older cached `active` representation may authorize benefit;
- a crash between observing a newer revoke/expiry and durably fencing it must therefore lead to replay/refetch/reconciliation or fail closed, never silent reauthorization from the old cache;
- no cache read, reconnect, process start or retry resets `effective_from`, `authority_valid_until`, lifecycle revision or authority revision.

This is not a requirement for one specific distributed transaction primitive. It is a required externally observable crash-consistency property.

### 6.4 Persistence or continuity failure

If the durable fence owner or producer-continuity proof is unavailable:

- do not authorize a new permissive state;
- if newer restrictive evidence was observed, deny/quarantine immediately and do not acknowledge source progress past the unfenced evidence;
- after restart, an older cached `active` state is insufficient by itself when current producer-continuity/high-water safety cannot be proven;
- classify the entitlement `INVALID_OR_CONFLICTING` for benefit purposes when the monotonic fence/continuity itself is unsafe, or `AUTHORITY_UNAVAILABLE` when the fence is sound but fresh producer authority is unavailable and no more specific state applies;
- never mint a local replacement revision or lease.

Exact PostgreSQL/table/transaction/inbox/outbox/cursor design is a DUR-02/integration implementation concern, not frozen here.

## 7. Trusted time and conservative interval evaluation

Producer timestamps are Platform-authority-issued facts. Browser/client/device clocks have no authority.

Let trusted current server time be known only within:

```text
[now_lower, now_upper]
```

with finite uncertainty acceptable under the exact product/version `max_clock_skew` or an equivalently strong accepted authority-time mechanism.

Evaluation is asymmetric and conservative:

- **not-before:** `now_lower >= effective_from` is required; otherwise `NOT_YET_EFFECTIVE`;
- **not-after:** if `now_upper` reaches/passes finite `effective_until` or `authority_valid_until`, state is `EXPIRED`;
- known uncertainty only shrinks the usable interval;
- if uncertainty exceeds the product bound and no accepted bounded authority-time/monotonic anchor can evaluate both boundaries, entitlement benefit fails closed;
- receipt time, process start, cache refresh and reconnect never become a replacement start or lease origin.

A future authority-time recovery mechanism may restore evaluability but may not rewrite Platform-issued absolute boundaries. The FND-04 security-source `<=5s` freshness rule is not an entitlement lease.

## 8. Consumer classifications and decision procedure

The consumer exposes typed/equivalent enforcement classifications. Names may change; semantics may not.

### 8.1 Input acceptance before state classification

1. Reject malformed/unauthenticated/wrong-source input without changing accepted commercial state.
2. Reject evidence below the durable high-water fence as stale/out-of-order.
3. If the **current high-water interpretation itself** is unsafe because of an authenticated equal-revision contradiction, unsupported/downgraded required semantic revision or unreconstructable monotonic fence/continuity, classify `INVALID_OR_CONFLICTING` and deny benefit.
4. Otherwise classify the latest accepted authority using producer-compatible restrictive precedence.

Thus an old hostile/replayed packet cannot turn a known revoke into `INVALID_OR_CONFLICTING`, while a genuine contradiction at the current authority revision cannot be masked as merely `NOT_YET_EFFECTIVE` or `CURRENT_AUTHORITY`.

### 8.2 Producer-compatible restrictive precedence

For a non-conflicting latest accepted authority:

```text
1. REVOKED
2. EXPIRED
3. NOT_YET_EFFECTIVE
4. CURRENT_AUTHORITY
5. STALE_WITHIN_BOUND
6. AUTHORITY_UNAVAILABLE
```

Transport failure alone never manufactures `ACTIVE`, `REVOKED`, `EXPIRED` or `NOT_YET_EFFECTIVE` commercial truth.

### 8.3 State semantics

- **`REVOKED`** — newer accepted Platform lifecycle decision explicitly revokes the entitlement. Deny new/continued Profile-B benefit. Older active evidence stays fenced.
- **`EXPIRED`** — producer expiry, finite `effective_until`, or finite `authority_valid_until` has conservatively ended. An elapsed cutoff is not `AUTHORITY_UNAVAILABLE`.
- **`NOT_YET_EFFECTIVE`** — accepted ordered active evidence exists but conservative trusted time cannot prove `effective_from` reached. Deny benefit; outage/reconnect cannot start it early.
- **`CURRENT_AUTHORITY`** — latest accepted active evidence is usable, conservative start reached, no end/cutoff reached, trusted time safe and refresh not known overdue.
- **`STALE_WITHIN_BOUND`** — refresh due/failed or producer temporarily unreachable, but accepted active evidence remains inside the conservative interval, trusted time is safe, producer policy permits bounded stale use and the exact Oteryn consumer surface policy also permits it.
- **`AUTHORITY_UNAVAILABLE`** — acceptable current producer evidence cannot be obtained and no accepted fact yields `REVOKED`, `EXPIRED` or `NOT_YET_EFFECTIVE`; includes a within-bound stale candidate when stale use is not permitted for the requested consumer surface. Deny benefit without inventing revocation.
- **`INVALID_OR_CONFLICTING`** — current authority cannot be safely interpreted because of authenticated same-revision conflict, unsupported/downgraded required semantics or unreconstructable monotonic fence/continuity. Deny benefit and require reconciliation.

A previously accepted restrictive fact is never erased by later malformed, unavailable or lower-revision evidence.

## 9. Product/version consumer enforcement policy

Before any Profile-B product/version activates, Oteryn-v2 must have an explicitly versioned consumer policy bound to the exact Platform product/version and supported producer semantics:

```text
producer product/version authority-policy binding
consumer policy revision
fresh-admission surface: REQUIRE_CURRENT | ALLOW_PRODUCER_STALE
reconnect/recovery surface: REQUIRE_CURRENT | ALLOW_PRODUCER_STALE
running-session surface: REQUIRE_CURRENT | ALLOW_PRODUCER_STALE
product-specific entitlement-loss/degraded behavior owner
```

Rules:

- every field is explicit before activation; missing configuration is not an allow;
- `ALLOW_PRODUCER_STALE` is legal only if producer product/version policy permits `STALE_WITHIN_BOUND`;
- Oteryn-v2 may be stricter than Platform, never more permissive;
- local policy cannot extend `effective_from`, `effective_until` or `authority_valid_until`;
- any stricter local interval must remain anchored to producer-issued time/revision semantics, never receipt time;
- increasing stale/lease permissiveness is an authorization-policy change requiring compatibility/rollout review.

No actual Premium/VIP product or numeric value is chosen here.

## 10. Fresh admission boundary

FND-04 fresh admission remains independently valid/invalid. Profile-B state does not become part of `PreAdmissionGrant` by implication.

After authoritative admission/session state exists, an entitlement-derived capability may be enabled only for:

```text
CURRENT_AUTHORITY
or
STALE_WITHIN_BOUND when explicitly allowed for the fresh-admission surface
```

All other classifications deny the capability.

Generic `PROD-ENTITLEMENTS-01` does **not** reject base game login merely because a Premium/VIP capability is unavailable. A product that makes entitlement a prerequisite for admission needs a separate explicit product/gameplay decision.

## 11. Reconnect and recovery boundary

FND-04 reconnect/recovery may restore authoritative control independently of entitlement availability. Reconnect proof, recovery JWT, `GameSessionId`, `connection_generation` and same-session continuity carry no commercial authority.

At/after authoritative control restoration, entitlement capability is re-evaluated against the current consumer fence, trusted time and surface policy.

Therefore:

- reconnect cannot resurrect a benefit revoked/expired while absent;
- stale evidence works only if producer + reconnect consumer policy allow it before cutoff;
- reconnect after cutoff may still restore the base session under FND-04, but entitlement benefit remains denied;
- same-session recovery never resets lease/start/revision high water.

## 12. Already-running sessions

Commercial authority loss and session termination are distinct:

- expiry/revocation/unsafe authority evaluation does not automatically force logout;
- entitlement authorization is checked at authoritative gameplay decision boundaries, not trusted forever from session start;
- no authoritative action after conservative cutoff may rely on expired Profile-B capability;
- newer accepted revocation stops future entitlement-derived authorization once that restrictive evidence becomes authoritative to Oteryn-v2;
- UI/projection cleanup may lag only as presentation; it cannot grant gameplay rights;
- session continuity is never hidden entitlement grace.

If a product benefit cannot safely degrade through ordinary capability checks, that product needs a separate gameplay transition contract before activation. This generic gate does not invent inventory relocation, stat recalculation, teleport, combat, logout, housing or other product-specific mechanics.

Durable value already authoritatively granted under Profile C/D is not retroactively treated as a Profile-B session capability.

## 13. Game-affecting delivery identity and reconciliation

For Profile C/D or another separately accepted gameplay-mutating delivery, Oteryn-v2 consumes one stable semantic `delivery_operation_id`.

Minimum game-side invariants:

1. exact retries of one semantic fulfilment reuse the same identity;
2. identity binds exact entitlement, product/version, target and intent/revision;
3. conflicting identity reuse fails closed;
4. duplicate/replayed delivery cannot duplicate authoritative gameplay value;
5. timeout/lost response after possible commit reconciles the same identity and never mints a blind replacement solely because response was lost;
6. Oteryn-v2 retains a durable game result/receipt sufficient for Platform reconciliation without asserting payment truth;
7. character-service mutation uses accepted Character Authority semantics;
8. item/currency/value conservation remains owned by DUR-03.

No Profile-C/D product, reversal or compensation policy is approved by this candidate.

## 14. Rollout and rollback

### 14.1 First activation against current producer

```text
1. producer semantic baseline stays pinned at afaa6d1...
2. Oteryn-v2 consumer contract is canonically accepted
3. consumer implementation deploys dark/disabled
4. durable consume-and-fence + trusted-time + negative-path + mixed-version evidence passes
5. exact producer product policy + Oteryn consumer surface policy are registered
6. product/version activation receives separate explicit authorization
```

This architecture PR performs none of steps 2-6.

### 14.2 Future evolution

A new producer semantic revision may be deployed before it is required only if compatible old semantics remain available to older supported consumers. A required new semantic cannot be emitted to an incompatible consumer. A new consumer may accept an older producer only when an explicit compatibility record allows it.

### 14.3 Rollback invariants

Rollback must never:

- lower entitlement lifecycle/authority high water;
- restore an older `active` snapshot over newer restrictive evidence;
- move `effective_from` earlier;
- move cutoff later or restart an expired lease;
- roll producer-consumption progress ahead of the durable high water it represents;
- activate a producer/consumer revision unable to understand current fence/validity semantics;
- blindly replay ambiguous gameplay mutation through a fallback path.

If the rollback target cannot safely consume current authority/fence semantics, affected game-consumed entitlement activation remains disabled/fail closed until compatibility is re-established. Base sessions need not be terminated merely to enforce that disablement.

## 15. Observability and audit

Implementation must expose bounded operator evidence for:

- accepted/rejected producer semantic revisions;
- high-water advancement and rejected rollback/out-of-order evidence;
- producer-consumption/ack continuity relative to durable high water;
- all consumer classifications and transitions;
- conservative remaining authority interval and refresh failure age;
- trusted-time/skew fail-closed/recovery;
- durable fence persistence/recovery failures;
- duplicate/conflicting delivery operations and reconciliation age;
- entitlement capability enable/disable reason at game authority boundaries;
- producer/consumer/product-policy mismatch.

Logs/traces/audit must not contain bearer credentials, payment/provider secrets, voucher plaintext, complete provider payloads or unnecessary private game/account state. Credential-free correlation/canonical IDs require applicable access/privacy policy.

Analytics is observational only and never becomes commercial/gameplay authorization.

## 16. Mandatory negative-path/security acceptance matrix

These are future implementation requirements, **not runtime PASS claims** for this paper-only candidate.

| Scenario | Required result | Security property |
| --- | --- | --- |
| Valid active; conservative start reached; refresh current | `CURRENT_AUTHORITY` | benefit only inside producer interval |
| Pre-issued active; `now_lower < effective_from` | `NOT_YET_EFFECTIVE` | no early paid benefit |
| Clock uncertainty straddles start | `NOT_YET_EFFECTIVE` | uncertainty delays start |
| Platform unavailable after start/before cutoff; producer + surface allow stale | `STALE_WITHIN_BOUND` | bounded benefit; unchanged cutoff |
| Same outage; surface requires current | `AUTHORITY_UNAVAILABLE` | stricter local deny; no invented revoke |
| **Platform revokes during an unobservable partition** | locally remain only in previously provable `STALE_WITHIN_BOUND` when allowed, never past existing cutoff; then `EXPIRED` if no newer evidence | no false claim of instantaneous unseen revocation; finite maximum stale-allow window |
| Partition heals and newer revoke arrives | `REVOKED` | restrictive revision fences old active |
| Crash after observing revoke but before fence commit | entitlement remains denied after restart until revoke is replayed/refetched or current authority/high-water continuity is re-proven; old cache alone cannot authorize | no observed-revoke resurrection through crash window |
| Source ack/cursor tries to advance before fence commit | reject/defer ack/progress | source-consumption progress cannot outrun durable authority fence |
| Platform outage at/after cutoff | `EXPIRED` | new/continued benefit denied |
| Commercial end occurs before lease cutoff | `EXPIRED` | commercial end wins |
| Delayed older active after newer revoke | retain newer restrictive state; reject old evidence | anti-rollback |
| Higher authority refresh then older refresh replay | retain higher authority revision | no cutoff reset |
| Same current revision with contradictory authenticated payload | `INVALID_OR_CONFLICTING` | fail closed + reconciliation |
| Lower stale revision has contradictory payload | reject as stale; retain current classification | old input cannot poison newer truth |
| Unauthenticated/malformed input | reject; no accepted-state mutation | input cannot manufacture commercial state |
| Restart with cached active | reconstruct original times + high water + source continuity before benefit | no lease/fence reset |
| Projection/snapshot rollback | fail closed until monotonic fence/continuity proven | no authority resurrection |
| Fence store unavailable; newer permissive evidence arrives | do not authorize new permissive state | fence-before-authorize |
| Fence store unavailable; newer restrictive evidence arrives | deny/quarantine; do not ack source progress; reconcile before future benefit | restrictive crash window cannot resurrect old allow |
| Trusted-time uncertainty exceeds product bound | deny benefit | unsafe clock cannot extend/start authority |
| Fresh authority while time remains unsafe | remain fail closed unless exchange establishes accepted bounded time anchor | fresh state alone insufficient |
| Wall-clock rollback / VM snapshot restore | preserve original boundaries/fence or fail closed | time rollback never widens interval |
| Fresh admission with stale evidence | only per explicit fresh-admission surface policy | no implicit stale admission |
| Reconnect inside stale bound | only per explicit reconnect surface policy | reconnect does not reset lease |
| Reconnect after cutoff | FND-04 may restore session; entitlement `EXPIRED` | session continuity is not grace |
| Running session crosses cutoff | session may remain; future entitlement authorization denied | no benefit beyond cutoff |
| Duplicate/replayed gameplay grant | stable same-operation result, at most one effect | no double grant |
| Lost response after possible gameplay commit | reconcile same operation identity | no blind replacement mutation |
| Producer semantic revision unknown | `INVALID_OR_CONFLICTING` | no compatibility guessing |
| Attempted downgrade to older producer semantics not explicitly compatible | fail closed / activation disabled | no security downgrade |
| Consumer rollback target lacks current fence semantics | product disabled/fail closed | rollback cannot weaken authority |
| Mixed compatible producer/consumer revisions | follow explicit compatibility record | no hidden downgrade |
| Inability to refresh despite transport being otherwise healthy | classify by accepted finite evidence/policy; never reset cutoff | refresh failure cannot mint authority |

Future implementation evidence must name exact producer, consumer, product-policy, runtime, persistence and test revisions plus fault/clock mode. Synthetic unit tests alone do not prove cross-repository activation safety.

## 17. Security invariants

1. **Finite authority:** no Profile-B benefit survives beyond the conservatively evaluated producer-grounded interval.
2. **No early authority:** uncertainty/pre-issued evidence cannot begin benefit before conservatively proven start.
3. **Monotonic restriction:** newer producer authority fences older evidence across delivery/restart/reconnect/restore.
4. **Crash-consistent consume-and-fence:** acknowledged/current producer progress can never outrun the durable entitlement high water needed to prevent resurrection.
5. **Transport is not truth:** availability affects evidence availability, not commercial lifecycle.
6. **Session is not truth:** login/reconnect continuity cannot mint/extend commercial authority.
7. **No local commercial authority:** game policy may narrow, never extend, producer rights.
8. **Idempotent mutation:** ambiguous/duplicate delivery cannot double-apply gameplay value.
9. **Explicit compatibility:** semantic mismatch never silently downgrades security.
10. **Secret-safe evidence:** diagnosis does not leak bearer/payment/private payloads.

## 18. Cross-domain findings — `REPORT_ONLY`

```yaml
cross_domain_finding:
  id: ENT-CDF-01
  observed_in_domain: PROD-ENTITLEMENTS-01
  target_owner: DUR-02 / future entitlement persistence implementation
  severity: P1
  evidence: Producer contract requires durable lifecycle/authority high water across restart/rollback; consumer security additionally requires source-consumption progress not to outrun that fence.
  conflict_or_gap: Exact physical storage, transaction and durable inbox/cursor/receipt coupling mechanism is intentionally not selected here.
  required_before: Profile-B implementation can claim crash/restart/rollback proof
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: ENT-CDF-02
  observed_in_domain: PROD-ENTITLEMENTS-01
  target_owner: product-specific gameplay capability/effect owner
  severity: P1
  evidence: Running sessions may remain connected but no Profile-B benefit may remain authoritative after conservative cutoff.
  conflict_or_gap: Any benefit that cannot be removed through ordinary capability checks needs an explicit safe gameplay transition/degraded-state contract.
  required_before: activation of that product/version
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: ENT-CDF-03
  observed_in_domain: PROD-ENTITLEMENTS-01
  target_owner: future Platform↔Oteryn entitlement integration/transport owner
  severity: P1
  evidence: Consumer acceptance requires authenticated source/purpose, bounded schema/version, replay protection, exact semantic revision binding and replay/refetch/reconciliation continuity after an unfenced crash window.
  conflict_or_gap: Exact API/event/query/command transport, serialization, service authentication and replay/query continuity mechanism remain unselected.
  required_before: executable cross-repository entitlement integration
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: ENT-CDF-04
  observed_in_domain: PROD-ENTITLEMENTS-01
  target_owner: runtime/security trusted-time implementation owner
  severity: P1
  evidence: Producer contract requires conservative not-before/not-after evaluation and fail-closed behavior when clock uncertainty exceeds product bound.
  conflict_or_gap: Exact trusted-time source and bounded authority-time/monotonic recovery mechanism are deferred.
  required_before: Profile-B implementation and clock-fault evidence
  worker_action: REPORT_ONLY
```

No foreign-owner file is modified by these findings.

## 19. Risks and trade-offs

- **False allow after rollback/crash:** mitigated by durable high water plus consume-and-fence continuity.
- **Temporary paid-benefit denial:** conservative time/outage/persistence/continuity failures may create false negatives; bounded stale policy may preserve availability while base session continuity remains independent.
- **High-cardinality authority state:** retention/compaction/indexing remain implementation decisions but cannot discard anti-resurrection evidence.
- **Producer/consumer drift:** mitigated by exact compatibility records, dark deployment and fail-closed unknown revisions.
- **Wrong delivery profile:** irreversible durable benefits must not be disguised as Profile B; use separately accepted Profile C/D and DUR/gameplay contracts.

## 20. Decision timing

| Material decision | Must decide now? | Concrete work blocked | Harder later if wrong | Supersession evidence |
| --- | --- | --- | --- | --- |
| Platform commercial vs Oteryn gameplay authority | `YES` — accepted producer baseline | any entitlement integration | split-brain truth/direct mutation | newer explicit cross-repository contract |
| Durable high water + consume-progress continuity | `YES` | crash-safe consumer + restart/rollback tests | permissive cache/ack behavior can resurrect authority | equivalent reviewed monotonic crash-consistency mechanism |
| Conservative producer-issued time | `YES` | expiry/clock tests | local grace becomes compatibility behavior | stronger bounded authority-time proof |
| Typed state/conflict precedence | `YES` | deterministic enforcement/telemetry | nodes may disagree on outage/expiry | newer producer semantic model + safe mapping |
| Explicit admission/reconnect/running stale policy | `YES` | FND-04 integration + product activation | implicit session grace becomes product behavior | safer accepted product/producer model |
| Stable delivery operation identity | `YES` | Profile C/D retry safety | duplicate value cleanup becomes unsafe | equivalent idempotency/reconciliation proof |
| Exact storage/inbox/cursor schema | `NO` | nothing now | premature physical coupling | DUR-02/integration workload and fault evidence |
| Exact API/event/query transport/serializer | `NO` | nothing until integration implementation | premature topology/library coupling | bounded integration/security spike |
| Exact Premium/VIP benefits/catalogue | `NO` | generic contract independent | freezes product design without evidence | owner/player product decision |
| Numeric lease/refresh/skew values | `NO` here; mandatory per product pre-activation | concrete activation/tests | guessed availability/security trade-off | measured latency/outage/clock/product-risk evidence |
| Forced logout/UI transition | `NO` | authoritative denial already defined | couples commerce to session UX | explicit product/session UX evidence |

## 21. Decisions deliberately not taken

Not selected here:

- payment provider/order implementation/Platform schema;
- game-side table/index/ORM/transaction/inbox/cursor syntax;
- REST/gRPC/event/broker/pull/push transport or wire encoding;
- mTLS/JWT/signature/KMS vendor or exact service-auth primitive;
- actual Premium/VIP existence, price, benefits or catalogue;
- numeric `max_authority_lease`, `refresh_before`, `max_clock_skew` or stale values;
- exact trusted-time implementation/recovery anchor;
- forced disconnect/UI wording/animation;
- product-specific reversal/compensation mechanics;
- durable item/currency grant catalogue;
- crate/service names, deployment topology, capacity or rollout dates.

These are explicit future extension points, not permission for an implementer to guess.

## 22. Future implementation evidence required before activation

A separately authorized implementation programme must prove on exact revisions:

- authenticated producer interoperability and semantic/profile compatibility;
- crash-consistent high-water persistence plus source-consumption/replay/refetch continuity;
- restore/rollback and crash-after-observe-before-fence faults;
- trusted-time/skew/clock-rollback/recovery faults;
- every matrix scenario above;
- product/version policy completeness and fail-closed invalid config;
- delivery idempotency/reconciliation for gameplay-mutating profiles;
- bounded resource/abuse/observability behavior;
- cross-repository mixed-version rollout/rollback;
- truthful user-visible entitlement/degraded presentation for activated products;
- applicable QA-E2E tiers for the actual implemented journey.

No executable scenario is `PASS` in this candidate because no entitlement consumer runtime exists.

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

The candidate is ready for coordinator review only after exact-scope/full-diff self-review, producer-fidelity/security/failure-path audit, ordinary exact-head repository CI and zero unresolved material findings. Canonical acceptance and lifecycle reconciliation remain separate coordinator/owner actions.
