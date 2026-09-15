# SIM-DETERMINISM-01 — Authoritative Simulation Determinism Analysis

- Status: `PROPOSED / IN_REVIEW / NOT_STARTED`
- Date: 2026-08-13
- Gate: `SIM-DETERMINISM-01`
- Coordination ID: `OTV2-GLOBAL-ARCHITECTURE`
- Scope: paper-only architecture analysis; no runtime, combat, AI, script, DDL or production implementation is authorized.

## 1. Purpose

`SIM-DETERMINISM-01` closes the cross-domain deterministic-simulation semantics deliberately left after FND-03, GAME-CHAR, GAME-ITEM, DUR-03 and DUR-04.

FND-03 already defines **who may mutate authoritative runtime state and in what owner-local order**. DUR-03 defines exact item/currency/value conservation and durable transaction authority. DUR-04 defines authoritative scripts as bounded deterministic proposal components. SIM therefore does not invent a scheduler, database owner, script sandbox or global order.

The question for this gate is narrower:

> Given one canonical authoritative starting state, one accepted owner-local normalized input order and one exact semantic revision/profile set, what numeric, random, time/order and replay semantics guarantee the same normalized authoritative result and make divergence localizable?

Determinism here explicitly does **not** mean recreating live OS thread interleaving, socket wake-up order, CPU scheduling, worker placement or non-semantic wall-clock jitter.

## 2. Consumed accepted authority

### FND-03

SIM must preserve:

- one logical authoritative mutation owner per Channel/Instance scope;
- one `RuntimeExecutionOrdinal` scoped to `(semantic runtime scope, scope ownership generation)`;
- no global runtime total order;
- FND-02 per-GameSession CommandId ordering;
- normalized commands/timers/control/handoff/service/worker/recovery inputs entering one owner resolution boundary;
- separation of wall clock, process-local monotonic time and authoritative execution order;
- no mandatory project-global fixed tick;
- stale-generation/revision rejection;
- workers/services as proposal producers whose completions re-enter through normalized authoritative input;
- replay evidence retaining applicable NodeId/process-incarnation attribution plus protocol/ruleset/content/World Bundle/build revisions, while replay correctness itself does not require recreating the original process placement.

### GAME-CHAR-01 and GAME-ITEM-01

GAME-CHAR keeps authoritative progression facts formula-neutral where exact arithmetic does not constrain identity/ownership/migration, while GAME-ITEM requires deterministic modifier contribution ordering. SIM/ruleset therefore own exact arithmetic/rounding machinery, but not unevidenced Global values or item legality.

### DUR-03

DUR-03 conservation is stricter than ordinary formula arithmetic. Exact item/asset quantities and bounded exact non-item value lines cannot be weakened into binary floating/tolerance arithmetic by SIM.

### DUR-04

DUR-04 already freezes guest execution determinism through immutable invocation snapshots, logical time, invocation-local deterministic RNG, stable host-query ordering, deterministic numeric/fuel/resource semantics under `script_execution_profile_revision`, proposal-only mutation and authority-scoped action plans.

SIM owns how script invocations/results compose with core simulation order, semantic revision binding and replay. It does not replace the script execution profile.

### GAME-VISION-01

A deterministic formula is not automatically the correct Reference formula. `UNKNOWN/CONFLICT` Reference behavior remains fail-closed until evidenced or explicitly declared different.

## 3. Core reproducibility invariant

The target semantic invariant is:

```text
same canonical authoritative starting deterministic state
+ same semantic scope/ownership-generation order evidence
+ same ordered normalized authoritative inputs
+ same exact behavior-affecting revision/profile set bound to those inputs
+ same normalized external authoritative facts
=> same normalized authoritative state/result sequence
```

The replay contract retains semantic order and inputs chosen by authority. It does not promise that independent live network/service arrivals choose the same order on another execution.

## 4. `SimulationDeterminismProfileRevision`

A separate versioned semantic compatibility concept is required:

```text
SimulationDeterminismProfileRevision
```

It is not an entity identity, protocol version or replacement for `ruleset_revision`, `content_revision`, `world_policy_revision` or DUR-04 `script_execution_profile_revision`.

