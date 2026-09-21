# Game agent documentation rules

The root bootstrap and `docs/agents/META_AGENT_POLICY_BINDING.json` supply organization-policy delivery for this tree. Load specialist Game procedures only when their domain or operation applies.

## Ownership and placement

- Live Issues, PRs and checks govern task status. Task packets and shared indexes are reconstruction and coordination records, not a second status authority.
- Put accepted architecture in `docs/architecture/`, durable public integration contracts in `docs/contracts/`, and retained evidence under `docs/agents/evidence/` or immutable workflow artifacts.
- Preserve stable terms: `WorldId`, `ChannelId`, `InstanceId`, `NodeId`, `GameSessionId` and `protocol-oteryn`.
- Do not reintroduce Canary as a target runtime or protocol adapter without an owner-approved ADR that explicitly supersedes ADR-0001.

## Routed Game procedures

Use `tasks/TASK_TEMPLATE.md` for substantial task records. For architecture or contract work, load `ARCHITECTURE_DECISION_DISCIPLINE.md` and the current domain authority. For execution and validation, load only the applicable build matrix, task allocation and nearest path instructions. Multi-agent architecture programmes additionally use `MULTI_AGENT_ARCHITECTURE_ORCHESTRATION.md` when a live allocation invokes it.

When the active control plane explicitly records `CONVERGENCE_MODE`, load `CLOSURE_CONVERGENCE_PROTOCOL.md`. That routed protocol changes closure batching and audit shape only; it grants no new mutation, review, architecture or integration authority.

For an independent audit dispatched under convergence mode:

- `audit_mode: DISCOVERY_SWEEP` means one comprehensive read-only search for all currently knowable current/next-gate defects before the final repair generation, with mandatory `gate_classification: MATERIAL_BLOCKER | EVIDENCE_GAP | HARDENING | OUT_OF_SCOPE` and root-cause collapse;
- `audit_mode: FINAL_CANDIDATE_REVIEW` means review the exact qualified candidate against the frozen root-cause inventory and still-binding current-gate requirements rather than starting a new open-ended architecture expedition;
- preserve the auditor's existing evidence `classification: PROVEN | DERIVED | UNKNOWN | CONFLICT`; convergence gate classification is a separate field;
- after the inventory freeze, a newly proposed blocker must satisfy a novelty trigger from `CLOSURE_CONVERGENCE_PROTOCOL.md`; otherwise mark a real current-gate miss `FINAL_SWEEP_MISS` and keep discovery bounded.

Canonical material workers must also obey the convergence protocol's publication-safety rule. A selected local Git candidate uses the guarded Git route. Repository-native API authoring may use bounded sequential high-level file mutations on an exclusively allocated task branch before candidate freeze when the intended mutation is the API write and no selected local Git candidate is being reconstructed. Bind the commit SHA returned by the final authoring write as `expected_final_authoring_head`. Fresh live readback must equal that exact SHA and verify the complete bounded delta and owned paths before the remote head is frozen as the candidate; any mismatch fails closed as writer/state drift. Candidate-specific CI/review evidence starts only from that fenced frozen head. A separately allocated API-native publication may instead select a **new candidate** only when one server-side mutation atomically fences the exact expected task-branch head and creates the complete bounded delta as one successor commit. Never use sequential API writes to reconstruct a selected local candidate, mutate a frozen candidate, or work around shared/ambiguous writer custody; never substitute ancestry-only `force=false` ref movement, low-level Git commit/tree/blob/ref construction, or raw Git Data reconstruction as publication fallbacks.

Classify evidence as `PROVEN`, `DERIVED`, `UNKNOWN` or `CONFLICT`. An unmerged task document cannot authorize architecture, production access or cross-repository writes. Preserve history when later decisions supersede only part of an older contract.

## Integration routing precedence

Integration wording in reusable prompts and routed procedures such as `merge`, `squash merge`, `expected-head merge`, `protected merge` or `auto-merge` describes lifecycle intent only; it does not select or authorize a GitHub merge primitive. For every integration action, resolve the immutable META policy bound by `META_AGENT_POLICY_BINDING.json` and the live repository control-plane capability.

Autonomous Merge Queue submission inherits the authenticated bound META 3.1 native exact-head Merge Queue contract. The selected route is REST `merge-async` with the exact qualified `sha` and explicit `merge_action="merge_queue"`, preceded by fresh target-bound repository, PR, `base=main`, head, authorization and eligibility preflight. A `202` is acceptance only: bind its exact returned UUID to an executor-owned receipt sequence and require an immediate live readback carrying the same UUID at a strictly later executor sequence; timestamps are freshness-only. Reconcile `200` and `409` responses. Queue admission is not terminal proof; require the real `merge_group` aggregate `game-gate` and protected-main readback before closeout. Direct/immediate merge, generic `enablePullRequestAutoMerge`, bypass, force, a default merge action, no-op/retrigger commits and ambiguous dequeue cleanup are forbidden substitutes. If the selected native operation is unavailable, record `BLOCKED_CAPABILITY_UNAVAILABLE`, preserve the qualified candidate, release active waiting ownership as applicable and continue safe path-disjoint work.
