> Lifecycle closeout: **COMPLETED / ARCHIVED / OWNERSHIP RELEASED**. Canonical PR #687 is merged on protected main as `037bf7818122ad7d2bb10ad47f89fdd90d2a35dc`. Any nonterminal/checkpoint wording below is historical provenance only; live GitHub and current protected state supersede it.

# OTV2-20260919-content-world-cw3-project-filesystem-capture-504

```yaml
task_id: OTV2-20260919-content-world-cw3-project-filesystem-capture-504
title: CW3 Linux canonical project filesystem capture
mode: IMPLEMENT
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-cw3-project-filesystem-capture-504
issue: 162
pr: null
base_sha: cb7ade54d5622cb75a6a059e6c8517eca4a2a45b
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: content world build
created_at: 2026-09-19T23:14:55Z
updated_at: 2026-09-19T23:34:33Z
execution_policy: continuous_progress
owned_paths:
  - Cargo.toml
  - Cargo.lock
  - apps/game-server/Cargo.toml
  - apps/game-server/src/content/mod.rs
  - apps/game-server/src/content/project.rs
  - apps/game-server/src/content/project_fs.rs
  - apps/game-server/tests/content_world_project_fs.rs
  - docs/agents/tasks/active/OTV2-20260919-content-world-cw3-project-filesystem-capture-504.md
public_contracts:
  - OTERYN_WORLD_PROJECT_SOURCE_PROFILE/v1 Linux capture subset
depends_on:
  - protected PR #683 source-profile decision
  - protected PR #685 canonical project snapshot
  - protected PR #686 structural Ability/Effect/Formula reference model
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Add the smallest secure filesystem adapter that captures a canonical editable
World project on Linux and passes the resulting complete immutable byte set to
the existing strict `ProjectSnapshot` parser. The adapter opens a
caller-selected root basename without following it, resolves every canonical
locator one component at a time beneath the opened root, verifies exact entry
spelling and opened-handle identity, rejects links, special files, hardlinks and
identity aliases, and bounds directory enumeration and file allocation.

Windows and every other non-Linux target return `UnsupportedPlatform` before
metadata, open or read. This child does not claim a universal filesystem
foundation. A safe maintained same-opened-handle 128-bit Windows file identity
API, or an opened-root-anchored supported-filesystem guard, remains unavailable;
the legacy 64-bit identifier cannot safely cover ReFS.

## Architecture and source of truth

- `PROVEN` — the protected source-profile decision defines canonical locators,
  root containment, no-follow admission, ordinary files, hardlink rejection,
  unique filesystem identities and bounded allocation.
- `PROVEN` — protected PR #685 owns strict JSON, manifest/root/lock coherence,
  resource limits, the typed project graph and final linking. Filesystem capture
  does not duplicate those authorities.
- `PROVEN` — `cap-std = 4.0.3` and `cap-fs-ext = 4.0.3`, upstream commit
  `b7acf8e8807fe3fab991884d2208b7e03d35a409`, expose capability directories,
  one-component no-follow directory/file opening, full entry metadata,
  `(dev, ino, nlink)`, nonblocking opens and safe custom flags.
- `PROVEN` — `rustix = 1.1.5`, upstream commit
  `287214b889865d8e1406a0ee71cc409b6f6191c8`, supplies the maintained
  `OFlags::NOCTTY` constant without unsafe FFI or hardcoded OS values.
- `OTERYN_LAYER_FIX` — a bounded Oteryn adapter composes those supported APIs;
  no fork, vendored patch or generic filesystem framework is introduced.

## High-risk authority/recovery qualification

`NOT_APPLICABLE` — this reader captures project source bytes and creates no
production mutation, PREPARE/COMMIT authority, controller, session, fence or
persisted recovery interpretation.

## Evidence limits

The canonical focused corpus is the protected B4-containing six-document
project used by the snapshot proof. Its measured filesystem dimensions are:

| Dimension | Exact corpus maximum |
| --- | ---: |
| Root directory entries in one scan | 6 |
| Aggregate entries across all component scans | 40 |
| Largest document | 2,260 bytes |
| Aggregate document bytes | 5,314 bytes |

These are non-production evidence measurements. Exact limits pass; one-more
input against each exact limit fails. Scan arithmetic overflow fails closed.
The existing project evidence suite continues to own every JSON, locator,
semantic-count and canonical-write limit dimension.

## Acceptance criteria

- [x] Only the eight allocated paths change.
- [x] Non-Linux dispatch returns unsupported before filesystem access.
- [x] Root and locator components use one-component no-follow opens beneath one
  opened root; actual spelling and enumeration/open identity must match.
- [x] Final file opens carry no-follow, nonblocking and no-controlling-terminal
  flags before opened-handle regular-file, identity and link-count validation.
- [x] Every admitted document has link count one and a unique `(dev, ino)`;
  opened identity, type, length and link count are rechecked immediately before
  consuming bytes from that same handle.
- [x] Control files produce one strict internal capture plan; the complete
  captured set still passes through `ProjectSnapshot::new(...).parse(...)` as
  final authority.
- [x] Directory enumeration is lazy and charged against checked per-scan and
  aggregate budgets. Declared inventory count and total bytes are preflighted
  before source-document allocation.
- [x] The real protected B4 candidate batch survives capture unchanged and
  remains blocked from native promotion.
- [ ] Exact published head passes hosted repository gates and independent
  review owned by the parent control plane.

## Excluded scope

No filesystem writer, staging, journal, crash-safe save or atomic publication;
no Windows/other-platform support claim; no unsafe FFI or custom filesystem
framework; no Formula execution or native B4 promotion; no B5/B6, World Bundle,
compiler, runtime or client authority.

## Implementation / findings

`ProjectCapturePlan` is an internal view over the existing strict control
parser. It validates exact control digests, root/manifest/lock/package
coherence, inventory order/uniqueness/roles, locator grammar and declared
resource totals before the adapter opens inventoried sources. The complete
captured snapshot then repeats authoritative length/digest/document-set and
typed validation through the existing parser.

The lockfile adds the cap-std lineage and its support packages. The exact
workspace pin intentionally updates the existing related transitive
`rustix 1.1.4` entry to `1.1.5`; no unrelated locked package is upgraded.
The resolved delta is eleven new support-package entries plus that one rustix
patch entry.

## Validation

### Focused

- `cargo test -p oteryn-game-server --test content_world_project_fs`: PASS,
  9 tests.
- `timeout 60s cargo test -p oteryn-game-server --lib
  content::project_fs::linux::tests::final_open_flags_do_not_wait_for_a_fifo_writer
  -- --exact`: PASS; the unpaired FIFO opens and rejects without hanging.
- `cargo test -p oteryn-game-server --test content_world_project`: PASS,
  19 tests.
- `cargo test -p oteryn-game-server --test content_reference_playable`: PASS,
  31 tests.
- `cargo test -p oteryn-game-server --lib
  content::project_fs::linux::tests`: PASS, 2 tests.
- `cargo check -p oteryn-game-server`: PASS.
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`: PASS.

### Component/integration

- Linux canonical B4 capture equals the in-memory parser result: PASS.
- `cargo fmt --all -- --check`: PASS.
- `python -B tools/agents/validate_governance.py`: PASS.
- `python -B tools/repository/validate_repository_policy.py`: PASS.
- `cargo metadata --locked --format-version 1`: PASS.
- `cargo run --locked -p oteryn-architecture-check -- workspace .`: PASS.
- `cargo build --locked --workspace --all-targets`: PASS.
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: PASS.
- `cargo test --locked --workspace`: PASS.
- `cargo run --locked -p oteryn-synthetic-client-harness`: PASS.
- The default local `x86_64-pc-windows-msvc` check first stopped in the existing
  `ring` build because this Linux environment has no MSVC `lib.exe`. A delegated
  toolchain-only rerun set the target archiver to Rust 1.94's official
  `llvm-ar` (LLVM 21.1.8, COFF support) and
  `cargo +1.94.0 check --locked -p oteryn-game-server --all-targets --target
  x86_64-pc-windows-msvc` passed through the game-server. This is compile-only
  evidence; it is not Windows runtime evidence or a Windows capture claim. No
  source, Cargo or workflow mutation was used for the rerun.
- `cargo-deny` is not installed in this environment; dependency review and
  supply-chain policy remain hosted exact-head gates.

### E2E

`NOT_APPLICABLE` — this child ends at canonical project parsing and creates no
runtime activation or filesystem publication path.

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing agent, whole-diff adversarial review
- material findings: none
- verdict: PASS — all filesystem authority stays below the opened root; final
  handles retain no-follow/nonblocking/no-controlling-terminal flags and are
  revalidated immediately before consumption; final parsing remains the sole
  project authority.

## Independent review

- required: YES — secure filesystem boundary and shared dependency delta
- exact head: pending
- method/auditor: parent control plane allocation
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: #683, #685 and #686 are protected predecessors;
  paused #673 and open dependency PRs remain collision evidence only
- protected auto-merge: parent control plane only
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: implementation and complete available local validation passed
status: ready
branch: agent/content-world-cw3-project-filesystem-capture-504
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: publish immutable head and open the review PR
```
