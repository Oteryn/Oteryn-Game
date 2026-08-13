# SIM-DETERMINISM-01 — Authoritative Simulation Determinism Contract

- Status: `PROPOSED / IN_REVIEW / NOT_STARTED`
- Date: 2026-08-13
- Gate: `SIM-DETERMINISM-01`
- Scope: paper-only architecture contract.
- Runtime/combat/AI/script implementation authority: **NONE**.
- PostgreSQL DDL/migration authority: **NONE**.
- Production authority: **NONE**.

## 1. Purpose

This contract defines deterministic execution semantics required before broad authoritative combat, AI, progression and formula implementation.

It refines FND-03 runtime ordering/RNG/replay requirements and composes with DUR-04 script determinism. It MUST NOT replace FND-03 authority/order, DUR-03 exact value conservation or ruleset/content ownership of actual gameplay formulas.

## 2. Core reproducibility invariant

For one semantic runtime scope:

```text
same canonical authoritative starting state
+ same ownership-generation/order evidence
+ same ordered normalized authoritative inputs
+ same exact semantic revision set bound to those inputs
+ same SimulationDeterminismProfileRevision
+ same applicable script artifact/WIT/execution profiles
+ same normalized external authoritative facts
=> same normalized authoritative state/result sequence
```

Thread IDs, CPU count, worker placement, OS scheduling, socket wake-up timing and non-semantic wall-clock jitter MUST NOT be replay prerequisites.

## 3. FND-03 authority remains binding

`RuntimeExecutionOrdinal` remains owner-local to `(semantic scope, ownership generation)` and remains the authoritative runtime input-linearization evidence.

SIM MUST NOT create a second runtime commit counter or global total order.

FND-02 CommandRef/order, domain revisions, EventId, OperationId and TransactionId remain distinct identities.

## 4. Simulation determinism profile

Every authoritative simulation execution MUST bind one immutable/versioned:

```text
SimulationDeterminismProfileRevision
```

The profile MUST identify cross-cutting deterministic semantics that can alter normalized authoritative outcome, including:

- numeric semantic-class policy;
- checked overflow/invalid-number defaults;
- rounding vocabulary/semantics;
- authoritative gameplay RNG algorithm/profile identity and stream-derivation version;
- RNG decision/consumption semantics;
- SIM-owned tie-break/canonical-order profiles;
- canonical state-hash serialization/profile;
- supported authoritative-target policy;
- compatibility with applicable DUR-04 `script_execution_profile_revision`.

An implementation MUST NOT change those semantics under an unchanged profile revision.

The SIM profile MUST NOT replace `ruleset_revision`, `content_revision`, `world_policy_revision` or DUR-04 script execution profile identity.

## 5. Semantic revision binding

Every accepted authoritative occurrence that can outlive immediate synchronous resolution or be retried MUST bind the exact behavior-affecting semantic revision set needed to reproduce it, including as applicable:

- ruleset/formula revision;
- content revision;
- world-policy revision;
- `SimulationDeterminismProfileRevision`;
- script artifact/WIT/`script_execution_profile_revision`.

A physical retry, reconnect, failover or delayed completion MUST NOT silently re-evaluate the same logical occurrence under a newer incompatible revision merely because activation advanced in the meantime.

The owning domain MUST either:

1. complete/replay the occurrence under its originally bound compatible semantics; or
2. fail/reconcile through an explicit version-transition policy.

It MUST NOT simply reroll/recalculate the same occurrence under the new revision.

## 6. Numeric semantic classes

Every authoritative numeric field/formula MUST declare a semantic class.

### 6.1 Exact discrete

Identifiers, counts, ordinals, quantities and intrinsically discrete values MUST use exact checked integer semantics. Implicit overflow wrapping is forbidden.

### 6.2 Exact conservation/value

DUR-03 item/currency/value conservation quantities MUST remain exact. Binary floating point and tolerance comparison MUST NOT be authoritative conservation arithmetic.

### 6.3 Formula exact/fixed-scale

Rates, coefficients, percentages and formula intermediates SHOULD use explicit integer/fixed-scale/rational semantics where accepted behavior allows it. No one global scale is required; scale/denominator is formula/profile-owned.

### 6.4 Deterministic floating

Authoritative binary floating point MAY be used only when explicitly declared by the owning formula/profile and cross-target evidence proves identical normalized authoritative outcome.

The profile MUST define finite-input rules, NaN/infinity handling, conversion/serialization, rounding/canonicalization and relevant SIMD/floating feature policy.

