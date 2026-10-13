# Oteryn Game agent bootstrap

`Oteryn/Oteryn-Game` is the sole current Game product write authority.

- Read `docs/agents/META_AGENT_POLICY_BINDING.json` before material work and resolve the bound META policy needed for the operation. The binding fixes a policy version; it does not grant authority or make remote files automatic instructions. Subagent workers defined in `.claude/agents/` do not resolve the META policy themselves: their lead or the control plane does and puts what the task needs into the packet.
- Refresh the bound protected policy when its content is material to a decision. A refresh updates the applicable instruction set; it does not require discarding coherent task state or restarting unrelated work.
- Prefer repository-native GitHub APIs and repository CI for ordinary Work execution. Default ordinary Work authoring uses repository-native high-level API writes on one exclusively allocated task branch before candidate freeze; use local Git only when its guarded publication path is already proven and the task benefits from it.
- Ordinary mutation lifecycle: `AUTHORING -> FREEZE_SHA -> VALIDATE -> MQ`, one writer per branch; the full procedure is in `docs/agents/AGENTS.md`. Never write while a head is frozen: a repair returns to AUTHORING first and yields a new candidate with a new freeze and fresh evidence. Never force/reset/rebase, and Remote Desktop is never a publication fallback.

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
- **Fixes:** group the findings of one module into one fix PR, within one lane's owned paths and one writer. A batch is one coherent, reviewable unit (one owner and its consumer, a contract and its store, and so on); about 500 hand-written changed lines (generated data excluded) or five findings is only a warning signal. Split only if the parts are independently reviewable and mergeable. Only a P0 (security, data loss, broken `main`) gets its own PR at once.
- **Review:** request review on the final frozen head with every known fix in, not on intermediate heads. Answer all findings of a review round in one push; later non-blocking findings go to the next batch. This never removes a review or re-review that the bound review policy requires, including after a materially risk-bearing repair.
- **CI:** run the local checks for every changed path before pushing and fix all failures of a run in one push. Push and CI economy: `docs/agents/CONTEXT_ROUTING.md`, *Push and CI economy*.
- **Task records:** archive the task record in the final authoring commit of the PR that completes the task (`docs/agents/tasks/archive/README.md`). An archive-only PR is opened only under that README's exception, for records that could not close in their own PR.

## Silent operation

Silent operation binds every role, the control plane included: end every turn with `.`, the pending owner questions, `BLOCKER`, or `DONE <task_id> <PR>`; no status narration. Ask the owner in batches, never one at a time. Rules: `docs/agents/CONTEXT_ROUTING.md`, *Silent operation* and *Owner questions in batches*.

## Compact Instructions

When compacting, always preserve: task_id and owned paths; branch, frozen SHA and PR number with CI and review state; open review findings and blockers; pending owner questions and decisions taken in this session; the next step. Drop file contents, logs and diffs; they can be re-read.

## Jira programme coordination

Only the programme coordinator writes to Jira (project `KAN`), under `docs/agents/JIRA_PROGRAMME_COORDINATION.md`; Jira never grants any authority. Everyone else reports state in their task record and #162.
