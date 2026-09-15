# Prompting handover

A handover must allow continuation without the previous chat.

## Required durable state

- task/programme ID and mode;
- repository, branch, base/head SHA and PR;
- current task status and terminal invocation result if stopping;
- owned paths/contracts and unresolved overlaps;
- completed work and exact changed paths;
- validation commands/runs/results tied to SHA;
- audit/E2E/review state;
- dependencies/blockers/decisions;
- anti-stall counters and wait generation where applicable;
- exactly one concrete `next_action`.

## Resume prompt

The next agent must be told to read trusted-base governance, task checkpoint and live PR/CI state first; verify drift; then execute the recorded next action. Do not paste entire logs or source files when immutable identifiers suffice.

## Quality rule

A handover saying only `continue`, `finish CI` or `check status` is invalid. Name the exact branch/PR/head/gate and action.

## Retained handover lifecycle

Retained handoff/handover records under `docs/agents/evidence/` or `docs/agents/reports/` are historical continuation evidence, not current authority. Every retained record matching that role must have exactly one entry in `docs/agents/HANDOVER_LIFECYCLE.json` with `authoritative: false`, a deterministic `expiry_rule` and at least one `superseded_by` source.

A retained handover expires for current-state authority as soon as any recorded branch, PR, head, ownership, blocker or accepted programme state changes. Before continuation, resolve live GitHub Issue/PR/CI state and current `main` again. Current repository governance and accepted architecture/contracts outrank every handover.

A handover never grants write authority, production access, external-repository authority or permission to reuse a stale allocation. Historical handover files should be preserved rather than rewritten to look current; lifecycle metadata carries the explicit non-authority and supersession state.