Platform-dependent NaN payloads, reassociation-sensitive parallel reductions or tolerance-only branches MUST NOT decide authoritative gameplay.

### 6.5 Non-authoritative approximate

Renderer/UI/telemetry/analytics MAY use approximate floating values where they cannot feed back into authoritative gameplay. Re-entry into authority requires validated conversion to an authoritative semantic class.

## 7. Formula semantics descriptor

Every authoritative formula family affecting combat, progression, item modifiers, eligibility or other gameplay state MUST have a versioned descriptor/equivalent defining at least:

- formula/ruleset/content revision identity;
- input units/classes/ranges;
- internal numeric class;
- operation/order constraints where reassociation changes output;
- explicit rounding points and named modes;
- conversion/clamp/min/max behavior;
- overflow/underflow/divide-by-zero/invalid-value disposition;
- output unit/class/range;
- deterministic edge fixtures;
- Reference evidence status when claiming Reference behavior.

An unchanged formula/ruleset/SIM revision MUST NOT silently change arithmetic semantics.

## 8. Rounding and invalid numeric state

Rounding MUST occur only at explicit semantic boundaries declared by the formula profile. Rounding modes MUST be named/versioned rather than inherited from language/library defaults.

Unless an explicitly accepted formula says otherwise:

- integer overflow/underflow MUST fail before partial authoritative mutation;
- division by zero MUST fail deterministically;
- NaN/infinity in authoritative state MUST fail validation;
- out-of-range conversion MUST fail rather than wrap/truncate;
- impossible postconditions MUST trigger fail-closed invariant handling.

Debug/release or supported-target build mode MUST NOT change the result/disposition.

No rounding rule is assumed to match the Reference target without evidence.

## 9. Randomness classes

The implementation MUST separate:

- cryptographic/security randomness — nondeterministic, security-owned;
- authoritative gameplay randomness — deterministic/replayable under SIM;
- presentation randomness — non-authoritative.

Gameplay RNG MUST NOT generate credentials/secrets. A process-global mutable gameplay RNG stream is forbidden.

## 10. RNG semantic ownership and seed protection

Every authoritative random decision MUST belong to a named semantic scope/domain and stable purpose identity.

RNG identity MUST NOT depend on NodeId, OS thread, worker identity, pointer address, source-code line number or unordered-container position.

Changing process placement MUST NOT reseed gameplay semantics.

Server-controlled gameplay seed/root state MAY be deterministic and durable/replayable while still being security-sensitive. For exploit-sensitive decisions, seed/derivation MUST NOT be based only on public client-known values unless the owning product/security contract explicitly accepts predictability.

If secrecy is required to prevent loot/spawn/combat prediction, seed/root/substream evidence MUST be access-controlled and MUST NOT be exposed to clients or ordinary telemetry. Replay retention of such evidence follows security/privacy policy.

## 11. Purpose isolation

Unrelated mechanics MUST NOT share accidental draw-order coupling.

Every random decision MUST have a stable purpose/effect path equivalent to a versioned `RngPurposeKey`/semantic purpose identity.

An added unrelated draw in one purpose MUST NOT silently perturb another purpose's future authoritative sequence.

The implementation MAY satisfy this through:

- keyed/counter-style decisions using stable decision identity + draw index; or
- isolated stateful substreams with explicit authoritative checkpoint/replay state.

The chosen RNG algorithm/profile/parameters/stream derivation MUST be immutable under the active SIM profile and proven by cross-target fixtures before implementation acceptance.

## 12. Retry-stable random decisions

The same idempotently retried logical authoritative occurrence MUST NOT receive a new random outcome merely because physical execution was retried, runtime ownership moved or a newer content/ruleset/SIM revision became active.

Random decision identity MUST derive from stable owning semantic identity such as CommandRef, OperationId/TransactionId, stable timer/event occurrence identity or another accepted parent occurrence plus stable purpose path.

The random decision MUST use the semantic revision set bound under section 5 for that logical occurrence.

Transient RuntimeExecutionOrdinal/generation MAY be retained as order evidence but MUST NOT alone force a new random result for the same retry-safe semantic occurrence.

## 13. Stateful RNG advancement

When a stateful RNG/substream is used, stream advancement is authoritative state.

Therefore:

- work rejected before authoritative acceptance consumes no gameplay RNG;
- speculative workers consume no owner RNG state;
- abort/retry cannot advance one logical occurrence twice;
- if the owning resolution aborts before authoritative commit, tentative RNG advancement MUST NOT survive independently;
- committed advancement is checkpoint/replay state;
- draw count/order is deterministic for the purpose stream.

## 14. Script RNG composition

DUR-04 remains authoritative for guest execution behavior under `script_execution_profile_revision`.

SIM owns how an authoritative script invocation receives stable semantic random-decision/seed identity and how its proposal enters core FND-03 execution order.

Scripts MUST NOT receive a mutable core/process-global RNG handle. Script rejection/retry MUST NOT perturb unrelated core RNG sequences.

A retried script-backed logical occurrence MUST remain bound to the same compatible SIM/content/ruleset/script semantic revision set or use an explicit transition/reconciliation policy.

## 15. Time semantics

The implementation MUST preserve:

```text
wall clock != monotonic elapsed time != authoritative execution order
```

No universal fixed global tick is required.

An authoritative formula consuming time MUST receive an explicit normalized semantic time value with declared unit/precision/counting semantics. Direct system-clock reads are forbidden as hidden formula inputs.

Replay MUST use recorded/injected normalized time/calendar facts, never the replayer's current clock.

## 16. Simultaneous/conflicting input policy

For semantically simultaneous inputs, the owning domain MUST define one of:

1. commutative/set semantics making order irrelevant;
2. stable deterministic tie-break key;
3. FND-03 accepted RuntimeExecutionOrdinal order retained in replay evidence.

Pointer order, hash-map iteration, OS thread order and worker wake-up timing are forbidden tie-breakers.

## 17. Cross-session/cross-source order

SIM does not promise that independent live network/service arrivals occur in the same order on separate runs.

Determinism begins after normalization and FND-03 owner acceptance.

When chosen cross-source order affects gameplay, replay evidence MUST retain that accepted order.

## 18. Worker/service results

Auxiliary work results MUST enter as new normalized FND-03 inputs with generation/revision/input evidence sufficient for stale-result rejection.

If concurrently eligible results can produce different gameplay outcomes, the owning domain MUST use a stable comparator or retain the accepted FND-03 order for replay.

Worker wake-up order MUST NOT be gameplay authority.

A delayed result whose bound semantic revision is no longer compatible MUST fail/reconcile explicitly rather than being reinterpreted by current code/config.

## 19. External nondeterminism

Any external fact affecting authoritative gameplay MUST be normalized into a typed authoritative input/fact before use.

Replay MUST retain the accepted behavior-affecting value, identity and revision needed to reconstruct behavior, subject to privacy/security rules.

Runtime replay MUST NOT requery mutable external systems to rediscover historical gameplay facts.

Collections returned by an external source MUST be canonicalized to a defined semantic order before authority or have the exact accepted order retained as replay evidence where order is itself meaningful.

Secrets/reusable credentials/security randomness MUST NOT be stored merely to satisfy replay.

## 20. Replay envelope

A deterministic replay interval MUST identify at least:

- semantic runtime scope;
- relevant ownership-generation boundary;
- original NodeId/process-incarnation evidence where retained for forensic attribution;
- exact authoritative server/build executable revision or artifact identity needed to attribute the implementation that produced the interval;
- applicable `protocol-oteryn` application/protocol revision and compatibility/profile identity;
- exact World Bundle artifact identity/digest plus its behavior-affecting content/map/ruleset/world-policy compatibility revisions as applicable;
- initial canonical deterministic state/checkpoint reference + composite hash;
- ordered normalized inputs and RuntimeExecutionOrdinal evidence;
- applicable CommandRef/OperationId/TransactionId/timer/event/work identities;
- the exact semantic revision set bound to each long-lived/retryable occurrence where it differs from the interval default;
- content/ruleset/world-policy revisions;
- active `SimulationDeterminismProfileRevision`;
- applicable formula profile revisions;
- applicable script artifact/WIT/`script_execution_profile_revision`;
- RNG root/substream/decision evidence required by the selected model;
- normalized logical-time/calendar/external facts;
- relevant state-domain revisions;
- expected canonical deterministic state/result hashes at selected cuts.

Original NodeId/process-incarnation evidence is attribution evidence, not simulation identity. Replay MUST NOT require recreating the original NodeId placement, thread IDs, CPU count, worker placement or wall-clock scheduling jitter.

