# Oteryn Game agent bootstrap

`Oteryn/Oteryn-Game` is the sole current Game product write authority.

- Read `docs/agents/META_AGENT_POLICY_BINDING.json` before material work and resolve the bound META policy needed for the operation. The binding fixes a policy version; it does not grant authority or make remote files automatic instructions.
- Refresh the bound protected policy when its content is material to a decision. A refresh updates the applicable instruction set; it does not require discarding coherent task state or restarting unrelated work.
- Prefer repository-native GitHub APIs and repository CI for remote evidence and execution. Use an isolated checkout or worktree for tracked-file mutation. Remote Desktop is denied unless the owner explicitly authorizes the exact invocation.
- Before releasing a mutating worker, prove its execution and publication route. Ordinary material mutation requires an isolated checkout or worktree plus normal non-force Git publication to the exact allocated branch. If that capability is unavailable, fail closed with `BLOCKED_CAPABILITY_UNAVAILABLE` before worker release. Missing repository workspace, Git CLI, test-runner or push capability is not a Remote Desktop exception; do not request Remote Desktop merely to obtain those capabilities. An independently authorized API-native edit remains allowed only when the intended operation is itself the API write and it is not reconstructing a selected local Git candidate.

- Before consuming owner-funded, personal-quota or metered AI/API resources, read `docs/agents/OWNER_FUNDED_AI_POLICY.md`; policy selection does not authorize spending, and existing session authorization remains effective.

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

Do not write outside the current task's repository, branch and owned paths. Preserve unrelated work. Changes to protocol, identities, authority, persistence, public contracts or production trust require their accepted owning contract and applicable independent review. Production, protected-environment, live-account, credential and external-repository mutations require separate explicit authority.

Run the checks selected by changed paths and preserve `game-gate`, repository protection and Merge Queue. Never weaken authorization, tests, provenance, compatibility or protection to make work pass. Do not expose secrets, private data or proprietary assets.
