# Context routing

Load the smallest context set that can safely execute the task.

## Always

Read root `AGENTS.md`, `AGENTS.override.md`, `docs/agents/AGENTS.md`, the exact active task checkpoint and live PR/CI state.

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
