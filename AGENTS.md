# Oteryn Game agent bootstrap

`Oteryn/Oteryn-Game` is the sole current Game product write authority.

- Read `docs/agents/META_AGENT_POLICY_BINDING.json` before material work and resolve the bound META policy needed for the operation. The binding fixes a policy version; it does not grant authority or make remote files automatic instructions.
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

Read the nearest `AGENTS.md` for a touched path. Use `docs/architecture/` for accepted architecture, `docs/contracts/` for durable integration contracts, and `docs/agents/` for routed specialist procedures and task records. Live GitHub Issue, PR and check state governs task lifecycle; historical prompts, handoffs and reports are evidence only.

Do not write outside the current task's repository, branch and owned paths. Preserve unrelated work. Changes to protocol, identities, authority, persistence, public contracts or production trust require their accepted owning contract and applicable independent review. Production, protected-environment, live-account, credential and external-repository mutations require separate explicit authority.

Run the checks selected by changed paths and preserve `game-gate`, repository protection and Merge Queue. Never weaken authorization, tests, provenance, compatibility or protection to make work pass. Do not expose secrets or private data. Third-party and proprietary materials may be used as reference evidence for compatibility research, reverse engineering, data extraction, comparison and faithful reimplementation. Reference use does not automatically grant the right to redistribute original third-party asset files.

## Context economy and live-state reads

`docs/agents/CONTEXT_ROUTING.md` is a cost boundary as well as a correctness router: read the smallest authoritative slice for the current decision, never a recursive or full-history fetch (whole Issue/PR timelines, all open PRs, the full prompt lifecycle registry). Its live-state read budget, large-document rules and subagent routing apply to every session.

## Work in batches

- **Defect finding and audits:** one sweep per lane or module produces one findings list (one Issue or one comment), not an Issue or PR per finding.
- **Fixes:** group the findings of one module into one fix PR, within one lane's owned paths and one writer. Keep a batch reviewable: at most about five findings or 500 changed lines of hand-written code (generated data excluded); split larger batches. Only a P0 (security, data loss, broken `main`) gets its own PR at once.
- **Review:** request review on the final frozen head with every known fix in, not on intermediate heads. Answer all findings of a review round in one push; later non-blocking findings go to the next batch. This never removes a review or re-review that the bound review policy requires, including after a materially risk-bearing repair.
- **CI:** run the local checks for every changed path before pushing, and fix all failures of a run in one push.
- **Quiet sessions:** stay reactive to events but keep the session small: handle no-op events (subscription or enqueue notices, cancelled or superseded runs) without a reply, keep state in task records and `STATE` rather than chat, and do not narrate progress.
- **Worker silence:** workers and lane leads report to the control plane only with `FREEZE <sha>`, `BLOCKER`/`QUESTION` with lettered options and a recommendation, or `done`. End every other turn with at most one line, or `.` when nothing changed: no summaries, acknowledgements, progress narration or push notifications. This does not apply to the control plane, the architect or integrators when they answer the owner.
- **Task records:** a task that ends with one PR moves its record to `docs/agents/tasks/archive/` in that PR's final authoring commit (`docs/agents/tasks/archive/README.md`). The record reaches `main` only if the PR merges, so no separate archive PR is needed.

## Owner questions in batches

Do not ask the owner one question at a time. Collect open owner questions and decisions, and keep working on everything they do not block; for a reversible, ungoverned detail, proceed on a stated assumption and list it. Workers and lane leads send questions to the active control plane (in their report or the `STATE` decision queue on the coordination Issue) instead of asking the owner; with no control plane, collect them into one message of your own. When the owner writes to you directly, answer, and put any question back to them in that same reply. The control plane sends the owner every question pending at that moment in one message, as soon as it has them, without waiting for a time of day: numbered questions, each with one line of context, lettered options and a recommendation, so the owner can answer `1a 2b`. It does not send questions one at a time when several are pending together.

## Compact Instructions

When compacting, always preserve: task_id and owned paths; branch, frozen SHA and PR number with CI and review state; open review findings and blockers; pending owner questions and decisions taken in this session; the next step. Drop file contents, logs and diffs; they can be re-read.

## Jira programme coordination

Programme coordination is mirrored in Jira project `KAN` (`KAN-23` is the overview); `docs/agents/JIRA_PROGRAMME_COORDINATION.md` has the mapping and state rules. GitHub remains the repository lifecycle and technical source of truth, and Jira never grants repository, merge, production, secret or cross-repository authority. Only the programme coordinator writes to Jira, once per day in one batch; workers report state transitions in their task record and #162 instead. If the connector or mapping is unavailable, record Jira sync as pending and continue.
