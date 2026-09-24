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
-> freeze one exact closure head and protected-main generation
-> one read-only final defect sweep
-> freeze a root-cause inventory
-> reconcile unresolved evidence through immutable successor inventory when required
-> prepare repair authority for the whole compatible generation
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

## Phase 1 — freeze the closure generation

Record:

```yaml
closure_head:
  repository: Oteryn/Oteryn-Game
  issue: <issue>
  task_id: <task id>
  pr: <pr>
  branch: <branch>
  base_main_sha: <protected main sha>
  head_sha: <sha>
  tree_sha: <sha>
  custody: RELEASED_TO_CONTROL_PLANE
  state: FROZEN_FOR_SWEEP
```

`base_main_sha` and `head_sha` are jointly part of the closure generation. Do not silently combine a candidate head frozen against one protected-main generation with contracts/evidence resolved from another.

Do not move this head merely to refresh prose, trigger CI, manufacture review evidence or copy status into tracked files.

Protected-`main` movement after this freeze does not itself require or authorize a merge-up of the frozen candidate. Reconcile the upstream delta read-only first. If current accepted requirements and candidate semantics are unchanged, preserve the frozen head and let canonical Merge Queue qualification prove composition against current protected `main`. Only a proven current-gate contract/authority/semantic conflict or another explicit source-reconciliation requirement may require a new candidate head.

## Convergence dispatch descriptor

The Work coordinator's mandatory minimal context packet remains authoritative. Convergence mode does **not** add unregistered top-level packet keys.

When a convergence discovery audit is dispatched, encode the mode and frozen closure generation inside the existing `accepted_decisions` field as one bounded descriptor:

```yaml
accepted_decisions:
  - convergence:
      protocol: docs/agents/CLOSURE_CONVERGENCE_PROTOCOL.md
      audit_mode: DISCOVERY_SWEEP
      base_main_sha: <Phase 1 protected main sha>
      closure_head: <Phase 1 head sha>
```

Immediately before the sweep, the auditor must independently resolve both the live protected `main` and the live PR/branch target. Require:

```text
live protected main == base_main_sha
auditor audit_main_sha == base_main_sha
live target head == closure_head
```

If protected `main` differs, fail closed with convergence reason code `STALE_SWEEP_BASE` and do not run the discovery sweep against mixed-generation contracts/evidence. The control plane must reconcile the changed protected-main contracts/policy and freeze a new Phase 1 closure generation before retrying. If the branch or PR head differs, fail closed with convergence reason code `STALE_TARGET` and do not silently audit the newer generation under the frozen sweep.

Convergence stale/drift codes are subordinate reason codes, not new values in the independent auditor's closed overall-disposition vocabulary. When `OTV2_WORK_DELIVERY_INDEPENDENT_AUDITOR` is the selected reviewer and a stale/drift condition prevents a reliable verdict, use overall disposition `INSUFFICIENT_EVIDENCE` plus `convergence_reason_code: STALE_SWEEP_BASE | STALE_TARGET | STALE_QUALIFIED_HEAD | DRIFTED_INVENTORY` as applicable. Do not invent a new overall disposition. The control plane then performs the fail-closed refreeze/requalification action required by this protocol.

For final review the same existing field carries the active immutable inventory generation and exact qualified candidate:

```yaml
accepted_decisions:
  - convergence:
      protocol: docs/agents/CLOSURE_CONVERGENCE_PROTOCOL.md
      audit_mode: FINAL_CANDIDATE_REVIEW
      frozen_root_cause_inventory:
        inventory_generation: <positive integer>
        evidence_locator: <generation-specific GitHub evidence-note ref>
        sweep_target:
          repository: Oteryn/Oteryn-Game
          issue: <governing issue>
          task_id: <task id>
          pr: <pr>
          branch: <branch>
          base_main_sha: <Phase 1 protected main sha>
          audit_main_sha: <DISCOVERY_SWEEP audit main sha>
          closure_head: <Phase 1 head_sha>
          tree_sha: <Phase 1 tree_sha>
        content_identity:
          serialization: RFC8785_JSON
          sha256: <lowercase hex digest of the canonical serialized inventory envelope>
      qualified_head: <exact qualified candidate sha>
```

