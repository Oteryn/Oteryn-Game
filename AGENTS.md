# Oteryn Game agent instructions

These instructions govern `Oteryn/Oteryn-Game`, the canonical repository for the native game server, native client, protocol/domain code, world/content tooling and Game-owned export contracts.

## Authority and boundaries

- `Oteryn/Oteryn-Game` is the only current Game product write authority.
- Durable Game invariants: `protocol-oteryn` is the target runtime protocol; the world model is `multichannel`; `WorldId` and `ChannelId` remain distinct; character writes remain `session-generation` fenced.
- `blakinio/Oteryn-v2` is historical migration provenance/reference only after source retirement; do not create ordinary work there.
- Platform owns web identity/commercial/control-plane responsibilities defined by accepted cross-repository contracts. Atlas consumes normalized Game-owned exports and may not become Game truth authority.
- Read a nearer `AGENTS.md` for any touched path. A same-directory `AGENTS.override.md` replaces, rather than extends, the base instruction file and therefore must only exist when true replacement semantics are intended.

## GitHub-first execution gate

GitHub is the authoritative control plane for Game repository identity, `main`, Issue/task status, PR, task branch, exact remote SHA, checks, reviews and merge state.

Before any local/remote repository mutation, including work through Remote Desktop/Desktop Commander, Synology, WSL, Docker or a local worktree, the agent MUST first resolve from GitHub the exact repository, current `main` SHA, governing Issue/task (or explicit `NOT_APPLICABLE` for bounded trivial/read-only work), active PR/task branch, exact base/head SHAs and material overlapping work.

Only after that preflight may host-local tooling be used for implementation, builds, tests, containers, Playwright or artifact generation. Local clones, filesystems, worktrees, containers, shell history and cached state are execution/cache planes only and MUST NOT be treated as authority or used to bypass GitHub lifecycle.

Before editing locally, verify remote URL, branch/worktree identity, HEAD and working-tree state against the GitHub-resolved task. Preserve unrelated dirty work. After durable local changes, commit on the authorized task branch, push to GitHub, verify the remote head equals the intended commit, update the PR/task when applicable, and use exact-head GitHub CI/review state for readiness and completion.

Local-only work receives no completion credit until the durable result exists on the approved GitHub branch/PR. If GitHub is genuinely unavailable, continue safe read-only analysis/patch preparation but do not start new product mutations merely to bypass the control plane unless the owner explicitly authorizes an emergency exception.

## Parallel-agent Git concurrency

The organization baseline is META ADR 0004 plus the central agent execution/continuation contract. Game keeps the bootstrap-critical minimum here because repository instructions do not inherit across repositories.

- For substantial mutating work, keep `admission_main_sha`, `task_head_sha` and `integration_main_sha` distinct. `admission_main_sha` is immutable task provenance; `task_head_sha` is the current task-branch head; `integration_main_sha` is the protected `main` selected at final integration.
- One active mutating worker owns one canonical task branch and one writable worktree. Do not share a writable branch/worktree between active agents.
- If protected `main` advances after admission, classify it as `UPSTREAM_ADVANCED`; that movement alone does not invalidate implementation and is not a reason to restart, reset, recreate, rebase, force-push or discard still-applicable work.
- If the upstream delta changes an applicable instruction, safety/security/provenance rule, architecture authority, compatibility contract or invariant, reload and reconcile that governing authority before further mutation while preserving unaffected work.
- Preserve published task history by default. When entering final integration, refresh to current `integration_main_sha` with a normal non-force merge-up, resolve only authorized conflicts, review the resulting diff and rerun every validation/review layer invalidated by the new `task_head_sha`.
- A lost merge race returns the task to integration/reconciliation, not to implementation from scratch.
- Invalidate affected work only when verified task cancellation/supersession/rescope, incompatible governing authority, semantic contract/API/schema/invariant conflict, an unresolvable authorized reconciliation, or required tests prove prior assumptions no longer hold. Textual overlap or a changed filename alone is not sufficient proof.

## Lifecycle

- GitHub Issue is authoritative for substantial task status, dependencies and acceptance criteria.
- Use one independently mergeable task -> one branch -> one PR. Read-only research/review normally creates no branch.
- Markdown task packets may contain bounded technical detail but must not become a second mutable status database.
- Do not push ordinary work directly to `main`.

## Preflight

Before editing, verify current `main`, applicable instructions, active Issue/PR, overlapping work, and the exact architecture/contracts for the owned paths. Treat historical prompts, handovers and reports as evidence, not current lifecycle authority.

## Validation and merge

- Run the repository-selected checks applicable to changed paths.
- Review all changed files and the full diff on the exact final head.
- Preserve protocol/schema/security compatibility and negative-path evidence when those surfaces change.
- Require the current protected Game merge gate on the exact PR head.
- Security-, authorization-, durable-schema-, production-trust- and cross-repository-contract changes require genuinely independent exact-head review when repository policy says so.
- Squash merge only after required checks/reviews pass; delete the merged task branch unless it has a documented continuing provenance role.

## Safety

- Never weaken tests, protection, authorization, provenance or compatibility gates merely to make a task pass.
- Never expose secrets, credentials, private data or proprietary assets.
- Production/protected-environment/live-account mutations require separate explicit authority.
- Do not use owner-funded Codex/OpenAI/API or other metered AI services without explicit owner authorization for that invocation.
- Preserve unique migration/backup history until its retention and restore obligations are explicitly dispositioned.

## Routing

Use `docs/architecture/` for accepted architecture, `docs/contracts/` for durable cross-repository contracts, `docs/agents/` for reusable agent procedures and task packets, and repository workflows/scripts for deterministic validation. Current Issue/PR/check state always outranks stale task prose.