# Closure Convergence Protocol

This is a routed delivery protocol for late-stage implementation closure in `Oteryn/Oteryn-Game`. It is a task-specific convergence layer over the current coordinator, worker, review, validation and integration rules. It grants no new write, architecture, review, merge, production or cross-repository authority.

Use it only when the active control plane explicitly records `CONVERGENCE_MODE` for a lane or programme.

## Purpose

Convergence mode exists to prevent late delivery from degenerating into repeated finding-by-finding repair loops:

```text
one finding -> one repair -> one heavy qualification -> another already-knowable finding
```

The target sequence is:

```text
finish current generation
-> freeze one exact closure head
-> one read-only final defect sweep
-> freeze a root-cause inventory
-> prepare repair authority for the whole generation
-> one coherent repair generation
-> one complete exact-head qualification
-> one final whole-diff review
-> normal protected integration
```

Convergence does not weaken correctness, required validation, independent review, fail-closed behavior, authority boundaries or protected integration.

## Entering convergence mode

The active control plane may enter convergence mode when one or more of these are true:

- the lane is in final closure rather than broad feature construction;
- two or more material repair generations have occurred after a candidate was treated as near-final;
- repeated review findings hit the same ownership, lifetime, finality, resource-accounting, authority or qualification family;
- historical finding count is growing faster than distinct root causes;
- the owner explicitly requests convergence mode.

Record the transition durably with the exact repository, governing Issue/task, branch/PR, protected `main`, current head/tree, current custody and reason. Entering convergence mode does not itself activate a new writer or widen owned paths.

If a material writer or diagnostic generation is already active, finish only that already-authorized generation first. Do not start the sweep concurrently with a mutating writer. Obtain one stable canonical exact head and return custody before freezing closure state.

## Phase 1 — freeze the closure head

Record:

```yaml
closure_head:
  repository: Oteryn/Oteryn-Game
  issue: <issue>
  pr: <pr>
  branch: <branch>
  base_main_sha: <sha>
  head_sha: <sha>
  tree_sha: <sha>
  custody: RELEASED_TO_CONTROL_PLANE
  state: FROZEN_FOR_SWEEP
```

Do not move this head merely to refresh prose, trigger CI, manufacture review evidence or copy status into tracked files.

## Convergence dispatch descriptor

The Work coordinator's mandatory minimal context packet remains authoritative. Convergence mode does **not** add unregistered top-level packet keys.

When a convergence audit is dispatched, encode the mode inside the existing `accepted_decisions` field as one bounded descriptor, for example:

```yaml
accepted_decisions:
  - convergence:
      protocol: docs/agents/CLOSURE_CONVERGENCE_PROTOCOL.md
      audit_mode: DISCOVERY_SWEEP
```

For final review the same existing field carries the additional frozen locators:

```yaml
accepted_decisions:
  - convergence:
      protocol: docs/agents/CLOSURE_CONVERGENCE_PROTOCOL.md
      audit_mode: FINAL_CANDIDATE_REVIEW
      frozen_root_cause_inventory: <ref>
      qualified_head: <exact sha>
```

These descriptors narrow how the requested audit is performed. They do not add authority and do not replace the normal `objective`, `relevant_findings`, `required_validation` or `lazy_refs` packet fields.

## Phase 2 — one final defect sweep

Run one comprehensive read-only sweep over the complete remaining closure surface that is material to the current gate. Prefer the existing independent auditor when current review policy permits it; dispatch it through the convergence descriptor above with `audit_mode: DISCOVERY_SWEEP`.

The sweep must inspect the full currently reachable ownership/finality/resource/authority/production-path surface required by the accepted contracts and acceptance cells, not only the latest diff or latest review comment.

The independent auditor's existing evidence field remains unchanged:

```text
classification: PROVEN | DERIVED | UNKNOWN | CONFLICT
```

Convergence adds a separate gate-disposition field. Every finding must set exactly one:

- `gate_classification: MATERIAL_BLOCKER` — a concrete current/next-gate correctness, authority, ownership, lifetime, finality, bounded-resource, recovery, security or production-path defect that can invalidate delivery;
- `gate_classification: EVIDENCE_GAP` — code may be correct, but required proof is absent or stale; obtain evidence before assuming a production-code repair is needed;
- `gate_classification: HARDENING` — worthwhile robustness/clarity/diagnostic improvement that is not required for the current accepted gate;
- `gate_classification: OUT_OF_SCOPE` — belongs to a later programme, deferred capability or unrelated architecture surface and does not block the current gate.

Evidence classification, severity and gate classification are independent. A future-only concern is not a current blocker merely because it is interesting or high impact in another programme.

### Root-cause collapse

Before publishing the sweep result, collapse symptoms that share the same underlying cause. Review-thread count, test count and file count must not become repair-batch count.

Common collapse families include:

- ownership or custody transfer;
- allocation/reservation or retained-copy accounting;
- lifetime/destruction-bound release;
- transaction/pool/connection finality;
- cancellation/timeout/restart recovery;
- semantic identity or fencing;
- authority/scope;
- production-path versus test-path mismatch.

For each material root cause record at least:

```yaml
root_cause_id: <stable id>
gate_classification: MATERIAL_BLOCKER
evidence_classification: PROVEN | DERIVED | UNKNOWN | CONFLICT
requirement_source: <contract/review/acceptance ref>
production_entry_points: []
exact_paths_or_symbols: []
failure_mode: <concise>
why_currently_knowable: <evidence>
authority_state: ACTIVE | PROTECTED_INACTIVE | ADDITIONAL_GRANT_REQUIRED
can_batch_with: []
required_repair: <smallest coherent repair>
required_validation: []
required_independent_review: <yes/no + reason>
evidence_refs: []
```

If the selected auditor emits its canonical `classification` field rather than duplicating it as `evidence_classification`, the control plane may copy that exact value into the root-cause inventory. Never reinterpret `UNKNOWN` or `CONFLICT` as `PROVEN` merely to complete the inventory.

The sweep ends with one frozen `FINAL_ROOT_CAUSE_INVENTORY` and no source mutation.

## Phase 3 — freeze inventory and prepare authority once

The active control plane consumes the sweep and freezes the material root-cause set before repair.

Prepare the smallest authority set that can close the entire compatible repair generation. Do not stop at the first missing file/symbol lease when the sweep already proves additional required paths in the same root-cause generation.

Split repair generations only for a real boundary:

- distinct architecture decision;
- distinct authority/custody owner that cannot be granted coherently;
- incompatible validation/integration lifecycle;
- dependency that must become protected before the next mutation is legal.

Do not split merely by file, review comment, test name or historical finding ID.

## Phase 4 — consolidated repair generation

During convergence mode, the normal bounded-worker rule means **one bounded coherent repair generation**, not one finding per worker return.

The writer receives the frozen root-cause inventory and repairs all compatible `MATERIAL_BLOCKER` items authorized for that generation before handoff.

The writer must not:

- stop after fixing the first compatible finding;
- broaden discovery into unrelated future systems;
- add speculative hardening while a frozen blocker batch is active;
- silently widen paths or authority;
- turn an `EVIDENCE_GAP` into production-code mutation without evidence that a defect exists.

Focused tests may run while iterating. Do not run a full hosted qualification after every individual edit when one coherent generation is still in progress unless a governing gate specifically requires it.

A diagnostic-only generation is not a repair generation. When authority says diagnosis only, add only the minimum authorized observability, return the exact first failing stage/evidence and stop; do not opportunistically repair a suspected cause in the same generation.

## Phase 5 — complete exact-head qualification

After the coherent repair generation is complete, freeze the candidate and run one complete qualification generation required by the changed paths and accepted gate. Reuse still-valid immutable evidence only within its proven generation; do not rerun unchanged heavy evidence merely for narration.

A qualification failure may create a new concrete repair trigger. Diagnose the failure before mutating again.

## Phase 6 — final whole-diff review

Dispatch the independent auditor through the convergence descriptor above with `audit_mode: FINAL_CANDIDATE_REVIEW`, the frozen root-cause inventory ref and the exact qualified head.

The final review asks primarily:

> Does this exact candidate close the frozen root-cause inventory and every still-binding current-gate acceptance requirement?

It is not a new open-ended architecture expedition.

The reviewer may still report a new material blocker, but after inventory freeze the control plane may expand WP closure only when at least one novelty trigger is proven:

1. a new concrete CI/runtime failure on the candidate;
2. a genuinely newly exposed dependency caused by the repair and not reasonably inspectable during the frozen sweep;
3. an independent review finding whose material fact could not reasonably have been established from the frozen source/evidence;
4. material protected-`main`, policy or accepted-contract movement.

If a later material finding was reasonably knowable during the sweep, set `gate_classification: MATERIAL_BLOCKER` if it is a real current-gate defect and additionally mark it `FINAL_SWEEP_MISS`. The defect still must be handled safely, but the coordinator must record why the sweep missed it, add it once to the existing root-cause model, and avoid reopening unrestricted discovery.

`HARDENING` and `OUT_OF_SCOPE` findings do not block integration unless current accepted authority explicitly makes them acceptance requirements.

## Phase 7 — integration

After clean final review and exact-head qualification, return to the normal repository integration policy. This protocol does not redefine review authority, Merge Queue routing, protected-main readback or lifecycle closeout.

## Publication safety

Canonical material work must use the worker's normal authorized high-level Git publication path.

If that publication path is unavailable or rejected, return `BLOCKED_CAPABILITY_UNAVAILABLE` or the repository-defined equivalent and hand custody back to the control plane.

Do **not** use direct Git object construction as an emergency publication fallback for a canonical material branch. In particular, do not synthesize replacement commits, trees, blobs or refs through low-level Git object APIs to bypass an unavailable normal push/publication path. Do not force, reset, rebase or manufacture replacement history as recovery.

A separately authorized coordinator recovery operation may reconcile a damaged branch under current governance; an implementation worker must not improvise that authority.

## Required convergence checkpoint

The control plane keeps one compact checkpoint:

```yaml
convergence_mode: ACTIVE | COMPLETE
closure_head: <sha or null>
final_root_cause_inventory: <ref or null>
material_root_causes_open: <count>
evidence_gaps_open: <count>
hardening_deferred: <count>
out_of_scope_deferred: <count>
repair_generation: <id or null>
qualification_generation: <id or null>
final_review_generation: <id or null>
new_scope_trigger: <ref or null>
next_action: <one concrete action>
```

Convergence is complete only after the normal protected integration/readback/closeout contract is satisfied.