Before dispatch, the control plane must require the descriptor's `sweep_target` to exact-match the corresponding immutable fields of the Phase 1 `closure_head` record and require `audit_main_sha == base_main_sha`. The final reviewer must retrieve the active inventory through its generation-specific `evidence_locator`, canonicalize the complete inventory envelope using the declared serialization, recompute the digest, and require an exact match with `content_identity`. The reviewer must also require the envelope's `inventory_generation` and `sweep_target` to exact-match the descriptor and the Phase 1 closure generation.

If the active inventory is a successor (`inventory_generation > 1`), the control plane before dispatch and the final reviewer independently must also validate the complete immutable predecessor chain back to generation 1. Start from the descriptor's active `evidence_locator`; for every successor envelope require a non-null `predecessor_evidence_locator` and use that exact locator to retrieve generation N-1. Canonicalize and recompute each predecessor's declared digest, require generation numbers to decrease by exactly one, require the successor's `predecessor_content_identity` to equal the predecessor's recomputed SHA-256 identity, require the successor's `predecessor_evidence_locator` to resolve exactly that predecessor generation, and require every envelope to preserve the same `sweep_target`. Generation 1 must have both `predecessor_content_identity: null` and `predecessor_evidence_locator: null`. Validate each transition against its declared `transition_reason`: `EVIDENCE_RECONCILIATION` must preserve every unaffected root cause and add no new root cause; `LATE_BLOCKER_ADMISSION` must preserve every pre-existing root cause unchanged and add only the specifically evidenced late root causes. The chain must prove that no unresolved or still-binding prior root cause disappeared. A missing predecessor locator/content, broken locator/digest link, skipped/duplicated generation, sweep-target drift, invalid transition, or silently dropped prior blocker is fail-closed `INSUFFICIENT_EVIDENCE` with convergence reason code `DRIFTED_INVENTORY`.

A missing identity, unavailable content, unsupported serialization, digest mismatch, inventory-generation mismatch or sweep-target mismatch is fail-closed `INSUFFICIENT_EVIDENCE` with convergence reason code `DRIFTED_INVENTORY`; the reviewer must not qualify the candidate against changed, cross-lane, cross-task or wrong-generation inventory content.

Independently of the inventory checks, the control plane immediately before dispatch and the reviewer immediately before review must resolve the live PR/branch target and require its exact current head to equal `qualified_head`. A live-target mismatch is fail-closed `INSUFFICIENT_EVIDENCE` with convergence reason code `STALE_QUALIFIED_HEAD`. Never review a newer live head under qualification evidence for an older candidate.

The final reviewer must also independently resolve current protected `main` as `review_main_sha`. If `review_main_sha != audit_main_sha`, evaluate the protected-main/policy/accepted-contract movement under the novelty rule in Phase 6 before qualifying the candidate. Movement is material only when it changes an applicable accepted requirement, authority/contract assumption or candidate-relevant semantic dependency. A protected workflow/runner/gate implementation update with unchanged acceptance semantics is not, by itself, sweep-base invalidation; preserve the candidate and require the current canonical `merge_group` gate to qualify composition against that protected generation. Material movement is fail-closed `INSUFFICIENT_EVIDENCE` with convergence reason code `STALE_SWEEP_BASE` and requires a new Phase 1 + discovery sweep; proven immaterial or Merge-Queue-owned qualification movement may continue only with exact evidence recording why the frozen current-gate requirements remain unchanged. Never rewrite the frozen inventory merely to track a newer main SHA.

These descriptors narrow how the requested audit is performed. They do not add authority and do not replace the normal `objective`, `relevant_findings`, `required_validation` or `lazy_refs` packet fields.

## Phase 2 — one final defect sweep

Run one comprehensive read-only sweep over the complete remaining closure surface that is material to the current gate. Prefer the existing independent auditor when current review policy permits it; dispatch it through the convergence descriptor above with `audit_mode: DISCOVERY_SWEEP`, the exact frozen `base_main_sha`, and the exact frozen `closure_head`.

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
repair_eligibility: ELIGIBLE | EVIDENCE_RECONCILIATION_REQUIRED
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

The sweep ends with one frozen initial `FINAL_ROOT_CAUSE_INVENTORY` and no source mutation. The immutable object whose identity is bound must be a complete inventory envelope, not the root-cause payload alone:

