# Oteryn Game agent instructions

These instructions govern `Oteryn/Oteryn-Game`, the canonical repository for the native game server, native client, protocol/domain code, world/content tooling and Game-owned export contracts.

## Authority and boundaries

- `Oteryn/Oteryn-Game` is the only current Game product write authority.
- Durable Game invariants: `protocol-oteryn` is the target runtime protocol; the world model is `multichannel`; `WorldId` and `ChannelId` remain distinct; character writes remain `session-generation` fenced.
- `blakinio/Oteryn-v2` is historical migration provenance/reference only after source retirement; do not create ordinary work there.
- Platform owns web identity/commercial/control-plane responsibilities defined by accepted cross-repository contracts. Atlas consumes normalized Game-owned exports and may not become Game truth authority.
- Read a nearer `AGENTS.md` for any touched path. A same-directory `AGENTS.override.md` replaces, rather than extends, the base instruction file and therefore must only exist when true replacement semantics are intended.

## External execution-skill precedence

Repository and user authority govern execution. Agent skills, plugins and workflow frameworks such as Superpowers are subordinate execution aids, not independent task or lifecycle authority.

For an already-authorized Oteryn programme or task with an approved canonical design, implementation plan, checkpoint, or explicit continuation directive, Superpowers workflows MUST NOT introduce additional approval gates, re-brainstorm an approved design, require duplicate planning artifacts, replace canonical authority, or interrupt autonomous continuation solely because the skill's default workflow would do so. Relevant skills MAY still be used internally for implementation, testing, debugging, review, isolation, or verification when they do not conflict with the governing Oteryn authority.

A skill or plugin MUST NOT weaken repository safety, validation, review, GitHub-first, or authorization requirements. When a skill workflow conflicts with applicable user instructions, this `AGENTS.md`, repository policy, or canonical task authority, the applicable higher-priority Oteryn authority controls.

## GitHub-first execution gate

GitHub is the authoritative control plane for Game repository identity, `main`, Issue/task status, PR, task branch, exact remote SHA, checks, reviews and merge state.

Before any local/remote repository mutation, including work through Remote Desktop/Desktop Commander, Synology, WSL, Docker or a local worktree, the agent MUST first resolve from GitHub the exact repository, current `main` SHA, governing Issue/task (or explicit `NOT_APPLICABLE` for bounded trivial/read-only work), active PR/task branch, exact base/head SHAs and material overlapping work.

Only after that preflight may host-local tooling be used for implementation, builds, tests, containers, Playwright or artifact generation. Local clones, filesystems, worktrees, containers, shell history and cached state are execution/cache planes only and MUST NOT be treated as authority or used to bypass GitHub lifecycle.

Before editing locally, verify remote URL, branch/worktree identity, HEAD and working-tree state against the GitHub-resolved task. Preserve unrelated dirty work. After durable local changes, commit on the authorized task branch, push to GitHub, verify the remote head equals the intended commit, update the PR/task when applicable, and use exact-head GitHub CI/review state for readiness and completion.

Local-only work receives no completion credit until the durable result exists on the approved GitHub branch/PR. If GitHub is genuinely unavailable, continue safe read-only analysis/patch preparation but do not start new product mutations merely to bypass the control plane unless the owner explicitly authorizes an emergency exception.

## Capability truthfulness and tool discovery

Technical execution capability is determined by the tools, connectors and actions actually exposed in the current session, not by assumptions about Chat, Work, Codex or another UI mode. A rejected handoff, missing local checkout, missing `gh`, unauthenticated local CLI, or an earlier agent statement is not proof that GitHub or write capability is unavailable.

Before reporting that GitHub is read-only, commit/push/PR cannot be performed, Work mode is required, or repository work cannot continue, inspect all relevant currently exposed tools/actions and available authentication/permission evidence. Prefer repository-native GitHub operations for repository lifecycle work. If the preferred route fails, evaluate safe authorized fallbacks before asking the owner to switch modes or perform work manually.

Classify a real limitation precisely as missing tool/action, unauthenticated context, permission denied, unsupported operation, repository/policy restriction, transient transport/service failure, or another directly observed condition. Do not generalize one failed action into a broader capability claim. If the capability has not been checked, record it as `UNKNOWN` and perform discovery rather than presenting it as a blocker.

Capability discovery MUST be observational and least-mutating. Do not create throwaway branches, files, commits, comments, PRs, workflow runs, deployments or other durable state merely to prove write access. A genuine blocker report must name the exact operation, tool/connector/action inspected or attempted, observed failure, checked safe authorized fallbacks, and smallest missing capability or permission.

Remote Desktop/Desktop Commander remains exception-only under the organization execution-routing policy and is not the routine fallback for repository work. Tool availability never grants or broadens authorization.

## Parallel-agent Git concurrency

The organization baseline is META ADR 0004 plus the central agent execution/continuation contract. Game keeps the bootstrap-critical minimum here because repository instructions do not inherit across repositories.

