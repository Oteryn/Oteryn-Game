# Context routing

Load the smallest context set that can safely execute the task.

## Baseline

Read root `AGENTS.md` and the nearest `AGENTS.md` applicable to the working path. Resolve the bound META policy required by the operation as directed by the root bootstrap.

Read an active task checkpoint only for a substantial task that has one. Resolve live Issue, PR, branch, head or CI state only when the current mutation, lifecycle decision, review or integration depends on it. Bounded read-only analysis and trivial work do not require a fabricated task, PR or CI lookup.

### Live-state read budget

Use targeted reads by default:

- do not bulk-read complete Issue or PR comment timelines; start from metadata/current state and fetch only specifically referenced or latest material comments needed by the decision;
- do not enumerate every open PR/task when the affected lane, dependency or ownership set is already bounded;
- do not read the complete `PROMPT_LIFECYCLE.json` to invoke one known alias; resolve the matching entry only;
- do not read historical sections of long-lived allocation/task documents when a current checkpoint already supersedes them, unless history itself is material evidence;
- do not load `OTERYN_GAME_AGENT_OPERATOR_RUNBOOK.md` for a technical worker unless owner-facing launch/status placement is the task;
- prompt evaluation is for prompt authoring/material changes/lifecycle evaluation or an explicit evaluation task, not ordinary alias reuse.

### Large documents and command output

Load these only when their condition holds:

- `CLOSURE_CONVERGENCE_PROTOCOL.md`: only when a PR is in a repeated review/repair loop;
- `prompts/OTV2_REFERENCE_INVESTIGATOR.md`: the shared sections plus the section for your own alias, not the other aliases' sections;
- `prompts/OTV2_WORK_DELIVERY_COORDINATOR.md`: only the active control plane;
- audit and architecture-continuation prompts: only when running that alias.

Run builds and tests quietly (`cargo ... --quiet`, keep the last ~20 lines of output) and report pass/fail counts instead of pasting logs or diffs.

Many data files are multi-megabyte JSON on a single line (`content/**/definitions/*.json`, `content/loot/`, `content/world/definitions/`, `imports/**/bindings/*.json`, `docs/agents/evidence/*.json`). A plain `grep`/`rg` match there prints the whole line. Search them with `rg -l` or `--count` first, then extract the record with `jq` or Python; never print matching lines from them. `docs/agents/evidence/.rgignore` excludes that directory's JSON from ripgrep searches entirely.

### Subagent routing

Leads that run workers as subagents use the definitions in `.claude/agents/`:

- `oteryn-impl-worker` (Sonnet, effort medium): ordinary allocated implementation, data and docs slices;
- `oteryn-hard-worker` (Opus, effort high): persistence, session-generation fencing, `protocol-oteryn` wire format, authority or durable value;
- `oteryn-ref-reader` (Haiku, read-only): code search, Reference evidence and live-state lookups.

A subagent's final report is at most 15 lines. Subagents never trigger paid review, allocate or merge.

A current-state read may expand only when the smaller slice leaves a material authority, ownership, dependency, safety or acceptance fact unresolved. Reuse authenticated immutable exact-revision material within the coherent task instead of re-reading it merely because another step begins.

For an ungoverned, low-risk and reversible implementation detail, state a bounded assumption and continue. Do not infer permission, ownership, production access, destructive intent or a durable product decision from missing context.

For Oteryn-v2 foundation or architecture continuation, resolve current progress, blockers and next action from live Issue/PR/check state and the active task checkpoint. Treat dated programme-status and coordination-register documents as historical snapshots unless a protected change explicitly refreshes and re-establishes them as current.

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