```json
{
  "schema": "OTV2_FINAL_ROOT_CAUSE_INVENTORY_V1",
  "inventory_generation": 1,
  "predecessor_content_identity": null,
  "predecessor_evidence_locator": null,
  "transition_reason": "DISCOVERY_SWEEP",
  "sweep_target": {
    "repository": "Oteryn/Oteryn-Game",
    "issue": "<governing issue>",
    "task_id": "<task id>",
    "pr": "<pr>",
    "branch": "<branch>",
    "base_main_sha": "<Phase 1 protected main sha>",
    "audit_main_sha": "<DISCOVERY_SWEEP audit main sha>",
    "closure_head": "<Phase 1 head_sha>",
    "tree_sha": "<Phase 1 tree_sha>"
  },
  "reconciliation_refs": [],
  "late_blocker_refs": [],
  "inventory": {
    "...": "<complete FINAL_ROOT_CAUSE_INVENTORY structured value>"
  }
}
```

The `inventory` member above is a structured JSON value, not a string serialization; the placeholder object only illustrates that requirement and does not prescribe a different root-cause schema. Serialize this entire envelope as RFC 8785 canonical JSON and bind it to a lowercase hexadecimal SHA-256 digest. The `sweep_target` fields must exact-match the Phase 1 closure record, and `audit_main_sha` must exact-match `base_main_sha`, before the identity is accepted. The resulting immutable `content_identity` therefore proves the exact inventory and exact sweep generation. The auditor's normal GitHub evidence note remains only a locator; the digest and generation binding define inventory identity.

## Phase 3 — freeze inventory, reconcile evidence, and prepare authority once

The active control plane consumes the sweep and freezes the material root-cause set before repair.

The freeze record must carry the immutable sweep target, active inventory generation, generation-specific evidence locator and immutable content identity:

```yaml
final_root_cause_inventory:
  inventory_generation: <positive integer>
  evidence_locator: <generation-specific GitHub evidence-note ref>
  sweep_target:
    repository: Oteryn/Oteryn-Game
    issue: <governing issue>
    task_id: <task id>
    pr: <pr>
    branch: <branch>
    base_main_sha: <Phase 1 protected main sha>
    audit_main_sha: <DISCOVERY_SWEEP audit main sha>
    closure_head: <Phase 1 head_sha>
    tree_sha: <Phase 1 tree_sha>
  content_identity:
    serialization: RFC8785_JSON
    sha256: <lowercase hex digest of the canonical serialized inventory envelope>
```

Immediately before any evidence reconciliation that interprets governing contracts, and again immediately before any mutating repair dispatch, independently resolve current protected `main` as `live_main_sha`. If `live_main_sha != base_main_sha`, inspect the exact protected-main delta relevant to current accepted contracts/policy before proceeding. Material current-gate movement fails closed with convergence reason code `STALE_SWEEP_BASE`; do not mutate, and freeze a new Phase 1 closure generation plus discovery sweep. Proven immaterial movement may continue only with exact external drift evidence explaining why the frozen current-gate requirements remain unchanged. Do not mutate the frozen inventory merely to record an immaterial main movement; if the movement changes any preserved finding field, use the successor mechanism below.

Immediately before any repair dispatch, retrieve the active inventory through its generation-specific locator, canonicalize the complete envelope using the recorded serialization, recompute the digest, and require an exact match with `content_identity`. Independently exact-match the envelope's `inventory_generation` and `sweep_target` against the freeze record and the Phase 1 `closure_head` record, including `base_main_sha`, and require `audit_main_sha == base_main_sha`. Missing/unavailable identity material, an unsupported serialization, digest mismatch/drift, inventory-generation mismatch or target-generation mismatch fails closed and returns the inventory for reconciliation; valid inventory from another repository, task, PR, branch, protected-main generation or closure generation cannot silently qualify for this repair generation.

Immediately before dispatching the **first mutating worker of a new repair generation**, the control plane and writer must independently resolve the live PR/branch head and exact-match it to the recorded repair base for that generation. For the initial post-sweep repair generation, `repair_base_head` is the active inventory's `sweep_target.closure_head`. For a new repair generation created by `LATE_BLOCKER_ADMISSION` during Phase 5 qualification, `repair_base_head` is the exact frozen `qualification_target_head` on which the qualifying evidence was produced; after successful qualification this is also `qualified_head`, while after failed qualification it remains the exact failed candidate head and does not require a nonexistent `qualified_head`. For a new repair generation created during Phase 6 final review, `repair_base_head` is the exact reviewed `qualified_head`. Record that head immutably with the late-blocker successor evidence before dispatch. If the live head differs from the applicable recorded `repair_base_head`, return custody without mutation with convergence reason code `STALE_REPAIR_BASE`; do not infer or advance the repair base from the live branch. This exception does not rewrite the preserved Phase 1 `sweep_target` and does not permit arbitrary post-sweep movement.

