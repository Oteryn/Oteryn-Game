# Oteryn v2 Agent Instructions

## Instruction order

1. System, developer and explicit owner instructions.
2. This root `AGENTS.md`.
3. `AGENTS.override.md`.
4. The nearest nested `AGENTS.md` governing a touched path.
5. `docs/agents/AGENTS.md`, `REPOSITORY_MAP.md` and `CONTEXT_ROUTING.md`.
6. The active task checkpoint, live branch/PR and linked architecture/contracts.

When instructions overlap, follow the more restrictive safety, ownership and validation rule.

## Repository allowlist — highest priority

- Routine autonomous writes authorized by this repository policy are limited to `blakinio/Oteryn-v2`.
- `blakinio/Oteryn-Platform`, `blakinio/Otheryn`, `blakinio/otclient`, Canary repositories and all other repositories are read-only unless the owner explicitly authorizes a write task for that exact repository.
- Before every GitHub write, verify `repository_full_name == blakinio/Oteryn-v2` unless the current owner instruction explicitly authorizes another repository.
- Cross-repository work requires one task, branch and PR per repository plus a shared coordination ID and explicit rollout order.
- Never use a governance edit on an unmerged branch to expand the current task's authority.

## Product boundary

Oteryn v2 is the greenfield native Rust gameplay stack:

- an authoritative Rust game server;
- a native Rust game client;
- shared Rust domain, protocol and tooling crates;
- one project-owned gameplay protocol: `protocol-oteryn`;
- a multichannel-first world model.

The existing Oteryn Platform remains the external source of truth for web portal, Identity, OAuth/PKCE, Game Login Tickets, Game Gateway and World Registry until a separately accepted migration changes that boundary.

Otheryn C++ and the current otclient repository are reference/migration sources, not implicit runtime dependencies. `protocol-canary` and legacy Tibia packet compatibility are not part of the target Oteryn v2 runtime.

## Non-negotiable architecture

- One logical world may contain multiple gameplay channels.
- `WorldId`, `ChannelId`, `InstanceId`, `ZoneId`, `NodeId` and `GameSessionId` are distinct identities.
- Each channel has one logical authoritative mutation owner.
- World-shared state and channel-local state must have explicit owners and consistency models.
- No mutable gameplay state may be process-global without an accepted owner and scope.
- Character writes require session-generation fencing; one character may have at most one active authoritative session.
- Client sends intent; server owns legality, ordering and results.
- Gameplay domain code must not depend on wire layouts, renderer state or UI widgets.
- Rulesets such as classic 7.6/8.0 and modern 15+ are data/policy profiles over `protocol-oteryn`, not separate protocols or engine forks.
- House topology remains provisional; preserve the accepted one-state-per-world anti-duplication invariants and do not freeze a final topology without a dedicated ADR.

Read the accepted architecture documents under `docs/architecture/` before changing these boundaries.

## Mandatory startup

Before substantial work:

1. Read this file and `AGENTS.override.md`.
2. Read `docs/agents/AGENTS.md`, `REPOSITORY_MAP.md`, `BUILD_TEST_MATRIX.md` and task-routed policies.
3. Inspect the exact current default-branch head, open PRs, active tasks and overlapping owned paths/contracts.
4. When continuing, read the task `## Context checkpoint` and verify its branch, PR, exact head, CI and next action against live state.
5. Search existing code, crates, contracts and ADRs before creating a new abstraction.
6. Record `PROVEN`, `DERIVED`, `UNKNOWN` and `CONFLICT` truthfully; never convert missing evidence into an assumption.
7. When a local checkout exists, verify branch, upstream, worktree and uncommitted changes before editing.

Do not recursively load unrelated documentation.

## Task visibility and concurrency

For substantial work:

- create `docs/agents/tasks/active/OTV2-YYYYMMDD-short-slug.md` from `TASK_TEMPLATE.md`;
- declare `owned_paths`, public contracts, dependencies, blockers, external repositories and execution budget;
- use one dedicated branch/worktree per task;
- open a draft PR early when possible;
- keep one compact `## Context checkpoint` with exactly one `next_action`;
- treat owned paths as advisory locks and resolve overlap before editing;
- avoid unrelated cleanup and broad formatting churn;
- archive the task after terminal completion and release ownership.

Chat history is disposable. Git, task records, contracts, PRs and exact validation evidence are authoritative.

## Delivery and merge

Default workflow:

