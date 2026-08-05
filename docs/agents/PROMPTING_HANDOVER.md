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