- For substantial mutating work, keep `admission_main_sha`, `task_head_sha` and `integration_main_sha` distinct. `admission_main_sha` is immutable task provenance; `task_head_sha` is the current task-branch head; `integration_main_sha` is the protected `main` selected at final integration.
- One active mutating worker owns one canonical task branch and one writable worktree. Do not share a writable branch/worktree between active agents.
- If protected `main` advances after admission, classify it as `UPSTREAM_ADVANCED`; that movement alone does not invalidate implementation and is not a reason to restart, reset, recreate, rebase, force-push or discard still-applicable work.
- If the upstream delta changes an applicable instruction, safety/security/provenance rule, architecture authority, compatibility contract or invariant, reload and reconcile that governing authority before further mutation while preserving unaffected work.
- Preserve published task history by default. When entering final integration, refresh to current `integration_main_sha` with a normal non-force merge-up, resolve only authorized conflicts, review the resulting diff and rerun every validation/review layer invalidated by the new `task_head_sha`.
- A lost merge race returns the task to integration/reconciliation, not to implementation from scratch.
- Invalidate affected work only when verified task cancellation/supersession/rescope, incompatible governing authority, semantic contract/API/schema/invariant conflict, an unresolvable authorized reconciliation, or required tests prove prior assumptions no longer hold. Textual overlap or a changed filename alone is not sufficient proof.

## META execution-routing policy

The canonical organization policy is [`Oteryn/Oteryn@e002fc7532188e73a0f495da3e20710541ed50e0:ecosystem/agent-execution-routing-policy.json`](https://github.com/Oteryn/Oteryn/blob/e002fc7532188e73a0f495da3e20710541ed50e0/ecosystem/agent-execution-routing-policy.json). It is adopted by reference; do not fork or weaken it in Game.

For project work, use GitHub state, GitHub Actions or an approved runner, and an isolated worktree first. Remote Desktop/Desktop Commander is default-deny. A host exception must record one closed reason, the least-privilege semantic host action and the exact requested connector tool; it is never justification for routine builds, tests, Git inspection or polling. When equivalent CI exists, agents MUST NOT use RDC to poll process output, Docker logs, workflow state or Git state.

Before resuming work, refresh the current GitHub repository, default-branch SHA, governing Issue, PR and task-head facts. Existing worktrees and handoffs are evidence only. A substantial task packet must plan parallel-first: independent lanes, exclusive branch/worktree and owned paths, dependencies, any shared-resource lease, and integration order. Serial work requires an explicit reason.

## Remote Desktop execution routing

Before any Remote Desktop/Desktop Commander use, resolve this Game `AGENTS.md` and the canonical META execution-routing policy at `Oteryn/Oteryn@e002fc7532188e73a0f495da3e20710541ed50e0`. Out-of-band local connector/tool registration and argument-schema inspection is capability discovery; every direct `Remote_Desktop_Commander.*` invocation is exception-only and requires a fresh valid host-exception context plus a positive per-action decision for the exact semantic host action and exact connector tool immediately before the call.

`list_devices`, `who_am_i`, `ping`, `get_config`, filesystem/search/process/session/terminal/history operations and other direct connector calls are not capability-discovery exemptions. Unknown or undeclared tools fail closed, and a prior ALLOW never authorizes a different action or tool. Game cannot broaden META exception reasons or use Remote Desktop as a routine fallback for repository tests, Git inspection, CI/log polling or convenience. A Remote Desktop DENY is not automatically a blocker: continue through GitHub, GitHub Actions, repository-native connectors or an isolated workspace when they can perform useful authorized work.

This provider binding is repository/prompt enforcement only. It MUST NOT be described as connector/router physical enforcement unless the actual Remote Desktop transport has a verified fail-closed hook consuming the same per-action semantics.

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

## AI review policy — META-owned

Game adopts the current organization AI review policy from protected `Oteryn/Oteryn:docs/governance/AI_REVIEW_POLICY.md` by reference and must not maintain a competing local review-routing authority.

- Default: no external AI review.
- Ordinary code change with clear independent-review value: prefer Codex Spark when available.
- Material high-risk/control-plane change: use one Codex deep review on a stable material candidate.
- Trivial docs, formatting, generated evidence, metadata and other low-risk changes: no external AI review.
- External AI review is advisory and never a required status or GitHub merge authority; `game-gate`, repository protection and Merge Queue remain the enforcement path.
- Do not recreate local R0/R1/R2 tiers, review fingerprints, review envelopes, attestations, standing review controllers or equivalent merge authority.
- Re-review only when a material risk-bearing repair makes the prior review no longer representative.

This section is the current repository-wide AI-review authority and supersedes conflicting standing-authorization, review-tier or Work/Terra controller prose in older `docs/agents/**` policies/prompts. Those legacy files may remain as historical/procedural evidence but do not override this rule.

## Safety

- Never weaken tests, protection, authorization, provenance or compatibility gates merely to make a task pass.
- Never expose secrets, credentials, private data or proprietary assets.
- Production/protected-environment/live-account mutations require separate explicit authority.
- Metered AI/API use outside the central AI review policy still requires the task-specific authority applicable to that use; mere availability of credentials never grants authorization.
- Preserve unique migration/backup history until its retention and restore obligations are explicitly dispositioned.

## Routing

Use `docs/architecture/` for accepted architecture, `docs/contracts/` for durable cross-repository contracts, `docs/agents/` for reusable agent procedures and task packets, and repository workflows/scripts for deterministic validation. Current Issue/PR/check state always outranks stale task prose.
