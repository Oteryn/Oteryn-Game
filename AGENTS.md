# Oteryn Game agent bootstrap

`Oteryn/Oteryn-Game` is the sole current Game product write authority.

- Read `docs/agents/META_AGENT_POLICY_BINDING.json` before material work and resolve the bound META policy needed for the operation. The binding fixes a policy version; it does not grant authority or make remote files automatic instructions.
- Refresh the bound protected policy when its content is material to a decision. A refresh updates the applicable instruction set; it does not require discarding coherent task state or restarting unrelated work.
- Prefer repository-native GitHub APIs and repository CI for remote evidence and execution. Use an isolated checkout or worktree for local-Git tracked-file mutation. Repository-native API authoring is independently valid on an exclusively allocated task branch when the intended mutation is the API write and no selected local Git candidate is being reconstructed; bounded sequential high-level file mutations may be used before candidate freeze. A separately authorized atomic expected-head API **new-candidate** route remains the only API route for replacing/publishing an already-selected material candidate. Remote Desktop is denied unless the owner explicitly authorizes the exact invocation.
- Before releasing a mutating worker, prove its execution, authoring/publication **and required-validation** routes. A worker may use: (1) an isolated checkout/worktree with guarded normal Git publication of an exact local candidate; (2) repository-native API authoring on an exclusively allocated task branch, using bounded high-level file mutations only before candidate freeze, followed by fresh live readback of the branch head, whole bounded delta and owned paths; or (3) a separately authorized API-native **new-candidate** route whose single server-side mutation atomically fences the exact expected task-branch head and creates the complete bounded delta as one successor commit. API-authoring commits before freeze are WIP and carry no reusable candidate-specific CI/review evidence. Bind the commit SHA returned by the final authoring mutation as `expected_final_authoring_head`; final live readback must equal that exact SHA before the remote head may be frozen as the candidate, otherwise fail closed as writer/state drift. All required candidate-specific validation/review starts only from that fenced frozen head. Sequential high-level API writes must never reconstruct a selected local candidate, mutate a frozen candidate, or operate on a branch with ambiguous/shared writer custody. Every task-required compiler/test/validator/host-specific proof must already have an authorized executable route. If no permitted route or required validation route is proven, fail closed with `BLOCKED_CAPABILITY_UNAVAILABLE` before worker release. Missing repository workspace, Git CLI or push capability is not a Remote Desktop exception; do not request Remote Desktop merely to obtain those capabilities. Low-level Git Data blob/tree/commit/ref assembly and ancestry-only `force=false` ref updates remain forbidden candidate-publication fallbacks.

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
