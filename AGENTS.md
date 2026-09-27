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

`docs/repository/PLAYABLE_FIRST_ENGINEERING_POLICY.md` is the repository-wide engineering policy for dependency customization and minimum-sufficient delivery. Advance the real playable Oteryn path with the smallest change that satisfies the current accepted requirement; do not build speculative infrastructure, generalized abstractions, future-scale machinery or dependency forks merely because they may become useful later.

Default to mature upstream implementations and supported configuration/APIs. Do not fork, vendor-modify, reimplement or deeply instrument third-party components without concrete evidence that the exact upstream version cannot satisfy an accepted Oteryn requirement through upstream configuration, extension points or a bounded Oteryn-owned layer. When an exception is proven, use the smallest maintainable patch and preserve a clear path back to upstream.

Minimum effort never means lowering accepted correctness, security, durability, compatibility, validation or measured performance requirements. It means removing unnecessary work, speculative hardening and premature optimization from the critical path to a real playable server. Existing forks are not automatically justified forever and must be re-evaluated when touched or superseded.

## Repository boundaries

Read the nearest `AGENTS.md` for a touched path. Use `docs/architecture/` for accepted architecture, `docs/contracts/` for durable integration contracts, and `docs/agents/` for routed specialist procedures and task records. Live GitHub Issue, PR and check state governs task lifecycle; historical prompts, handoffs and reports are evidence only.

## Context economy and live-state reads

Treat `docs/agents/CONTEXT_ROUTING.md` as a cost boundary as well as a correctness router. A requirement to read, refresh or resolve current state means the smallest authoritative slice needed for the current decision, not a recursive or full-history fetch.

- Do not bulk-fetch complete Issue/PR comment timelines, all open PRs, whole live-allocation history, the full prompt lifecycle registry or every architecture/contract family merely because a reusable prompt names the container.
- For long-lived coordinator Issues, use Issue metadata, the current task/checkpoint and specifically referenced or latest material comments. Read older comments only when a concrete historical claim is material and cannot be resolved from the current checkpoint.
- Resolve one alias through its targeted lifecycle entry; ordinary alias invocation does not require loading the whole registry or re-running prompt evaluation.
- `OTERYN_GAME_AGENT_OPERATOR_RUNBOOK.md` is owner-facing launch/status guidance, not a technical-worker bootstrap dependency unless the current request is owner-facing placement/status guidance.
- Reuse already verified immutable exact-revision sources inside one coherent task. Refresh only changing facts that are material to the next mutation, lifecycle, review or integration decision.

Do not write outside the current task's repository, branch and owned paths. Preserve unrelated work. Changes to protocol, identities, authority, persistence, public contracts or production trust require their accepted owning contract and applicable independent review. Production, protected-environment, live-account, credential and external-repository mutations require separate explicit authority.

Run the checks selected by changed paths and preserve `game-gate`, repository protection and Merge Queue. Never weaken authorization, tests, provenance, compatibility or protection to make work pass. Do not expose secrets, private data or proprietary assets.

## Jira programme coordination

Oteryn programme coordination is mirrored in Jira project `KAN` at `https://oteryn.atlassian.net`; `KAN-23` is the programme overview. When an Atlassian/Jira connector is available in the current session, use it as a bounded programme-coordination surface.

- After the normal GitHub preflight for a substantial start or resume, resolve an **existing mapped Jira Story** for the current workstream. Prefer a native GitHub source link on the Jira item; otherwise require an exact repository/workstream label match. Do not map work by a similar title alone.
- Read only the mapped Story, its parent Epic, priority, status, fixVersion/milestone and readiness labels needed for the current decision. Do not bulk-load unrelated Jira history.
- **GitHub remains repository lifecycle and technical source of truth** for repository identity, Issues/tasks, branches, PRs, exact SHAs, checks, review, Merge Queue and integration. Repository contracts/task records remain implementation authority. Jira is the programme roadmap/readiness/milestone view and never grants repository, merge, production, secret or cross-repository mutation authority.
- Ordinary repository workers may update only their already-mapped programme Story after a verified material state transition. Broad Jira restructuring, new programme Epics/Versions, cross-workstream reprioritization and edits to `KAN-23` belong to the programme coordinator unless the owner explicitly delegates them.
- Use the established programme state convention: queued/blocked/stalled work stays `Do zrobienia` with the matching `readiness-queued`, `readiness-blocked` or `readiness-stalled` label; active work is `W toku` with `readiness-active`; completed work is `Gotowe` with `readiness-complete` only after the Story's full acceptance is verified. Use `W trakcie weryfikacji` when implementation is complete but required review/qualification is still pending.
- Fresh-read both Jira and the linked GitHub state before a Jira mutation. Do not spam comments or rewrite unchanged fields. A closed individual GitHub Issue/PR does not make an aggregate Jira Story complete while another linked acceptance source remains open.
- If the Jira connector is unavailable, the mapping is absent, or Jira write capability is unavailable, continue otherwise-authorized repository work. Record Jira synchronization as pending/unknown rather than inventing a mapping, creating duplicate programme items, or treating Jira availability as an implementation blocker.
