# Context routing

Load the smallest context set that can safely execute the task.

## Baseline

Read root `AGENTS.md` and the nearest `AGENTS.md` applicable to the working path. Resolve the bound META policy required by the operation as directed by the root bootstrap.

Read an active task checkpoint only for a substantial task that has one. Resolve live Issue, PR, branch, head or CI state only when the current mutation, lifecycle decision, review or integration depends on it. Bounded read-only analysis and trivial work do not require a fabricated task, PR or CI lookup.

For an ungoverned, low-risk and reversible implementation detail, state a bounded assumption and continue. Do not infer permission, ownership, production access, destructive intent or a durable product decision from missing context.

For Oteryn-v2 foundation or architecture continuation, also read `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md` before interpreting progress, blockers or next-action text in long-lived backlog/register/baseline documents.

## Architecture or domain ownership

Also read:

- relevant files under `docs/architecture/`;
- `MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md`;
- relevant contracts under `docs/contracts/` when present;
- overlapping active tasks/PRs.

## Protocol, login or session

Also read:

- ADR-0001;
- `CROSS_REPO_CONTRACTS.md`;
- protocol/session contracts;
- producer revisions in Oteryn Platform and consumer revisions in client/server repositories;
- security and downgrade/replay acceptance.

## Server/world/channel/persistence

Also read:

- multichannel scope matrix;
- character lease, persistence and item-transaction contracts when present;
- failure/recovery policies;
- deterministic E2E/soak requirements.

## Client/rendering/UI/assets

Also read:

- client architecture/contracts and module map when present;
- asset provenance/security policy;
- platform-specific build/test matrix;
- exact server/protocol producer revision.

## Content or Otheryn migration

Also read:

- `OTHERYN_REFERENCE_MIGRATION_PLAN.md`;
- exact source paths/revision in Otheryn;
- provenance and licensing evidence;
- target ruleset/scope and deterministic behavior fixtures.

## Governance or prompts

Also read:

- all modified policy files;
- `GOVERNANCE_CONTRACT.json` and `PROJECT_LANES.json`;
- `PROMPTING_STANDARD.md`, `PROMPTING_HANDOVER.md`, `PROMPT_EVAL_STANDARD.md` as relevant;
- governance validation workflow/script.

## GitHub-only, continuation or recovery

Load the corresponding dedicated policy only when the execution mode requires it. Do not read every policy recursively for a small bounded edit.