It versions cross-cutting deterministic machinery capable of changing authoritative outcome, including:

- numeric semantic-class policy;
- checked overflow/invalid-number defaults;
- rounding vocabulary and stage semantics;
- gameplay RNG algorithm/profile identity and stream-derivation version;
- RNG decision/consumption semantics;
- SIM-owned canonical ordering/tie-break profiles;
- canonical deterministic-state serialization/hash profile;
- supported authoritative-target determinism policy;
- compatibility with DUR-04 script execution profiles.

Changing these semantics under an unchanged SIM profile revision is forbidden.

## 5. Semantic revision binding for long-lived/retryable work

A key review finding is that deterministic retry requires more than stable operation identity.

Every accepted logical occurrence capable of outliving immediate synchronous resolution or being retried must bind the exact behavior-affecting semantic revision set it needs, including as applicable:

- ruleset/formula revision;
- content revision;
- world-policy revision;
- SIM profile revision;
- script artifact/WIT/script execution profile.

Physical retry, reconnect, failover or delayed completion must not silently recalculate the same occurrence under a newer incompatible revision that became active later.

The owning domain either:

1. completes/replays under the originally bound compatible semantics; or
2. fails/reconciles through an explicit version-transition policy.

“Retry using whatever is current now” is rejected.

## 6. Numeric semantic classes

One universal numeric type is rejected.

### 6.1 Exact discrete

Identifiers, counts, ordinals, quantities and intrinsically discrete fields use exact checked integer semantics. Signedness/range are explicit and implicit wrapping is forbidden.

### 6.2 Exact conservation/value

DUR-03 quantity/currency/value conservation remains exact. Binary floating point and tolerance comparison are forbidden for authoritative conservation.

### 6.3 Formula exact/fixed-scale

Rates, percentages, coefficients and formula intermediates should use project-owned integer/fixed-scale/rational semantics where accepted behavior allows. Scale/denominator belongs to the formula/profile; there is no universal project-wide fixed scale.

### 6.4 Deterministic floating

Binary floating point may appear in authoritative logic only when explicitly declared by the owning formula/profile and cross-target evidence proves identical normalized authoritative outcomes.

The profile must define finite-input rules, NaN/infinity handling, conversion/serialization, canonicalization/rounding and relevant SIMD/relaxed-floating policy. Platform-dependent NaN payloads, reassociation-sensitive parallel reductions and tolerance-only branches may not decide authority.

### 6.5 Non-authoritative approximate

Renderer/UI/telemetry/analytics may use approximate floats where those values cannot feed back into authority. Re-entry requires explicit validated conversion into an authoritative semantic class.

## 7. Formula semantics descriptor

Every authoritative formula family affecting combat, progression, item modifiers, eligibility or another gameplay result needs a versioned descriptor/equivalent that declares:

- formula/ruleset/content revision identity;
- input units/classes/ranges;
- internal numeric class;
- operation-order constraints where reassociation changes result;
- explicit rounding points and named rounding modes;
- conversion/clamp/min/max behavior;
- overflow/underflow/divide-by-zero/invalid-value disposition;
- output unit/class/range;
- deterministic edge fixtures;
- Reference evidence status for claimed Reference behavior.

An unchanged formula/ruleset/SIM revision cannot silently change arithmetic semantics through helper/library refactoring.

## 8. Rounding and invalid state

“Round at the end” is insufficient. Rounding happens only at explicit semantic boundaries declared by the owning formula.

Rounding modes are named/versioned, not host-language defaults. No mode is assumed to match Global without evidence.

Absent an explicitly accepted different rule:

- integer overflow/underflow fails before partial authoritative mutation;
- divide-by-zero fails deterministically;
- NaN/infinity in authoritative state is invalid;
- out-of-range conversion fails rather than wrapping/truncating;
- impossible postconditions trigger fail-closed invariant handling;
- debug/release or supported-target build mode cannot change the result/disposition.

## 9. Randomness classes and ownership

The architecture separates:

- cryptographic/security randomness — nondeterministic and security-owned;
- authoritative gameplay randomness — deterministic/replayable under SIM;
- presentation randomness — non-authoritative.

Gameplay RNG never generates security credentials/secrets.

A single mutable process-global gameplay RNG stream is rejected. RNG ownership follows named semantic simulation/domain scope and current runtime authority; NodeId, thread, worker and scheduler placement are never RNG identity.

## 10. Purpose-isolated RNG

Unrelated mechanics must not perturb each other's future random sequence merely because one feature adds a draw.

Every authoritative random decision has a stable versioned semantic purpose identity equivalent to `RngPurposeKey`, for example `combat.hit`, `combat.damage_roll`, `loot.table.<key>` or `ai.choice.<key>`.

Source-code line number, pointer identity and unordered-container position are invalid purpose identities.

Two implementation models remain architecture-compatible:

1. keyed/counter-style decision addressing using stable logical decision identity + draw index;
2. isolated stateful substreams with explicit checkpoint/replay state.

The concrete RNG library/algorithm is not frozen here, but algorithm/profile/parameters/stream derivation cannot change under one SIM profile and require cross-target fixtures before implementation acceptance.

## 11. Retry-stable random decisions

The same idempotently retried logical occurrence must not receive a new random result because physical execution was retried, ownership moved or a newer semantic revision became active.

Random-decision identity derives from a stable owning semantic occurrence such as CommandRef, OperationId/TransactionId, stable timer/event occurrence identity or another accepted parent occurrence plus stable purpose path.

Transient `RuntimeExecutionOrdinal` may be retained as order evidence but cannot alone force a new random result for the same retry-safe occurrence.

The random decision uses the exact semantic revision set bound to the occurrence under section 5.

## 12. RNG consumption and abort behavior

For stateful streams, RNG advancement is authoritative state.

Therefore:

- rejected work before authoritative acceptance consumes no gameplay RNG;
- speculative workers cannot consume owner RNG state;
- an aborted/retried resolution cannot advance one logical occurrence twice;
- tentative RNG advancement must not survive independently if the owning authoritative resolution aborts before commit;
- committed advancement is checkpoint/replay state;
- draw count/order is deterministic within one purpose stream.

Keyed/counter-style models satisfy the same semantic contract without mutable cursor advancement.

## 13. Deterministic does not mean publicly predictable

Another review finding is that reproducibility for authority/replay does not require client predictability.

Server-controlled gameplay seed/root state may be deterministic and replayable while remaining security-sensitive. Exploit-sensitive loot/spawn/combat derivation must not rely only on public client-known identifiers/timestamps unless an explicit owning product/security contract accepts predictability.

Where secrecy protects game integrity:

- seed/root/substream evidence is access-controlled;
- it is not exposed through client protocol or ordinary telemetry;
- replay retention follows security/privacy policy;
- determinism does not require storing reusable credentials or raw cryptographic-security RNG.

Gameplay RNG remains distinct from cryptographic RNG even if seed material is confidential.

## 14. DUR-04 script RNG composition

Scripts receive invocation-local deterministic RNG through DUR-04, never a mutable process-global/core RNG handle.

SIM owns upstream semantic decision/seed identity and how script proposals enter core FND-03 order. `script_execution_profile_revision` owns deterministic guest-engine execution; `SimulationDeterminismProfileRevision` owns core composition/replay semantics.

Retry of a script-backed logical occurrence remains bound to the same compatible SIM/content/ruleset/script semantic set or an explicit transition/reconciliation policy. Script rejection/retry cannot perturb unrelated core RNG sequences.

## 15. Logical time

SIM preserves:

```text
wall clock != process-local monotonic elapsed time != authoritative execution order
```

There is no mandatory global fixed tick.

Authoritative formulas consume normalized semantic time facts with declared units/precision/counting semantics, not hidden direct system-clock reads.

A cooldown may originate from process-local monotonic scheduling and later enter authority as a due timer input; durable/calendar mechanics consume trusted facts under their owning contracts. Replay uses recorded/injected normalized facts, not the replayer's clock.

## 16. Owner-local order and simultaneous inputs

`RuntimeExecutionOrdinal` remains FND-03 owner-local linearization evidence. SIM creates no global ordinal.

For semantically simultaneous inputs the owning domain chooses one of:

1. commutative/set semantics where order cannot affect result;
2. stable deterministic tie-break key;
3. accepted owner-local RuntimeExecutionOrdinal order retained in replay evidence.

Pointer address, hash-map iteration, worker completion timing and OS thread order are forbidden tie-breakers.

## 17. Cross-source and worker ordering

Live network/service/timer arrival order may legitimately differ between executions. Determinism begins after normalization and accepted owner-local order.

If chosen cross-source order affects gameplay, replay retains the order actually accepted.

Worker/service completions are normalized inputs. Stale generation/revision/input results are rejected before authority. If several valid completions are concurrently eligible and order matters, the domain uses a stable comparator or retains the actual accepted FND-03 order. Worker wake-up order is never gameplay authority.

A delayed result bound to now-incompatible semantics must fail/reconcile explicitly rather than being reinterpreted under current code/config.

## 18. External nondeterminism capture

Every external fact that can alter authoritative simulation crosses a typed normalization boundary first.

Examples include accepted entitlement facts, active revision/configuration facts, admin/operator commands, durable service/query results and real-world calendar facts for mechanics that explicitly depend on them.

Replay retains the normalized behavior-affecting fact/value/identity/revision. It does not requery mutable external systems to rediscover history.

External collections are either canonicalized to semantic order before authority or retain the exact accepted order where order itself is meaningful.

Secrets/reusable credentials are never retained merely to satisfy replay.

## 19. Replay envelope

A deterministic replay interval requires enough evidence to reconstruct one semantic execution slice, including:

- semantic runtime scope identity;
- relevant ownership-generation boundary for interpreting RuntimeExecutionOrdinal;
- original NodeId/process-incarnation evidence where retained for forensic attribution;
- exact authoritative server/build executable revision or artifact identity needed to attribute the implementation that produced the interval;
- applicable `protocol-oteryn` application/protocol revision and compatibility/profile identity;
- exact World Bundle artifact identity/digest plus its behavior-affecting content/map/ruleset/world-policy compatibility revisions as applicable;
- initial canonical deterministic checkpoint/state reference and composite hash;
- ordered normalized inputs and RuntimeExecutionOrdinal evidence;
- applicable CommandRef/OperationId/TransactionId/timer/event/work identities;
- exact semantic revision set bound to long-lived/retryable occurrences where it differs from interval defaults;
- active content/ruleset/world-policy/formula/SIM revisions;
- applicable script artifact/WIT/`script_execution_profile_revision`;
- RNG root/substream/decision evidence required by the chosen model;
- normalized time/calendar/external facts;
- relevant state/domain revisions;
- expected canonical deterministic-state/result hashes at selected cuts.

Original NodeId/process-incarnation evidence is attribution evidence, not simulation identity. Replay does not require recreating original NodeId placement, thread IDs, CPU count, worker placement or OS scheduling jitter.

Executable/protocol/World Bundle provenance remains part of replay evidence even when conforming supported targets are expected to normalize to the same result, because investigators must be able to attribute a historical divergence to the exact implementation/artifact context that produced it.

## 20. Replay authority

Replay/test/investigation is read-only evidence reconstruction.

Replay cannot directly mutate live state, repair a discrepancy, re-emit a historical command as trusted live authority or bypass DUR/ANL correction mechanisms. Confirmed corrections use separately authorized audited domain transactions.

## 21. Future-determining canonical deterministic state

The second review repair clarifies that “current visible gameplay state” is not enough for a deterministic checkpoint/hash.

Canonical deterministic state must include every authoritative fact able to change a future authoritative outcome **without a new external normalized fact**.

That includes, where applicable:

- canonical gameplay/domain state;
- currently active behavior-affecting content/ruleset/world-policy/formula/SIM/script revision/profile set;
- stateful authoritative RNG/substream state/cursors;
- pending accepted timers/operations/continuations whose later resolution is already semantically determined;
- stable occurrence/work identities needed to prevent duplicate/reordered continuation;
- revisions/profiles bound to pending work where they differ from current active semantics;
- relevant domain revisions and authority/fence state when they alter stale-result eligibility;
- deterministic owner-local pending/queue metadata only when it can affect future resolution.

For keyed/counter-style RNG, mutable cursor state may not exist, but replay/profile evidence still retains the root/profile/decision-derivation identity needed to reconstruct future decisions.

Thus two states with identical visible position/HP/items but different active revision, RNG cursor or pending timer are **not** the same deterministic state.

## 22. Canonical hierarchical hashing

Hashes are divergence evidence, not gameplay authority.

Hash input uses versioned canonical semantic serialization independent of memory address, Rust/ABI layout, padding, unordered collection iteration, machine paths and non-authoritative cache/presentation state.

A useful hierarchy is:

```text
scope deterministic-state root
 -> active revision/profile hash
 -> gameplay/domain hashes
 -> RNG/determinism-support hashes
 -> pending/timer/continuation hashes
 -> authority/fence hash when semantically relevant
 -> aggregate/entity/component hashes
```

The exact hash algorithm is an implementation-profile choice, but canonicalization/hash semantics cannot change under an unchanged SIM profile revision.

The hierarchy allows immediate evidence divergence when future-determining state differs even before a later timer/random/input exposes a visible gameplay difference.

## 23. Hash cadence and first-divergence evidence

Production does not have to hash every input.

Candidate profiles may use:

- every-ordinal hashing in deterministic tests;
- periodic checkpoint/domain hashing in soak/replay;
- on-demand higher-detail hashing after a mismatch.

A deterministic comparison reports at least:

1. first mismatching RuntimeExecutionOrdinal/checkpoint cut;
2. normalized input identity/type;
3. active revision/profile set;
4. exact server/build, protocol and World Bundle provenance for the compared interval;
5. first mismatching deterministic-state domain/support/component path;
6. RNG decision/stream evidence;
7. pending timer/operation state where relevant;
8. formula descriptor/rounding boundary;
9. script artifact/execution profile where applicable.

Cadence, retention/storage and detailed evidence budget remain bounded QA/ANL/PERF choices. Evidence never repairs state automatically.

## 24. Supported authoritative targets

For the same replay envelope, every supported authoritative server target must produce the same normalized authoritative outcome.

- exact discrete/conservation/fixed-scale outputs match exactly;
- gameplay RNG decisions/stream outputs match exactly under one SIM profile;
- canonical deterministic-state hashes match exactly;
- authoritative floating is permitted only when cross-target fixtures prove the same normalized outcome;
- a server target unable to satisfy the active profile fails compatibility/readiness rather than silently diverging.

Client prediction/rendering remains non-authoritative and does not have to reproduce every server intermediate bit pattern; reconciliation remains the trust boundary.

## 25. Formula/ruleset compatibility

Formula/ruleset content declares compatibility with required SIM profile capabilities/revisions.

Activation fails closed when required numeric/rounding/RNG semantics are unavailable, implementation semantics changed without a new compatible profile, required cross-target evidence is missing or claimed Reference behavior remains `UNKNOWN/CONFLICT` for an exercised path.

A newer SIM profile does not silently reinterpret historical replay/content/ruleset revisions or pending retryable occurrences bound to older compatible semantics.

## 26. Failure and recovery

On numeric/RNG/determinism invariant failure:

- no partial authoritative mutation commits when the owning resolution can fail/abort;
- unexpected invariant failures follow FND-03 fail-closed handling;
- DUR transactions retain their own commit/rollback authority;
- no implicit clamp/wrap/reseed/reroll is invented;
- diagnostics identify input/profile/formula scope without leaking secrets.

Recovery onto a new NodeId/ownership generation preserves semantic gameplay/RNG/replay state through accepted checkpoint/evidence. Process restart cannot reseed gameplay from current wall clock/entropy merely because placement changed.

## 27. Resource safety

Before implementation acceptance, explicit hard limits/boundary tests must cover applicable replay input count/bytes, replay/checkpoint bytes, RNG purpose/substream state, pending continuation/timer evidence, authored formula expression/depth/operation bounds if applicable, state-hash work/bytes, divergence evidence and deterministic replay/test work budgets.

Missing required limits block implementation and never mean unlimited.

## 28. Security/privacy

Gameplay RNG seed/root/substream evidence must not be exposed to clients where prediction enables abuse. Public-only deterministic derivation is unacceptable for exploit-sensitive outcomes unless explicit product/security policy accepts it.

Replay/divergence artifacts may contain sensitive player/world/security evidence and follow ANL/privacy retention/access/export controls.

Determinism never requires reusable credentials, secret keys or raw security RNG.

## 29. Architecture decision test

### Must decide now? — YES

Semantic numeric/RNG/order/replay boundaries must be frozen before broad combat/AI/progression implementation. Otherwise helper/library/scheduler choices become implicit gameplay behavior and later parity/replay migration becomes expensive.

### Concrete downstream work blocked

- broad combat/damage/healing formula implementation;
- production creature AI decisions using authoritative randomness;
- exact progression/skill arithmetic delegated from GAME-CHAR;
- ruleset formula package freeze;
- deterministic replay/state-hash implementation contract;
- parity-confirmed claims for unresolved formula/rounding behavior.

### Future migration cost if wrong

An incompatible later change can require formula/ruleset version migration, preservation of old RNG algorithms/stream derivation, golden-fixture rebuild, checkpoint/replay schema changes, hash-profile migration, cross-target retesting, durable-progression reconciliation and compatibility bridges for old content/script profiles.

### Superseding evidence

Named evidence that may justify a later explicit supersession includes:

- Reference behavior proving a different numeric/rounding rule;
- cross-target divergence fixtures;
- replay/fault evidence showing current stream/order/state model is insufficient;
- representative performance evidence proving a deterministic profile cannot meet required budgets;
- changed product requirements requiring a different deterministic simulation model;
- security evidence showing current seed/derivation exposes exploitably predictable outcomes.

A library preference or benchmark alone cannot weaken exact conservation, authority or replay correctness.

### Deliberately not decided

- Rust numeric/RNG/hash libraries;
- exact gameplay RNG algorithm;
- exact per-formula fixed-point scale;
- exact combat/XP/skill/item formulas or Global values;
- global fixed tick rate;
- scheduler implementation/weights;
- worker/thread counts or CPU affinity;
- concrete state-hash algorithm until implementation artifact review;
- replay storage backend/retention;
- production tracing/analytics backend.

## 30. Review reconciliation and recommendation

The candidate analysis and contract are aligned after the initial draft and four material repair/reconciliation cycles:

1. initial draft established profiled arithmetic/RNG/time/order/replay semantics;
2. review repair 1 bound retryable logical occurrences to exact semantic revisions and separated deterministic replayability from public RNG predictability;
3. review repair 2 expanded canonical deterministic state/hash scope to active semantic revisions, RNG state and pending future-determining authority state;
4. review repair 3 reconciled the analysis with the normative retry/hash model so the candidate sources no longer described different semantics;
5. owner-authorized repair 4 restored the FND-03 executable/protocol/World Bundle replay provenance omitted from the frozen candidate and explicitly separated original NodeId/process-incarnation forensic attribution from any requirement to recreate original placement.

Recommended architecture:

```text
FND-03 owner/order remains authority
+ exact semantic revisions bind retryable occurrences
+ numeric semantics are explicit/versioned per formula class
+ random decisions are purpose-isolated, retry-stable and not necessarily public
+ external nondeterminism becomes normalized input
+ replay reproduces accepted semantic order, not OS scheduling
+ replay retains exact server/build, protocol and World Bundle provenance without recreating original process placement
+ canonical deterministic state includes future-determining RNG/revision/pending state
+ hierarchical hashes localize the first divergence
+ supported authoritative targets agree on normalized outcome
+ DUR-04 script determinism nests under, not beside, core SIM semantics
```

This allows evidenced Reference quirks to live in explicit formula/ruleset revisions while keeping the Oteryn engine deterministic, testable, migration-safe and resistant to accidental scheduling or RNG coupling.