Before any `MATERIAL_BLOCKER` enters a mutating repair generation, reconcile its evidence classification:

- `PROVEN` may be `repair_eligibility: ELIGIBLE` when authority is otherwise sufficient;
- `DERIVED` may be eligible only when the derivation is explicit and the governing contract/control plane accepts that inference as sufficient mutation evidence; otherwise hold it for evidence reconciliation;
- `UNKNOWN` or `CONFLICT` is never repair-eligible. Resolve the missing/conflicting evidence first, or hold/escalate the affected claim under existing evidence/architecture/authority rules. Do not dispatch production-code repair merely because the gate classification says `MATERIAL_BLOCKER`.

### Immutable successor inventory after evidence reconciliation

A frozen inventory is never edited in place. If evidence reconciliation changes any preserved finding field — including `evidence_classification`, `repair_eligibility`, evidence refs, gate applicability, required repair or required validation — create one immutable **successor inventory** for the whole completed reconciliation batch.

The evidence-reconciliation successor must:

1. preserve the exact same `sweep_target` as its predecessor;
2. increment `inventory_generation` by exactly one;
3. set `predecessor_content_identity` to the predecessor's exact SHA-256 identity;
4. set `predecessor_evidence_locator` to the predecessor generation's exact immutable `evidence_locator`;
5. set `transition_reason: EVIDENCE_RECONCILIATION`;
6. list exact immutable `reconciliation_refs` proving every changed finding field while preserving `late_blocker_refs` from its predecessor;
7. preserve every unaffected root cause unchanged at the structured-value level;
8. add no newly discovered root cause under the guise of evidence reconciliation;
9. be canonicalized and hashed as a new complete RFC 8785 envelope before it can become active;
10. be persisted at a **new generation-specific `evidence_locator`**. Never edit, overwrite or repurpose the predecessor generation's locator to store successor content.

Create successors per **coherent reconciliation generation**, not one successor per individual finding.

### Immutable successor inventory for a late current-gate blocker

A new material root cause discovered after the initial sweep may enter the frozen model only through this transition. Use it when Phase 3 evidence reconciliation, Phase 4 repair, Phase 5 qualification or Phase 6 final review proves either:

- a new current-gate blocker satisfying novelty trigger 1, 2 or 3 in Phase 6; or
- a real current-gate defect that was reasonably knowable during the sweep and is therefore marked `sweep_disposition: FINAL_SWEEP_MISS`.

Do **not** use this transition for material protected-`main`/policy/accepted-contract movement under novelty trigger 4. That movement invalidates the sweep base and requires a new Phase 1 closure generation plus discovery sweep.

The late-blocker successor must:

1. preserve the predecessor's exact `sweep_target`;
2. increment `inventory_generation` by exactly one;
3. set `predecessor_content_identity` to the predecessor's exact SHA-256 identity;
4. set `predecessor_evidence_locator` to the predecessor generation's exact immutable `evidence_locator`;
5. set `transition_reason: LATE_BLOCKER_ADMISSION`;
6. preserve all prior `reconciliation_refs` and append exact immutable `late_blocker_refs` for every newly admitted root cause, including the qualifying CI/runtime/review/evidence-reconciliation/repair evidence and the proven novelty trigger or `FINAL_SWEEP_MISS` disposition;
7. add only the newly proven current-gate root cause(s) from that coherent late-finding generation, using the same root-cause schema and evidence/repair-eligibility rules as the original inventory;
8. preserve all pre-existing root causes unchanged at the structured-value level unless a separate evidence-reconciliation transition is also required; do not silently combine unrelated state changes into late admission;
9. be canonicalized and hashed as a new complete RFC 8785 envelope and persisted at a new generation-specific locator before becoming active.

Create one late-blocker successor for the coherent late-finding generation, not one per review comment. If the late finding maps to an already-recorded root cause rather than a genuinely new root cause, do not add a duplicate; reconcile the existing root through the evidence-reconciliation successor only when one of its preserved fields must change.

If a genuinely new root cause is proven during **Phase 3 evidence reconciliation before any repair mutation**, admit the successor first, re-run the Phase 3 authority/evidence checks against the new active tuple, and only then dispatch the repair generation from the unchanged `closure_head`.

