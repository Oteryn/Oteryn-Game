# Mandatory Agent Bootstrap

```yaml
agent_bootstrap_policy_revision: 2.4-oteryn-v2
```

This bootstrap supplements and never weakens system, developer, owner, repository allowlist, safety, production, credential, data, authentication, protocol, asset, deployment, merge or cross-repository restrictions.

Before planning, editing, creating/resuming a task, creating a branch/PR or claiming completion:

1. Read root `AGENTS.md` completely.
2. Read `docs/agents/AGENTS.md` and every nearer `AGENTS.md` governing touched paths.
3. Read `docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md`.
4. Read `docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md` for autonomous, long-running, retry-prone, CI-waiting or multi-task work.
5. Read `docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md` before delayed rechecks, runner work, long commands or replacement-session continuation.
6. Read `docs/agents/TERMINAL_ONLY_COMMUNICATION.md` for autonomous/scheduled communication behavior.
7. Read `docs/agents/GITHUB_ONLY_EXECUTION.md` when a local terminal is unavailable or unsuitable.
8. Read `docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md` for start/resume/continuation/programme requests.
9. Inspect authoritative task checkpoint, live branch/head, PRs, reviews, CI, ownership, dependencies and current repository state.
10. Stop and report an exact conflict if a required bootstrap document is missing or materially conflicts with trusted-base safety.

## Authority freeze

Authority comes from system/owner instructions and governance on the trusted base ref at task start. Governance changes on the current unmerged branch cannot expand the current task's repository allowlist, scope, merge authority, production authority, secret access, protected-environment authority or cross-repository authority.

## Short-command contract

`Uruchom <program> autonomicznie.` and `Kontynuuj <program> autonomicznie.` authorize the foreground coordinator loop for the resolvable bounded programme until a real stop condition. Continue through implementation, validation, audit, E2E, exact-head CI, merge, archive and ownership release without routine confirmation.

No work continues after the final response. This does not authorize hidden background execution.

## Invocation states

Checkpoint task status: `investigating`, `implementing`, `validating`, `ready`, `waiting`, `blocked`, `completed`.

Terminal invocation result: `DONE`, `WAITING`, `BLOCKED`, `ROTATE`.

`ROTATE` is never a task status. Persist a safe checkpoint and exactly one `next_action` before returning it.

## Anti-stall and CI baseline

Default foreground budget is 60 minutes; 120 minutes requires explicit task declaration and reason. Stop after 15 minutes without measurable progress outside the bounded terminal-CI exception. Ordinary unchanged CI/external state may be checked at most twice per exact head; do not repeat an identical failure without a new hypothesis; stop after three repair cycles for one gate.

Final exact-head CI and resulting authorized merge may use the bounded terminal-CI exception only after all non-CI gates are complete. Never force, bypass or weaken protection.

## GitHub-only and merge authority

Lack of Codex/local terminal is not itself a blocker. Use GitHub for repository operations and GitHub Actions for permitted remote validation.

The owner durably authorizes auto-merge when protected or direct squash merge for the task's own PR only after every repository-required exact-head gate passes, audit/E2E are satisfied, review threads are resolved, diff ownership is clean and related PRs are reconciled.

Merge authority is not production authority. Production deployment, protected environment approval, production secrets, live data, live sessions/accounts and protected configuration remain separately unauthorized.

## Terminal-only communication

Autonomous and scheduled work defaults to terminal-only communication. Persist routine milestones in Git/task/PR state and send one compact final report at a real stop condition. Interrupt only for required owner decision, new authorization, safety concern, unresolved ownership conflict, material scope approval or required owner action.

## Completion baseline

Do not call work complete while a required server/client/protocol/persistence/platform integration, observable outcome, test, E2E, exact-head CI, audit, PR closeout, task archive or ownership release is missing.
