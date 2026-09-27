---
task_id: OTV2-20260927-resume-deadline
title: Bound the resume candidate deadline by its verified evidence
mode: IMPLEMENT
status: completed-on-merge
repository: Oteryn/Oteryn-Game
base_branch: main
base_sha: ee85b808
branch: agent/resume-deadline-20260927
issue: 162
jira: KAN-13
allocation_comment: 5859328381
owned_paths:
  - apps/game-server/src/gameplay_transport/resume.rs
  - docs/agents/tasks/archive/OTV2-20260927-resume-deadline.md
---

# Bound the resume candidate deadline by its verified evidence

This is a #822 follow-up to 5b (#1000) and 5c (#1010).

## Defect

The node-boot qualification intermittently refused a valid same-session resume (`same-session resume diverged: Closed`). It failed on the #1010 CI run and in two of seven local runs, reproduced under CPU load.

Instrumentation located the refusal at `CompleteReconnectAuthorizationV1::authorize`. It rejects a candidate whose `prepared_deadline` exceeds the verified credential's `accepted_deadline`.

- The candidate deadline was `now + 5`, where `now` is the durable time sampled after the recovery evidence refresh.
- The accepted deadline is at most `source_observed_at + 5 - clock_uncertainty`.

Any second boundary, or any delay between the two samples, therefore refused the resume. The defect has existed since 5b. 5c's second resume stage made it more frequent.

## Outcome

The candidate deadline is `now + 5`, clamped to the verified `accepted_deadline` and to the loss's original grace deadline. The Foundation refuses a deadline past either. Behaviour is otherwise unchanged:
- stale evidence (`accepted_deadline < now`) still makes the candidate not live, and the resume is refused;
- COMMIT still happens within the candidate lifetime.

Validation:
- node boot passes under CPU load twice, where it failed one in two before the fix;
- the local seam passes;
- the full test suite and clippy pass.
