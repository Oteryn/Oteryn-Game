# ANL-03 — Economy, Integrity and Security Analytics Contract Candidate

- Date: 2026-08-15
- Gate: `ANL-03`
- Issue: #264
- DecisionStatus: `CANDIDATE`
- DeliveryStatus: `IN_REVIEW`
- ImplementationStatus: `NOT_STARTED`
- Status on worker branch: **CANDIDATE / NONBINDING**
- Canonical semantic effect: only after Architecture Coordinator acceptance/merge under repository governance
- Runtime/client/Platform/PostgreSQL/production/enforcement authority: **NONE**
- Merge authority: `ARCHITECTURE_COORDINATOR_ONLY`
- Analysis source: `ANL-03_ECONOMY_INTEGRITY_SECURITY_ANALYTICS_ANALYSIS.md`

## 1. Purpose

Freeze the minimum read-only consumer contract for economy health, item/currency integrity, security anomaly detection and auditable human evidence handoff.

ANL-03 owns:

- integrity/economy analytical classifications;
- normative deterministic invariant-evaluation catalogue over accepted authoritative evidence;
- detector definition/version semantics;
- signal evidence requirements;
- deterministic-versus-statistical conclusion boundaries;
- false-positive controls;
- auditable signal/case evidence lifecycle through human disposition/referral;
- privacy/access/retention/resource requirements for these consumers.

ANL-03 does not own authoritative prevention, gameplay/database mutation, sanctions, rollback, GM policy, concrete producer event IDs, detector technology or production rollout.

## 2. Non-authority invariant

```text
ANL-03 detector/signal/case
= read-only evidence + triage input
!= authority to mutate gameplay/database/account state
!= sanction
!= rollback/confiscation
!= automatic economy/balance control
```

No detector score, anomaly, model output, graph feature, invariant observation, signal disposition or case disposition grants mutation/enforcement authority by itself.

## 3. Upstream authority

A conforming ANL-03 consumer preserves:

- ADR-0006 observational/investigative and human-review boundary;
- ANL-01 immutable event/schema/durability/privacy/retention/order/idempotency rules;
- DUR-03 authoritative item/currency/value conservation, transaction and prevention semantics;
- GAME-CHANNEL World economy and reward/source multiplicity semantics;
- SIM-DETERMINISM read-only replay/state provenance;
- FND session/runtime/admission/fencing semantics;
- owning market/trade/mail/depot/reward/gameplay semantics where applicable.

ANL-03 never replaces these owners with inferred analytics semantics.

## 4. Input-class rules

### Durable integrity input

Claims about complete item/currency/value provenance, conservation, transaction outcome or reward commit require accepted `DURABLE_AUDIT` evidence and source completeness appropriate to the declared scope.

Best-effort telemetry may enrich context but cannot be sole conservation/security completeness proof.

### Durable security observation

A non-mutating security observation may be consumed as durable audit only when its producer-owned ANL-01 event type explicitly permits that classification. Missing events are not invented by ANL-03.

### Best-effort behavioral input

Behavioral/statistical detectors may consume best-effort telemetry if their detector definition declares sampling/loss tolerance and propagates data quality. Their output remains hypothesis-class unless a separate accepted invariant proves the behavior impossible.

### Operational observability

Low-cardinality process/service health may contextualize pipeline lag/failure. Player/item/session identifiers remain forbidden as ordinary Prometheus labels.

## 5. Analytical classification

Every material ANL-03 output is classified as one of:

- `AUTHORITATIVE_INVARIANT_EVIDENCE` — evaluates an already-accepted FND/DUR/gameplay invariant from appropriate evidence;
- `AUDIT_PIPELINE_INTEGRITY` — identifies event/checkpoint/schema/hash/completeness problems in the evidence path;
- `STATISTICAL_SECURITY_ANOMALY` — hypothesis from rule/model/behavior/graph analysis;
- `ECONOMY_HEALTH_OBSERVATION` — non-guilt economy source/sink/supply/concentration/liquidity/scarcity evidence;
- `ENGINE_DEFECT_HYPOTHESIS` — evidence suggests a product defect, optionally replay-corroborated.

