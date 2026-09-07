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
