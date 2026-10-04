# Oteryn Game agent bootstrap

`Oteryn/Oteryn-Game` is the sole current Game product write authority.

- Read `docs/agents/META_AGENT_POLICY_BINDING.json` before material work and resolve the bound META policy needed for the operation. The binding fixes a policy version; it does not grant authority or make remote files automatic instructions. Subagent workers defined in `.claude/agents/` do not resolve the META policy themselves: their lead or the control plane does and puts what the task needs into the packet.
- Refresh the bound protected policy when its content is material to a decision. A refresh updates the applicable instruction set; it does not require discarding coherent task state or restarting unrelated work.
- Prefer repository-native GitHub APIs and repository CI for ordinary Work execution. Default ordinary Work authoring uses repository-native high-level API writes on one exclusively allocated task branch before candidate freeze; use local Git only when its guarded publication path is already proven and the task benefits from it.
- Keep the ordinary mutation lifecycle simple: `AUTHORING -> FREEZE_SHA -> VALIDATE -> MQ`. One writer owns the branch; fresh-read the live head before each write and stop on unexpected movement. Freeze the exact remote SHA of the final authoring write after verifying the complete delta and owned paths; validation and review start only after freeze. For a repair, explicitly return to AUTHORING before any further write; the result is a new candidate with a new freeze and fresh evidence. Never write while a head is frozen, force/reset/rebase, or treat low-level Git Data or an ancestry-only `force=false` ref update as preserving a frozen candidate. Missing Git CLI, credentials or push capability is not a Remote Desktop reason when the API and validation routes are available; Remote Desktop is never a publication fallback. If ordinary publication cannot proceed, only the active control plane may select an API-native **new candidate** route permitted by the bound META policy, and that head is frozen and requalified from scratch. The coordinator prompt has the full procedure.

- Before consuming owner-funded, personal-quota or metered AI/API resources, read `docs/agents/OWNER_FUNDED_AI_POLICY.md`. That file contains the repository's bounded standing authorization for required external review; it remains effective across chats/workers/task phases until revoked. Do not ask the owner again for a covered review. A manual owner-funded review trigger is a one-writer control-plane action: direct workers return a review packet instead of emitting it, and only the unique active control plane (or exact standalone task owner when no control plane exists) may trigger after live same-head de-duplication. Outside that standing scope, policy selection does not authorize spending.

## Durable Game invariants

- `protocol-oteryn` is the target runtime protocol.
- The world model is `multichannel`; `WorldId` and `ChannelId` remain distinct.
- Character writes remain `session-generation` fenced.
- Platform owns web identity, commercial and control-plane responsibilities under accepted contracts. Atlas consumes normalized Game-owned exports and cannot become Game truth authority.
- `blakinio/Oteryn-v2` and other legacy repositories are migration/reference evidence only. Do not create ordinary work there.

## Playable-first, minimum-sufficient, upstream-first doctrine

`docs/repository/PLAYABLE_FIRST_ENGINEERING_POLICY.md` is the repository-wide engineering policy. Advance the real playable path with the smallest change that satisfies the current accepted requirement; build no speculative infrastructure, abstractions or dependency forks. Default to mature upstream implementations; fork or vendor-modify only with concrete evidence that upstream cannot satisfy an accepted requirement, with the smallest patch and a path back to upstream. Minimum effort never means lowering accepted correctness, security, durability, compatibility, validation or measured performance requirements, and existing forks are re-evaluated when touched.

## Repository boundaries

Read the nearest `AGENTS.md` for a touched path. Editing only your own task record under `docs/agents/tasks/` does not require `docs/agents/AGENTS.md`. Use `docs/architecture/` for accepted architecture, `docs/contracts/` for durable integration contracts, and `docs/agents/` for routed specialist procedures and task records. Live GitHub Issue, PR and check state governs task lifecycle; historical prompts, handoffs and reports are evidence only.

