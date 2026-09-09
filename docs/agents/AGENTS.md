# Game agent documentation rules

The root bootstrap and `docs/agents/META_AGENT_POLICY_BINDING.json` supply organization-policy delivery for this tree. Load specialist Game procedures only when their domain or operation applies.

## Ownership and placement

- Live Issues, PRs and checks govern task status. Task packets and shared indexes are reconstruction and coordination records, not a second status authority.
- Put accepted architecture in `docs/architecture/`, durable public integration contracts in `docs/contracts/`, and retained evidence under `docs/agents/evidence/` or immutable workflow artifacts.
- Preserve stable terms: `WorldId`, `ChannelId`, `InstanceId`, `NodeId`, `GameSessionId` and `protocol-oteryn`.
- Do not reintroduce Canary as a target runtime or protocol adapter without an owner-approved ADR that explicitly supersedes ADR-0001.

## Routed Game procedures

Use `tasks/TASK_TEMPLATE.md` for substantial task records. For architecture or contract work, load `ARCHITECTURE_DECISION_DISCIPLINE.md` and the current domain authority. For execution and validation, load only the applicable build matrix, task allocation and nearest path instructions. Multi-agent architecture programmes additionally use `MULTI_AGENT_ARCHITECTURE_ORCHESTRATION.md` when a live allocation invokes it.

Classify evidence as `PROVEN`, `DERIVED`, `UNKNOWN` or `CONFLICT`. An unmerged task document cannot authorize architecture, production access or cross-repository writes. Preserve history when later decisions supersede only part of an older contract.

## Integration routing precedence

Integration wording in reusable prompts and routed procedures such as `merge`, `squash merge`, `expected-head merge`, `protected merge` or `auto-merge` describes lifecycle intent only; it does not select or authorize a GitHub merge primitive. For every integration action, resolve the immutable META policy bound by `META_AGENT_POLICY_BINDING.json` and the live repository control-plane capability.

Autonomous Merge Queue submission inherits the authenticated bound META 3.1 native exact-head Merge Queue contract. The selected route is REST `merge-async` with the exact qualified `sha` and explicit `merge_action="merge_queue"`, preceded by fresh target-bound repository, PR, `base=main`, head, authorization and eligibility preflight. A `202` is acceptance only: bind its exact returned UUID to an executor-owned receipt sequence and require an immediate live readback carrying the same UUID at a strictly later executor sequence; timestamps are freshness-only. Reconcile `200` and `409` responses. Queue admission is not terminal proof; require the real `merge_group` aggregate `game-gate` and protected-main readback before closeout. Direct/immediate merge, generic `enablePullRequestAutoMerge`, bypass, force, a default merge action, no-op/retrigger commits and ambiguous dequeue cleanup are forbidden substitutes. If the selected native operation is unavailable, record `BLOCKED_CAPABILITY_UNAVAILABLE`, preserve the qualified candidate, release active waiting ownership as applicable and continue safe path-disjoint work.