If a genuinely new root cause is proven **after Phase 4 mutation has already begun**, the writer must stop at the nearest safe bounded point, record the exact current branch head as `paused_repair_head`, record the active `repair_generation`, and return custody without making any mutation for the newly discovered root cause. The control plane may admit one `LATE_BLOCKER_ADMISSION` successor from the proven evidence. The same already-started repair generation may resume only if (1) the successor is fully active and identity-verified, (2) authority/evidence checks pass for the enlarged blocker set, (3) the live branch head still exact-matches `paused_repair_head`, (4) no intervening writer or unrecorded mutation occurred, and (5) the same repair-generation identity is preserved. This resume is not a new repair dispatch from the Phase 1 base, so the initial post-sweep repair-base rule does not reapply. If any resume condition fails, terminate that repair generation, return custody, and freeze a new Phase 1 closure generation plus discovery sweep from the live head.

If a genuinely new root cause is proven during **Phase 5 qualification**, the late-blocker successor must additionally record `repair_base_head` equal to the exact frozen `qualification_target_head` on which that evidence was produced, plus the exact qualification evidence ref proving the binding. A failed qualification uses that failed candidate head directly; it does not require a `qualified_head`. If the qualification had already succeeded before the late blocker was established, `qualification_target_head == qualified_head`. If a genuinely new root cause is proven during **Phase 6 final review**, record `repair_base_head` equal to the exact reviewed `qualified_head` plus the exact review evidence ref. In either case the following repair generation may start only from that exact recorded `repair_base_head`; any intervening head movement is `STALE_REPAIR_BASE`. This creates a bounded continuation from exact evidence without pretending the post-repair candidate is still the Phase 1 `closure_head`.

After any late-blocker successor becomes active, previous qualification/review evidence is historical for integration readiness. When the successor is admitted before qualification/review, complete or safely resume the authorized coherent repair generation under the rules above; otherwise return to the Phase 3 authority/evidence checks and perform the required coherent repair generation from the exact recorded `repair_base_head`. In all cases, run a fresh complete Phase 5 exact-head qualification and obtain a fresh Phase 6 final review bound to the new active inventory generation. A late current-gate blocker can never be admitted and then bypass requalification/re-review.

### Activating any successor

The predecessor envelope, digest and locator remain preserved audit evidence. After a successor is fully persisted and its identity verified, atomically advance the control-plane active-inventory pointer as one tuple:

```text
(inventory_generation, evidence_locator, content_identity)
```

All later repair dispatch, qualification reconciliation and final review must bind to that exact active tuple. A partially written successor, reused predecessor locator, missing/incorrect `predecessor_evidence_locator`, digest mismatch or ambiguous active-generation pointer fails closed.

If unresolved `EVIDENCE_RECONCILIATION_REQUIRED` items remain, they stay outside mutating repair scope. A material blocker that remains unresolved at final qualification prevents closure; it cannot be silently dropped from the active inventory.

Prepare the smallest authority set that can close the entire compatible **repair-eligible** material generation. Do not stop at the first missing file/symbol lease when the sweep already proves additional required paths in the same root-cause generation.

Split repair generations only for a real boundary:

- distinct architecture decision;
- distinct authority/custody owner that cannot be granted coherently;
- incompatible validation/integration lifecycle;
- dependency that must become protected before the next mutation is legal.

Do not split merely by file, review comment, test name or historical finding ID.

## Phase 4 — consolidated repair generation

During convergence mode, the normal bounded-worker rule means **one bounded coherent repair generation**, not one finding per worker return.

The writer receives the exact active frozen root-cause inventory generation and repairs all compatible, repair-eligible `MATERIAL_BLOCKER` items authorized for that generation before handoff. Items still marked `EVIDENCE_RECONCILIATION_REQUIRED` remain outside the mutating batch until their evidence state is resolved through the immutable successor mechanism above.

If evidence reconciliation or the active repair itself proves a genuinely new material root cause, do not repair that new root outside the active inventory. Apply the late-blocker admission rule above: admit it before mutation when repair has not started, or pause the already-started generation at an exact `paused_repair_head`, activate the successor, and resume only under the exact-head/custody/generation conditions defined above.

The writer must not:

- stop after fixing the first compatible finding;
- broaden discovery into unrelated future systems;
- add speculative hardening while a frozen blocker batch is active;
- silently widen paths or authority;
- turn an `EVIDENCE_GAP` into production-code mutation without evidence that a defect exists;
- mutate or replace the active inventory in place.

Focused tests may run while iterating. Do not run a full hosted qualification after every individual edit when one coherent generation is still in progress unless a governing gate specifically requires it.

A diagnostic-only generation is not a repair generation. When authority says diagnosis only, add only the minimum authorized observability, return the exact first failing stage/evidence and stop; do not opportunistically repair a suspected cause in the same generation.

## Phase 5 — complete exact-head qualification

After the coherent repair generation is complete, freeze the candidate as the exact `qualification_target_head` and run one complete qualification generation required by the changed paths and accepted gate. Reuse still-valid immutable evidence only within its proven generation; do not rerun unchanged heavy evidence merely for narration. Only after the required qualification succeeds may that exact `qualification_target_head` become the `qualified_head`.

Before treating qualification as closure evidence, confirm that every active-inventory `MATERIAL_BLOCKER` is either closed by the candidate or explicitly remains open and blocking. An unresolved evidence-reconciliation item cannot disappear merely because deterministic CI is green.

A qualification failure may create a new concrete repair trigger. Diagnose the failure before mutating again. If diagnosis proves a genuinely new current-gate root cause satisfying the Phase 6 novelty rules, admit it through the late-blocker successor transition above, bind its `repair_base_head` to the exact failed `qualification_target_head`, and record the exact failed-qualification evidence before any repair mutation.

## Phase 6 — final whole-diff review

Dispatch the independent auditor through the convergence descriptor above with `audit_mode: FINAL_CANDIDATE_REVIEW`, the active frozen root-cause inventory generation/locator, immutable sweep-target identity, immutable content identity, and the exact qualified head. Before dispatch, the control plane must (1) exact-match the recomputed active inventory envelope identity and generation, (2) exact-match the envelope and descriptor `sweep_target` to the Phase 1 closure record including `base_main_sha`, (3) require `audit_main_sha == base_main_sha`, (4) resolve the live candidate and require its current exact head to equal `qualified_head`, and (5) when the active inventory is a successor, validate the complete predecessor chain and every transition's preservation/addition rules as specified in the convergence dispatch descriptor.

The reviewer must repeat these checks against independently resolved evidence immediately before review, including the complete predecessor-chain validation for every successor generation. Inventory mismatch/drift, active-generation mismatch, sweep-target mismatch, broken predecessor locator/identity, invalid transition, or loss of a prior unresolved/still-binding root cause fails closed as `INSUFFICIENT_EVIDENCE` with convergence reason code `DRIFTED_INVENTORY`. If the live PR/branch head does not equal `qualified_head`, return `INSUFFICIENT_EVIDENCE` with convergence reason code `STALE_QUALIFIED_HEAD` and do not attach review conclusions to the newer generation. The pre-repair `closure_head` and post-repair `qualified_head` may legitimately differ; the required invariant is that the active inventory remains bound to the exact Phase 1 sweep generation while the review itself remains bound to the exact qualified candidate.

Resolve current protected `main` as `review_main_sha`. If it differs from the frozen `audit_main_sha`, inspect only the delta relevant to current accepted contracts/policy/candidate semantics before applying the novelty rule below. Material movement means an applicable accepted requirement, authority/contract assumption or candidate-relevant semantic dependency changed. A change limited to the implementation of protected CI/Merge Queue gates, with the same accepted gate semantics, is qualification movement owned by the current synthetic `merge_group` and does not require mutating the source candidate or refreezing the sweep. Material movement invalidates the sweep base, requires overall disposition `INSUFFICIENT_EVIDENCE` with convergence reason code `STALE_SWEEP_BASE`, and requires a fresh Phase 1 + discovery sweep; proven immaterial or Merge-Queue-owned qualification movement must be recorded as exact evidence and does not mutate the active inventory.

The final review asks primarily:

> Does this exact candidate close the exact active immutable root-cause inventory generation and every still-binding current-gate acceptance requirement?

It is not a new open-ended architecture expedition.

The reviewer may still report a new material blocker, but after inventory freeze the control plane may expand WP closure only when at least one novelty trigger is proven:

1. a new concrete CI/runtime failure on the candidate;
2. a genuinely newly exposed dependency caused by the repair and not reasonably inspectable during the frozen sweep;
3. an independent review finding whose material fact could not reasonably have been established from the frozen source/evidence;
4. protected-`main`, policy or accepted-contract movement that materially changes an applicable accepted requirement, authority/contract assumption or candidate-relevant semantic dependency; a gate/workflow implementation change with unchanged acceptance semantics is handled by current Merge Queue qualification instead of becoming a new root cause.

If trigger 1, 2 or 3 proves a genuinely new current-gate root cause, admit it through one immutable `LATE_BLOCKER_ADMISSION` successor before repair. Trigger 4 does not use a successor; it invalidates the sweep base and requires fresh Phase 1 + discovery.

If a later material finding was reasonably knowable during the sweep, set `gate_classification: MATERIAL_BLOCKER` if it is a real current-gate defect and additionally set `sweep_disposition: FINAL_SWEEP_MISS`. The defect still must be handled safely: record why the sweep missed it, admit it once through the immutable late-blocker successor when it is a new root cause, and avoid reopening unrestricted discovery. If it maps to an existing root cause, update that root only through the evidence-reconciliation successor when preserved fields change.

Any material late blocker makes the current qualification/final-review generation non-terminal for integration readiness. After its successor/repair path, fresh exact-head qualification and a fresh final review are mandatory.

`HARDENING` and `OUT_OF_SCOPE` findings do not block integration unless current accepted authority explicitly makes them acceptance requirements.

## Phase 7 — integration

After clean final review and exact-head qualification, return to the normal repository integration policy. This protocol does not redefine review authority, Merge Queue routing, protected-main readback or lifecycle closeout.

## Publication safety

Canonical material work uses the ordinary lifecycle:

`AUTHORING -> FREEZE_SHA -> VALIDATE -> MQ`

Default ordinary authoring is repository-native high-level API mutation on one exclusively allocated task branch before freeze. Local Git is optional and may be selected only when its normal guarded publication path was proven before mutation.

During AUTHORING, one writer owns the branch. Fresh-read the live head before each write and stop on unexpected movement. After the final authoring write, require the returned SHA to equal the live branch head, verify the complete bounded delta and owned paths, and freeze that exact remote SHA. Candidate-specific qualification/review begins only after freeze.

If a material repair is needed after freeze, first return to AUTHORING on the same allocated branch; only then may high-level API writes produce a successor head. Freeze the new SHA and rerun candidate-specific evidence. Missing Git credentials or push capability is not a reason to request Remote Desktop. Do not use low-level Git Data reconstruction, ancestry-only `force=false` ref movement, writes while a head remains frozen, force/reset/rebase, or Remote Desktop as publication fallbacks. Recovery-specific atomic publication remains a separately governed control-plane operation under current root/META policy.

## Required convergence checkpoint

The control plane keeps one compact checkpoint:

```yaml
convergence_mode: ACTIVE | COMPLETE
closure_generation:
  base_main_sha: <Phase 1 protected main sha or null>
  head_sha: <Phase 1 candidate sha or null>
  tree_sha: <Phase 1 tree sha or null>
final_root_cause_inventory:
  active_inventory_generation: <positive integer or null>
  evidence_locator: <active generation-specific evidence ref or null>
  sweep_target:
    repository: <repo or null>
    issue: <issue or null>
    task_id: <task id or null>
    pr: <pr or null>
    branch: <branch or null>
    base_main_sha: <Phase 1 protected main sha or null>
    audit_main_sha: <DISCOVERY_SWEEP audit main sha or null>
    closure_head: <Phase 1 sha or null>
    tree_sha: <Phase 1 tree or null>
  predecessor_content_identity: <sha256 or null>
  predecessor_evidence_locator: <generation-specific predecessor evidence ref or null>
  content_identity:
    serialization: RFC8785_JSON
    sha256: <lowercase hex digest or null>
  transition_reason: DISCOVERY_SWEEP | EVIDENCE_RECONCILIATION | LATE_BLOCKER_ADMISSION | null
material_root_causes_open: <count>
evidence_gaps_open: <count>
hardening_deferred: <count>
out_of_scope_deferred: <count>
repair_generation: <id or null>
repair_start_head: <sha or null>
paused_repair_head: <sha or null>
qualification_generation: <id or null>
qualification_target_head: <sha or null>
final_review_generation: <id or null>
new_scope_trigger: <ref or null>
next_action: <one concrete action>
```

Convergence is complete only after the normal protected integration/readback/closeout contract is satisfied.
