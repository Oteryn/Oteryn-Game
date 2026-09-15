# OTV2-20260813-sim-determinism-01-architecture — archived

```yaml
task_id: OTV2-20260813-sim-determinism-01-architecture
title: SIM-DETERMINISM-01 authoritative simulation determinism architecture
mode: CONTRACT
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
delivery_branch: agent/otv2-20260813-sim-determinism-01-architecture
delivery_pr: 214
base_sha: 27c313a0c6032f0433ad9598c3cf53e4f0179813
final_head_sha: 4c6684328123aebd657696808372a5855980d34e
delivery_merge_sha: 1e16b32069868f14aa1761a512b6cd8b1024e277
lifecycle_closeout_branch: docs/OTV2-20260813-sim-determinism-01-closeout
lifecycle_closeout_pr: 215
owner: released_after_closeout
created_at: 2026-08-13T00:55:00+02:00
completed_at: 2026-08-13T09:24:00+02:00
execution_budget_minutes: 90
implementation_status: NOT_STARTED
runtime_client_authority: NONE
postgresql_ddl_migration_authority: NONE
platform_write_authority: NONE
production_authority: NONE
owned_paths:
  - docs/agents/tasks/archive/OTV2-20260813-sim-determinism-01-architecture.md
  - docs/architecture/SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_ANALYSIS.md
  - docs/architecture/SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md
public_contracts:
  - SIM-DETERMINISM-01
depends_on:
  - FND-03
  - GAME-VISION-01
  - GAME-CHAR-01
  - GAME-ITEM-01
  - DUR-03
  - DUR-04
  - ANL-01
cross_repository_coordination_id: OTV2-GLOBAL-ARCHITECTURE
external_repositories: []
```

## Outcome

Delivered one bounded paper-only `SIM-DETERMINISM-01 — Authoritative Simulation Determinism Contract` refining, rather than replacing, accepted FND-03 runtime order/RNG/replay semantics and DUR-04 guest/script determinism.

Delivery PR #214 was squash-merged from exact repaired final head `4c6684328123aebd657696808372a5855980d34e` as `1e16b32069868f14aa1761a512b6cd8b1024e277` after the repository owner explicitly instructed the coordinator to finish the task and thereby overrode the otherwise-required fresh independent-review-after-repair mechanism for this exact PR/head.

The delivery implements nothing. Rust/runtime/client/combat/AI/script implementation, exact gameplay/Reference/balance formulas, RNG/numeric/hash dependency adoption, PostgreSQL DDL/migrations, Platform writes and production activation remain unauthorized.

## Binding sources consumed

- FND-03 runtime execution contract — one logical writer per Channel/Instance, owner-local `RuntimeExecutionOrdinal`, no global total order, wall/monotonic/execution-order separation, deterministic gameplay RNG/replay evidence and stale-result rejection;
- GAME-VISION-01 — Reference-sensitive arithmetic remains evidence-gated; determinism does not establish Global correctness;
- GAME-CHAR-01 — authoritative progression facts remain formula-neutral where arithmetic does not constrain identity/ownership/migration;
- GAME-ITEM-01 — deterministic modifier contribution ordering while exact arithmetic remains SIM/ruleset-owned;
- DUR-03 — exact item/currency/value conservation, idempotency and transaction authority cannot be weakened by approximate SIM arithmetic;
- DUR-04 — authoritative script execution uses immutable invocation snapshots, invocation-local deterministic RNG, stable query order, proposal-only mutation and versioned `script_execution_profile_revision`;
- ANL-01 — replay/divergence evidence remains observational/read-only and privacy/access controlled;
- Resource Limits Registry — missing applicable hard resource bounds block implementation rather than meaning unlimited.

## Accepted candidate semantic closure delivered by #214

### Core reproducibility and authority

