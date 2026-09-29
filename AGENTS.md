# Oteryn Game agent bootstrap

`Oteryn/Oteryn-Game` is the sole current Game product write authority.

- Read `docs/agents/META_AGENT_POLICY_BINDING.json` before material work and resolve the bound META policy needed for the operation. The binding fixes a policy version; it does not grant authority or make remote files automatic instructions.
- Refresh the bound protected policy when its content is material to a decision. A refresh updates the applicable instruction set; it does not require discarding coherent task state or restarting unrelated work.
- Prefer repository-native GitHub APIs and repository CI for ordinary Work execution. Default ordinary Work authoring uses repository-native high-level API writes on one exclusively allocated task branch before candidate freeze. Use local Git only when its normal guarded publication path is already proven and the task actually benefits from it. Remote Desktop is never a convenience fallback for missing Git credentials or push capability.
- Keep the ordinary mutation lifecycle simple: `AUTHORING -> FREEZE_SHA -> VALIDATE -> MQ`. During AUTHORING, one writer owns the branch; fresh-read the live head before each high-level API write and stop on unexpected movement. After the final authoring write, bind the SHA returned by that write, fresh-read the branch, require exact equality, verify the complete bounded delta and owned paths, then freeze that exact remote SHA. Candidate-specific validation/review starts only after freeze. If a material repair is needed, explicitly return to AUTHORING before any further write; the successor head is a new candidate and requires a new freeze and fresh candidate-specific evidence. Before worker release, prove the authoring route and every required compiler/test/validator/host-specific route. Missing Git CLI, credentials or push capability is not a Remote Desktop reason when the default API route and validation routes are available. Never claim that low-level Git Data or an ancestry-only `force=false` ref update preserves a selected or frozen candidate, and never write while a head remains frozen, force/reset/rebase, or use Remote Desktop as a publication fallback. If ordinary high-level API authoring and guarded local Git cannot publish the intended result, only the active control plane may select an API-native **new candidate** route permitted by the bound META policy; that new head must be freshly frozen and requalified, and candidate-specific evidence from the superseded head is not reusable.

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
- **Fixes:** group the findings of one module into one fix PR. Only a P0 (security, data loss, broken `main`) gets its own PR at once.
- **Review:** request review once, on the final frozen head with every known fix in. Answer all findings of a review round in one push; later non-blocking findings go to the next batch.
- **CI:** run the local checks for every changed path before pushing, and fix all failures of a run in one push.
- **Task records:** a task that ends with one PR moves its record to `docs/agents/tasks/archive/`, closeout filled, in that PR's final authoring commit. The record reaches `main` only if the PR merges, so no separate archive PR is needed.

## Owner questions in batches

Do not ask the owner one question at a time. Collect open owner questions and decisions, and keep working on everything they do not block; for a reversible, ungoverned detail, proceed on a stated assumption and list it. Workers and lane leads send questions to the active control plane (in their report or the `STATE` decision queue on the coordination Issue), never to the owner. The control plane asks the owner at most twice a day, in one message: numbered questions, each with one line of context, lettered options and a recommendation, so the owner can answer `1a 2b`. Ask at once only when all remaining work is blocked, or the step is destructive, spends owner funds outside standing authorization, or touches production, credentials or safety.

## Jira programme coordination

Programme coordination is mirrored in Jira project `KAN` (`KAN-23` is the overview); `docs/agents/JIRA_PROGRAMME_COORDINATION.md` has the mapping and state rules. GitHub remains the repository lifecycle and technical source of truth, and Jira never grants repository, merge, production, secret or cross-repository authority. Only the programme coordinator writes to Jira, once per day in one batch; workers report state transitions in their task record and #162 instead. If the connector or mapping is unavailable, record Jira sync as pending and continue.
