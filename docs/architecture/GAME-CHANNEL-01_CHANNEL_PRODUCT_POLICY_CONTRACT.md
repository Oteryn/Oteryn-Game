# GAME-CHANNEL-01 — Channel Product Policy Contract

- Date: 2026-08-12
- Gate: `GAME-CHANNEL-01`
- Delivery task: `OTV2-20260812-game-channel-01-architecture`
- Delivery PR: #209
- Status on delivery branch: **CANDIDATE / NONBINDING**
- Canonical semantic effect: only after accepted delivery merge; programme `ACCEPTED / LIFECYCLE_CLOSED` promotion requires a separate lifecycle closeout
- ImplementationStatus: **NOT_STARTED**
- Runtime/client authority: **NONE**
- PostgreSQL DDL/migration authority: **NONE**
- Production authority: **NONE**
- Analysis source: `GAME-CHANNEL-01_CHANNEL_PRODUCT_POLICY_ANALYSIS.md`
- Historical lifecycle framework consumed: `GAME-CHANNEL-01_PREDECISION_CAPACITY_TRIGGERS_ADDENDUM.md` and `GAME-CHANNEL-01_PREDECISION_ANALYSIS.md` as repository evidence only; historical numeric thresholds remain nonbinding

## 1. Purpose

Freeze the minimum Oteryn product policy required before the accepted multichannel technical capability becomes player-visible.

GAME-CHANNEL owns:

- public Channel identity/presentation semantics;
- recommendation versus explicit target selection;
- bounded target-Channel queue meaning;
- party/friend co-location policy boundary;
- voluntary Channel switching and durable anti-hopping semantics;
- public Channel create/drain/drain-abort/retirement product predicates;
- Channel multiplicity versus durable source/reward eligibility policy;
- world-global event/reward scope requirements;
- cross-Channel PvP consequences and community safeguards;
- player-facing capacity/drain/recovery semantics;
- the boundary with FND, DUR, PERF, OPS and downstream gameplay domains.

It does **not** own GameNode/ChannelRuntime implementation, process/container orchestration, FND admission/session/lease mechanics, DUR value transaction mechanics, physical persistence, exact PvP/reward/economy formulas, Platform implementation or production rollout.

## 2. Authority chain

```text
World/Channel identity + multichannel foundation     -> FND-ID-01 + ADR-0001
ChannelRuntime/GameNode lifecycle                    -> ADR-0009 + FND-03
fresh admission / GameSession / CharacterLease       -> FND-04
minimum product/economy/party/PvP direction          -> GAME-VISION-01
Character semantics                                  -> GAME-CHAR-01
item legality                                        -> GAME-ITEM-01
item/currency/value conservation                     -> DUR-03
channel product/lifecycle policy                     -> GAME-CHANNEL-01
numeric capacity/service objectives                  -> PERF-01
activation algorithms/hysteresis/orchestration       -> OPS-CHANNEL-01
exact PvP/event/reward/economy/social business rules -> owning domain gates
```

No layer may redefine another owner's semantic authority for convenience.

## 3. Canonical model

```text
WorldId
= one persistent product/economy/community/ruleset boundary

ChannelRef = WorldId + ChannelId
= one persistent identity of a parallel public-world simulation

current Character Channel placement
= current GameSession/lease/runtime placement fact
!= Character identity
!= economy namespace
!= progression namespace
!= social namespace
```

Different profile/ruleset families remain different WorldId values. InstanceId is not ChannelId. House/instance transitions never silently change ChannelId.

Canonical identity is ChannelRef. Display labels/ordinals are presentation only, never queue/admission/switch/audit/reward identity. Same semantic Channel recovery preserves ChannelId; semantic retirement never permits ChannelId reuse.

## 4. Public directory, recommendation and explicit target

A bounded public directory/offer exposes only current product-eligible Channels and safe presentation/availability information. It does not expose hidden presence, NodeId, ownership generation or grant gameplay authority merely by listing a target.

Accepted entry modes:

- `RECOMMENDED` — control plane suggests one current eligible target;
- `EXPLICIT` — player selects another current eligible visible target.

Recommendation may consider capacity/health, previous-Channel affinity, party/friend hints subject to privacy, maintenance state and demand balancing. Exact scoring remains downstream. Recommendation never bypasses FND-04 final admission.

For explicit Channel C:

```text
current eligible offer for C
-> bounded authorization for C
-> final FND-04 validation for C
```

If C becomes capacity-limited, draining, recovering, unavailable, stale or incompatible:

- no admission to C;
- no silent retarget using the same authorization;
- fresh directory/offer/grant is required for another target;
- failed target creates no GameSession/lease/value authority.

## 5. Queue contract

A World may support an optional bounded target-Channel **pre-admission queue**.

```text
queue state
!= GameSession
!= CharacterLease
!= runtime ownership
!= durable value reservation
!= guaranteed admission
```

Rules:

- one queue target resolves exactly one ChannelRef;
- priority/order is control-plane authoritative, never client-declared;
- queue state is bounded, purpose-limited and expires/cancels explicitly;
- duplicate/cancel/expiry is idempotent and creates no gameplay mutation;
- final admission revalidates current target/security/revision/account/Character/lease facts;
- short-lived FND-04 admission material is issued/refreshed only when the queued attempt is ready, not held as a long-lived queue credential;
- queue reference is never gameplay bearer authority;
- exact storage/service/algorithm/timeouts/limits/priority remain downstream/cross-repository implementation work.

First generation does not queue/reserve a destination while the Character continues authoritative mutation in the source Channel. Any live-session destination reservation requires a separate lease/capacity/fairness/failure contract.

## 6. Party/friend co-location and privacy

- co-location is a privacy-bounded recommendation/target preference, not authority;
- every Character is independently admitted with independent GameSession/lease;
- no all-or-nothing party admission transaction;
- no PartyId owns/reserves a Channel by itself;
- no automatic teleport/migration when another party member changes Channel;
- target Full follows ordinary queue/alternate-choice policy;
- gameplay effects requiring co-location activate only when members actually share the authoritative simulation;
- exact Channel-placement visibility remains social/privacy-owned and hidden presence cannot be inferred through co-location endpoints.

Atomic group-capacity reservation is deferred.

## 7. Reconnect is not Channel switch

Same-Channel eligible reconnect/recovery:

- uses FND-04 continuity;
- may preserve GameSessionId when permitted;
- keeps the same ChannelId;
- is not a voluntary Channel switch.

A completed Channel switch uses ADR-0001 safe source exit/checkpoint, fresh destination authorization, fresh FND-04 admission and a fresh canonical GameSessionId. It is never an in-place transport rebind or teleport.

## 8. Hard switch locks

Voluntary switching fails closed while any accepted blocker applies, including:

- combat/protected PvP;
- direct trade;
- unresolved item/currency/value transaction;
- protected boss/raid/event participation where hopping changes eligibility;
- unsafe instance/house transition;
- pending Character checkpoint/handoff/authority transition;
- stale/unavailable session/lease/runtime evidence;
- destination incompatibility;
- destination capacity-limited/draining/recovering/unavailable state.

Client cannot override these locks.

## 9. Durable voluntary anti-hopping guard

GAME-CHANNEL requires durable game-domain world channel-policy state:

```text
scope: CharacterId + WorldId
semantic owner: GAME-CHANNEL / game-domain world channel-policy authority
lifetime: may outlive GameSession/connection/GameNode
interpretation: world_policy_revision
```

The guard is not automatically GAME-CHAR progression state merely because it is Character-scoped.

It retains enough semantic state to classify later admissions unambiguously, including `last_successful_channel_ref` or an equivalent durable prior-placement representation, plus current switch eligibility/cooldown state and policy context.

## 10. Switch classification and cooldown

- first admission with no prior successful Channel establishes baseline and is not a switch;
- fresh admission to the same prior Channel is not a switch;
- fresh admission to a different Channel is a switch even after the old GameSession ended;
- failed destination attempt leaves remembered Channel/guard unchanged;
- successful different-Channel admission updates remembered Channel and advances guard exactly once;
- same-Channel reconnect/recovery never becomes a switch.