- determinism begins from canonical authoritative deterministic state plus the exact accepted owner-local normalized input order and exact semantic revision/profile set;
- OS thread scheduling, worker wake-up order, socket timing, CPU count and non-semantic wall-clock jitter are not replay prerequisites;
- FND-03 `RuntimeExecutionOrdinal` remains owner-local authority/evidence; SIM introduces no second runtime commit ordinal and no global total order;
- FND-02 CommandRef/order, domain revisions, EventId, OperationId and TransactionId remain distinct identities.

### `SimulationDeterminismProfileRevision`

A versioned SIM profile binds cross-cutting deterministic machinery that can alter normalized authoritative outcome, including:

- numeric semantic-class rules;
- checked overflow/invalid-number behavior;
- rounding vocabulary/semantics;
- gameplay RNG algorithm/profile identity and stream-derivation version;
- RNG decision/consumption semantics;
- SIM-owned tie-break/canonical-order profiles;
- canonical deterministic-state hash serialization/profile;
- supported authoritative-target policy;
- compatibility with applicable DUR-04 `script_execution_profile_revision`.

The SIM profile does not replace ruleset/content/world-policy/script execution revisions.

### Semantic revision binding

Every authoritative occurrence that can be retried or outlive immediate synchronous resolution binds the exact behavior-affecting semantic revision set needed to reproduce it, including as applicable:

- ruleset/formula revision;
- content revision;
- world-policy revision;
- `SimulationDeterminismProfileRevision`;
- script artifact/WIT/`script_execution_profile_revision`.

Retry/reconnect/failover/delayed completion cannot silently reinterpret the same logical occurrence under a newer incompatible revision. The owner must either continue under originally bound compatible semantics or use an explicit transition/reconciliation policy.

### Numeric semantic classes and formulas

- exact discrete identifiers/counts/ordinals/quantities use exact checked integer semantics;
- DUR-03 item/currency/value conservation quantities remain exact and cannot use binary floating/tolerance comparison as authority;
- formula coefficients/rates/percentages should use explicit integer/fixed-scale/rational semantics where accepted behavior permits;
- authoritative floating point is allowed only under an explicit deterministic profile with cross-target proof and controlled NaN/infinity/conversion/SIMD behavior;
- approximate renderer/UI/telemetry/analytics numbers remain non-authoritative unless explicitly validated into an authoritative class;
- authoritative formula families use versioned descriptors defining units, representations, operation-order constraints, rounding points/modes, clamps, invalid-state behavior, output range and Reference evidence state;
- implicit wrap/truncation and debug-vs-release arithmetic drift are forbidden;
- no rounding mode is assumed to be Global-correct without evidence.

### Gameplay RNG ownership and isolation

- cryptographic/security randomness, deterministic authoritative gameplay randomness and presentation randomness remain separate classes;
- one mutable process-global gameplay RNG stream is forbidden;
- every random decision belongs to a stable semantic scope/domain and purpose identity, never NodeId/thread/pointer/source-line/unordered-container position;
- unrelated mechanics must not perturb each other's future sequence merely because one adds draws;
- implementations may use keyed/counter decisions or isolated checkpointed stateful substreams;
- the chosen gameplay RNG algorithm/profile/stream derivation is immutable under one SIM profile and requires cross-target fixtures before implementation acceptance;
- same idempotently retried logical occurrence preserves its random outcome and bound semantic revisions;
- transient RuntimeExecutionOrdinal/generation may be order evidence but does not by itself create a new random decision;
- stateful RNG advancement is authoritative state: rejected/speculative/aborted work cannot advance the committed future stream independently;
- process restart/failover cannot reseed gameplay semantics from current wall clock/entropy merely because NodeId/ownership generation changed.

### RNG anti-prediction boundary

Deterministic/replayable does not imply publicly predictable.

- exploit-sensitive server-controlled seed/root/substream state may remain confidential;
- public-only deterministic seed derivation is forbidden for exploit-sensitive decisions unless an explicit product/security contract accepts predictability;
- gameplay seed/substream evidence must not leak into clients or ordinary telemetry where it enables loot/spawn/combat prediction;
- replay retention of sensitive RNG evidence follows security/privacy controls;
- deterministic gameplay RNG remains forbidden for credentials/secrets.

### DUR-04 script composition

- DUR-04 owns guest-engine deterministic semantics and `script_execution_profile_revision`;
- SIM owns how stable authoritative script invocation/random-decision identity composes with core simulation/replay;
- scripts never receive a mutable core/process-global RNG handle;
- script proposal rejection/retry cannot perturb unrelated core RNG sequences;
- retried script-backed logical occurrences retain the same compatible SIM/content/ruleset/script semantic revision set or use explicit transition/reconciliation.

### Time and order

The accepted distinction remains:

```text
wall clock != process-local monotonic elapsed time != authoritative execution order
```

- no project-global fixed tick is introduced;
- authoritative formulas consume explicit normalized semantic time values, not hidden system-clock reads;
- replay uses recorded/injected normalized time/calendar facts;
- semantically simultaneous/conflicting inputs use commutative/set semantics, stable semantic tie-breaks or exact retained FND-03 RuntimeExecutionOrdinal order;
- pointer/hash-map/thread/worker wake-up order cannot become gameplay authority;
- live network/service arrival order may differ across executions; when accepted cross-source order affects gameplay, replay retains the actual owner-accepted order;
- worker/service results re-enter authority only as normalized FND-03 inputs with stale-generation/revision/input evidence.

### External nondeterminism

Any external fact that affects authoritative gameplay must become a typed normalized authoritative input/fact before use.

- replay retains the accepted behavior-affecting value/identity/revision rather than requerying mutable external systems;
- externally returned collections are canonicalized to semantic order or exact accepted meaningful order is retained;
- reusable credentials/secret keys/raw security RNG are not retained merely for replay.

### Replay envelope and provenance

The final repaired replay envelope retains at least:

- semantic runtime scope and ownership-generation boundary;
- original NodeId/process-incarnation evidence where retained for forensic attribution;
- exact authoritative server/build executable revision/artifact identity;
- applicable `protocol-oteryn` application/protocol revision and compatibility/profile identity;
- exact World Bundle artifact identity/digest plus behavior-affecting content/map/ruleset/world-policy compatibility revisions;
- initial canonical deterministic state/checkpoint + composite hash;
- ordered normalized inputs + RuntimeExecutionOrdinal evidence;
- CommandRef/OperationId/TransactionId/timer/event/work identities where applicable;
- exact semantic revision set bound to long-lived/retryable occurrences where it differs from interval defaults;
- active SIM/formula/script/WIT/execution-profile identities;
- RNG root/substream/decision evidence required by the selected model;
- normalized logical-time/calendar/external facts;
- relevant state-domain revisions;
- expected deterministic-state/result hashes at selected cuts.

Original NodeId/process-incarnation evidence is forensic attribution only. Deterministic replay must not require recreating original process placement, thread IDs, CPU count, worker placement or wall-clock scheduling jitter.

### Canonical future-determining state and hashing

Canonical deterministic state covers every authoritative fact that can alter a future result without a new external normalized fact, including as applicable:

- canonical gameplay/domain state;
- active behavior-affecting content/ruleset/world-policy/formula/SIM/script revision/profile set;
- stateful authoritative RNG/substream state/cursors;
- pending accepted timers/operations/continuations;
- stable occurrence/work identities preventing duplicate/reordered continuation;
- revision/profile identities bound to pending work;
- semantically relevant domain revisions and authority/fence state;
- deterministic owner-local pending/queue metadata only where it can alter future resolution.

Hierarchical canonical deterministic-state hashes are evidence, never gameplay authority. They are versioned and independent of memory layout/padding/pointers/unordered collection iteration/non-authoritative caches.

### Divergence evidence

A comparison can localize at least:

- first mismatching owner-local ordinal/checkpoint cut;
- normalized input identity/type;
- active semantic revision/profile set;
- exact server/build, protocol and World Bundle provenance;
- first mismatching deterministic-state domain/support/component hash path;
- relevant RNG decision/stream evidence;
- pending timer/operation state where relevant;
- formula descriptor/rounding boundary;
- script artifact/execution profile when applicable.

Divergence evidence remains read-only and cannot autonomously repair state.

### Supported targets

For the same replay envelope, every supported authoritative server target must produce the same normalized authoritative outcome.

- exact discrete/conservation/fixed-scale outputs, RNG decisions and canonical deterministic-state hashes match exactly;
- authoritative floating implementations require cross-target proof of identical normalized outcome;
- incompatible targets fail compatibility/readiness rather than silently diverging;
- non-authoritative client prediction/rendering does not become server truth.

### Failure, resource, security and privacy

- numeric/RNG/determinism invariant failures do not invent implicit clamp/wrap/reseed/reroll behavior;
- unexpected authoritative invariant failure follows FND-03 fail-closed handling;
- DUR transactions retain their own commit/rollback authority;
- required replay/hash/RNG/formula/pending-state limits must exist before implementation acceptance;
- replay/divergence artifacts follow ANL/privacy access/retention/export controls;
- replay/testing/investigation cannot directly mutate live authority or reissue historical actions as trusted live commands.

## Architecture decision test

- **Must decide now:** YES for the cross-cutting numeric/RNG/order/replay semantic boundary before broad combat/AI/progression formula implementation.
- **Concrete work blocked:** broad combat/damage/healing formula implementation, production AI decisions consuming authoritative randomness, exact Character progression/skill arithmetic delegated from GAME-CHAR, ruleset formula-package freeze, deterministic replay/state-hash implementation contract and parity-confirmed formula/rounding claims.
- **Later migration cost:** versioning/migrating affected formulas/rulesets, retaining old RNG algorithms/stream derivations for replay, rebuilding fixtures, migrating replay/hash evidence, retesting supported targets, reconciling durable progression produced under old arithmetic, and compatibility bridges for old content/script execution profiles.
- **Superseding evidence:** Reference behavior, cross-target divergence, replay/fault evidence, representative deterministic-profile performance evidence, security findings or materially changed product requirements.
- **Deliberate deferrals:** concrete Rust numeric/RNG/hash library, exact gameplay RNG algorithm, per-formula fixed scale, exact combat/XP/skill/item values, global tick rate, scheduler implementation/weights, worker counts/CPU affinity, replay storage backend/retention and production hash cadence.

## Repair history

### Cycle 1 — retry semantic-revision binding and RNG anti-prediction

Adversarial self-review found that retry/delayed completion could otherwise be evaluated under newer semantics and deterministic RNG could be replayable yet trivially predictable. Repair 1 bound occurrences to their exact behavior-affecting revision set and established the server-controlled anti-prediction boundary.

### Cycle 2 — future-determining deterministic state

Adversarial self-review found visible gameplay state insufficient for divergence hashing. Repair 2 expanded canonical deterministic state/hash scope to active behavior revisions, RNG state, pending accepted work, occurrence identities and semantically relevant fence/revision metadata.

### Cycle 3 — analysis/contract reconciliation

Final pre-freeze self-review found the analysis still described pre-repair retry/hash semantics. Repair 3 reconciled analysis and normative contract to one model.

### Cycle 4 — executable/protocol/World Bundle replay provenance

Independent exact-head architecture review `4924203877` on frozen head `5dc628f32ca4573725bcb4a42c3a7702536d7f35` confirmed one material P1: the proposed replay envelope had lost FND-03-required server/build, protocol and exact World Bundle provenance. The owner explicitly authorized one additional bounded repair cycle for that finding. Repair 4 restored those identities, aligned analysis/contract, added divergence/implementation evidence, and preserved original process-incarnation identity only as optional forensic attribution rather than replay placement authority.

Repair budget ended at **`4/4`**, where cycle 4 was the explicit one-time owner exception for the confirmed P1.

## Terminal delivery validation

Final repaired exact delivery head: `4c6684328123aebd657696808372a5855980d34e`.

Evidence:

- independent exact-head architecture review `4924203877` on superseded pre-repair head `5dc628f32ca4573725bcb4a42c3a7702536d7f35`: **BLOCK / 1 material P1**;
- P1 inline thread `PRRT_kwDOTuGrds6Ywan8` / comment `PRRC_kwDOTuGrds7gxc4H`: materially repaired in cycle 4 and resolved;
- terminal full-diff exact-head self-review `4924321455` on `4c668432...`: **PASS**, material findings `0`;
- owner-requested repeat exact-head review `4924423397` on `4c668432...`: **PASS**, material findings `0`; explicitly self-review from the same repair session, not independent review;
- Agent Governance `31676250271`: **PASS**;
- Dependency Review `31676250273`: **PASS**;
- CodeQL `31676250272`: **PASS**;
- unresolved review threads immediately before merge: `0`;
- final scope: exactly task + analysis + candidate contract;
- component/integration/runtime E2E: `NOT_APPLICABLE` — paper-only architecture;
- no new Codex/OpenAI API/owner-funded AI invocation was requested by the coordinator after the owner-funded-AI prohibition became applicable to this task.

### Owner review-gate override for finalization

After the repaired P1, updated exact-head self-review and review history were explicitly presented on 2026-08-13, the repository owner instructed the coordinator to **finish the task**.

For PR #214 at exact final head `4c6684328123aebd657696808372a5855980d34e`, this is the explicit owner override of the otherwise-required fresh independent-review-after-repair gate. It does not retroactively make self-review independent, does not dismiss the historical P1, does not authorize any further repair beyond the accepted final head, and does not authorize any new Codex/OpenAI API/owner-funded AI invocation.

Delivery PR #214 was then squash-merged using `expected_head_sha=4c6684328123aebd657696808372a5855980d34e` as `1e16b32069868f14aa1761a512b6cd8b1024e277`.

## Lifecycle closeout discipline

The separate closeout may only:

1. complete active -> archive movement and preserve complete delivery/review/repair/CI/owner-override/merge history;
2. promote `SIM-DETERMINISM-01` to `ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED` in maintained programme/register/horizon/index sources;
3. preserve runtime/client/combat/AI/script/formula/dependency/DDL/Platform/production authority as unauthorized;
4. refresh the non-owning programme checkpoint and successor handoff to exactly one next safe paper-only action;
5. release SIM path ownership only after closeout merge.

## Context checkpoint

```yaml
last_progress: SIM-DETERMINISM-01 delivery PR #214 merged unchanged from final repaired exact head 4c6684328123aebd657696808372a5855980d34e as 1e16b32069868f14aa1761a512b6cd8b1024e277 after the owner explicitly overrode the fresh-independent-review-after-repair gate for this exact PR/head; lifecycle closeout PR #215 reconciles canonical status and releases SIM ownership.
status: completed
delivery_pr: 214
final_head_sha: 4c6684328123aebd657696808372a5855980d34e
delivery_merge_sha: 1e16b32069868f14aa1761a512b6cd8b1024e277
lifecycle_closeout_pr: 215
independent_review_superseded_head: 4924203877
terminal_self_review: 4924321455
repeat_self_review: 4924423397
ci_run_ids:
  - 31676250271
  - 31676250273
  - 31676250272
repair_cycles_for_delivery_gate: 4
owner_review_gate_override: true
owner_action_required: false
blocker: null
next_action: From live main after lifecycle closeout PR #215 merges, follow the canonical programme checkpoint and successor handoff: create one bounded paper-only task to build the versioned Reference evidence/parity manifest under its owning contract; do not invent a new stable gate ID or implement runtime/DDL/production behavior.
```