A statistical anomaly is never relabeled as deterministic proof merely because its score is high or because a human reviewer finds it credible. Human corroboration may produce the non-sanctioning `SUPPORTED_SECURITY_FINDING` disposition defined below while the underlying analytical class remains statistical/rule/model/graph evidence unless a separately accepted deterministic invariant is proven.

## 6. DUR-03 provenance projection

When producer event families exist, ANL-03 may maintain a read-only derived provenance/ledger graph from typed evidence including as applicable:

- ItemInstanceId lifecycle;
- source/destination semantic location/custody;
- quantity/value before/after required by the owning operation;
- DUR-03 conservation class: `TRANSFER`, `SPLIT_MERGE_QUANTITY`, `STATE_MUTATION`, `MINT`, `BURN`, `TRANSFORM`, `CONVERSION`;
- typed source/sink/occurrence/cause;
- TransactionId/OperationId/CommandRef/CausationRef;
- TransactionEventRef set/completeness;
- world/channel/instance and ownership-generation evidence;
- item/domain/ruleset/content/build revisions;
- reward/source occurrence and multiplicity/eligibility context.

This projection is evidence only. It is not a second authoritative item-location/value store and cannot repair production state.

### 6.1 Normative deterministic invariant catalogue

A conforming ANL-03 integrity implementation MUST evaluate the following invariant classes whenever applicable producer-owned event families, authoritative semantics and completeness evidence exist for the declared scope. It MUST NOT implement an arbitrary subset and claim complete ANL-03 integrity coverage.

1. **Single authoritative location** — one live `ItemInstanceId` cannot occupy two authoritative locations at the same authoritative state/revision.
2. **Duplicate/idempotency safety** — duplicate or replayed delivery/evaluation of one accepted event, idempotency key, operation, transaction or source occurrence cannot duplicate item, currency, quantity, value or derived durable effect.
3. **Split conservation and identity safety** — split conserves total quantity/value under owning DUR-03 semantics and resulting live identities are non-conflicting.
4. **Merge conservation and identity retirement** — merge conserves total quantity/value and retires/retains source/result identities exactly according to owning DUR-03 rule.
5. **Authorized creation/credit** — creation/credit requires accepted authorized source/cause/source occurrence; unexplained mint/credit is an integrity finding only when required evidence is complete.
6. **Authorized destruction/debit** — destruction/debit requires accepted authorized sink/cause; unexplained burn/debit is an integrity finding only when required evidence is complete.
7. **Reward exactly-once** — one accepted reward/source occurrence cannot commit durable value more than once for owning idempotency/source-occurrence identity and multiplicity/eligibility semantics.
8. **Ownership-generation fencing** — stale gameplay session, connection/session generation or stale ownership generation cannot transfer/acquire authoritative ownership.
9. **Transaction/outbox agreement** — transaction commit/abort state and required durable outbox/audit evidence cannot silently disagree; ANL-03 never guesses or repairs authoritative outcome.
10. **Retry/crash/rollback no-unexplained-value** — retry, timeout, failure, recovery, replay or rollback cannot create unexplained value or duplicate one accepted mutation/source occurrence.
11. **Authoritative view reconciliation** — inventory, ground, container, depot, trade, market and mail projections/views reconcile to authoritative owner/location model and accepted transaction state; disagreement is evidence, not analytics repair.

Normative evaluation rules:

- DUR-03 and relevant owning runtime/domain contract remain authoritative prevention/conservation/mutation owners. ANL-03 is read-only evidence/detection only.
- ANL-03 MUST consume exact applicable invariant semantics/revisions and MUST NOT invent market/reward/inventory/transaction business rules.
- Missing/unregistered producer evidence, unsupported semantic revision or incomplete durable scope is `INCONCLUSIVE_INSUFFICIENT_EVIDENCE` or `DATA_QUALITY_OR_PIPELINE_FAILURE`; absence of evidence is neither invariant satisfaction nor guilt.
- `INVARIANT_VIOLATION_SUPPORTED` requires completeness/reproducibility preconditions in section 10.
- Duplicate/out-of-order/replayed evidence is handled under ANL-01 identity/order/idempotency before deterministic conclusion.
- No catalogue result authorizes automatic ban, confiscation, rollback, DB mutation, value repair, economy tuning, balance change or deployment.

## 7. Economy interpretation

