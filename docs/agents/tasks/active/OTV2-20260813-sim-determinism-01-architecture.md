# OTV2-20260813-sim-determinism-01-architecture

```yaml
task_id: OTV2-20260813-sim-determinism-01-architecture
title: SIM-DETERMINISM-01 authoritative simulation determinism architecture
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: agent/otv2-20260813-sim-determinism-01-architecture
pr: 214
base_sha: 27c313a0c6032f0433ad9598c3cf53e4f0179813
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: architecture-coordinator/current-session
created_at: 2026-08-13T00:55:00+02:00
updated_at: 2026-08-13T08:57:00+02:00
execution_budget_minutes: 90
large_budget_reason: Cross-cutting paper-only determinism gate spans authoritative arithmetic, RNG ownership, logical time/order, replay inputs and cross-target evidence without authorizing runtime implementation.
owned_paths:
  - docs/agents/tasks/active/OTV2-20260813-sim-determinism-01-architecture.md
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
blocks:
  - broad combat and AI formula freeze
  - deterministic replay implementation contract
  - PARITY_CONFIRMED claims for unresolved authoritative Character arithmetic
cross_repository_coordination_id: OTV2-GLOBAL-ARCHITECTURE
external_repositories: []
```

## Outcome

Produce one bounded paper-only `SIM-DETERMINISM-01 — Authoritative Simulation Determinism Contract` refining, not replacing, accepted FND-03 runtime order/RNG/replay foundations and DUR-04 script execution determinism.

The candidate freezes cross-domain deterministic semantics needed before broad combat/AI/progression formula implementation: authoritative numeric semantic classes, rounding/overflow/invalid numeric state, exact semantic revision binding, RNG stream identity/ownership/consumption and anti-prediction policy, logical time and owner-local order/tie-break rules, normalized external facts, replay envelope, future-determining canonical deterministic state/hash/divergence evidence, supported-target determinism and formula/ruleset compatibility.

No Rust/runtime/client/combat/AI/script implementation, PostgreSQL DDL/migrations, Platform write, production configuration/deployment or exact formula/balance-value acceptance is authorized.

Maintained programme/register/horizon/index/handoff files remain outside this delivery PR and may be promoted only by a separate lifecycle closeout after reviewed delivery merge.

## Verified source and ownership facts

- `PROVEN`: post-DUR-04-closeout `main@27c313a0c6032f0433ad9598c3cf53e4f0179813` selects `SIM-DETERMINISM-01 = PROPOSED / PLANNED / NOT_STARTED` as the next bounded paper-only architecture action.
- `PROVEN`: no active SIM task and no competing open SIM delivery PR existed at task start.
- `PROVEN`: FND-03 already owns one logical authoritative writer per Channel/Instance, owner-scoped `RuntimeExecutionOrdinal`, wall/monotonic/execution-order separation, no universal fixed tick, deterministic gameplay RNG/replay evidence and stale-result rejection. SIM cannot invent a competing scheduler or global order.
- `PROVEN`: FND-03 replay evidence retains applicable NodeId/process-incarnation attribution and protocol/ruleset/content/World Bundle/build revisions; replay correctness does not require recreating original process placement.
- `PROVEN`: GAME-CHAR keeps authoritative progression facts formula-neutral and delegates exact arithmetic/rounding to ruleset/SIM parity work where identity/ownership/migration is unaffected.
- `PROVEN`: GAME-ITEM requires deterministic modifier contribution ordering while SIM/ruleset own arithmetic/rounding.
- `PROVEN`: DUR-03 exact conservation/value arithmetic cannot be weakened by SIM formula choices.
- `PROVEN`: DUR-04 owns guest/script execution determinism through immutable invocation snapshots, invocation-local deterministic RNG, stable query order, proposal-only mutation and `script_execution_profile_revision`; SIM owns composition with core simulation/replay, not guest engine internals.
- `PROVEN`: GAME-VISION Reference claims remain evidence-gated; deterministic implementation convenience cannot fill `UNKNOWN/CONFLICT` Global behavior.
- `PROVEN`: current governance forbids Codex/OpenAI API/owner-funded AI use without exact current-task permission. The PR #212 owner review override is not standing permission and has not been used for #214.
- `PROVEN`: independent exact-head review `4924203877` on frozen head `5dc628f32ca4573725bcb4a42c3a7702536d7f35` confirmed one material P1: SIM replay evidence omitted server/build, protocol and exact World Bundle provenance required by FND-03.
- `PROVEN`: on 2026-08-13 the owner explicitly instructed `wykonaj to wszystko` in direct response to the P1 audit, authorizing one additional bounded repair cycle for this finding; this does not authorize Codex/OpenAI API/owner-funded AI use or waive the independent-review gate.

## Candidate semantic closure