The replay envelope MUST retain executable/protocol/World Bundle provenance even when two conforming supported targets are expected to normalize to the same result; otherwise a historical divergence after a build, protocol or bundle change cannot be attributed to the implementation/artifact context that produced it.

## 21. Replay is not authority

Replay/testing/investigation is read-only evidence reconstruction.

A replay result MUST NOT directly mutate live gameplay, repair durable state, reissue historical commands as trusted live authority or bypass DUR/ANL correction mechanisms.

## 22. Canonical deterministic state and hashing

A deterministic checkpoint/hash MUST represent all **authoritative state that can change a future authoritative outcome without a new external normalized fact**.

That includes, as applicable:

- canonical gameplay/domain state;
- the currently active behavior-affecting content/ruleset/world-policy/formula/SIM/script revision/profile set;
- stateful authoritative RNG/substream state/cursors;
- pending accepted timer/operation/continuation state whose later resolution is already semantically determined;
- stable occurrence/work identities required to prevent duplicate/reordered continuation;
- revision/profile identities bound to pending work where they differ from the currently active set;
- relevant domain revisions and authority/fence state where they affect stale-result eligibility;
- deterministic owner-local queue/pending metadata only when it can alter future authoritative resolution.

For keyed/counter-style RNG without mutable stream state, the relevant root/profile/decision-derivation identity MUST still be represented by the replay envelope/profile so future decisions remain reconstructable.

A state hash MUST NOT omit active/bound revision state, future-determining RNG state or pending semantic work merely because current visible gameplay fields are identical.

Hash input MUST use versioned canonical semantic serialization independent of memory layout, padding, addresses, unordered collection iteration and non-authoritative cache/presentation state.

Hashing SHOULD be hierarchical, separating concerns where useful:

```text
scope deterministic-state root
 -> active revision/profile hash
 -> gameplay/domain hashes
 -> RNG/determinism-support hashes
 -> pending/timer/continuation hashes
 -> authority/fence hash when semantically relevant
 -> aggregate/entity/component hashes
```

This allows visually identical states with different active semantics, RNG cursor or pending work to diverge immediately in evidence rather than only after a later input/random/timer outcome.

Concrete hash algorithm/canonicalization belongs to the SIM profile implementation artifact and MUST NOT silently change under one profile revision.

State hashes are evidence, not gameplay authority.

## 23. Hash cadence and divergence evidence

This contract does not require production hashing after every input.

Test/replay profiles MAY hash every ordinal; production/soak MAY hash checkpoint/domain cuts; incident tooling MAY increase detail on demand.

A deterministic comparison MUST be able to identify at least:

- first mismatching RuntimeExecutionOrdinal/checkpoint cut;
- normalized input identity/type;
- active semantic revision/profile set;
- exact server/build, protocol and World Bundle provenance for the compared interval;
- first mismatching deterministic-state domain/support/component hash path;
- relevant RNG decision/stream evidence;
- pending timer/operation state when relevant;
- formula descriptor/rounding boundary;
- script artifact/execution profile when applicable.

Cadence, retention and storage remain bounded QA/ANL/PERF choices. Divergence evidence MUST NOT autonomously repair state.

## 24. Supported authoritative targets

For the same replay envelope, every supported authoritative server target MUST produce the same normalized authoritative outcome.

Exact discrete/conservation/fixed-scale outputs, RNG decisions and canonical deterministic-state hashes MUST match exactly.

Authoritative floating implementations are acceptable only if cross-target fixtures prove the same normalized authoritative result.

A target unable to satisfy the current SIM profile MUST fail compatibility/readiness rather than silently diverge.

Client prediction/rendering is non-authoritative and need not match every server intermediate bit pattern; reconciliation remains the trust boundary.

## 25. Formula/ruleset compatibility

Formula/ruleset content MUST declare compatibility with required SIM profile capabilities/revisions.

Activation MUST fail closed if:

- required numeric/rounding/RNG semantics are unavailable;
- implementation semantics changed without a compatible new profile;
- claimed Reference behavior is still `UNKNOWN/CONFLICT` for the exercised path;
- required cross-target deterministic evidence is missing.

A newer SIM profile MUST NOT silently reinterpret old replay/content/ruleset revisions or pending retryable occurrences bound to older compatible semantics.

## 26. Failure semantics

On numeric/RNG/determinism invariant failure:

- no partial authoritative mutation may be committed where the owning resolution can fail/abort;
- unexpected invariant failure follows FND-03 fail-closed handling;
- DUR transactions retain their own commit/rollback semantics;
- no implicit clamp/wrap/reseed/reroll is invented;
- diagnostics identify input/profile/formula scope without exposing secrets.

Process recovery MUST NOT reseed gameplay randomness from current wall clock/entropy merely because NodeId/ownership generation changed.

## 27. Resource bounds

Before implementation acceptance, Resource Limits Registry/equivalent owning limits MUST cover applicable replay/state-hash/RNG/formula resources, including replay input/bytes, checkpoint/replay bytes, RNG purpose/substream state, pending continuation/timer evidence, authored formula expression/depth/operations where applicable, state-hash work/bytes, divergence evidence and deterministic replay/test work budget.

Missing required limits block implementation; they do not mean unlimited.

## 28. Security/privacy

Gameplay RNG seed/root/substream state MUST NOT be exposed to clients where prediction enables abuse.

A deterministic scheme that is trivially derivable from public identifiers/timestamps is unacceptable for exploit-sensitive decisions unless an explicit product/security contract chooses public predictability.

Replay/divergence artifacts are potentially sensitive player/world/security evidence and MUST use ANL/privacy access/retention/export controls.

Determinism MUST NOT require retention of reusable credentials, secret keys or raw security RNG.

## 29. Architecture decision timing

The numeric/RNG/order/replay semantic boundary MUST be accepted before broad combat/AI/progression formula implementation.

Concrete Rust libraries, exact gameplay RNG algorithm, per-formula fixed scale, formula values, global tick rate, scheduler implementation, worker counts, replay storage backend and production hash cadence are deliberately deferred.

A later explicit contract may supersede a SIM rule only with named evidence such as Reference behavior, cross-target divergence, replay/fault evidence, representative deterministic-profile performance evidence, security findings or changed product requirements.

## 30. Required implementation evidence

A future implementation MUST prove at least:

1. same replay envelope yields identical canonical final deterministic-state/result hashes across supported authoritative server targets;
2. exact arithmetic overflow/divide-by-zero/out-of-range fixtures fail identically across build modes/targets;
3. each formula's named rounding points/modes match fixtures;
4. changing an unrelated RNG purpose draw does not perturb another purpose's sequence;
5. retry/failover of the same logical occurrence preserves its random decision and originally bound compatible semantic revision set;
6. activation of a newer revision while an occurrence is pending cannot silently cause that occurrence to be recalculated under new semantics;
7. rejected/speculative work does not advance owner RNG state;
8. stateful stream advancement aborts with a failed resolution and survives committed checkpoint/recovery exactly once;
9. process restart does not wall-clock-reseed gameplay RNG;
10. exploit-sensitive RNG cannot be reconstructed from public-only inputs under the selected seed/derivation policy unless predictability is explicitly accepted;
11. two states with identical visible gameplay fields but different active revision set/RNG cursor/pending timer/bound revision produce different deterministic-state evidence hashes before future outcomes diverge;
12. equal-deadline/simultaneous deterministic tie-break fixtures are stable where defined;
13. recorded cross-source order replays concurrency-sensitive result under different test thread scheduling;
14. stale worker/service results are rejected and cannot reorder authority;
15. replay uses captured/canonicalized external facts rather than current mutable external-system responses;
16. hierarchical hashes localize a seeded divergence to the first mismatching revision/domain/support/component cut;
17. authoritative floating fixture, where allowed, normalizes identically across supported targets;
18. incompatible SIM/formula/script execution profile activation fails closed;
19. script proposal replay with the same bound semantic revision set produces the same normalized accepted/rejected result;
20. replay tooling cannot mutate live authority;
21. required replay/hash/RNG/formula/pending-state limits have explicit boundary tests;
22. replay evidence retains exact server/build, protocol and World Bundle provenance across implementation/artifact changes while replay correctness remains independent of recreating original process placement.

## 31. Non-authority statements

Acceptance of SIM-DETERMINISM-01 does NOT authorize Rust simulation implementation, RNG/numeric/hash dependency adoption, combat/AI/progression/script implementation, exact Reference/gameplay/balance formulas, protocol changes, PostgreSQL DDL/migrations, Platform writes, live/production replay correction or production deployment/configuration.

## 32. Lifecycle rule

This document remains a candidate while its delivery PR is open.

`SIM-DETERMINISM-01` may become `ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED` only after reviewed delivery merge plus a separate lifecycle closeout that reconciles maintained programme status/register/horizon/index/handoff and archives/releases the active task.