1. `WorldId` is the default economy boundary; ChannelId is contextual simulation scope, not separate economy namespace.
2. Channel-local simulation does not imply repeatable durable reward/source eligibility.
3. Reference analysis measures Oteryn actual world history under Reference mechanical source/sink rules; Global historical market prices/supply are not parity constants.
4. Supply/concentration/liquidity/scarcity observations include world age, population/exposure and semantic revision context.
5. “Unexplained value” requires proven durable completeness for declared scope; otherwise result is partial/inconclusive.
6. Durable value truth outranks best-effort activity telemetry where they disagree.
7. Market valuation requires an accepted market semantic source; external scrape/value guesses do not become authority.
8. Economy-health evidence may recommend owner review but cannot change rates, sinks, drops, fees or live policy.

## 8. Detector-definition identity

Every material detector has stable logical `detector_id` and immutable positive `detector_revision`.

A detector definition declares at least:

1. purpose/threat/defect hypothesis;
2. detector class (`deterministic-rule`, `statistical-rule`, `model`, `graph`, `reconciliation` or later versioned class);
3. required input event/projection families and supported schema revisions;
4. required durability/completeness/privacy classes;
5. applicable World/ruleset/content/build/SIM revisions;
6. feature definitions/revisions;
7. observation-window/time basis;
8. subject scope;
9. rule/model/algorithm artifact identity and digest;
10. threshold/configuration revision where applicable;
11. output category/severity/confidence semantics;
12. source-quality prerequisites;
13. known false-positive modes/exclusions;
14. resource bounds/degradation behavior;
15. evaluation corpus/revision requirements;
16. privacy/retention/access policy.

Rule/feature/model/threshold semantic change receives a new explicit revision. Historical outputs retain exact detector/config/artifact revision used.

## 9. Signal contract

Every emitted detector signal has a stable non-reused reference and records at least:

```text
signal reference
detector_id + detector_revision
configuration/model/artifact digest
run/evaluation reference
observation window
subject scope/reference under correct privacy class
analytical classification + signal category
impact severity / triage priority
score/confidence semantics when applicable
invariant/rule/hypothesis evaluated
source evidence/checkpoint references
source completeness/quality vector
relevant World/Channel/Instance + semantic revisions
known exclusions/false-positive caveats
creation time
terminal/current disposition reference(s) when disposition exists
case linkage when a case is opened
```

Severity is triage/impact context, not guilt. Confidence/score expresses detector semantics, not enforcement authorization.

Duplicate re-evaluation may coalesce operational noise only if original evidence/signal lineage remains addressable and historical outputs/dispositions are not rewritten.

## 10. Evidence conclusion rules

### `INVARIANT_VIOLATION_SUPPORTED`

May be used only when:

- invariant and relevant revisions are accepted;
- required durable source/checkpoint/event-set evidence is proven complete for declared scope;
- EventId/schema/hash/order/TransactionEventRef prerequisites pass;
- detector/projection logic is reproducible/versioned;
- no known evidence-pipeline failure explains mismatch.

Otherwise result is data-quality/inconclusive, not deterministic subject attribution.

### `ANOMALY_HYPOTHESIS`

Statistical/rule/model/graph anomaly remains a hypothesis pending human review and corroborating evidence. Probability/score never converts directly into sanction authority.

Human review may conclude that available affirmative evidence sufficiently corroborates the security concern for investigation/referral/monitoring. That conclusion is recorded as `SUPPORTED_SECURITY_FINDING`; it does **not** reclassify statistical evidence as deterministic authoritative proof and does not itself authorize sanction.

### `REPLAY_CORROBORATED_DEFECT`

May be used when SIM-DETERMINISM exact provenance reproduces a product divergence/invariant failure under relevant semantic revision set. Repair and player/account action remain foreign authority.

## 11. Signal/case evidence lifecycle

The ANL-03 analytical lifecycle has two explicit auditable branches after triage:

```text
SIGNAL_EMITTED
-> TRIAGED
   -> SIGNAL_DISPOSITION_RECORDED -> [ROUTING_REFERRAL_RECORDED] -> SIGNAL_CLOSED   # no case required
   OR
   -> CASE_OPENED
      -> ASSIGNED / REASSIGNED (when assignment exists)
      -> EVIDENCE_ASSEMBLED / UPDATED
      -> HUMAN_EVIDENTIARY_DISPOSITION
      -> [ROUTING_REFERRAL_RECORDED]
      -> CLOSED
      -> REOPENED (when later evidence/review requires it)
```

