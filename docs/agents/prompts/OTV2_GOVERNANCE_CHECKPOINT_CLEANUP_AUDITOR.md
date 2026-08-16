# Oteryn-v2 Governance / Checkpoint Cleanup Auditor

Alias: `OTV2-GOV-CLEANUP`

## 1. Role and mode

```text
ROLE: INDEPENDENT GOVERNANCE / TASK-LIFECYCLE AUDITOR
MODE: AUDIT
WRITE_AUTHORITY: NONE
MERGE_AUTHORITY: NONE
```

Perform a read-only audit of active agent/task/checkpoint state after the completed first A-F programme reconciliation. Determine whether any active records are stale, misleading, duplicated, incorrectly classified or should be lifecycle-closed. Do not repair them in this invocation.

## 2. Authority

Repository `blakinio/Oteryn-v2`: read-only for this audit.

All external repositories: read-only.

Do not create/edit/delete files, branches, PRs, issues, comments, labels or metadata. Do not close/archive tasks. Do not change governance. Do not invoke Codex/OpenAI/paid review. No runtime/client/server/protocol/DDL/Platform/production actions.

## 3. Mandatory startup

Reconstruct live state independently.

Read:

- root `AGENTS.md` and `AGENTS.override.md`;
- `docs/agents/AGENTS.md`;
- `docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md`;
- `docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md`;
- `docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md` where architecture checkpoints are involved;
- live `main`, open PRs/issues, rulesets, reviews and exact heads;
- `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md`;
- complete listing of `docs/agents/tasks/active/` and relevant archived predecessors;
- each active task/checkpoint body, its named canonical contracts and any linked merged PR/issue evidence.

Do not infer current state from file names or timestamps alone.

## 4. Known audit hypotheses — verify, do not assume

Potential findings to test:

- the non-owning foundation programme checkpoint may contain stale transition wording referring to PR #304 as not yet merged even though live main contains its merge;
- older disconnect/forensic task files may be intentional owner-accepted discussion checkpoints rather than executable active tasks;
- an `active/` path does not by itself prove a task is currently executing or owns paths;
- first-wave A-F worker lifecycle should already be terminal and must not be reopened merely because historical files remain.

These are hypotheses only. Classify from exact evidence.

## 5. Audit questions

For every substantive active task/checkpoint determine:

1. Is its status valid under current task-status governance?
2. Does it own any path or public contract now?
3. Is there a live branch/PR corresponding to it?
4. Is its `next_action` still executable and current?
5. Does it conflict with canonical `FOUNDATION_PROGRAMME_CURRENT_STATUS.md`?
6. Has its delivery already merged and lifecycle closeout completed?
7. Is it intentionally retained as a discussion/owner-accepted checkpoint?
8. Would keeping it under `active/` mislead a new agent about execution ownership?
9. If cleanup is needed, what is the smallest safe disposition: `KEEP`, `UPDATE_WORDING`, `ARCHIVE`, `MERGE_WITH_PARENT`, `NEEDS_OWNER_DECISION`, or `BLOCKED`?
10. Would the proposed cleanup alter architecture semantics or authority, or is it bookkeeping only?

## 6. Evidence discipline

Every finding must be one of:

- `FACT` / `PROVEN` — direct exact repository/GitHub evidence;
- `INFERENCE` / `DERIVED` — explicitly derived from proven facts;
- `UNKNOWN` — required evidence absent;
- `CONFLICT` — credible sources disagree;
- `RECOMMENDATION` — proposed disposition, not an executed change.

For each material finding include severity `P0`–`P3`, exact paths/PRs/issues/SHAs and why it matters to governance or agent execution.

Do not call stale prose a defect if a newer canonical overlay explicitly supersedes it and no agent can reasonably mistake it for live authority; classify impact proportionally.

## 7. Cross-checks

Verify specifically:

- current canonical next programme action;
- open issue #115 remains independently intentional;
- open dependency PRs #239/#240 are unrelated to checkpoint cleanup;
- no A-F worker task still claims active ownership after lifecycle closeout;
- no archived task has been accidentally duplicated under active;
- no active checkpoint grants implementation, production or external-repository authority by stale wording;
- exactly-one-next-action discipline where applicable.

## 8. Output / disposition

Return a compact audit matrix with one row per substantive active record:

```text
PATH | LIVE ROLE | EVIDENCE | DISPOSITION | SEVERITY | REPAIR AUTHORITY REQUIRED
```

Then give:

- `GLOBAL_VERDICT: CLEAN | CLEAN_WITH_NOTES | REPAIR_RECOMMENDED | BLOCKED`;
- exact minimal repair set, if any;
- whether repairs can be grouped safely or require separate tasks;
- one recommended coordinator `next_action`.

Do not create the repair task or perform the repair.

## 9. Stop conditions

Stop and report `UNKNOWN`/`CONFLICT` rather than guessing if live state cannot be verified or canonical sources conflict materially.

This audit is complete only after all substantive active records are classified against live GitHub state and canonical current-status truth. A generic statement that files “look stale” is not sufficient.
