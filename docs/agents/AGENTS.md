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

Canonical material work uses the ordinary lifecycle `AUTHORING -> FREEZE_SHA -> VALIDATE -> MQ`. Default ordinary authoring is repository-native high-level API mutation on one exclusively allocated task branch before freeze. Fresh-read the branch head before each write and stop on unexpected movement. After the final authoring write, require the returned SHA to equal the live branch head, verify the complete bounded delta and owned paths, and freeze that exact remote SHA. Candidate-specific qualification/review begins only after freeze. If a material repair is needed, explicitly return to AUTHORING before any further write; only then may high-level API writes create a successor head, which must be frozen and requalified with fresh candidate-specific evidence. Local Git is an optional pre-proven route, not a required fallback. Missing Git credentials or push capability must not trigger Remote Desktop. Never reconstruct a selected candidate through ancestry-only `force=false` ref movement, low-level Git Data or raw Git object assembly, and never write while a head remains frozen. Recovery-specific atomic publication remains governed by bound META policy and is outside the ordinary worker flow.

Classify evidence as `PROVEN`, `DERIVED`, `UNKNOWN` or `CONFLICT`. An unmerged task document cannot authorize architecture, production access or cross-repository writes. Preserve history when later decisions supersede only part of an older contract.

## Integration routing precedence

Integration wording in reusable prompts and routed procedures such as `merge`, `squash merge`, `expected-head merge`, `protected merge` or `auto-merge` describes lifecycle intent only; it does not select or authorize a GitHub merge primitive. For every integration action, resolve the immutable META policy bound by `META_AGENT_POLICY_BINDING.json` and the live repository control-plane capability.

Autonomous Merge Queue submission inherits the authenticated bound META 3.1 native exact-head Merge Queue contract. The selected route is REST `merge-async` with the exact qualified `sha` and explicit `merge_action="merge_queue"`, preceded by fresh target-bound repository, PR, `base=main`, head, authorization and eligibility preflight. A `202` is acceptance only: bind its exact returned UUID to an executor-owned receipt sequence and require an immediate live readback carrying the same UUID at a strictly later executor sequence; timestamps are freshness-only. Reconcile `200` and `409` responses. Queue admission is not terminal proof; require the real `merge_group` aggregate `game-gate` and protected-main readback before closeout. Direct/immediate merge, generic `enablePullRequestAutoMerge`, bypass, force, a default merge action, no-op/retrigger commits and ambiguous dequeue cleanup are forbidden substitutes. If the selected native operation is unavailable, record `BLOCKED_CAPABILITY_UNAVAILABLE`, preserve the qualified candidate, release active waiting ownership as applicable and continue safe path-disjoint work.