`CASE_OPENED` remains optional, but **no-case triage is not allowed to disappear into mutable status**. If triage ends without opening a case, the signal MUST receive an immutable terminal substantive analytical disposition before `SIGNAL_CLOSED`. If it is routed to another owner, the referral is a separate immutable routing action recorded **after** that substantive disposition. Later evidence may append a new triage/reopen/review generation; it MUST NOT overwrite the prior terminal record.

Regardless of concrete workflow/state vocabulary, every material signal/case lifecycle transition and reviewer/operator action MUST append an immutable audit record. This includes signal emission/triage disposition/close/reopen; case open; assignment/reassignment; evidence addition/removal/supersession; state transition; close; reopen; referral; and final disposition.

Each lifecycle audit record retains at least:

```text
signal identity
case identity when a case exists
correlation identity
stable action/transition reference or ordered occurrence
reviewer/operator actor identity, or pseudonymous privileged identity under privacy model
actor role/capability used for action
timestamp plus sufficient ordering information
previous state when a state exists
new state and/or action performed
reason/rationale category or explanation where applicable
linked signal/evidence references relevant to action
detector/rule/model/config revision where relevant
substantive evidentiary disposition reference for any referral action
foreign target owner/type when a referral action exists
```

Required properties:

- original signal content/revision/evidence lineage remains immutable;
- every triaged signal has reconstructable outcome even if no case is opened;
- no-case false-positive, duplicate, inconclusive, data-quality or supported findings are represented by immutable signal-level disposition/close actions rather than absence of a case record;
- referral is never a substitute for evidentiary classification: a direct no-case referral and a case referral both require a preceding substantive disposition for the same review generation;
- one case may group multiple signals while preserving every source reference and each source signal's prior disposition/history;
- evidence additions, annotations, removals and supersessions are immutable lifecycle actions rather than history rewrites;
- signal triage before case creation retains correlation identity and links to the case if later opened;
- privileged identity resolution is separately purpose-authorized and audited;
- human disposition records reviewer role/identity, time, rationale and evidence refs through same immutable history;
- false-positive decisions and reviewer actions remain reconstructable/auditable within retention policy;
- a mutable latest-status projection may exist only as derived state and MUST NOT replace/truncate/rewrite lifecycle history;
- audit-history immutability does not authorize unlimited retention or override ANL-01/privacy deletion requirements;
- enforcement/remediation result is linked only as foreign-owner reference and is not executed by ANL-03.

Allowed **substantive evidentiary dispositions** at signal or case level, as applicable, are:

- `SUPPORTED_INTEGRITY_OR_DEFECT_FINDING` — accepted invariant or reproducible defect evidence supports the finding under section 10;
- `SUPPORTED_SECURITY_FINDING` — human-reviewed affirmative evidence corroborates a security concern such as bot/automation/protocol misuse sufficiently for analytical support/referral/monitoring, while remaining non-sanctioning and without pretending statistical/model evidence is deterministic invariant proof;
- `NOT_SUPPORTED_FALSE_POSITIVE`;
- `INCONCLUSIVE_INSUFFICIENT_EVIDENCE`;
- `DATA_QUALITY_OR_PIPELINE_FAILURE`;
- `DUPLICATE_OR_ALREADY_COVERED`.

`REFERRED_TO_SECURITY_GM_PRODUCT_OR_ENGINE_OWNER` is **not an evidentiary disposition**. It is an optional routing action only. It MUST NOT be the sole terminal analytical classification of a signal or case and MUST NOT be recorded until one of the substantive evidentiary dispositions above has been immutably recorded for the same review generation. This rule applies equally to no-case direct routing and case-based routing. If the reviewer cannot support a positive finding, the evidence still receives the truthful substantive outcome (`INCONCLUSIVE_INSUFFICIENT_EVIDENCE`, `DATA_QUALITY_OR_PIPELINE_FAILURE`, `NOT_SUPPORTED_FALSE_POSITIVE` or `DUPLICATE_OR_ALREADY_COVERED` as applicable) before any referral.

A referral record MUST reference the substantive disposition that caused/qualified the routing, the target foreign owner, actor/capability, rationale, evidence refs and ordering/time. Referral does not erase, upgrade or downgrade the evidentiary conclusion and does not imply the target owner accepted the referral, finding or enforcement action.

These are analytical dispositions/routing actions, not sanctions. A `SUPPORTED_SECURITY_FINDING` does not authorize ban/mute/kick/confiscation/rollback/account action, does not increase evidence quality beyond the recorded source quality, and does not waive independent enforcement-owner validation.

## 12. False-positive safety

To satisfy `FS-DETECTOR-FALSE-POSITIVE` semantically:

1. detector output creates no automatic sanction/mutation;
2. detector/rule/model/config revision retained with every signal;
3. source evidence/checkpoints traceable within policy;
4. human review/disposition and every material signal/case lifecycle/reviewer action reconstructable from immutable audit history;
5. no-case triage outcome always has an explicit audited terminal substantive signal disposition rather than disappearing because no case exists;
6. any referral is an auditable routing action attached to a substantive disposition and can never replace that disposition;
7. threshold/model changes do not rewrite historical outputs;
8. suppression/allow-list mechanism is separately versioned/scoped/audited and not hidden authority bypass;
9. false-positive outcomes may inform detector quality without deleting historical errors;
10. incomplete data yields inconclusive/data-quality disposition where material;
11. pipeline failures are separated from subject/gameplay findings;
12. corroborated security findings remain non-sanctioning evidence and retain statistical/model caveats where applicable.

Exact quality/precision/recall/calibration thresholds remain implementation/security-product decisions.

## 13. Security analytical scope

Subject to producer-owned typed event families, ANL-03 may analyze:

- command replay/duplicate/stale-session/stale-ownership attempts;
- admission/reconnect/channel-switch anomalies;
- cooldown/sequencing/action-rate anomalies;
- automation/bot-like patterns;
- repeated reward/source loops;
- item/currency provenance anomalies;
- unusual transfer fan-in/fan-out/cycles/concentration;
- market/trade/mail/depot inconsistencies once owning contracts exist;
- protocol capability/downgrade/revision anomaly evidence;
- engine defects exposed by invariant/replay mismatch.

No generalized device fingerprinting, invasive client surveillance, credential capture or secret retention is authorized. New device/network identity collection requires separate named security/privacy purpose and owning contract.

## 14. Privacy/access/retention

1. Broad detection uses `PSEUDONYMOUS_ANALYTICS` or non-personal features where feasible.
2. Restricted item/transaction/session/player evidence uses `RESTRICTED_PLAYER_LINKED` or `SECURITY_SENSITIVE` as required.
3. AnalyticsActorId mapping is separately protected; detector access is not implicit.
4. Privileged identity resolution is case/signal-purpose-authorized and access-audited.
5. Fine location/time and later device/network context are re-identification/security-sensitive where applicable.
6. Raw evidence, detector features, signals, signal dispositions, case evidence, identity-resolution logs and exports have separate finite retention profiles.
7. Legal hold is explicit audited exception, never ordinary unlimited retention.
8. Privacy classification can be raised but never silently downgraded.

### Optional client diagnostics are non-adverse

For every detector, feature, signal, triage path, case and reviewer workflow that may consume optional client-originated evidence — client diagnostics/crash reports, OS capsule, Launcher telemetry, Guardian telemetry or crash/network forensic package — the following invariant applies:

- diagnostics opt-out is not suspicious;
- absence, opt-out, upload failure or unavailability is not adverse evidence and MUST NOT be represented as guilt/concealment feature;
- disabling/withholding optional diagnostics MUST NOT increase abuse/suspicion/risk score, detector confidence, signal severity, triage/review/enforcement priority, escalation priority or adverse disposition;
- missing optional diagnostics MUST NOT by itself open/escalate/refer/worsen a signal or case;
- availability/quality may justify lower evidentiary certainty or inconclusive/data-quality result; it MUST NOT become guilt evidence;
- enabling diagnostics creates neither innocence presumption nor automatic risk reduction;
- affirmative content actually present may corroborate evidence according to detector contract; mere enablement cannot;
- server-generated evidence MUST remain sufficient for investigation without optional client diagnostics.

This applies equally to native client, OS allowlist capsule, Launcher/Guardian extension point or forensic package. ANL-03 MUST NOT implement “missing diagnostics = suspicious”.

## 15. Read-only credential boundary

Future ANL-03 components use least-privilege read-only credentials/views and must not possess:

- gameplay mutation credentials;
- production DB write authority;
- sanction/account-action authority;
- rollback/confiscation/value-repair authority;
- deployment authority.

Enforcement/remediation requires referral to a foreign owner that independently validates authority and evidence.

## 16. Resource bounds

ANL-03 inherits applicable ANL-01 event/replay/query/result/evidence-package limits.

Before implementation acceptance, externally controlled/high-growth detector work registers hard limits for applicable:

- observation-window duration/event count;
- per-subject retained state;
- feature cardinality/bytes;
- graph vertices/edges/depth/fan-out;
- join expansion;
- signals per run/window/subject;
- signal lifecycle/audit actions per retained signal;
- case evidence references/attachments;
- concurrent detector/backfill/reconciliation jobs;
- model/rule artifact size;
- investigator query/export scope;
- resumable provenance work units.

Exact numbers require implementation/PERF/OPS evidence and registry ownership. Large jobs are partitioned/resumable. Detector lag/unavailability never blocks gameplay solely to preserve analytics freshness; committed durable audit remains subject to ANL-01/DUR no-silent-drop rules.

## 17. Failure semantics

- Best-effort feature loss -> input partial; no completeness-dependent finding.
- Optional client diagnostics absent/opted-out/unavailable -> availability/quality dimension only; no adverse inference.
- Durable audit/event-set/checkpoint gap -> completeness-dependent detector stops/inconclusive; pipeline incident may be emitted.
- EventId conflict -> evidence-integrity incident; no overwrite.
- Unsupported durable schema -> quarantine/reject per ANL-01.
- Duplicate event -> one projection/evaluation effect per identical EventId.
- Out-of-order events -> bounded defer/reconcile; no fabricated provenance.
- Detector/model unavailable -> detector unavailable; gameplay unchanged.
- Model/threshold revision change -> new versioned evaluation; old signals/dispositions preserved.
- Identity mapping unavailable -> no unauthorized fallback.
- Missing privacy/retention/access policy -> no collection/projection/disclosure.
- Oversized query/evidence -> page/partition/reject within registry bounds.
- Triaged signal with no case -> explicit audited substantive signal-level disposition and close/review-generation record; never silent disappearance.
- Referral requested with no substantive disposition -> referral is rejected/deferred until a truthful substantive evidentiary disposition is recorded; naked referral is not a terminal outcome.
- Suspected violation + incomplete evidence -> `INCONCLUSIVE_INSUFFICIENT_EVIDENCE`, never automatic sanction.
- Human-corroborated statistical security concern -> may become `SUPPORTED_SECURITY_FINDING`, but never deterministic invariant proof or enforcement authority solely by that label.

## 18. Failure-scenario status

- `FS-AUDIT-OUTBOX-BACKLOG`: semantic `PASS` for consumer behavior; physical audit durability remains ANL-01/DUR.
- `FS-EVENT-DUPLICATE-DELIVERY`: semantic `PASS`.
- `FS-EVENT-OUT-OF-ORDER`: semantic `PASS`.
- `FS-AUDIT-MUTATION-MISMATCH`: semantic `PASS` as integrity/data-quality evidence; prevention remains DUR-02/03.
- `FS-ANALYTICS-PRIVACY-POLICY`: semantic `PASS`, including non-adverse diagnostics semantics.
- `FS-DETECTOR-FALSE-POSITIVE`: semantic `PASS` through no-auto-sanction + versioned evidence + immutable signal/case lifecycle + terminal no-case substantive dispositions + referral-as-routing + human disposition.
- `FS-INVESTIGATION-MUTATION-ATTEMPT`: `DEFERRED_BY_ACCEPTED_GATE` for executable proof to ANL-04/implementation; ANL-03 requires read-only credentials.

Architecture status does not imply runtime proof.

## 19. Required producer boundary

This candidate does not register concrete DUR/FND/gameplay event type IDs or payload schemas.

Concrete detector/ledger coverage is implementable only after producer owners register required typed events under ANL-01 with sufficient provenance, revision, authority and privacy semantics.

