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

Autonomous Merge Queue submission must use only a governed route that atomically binds the exact qualified head and the intended protected base/queue state. Direct/immediate merge and generic auto-merge are not substitutes. If integration is otherwise ready but no callable governed atomic Merge Queue route is available, record `BLOCKED_CAPABILITY_UNAVAILABLE`, preserve the qualified head, release active waiting ownership as applicable and continue safe path-disjoint work. Do not weaken protection, create no-op/retrigger commits or mutate merge state to work around a missing enqueue capability.
