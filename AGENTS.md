# Oteryn Game Agent Instructions

## Owner-funded AI and credential budget — highest priority

- Agents MUST NOT invoke Codex, OpenAI API, paid/limited AI review services, or any other mechanism that consumes the repository owner's personal AI quota, credits, tokens, subscription limits, or metered allowance unless the owner gives explicit permission for that specific use.
- Agents MUST NOT use, export, copy, inspect, forward, or authenticate with owner-supplied API keys, access tokens, session tokens, personal credentials, or secrets for AI/model services unless the owner explicitly authorizes that exact credential/service use.
- Availability of a credential, environment variable, CLI login, browser session, connector, MCP/plugin, or previously granted access does NOT constitute permission to consume owner-funded AI resources.
- Prior permission is not standing permission. Authorization must be explicit for the current task/use; if scope, provider, model, or expected consumption materially changes, ask again.
- If a workflow, policy, review gate, script, or tool would normally invoke Codex or another owner-funded AI service, skip that invocation and use a non-owner-funded alternative when one is genuinely available. If the requirement cannot be satisfied without such use, stop and report the exact blocker instead of consuming quota.
- When a genuinely independent review can be satisfied by a fresh separate agent/session that did not implement or materially author the change, prefer that mechanism by default instead of Codex.
- If an agent judges that Codex would be materially more efficient or effective for a particular implementation, review or audit, the agent MUST first inform the owner, explain the expected advantage, and provide a ready-to-run bounded Codex prompt for that exact task. The agent MUST NOT invoke Codex until the owner explicitly authorizes that exact use.
- Never weaken, bypass, or falsely mark a review/validation gate as satisfied merely because owner-funded AI use is forbidden.


## Central Spark PR pre-review — standing owner authorization

- The owner explicitly authorizes the central controller in `blakinio/github-projects-control` to perform recurring advisory PR pre-review for this repository using exactly `gpt-5.3-codex-spark` through ChatGPT-managed Codex authentication on its trusted private runner. This is a standing, bounded repository-automation exception to the owner-funded AI restriction above; it does **not** authorize repository agents to invoke Codex, OpenAI API, hosted Code Review, or any other AI service themselves.
- The central controller may inspect only bounded PR metadata/diff text and may post only concrete P0/P1 findings. A clean Spark pass is intentionally silent. Target PR code is not checked out or executed by the Spark runner.
- Keep a PR Draft while implementation is still in progress. Mark it Ready only when this repository's normal readiness rules already permit that transition. The controller considers only eligible ready, internal, non-bot, exact-head, green-CI, bounded changes.
- Do not automatically request `@codex review`, enable hosted Codex Automatic Reviews, invoke Codex CLI, use `OPENAI_API_KEY`, or select another model/provider as a fallback. Any such direct AI use still requires separate explicit owner authorization for the current task/use.
- Spark pre-review is advisory and does not replace self-review, required independent review, required checks, E2E/runtime evidence, branch protection, or any merge gate. Never infer that Spark ran or passed merely because no comment appeared. Do not delay or weaken a repository merge gate solely to manufacture Spark evidence.
- If the central controller posts a P0/P1 finding before merge, treat it as an unresolved material review finding: address or explicitly disposition it under the repository's normal review rules, then rerun any validation invalidated by the resulting change.
- `no-spark-review` opts a PR out of the central controller. `spark-review` may force consideration of an otherwise ignored path class, but it never bypasses draft, fork, bot, CI, exact-head, size, model, or safety fences.

## Instruction order

1. System, developer and explicit owner instructions.
2. This root `AGENTS.md`.
3. `AGENTS.override.md`.
4. The nearest nested `AGENTS.md` governing a touched path.
5. `docs/agents/AGENTS.md`, `REPOSITORY_MAP.md` and `CONTEXT_ROUTING.md`.
6. The active task checkpoint, live branch/PR and linked architecture/contracts.

When instructions overlap, follow the more restrictive safety, ownership and validation rule.

## Repository allowlist — highest priority

- Routine autonomous writes authorized by this repository policy are limited to `Oteryn/Oteryn-Game`.
- `blakinio/Oteryn-v2` is the preserved read-only legacy/migration source. `blakinio/Oteryn-Platform`, `blakinio/Otheryn`, `blakinio/otclient`, Canary repositories and all other repositories are read-only unless the owner explicitly authorizes a write task for that exact repository.
- Before every GitHub write, verify `repository_full_name == Oteryn/Oteryn-Game` unless the current owner instruction explicitly authorizes another repository.
- Cross-repository work requires one task, branch and PR per repository plus a shared coordination ID and explicit rollout order.
- Never use a governance edit on an unmerged branch to expand the current task's authority.

## Product boundary

Oteryn Game is the canonical native Rust gameplay stack, migrated with preserved Git history from `blakinio/Oteryn-v2`:

- an authoritative Rust game server;
- a native Rust game client;
- shared Rust domain, protocol and tooling crates;
- one project-owned gameplay protocol: `protocol-oteryn`;
- a multichannel-first world model.

The existing Oteryn Platform remains the external source of truth for web portal, Identity, OAuth/PKCE, Game Login Tickets, Game Gateway and World Registry until a separately accepted migration changes that boundary.

Otheryn C++ and the current otclient repository are reference/migration sources, not implicit runtime dependencies. `protocol-canary` and legacy Tibia packet compatibility are not part of the target Oteryn Game runtime.

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
7. perform mandatory self-review and any independent review required by the risk policy below;
8. resolve review threads and reconcile related PRs;
9. squash-merge only when every applicable gate passes;
10. archive task and release ownership.

Never push feature, fix or documentation task work directly to `main`.

Merge only when:

- base and head repositories are `Oteryn/Oteryn-Game` and base is `main`;
- the changed-file list is within declared scope;
- acceptance criteria and observable outcomes are satisfied;
- required focused/component/E2E checks pass;
- required GitHub checks pass on the exact unchanged head;
- mandatory self-review has no open material finding;
- any required independent review has no open material finding;
- no requested change, unresolved review thread, ownership conflict, migration hold or cross-repo ordering hold remains;
- task, architecture and contracts are current.

Use squash merge. Never force, bypass protection, weaken tests or mark failures successful.

## Review and audit policy

### Mandatory self-review

Every task requires a deliberate full-diff self-review by the implementing/coordinating agent before readiness. Review the exact final changed-file set for scope, architecture, security, failure paths, unsupported claims, stale evidence, temporary residue and omissions. Record material findings and repairs truthfully.

A self-review is valid review evidence but **must never be described as independent** when the reviewer implemented or materially authored the same change.

### When independent review is required

A genuinely independent second review/audit is mandatory when any of the following applies:

- an explicit owner instruction, accepted contract, trusted-base task policy or repository rule requires it;
- authentication, authorization, Game Login Ticket, admission, Game Session, CharacterLease, reconnect/recovery, session fencing or security-generation semantics change;
- gameplay protocol/wire/framing/transport compatibility, parser trust boundaries or downgrade/replay behavior change;
- authoritative persistence schemas/migrations, crash recovery, item/currency/economy conservation, market/bank/depot/loot ownership or other durable-value invariants change;
- secrets, signing keys, updater/download trust, artifact verification, privacy/security policy or protected-environment behavior changes;
- production deployment, live data/session/account behavior, protected configuration or operational authority changes;
- multichannel/world-shared ownership, failover/fencing or other change can create split-brain or stale-writer authority;
- governance is changed to reduce a safety gate, expand repository/write/merge/production/cross-repository authority or weaken required evidence;
- the implementer records material uncertainty, the change has unusual complexity/blast radius, or self-review cannot provide credible confidence against common-mode error.

An independent review is not automatically required for low-risk documentation/navigation, typo, stale-task bookkeeping or other non-semantic changes when no rule above applies, provided full-diff self-review and all applicable exact-head validation pass.

### Independent reviewer mechanisms

Independent review may be performed by a qualified human reviewer, a separate agent/session that did not implement or materially author the change, an independent review tool such as Codex, or a dedicated audit workflow that actually evaluates the exact final head. The evidence must identify the reviewer/method and exact SHA.

**The default independent-review mechanism is a fresh separate agent/session that did not implement or materially author the change**, when such a reviewer is available and qualified for the task. Give that reviewer the exact final SHA, governing contracts and audit scope, but do not treat the implementing agent's conclusions as trusted input; the reviewer must verify the change independently.

**Codex is optional, not a mandatory project dependency, and is subject to the owner-funded AI restriction above.** Do not invoke Codex without explicit owner permission for the current use. Recommend Codex when it is expected to provide a material efficiency or review-quality advantage over the available non-owner-funded reviewer; before invocation, explain that advantage to the owner and provide a bounded ready-to-run prompt for the exact task. If the owner does not authorize it, use another genuinely independent mechanism when one is suitable; if none can satisfy a mandatory gate, stop with that exact blocker.

If independent review is required but no genuinely independent mechanism is available, stop with that exact blocker. Do not relabel self-review as independent and do not weaken the gate.

Any independent-review finding that moves the head invalidates the prior exact-head review for merge readiness; re-run applicable exact-head validation and, when still required, re-review the repaired final head.

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

1. Inspect the available GitHub connector tools and determine whether the connector is registered and enabled and whether the required operations exist.
2. If an authenticated-identity operation exists and the connector is callable, call `github_get_user_login` or its equivalent; otherwise record the confirmed missing or disabled connector or missing identity operation.
3. If a repository lookup or listing operation exists and the connector is callable, call `github_get_repo` or `github_list_repositories` for the requested repository scope; otherwise record the missing capability.
4. If the required read operation exists and is callable, attempt it through the connector when it is safe and within the task's authority; otherwise record the unavailable capability.

Report a GitHub access blocker only after the applicable availability and capability checks above and, when an applicable operation exists and is safe to attempt, an actual connector call. Authentication or permission errors, a confirmed missing or disabled connector, a missing required operation, rate limiting, and transport or service failures are valid blockers when they prevent the task and no safe permitted connector, local `git`, or `gh` fallback can complete it. Include the exact availability and capability verification performed. When a call was attempted, include the failed operation and returned error; when no call was possible, identify the missing or disabled connector or unavailable operation instead.
