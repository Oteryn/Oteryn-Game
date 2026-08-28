# ANL-02 — Gameplay, Balance and World Analytics Contract Candidate

- Date: 2026-08-15
- Gate: `ANL-02`
- Issue: #264
- DecisionStatus: `CANDIDATE`
- DeliveryStatus: `IN_REVIEW`
- ImplementationStatus: `NOT_STARTED`
- Status on worker branch: **CANDIDATE / NONBINDING**
- Canonical semantic effect: only after Architecture Coordinator acceptance/merge under repository governance
- Runtime/client/Platform/PostgreSQL/production authority: **NONE**
- Merge authority: `ARCHITECTURE_COORDINATOR_ONLY`
- Analysis source: `ANL-02_GAMEPLAY_BALANCE_WORLD_ANALYTICS_ANALYSIS.md`

## 1. Purpose

Freeze the minimum consumer semantics required for trustworthy gameplay, balance and world/content analytics without selecting implementation technology or granting mutation authority.

ANL-02 owns:

- analytical metric-definition identity/versioning;
- observation/cohort/denominator semantics;
- deterministic session/hunt grouping and reporting-window semantics;
- evidence-quality representation;
- revision-aware comparison/regression semantics;
- minimum dashboard/presentation truth and warning semantics;
- privacy-safe gameplay/world analytical projections;
- bounded analytical evidence-package requirements.

ANL-02 does **not** own gameplay formulas, gameplay-session authority, content rules, channel/reward policy, item/currency prevention, authoritative replay, sanctions, warehouse/DB/dashboard technology, runtime implementation or production rollout.

## 2. Authority invariant

```text
ANL-02 output
= observation / comparison / hypothesis / reproducible evidence
!= gameplay authority
!= balance authority
!= content mutation authority
!= economy mutation authority
```

No ANL-02 metric, session/hunt grouping, dashboard, anomaly, model output or evidence packet may directly create/extend a gameplay session, change authoritative gameplay state, ruleset/content, reward rates or production configuration.

## 3. Upstream contracts

A conforming ANL-02 consumer preserves:

- ADR-0006 data-class, dashboard/regression and read-only boundaries;
- ANL-01 EventId/schema/durability/privacy/retention/order/idempotency semantics;
- FND-owned `GameSessionId`/session-generation authority when those fields are consumed as evidence;
- DUR-03 conservation/prevention authority when value facts are consumed;
- GAME-CHANNEL World/Channel/multiplicity semantics;
- SIM-DETERMINISM replay/state provenance when deterministic corroboration is claimed;
- owning gameplay/content semantics for every domain payload it interprets.

It must never invent a missing gameplay semantic from generic event fields or presentation logs.

## 4. Metric-definition identity

Every material metric has a stable logical `metric_id` and immutable positive `metric_revision`.

A metric definition declares at least:

1. purpose/analytical question;
2. semantic grain and unit;
3. aggregation operation;
4. required typed source event families and supported schema revisions;
5. required source durability/completeness class;
6. observation-window/time basis, including session/hunt/reporting-window semantics when applicable;
7. cohort dimensions;
8. denominator/exposure definition;
9. inclusion/exclusion rules;
10. sampling/weighting policy;
11. duplicate/order/late/missing-data handling;
12. mandatory semantic revision dimensions;
13. privacy floor and permitted projection purpose;
14. derived-dataset retention profile;
15. source-lineage/reproducibility requirements;
16. known confounders/interpretation limits.

A semantic change to any item above creates a new metric revision. Historical results retain the revision under which they were computed.

## 5. Observation contract

Every material observation binds:

```text
metric_id + metric_revision
observation window
session/hunt grouping contract identity/revision when applicable
reporting-window identity/time basis when applicable
WorldId
Channel/Instance scope policy when applicable
ruleset/content/world-policy revisions when material
server build/protocol revision when material
SIM profile/replay provenance when used
cohort/denominator definition revision
source event type/schema set
source checkpoint/window identity
sampling/loss/coverage summary
transform/query implementation revision or digest
sample/exposure size
result + unit
evidence-quality vector
privacy/retention/export classification
```

A value lacking required context is not a conforming decision-grade ANL-02 observation.

## 6. Evidence-quality vector

Every observation records explicit quality dimensions rather than only one opaque confidence score.

### Source completeness

- `PROVEN_COMPLETE_FOR_DECLARED_DURABLE_SCOPE`
- `NO_KNOWN_GAP_BEST_EFFORT`
- `PARTIAL`
- `UNKNOWN`

Only accepted durable source/checkpoint/event-set evidence may establish `PROVEN_COMPLETE_FOR_DECLARED_DURABLE_SCOPE`.

`NO_KNOWN_GAP_BEST_EFFORT` means no observed loss in the declared best-effort path; it is not conservation/security completeness.

### Schema compatibility

- `SUPPORTED`
- `MIXED_SUPPORTED_REVISIONS`
- `PARTIAL_UNSUPPORTED`
- `UNKNOWN`

### Required recorded dimensions

The observation additionally records:

- duplicate/order reconciliation status;
- sampling policy revision and weighting state;
- semantic revision homogeneity/stratification;
- material privacy suppression;
- source checkpoint/gap state.

Known source loss or unsupported semantics may not be hidden by an aggregate or presentation layer.

## 7. Cohort and denominator rules

1. Every comparative metric has an explicit denominator/exposure definition.
2. World/ruleset/profile is mandatory context for gameplay-value interpretation.
3. Reference and Evolved Worlds are separate cohorts unless an explicit comparison intentionally places them side by side; they are never silently pooled.
4. Incompatible ruleset/content/formula/metric revisions are stratified or rejected, not averaged into one homogeneous result.
5. Channel/Instance dimensions are included when they can alter exposure/simulation; Channel multiplicity does not imply independent durable reward/source multiplicity.
6. Solo/party, class/vocation, progression band, encounter/content and equipment/power dimensions are used only when their owning domain exposes stable semantics.
7. Sample/exposure size accompanies every aggregate/comparison.
8. Missing required evidence is `UNKNOWN`/partial, not zero.
9. Post-outcome cohort filters that can create survivorship bias must be declared.
10. One universal statistical method or numeric significance threshold is not frozen by this contract; any method used in a material decision is versioned/named in the evidence.

### 7.1 Deterministic session/hunt grouping

ANL-02 “session” or “hunt” is an **analytical grouping only**. It is not FND gameplay-session authority, cannot mint/extend/resume `GameSessionId`, and cannot grant gameplay rights.

A metric that claims session/hunt semantics MUST use a versioned grouping contract with an explicit producer-owned correlation identity and explicit boundary semantics from typed producer evidence registered through ANL-01/FND identity governance. The concrete identifier type/event family is producer-owned and intentionally not invented by ANL-02.

Required grouping behavior:

1. A session/hunt grouping is scoped to one `WorldId`; it MUST NOT silently span Worlds.
2. Group start, continuation and terminal boundary are determined by registered grouping identity/boundary evidence, not analyst-local wall-clock inactivity guesses, process restarts, dashboard refreshes or ingestion gaps.
3. A reconnect or `GameSessionId`/`session_generation` change MUST NOT by itself merge two otherwise unrelated analytical groups. If producer-owned grouping identity explicitly continues across the reconnect, the analytical group continues and the game-session/generation change is retained as a segment boundary/dimension; without continuity evidence, implementations MUST NOT heuristically stitch the groups.
4. Channel or Instance transition MUST NOT by itself create a new hunt/session when explicit grouping identity continues, but source `ChannelId`/`InstanceId` is retained. Metrics whose exposure or semantics differ by Channel/Instance MUST segment/stratify those contributions before compatible higher-level rollup.
5. An explicit terminal boundary or a new incompatible grouping identity ends the group. Missing terminal evidence leaves the analytical group open/incomplete rather than authorizing a guessed semantic stop time.
6. Duplicate/replayed source events produce at most one analytical effect per accepted ANL-01 `EventId`/idempotency semantics.
7. Out-of-order events are reconciled using accepted ANL-01 ordering/causation evidence. If ordering is insufficient to establish a boundary or attribution, the affected grouping/metric is partial or inconclusive; implementations MUST NOT manufacture order.
8. A semantic revision change that makes contributions incompatible does not silently reset or merge the producer grouping. The metric MUST stratify into compatible semantic segments or reject the mixed aggregate according to its metric revision.
9. If required explicit grouping identity/boundary evidence is unavailable, a session/hunt metric is unavailable/partial for that scope; implementations MUST NOT invent a timeout-based hunt definition and still claim conformance to that metric revision.

An implementation MAY use bounded operational expiry to release memory for an open group, but such expiry is storage/execution behavior only. It MUST NOT be reported as the semantic hunt/session end unless the metric's accepted grouping contract explicitly defines that boundary.

### 7.2 Deterministic UTC reporting windows

ADR-0006's UTC-day split is normative for calendar-day reporting.

For UTC day `D`, the reporting interval is the half-open interval:

```text
[D 00:00:00 UTC, (D + 1 day) 00:00:00 UTC)
```

Rules:

1. Calendar reporting assignment uses the source event's accepted canonical occurrence wall time (`occurred_at_wall` for event families that declare it valid/required for this purpose), normalized to UTC; ingestion, persistence, replay or query execution time MUST NOT choose the reporting day.
2. An event exactly at `00:00:00 UTC` belongs to the new day. An event strictly before that boundary belongs to the preceding day.
3. A session/hunt spanning UTC midnight is not semantically terminated merely by the calendar boundary. Session-wide evidence may span days, while each daily aggregate allocates event-derived contribution/exposure to the UTC window containing canonical event occurrence.
4. A duplicate or replay retains the original event-time window and contributes at most once.
5. A late-arriving event is assigned to its original event-time window, never to the arrival/reprocessing day. Any published result affected by late reconciliation MUST retain a source checkpoint/as-of/recomputation identity sufficient to reproduce which event set produced that result; correction MUST NOT silently erase provenance.
6. Out-of-order delivery does not change window assignment. If required canonical occurrence time is absent/invalid or semantic ordering is insufficient for a time-weighted attribution, the affected result is partial/inconclusive rather than assigned by arrival time.
7. Mixed incompatible metric/ruleset/content/formula/world-policy revisions at a reporting boundary are stratified/rejected under the metric contract; the UTC split does not authorize semantic pooling.
8. Numeric allowed-lateness/finalization horizons are implementation/operations evidence decisions. Their value may affect when a report is considered operationally settled, but not the event-time window to which an event belongs.

Required deterministic fixtures include at least: immediately before midnight; exactly at midnight; immediately after midnight; a hunt spanning midnight; reconnect with and without explicit grouping continuity; session-generation change; Channel/Instance transition; duplicate replay; out-of-order delivery; late arrival after an earlier report; an open/missing terminal boundary; missing grouping identity; and incompatible semantic-revision transitions.

## 8. Gameplay/balance analytical families

Subject to concrete producer events registered by owning gameplay domains, ANL-02 supports definitions for:

- progression/activity efficiency;
- damage dealt/received and combat pressure;
- effective healing/overhealing;
- death/survival and recovery burden;
- mana/resource/consumable use;
- ability/spell/item-use efficiency and failure/cancellation outcomes;
- encounter/monster performance and attribution;
- class/vocation/progression-band comparisons;
- solo/party/shared-experience comparisons;
- equipment/power-band comparisons where privacy-safe semantics exist;
- gameplay cost/loot/profit analysis using durable value evidence where correctness requires it;
- pre/post build/content/ruleset regression analysis.

These families define analytical questions, not expected numeric outcomes.

## 9. World/content analytical families

Subject to exact World Bundle/content revision semantics and owning domain events, ANL-02 supports:

- occupancy/exposure by authored `Area`, `Subarea`, `EncounterZone`, `RaidCell`, `RaidAnchor`, `Region` or `Chunk`;
- spawn/encounter utilization and lifetime;
- unusually unused/lethal content;
- privacy-minimized travel flow;
- pathfinding/collision/teleport/accessibility indicators;
- quest/event/raid/content participation/completion;
- Channel/Instance distribution and channel-friction observations;
- change attribution across World Bundle/content/ruleset/server-build revisions.

Precise continuous player movement history is not an ordinary ANL-02 requirement.

## 10. Balance/world-health evidence packet

A bounded packet handed to a gameplay/product/content owner contains at least:

- analytical question and intended decision owner;
- metric IDs/revisions;
- exact Worlds/windows/cohorts/denominators;
- session/hunt grouping and reporting-window contract revision when applicable;
- relevant semantic/build/content revisions;
- source event/schema/durability set;
- sample/exposure size;
- evidence-quality vector;
- result distribution/summary appropriate to the metric;
- known confounders and alternative explanations;
- comparison baseline/method revision;
- reproducible source checkpoint/query/transform references;
- conclusion classification;
- explicit `IMPLEMENTATION_AUTHORITY: NONE` / no automatic mutation statement.

Allowed conclusion classifications are:

- `DESCRIPTIVE` — reports observed distribution/rate;
- `REGRESSION_SIGNAL` — material compatible-baseline change requiring owner review;
- `BALANCE_HYPOTHESIS` — evidence suggests a gameplay/product imbalance but does not decide it;
- `REPLAY_CORROBORATED` — SIM evidence reproduces/isolates a behavioral divergence.

These values express analytical use, not architecture acceptance status.

## 11. Regression rule

A regression claim must name a compatible baseline and account for, as applicable:

- server build;
- ruleset/formula revision;
- content/World Bundle revision;
- channel/instance/product policy;
- metric/cohort revision;
- sampling/collection policy;
- population/exposure composition.

Multiple material simultaneous changes keep the cause ambiguous unless a controlled experiment, deterministic replay or owning-domain evidence isolates it.

### 11.1 Dashboard presentation and regression-acceptance boundary

A dashboard is a **presentation of ANL-02 evidence**, not an alternate metric definition or authority layer. Any dashboard/panel used for a material balance, world-health, regression, release or product decision MUST make the evidence state visible enough that a reader cannot mistake partial, suppressed, stale or incomparable data for complete homogeneous truth.

For every material displayed observation or comparison, the presentation MUST expose directly or through an unambiguous drill-down/evidence affordance at least:

- `metric_id` and `metric_revision`;
- observation/reporting window and World/cohort identity;
- denominator/exposure definition identity and sample/exposure size;
- material ruleset/content/world-policy/server-build revisions or an explicit comparable-revision grouping;
- source completeness and schema-compatibility state;
- duplicate/order/late-event reconciliation state where material;
- sampling/loss/coverage state;
- privacy suppression/redaction state;
- source checkpoint/as-of/recomputation identity sufficient to distinguish reconciled/stale results;
- comparison baseline/method revision for regression or comparative panels.

Presentation rules:

1. `PARTIAL`, `UNKNOWN`, `PARTIAL_UNSUPPORTED`, unresolved reconciliation, known collection loss, incompatible/mixed revisions, or privacy suppression MUST produce a visible warning/state. A visualization MUST NOT hide those conditions behind a normal-looking number or green status.
2. Missing/unknown/suppressed data MUST NOT be rendered or exported as semantic zero.
3. Minimum-sample/exposure warnings are mandatory for metrics whose interpretation is sample-sensitive. Exact minimums are metric/privacy/product-policy owned and versioned; this contract does not impose one universal numeric threshold.
4. If a minimum-sample or privacy rule suppresses a value, the dashboard MUST show that the result is suppressed/insufficient rather than silently omit it in a way that can be read as zero/no activity.
5. A dashboard MAY summarize dimensions for usability, but the underlying decision-grade evidence packet and material semantic revisions remain retrievable. Presentation aggregation MUST NOT pool incompatible revisions/cohorts forbidden by the metric contract.
6. A visual trend/regression flag is not itself a balance or deployment decision. Dashboard colors, alert icons and thresholds have no gameplay or release authority.

A material regression evaluation MUST retain a named, versioned acceptance record separate from the visualization. The record binds the metric/baseline/method revisions, exact compared windows/cohorts, sample/exposure, evidence-quality state, checkpoint/as-of identity, known confounders and the responsible human/product/release owner disposition.

Allowed analytical regression dispositions are:

- `REGRESSION_NOT_EVALUATED` — no owner disposition yet;
- `REGRESSION_REVIEW_REQUIRED` — signal is decision-relevant and requires owner review;
- `NO_MATERIAL_REGRESSION_SUPPORTED` — evidence supports no material regression under the named method/threshold and declared scope;
- `KNOWN_CHANGE_ACCEPTED_BY_OWNER` — a measured change is acknowledged/accepted by the appropriate owner for its declared purpose;
- `REGRESSION_EVIDENCE_INSUFFICIENT` — quality/sample/comparability/reconciliation state is insufficient for a valid acceptance decision.

`NO_MATERIAL_REGRESSION_SUPPORTED` is a **fail-closed disposition**. It MUST NOT be recorded unless every applicable acceptance precondition below is affirmatively satisfied for the exact compared evidence set:

1. **quality/completeness** — required source completeness and schema compatibility meet the metric/decision contract; `PARTIAL`, `UNKNOWN` or `PARTIAL_UNSUPPORTED` evidence cannot support the disposition, and mixed supported revisions are permitted only when the named comparison method explicitly stratifies or proves them comparable;
2. **sample/exposure** — every applicable versioned minimum-sample/exposure rule is satisfied and the evidence needed for the decision is not suppressed; when a sample-sensitive acceptance requires a threshold but no accepted threshold exists, the precondition is not satisfied;
3. **comparability** — baseline and current metric/cohort/denominator plus all material ruleset, content/World Bundle, world-policy, server-build and sampling/collection revisions are compatible under the named method or are explicitly stratified so the acceptance does not pool incompatible semantics;
4. **reconciliation/finality** — duplicate/order/late-event reconciliation, source checkpoint/as-of state and required backfill/recompute state are sufficiently terminal for the declared acceptance window; unresolved reconciliation or a known material source gap fails the precondition;
5. **privacy/suppression sufficiency** — privacy suppression/redaction does not remove evidence required to support the declared no-regression conclusion;
6. **method/provenance** — the exact baseline, method/threshold revision, compared windows/cohorts, checkpoint/as-of identity and material confounders are bound in the acceptance record.

If a material regression evaluation is attempted and **any applicable precondition above is not affirmatively satisfied**, the analytical disposition MUST be `REGRESSION_EVIDENCE_INSUFFICIENT`; a dashboard warning, visual caveat or local analyst note is not an alternative to that disposition. `REGRESSION_NOT_EVALUATED` is reserved for the state before a material evaluation/disposition is attempted. A green `NO_MATERIAL_REGRESSION_SUPPORTED` result MUST therefore be impossible while the same evidence is sample-insufficient, materially partial/unsupported, semantically incomparable, materially suppressed or reconciliation-pending.

These are analytical/release-evidence dispositions only. `KNOWN_CHANGE_ACCEPTED_BY_OWNER` does not authorize a code/content deploy, gameplay mutation or waiver of another gate. Numeric regression thresholds, confidence methodology and release policy remain versioned owner decisions rather than universal ANL-02 constants.

## 12. SIM integration

ANL-02 may consume SIM-DETERMINISM evidence to reproduce behavior, compare exact supported-target outcomes and locate first deterministic divergence.

It may not:

- substitute best-effort event order for SIM authoritative order;
- invent missing authoritative state;
- replay live commands into production;
- repair/mutate state from replay results;
- equate deterministic difference with product undesirability.

## 13. Privacy and access

1. Ordinary balance/world projections prefer `INTERNAL_NON_PERSONAL` or `PSEUDONYMOUS_ANALYTICS`.
2. Raw AccountId/CharacterId is not admitted to ordinary analyst datasets merely for convenience.
3. `AnalyticsActorId` follows ANL-01 domain/epoch rules; cross-epoch correlation requires separately authorized/audited mapping.
4. Fine geography, exact time, rare cohort or party composition is treated as potential re-identification risk.
5. Presentation/export applies purpose-specific aggregation/suppression before disclosure; exact thresholds remain privacy-policy owned.
6. Balance/world analyst roles do not receive privileged pseudonym-to-operational-identity mapping by default.
7. Raw, intermediate, aggregate and exported datasets each bind finite accepted retention profiles.
8. Privacy classification may be raised but not silently downgraded.

## 14. Resource and execution bounds

ANL-02 inherits all applicable ANL-01 registry limits, including bounded query pages/results, replay windows and evidence packages.

Any implementation must additionally register hard bounds before acceptance for externally controlled:

- time-range/window count;
- group-by/result cardinality;
- cohort expansion;
- cross-source join fan-out;
- geography resolution/result cells;
- backfill/recompute work units;
- concurrent jobs/intermediate memory;
- open session/hunt grouping state and late-reconciliation work;
- dashboard query/result/drill-down scope where user-controlled.

Large work is paged/partitioned/resumable. Analytical storage/dashboard failure does not block authoritative gameplay. ANL-02 processing may lag or become unavailable but never acquires gameplay mutation authority.

## 15. Failure semantics

- Best-effort loss/sampling -> explicit quality degradation; no completeness claim; dashboard warns when displayed.
- Durable source gap/conflict -> affected durable-derived observation stops/marks incomplete; no fabricated state.
- Duplicate event -> one derived effect per identical EventId.
- Replayed event -> same event-time/grouping semantics and at most one derived effect under ANL-01 idempotency.
- Out-of-order related events -> bounded reconcile/defer or incomplete result; no arrival-order boundary invention.
- Late event -> original event-time reporting window; report/checkpoint provenance records reconciliation state.
- Missing explicit session/hunt grouping evidence -> session/hunt result partial/unavailable; no heuristic stitch.
- Unsupported schema -> ANL-01 class-specific behavior; no reinterpretation.
- Missing denominator/revision -> observation invalid/unknown for that use.
- Insufficient sample/exposure for an applicable metric rule -> visible warning/suppression as presentation state; if a material regression evaluation is attempted, the disposition MUST be `REGRESSION_EVIDENCE_INSUFFICIENT` and MUST NOT remain/become `NO_MATERIAL_REGRESSION_SUPPORTED`.
- Any failed quality/completeness, comparability, reconciliation/finality or required privacy-sufficiency precondition for a material regression evaluation -> `REGRESSION_EVIDENCE_INSUFFICIENT`; warning-only green acceptance is forbidden.
- Privacy/retention/access policy absent -> collection/projection/disclosure fails closed.
- Dashboard/presentation dependency unavailable -> evidence remains read-only; gameplay unchanged and no fabricated green/zero state.
- Analytical dependency unavailable -> gameplay unchanged.
- SIM provenance incomplete -> no `REPLAY_CORROBORATED` claim.

## 16. Required producer boundary

This contract does not register gameplay event IDs, payload schemas or a new gameplay/session identifier.

A concrete metric becomes implementable only after every required producer owner has registered typed payloads compatible with ANL-01, including required attribution, semantic scope, session/hunt grouping evidence where the metric requires it, canonical occurrence-time semantics for calendar reporting, and revision context.

`GAME_EVENT_FOUNDATION_REGISTRY.json.event_types=[]` at the trusted base is therefore a known dependency, not permission for ANL-02 to mint foreign event semantics.

## 17. Required future evidence

Implementation acceptance requires, proportionally to each metric family:

- versioned metric-definition fixtures;
- deterministic session/hunt grouping fixtures for explicit start/continue/end, reconnect continuity/non-continuity, session-generation and Channel/Instance transitions, missing terminal evidence and missing grouping identity;
- UTC half-open day-boundary fixtures for before/exactly-at/after midnight and hunts spanning midnight;
- duplicate/out-of-order/late/replayed event tests proving original grouping/event-time assignment and at-most-once derived contribution;
- late reconciliation/recomputation provenance tests;
- known best-effort drop/sampling propagation into quality metadata;
- mixed-revision rejection/stratification tests, including revision changes inside one producer grouping;
- denominator/cohort edge cases including empty/partial windows;
- dashboard fixtures proving metric revision, sample/exposure, quality/completeness/schema/reconciliation/suppression/as-of state cannot be silently hidden for material panels;
- minimum-sample warning/suppression tests proving insufficient or suppressed data is never rendered as zero/normal confidence;
- regression-acceptance fixtures proving compatible baseline/method/revisions and evidence quality are retained and insufficient evidence cannot become a green acceptance;
- negative regression-disposition fixtures proving each failed acceptance precondition (partial/unknown/unsupported quality, insufficient sample/exposure, incompatible unstratified revisions, unresolved reconciliation/material gap, required-evidence privacy suppression or unbound method/provenance) forces `REGRESSION_EVIDENCE_INSUFFICIENT` once evaluation is attempted;
- replay-recompute determinism for analytical transforms where applicable;
- privacy raw-ID rejection and cross-epoch mapping separation;
- geography suppression/redaction tests;
- query/result/evidence-package boundary tests;
- proof no analytical/dashboard path can mutate gameplay or synchronously block authoritative execution.

## 18. DECISIONS_NOT_TAKEN

This candidate does not select or authorize:

- concrete producer event IDs/payload schemas or concrete session/hunt identifier representation;
- database/warehouse/lake/broker/dashboard vendor or visualization framework;
- physical schemas/indexes/partitions;
- collector/service topology;
- exact KPI/balance/regression thresholds or statistical significance numbers;
- universal minimum-sample size or dashboard color/visual design;
- sampling percentages or numeric late-data/finalization horizons;
- inactivity timeout as implicit hunt/session semantic boundary;
- exact retention durations or anonymity/suppression thresholds;
- gameplay/content/reward/formula policy;
- automatic balancing/LiveOps control;
- runtime/client/Platform/production implementation.

## 19. CROSS_DOMAIN_FINDINGS

- `ANL02-XD-01` (`P1`, report only): concrete gameplay analytical event families, including producer-owned session/hunt grouping evidence required by concrete metrics, are not yet registered; owning gameplay/content domains plus ANL-01 registry integration must supply them before concrete coverage claims.
- `ANL02-XD-02` (`P2`, report only): ability/AI/interaction/quest/event attribution semantics remain dependent on their owning gameplay gates and cannot be invented by analytics.

Full evidence is recorded in `ANL-02_GAMEPLAY_BALANCE_WORLD_ANALYTICS_ANALYSIS.md`.

## 20. Acceptance boundary

This worker artifact is a nonbinding candidate. It may become canonical only through Architecture Coordinator audit/acceptance/merge under repository governance.

Even after architectural acceptance:

```text
DecisionStatus: ACCEPTED only if coordinator promotes it
ImplementationStatus: NOT_STARTED until separately proven
Runtime/production authority: NONE
```

A canonical dashboard implementation must preserve the evidence/warning/regression-acceptance semantics above, but architecture acceptance does not select a dashboard technology or authorize any gameplay/release mutation.

`MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY`