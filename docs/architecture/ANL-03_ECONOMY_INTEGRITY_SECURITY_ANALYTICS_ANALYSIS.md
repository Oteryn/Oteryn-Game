# ANL-03 — Economy, Integrity and Security Analytics Analysis

- Date: 2026-08-15
- Gate: `ANL-03`
- Issue: #264
- Delivery branch: `docs/arch-f-analytics-integrity`
- Trusted worker base: `main@088b46638ac014cd7928d6b0b75cee44902fe22c`
- DecisionStatus: `CANDIDATE`
- DeliveryStatus: `IN_REVIEW`
- ImplementationStatus: `NOT_STARTED`
- Runtime/client/Platform/PostgreSQL/production/enforcement authority: **NONE**
- Merge authority: `ARCHITECTURE_COORDINATOR_ONLY`

## 1. Purpose

ANL-03 refines ADR-0006 into a bounded consumer architecture for economy health, item/currency integrity, security anomaly detection and auditable human investigation handoff.

The central safety boundary is deliberately strict:

```text
authoritative prevention / fencing / conservation -> FND + DUR-03 + gameplay owners
analytical projection / detector / case evidence   -> ANL-03
read-only investigation / AI                       -> ANL-04
sanction / GM / account action / rollback          -> separate human-authorized owner
```

ANL-03 may discover evidence of a defect, exploit, suspicious pattern or economy-health problem. It may not itself ban, mute, delete, rollback, repair, confiscate, change rates, change content, deploy or mutate a production database/runtime.

## 2. Source classification

### PROVEN

- ADR-0006 requires durable economy/security audit for item/currency/reward/transaction evidence and states that alerts identify evidence requiring investigation rather than authorizing mutation.
- ANL-01 freezes immutable EventId/content semantics, `DURABLE_AUDIT`, TransactionEventRef completeness, at-least-once delivery, idempotent consumers, privacy/retention/access classes and read-only replay.
- ANL-01 explicitly defers `FS-DETECTOR-FALSE-POSITIVE` to ANL-03.
- DUR-03 owns single-location, item identity lifecycle, typed conservation classes, stable transaction/operation identity, fencing/idempotency, runtime↔durable handoff and restore/recovery integrity. It requires mandatory durable provenance/evidence where policy requires it.
- GAME-CHANNEL-01 makes one World one economy boundary and requires explicit durable source/reward multiplicity semantics rather than treating each Channel copy as a new eligibility source.
- SIM-DETERMINISM-01 makes deterministic replay/state hashes read-only evidence, preserves exact build/protocol/World Bundle provenance and prohibits replay from repairing live state.
- GAME-VISION-01 accepts conservation before tuning, measurable value provenance and economy-health evidence in world-age/population context.
- `GAME_EVENT_FOUNDATION_REGISTRY.json.event_types` is intentionally empty at the trusted base; concrete domain event families remain producer-owned.
- Accepted client crash/disconnect privacy baselines make client-originated diagnostics optional, define diagnostic opt-out as non-suspicious/non-adverse and keep server-generated evidence sufficient for investigation without optional client diagnostics.

### DERIVED

- ANL-03 can define detector/evidence/case semantics now, but concrete detector coverage cannot become production-valid until DUR/gameplay/FND owners register the necessary event families and evidence fields.
- A deterministic invariant violation and a statistical anomaly are different epistemic classes and must not share one undifferentiated “cheat score”.
- Missing durable evidence can be a data-quality/integrity incident; it is not automatically proof that a player caused the problem.
- Missing, opted-out or unavailable optional client diagnostics are evidence-availability/quality facts only; they must never become a suspiciousness feature or adverse subject inference.

## 3. Decision timing

### Must this be decided now?

**YES — detector/evidence/case semantics only.**

DUR-03 and future gameplay/security producers need to know which immutable transaction, lineage, authority, revision and source/sink facts downstream integrity consumers require. If detector semantics are invented after production collection, critical provenance can be permanently absent and false-positive handling can become an afterthought.

### Concrete downstream work blocked without this decision

- registration of DUR-03 item/currency/value audit payloads sufficient for provenance/invariant projections;
- security-event payload families for replay/stale-session/admission/channel-switch anomaly consumers;
- read-only economy/integrity projections;
- versioned detector implementations and evaluation corpora;
- human-review case/evidence tooling;
- later ANL-04 investigation/AI composition.

### What becomes expensive to change later?

- historical detector reproducibility if rule/model/config versions are not retained;
- provenance if item/value lineage is omitted at emission time;
- false-positive auditability if original signals are overwritten by later thresholds;
- privacy/legal controls if broad raw identity/device/location collection starts without purpose boundaries;
- distinction between pipeline-data defects and player/gameplay integrity defects.

### Evidence that may supersede this candidate

Measured incident response, detector precision/recall/calibration, false-positive cohorts, real economy topology, privacy/legal decisions and implementation performance may justify new detector classes, workflow states or resource limits. Historical detector/event/case revisions must remain interpretable after supersession.

## 4. Authority matrix

| Concern | Authoritative owner | ANL-03 authority |
|---|---|---|
| Session/admission/replay prevention | FND-02/FND-04 and security owners | Observe evidence only |
| Runtime fencing/order | FND-03 | Observe stale/invalid attempt evidence only |
| Item/currency/value prevention and conservation | DUR-03 | Independent projection/invariant monitoring only |
| Reward/source multiplicity | GAME-CHANNEL + reward/source owner | Verify/classify against accepted policy only |
| Event identity/audit/privacy | ANL-01 | Consume/preserve only |
| Deterministic replay/state provenance | SIM-DETERMINISM-01 | Corroborate defects only |
| Detector definition/version/output semantics | ANL-03 | Candidate owner |
| Case evidence/false-positive analytical workflow | ANL-03 | Candidate owner up to human disposition/referral |
| Read-only AI investigation | ANL-04 | ANL-03 supplies bounded evidence, no mutation |
| Sanctions/GM/account enforcement | Separate product/security/GM owner | `REPORT_ONLY` / referral only |
| Rollback/confiscation/repair/value mutation | DUR/gameplay/operations owners under separate authority | None |

## 5. Input classes

### 5.1 Durable transaction/value audit

Conservation, provenance and duplication/invariant analysis uses `DURABLE_AUDIT` evidence whose producer contract preserves DUR-03 semantics.

A best-effort gameplay stream may enrich context, but it must never be the sole evidence for:

- complete item/currency provenance;
- one-location/conservation proof;
- committed transaction-set completeness;
- whether a reward/value mutation committed.

### 5.2 Security-relevant non-mutating observations

ANL-01 permits a non-mutating security observation to be durable audit without transaction context when its event type declares `atomic_mutation_evidence=false`.

Future FND/gameplay/security owners may use that class for accepted security facts such as rejected stale authority, command replay detection or explicit authorization failures. ANL-03 cannot invent these producer facts from missing events.

### 5.3 Best-effort behavior telemetry

Statistical bot/automation/behavior detectors may consume best-effort telemetry when their contract explicitly tolerates gaps/sampling and propagates those limitations.

A behavioral anomaly score remains a hypothesis even with high confidence; it is not equivalent to a deterministic authoritative invariant violation.

### 5.4 Operational observability

Low-cardinality service health may contextualize detector gaps, outbox lag or incident timing. High-cardinality player/item/session IDs remain prohibited as ordinary Prometheus labels.

### 5.5 Optional client diagnostics are non-adverse evidence availability

If an ANL-03 detector, signal, case or investigator can consume optional client-originated evidence — including client diagnostics, an OS forensic capsule, Launcher telemetry, Guardian telemetry, or a crash/network forensic package — that source remains supplemental and subject to its owning privacy/consent contract.

The following invariant is mandatory:

- diagnostics opt-out is not suspicious;
- absence, opt-out, disabled collection, unavailability, delay, expiry or non-submission of optional diagnostics is not adverse evidence and must not be treated as guilt evidence;
- disabling or withholding optional diagnostics **MUST NOT** increase an abuse/risk score, signal severity, confidence, enforcement priority or reviewer/triage priority;
- optional-diagnostic availability/completeness/quality may be recorded only as an evidence availability/quality dimension and may constrain what conclusions can be supported; missing optional diagnostics can reduce usable evidence or yield an inconclusive result but can never strengthen an adverse inference;
- missingness/opt-out itself must not be transformed into a detector feature, anti-cheat signal, correlation bonus or suspiciousness indicator;
- server-generated/authoritative evidence must remain sufficient to investigate and triage the incident/case without optional client diagnostics.

Presence of an optional diagnostic package may corroborate or refute a hypothesis according to its producer-owned provenance and trust class. Its absence has no converse adverse meaning. ANL-03 does not define, expand or modify any producer schema by naming these optional evidence classes.

## 6. Integrity/economy analytical taxonomy

ANL-03 separates at least five classes.

### 6.1 `AUTHORITATIVE_INVARIANT_EVIDENCE`

Evidence evaluates an invariant already owned by FND/DUR/gameplay contracts, such as:

- one live ItemInstanceId in one authoritative location;
- split/merge/transfer conservation;
- typed authorized mint/burn/transform/conversion lineage;
- stable TransactionId/idempotency outcome;
- reward occurrence not committed twice;
- stale authority unable to commit value;
- transaction/audit completeness consistency.

ANL-03 does not create the invariant. It projects/validates evidence against the accepted invariant revision.

### 6.2 `AUDIT_PIPELINE_INTEGRITY`

Evidence identifies problems in the evidence system itself, including:

- same EventId with conflicting immutable content;
- TransactionEventRef gap/duplicate/inconsistent count;
- unsupported durable schema;
- publication/checkpoint gap;
- outbox/backlog conditions that prevent a completeness claim;
- evidence hash/immutability failure.

A pipeline-integrity problem is not automatically attributed to player misconduct.

### 6.3 `STATISTICAL_SECURITY_ANOMALY`

A versioned detector identifies unusual behavior such as impossible-looking action rates, automation patterns, suspicious transfer graphs, repeated loops or abnormal session/channel patterns.

Unless an accepted authoritative rule independently proves impossibility, the result remains a statistical/hypothesis class.

### 6.4 `ECONOMY_HEALTH_OBSERVATION`

Measures population/world-context outcomes such as:

- item/currency source/sink composition;
- supply/money/value growth;
- concentration/distribution;
- turnover/velocity and liquidity where an owning market contract exists;
- scarcity/progression-to-upgrade outcomes;
- Channel multiplicity exposure versus source/eligibility policy;
- unexplained value residuals only when durable completeness permits the claim.

Economy health does not identify guilt and cannot autonomously tune rates/sinks.

### 6.5 `ENGINE_DEFECT_HYPOTHESIS`

Correlates invariant, replay, build/content/ruleset and failure-path evidence to identify a likely engine defect. SIM replay may upgrade reproducibility of the evidence, but repair remains foreign authority.

## 7. DUR-03 provenance projection

When owning DUR/gameplay event families exist, ANL-03 should be able to reconstruct a read-only provenance graph/ledger for the declared scope using typed facts such as:

- ItemInstanceId lifecycle transitions;
- immediate semantic source/destination location/custody;
- quantity/value before/after where the owning transaction semantics require it;
- DUR-03 conservation classification: `TRANSFER`, `SPLIT_MERGE_QUANTITY`, `STATE_MUTATION`, `MINT`, `BURN`, `TRANSFORM`, `CONVERSION`;
- typed source/sink/occurrence/cause;
- TransactionId/OperationId/CommandRef/CausationRef as applicable;
- TransactionEventRef complete set;
- world/channel/instance/runtime ownership-generation context where material;
- item/domain revision and ruleset/content/build context;
- stable reward/source occurrence and multiplicity/eligibility context where applicable.

The graph is a derived investigation projection. It is not a second location ledger and cannot repair the authoritative store.

## 8. Economy interpretation rules

1. `WorldId` is the default economy boundary; ChannelId is context, not a separate economy namespace.
2. Channel-local simulation does not imply channel-local durable eligibility/source multiplicity.
3. Reference economy analysis uses mechanical source/sink semantics and Oteryn's actual world history; historical Global prices/supply are not parity constants.
4. World age, active population/exposure and content/ruleset revision accompany supply/concentration/liquidity interpretation.
5. Unexplained residual value may be called an integrity gap only when the relevant durable projection is proven complete for the declared scope.
6. Best-effort loot/activity telemetry may explain behavior but cannot override DUR-03 durable value truth.
7. Market value metrics require an accepted market semantic source; no arbitrary external/live scrape becomes authoritative valuation by convenience.

## 9. Detector definition/version contract

Every material detector has a stable logical `detector_id` and immutable positive `detector_revision`.

A detector definition declares at least:

```text
purpose / threat or defect hypothesis
detector class: deterministic-rule | statistical-rule | model | graph | reconciliation
required input event/projection families + schema revisions
required durability/completeness and privacy classes
applicable world/ruleset/content/build/SIM revisions
feature definitions and revisions
observation window / time basis
subject scope
rule/model/algorithm artifact identity + digest
threshold/configuration revision where applicable
output category/severity/confidence semantics
source-quality prerequisites
known false-positive modes / exclusions
resource bounds and degradation behavior
validation/evaluation corpus revision
privacy/retention/access requirements
```

Changing rule logic, feature meaning, model artifact, threshold semantics, subject scope or source requirements creates a new detector revision or explicitly versioned configuration revision according to the owning implementation contract. Historical signals retain the exact detector/config/artifact revision that produced them.

## 10. Detector run and signal output

Exact storage/wire ID types remain implementation-owned, but every emitted signal requires a stable non-reused reference and must record:

```text
signal reference
detector_id + detector_revision
configuration/model/artifact digest
run/evaluation reference
observation window
subject scope/reference under appropriate privacy class
signal category
impact severity / triage priority
confidence or score semantics when applicable
invariant/rule/hypothesis evaluated
source event/checkpoint/evidence references
source completeness/quality vector
relevant World/Channel/Instance + semantic revisions
known exclusions/false-positive caveats
creation time
status/disposition linkage
```

Severity estimates possible impact/triage priority. Confidence/score estimates detector evidence under its documented semantics. Neither field means guilt or grants enforcement authority.

When optional client diagnostics are a possible source, the source completeness/quality vector may record their availability/quality only. Missing or opted-out optional diagnostics cannot increase severity/priority, confidence/score or any adverse classification and cannot be used as guilt evidence.

Repeated evaluation of identical evidence should be deduplicable/coalescible without deleting the original signal/evidence lineage.

## 11. Deterministic versus statistical conclusions

### Deterministic invariant conclusion

May be labeled `INVARIANT_VIOLATION_SUPPORTED` only when:

- the referenced invariant is accepted/versioned;
- the required durable evidence is complete for the declared scope;
- event/schema/hash/order/transaction-set integrity prerequisites pass;
- the consumer logic revision is reproducible;
- no known evidence-pipeline failure explains the mismatch.

If those prerequisites fail, disposition is `INCONCLUSIVE_DATA_QUALITY` or an audit-pipeline incident, not a player/security conclusion.

### Statistical/model conclusion

Remains `ANOMALY_HYPOTHESIS` until human/independent evidence review. A calibrated probability/score may prioritize review but is not transformed into deterministic guilt.

### Replay-corroborated defect

SIM evidence may support `REPLAY_CORROBORATED_DEFECT` when exact replay provenance reproduces the divergence/invariant failure under the relevant semantic revision set. It still does not authorize repair or player sanction.

## 12. Case/evidence lifecycle

ANL-03 defines a read-only analytical lifecycle, not enforcement workflow. Signal-level disposition is mandatory even when a case is not opened, and referral is a separate routing action rather than an evidence conclusion:

```text
SIGNAL_EMITTED
-> TRIAGED
   -> SIGNAL_DISPOSITION_RECORDED -> [ROUTING_REFERRAL_RECORDED] -> SIGNAL_CLOSED
   OR
   -> CASE_OPENED
      -> EVIDENCE_ASSEMBLED / UPDATED
      -> HUMAN_EVIDENTIARY_DISPOSITION
      -> [ROUTING_REFERRAL_RECORDED]
      -> CLOSED
      -> REOPENED (when later evidence/review requires it)
```

### Required lifecycle properties

- original signals are immutable historical evidence except for separately versioned annotation/status projections; the immutable analytical history is never replaced by a mutable latest status;
- every triaged signal receives a substantive evidentiary disposition for that review generation, including when no case is opened;
- grouping signals into a case preserves every source signal reference and prior signal-level disposition/history;
- every material lifecycle transition and reviewer/operator action appends an immutable ordered audit record; a mutable latest-status projection may exist for convenience but cannot be the only history;
- audited actions include at minimum signal triage, signal disposition/close/reopen, case open, assignment/reassignment when assignment exists, evidence addition/removal/supersession, state transition, close, reopen, referral and final disposition;
- each audit record carries the privacy-appropriate reviewer/operator identity or pseudonymous privileged identity, role/capability, timestamp and ordered occurrence, previous state where applicable, new state/action, reason/rationale where applicable, linked evidence references, detector/rule/model/config revision where relevant, and signal/case/correlation identity;
- a referral audit record additionally references the preceding substantive evidentiary disposition for the same review generation and the target foreign owner;
- evidence removal or supersession changes the active evidence set without erasing the historical audit event or original evidence reference; underlying evidence retention/deletion remains governed by its owning privacy/retention/legal-hold policy;
- false-positive dispositions and reviewer actions remain reconstructible and auditable from the immutable history;
- privileged identity resolution is separately authorized and access-audited;
- enforcement/remediation outcome may be linked by foreign reference but is not executed by ANL-03.

### Substantive evidentiary disposition vocabulary

A signal or case, as applicable, records one truthful substantive evidentiary disposition from:

- `SUPPORTED_INTEGRITY_OR_DEFECT_FINDING`;
- `SUPPORTED_SECURITY_FINDING`;
- `NOT_SUPPORTED_FALSE_POSITIVE`;
- `INCONCLUSIVE_INSUFFICIENT_EVIDENCE`;
- `DATA_QUALITY_OR_PIPELINE_FAILURE`;
- `DUPLICATE_OR_ALREADY_COVERED`.

`REFERRED_TO_SECURITY_GM_PRODUCT_OR_ENGINE_OWNER` is **not** an evidence disposition. It is an optional routing action only and cannot stand alone as the terminal analytical classification. Both no-case and case-based referral require a preceding substantive disposition for the same review generation. If positive support is unavailable, referral—when still appropriate for foreign-owner context—follows the truthful inconclusive/data-quality/false-positive/duplicate disposition rather than replacing it.

Referral does not strengthen evidence quality, turn a statistical anomaly into deterministic proof, imply target-owner acceptance or authorize sanction. `SUPPORTED_SECURITY_FINDING` is likewise non-sanctioning human-reviewed analytical support; it preserves the underlying statistical/model caveats unless a separate accepted deterministic invariant is proven.

## 13. False-positive controls

`FS-DETECTOR-FALSE-POSITIVE` is a first-class ANL-03 requirement.

A conforming design requires:

1. no automatic sanction, value mutation or gameplay effect from detector output;
2. exact detector/rule/model/config revision retained with every signal;
3. input evidence/checkpoint references retained within policy;
4. every material reviewer action, lifecycle transition and false-positive disposition is preserved in immutable ordered signal/case audit history;
5. every no-case triage records a substantive terminal signal disposition rather than disappearing from case statistics;
6. referral is an auditable routing action attached to a substantive evidentiary disposition and can never replace that disposition;
7. detector threshold/model changes create new versioned semantics rather than rewriting historical signals;
8. suppression/allow-list policy, if later authorized, is versioned, scoped and auditable and cannot become a hidden enforcement bypass;
9. false positives are measurable as detector-quality evidence where ground truth/reviewer disposition is suitable;
10. missing required authoritative/durable evidence may constrain the conclusion to inconclusive/data-quality; missing or opted-out optional client diagnostics are non-adverse and cannot increase score, confidence, severity, review/enforcement priority or guilt inference;
11. a data-pipeline defect is classified separately from subject behavior;
12. a reviewed false positive remains available according to its retention policy for detector evaluation; it is not silently deleted to improve reported accuracy.

Exact detector quality thresholds are not frozen here.

## 14. Security analytics scope

Subject to producer-owned typed evidence, ANL-03 may analyze:

- command replay/duplicate/stale-session or stale-ownership attempts;
- suspicious admission/reconnect/channel-switch patterns;
- cooldown/sequencing/action-rate anomalies;
- automation/bot-like behavior patterns;
- repeated reward/source occurrence patterns;
- item/currency provenance anomalies;
- unusual transfer concentration/fan-in/fan-out/cycles;
- market/trade/mail/depot inconsistencies once those domain contracts exist;
- protocol capability/downgrade/revision anomaly evidence;
- engine defects exposed by durable invariant/replay mismatch.

ANL-03 does not authorize generalized device fingerprinting, invasive client surveillance, raw credential capture or secret retention. Any new device/network identity collection requires a named security/privacy purpose and owning contract.

## 15. Privacy, retention and access

ANL-01 privacy classes remain normative.

### Broad detection

Where feasible, broad statistical detection uses `PSEUDONYMOUS_ANALYTICS` or non-personal features. Raw operational identity is not collected merely because it improves convenience.

### Restricted investigation

Item/transaction/session/player-linked evidence needed for a concrete integrity/security case uses `RESTRICTED_PLAYER_LINKED` or `SECURITY_SENSITIVE` according to the producer/purpose. Access is least-privilege and audited.

### Pseudonym mapping

The AnalyticsActorId-to-operational identity mapping remains separately protected. A detector does not gain mapping access by default. Privileged resolution is purpose-authorized for a case and recorded.

### Fine location/time/network context

Fine location/timestamps and any later network/device facts are re-identification/security-sensitive context and require explicit purpose/retention/export controls. Ordinary economy-health analytics should use coarser aggregates.

### Optional client diagnostic availability

Optional client diagnostics remain governed by their owning consent/privacy contract. ANL-03 may record whether eligible evidence was available and its quality, but opt-out, non-collection or absence is never an adverse subject attribute and never grants broader collection, identity resolution, retention or access authority.

### Retention

At minimum, separate finite profiles are required for:

- raw event/audit evidence;
- detector features/projections;
- signals;
- signal dispositions and routing records;
- case evidence/annotations;
- immutable signal/case lifecycle/reviewer audit history;
- privileged identity-resolution access logs;
- exported evidence packages.

Legal hold is an explicit authorized exception, never default unlimited retention. Lifecycle-history immutability means recorded audit events cannot be rewritten into a latest-only narrative; it does not override an applicable authorized privacy deletion/anonymization requirement for referenced personal evidence.

## 16. Access/credential boundary

A future ANL-03 detector/investigation service must operate with read-only privileges appropriate to its source. It must not possess:

- gameplay mutation credentials;
- production DB write authority;
- sanction/account-action authority;
- deployment authority;
- rollback/confiscation authority.

If a downstream workflow needs enforcement or remediation, ANL-03 first records its substantive evidentiary disposition and then may emit a separate referral/evidence reference to the foreign owner. The foreign owner revalidates current authority/evidence under its own contract; referral does not imply acceptance or authority transfer.

ANL-04 retains the explicit AI/read-only investigation boundary from ADR-0006.

## 17. Resource-bound needs

ANL-03 inherits applicable ANL-01 hard bounds for event/payload, publication/quarantine, replay windows, query pages/results and evidence packages. Current shared bounds include the bounded `ANL01-EVIDENCE-PACKAGE-BYTES` export ceiling.

Before implementation acceptance, every externally controlled/high-growth detector operation must additionally register bounds for applicable:

- detector observation-window duration/event count;
- per-subject retained state;
- feature-vector cardinality/bytes;
- graph traversal vertices/edges/depth/fan-out;
- join expansion;
- signals emitted per run/window/subject;
- signal lifecycle/audit actions per retained signal;
- case evidence references/attachments;
- concurrent detector/backfill jobs;
- model/rule artifact size;
- investigator query/export scope;
- resumable provenance/reconciliation work units.

Exact hard numbers require implementation/PERF/OPS evidence and belong in `RESOURCE_LIMITS_REGISTRY.json` under the owning implementation contract.

A detector may lag, pause, quarantine or fail. It may not block authoritative gameplay merely because an analytical dependency is slow. Durable audit evidence itself remains governed by ANL-01/DUR no-silent-drop semantics.

## 18. Detector degradation and failure semantics

| Condition | Required ANL-03 behavior |
|---|---|
| Best-effort feature loss | mark detector input partial; no completeness-dependent conclusion |
| Missing/opted-out/unavailable optional client diagnostics | record availability/quality only; no adverse evidence, no increase in score/confidence/severity/review or enforcement priority, no guilt inference; server-generated evidence path remains sufficient for investigation |
| Durable audit checkpoint/event-set gap | stop/inconclusive for completeness-dependent invariant; emit pipeline-quality incident as appropriate |
| Same EventId conflict | ANL evidence-integrity incident; no overwrite |
| Unsupported durable schema | quarantine/reject per ANL-01; detector does not reinterpret |
| Duplicate delivery | idempotent projection/signal evaluation |
| Out-of-order durable events | bounded defer/reconcile using accepted refs/revisions; no invented state |
| Detector model/rule unavailable | detector unavailable; gameplay unchanged |
| Detector threshold/model revision changes | new versioned evaluation; historical signal unchanged |
| Investigator identity mapping unavailable | no fallback to unauthorized raw mapping |
| Privacy/retention/access policy missing | no collection/projection/disclosure for that class |
| Evidence package exceeds bound | paginate/partition/redact; never bypass bound by privileged role |
| Triaged signal with no case | record substantive signal disposition and close/review-generation action; no silent disappearance |
| Referral requested without substantive disposition | reject/defer routing until a truthful substantive evidentiary disposition exists for the same review generation |
| Suspected violation with incomplete evidence | `INCONCLUSIVE_INSUFFICIENT_EVIDENCE`, not automatic sanction |

## 19. Failure-scenario mapping

- `FS-AUDIT-OUTBOX-BACKLOG`: **PASS at candidate consumer semantic level** — detector completeness degrades/stops; committed durable records are not discarded by ANL-03.
- `FS-EVENT-DUPLICATE-DELIVERY`: **PASS** — one derived analytical effect per identical EventId.
- `FS-EVENT-OUT-OF-ORDER`: **PASS** — bounded reconcile/defer, no fabricated provenance.
- `FS-AUDIT-MUTATION-MISMATCH`: **PASS at analytical semantic level** — mismatch becomes named integrity/data-quality evidence and never silent repair; physical prevention remains DUR-02/03.
- `FS-ANALYTICS-PRIVACY-POLICY`: **PASS** — missing purpose/privacy/retention/access blocks collection/projection/disclosure; optional diagnostic opt-out/absence remains non-adverse.
- `FS-DETECTOR-FALSE-POSITIVE`: **PASS at candidate semantic level** — no auto-sanction/mutation; detector version, evidence, immutable signal/case transition/reviewer history, substantive no-case disposition and referral-as-routing semantics remain auditable.
- `FS-INVESTIGATION-MUTATION-ATTEMPT`: **DEFERRED_BY_ACCEPTED_GATE** for runtime proof to ANL-04/implementation; ANL-03 preserves read-only credential requirements.

Architecture PASS does not claim runtime evidence.

## 20. Proposed decisions

### ANL03-D1 — Deterministic invariant evidence and statistical anomaly are separate classes

- Must decide now: **YES**.
- Blocks: trustworthy detector outputs and human triage.
- Owner: ANL-03 consumer semantics, constrained by FND/DUR invariant owners.
- Failure/security implication: prevents a model score from being represented as deterministic proof.
- Supersession evidence: new classes may be added, but the proof-versus-hypothesis distinction must remain explicit.

### ANL03-D2 — Every detector/output is reproducibly versioned

- Must decide now: **YES**.
- Blocks: false-positive audit, incident reproduction, model/rule changes and case review.
- Owner: ANL-03.
- Resource/security implication: version/digest metadata must be retained within bounded policy; historical outputs cannot silently drift.
- Supersession evidence: implementation may refine identity representation, not historical semantic immutability.

### ANL03-D3 — Evidence disposition precedes optional referral; enforcement remains foreign

- Must decide now: **YES**.
- Blocks: safe security tooling and future ANL-04 AI composition.
- Owner: ANL-03 for evidence workflow and routing record; sanction/GM/product action foreign.
- Failure/security implication: false positive, inconclusive or data-quality outcomes cannot disappear into a naked referral, and a supported finding cannot directly become punitive mutation; every material lifecycle/reviewer/routing action remains reconstructible from immutable audit history.
- Supersession evidence: any automated enforcement would require a separately accepted higher-authority security/product contract and explicit override of ADR-0006; not an ANL-03 local change.

### ANL03-D4 — Absence is proof only under proven durable completeness

- Must decide now: **YES**.
- Blocks: duplication/source-sink/invariant claims.
- Owner: ANL-03 interpretation constrained by ANL-01/DUR-03.
- Failure implication: consumer lag/schema gaps cannot falsely implicate or exonerate; optional client-diagnostic absence is separately non-adverse and never increases suspicion, score, confidence or priority.
- Supersession evidence: none that removes completeness requirement; implementation can strengthen proofs.

### ANL03-D5 — Economy health remains world-contextual and non-controlling

- Must decide now: **YES**.
- Blocks: source/sink/concentration/scarcity analytics.
- Owner: ANL-03 for observation; product/economy domains own desired targets/actions.
- Failure implication: prevents channel-count or historical price artifacts from being treated as authoritative tuning inputs.
- Supersession evidence: later economy owner decisions may define numeric targets/control proposals, but automated mutation remains separately gated.

## 21. DECISIONS_NOT_TAKEN

ANL-03 deliberately does **not** decide:

- concrete DUR/FND/gameplay event type IDs or payload schemas;
- physical audit/warehouse/lake/database/broker topology;
- detector implementation language/framework/vendor/ML platform;
- exact detector algorithms, model architectures or numeric thresholds;
- production detector enablement/rollout;
- GM/sanction/account/moderation policy;
- automatic ban/mute/kick/confiscation/rollback/value repair;
- production DB writes or gameplay mutation;
- device fingerprinting/client surveillance/network identity collection;
- exact privacy retention durations/legal policy;
- market/trade/mail/depot/reward business rules;
- exact economy-health target values, taxes/fees/drop/sink rates;
- Platform implementation;
- ANL-04 AI implementation.

## 22. CROSS_DOMAIN_FINDINGS

### ANL03-XD-01 — DUR-03 concrete durable event families are not registered

```yaml
cross_domain_finding:
  id: ANL03-XD-01
  observed_in_domain: ANL-03
  target_owner: DUR-03/item-currency transaction implementation owners + ANL-01 registry integration owner
  severity: P1
  evidence: DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md requires mandatory provenance/evidence; ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md leaves domain payloads downstream; GAME_EVENT_FOUNDATION_REGISTRY.json event_types=[]
  conflict_or_gap: ANL-03 can define provenance/invariant consumer semantics, but no concrete durable item/currency/value event catalogue currently exists to implement complete conservation projections.
  required_before: production-capable item/currency provenance, duplication/invariant detector or complete economy source/sink projection is claimed
  worker_action: REPORT_ONLY
```

### ANL03-XD-02 — enforcement/GM disposition is intentionally unowned by ANL-03

```yaml
cross_domain_finding:
  id: ANL03-XD-02
  observed_in_domain: ANL-03
  target_owner: security/GM/product-policy owner to be assigned by coordinator
  severity: P2
  evidence: ADR-0006 human review/authorization boundary; issue #264 forbids sanctions and requires downstream GM/security dependencies
  conflict_or_gap: ANL-03 can close evidence and separate referral routing semantics, but no accepted contract in this worker scope defines how a supported security/integrity finding authorizes account/player sanctions, confiscation or remediation.
  required_before: any production workflow performs enforcement/remediation from an ANL-03 signal/case referral
  worker_action: REPORT_ONLY
```

### ANL03-XD-03 — security producer event coverage is not registered

```yaml
cross_domain_finding:
  id: ANL03-XD-03
  observed_in_domain: ANL-03
  target_owner: FND-02/FND-04/FND-03/GAME-CHANNEL and owning security integration gates
  severity: P2
  evidence: ADR-0006 security analytics examples; ANL-01 typed producer ownership and empty event registry
  conflict_or_gap: replay/stale-session/admission/reconnect/channel-switch detectors require explicit producer-owned security observations; ANL-03 cannot infer complete coverage from operational logs or missing events.
  required_before: corresponding security detector claims concrete coverage
  worker_action: REPORT_ONLY
```

### ANL03-XD-04 — economy business-domain semantics remain downstream

```yaml
cross_domain_finding:
  id: ANL03-XD-04
  observed_in_domain: ANL-03
  target_owner: market/trade/mail/depot/reward/economy domain gates
  severity: P2
  evidence: DUR-03 authority chain and ARCHITECTURE_ANALYSIS_GAP_REGISTER.md economy unresolved scope
  conflict_or_gap: transfer graph and economy-health analytics can use DUR-03 conservation classes now, but fraud/business-policy interpretation for market/trade/mail/depot/reward operations requires their owning domain contracts.
  required_before: domain-specific fraud/abuse detector semantics become canonical
  worker_action: REPORT_ONLY
```

## 23. Candidate conclusion

`RECOMMENDATION` — ANL-03 is mature enough for a bounded candidate contract because detector reproducibility, evidence quality, false-positive safety, substantive signal/case disposition, separate referral routing and read-only authority can be frozen independently of concrete algorithms/storage/producer payload IDs.

Concrete integrity/security coverage remains fail-closed on producer event registration and foreign enforcement/business-domain contracts. This worker draft is nonbinding until Architecture Coordinator acceptance; implementation remains `NOT_STARTED`.