- [x] Core reproducibility invariant is canonical starting deterministic state + exact owner-local normalized input order + exact semantic revision/profile set + normalized external facts, not OS/thread/network timing.
- [x] `SimulationDeterminismProfileRevision` versions cross-cutting numeric/RNG/tie-break/hash/target semantics without replacing ruleset/content/world-policy or DUR-04 script execution profiles.
- [x] Long-lived/retryable logical occurrences bind exact behavior-affecting semantics and cannot silently recalculate under a newer revision after retry/reconnect/failover/delayed completion.
- [x] Numeric classes separate exact discrete, DUR-03 exact conservation/value, formula exact/fixed-scale, explicitly proven deterministic floating and non-authoritative approximate values.
- [x] Formula descriptors own units, representation, operation order, rounding boundaries/modes, invalid-value disposition, outputs and Reference evidence state.
- [x] Gameplay RNG is deterministic/replayable but separate from security/presentation randomness; one process-global mutable gameplay RNG is forbidden.
- [x] Stable semantic RNG purposes isolate unrelated mechanics; keyed/counter decisions or isolated checkpointed substreams remain possible implementations.
- [x] Retry-stable random decisions derive from stable logical occurrence identity and bound revisions rather than process placement or transient order alone.
- [x] Exploit-sensitive deterministic seed/root/substream evidence can remain confidential; public-only derivation is forbidden unless predictability is explicitly accepted.
- [x] Stateful RNG advancement is authoritative state and cannot survive aborted resolution independently or be consumed by speculative workers.
- [x] Wall clock, monotonic elapsed time and authoritative execution order remain distinct; no universal fixed tick is introduced.
- [x] Simultaneous/conflicting inputs use commutative semantics, stable tie-break or retained FND-03 RuntimeExecutionOrdinal; no pointer/hash-map/thread ordering authority.
- [x] External nondeterminism must become typed normalized authority input/fact with canonicalized or retained meaningful collection order.
- [x] Replay envelope binds initial deterministic state, accepted input order/identities, exact server/build + protocol + World Bundle provenance, semantic revisions/profiles, formula/script identities, RNG evidence and normalized external/time facts; original NodeId/process evidence is forensic attribution, not a replay-placement prerequisite.
- [x] Canonical deterministic state includes all future-determining authority state: active revision/profile set, gameplay state, RNG cursors/state, pending timers/operations/continuations, occurrence identities and semantically relevant fence/revision metadata.
- [x] Hierarchical deterministic-state hashing/localization is evidence only and cannot repair live state.
- [x] Supported authoritative server targets must produce identical normalized outcomes; authoritative floating requires cross-target proof.
- [x] Replay/investigation remains read-only and cannot mutate live authority.
- [x] Resource/security/privacy boundaries fail closed.
- [x] Architecture decision test records must-decide timing, concrete blocked work, migration cost, superseding evidence and deliberate deferrals.

## Repair history

### Repair cycle 1 — retry revision binding and RNG anti-prediction

Adversarial self-review found that retry/delayed completion could otherwise be re-evaluated under a newer semantic revision and deterministic RNG could be replayable yet trivially predictable from public facts. The repair binds long-lived/retryable occurrences to their exact behavior-affecting revision set and requires explicit transition/reconciliation rather than silent recalculation. Exploit-sensitive seed/root/substream evidence remains server-controlled/access-controlled and public-only derivation is rejected unless explicitly accepted.

### Repair cycle 2 — future-determining deterministic state

Adversarial self-review found that visible gameplay state alone is insufficient for divergence hashing: equal position/HP with a different active revision, RNG cursor or pending timer can yield a different next authoritative result. The contract therefore includes active behavior revisions, RNG state, pending accepted work, occurrence identities and semantically relevant fence/revision metadata in canonical deterministic state and hierarchical hashes.

### Repair cycle 3 — analysis/contract reconciliation

Final pre-freeze review found the analysis still described the pre-repair retry/hash model while the normative contract already required semantic revision binding, anti-prediction RNG and future-determining deterministic-state hashing. The analysis was rewritten/reconciled to the reviewed contract so the two candidate architecture sources no longer provide different interpretations.

### Repair cycle 4 — executable/protocol/World Bundle replay provenance

Independent exact-head review `4924203877` confirmed that the frozen replay envelope had dropped FND-03-required executable provenance. The owner explicitly authorized one additional bounded repair cycle. The analysis and normative contract now retain exact authoritative server/build identity, applicable `protocol-oteryn` revision/profile identity and exact World Bundle artifact identity/digest with behavior-affecting compatibility revisions. Original NodeId/process-incarnation evidence may remain for forensic attribution, while replay correctness remains independent of recreating original process placement.

Repair budget used: **`4/4`**, where cycle 4 is the one owner-authorized exception granted on 2026-08-13 for the confirmed P1. Any further material finding must block/rotate unless the owner explicitly authorizes another repair cycle.

## Validation

### Focused

- live main/ownership preflight: `PASS`
- accepted-source audit: `PASS`
- analysis/contract drafting: `PASS`
- material repair cycles: `4/4`
- exact changed-file scope: expected task + analysis + contract only
- previous independent exact-head review `4924203877`: `BLOCK/P1` on superseded head `5dc628f32ca4573725bcb4a42c3a7702536d7f35`; finding repaired in cycle 4
- terminal full-diff exact-head self-review: pending after repair-4 freeze

### Component/integration/runtime E2E

- `NOT_APPLICABLE` — paper-only architecture task

### Review and owner-funded AI

- current task has unusual cross-cutting deterministic correctness/replay/RNG complexity; a genuinely independent second review remains mandatory under root risk policy on the new exact final head.
- Codex/OpenAI API/paid AI reviewer: **NOT AUTHORIZED / NOT INVOKED BY THIS REPAIR**.
- prior owner override for PR #212: **NOT INHERITED**.
- owner authorization for repair cycle 4 does **not** waive independent review.
- implementing/reparing-agent self-review remains self-review and will not be relabeled independent.

## Context checkpoint

```yaml
last_progress: Owner-authorized repair cycle 4 restored FND-03-required server/build, protocol and exact World Bundle replay provenance and separated process-incarnation forensic attribution from replay placement semantics. No owner-funded AI was invoked by this repair.
status: validating
branch: agent/otv2-20260813-sim-determinism-01-architecture
head_sha: null
pr: 214
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: pending_after_repair_4
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 4
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: false
blocker: null
next_action: Freeze the repair-4 exact three-path head, perform terminal full-diff self-review and exact-head Agent Governance / Dependency Review / CodeQL, resolve the repaired P1 thread with exact-head evidence, then obtain a genuinely independent clean review of the unchanged repaired head before merge.
```