The first voluntary anti-hopping mechanism is **time-based cooldown + hard locks**.

- exact duration is deliberately not guessed and requires product/playtest/economy/PvP evidence before activation;
- guard survives logout/relog/fresh GameSession/reconnect/GameNode restart;
- client wall time is not authoritative;
- incompatible policy changes require explicit version-aware migration/interpretation.

An implementation cannot claim voluntary switching conformance until the numeric duration and trusted durable time interpretation are accepted.

## 11. Destination admission + guard recovery invariant

This outcome is invalid:

```text
new destination GameSession/placement becomes playable
AND remembered Channel / ChannelSwitchGuard did not advance consistently
```

For a voluntary different-Channel admission, final destination admission must include switch classification/guard/prior-placement update in the **same authoritative acceptance boundary** as destination session/placement authority, or an equivalently proven recovery protocol satisfying:

1. no playable destination authority before durable switch outcome is determined;
2. final admission revalidates current world-scoped locks/guard rather than trusting stale source checks;
3. ambiguous outcome reconciles the same admission/switch attempt;
4. retry cannot create a second destination authority or skip guard;
5. current Channel, remembered successful Channel and guard cannot disagree silently;
6. crash after durable acceptance reconstructs one semantic outcome.

Physical transaction/session persistence mechanics remain FND/DUR implementation work.

No mandatory `ChannelSwitchId` is introduced; existing FND admission-attempt/session and ANL correlation/operation identities are reused unless later evidence proves a separate durable lifecycle identity is necessary.

## 12. Trusted non-voluntary exception

Maintenance/incident/admin-safe evacuation may later define a typed exception only when it is server/operator-authored, audited, unforgeable by client, reaches a safe authority/value boundary, preserves PvP/reward/value consequences, keeps remembered prior-placement semantics deterministic and still uses fresh admission when ChannelId changes.

There is no generic bypass boolean.

## 13. Runtime locality is not durable source multiplicity

Critical invariant:

```text
runtime object is Channel-local
!= durable reward/source is automatically repeatable per Channel
```

For every value-producing source/encounter family whose durable output can differ with active Channel count, compiled/validated authored policy must explicitly select a supported multiplicity class. **There is no runtime fallback class for a value-producing source.** Missing classification blocks multichannel activation of that source.

A reviewed content/profile package may define a versioned default for a bounded source category, but that default is explicit authored policy rather than runtime assumption.

## 14. Multiplicity classes

### `CHANNEL_LOCAL_REPEATABLE`

Independent source/simulation per Channel; output may repeat independently under unchanged in-Channel mechanics; aggregate World supply may scale with active Channels/player demand. No hidden automatic rate division by active Channel count.

### `CHANNEL_LOCAL_SHARED_ELIGIBILITY`

Simulation may exist in multiple Channels while durable eligibility is shared at an explicitly declared Character/Account/World/etc. scope. Channel change/new GameSession does not reset eligibility.

### `WORLD_SCOPED_UNIQUE`

One semantic World occurrence/eligibility exists regardless of Channel copies. Event/world-service owner controls execution/presentation placement.

### `EXPLICIT_EVENT_POLICY_REQUIRED`

No generic class is safe; high-impact boss/raid/event owner must declare exact simulation and eligibility semantics before activation.

## 15. Simulation scope and durable eligibility scope

Every reward-bearing event where Channels matter declares at least:

```text
simulation_scope
eligibility_scope
reset/repeat policy
stable reward/source occurrence semantics
```

ChannelId is not silently inserted into Character/Account/World reward reset/idempotency keys.

GAME-CHANNEL defines scope requirements. Event/reward domains define business eligibility. DUR-03 defines transaction/conservation/idempotency mechanics.

## 16. Dynamic lifecycle ownership split

GAME-CHANNEL freezes **qualitative product predicates** for creating/draining/removing public Channels.

PERF-01 supplies measured numeric capacity/headroom/service objectives. OPS-CHANNEL-01 supplies operational trend windows, hysteresis, timers, orchestration, placement and algorithms implementing these predicates.

This prevents OPS/PERF from accidentally becoming product/economy policy owners.

## 17. Lifecycle trigger vocabulary

The current candidate adopts the qualitative vocabulary recorded in the predecision capacity-trigger addendum, without adopting its historical numbers.

### `DEMAND_PRESSURE`

Eligible public-channel demand exceeds currently healthy/selectable capacity for a **sustained** evidence window under accepted service-objective/headroom policy.

`Sustained`, `service objective` and `required headroom` are semantic requirements; numeric evaluation belongs PERF/OPS.

### `RECOVERY_PRESSURE`

Current World public capacity is unavailable/insufficient because a Channel or hosting capacity has failed or cannot safely serve eligible demand.

Recovery pressure never permits affected actors to be silently moved to a different semantic Channel. If the failed Channel is recoverable, ADR-0009 same-ChannelId recovery remains authoritative.

### `LOW_LOAD_CONSOLIDATION_CANDIDATE`

A public Channel has sustained low utilization such that consolidation may be safe if retained eligible Channels can absorb expected demand with required headroom and no event/recovery/product policy requires that Channel.

### `CHANNEL_UNHEALTHY`

A Channel is not safely routable/authoritative under runtime health/fencing/recovery policy.

`CHANNEL_UNHEALTHY` maps primarily to Recovering/Unavailable/fencing and is not authority to move players silently into another Channel.

## 18. New semantic Channel creation predicate

Creating/activating a **new semantic public ChannelId** is product-legitimate only when all applicable conditions hold:

1. trigger is `DEMAND_PRESSURE`, or `RECOVERY_PRESSURE` requires additional public capacity beyond safe same-Channel recovery/current healthy capacity;
2. for demand creation, eligible demand is sustained and no current eligible healthy Channel can satisfy the accepted service objective while preserving required headroom;
3. for recovery-driven creation, a new semantic Channel serves fresh/general capacity need and never becomes silent continuation of actors bound to the failed ChannelId;
4. extra Channel multiplicity does not violate World/ruleset/content/event/reward source/eligibility policy;
5. safe infrastructure capacity exists under PERF/OPS authority;
6. the candidate reaches the accepted fully Ready/compatible runtime state before becoming `SELECTABLE` or receiving ordinary admission;
7. canonical ChannelId is issued by the accepted topology authority and is not derived from a display ordinal.

The following alone are **forbidden creation triggers**:

- one party/friend group wants a private copy;
- a player prefers an emptier Channel;
- rare spawn/loot/resource farming;
- PvP avoidance;
- manual operator preference without demand/recovery/product justification.

Such preferences may influence selection among already eligible Channels but cannot create new public simulation/economy capacity by themselves.

## 19. Recovery pressure versus same-Channel recovery

```text
replace hosting/runtime capacity for failed Channel A
-> preserve ChannelId A when semantic recovery is possible

create new semantic Channel B because the World needs additional general capacity
-> new ChannelId B
-> affected Channel A actors are not moved automatically
```

`RECOVERY_PRESSURE` can justify infrastructure replacement or additional capacity, but never supersedes ADR-0009 Channel identity/recovery semantics.

## 20. Drain eligibility predicate

A Channel may enter product `DRAINING` for low-load consolidation only when all applicable conditions hold:

1. it is a sustained `LOW_LOAD_CONSOLIDATION_CANDIDATE` under measured policy;
2. retained eligible Channels can absorb expected demand with required headroom/service objective;
3. no World/event/reward/recovery/reserved-capacity policy currently requires the candidate Channel;
4. new ordinary admissions and voluntary switches into the candidate stop before drain progresses;
5. incumbent actors can reach bounded safe exit/session boundaries without violating combat/session/item/value correctness;
6. drain does not create hidden reward/source reset behavior.

A merely `CHANNEL_UNHEALTHY` Channel follows recovery/fencing semantics; unhealthy is not synonymous with low-load consolidation drain.

## 21. Drain abort/hold predicate

Before terminal retirement/removal, drain must **abort or hold** if any material condition becomes true, including:

- `DEMAND_PRESSURE` reappears and retained capacity can no longer satisfy service objective/headroom safely;
- a retained eligible Channel becomes unhealthy/unavailable and reduces safe capacity;
- `RECOVERY_PRESSURE` requires the candidate capacity;
- World/event/reward/product policy requires the candidate Channel;
- continuing drain risks GameSession/CharacterLease, item/currency/value, encounter, checkpoint or recovery correctness.

If drain is reversed before terminal retirement:

- canonical ChannelId remains unchanged;
- runtime ownership/readiness must be revalidated before returning to `SELECTABLE`;
- stale directory/queue/admission evidence from the earlier lifecycle state is not revived blindly; fresh control-plane evidence is required.

## 22. Terminal removal/retirement predicate

A semantic Channel may be terminally removed/retired only after all applicable conditions hold:

1. no authoritative player GameSession/Character placement remains bound to it;
2. no active Instance/encounter/event obligation depends on its runtime scope;
3. no unresolved item/value transaction, handoff, checkpoint or recovery obligation depends on it;
4. required durable checkpoint/audit/evidence finalization is complete under owning contracts;
5. old runtime owner/ownership generation is fenced and cannot regain authority;
6. directory/control plane has removed/invalidated stale routing, queue and admission references;
7. policy confirms this is **semantic retirement**, not a temporary inactive/stopped state intended for later same-ChannelId reactivation.

After semantic retirement, ChannelId is never reused.

Temporary stop/inactivity is not retirement; later safe reactivation of the same semantic Channel preserves ChannelId.

## 23. Numeric lifecycle policy remains PERF/OPS-owned

GAME-CHANNEL deliberately does not freeze:

- demand window length;
- utilization percentages;
- target queue latency;
- headroom percentages;
- players/Channel thresholds;
- min/max public Channels;
- drain grace time;
- scaling cooldown/hysteresis timing;
- GameNode placement capacity.

PERF-01 measures supported capacity/service objectives. OPS-CHANNEL-01 implements bounded trend windows, hysteresis and orchestration satisfying the predicates above.

## 24. Fresh/recovered Channel abuse prevention

- stop/recover same ChannelId cannot reset durable shared eligibility;
- new ChannelId cannot erase Character/Account/World eligibility;
- display-label reuse cannot reset eligibility;
- one-time/high-impact sources cannot use process/channel uptime alone as eligibility truth;
- Channel lifecycle trigger is not a player reward occurrence;
- exact ordinary spawn initialization remains content/runtime-owned.

## 25. PvP implications

- direct PvP execution remains current Channel/Instance-local;
- Character/World-scoped PvP consequences survive Channel/GameSession transition;
- active combat/protected PvP blocks voluntary switch;
- switching cannot clear/re-arm/reduce consequences through a new GameSession;
- Channel failure cannot silently relocate an actor to another combat simulation;
- client disconnect claims cannot create trusted switch exceptions;
- exact PvP/skull/frag/combat formulas remain profile/parity-owned.

## 26. One-World community

- guild identity/membership is not per Channel;
- market/bank/depot/rankings are not separate Channel economies;
- accepted World/guild/private communication remains cross-channel;
- local speech remains channel/spatial;
- party membership may span Channels while gameplay co-location effects require shared simulation;
- recommendation should reduce involuntary fragmentation;
- future client makes current World/Channel understandable;
- Channel is not protocol/ruleset/account namespace.

## 27. Product-facing availability classes

- `SELECTABLE` — fresh attempt may proceed subject to FND-04;
- `CAPACITY_LIMITED` — no immediate admission; target queue may be offered;
- `DRAINING` — no new ordinary admission while drain predicate remains valid;
- `RECOVERING` — same ChannelId recovery path for affected actors;
- `UNAVAILABLE` — not selectable/routable.

Internal runtime states may map to these without exposing infrastructure/security internals.

## 28. Failure and same-Channel recovery

```text
failed Channel A
-> recover same semantic ChannelId A
-> never silently continue in Channel B
```

If recovery cannot continue, actor first reaches a proven safe offline/terminal authority state. Only then may fresh directory/selection/admission target another Channel. Failure does not erase combat/reward/value consequences.

## 29. World-policy revision and cross-world boundary

GAME-CHANNEL uses existing `world_policy_revision` for Channel product policy compatibility. Stale grants/offers cannot silently use superseded policy; durable guard/lifecycle state retains enough revision context for deterministic interpretation/migration.

Changing WorldId is **not** a Channel switch. It remains a separate Character/world lifecycle/transfer/admission concern and cannot bypass world-scoped value/profile isolation.

## 30. Client and Platform boundaries

Client may display/select/request/queue but cannot decide target eligibility/capacity, queue priority, switch eligibility, hard locks, reward eligibility, Channel health/lifecycle, recovery target, final admission or lease/runtime authority.

Future implementation may require separately authorized Platform/Gateway/World Registry changes for directory/recommendation/queue/target-bound grants. This Oteryn-v2 delivery grants no external-repository or production authority.

## 31. DUR-03 and downstream boundaries

GAME-CHANNEL owns product eligibility/multiplicity/lifecycle. DUR-03 retains durable item/currency/value atomicity, transaction idempotency, semantic item location, source/sink lineage, typed custody, stale-authority rejection, ambiguous commit reconciliation and durable audit/restore anti-duplication.

Party/social, market/trade/bank/depot/mail, boss/event/reward, exact PvP, houses/instances and entitlement business rules remain their own domains. `PROD-ENTITLEMENTS-01` remains unaccepted; no paid queue/switch priority is accepted.

## 32. Security invariants

A conforming implementation must prevent:

- client-forged Channel eligibility/capacity/queue priority;
- queue reference becoming bearer authority;
- silent target retarget;
- GameSession/relog reset of switch guard;
- loss of prior successful Channel across logout/restart;
- destination playable authority without recovery-safe guard advancement;
- client-forged maintenance exception;
- display label becoming Channel identity;
- one Character authoritative on two Channels;
- stale Channel owner durable writes;
- switch bypass of combat/trade/item/event/instance locks;
- runtime locality becoming implicit per-Channel durable output policy;
- preference-only, farming-only or PvP-avoidance-only Channel creation;
- draining past reappeared demand/recovery/safety need;
- retirement with sessions/transactions/recovery/stale routes remaining;
- per-Channel reset of shared reward eligibility;
- hidden rate tuning by active Channel count;
- silent alternate-Channel failure recovery;
- co-location privacy bypass;
- Channel lifecycle reward resets;
- GAME-CHANNEL weakening DUR-03 conservation.

## 33. Resource ceilings and analytics

Before implementation acceptance, concrete bounds must exist for applicable directory results, queue entries/bytes, per-actor queue requests, pending offers, co-location hints, switch-attempt state, lifecycle decision fan-out, policy size and event/reward multiplicity work. Numeric values are not invented here.

Privacy-safe analytics should observe active Channel count, population, recommendation override, queue behavior, co-location, switch outcomes, lifecycle trigger/decision reasons, create/drain/abort/removal decisions, source/sink composition, reward conflicts, social fragmentation and same-Channel recovery.

Game Intelligence remains read-only; it cannot autonomously open/close Channels, change rates, waive guards, alter eligibility, move players or repair value.

## 34. Required implementation evidence

Architecture acceptance alone proves no runtime behavior. Future conformance proves at least:

### Selection/queue/co-location

- recommendation remains non-authoritative;
- explicit target cannot silently retarget;
- stale/full/draining/recovering/incompatible target fails safely;
- queue creates no GameSession/lease/value authority and revalidates fresh facts;
- queue duplicate/cancel/expiry and bounded overload are safe;
- co-location hints preserve privacy and one-session authority.

