# Oteryn Game agent instructions

These instructions govern `Oteryn/Oteryn-Game`, the canonical repository for the native game server, native client, protocol/domain code, world/content tooling and Game-owned export contracts.

## Authority and boundaries

- `Oteryn/Oteryn-Game` is the only current Game product write authority.
- Durable Game invariants: `protocol-oteryn` is the target runtime protocol; the world model is `multichannel`; `WorldId` and `ChannelId` remain distinct; character writes remain `session-generation` fenced.
- `blakinio/Oteryn-v2` is historical migration provenance/reference only after source retirement; do not create ordinary work there.
- Platform owns web identity/commercial/control-plane responsibilities defined by accepted cross-repository contracts. Atlas consumes normalized Game-owned exports and may not become Game truth authority.
- Read a nearer `AGENTS.md` for any touched path. A same-directory `AGENTS.override.md` replaces, rather than extends, the base instruction file and therefore must only exist when true replacement semantics are intended.

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