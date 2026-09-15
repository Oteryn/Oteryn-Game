# Terminal-only communication

Autonomous, scheduled, continuation, audit, repair and multi-task invocations default to `user_communication: terminal_only` unless the owner explicitly requests live progress.

## Behavior

- Do not narrate routine preflight, reads, searches, tool calls, commits, PR creation, phase changes, CI observations, merge, archive or next-task selection.
- Persist detailed milestones in task records, commits, PRs, Issues and evidence.
- Send one compact canonical final report at a real stop condition.
- Interrupt earlier only for a required owner decision, new authorization, safety concern, unresolved ownership conflict, material scope approval or required owner action.
- An allowed interruption is at most two short sentences and is not repeated while state is unchanged.

CI pending, routine repair, a worker finishing, a commit, PR creation, merge or archive are milestones, not interruption conditions.

This policy does not prohibit platform-required progress messages imposed by higher-priority instructions; in that case keep them sparse and material.