Trusted-base empty `GAME_EVENT_FOUNDATION_REGISTRY.json.event_types` is explicit dependency, not permission for ANL-03 to define foreign producer schemas.

## 20. Required future evidence

Implementation acceptance requires, proportionally to each detector family:

- deterministic projection fixtures for every applicable catalogue invariant: single authoritative location, duplicate/idempotency, split/merge conservation/identity, authorized source/sink, reward exactly-once, stale-generation fencing, transaction/outbox agreement, retry/crash/rollback no-unexplained-value and authoritative view reconciliation;
- fixtures showing incomplete/unregistered evidence yields inconclusive/data-quality rather than false pass/guilt;
- TransactionEventRef complete/gap/duplicate/conflict cases;
- EventId duplicate/conflict and unsupported-schema cases;
- checkpoint loss/recovery and durable source-completeness tests;
- stable detector/model/config/artifact revision reproduction;
- known false-positive corpus and human-disposition workflow evidence;
- no-case triage fixtures proving false-positive/duplicate/inconclusive/data-quality/supported-security outcomes receive immutable substantive signal-level disposition + close records and remain countable/reconstructable;
- referral fixtures proving both no-case and case routing are rejected/deferred without a preceding substantive evidentiary disposition, and proving accepted routing retains both the substantive disposition and separate target-owner referral action;
- supported-security fixtures proving a corroborated bot/automation/protocol-misuse signal can retain `STATISTICAL_SECURITY_ANOMALY` evidence class while receiving `SUPPORTED_SECURITY_FINDING` without becoming sanction/deterministic proof;
- immutable lifecycle-audit replay proving signal triage/disposition/close/reopen and case open/assignment/evidence/state/close/reopen/referral/disposition actions reconstructable with actor/capability/reason/evidence/revision linkage;
- optional client-diagnostics tests proving opt-out/absence cannot raise risk/confidence/review/enforcement priority or become guilt evidence;
- best-effort sampling/loss propagation tests;
- privacy raw-ID/pseudonym mapping/access-audit tests;
- graph/query/export/resource boundary tests;
- SIM replay-corroboration fixtures where claimed;
- proof detector/investigation credentials cannot mutate gameplay/DB or sanction accounts.

## 21. DECISIONS_NOT_TAKEN

This candidate does not select/authorize:

- concrete producer event IDs/payload schemas;
- database/warehouse/lake/broker/vendor topology;
- detector framework/language/model architecture;
- exact detector algorithms/thresholds;
- production detector rollout;
- sanctions/GM/moderation/account-action policy;
- automatic ban/mute/kick/confiscation/rollback/value repair;
- device fingerprinting/client surveillance/network identity collection;
- exact retention/legal durations;
- exact UI/case-management implementation for signal/case lifecycle;
- market/trade/mail/depot/reward business policy;
- economy target values/automatic tuning;
- Platform/runtime/client/DDL/production implementation;
- ANL-04 AI implementation.

## 22. CROSS_DOMAIN_FINDINGS

- `ANL03-XD-01` (`P1`, report only): concrete DUR-03 item/currency/value durable event families are not yet registered; complete provenance/invariant coverage cannot be implemented until producer owners register them.
- `ANL03-XD-02` (`P2`, report only): enforcement/GM/account-remediation authority is intentionally outside ANL-03 and requires separately owned accepted contract before production action.
- `ANL03-XD-03` (`P2`, report only): FND/session/admission/channel security producer event coverage is not registered; corresponding detectors cannot claim completeness.
- `ANL03-XD-04` (`P2`, report only): market/trade/mail/depot/reward fraud/business interpretation remains dependent on owning economy/gameplay contracts.

Full evidence is recorded in `ANL-03_ECONOMY_INTEGRITY_SECURITY_ANALYTICS_ANALYSIS.md`.

## 23. Acceptance boundary

This worker artifact is a nonbinding candidate. It may become canonical only through Architecture Coordinator audit/acceptance/merge under repository governance.

Even after architectural acceptance, implementation remains separately gated and no runtime, DDL, production or enforcement authority is granted. Every triaged signal must have an auditable terminal substantive outcome even without a case; referral is only routing after that outcome, and supported security findings remain non-sanctioning analytical evidence.

`MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY`