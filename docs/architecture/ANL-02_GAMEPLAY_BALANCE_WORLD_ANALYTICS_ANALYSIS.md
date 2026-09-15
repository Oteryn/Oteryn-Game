# ANL-02 — Gameplay, Balance and World Analytics Analysis

- Date: 2026-08-15
- Gate: `ANL-02`
- Issue: #264
- Delivery branch: `docs/arch-f-analytics-integrity`
- Trusted worker base: `main@088b46638ac014cd7928d6b0b75cee44902fe22c`
- DecisionStatus: `CANDIDATE`
- DeliveryStatus: `IN_REVIEW`
- ImplementationStatus: `NOT_STARTED`
- Runtime/client/Platform/PostgreSQL/production authority: **NONE**
- Merge authority: `ARCHITECTURE_COORDINATOR_ONLY`

## 1. Purpose

ANL-02 refines the accepted ADR-0006 Game Intelligence direction into a bounded consumer contract for gameplay, balance and world/content analytics. It defines what an analytical observation means, how it is versioned and qualified, which dimensions are safe to compare, how best-effort loss and revision drift are represented, and what evidence may be handed to gameplay/product owners.

ANL-02 remains observational. It does not own combat formulas, progression rules, world/content authoring, channel policy, economy conservation, runtime authority or production tuning.

```text
authoritative gameplay/state                  -> gameplay/FND/DUR owners
event identity, durability, privacy envelope -> ANL-01
item/currency prevention + conservation       -> DUR-03
channel multiplicity/product semantics        -> GAME-CHANNEL-01
deterministic replay/state provenance         -> SIM-DETERMINISM-01
gameplay/balance/world analytical consumers   -> ANL-02
balance/content/ruleset mutation              -> owning gameplay/product gates only
```

## 2. Source classification

### PROVEN

- ADR-0006 accepts separate operational observability, best-effort gameplay telemetry and durable economy/security audit, and explicitly prohibits automatic balance changes by analytics.
- ANL-01 accepts a small common event envelope plus registered typed/versioned domain payloads, no global event order, explicit durability/privacy/retention classes, immutable EventId/content semantics and bounded query/replay/export foundations.
- ANL-01 intentionally leaves `GAME_EVENT_FOUNDATION_REGISTRY.json.event_types` empty so concrete gameplay/DUR owners register real payload families instead of the analytics foundation inventing them.
- `MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md` makes metrics non-authoritative and requires GameNode/Channel/World scope to remain explicit.
- GAME-CHANNEL-01 accepts `WorldId` as one persistent product/economy/community/ruleset boundary, ChannelRef as a parallel public simulation identity, and explicit reward/source multiplicity semantics.
- SIM-DETERMINISM-01 makes replay and deterministic hashes evidence only; analytics may use approximate arithmetic when it cannot feed back into authority.
- GAME-VISION-01 accepts category-level success evidence including Reference correctness, core-loop health, progress/value trust, economy health and product/operational health while leaving numeric targets milestone-owned.

### DERIVED

- ANL-02 can safely freeze analytical evidence semantics now without selecting a warehouse, lake, dashboard, stream processor or service topology.
- Concrete metric implementation remains blocked where the owning gameplay/content domain has not registered event families carrying the required semantic facts.
- A metric value without its denominator, revision context, collection quality and source lineage is not sufficient evidence for a balance decision.

## 3. Decision timing

### Must this be decided now?

**YES — consumer/evidence semantics only.**

If ANL-02 is deferred until dashboards or data jobs are implemented, gameplay event producers may omit denominator, attribution, geography, revision or loss metadata that cannot be reconstructed reliably later. Retrofitting those semantics after production collection would create incomparable time series and encourage balance decisions from ambiguous telemetry.

### Concrete downstream work blocked without this decision

- registration of gameplay-domain telemetry payloads intended to support balance/world-health analysis;
- stable metric definitions and regression comparisons across code/content/ruleset revisions;
- privacy-safe world/content utilization datasets;
- alpha/milestone analytical evidence packages;
- deterministic replay-to-analytics comparison tooling when later authorized.

### What becomes expensive to change later?

- the grain and denominator of historical metrics;
- cohort and revision compatibility rules;
- sampling/loss interpretation;
- geography precision retained for world analytics;
- whether derived values are reproducibly tied back to exact source evidence.

### Evidence that may supersede this candidate

Named production/playtest evidence may justify later changes to metric dimensions, default cohorts, aggregation windows, sampling policies or resource limits. A superseding proposal must preserve historical definition revision identity and must not rewrite old observations as if they used the new definition.

## 4. Authority matrix

| Concern | Authoritative owner | ANL-02 authority |
|---|---|---|
| Game command acceptance/result | FND/gameplay domains | Read-only consumer only |
| Combat/progression/item/world formulas | Owning gameplay/content gates + SIM semantics | Measure/compare only |
| Character/item/currency durable mutation | DUR/gameplay owners | Read-only evidence only |
| Item/currency conservation | DUR-03 | May consume durable evidence; never replace prevention |
| Event identity/durability/privacy | ANL-01 | Must preserve; no envelope reinterpretation |
| Channel/reward multiplicity | GAME-CHANNEL + reward/source owner | Dimension/interpret only |
| Deterministic replay/hash semantics | SIM-DETERMINISM-01 | Compare/corroborate only |
| Metric/cohort/derived observation semantics | ANL-02 | Candidate owner |
| Balance/content/ruleset changes | Gameplay/product/content owners | No mutation authority |
| Production rollout/LiveOps | Separate owner authorization | None |

## 5. Analytical data classes

ANL-02 consumes, but does not collapse, the ANL-01 classes.

### 5.1 Operational observability

Process/service health such as tick latency, queue depth or channel health remains low-cardinality operational observability. It may be correlated at coarse build/world/channel/time scope with gameplay analytics, but player/item/session identifiers must not become ordinary Prometheus labels.

Operational metrics are context, not proof of a player/gameplay fact.

### 5.2 Best-effort gameplay telemetry

High-volume facts such as damage/healing, ability outcomes, experience, monster interactions, coarse area presence and session summaries may use `BEST_EFFORT_TELEMETRY` according to ANL-01.

A best-effort dataset:

- may be sampled or dropped only under a registered policy;
- must carry loss/sampling quality evidence into every derived observation;
- may support balance/world-health description and comparison;
- must never be presented as conservation-complete or security-complete evidence.

### 5.3 Durable-audit-derived analytical facts

ANL-02 may read DUR-03/ANL-01 durable audit projections when a balance metric requires trustworthy value source/sink, loot, reward or transaction evidence.

This does not let ANL-02 define the transaction, mint/burn, reward eligibility or item-location semantics. The derived metric must name the exact durable source family and its completeness state.

## 6. Metric definition contract

Every material analytical metric is defined by an immutable `(metric_id, metric_revision)` pair. The exact storage/wire representation remains implementation-owned.

A metric definition must declare at least:

```text
metric_id
metric_revision
purpose / owning analytical question
semantic grain
unit and aggregation operation
required source event families + supported schema revisions
required durability/completeness class per source
subject/cohort dimensions
mandatory revision dimensions
time basis and observation-window semantics
denominator and exposure definition
inclusion/exclusion rules
sampling/weighting policy
missing/duplicate/out-of-order handling
late-data/finalization policy
privacy floor + permitted projection purpose
retention profile for the derived dataset
source-lineage requirements
known interpretation limits / confounders
```

Changing meaning, denominator, inclusion/exclusion, weighting, unit, source semantics or revision compatibility requires a new `metric_revision`. A label-only presentation change does not.

A historical result is never silently recomputed under a newer definition and published as the old revision.

## 7. Observation/evidence record

A human- or machine-consumable ANL-02 observation must bind enough context to reproduce its meaning:

```text
metric_id + metric_revision
observation window
WorldId
Channel/Instance scope policy when applicable
ruleset/content/world-policy revisions as applicable
server build/protocol revision when material
SIM profile/replay provenance when used
cohort/denominator definition revision
source event type/schema set
source checkpoint/window identity
sampling/loss/coverage summary
transform/query implementation revision or digest
sample/exposure size
result + unit
data-quality vector
privacy/retention/export classification
creation timestamp
```

Raw player identity is not part of ordinary analytical presentation. Where individual longitudinal analysis is approved, use ANL-01 `AnalyticsActorId` under its identity-domain/epoch rules.

## 8. Evidence-quality vector

ANL-02 does not invent one opaque universal confidence score. Every observation carries explicit quality dimensions.

Minimum dimensions:

### 8.1 Source completeness

- `PROVEN_COMPLETE_FOR_DECLARED_DURABLE_SCOPE` — only when durable source-set completeness/checkpoints and relevant transaction/event-set rules prove the declared window is complete;
- `NO_KNOWN_GAP_BEST_EFFORT` — best-effort ingestion reports no known loss for the declared collection path, but completeness is not proven;
- `PARTIAL` — known sampling/drop/unsupported-schema/privacy suppression or incomplete interval exists;
- `UNKNOWN` — completeness cannot be classified.

`NO_KNOWN_GAP_BEST_EFFORT` must never be promoted to conservation/security completeness.

### 8.2 Schema compatibility

`SUPPORTED`, `MIXED_SUPPORTED_REVISIONS`, `PARTIAL_UNSUPPORTED`, or `UNKNOWN` for the exact source window. Unsupported durable events follow ANL-01 quarantine/reject semantics rather than being reinterpreted.

### 8.3 Duplicate/order handling

Record whether EventId deduplication, TransactionEventRef completeness and any required causal/runtime-order reconciliation were satisfied for the metric's source class.

### 8.4 Sampling/weighting

Record no-sampling or the exact sampling policy revision and weighting method. Unsampled and sampled series are not automatically comparable.

### 8.5 Revision homogeneity

Record whether the observation is single-revision or an explicitly stratified combination. Mixed incompatible ruleset/content/build/SIM semantics may not be presented as one homogeneous balance result.

### 8.6 Privacy suppression

Record aggregation/suppression that materially changes denominator or dimensions. Privacy-filtered absence is not gameplay absence.

## 9. Cohort and denominator discipline

Every comparative metric must make cohort and denominator explicit. Candidate dimensions include only those semantically justified by owning domains, for example:

- World/ruleset/profile;
- Channel or Instance scope where channel-local behavior matters;
- server build, content/world bundle and relevant formula/ability revisions;
- class/vocation and level/progression band;
- solo/party/shared-experience context;
- equipment/power band where a privacy-safe stable definition exists;
- encounter/monster/content family;
- coarse authored world region semantics;
- activity duration, eligible exposure, attempts or other metric-specific denominator.

Rules:

1. Reference and Evolved Worlds are separate populations; they are never pooled as one balance cohort by default.
2. Different ruleset/content/formula revisions are stratified unless the metric definition explicitly proves semantic comparability.
3. Channel-local copies are not automatically independent durable reward/source opportunities; GAME-CHANNEL multiplicity policy remains an input.
4. Per-time rates declare which time is counted: active encounter time, session time, eligible exposure or another named semantic clock.
5. Missing events are not converted to zero unless source completeness proves zero is the correct semantic value.
6. Cohort selection may not use a post-outcome filter that silently creates survivorship bias without declaring it.
7. Sample/exposure size accompanies every aggregate and comparison.

ANL-02 does not freeze one statistical test, confidence interval or significance threshold. Those may differ by analytical question, but the method/version must be named in evidence used for a decision.

## 10. Gameplay and balance dimensions

ADR-0006's accepted scope is refined into analytical dimensions rather than fixed numeric targets.

### 10.1 Progression and activity efficiency

Potential metrics include experience/progress per declared exposure unit, completion/attempt rates, travel/setup versus activity time and recovery/restock burden.

They must be stratified by relevant world/ruleset/content/player-progression context. ANL-02 does not decide intended progression speed.

### 10.2 Combat pressure and survival

Potential metrics include damage dealt/received, effective healing/overhealing, deaths/survival, resource consumption, interruption/failure outcomes and encounter duration.

A death rate alone does not prove content is overtuned; access, player level, equipment, party composition, revision and data completeness remain potential confounders.

### 10.3 Ability/spell/item efficiency

Potential metrics include attempts, accepted outcomes, targets/effects, damage/healing/resource cost, delayed/periodic outcomes and failure/cancellation reasons where owning domains expose stable typed evidence.

Metric semantics must follow the owning ability/combat/item contract. Analytics must not derive hidden authoritative formula meaning from presentation logs.

### 10.4 Class/vocation/party balance

Comparisons require explicit progression band, activity/encounter context, party composition and relevant equipment/power context. One global average must not be treated as proof that a class or party mode is over/underpowered.

### 10.5 Value and cost in gameplay analysis

Loot/profit/cost metrics may combine best-effort activity facts with durable source/sink evidence. Trusted market valuation is a separate versioned input where a market contract exists; absence of such a contract must not be replaced by an arbitrary live price scrape.

DUR-03 conservation evidence outranks best-effort telemetry when the question is whether value existed or was duplicated.

## 11. World/content health dimensions

World analytics preserves ADR-0005 authored hierarchy and ADR-0006 privacy constraints.

Candidate dimensions include:

- occupancy/exposure by `Area`, `Subarea`, `EncounterZone`, `RaidCell`, `RaidAnchor`, `Region` and `Chunk` only where the exact World Bundle/content revision defines that semantic identity;
- encounter/spawn utilization, lifetime and kill/abandonment rates;
- unusually lethal or unusually unused authored content;
- travel flow at the coarsest precision sufficient for the analytical purpose;
- pathfinding/collision/teleport/accessibility failure indicators;
- quest/event/raid/content participation and completion where owning domains define events;
- Channel/Instance distribution and channel-friction observations;
- change attribution across exact World Bundle/content/ruleset/server-build revisions.

Precise continuous movement trails are not a default ANL-02 dataset. A finer-grained location history requires a separately accepted purpose/privacy/retention/access policy and must not be justified merely because storage is available.

## 12. Balance/world-health evidence packet

A recommendation to a gameplay/product/content owner should use a bounded evidence packet containing:

1. analytical question and proposed owner;
2. metric IDs/revisions;
3. exact observation windows and Worlds;
4. cohort/denominator definitions;
5. relevant ruleset/content/build/SIM revisions;
6. source event types/schema revisions and durability classes;
7. sample/exposure size;
8. quality vector including known loss/sampling/privacy suppression;
9. result distributions, not only a single average where distribution matters;
10. named confounders/alternative explanations;
11. comparison baseline and method revision;
12. links/refs to reproducible source checkpoints or bounded evidence package;
13. explicit conclusion classification: `DESCRIPTIVE`, `REGRESSION_SIGNAL`, `BALANCE_HYPOTHESIS`, or `REPLAY_CORROBORATED` as applicable;
14. proposed next experiment or gameplay-owner question;
15. explicit statement that ANL-02 has no mutation authority.

`BALANCE_HYPOTHESIS` is not an accepted balance decision. `REPLAY_CORROBORATED` means deterministic evidence supports a behavioral difference; it does not decide whether the behavior is desirable.

## 13. Regression semantics

A regression signal requires a named compatible comparison baseline. At minimum it must account for changes in:

- server build;
- ruleset/formula revision;
- content/World Bundle revision;
- channel/instance/product policy where material;
- metric/cohort definition revision;
- collection/sampling policy;
- population/exposure composition.

If more than one material semantic dimension changed, the observation must remain multi-change/ambiguous unless an experiment, replay or owning-domain evidence isolates the cause.

A time-series discontinuity alone is not proof of engine regression.

Regression acceptance is fail closed. `REGRESSION_NOT_EVALUATED` is only a pre-evaluation state. Once a material regression evaluation is attempted, every applicable prerequisite used by the candidate contract must be positively satisfied: required source completeness/schema quality; versioned minimum sample/exposure and unsuppressed required evidence; baseline/current metric, cohort, denominator and material semantic-revision comparability or explicit stratification; duplicate/order/late-event/source-checkpoint reconciliation and required backfill/recompute finality; privacy/suppression sufficiency; and exact method/provenance for the declared conclusion.

If any applicable prerequisite is not affirmatively satisfied after evaluation begins, the mandatory disposition is `REGRESSION_EVIDENCE_INSUFFICIENT`. A warning, caveat or degraded dashboard presentation may accompany that disposition, but it cannot coexist with a green `NO_MATERIAL_REGRESSION_SUPPORTED` conclusion for the same insufficient evidence. `NO_MATERIAL_REGRESSION_SUPPORTED` is permitted only when all applicable prerequisites are positively satisfied and the retained evidence names the exact baseline, method/threshold revision, windows, cohorts, checkpoints and material confounders.

## 14. Deterministic replay integration

SIM-DETERMINISM-01 may provide exact replay provenance, normalized input order and state/result hashes.

ANL-02 may use that evidence to:

- reproduce a candidate behavior under the same semantic revision set;
- compare exact output across builds/targets;
- identify the first deterministic divergence;
- validate deterministic aggregator logic against fixed event fixtures.

ANL-02 must not:

- treat best-effort event order as a substitute for SIM authoritative input order;
- reconstruct missing authoritative state from incomplete telemetry;
- replay live commands into production;
- use replay output to mutate live state or auto-apply balance changes.

## 15. Privacy, retention and access

ANL-01 remains normative.

### Ordinary balance/world datasets

Prefer `PSEUDONYMOUS_ANALYTICS` or `INTERNAL_NON_PERSONAL` projections. Raw AccountId/CharacterId is not admitted merely for analyst convenience. `AnalyticsActorId` is purpose/domain/epoch scoped and a new epoch uses a fresh pseudonym.

### Geography/time re-identification

Fine location, exact timestamps, rare party composition and small cohorts can re-identify a player even without names. Therefore presentation/export requires policy-owned aggregation/suppression rules appropriate to the purpose. ANL-02 does not invent a universal anonymity threshold.

### Retention

Raw events, intermediate feature/projection datasets, aggregate metrics and exported evidence packages each require an accepted finite retention profile. Derived data does not inherit unlimited retention simply because identifiers were reduced.

### Access roles

Balance/world analysts receive only the least-privilege analytical views required for their purpose. Ordinary ANL-02 roles do not receive the privileged pseudonym-to-operational-identity mapping. Security investigation belongs to ANL-03/ANL-04 boundaries.

## 16. Resource-bound needs

ANL-02 inherits the normative ANL-01 hard bounds, including event/payload size, bounded replay windows, paged analytics/investigation results and bounded evidence packages. In particular, current registry limits include:

- query page row bound (`ANL01-QUERY-PAGE-ROWS`);
- query result byte bound (`ANL01-QUERY-RESULT-BYTES`);
- replay window event bound (`ANL01-REPLAY-WINDOW-EVENTS`);
- evidence package byte bound (`ANL01-EVIDENCE-PACKAGE-BYTES`).

ANL-02 additionally requires every future implementation to bound before execution/materialization:

- requested time range/window count;
- group-by/result cardinality;
- cohort expansion count;
- cross-source join fan-out;
- geography resolution/result cells;
- backfill/recompute work units;
- concurrent analytical jobs and retained intermediate bytes.

Exact operational numbers are PERF/OPS/implementation evidence, not guessed by this architecture. Any externally controlled new limit absent from `RESOURCE_LIMITS_REGISTRY.json` must be registered by the owning implementation contract before acceptance.

No metric computation may hold the authoritative runtime writer or synchronously block gameplay on remote analytical storage.

## 17. Failure and degradation semantics

| Condition | Required ANL-02 interpretation |
|---|---|
| Best-effort queue drop/sampling | mark quality; never claim complete window |
| Unsupported best-effort schema | counted exclusion/drop; affected metric partial/unknown |
| Unsupported durable schema | follow ANL-01 quarantine/reject; no reinterpretation |
| Duplicate delivery | EventId-deduped derived effect only |
| Out-of-order related events | bounded defer/reconcile or partial result; never fabricate order |
| Missing required denominator/exposure | observation invalid/unknown, not zero |
| Mixed incompatible revisions | stratify or reject comparison |
| Privacy policy missing | no collection/projection/disclosure for that class |
| Source checkpoint gap | affected evidence marked partial/unknown |
| SIM replay provenance incomplete | no replay-corroborated claim |
| Analytical storage/query unavailable | gameplay unchanged; analysis delayed/unavailable |

## 18. Failure-scenario mapping

- `FS-ANALYTICS-TELEMETRY-OVERFLOW`: **PASS at candidate semantic level** — loss remains counted/bounded and propagates into evidence quality; no completeness claim.
- `FS-EVENT-DUPLICATE-DELIVERY`: **PASS at candidate semantic level** — derived effect is EventId-idempotent.
- `FS-EVENT-OUT-OF-ORDER`: **PASS at candidate semantic level** — consumer reconciles/buffers/fails partial rather than inventing order.
- `FS-ANALYTICS-PRIVACY-POLICY`: **PASS at candidate semantic level** — unclassified purpose/privacy/retention/access blocks production projection.
- `FS-DETECTOR-FALSE-POSITIVE`: `NOT_APPLICABLE` to ANL-02 balance metrics; ANL-03 owns security detector semantics.

Architecture PASS does not claim runtime evidence.

## 19. Proposed decisions

### ANL02-D1 — Versioned metric definition is the unit of analytical meaning

- Must decide now: **YES**.
- Blocks: stable event producers, dashboards, regression comparison and evidence packages.
- Owner: ANL-02.
- Failure/security/resource implication: prevents silent definition drift; every result remains attributable to bounded source/query work.
- Supersession evidence: measured implementation constraints may refine fields, but old revision meaning remains immutable.

### ANL02-D2 — Quality is a vector, not a single opaque confidence number

- Must decide now: **YES**.
- Blocks: trustworthy use of best-effort telemetry and durable-audit-derived metrics.
- Owner: ANL-02, constrained by ANL-01 durability/privacy semantics.
- Failure implication: missing/sampled/unsupported data remains visible instead of being converted to false certainty.
- Supersession evidence: a later validated domain-specific calibrated score may be added, but it cannot erase source quality dimensions.

### ANL02-D3 — Comparative analytics must bind cohort, denominator and semantic revisions

- Must decide now: **YES**.
- Blocks: class/vocation, party, encounter and pre/post-release comparisons.
- Owner: ANL-02 for analytical semantics; underlying cohort facts remain gameplay-owned.
- Failure implication: reduces confounding and invalid cross-revision aggregation.
- Supersession evidence: new cohort dimensions may be added when owning domains expose stable evidence.

### ANL02-D4 — Analytics may recommend but never mutate balance/world/content state

- Must decide now: **YES**, already constrained by ADR-0006 and reinforced here.
- Blocks: safe consumer implementation and future AI/investigation composition.
- Owner: architecture-wide accepted boundary; ANL-02 preserves it.
- Failure implication: analytical error cannot become an authoritative gameplay incident by itself.
- Supersession evidence: none within ANL-02; any change would require an explicit higher-level owner decision that supersedes ADR-0006 safety boundaries.

### ANL02-D5 — Coarse geography is the ordinary default

- Must decide now: **YES** as a privacy/storage boundary.
- Blocks: world telemetry family design.
- Owner: ANL-02 constrained by ANL-01 privacy policy; authored semantic hierarchy remains world/content-owned.
- Failure implication: reduces unnecessary player tracking and cardinality.
- Supersession evidence: a named security/product purpose with accepted privacy/retention/access and measured need for finer evidence.

## 20. DECISIONS_NOT_TAKEN

ANL-02 deliberately does **not** decide:

- concrete event type IDs/payload schemas owned by gameplay/DUR domains;
- warehouse, lake, broker, query engine, dashboard or vendor;
- physical database schema/index/partitioning;
- collector topology or deployment model;
- exact metric numeric targets, balance thresholds or KPI gates;
- one universal statistical test/significance/confidence threshold;
- exact sampling percentages or late-data windows;
- exact privacy suppression/anonymity thresholds or retention durations;
- class/vocation, combat, progression, reward, loot, market or world-content rules;
- channel lifecycle/multiplicity policy;
- automatic balancing, automatic content changes or LiveOps control loops;
- runtime/client/Platform/production implementation.

## 21. CROSS_DOMAIN_FINDINGS

### ANL02-XD-01 — concrete gameplay analytical event families are not registered

```yaml
cross_domain_finding:
  id: ANL02-XD-01
  observed_in_domain: ANL-02
  target_owner: owning gameplay/content domains + ANL-01 registry integration owner
  severity: P1
  evidence: ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_ANALYSIS.md section 8; GAME_EVENT_FOUNDATION_REGISTRY.json event_types=[]; issue #264 requires metric/evidence consumers only
  conflict_or_gap: ANL-02 can define consumer semantics, but no concrete gameplay-domain event payload catalogue currently exists from which production balance/world metrics can claim coverage.
  required_before: any ANL-02 implementation claims concrete metric coverage or production collection for gameplay/world families
  worker_action: REPORT_ONLY
```

### ANL02-XD-02 — metric attribution depends on unfinished owning gameplay families

```yaml
cross_domain_finding:
  id: ANL02-XD-02
  observed_in_domain: ANL-02
  target_owner: GAME-ABILITY-01 / GAME-AI-01 / GAME-INTERACTION-01 and later quest/event/combat owners as applicable
  severity: P2
  evidence: ADR-0006 gameplay/world metric requirements; ARCHITECTURE_ANALYSIS_GAP_REGISTER.md registered gameplay gaps; ANL-01 typed-payload ownership rule
  conflict_or_gap: ability outcome attribution, encounter/spawn semantics, interaction/accessibility outcomes and quest/event participation cannot be invented by analytics before the owning domain semantics are stable.
  required_before: corresponding ANL-02 metric families become canonical/implementable
  worker_action: REPORT_ONLY
```

## 22. Candidate conclusion

`RECOMMENDATION` — ANL-02 is mature enough for a bounded candidate contract because its consumer/evidence semantics can be frozen independently of storage/vendor/runtime topology. Acceptance must preserve the cross-domain prerequisite that concrete gameplay event families remain owned and registered by their gameplay/content producers.

The worker draft does not make these semantics canonical. Architecture Coordinator audit/acceptance is required, and implementation remains `NOT_STARTED`.