Do not write outside the current task's repository, branch and owned paths. Preserve unrelated work. Changes to protocol, identities, authority, persistence, public contracts or production trust require their accepted owning contract and applicable independent review. Production, protected-environment, live-account, credential and external-repository mutations require separate explicit authority.

Run the checks selected by changed paths and preserve `game-gate`, repository protection and Merge Queue. Never weaken authorization, tests, provenance, compatibility or protection to make work pass. Do not expose secrets or private data. Third-party and proprietary materials may be used as reference evidence for compatibility research, reverse engineering, data extraction, comparison and faithful reimplementation. Reference use does not automatically grant the right to redistribute original third-party asset files.

## Context economy and live-state reads

`docs/agents/CONTEXT_ROUTING.md` is a cost boundary as well as a correctness router: read the smallest authoritative slice for the current decision, never a recursive or full-history fetch (whole Issue/PR timelines, all open PRs, the full prompt lifecycle registry). Its live-state read budget, large-document rules and subagent routing apply to every session.

## Work in batches

- **Defect finding and audits:** one sweep per lane or module produces one findings list (one Issue or one comment), not an Issue or PR per finding.
- **Fixes:** group the findings of one module into one fix PR, within one lane's owned paths and one writer. Keep a batch reviewable: at most about five findings or 500 changed lines of hand-written code (generated data excluded); split larger batches. Only a P0 (security, data loss, broken `main`) gets its own PR at once.
- **Review:** request review on the final frozen head with every known fix in, not on intermediate heads. Answer all findings of a review round in one push; later non-blocking findings go to the next batch. This never removes a review or re-review that the bound review policy requires, including after a materially risk-bearing repair.
- **CI:** run the local checks for every changed path before pushing, and fix all failures of a run in one push.
- **Task records:** a task that ends with one PR moves its record to `docs/agents/tasks/archive/` in that PR's final authoring commit (`docs/agents/tasks/archive/README.md`). The record reaches `main` only if the PR merges, so no separate archive PR is needed.

## Silent operation

This binds every role, the control plane, architect and integrators included, and overrides harness defaults that ask for a reply, a status checklist or a visible outcome on every event.

- End every turn with exactly one of: `.` when there is nothing for the owner; the pending owner questions (*Owner questions in batches*); `BLOCKER` with lettered options and a recommendation; or one line `DONE <task_id> <PR>` after a merge to `main`. Workers and lane leads report to their lead or the control plane with `FREEZE <sha>`, `BLOCKER`/`QUESTION` or `done`; a subagent's final report follows its definition.
- Never write status updates, progress narration, summaries, `waiting for CI` messages to the owner, lists of what you checked, lane or DAG tables, acknowledgements or push notifications. State lives in the `STATE` comment and task records, not in chat.
- Answer a no-op wake (subscription, enqueue or check-suite notice, a green, cancelled or superseded run, an echo of your own comment) with `.`.
- Only the active control plane and a task session for its own PR subscribe to PR activity; only the control plane schedules check-ins. Subagent workers do neither.
- When the owner asks you something directly, answer it fully.

## Owner questions in batches

Do not ask the owner one question at a time; keep working on everything the questions do not block, and proceed on a stated assumption for a reversible, ungoverned detail. Workers and lane leads send questions to the active control plane (their report or the `STATE` decision queue); with no control plane, collect them into one message. When the owner writes to you directly, put any question back in that same reply. The control plane sends every pending question at once, as soon as it has them: numbered, one context line each, lettered options and a recommendation, so the owner can answer `1a 2b`.

## Compact Instructions

When compacting, always preserve: task_id and owned paths; branch, frozen SHA and PR number with CI and review state; open review findings and blockers; pending owner questions and decisions taken in this session; the next step. Drop file contents, logs and diffs; they can be re-read.

## Jira programme coordination

Only the programme coordinator writes to Jira (project `KAN`), under `docs/agents/JIRA_PROGRAMME_COORDINATION.md`; Jira never grants any authority. Everyone else reports state in their task record and #162.