1. preflight and claim scope;
2. create task record and dedicated branch;
3. implement the smallest complete result;
4. run focused validation;
5. open/update PR and inspect full diff;
6. run required exact-head CI and E2E;
7. perform an independent audit;
8. resolve review threads and reconcile related PRs;
9. squash-merge only when every gate passes;
10. archive task and release ownership.

Never push feature, fix or documentation task work directly to `main`.

Merge only when:

- base and head repositories are `blakinio/Oteryn-v2` and base is `main`;
- the changed-file list is within declared scope;
- acceptance criteria and observable outcomes are satisfied;
- required focused/component/E2E checks pass;
- required GitHub checks pass on the exact unchanged head;
- audit has no open material finding;
- no requested change, unresolved review thread, ownership conflict, migration hold or cross-repo ordering hold remains;
- task, architecture and contracts are current.

Use squash merge. Never force, bypass protection, weaken tests or mark failures successful.

## Rust implementation policy

- Use stable Rust unless an accepted ADR and toolchain file require otherwise.
- Discover the actual workspace and commands from repository files; do not invent a Cargo layout before bootstrap.
- Keep domain crates independent from transport, persistence implementation, UI and rendering.
- Prefer explicit typed IDs, bounded inputs and deterministic state transitions.
- Avoid `unsafe`; every unavoidable `unsafe` block requires a documented invariant, focused tests and review.
- Do not block async executors with filesystem, network, database, compression or heavy CPU work.
- External parsers and protocol decoders require size/depth limits, negative tests and fuzz/property coverage where practical.
- New dependencies require justification, maintenance/security review and least-capability feature selection.
- Do not commit generated build outputs, local caches or proprietary assets.

## Multichannel and durable-state safety

Treat these changes as high risk:

- character leases, login/relog and session recovery;
- inventory, loot, market, bank, depot and item transfers;
- channel switching and anti-hopping rules;
- PvP locks, skulls, frags and rewards;
- houses and instances;
- protocol sequencing, snapshots, deltas and reconciliation;
- persistence schemas, migrations and crash recovery.

Require idempotency, revision/session fences, deterministic tests and failure-path acceptance appropriate to the change. A stale node or channel must never overwrite newer character, item or house state.

## Security, secrets and assets

- Never commit secrets, credentials, tokens, private keys, cookies, production endpoints, personal data, dumps or backups.
- Production deployment, protected-environment approval, live account/session mutation and live database changes require separate explicit authority.
- Authentication and session changes fail closed and require replay, expiry, revocation and cross-world/channel misuse tests.
- Do not introduce client-authoritative inventory, damage, loot, movement legality or currency.
- Do not commit proprietary Tibia/CipSoft assets without confirmed rights and provenance.
- Treat download, updater, archive, TLS, manifest and asset-pack verification as security-sensitive.

## Validation truthfulness

- Select validation from `docs/agents/BUILD_TEST_MATRIX.md` and actual workspace/workflows.
- Review the full diff before readiness.
- Record exact commands, outcomes and commit SHA.
- A passing check proves only the exact code/configuration it executed.
- Do not claim runtime, multichannel, protocol, persistence, platform integration or performance success without named evidence.
- Documentation-only changes still require governance/link validation on the exact final head.

## PR communication

Agents may update and discuss their own PRs when required for delivery. Do not comment on unrelated PRs except for explicit overlap/dependency coordination. Repository PR communication must be in English.

## GitHub connector routing — mandatory

- For GitHub repository, pull request, issue, review, and remote-file tasks, inspect and use the connected GitHub plugin or connector before falling back to local `git` or `gh`.
- Treat an explicit `@GitHub` selection as a request to use the connected GitHub plugin.
- Local `git` may be used for checkout, worktree, diff, branch, and commit operations. Use `gh` only for operations the connector does not support or when repository policy explicitly requires it.
- A missing local checkout, missing `gh` binary, or unauthenticated local `gh` session is not evidence that the GitHub connector is unavailable.

Before claiming that GitHub access is unavailable:

1. Inspect the available GitHub connector tools.
2. Call `github_get_user_login` or the equivalent authenticated-identity operation.
3. Call `github_get_repo` or `github_list_repositories` for the requested repository scope.
4. Attempt the required read operation through the connector when it is safe to do so.

Report a GitHub access blocker only after an actual connector call returns an authentication or permission error. Include the exact failed operation and error.
