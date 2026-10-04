# Context routing

Load the smallest context set that can safely execute the task.

## Baseline

Read root `AGENTS.md` and the nearest `AGENTS.md` applicable to the working path. Resolve the bound META policy required by the operation as directed by the root bootstrap.

Read an active task checkpoint only for a substantial task that has one. Resolve live Issue, PR, branch, head or CI state only when the current mutation, lifecycle decision, review or integration depends on it. Bounded read-only analysis and trivial work do not require a fabricated task, PR or CI lookup.

### Live-state read budget

Use targeted reads by default:

- do not bulk-read complete Issue or PR comment timelines; start from metadata/current state and fetch only specifically referenced or latest material comments needed by the decision;
- do not enumerate every open PR/task when the affected lane, dependency or ownership set is already bounded;
- long threads such as #162: read the Issue body and the `STATE` comment it links, then only the last page of comments (last page = ceil(comment count / page size), from the Issue metadata); never start from page 1, which holds the oldest comments;
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

Many data files are multi-megabyte JSON on a single line (`content/**/definitions/*.json`, `content/loot/`, `content/world/definitions/`, `imports/**/bindings/*.json`, `docs/agents/evidence/*.json`). A plain `grep`/`rg` match there prints the whole line. The Grep tool omits long lines, but Bash `grep`, `rg` and `cat` print them in full. Search them with the Grep tool, `rg -l` or `--count` first, then extract the record with `jq` or Python; never print matching lines from them. `docs/agents/evidence/.rgignore` excludes that directory's JSON from ripgrep searches entirely.

### Finding a decision

To find what a numbered decision (`D84`, `SPELL-D7`) says, read its row in `docs/agents/DECISION_INDEX.md` and open the linked document, instead of searching the repository. The numbers are not unique across lanes, so check the subject.

### Subagent routing

Leads that run workers as subagents use the definitions in `.claude/agents/`:

- `oteryn-impl-worker` (Sonnet, effort medium): ordinary allocated implementation, data and docs slices;
- `oteryn-hard-worker` (Opus, effort high): persistence, session-generation fencing, `protocol-oteryn` wire format, authority or durable value;
- `oteryn-ref-reader` (Haiku, read-only): code search, Reference evidence and live-state lookups.

Subagent workers get the META policy and routing from their lead and read only the sections their definition names. A subagent's final report is at most 15 lines. Subagents never trigger paid review, allocate or merge.

### Local checks by changed path

Run the narrow local check for what you changed and leave the full workspace build, Windows client and E2E lanes to CI. Do not read `.github/workflows/merge-gate.yml` to discover checks.

| Changed path | Local check |
|---|---|
| `crates/<name>/`, `apps/<name>/`, a Rust crate under `tools/` | `cargo fmt --all --check`; `cargo clippy --locked -p <package> --all-targets --quiet -- -D warnings`; `cargo test --locked -p <package> --quiet` |
| `Cargo.toml`, `Cargo.lock`, `workspace-boundaries.toml` | the above for each affected package, plus `cargo run --locked -p oteryn-architecture-check -- workspace .` |
| `content/`, `imports/`, `rulesets/` | the tests of the package that loads the file (`rg -l '<file name>' crates apps --glob '*.rs'`) and the path's own validator named in its `AGENTS.md` or README |
| `docs/agents/`, `tools/agents/`, `.claude/` | `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests` |
| `tools/repository/`, `.github/` | `python tools/repository/validate_repository_policy.py` and the matching `tools/repository/test_*.py` |
| other docs | `git diff --check` |

The `apps/game-server/tests/*_postgres.rs` targets need PostgreSQL 17.6, the version CI pins. Start it once per session: run `(dockerd >/dev/null 2>&1 &)`, wait until `docker info` succeeds, then `docker run -d --name oteryn-pg -p 5432:5432 -e POSTGRES_USER=oteryn_test_admin -e POSTGRES_PASSWORD=localpw postgres:17.6-bookworm`. Run one target with `OTERYN_TEST_POSTGRES_ADMIN_URL=postgresql://oteryn_test_admin:localpw@127.0.0.1:5432/postgres cargo test --locked -p oteryn-game-server --test <target> --quiet`. Never relax the 17.6 version check, even temporarily. If the 17.6 image is unavailable (for example, Docker Hub returns 429), leave those targets to CI and say so in the report.

The package name is the `name` in that directory's `Cargo.toml`. A PR that touches `tools/agents/`, `tools/repository/`, `.github/workflows/` or the Cargo manifests runs the full Rust Linux and Windows lanes in CI (about 15 minutes), even when it changes no Rust. Keep ordinary docs, task-record and content PRs away from those paths, and put prompt-version pin updates into the coordinator's daily batch.

### Waiting for CI

After a push, do not poll check status in a loop: each poll re-reads the whole session. Report `waiting for CI` with the head SHA and end the turn; PR activity events or the lead resume you. Re-check once when resumed.

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