### Switching

- all hard blockers fail closed;
- guard survives logout/new GameSession/reconnect/restart;
- first admission establishes prior Channel without a switch;
- same prior Channel fresh login is not a switch;
- different Channel fresh login is a switch even after prior GameSession ended;
- failed target leaves prior Channel/guard unchanged;
- destination admission + remembered Channel/guard update survive crash as one semantic outcome;
- ambiguous retry cannot skip guard or duplicate destination authority;
- same-Channel reconnect is not a switch;
- trusted exception is unforgeable and consequence-preserving.

### Multiplicity/reward

- every channel-sensitive value source has explicit compiled class;
- missing class fails closed;
- repeatability occurs only when explicitly authored;
- shared eligibility does not repeat on new Channel/GameSession;
- World-unique occurrence cannot mint per Channel;
- stop/restart/new Channel/display alias cannot reset shared eligibility.

### Lifecycle predicates

- preference/rare-spawn/PvP-avoidance alone cannot create a public Channel;
- demand-driven creation proves sustained eligible demand, insufficient healthy eligible capacity with required headroom, policy compatibility, safe resources and Ready-before-selectable;
- recovery-driven capacity preserves same ChannelId recovery for affected actors and never uses new Channel B as silent continuation of failed A;
- low-load drain proves retained capacity/headroom, no policy need and safe incumbent boundary;
- drain aborts/holds when demand returns, retained capacity degrades, recovery pressure appears, policy requires the Channel or safety is threatened;
- terminal retirement occurs only with no sessions/dependent instances/events/transactions/checkpoints/recovery obligations, evidence finalization, fencing and stale-route removal;
- temporary stop/reactivation preserves ChannelId, terminal retirement never reuses it;
- lifecycle numeric windows/thresholds remain PERF/OPS evidence and do not alter qualitative predicates.

### Failure/community

- Full blocks new admission without moving existing actors;
- Draining never silently migrates actors;
- recoverable failure returns same ChannelId;
- stale owner cannot resume;
- alternate Channel only after safe actor state + fresh admission;
- failure does not erase combat/reward/value state;
- World-shared social/economy remains shared while local speech/combat/position remains Channel-local.

## 35. Decision timing

**Must decide now: YES.** Blocks player-visible multichannel selection, queue/co-location, switch/anti-hopping, create/drain/remove product semantics, multiplicity/reward policy, PvP-safe switching, recovery UX and VSL multichannel proof.

Late changes can force migration/rework of Gateway offers, guard persistence, control-plane lifecycle, reward keys/event definitions, presence/client UX, telemetry, support and E2E.

Supersession requires named product/playtest, economy, PvP abuse, availability/recovery, privacy/security, PERF/OPS or explicit product-strategy evidence.

## 36. Deliberately deferred

- numeric switch cooldown;
- queue limits/timeouts/priority;
- demand/load windows and percentages;
- min/max public Channel count;
- capacity/autoscaling thresholds;
- scaling/drain hysteresis/cooldowns;
- orchestrator/service topology;
- exact client UI;
- complete presence visibility matrix;
- exact PvP/boss/reward/spawn/loot/economy formulas;
- market/trade/bank/depot/mail rules;
- physical DB/control-plane schema;
- concrete ANL/protocol IDs;
- production admin exception implementation;
- Premium/VIP/commerce or paid queue priority.

## 37. Acceptance consequence

Only after exact-head self-review, required independent review, exact-head documentation/governance CI, clean review/ownership, unchanged squash merge of #209 and separate lifecycle closeout may programme state become:

```text
GAME-CHANNEL-01
DecisionStatus       = ACCEPTED
DeliveryStatus       = LIFECYCLE_CLOSED
ImplementationStatus = NOT_STARTED
runtime/client authority = NONE
DDL/migration authority  = NONE
production authority     = NONE
```

Architecture acceptance does not authorize multichannel runtime, Gateway queue/recommendation, switch persistence, dynamic scaling implementation, client UI, PvP/reward implementation or production activation.
