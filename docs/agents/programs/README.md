# Agent programme records

`docs/agents/programs/` is a **current operational surface**, not a historical ledger.

- Keep a programme/allocation/runbook at the top level only while it is still a current reusable dependency, routing surface or live allocation record.
- Move a terminal, retired or fully superseded execution record to `archive/` only after direct terminal/supersession evidence is available. Age and filename are never enough.
- Register every archived programme Markdown file in `../PROGRAM_LIFECYCLE.json` with terminal evidence and its current successor/source of truth.
- Files under `archive/` are cold provenance. They do not allocate work, grant authority or regain dispatchability because they remain readable.
- Current live allocation truth stays compact in `OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md`; completed checkpoints belong in task/archive/evidence.
- Before moving a programme record, search current prompts/runbooks/tasks for consumers. If a current consumer still requires the record, keep it top-level or update that consumer in the same reviewed change.

Live GitHub Issue/PR/check state, current task packets, accepted architecture/contracts and protected `main` outrank programme prose whenever they advance.
