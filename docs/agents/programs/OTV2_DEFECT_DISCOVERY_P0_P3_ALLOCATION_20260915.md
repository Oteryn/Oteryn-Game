# OTV2 Defect Discovery P0-P3 Allocation — 2026-09-15

```yaml
allocation_id: OTV2-ALLOC-DEFECT-DISCOVERY-P0-P3-628-20260915
repository: Oteryn/Oteryn-Game
coordinator_issue: 162
governing_issue: 628
protected_reconciliation_main: 88b04c9441b38e7353e0f66be50ba8d13fdd6cf2
prompt_package_merge: 627
worker_profile: OTV2_DEFECT_DISCOVERY_P0_P3_LEAD
worker_alias: "Oteryn: defect discovery p0-p3"
future_task_id: OTV2-20260915-defect-discovery-p0-p3-628
future_task_path: docs/agents/tasks/active/OTV2-20260915-defect-discovery-p0-p3-628.md
future_branch: agent/defect-discovery-p0-p3-628
allocation_state: NOT_ACTIVE
worker_state: NOT_STARTED
implementation_authority: P0_P3_DISCOVERY_HARNESS_ONLY
production_authority: NONE
product_source_mutation_authority: NONE
merge_authority: REPOSITORY_CONTROL_PLANE_ONLY
external_repository_write_authority: NONE
issue_write_reporter_authority: NONE
```

## Purpose

This is the single bounded #162 implementation allocation for the first useful proof of
`OTERYN_DEFECT_DISCOVERY_V1` after protected integration of prompt package #627.

It allocates **only P0-P3**:

1. **P0** — isolate heavy discovery from ordinary PR/push/Merge Queue execution and bind every run to exact tested target/dependency identity;
2. **P1** — qualify one deterministic property/state proof with a controlled TEST-ONLY defect;
3. **P2** — qualify one structure-aware fuzz target against a real production parser/decoder;
4. **P3** — qualify the finding/original-reproducer/replay contract and closed result classification.

This document does not start the worker. Protected integration/readback plus the explicit activation
rule below are required before `Oteryn: defect discovery p0-p3` receives mutation authority.

## Canonical source and live reconciliation

### Protected source of truth

The allocation was prepared against protected
`main@88b04c9441b38e7353e0f66be50ba8d13fdd6cf2`, which is the protected integration of PR #627.
The worker prompt is therefore already canonical at:

```text
docs/agents/prompts/OTV2_DEFECT_DISCOVERY_P0_P3_LEAD.md
```

Binding #162 architecture/proof comments are exactly:

```text
5684916133
5684920732
5684926851
```

Issue #626 is **not** implementation scope. Its findings are validation examples/future regression
candidates and remain with their existing owners.

### Active material lineages preserved

Fresh open-PR/path inspection at allocation time found:

- PR #356 / `agent/sqlx-driver-budget-351` owns active WP3/Durability, root `Cargo.toml`/`Cargo.lock`,
  `.github/workflows/merge-gate.yml`, `tools/qualification/sqlx-core/**`,
  `tools/repository/validate_pr_gate_pg_sim.py`, and large vendored SQLx/Tokio/rustls surfaces.
- PR #335 / `agent/durable-fresh-admission-child-b-329` owns the active fresh-admission migration,
  Durability source, PostgreSQL target/support, task and plan surfaces.
- PR #592 owns the bounded agentic/OpenSpec pilot paths including `.gitattributes`, its dedicated
  agentic workflow files, `openspec/**`, and its own task record.
- PR #609 owns `Cargo.toml` and `Cargo.lock` for its dependency update.
- PR #608 owns `.github/workflows/codeql.yml`, `.github/workflows/merge-gate.yml` and
  `.github/workflows/merge-group-gate.yml` for its dependency update.
- No live branch matching `defect-discovery` and no existing canonical
  `tools/defect-discovery/**` or `.github/workflows/defect-discovery.yml` path was found during the
  fresh reconciliation.

The allocation therefore deliberately avoids every overlapping active path above.

## Exact worker-owned paths — only after explicit activation

The future worker may write exactly:

```text
.github/workflows/defect-discovery.yml
tools/defect-discovery/**
docs/agents/evidence/OTV2-20260915-defect-discovery-p0-p3.md
docs/agents/evidence/OTV2-20260915-defect-discovery-p0-p3.json
docs/agents/tasks/active/OTV2-20260915-defect-discovery-p0-p3-628.md
```

No other path is implicitly writable.

After activation this allocation record and all other `docs/agents/programs/**` are read-only to the
worker. The canonical prompt and prompt lifecycle/README are also read-only.

## Shared paths and leases

### Writable shared paths

```text
NONE
```

No shared write lease is granted by this allocation. The dedicated workflow path and dedicated tool
subtree are exclusive to this lineage after activation.

If implementation discovers that a product source, root Cargo/workspace file, existing required
workflow, repository-governance helper, existing task record, vendor subtree, or any other non-owned
path must change, stop that change and return:

```text
ALLOCATION_AMENDMENT_REQUIRED
```

Do not borrow, inherit, widen, or infer another lane's lease.

### Read-only target seams

The worker may inspect and execute exact checked-out target bytes, but may not edit them under this
allocation. The first proof is intentionally anchored to:

```text
P1 preferred target:
  crates/simulation-determinism/**
  package: oteryn-simulation-determinism

P2 production parser target:
  apps/game-server/src/foundation/protocol.rs
  public production entry point: oteryn_game_server::foundation::decode_framed_envelope
```

`apps/game-server/src/foundation/mod.rs` may be read only to verify the production export boundary.
These are test targets, not leased implementation paths. A newer protected source may be selected at
activation/run time only if the exact target resolver proves compatibility and records the immutable
identity.

## Explicitly excluded paths and scope

The worker has no write authority for:

```text
Cargo.toml
Cargo.lock
rust-toolchain*
workspace-boundaries.toml
apps/**
crates/**
vendor/**
tools/qualification/**
tools/repository/**
openspec/**
.gitattributes
.github/workflows/merge-gate.yml
.github/workflows/merge-group-gate.yml
.github/workflows/codeql.yml
.github/workflows/agent-governance.yml
.github/workflows/architecture-semantic-audit.yml
docs/architecture/**
docs/contracts/**
docs/agents/PROMPT_LIFECYCLE.json
docs/agents/prompts/**
docs/agents/programs/**
```

except that this allocation file itself is created by the coordinator before activation and remains
read-only afterward.

Also excluded:

- P4 real PostgreSQL/TLS campaigns;
- P5 optional tool PoCs;
- Turmoil/turmoil-net, Shuttle, Loom, Toxiproxy or Kani adoption;
- mutation-testing rollout;
- Ability/Interaction/AI/Foundation/Durability/Content/native-client additional modules;
- broad module/profile rollout;
- production configuration, deployment, secrets or environments;
- product fixes or assertion weakening for #626 findings;
- autonomous Issue/PR creation from discovery results;
- external-repository writes;
- any claim that simulation or a harness proves real PostgreSQL/TLS/Tokio/socket behavior.

## Dependencies and prerequisites

Before worker activation all of the following must be true:

1. PR #627 remains present in protected `main` and the canonical P0-P3 prompt remains registered.
2. This exact allocation is protected-integrated and read back from `main`.
3. Governing Issue #628 remains open and bound to #162.
4. #162 performs a fresh activation-time read of protected `main`, open PRs/branches, active task
   records, ownership, leases, and all exact worker-owned paths.
5. No worker-owned path is claimed by another live lineage and no same-name implementation branch is
   already active.
6. The scheduler has an admissible mutating-writer slot. PR openness alone is not proof of writer
   admission; #162 must use its current scheduler/custody truth. If admitting this worker would exceed
   the current concurrency policy, leave this allocation `NOT_ACTIVE` until a slot is released.
7. P1/P2 source seams remain reachable without product-source mutation. If they require a source change
   merely to become fuzz/property accessible, fail closed and request a separate allocation decision.
8. The harness can preserve exact dependency fidelity. If a separate discovery workspace cannot prove
   the selected target's effective root patches/sources/features/toolchain/target identity, it must
   classify the run `BLOCKED_CAPABILITY`, never silently run an approximation.

No dependency on completion of #335 or #356 is created. Their paths are excluded instead.

## Authority boundaries

### Discovery execution

The heavy workflow must be read-only with respect to repository and product state. Its normal token
permissions must not include repository contents write, Issues write, pull-request write, deployment,
package publication, environment mutation or cross-repository authority.

For this first P0-P3 proof, the target repository is fixed to exactly `Oteryn/Oteryn-Game`. External
repositories and fork repositories are not accepted targets and must fail closed as
`BLOCKED_CAPABILITY` before target execution.

The workflow definition, controller and discovery harness are trusted execution code and must come
only from one immutable commit that is already present on the protected default branch. A manually
selected `target_ref` is data under test; it must never select, replace, source, or execute the
workflow/controller/harness identity. Even a same-repository non-protected target branch is treated as
untrusted target code.

Untrusted target bytes and every build script/test/fuzz process derived from them must execute in a
separate credential-free and secret-free execution compartment from the trusted controller/harness.
The target checkout must use `persist-credentials: false`; no repository write credential,
`GITHUB_TOKEN`, Actions secret, deployment credential, package credential, cloud credential, or other
privileged environment value may be made available to target-controlled processes. The untrusted
compartment must not have a writable controller/harness checkout or a path by which target bytes can
replace trusted harness executables. Any data crossing from the target compartment back to the
controller must use an explicit bounded evidence/artifact contract and be treated as untrusted data.

If the platform/harness cannot prove the immutable trusted controller identity or the credential/secret
and filesystem separation above, the attempt is `BLOCKED_CAPABILITY`; it must not fall back to
executing target-controlled code in the trusted job context.

The discovery workflow may publish GitHub Actions artifacts required by P3. Those artifacts are
evidence only; they do not mutate target source and do not grant reporter authority.

### Exact target identity

P0 must resolve the user-selected `target_ref` once to an immutable commit SHA and bind the complete
run to it. The retained run identity must distinguish at least:

- requested repository/ref and resolved tested commit SHA/tree identity;
- immutable protected-default-branch controller workflow commit/path identity;
- immutable protected-default-branch discovery harness commit/tree identity;
- trusted-controller compartment identity and untrusted-target compartment identity;
- selected module/profile/method/environment;
- Rust toolchain and compilation target;
- effective Cargo workspace root and lockfile identity where applicable;
- effective dependency package/source identities, including root patch/source replacement effects;
- effective selected feature set for the target/harness;
- exact replay selector and artifact hashes.

A ref that cannot be resolved, a moved/mismatched checkout, incomplete dependency enumeration, an
external/fork target, an incompatible harness/target pair, unprovable patch/source fidelity, or an
unprovable trusted/untrusted execution boundary must fail closed before a product finding can be
emitted.

### No second product oracle

P1/P2 may encode invariants and controlled TEST-ONLY negative behavior inside
`tools/defect-discovery/**`, but may not recreate a broad shadow registry of product truth. Product
contracts/constants must be consumed from the exact tested production seam where practical.

A controlled defect must never be committed into `apps/**`, `crates/**`, `vendor/**`, production
configuration or an ordinary required test path.

## P0 acceptance — isolation and identity

P0 is `PROVEN` only when one exact qualification packet demonstrates all of the following:

- `.github/workflows/defect-discovery.yml` has **no** `pull_request`, `push` or `merge_group` trigger;
  heavy discovery is selected only by explicit manual dispatch and a separately defined scheduled
  event on the trusted default-branch workflow;
- ordinary PR creation/update and Merge Queue do not execute the heavy workflow;
- `tools/defect-discovery/**` is not added to the root ordinary Rust workspace and an ordinary
  `cargo test --locked --workspace` / repository Merge Gate does not select or materially compile the
  discovery-only heavy dependencies;
- a normal PR/Merge Queue control provides concrete evidence that the heavy campaign did not run or
  impose its build step;
- manual dispatch accepts an explicit same-repository target ref/SHA and resolves it to one immutable
  tested SHA without mutating the target branch/ref; external/fork repositories are rejected before
  execution;
- the workflow/controller/harness used for the run are read from one immutable protected-default-branch
  commit and cannot be replaced or sourced from `target_ref`;
- target checkout/execution is in a separate credential-free, secret-free compartment with checkout
  credential persistence disabled and no writable trusted harness/controller surface;
- a controlled hostile-target negative proves that target-controlled code cannot read privileged
  credentials/secrets, replace the trusted harness/controller, or publish a product finding without
  passing the bounded evidence/replay contract;
- scheduled execution uses an explicitly selected default target policy and remains separate from PR/MQ;
- target, workflow, harness, compartment, dependency source/patch, feature, target-triple and toolchain
  identities are retained and machine-checkable;
- an unprovable trusted/untrusted isolation boundary is classified exclusively as `BLOCKED_CAPABILITY`;
  `INCOMPLETE_CAMPAIGN` is reserved for an otherwise valid campaign that cannot complete after the
  isolation boundary has been proven;
- a deliberately incompatible/unsupported target fails closed as `BLOCKED_CAPABILITY` or
  `INCOMPLETE_CAMPAIGN` according to whether capability is absent or an otherwise valid campaign is
  incomplete, never PASS;
- no hidden retry converts a failed attempt into an apparently green attempt; every attempt retains its
  own result identity.

## P1 acceptance — deterministic property/state proof

The first P1 module is intentionally bounded to the current deterministic simulation seam unless the
activation-time protected source makes that seam unavailable.

P1 is `PROVEN` only when:

- a small deterministic Oteryn-specific property/metamorphic/state campaign executes against the exact
  selected production target;
- the positive/bounded control is green and repeatable;
- at least one controlled TEST-ONLY wrong-behavior variant is detected by the same invariant/oracle;
- the wrong variant exists only under the discovery-owned tool/test surface and cannot enter product
  source or ordinary production behavior;
- the original concrete failing input/history is retained before any minimization;
- replay of that retained semantic case reproduces the same expected-vs-observed divergence;
- repeated runs with the same semantic case do not depend on an opaque seed as the sole reproducer;
- generator/oracle/harness breakage is classified `HARNESS_FAILURE`, not `PRODUCT_DIVERGENCE`.

Candidate invariant families must stay within already-existing deterministic behavior, for example
canonical-state permutation invariance, deterministic decision derivation, purpose isolation or
checked numeric relations within valid domains. The worker must select the smallest set sufficient to
prove the mechanism; this allocation does not authorize broad simulation testing.

## P2 acceptance — structure-aware fuzz proof

The first P2 target is the existing production Foundation parser entry point
`decode_framed_envelope`; it remains read-only.

P2 is `PROVEN` only when:

- the fuzz harness calls the real production parser/decoder rather than a copied parser;
- the campaign has a bounded raw/malformed input family and a structure-aware valid/near-valid input
  family capable of reaching deeper envelope semantics;
- current production frame/payload/field/semantic validation remains active; the harness does not
  bypass limits merely to increase coverage;
- cargo-fuzz/libFuzzer tool/version and the target dependency graph are pinned/retained as run identity;
- the dependency graph is verified against the selected target's effective source/patch/feature
  identity, or the run fails closed;
- at least one controlled discovery-owned TEST-ONLY seeded defect/oracle violation is found, with the
  original concrete failing bytes/structured case retained and replayable;
- malformed input rejection, panic/crash behavior and semantic divergence are distinguished;
- coverage is treated only as exploration-depth evidence; "ran for N time with no crash" is not a
  qualification result;
- no product parser source is changed by the pilot.

## P3 acceptance — finding / reproducer / replay contract

P3 must implement and validate one machine-readable envelope under the discovery-owned subtree and
emit representative artifacts under the allocated evidence paths.

Every retained attempt/finding must support this lifecycle:

```text
DISCOVER
  -> retain ORIGINAL concrete case/history/bytes
  -> classify attempt/finding
  -> optional minimize without replacing ORIGINAL
  -> validate machine-readable replay descriptor
  -> REPLAY exact retained semantic case
  -> same semantic divergence OR explicit replay-instability classification
```

Required top-level result classes are exactly:

```text
PRODUCT_DIVERGENCE
HARNESS_FAILURE
INFRASTRUCTURE_FAILURE
BLOCKED_CAPABILITY
INCOMPLETE_CAMPAIGN
```

The schema/validator must retain or bind at least:

- unique attempt identity and case identity;
- suspected-family fingerprint separate from concrete-case identity;
- P0 tested target/workflow/harness/dependency identity;
- module/profile/method/environment;
- expected invariant and observed result/divergence;
- ORIGINAL reproducer reference/hash;
- optional minimized reproducer reference/hash without deleting/replacing ORIGINAL;
- replay selector/arguments and replay result;
- artifact hashes and bounded retention metadata;
- explicit result classification and failure class;
- seed as supplementary metadata only;
- retry/attempt lineage so failed attempts cannot disappear behind a later retry.

A reporter that creates/updates Issues is out of scope.

## Required validation, review and CI

### Allocation PR

This allocation itself is documentation/governance only and must have:

- exact base-to-head changed-file review showing exactly this one allocation file;
- `git diff --check` equivalent / repository governance validation as provided by hosted checks;
- exact-head Agent Governance;
- exact-head Architecture Semantic Audit;
- exact-head Merge Gate / aggregate `game-gate` required by repository policy;
- zero unresolved requested-change/review threads;
- genuinely independent review before integration because this record grants future workflow/tool
  mutation authority and arbitrary-ref test execution;
- native protected Merge Queue integration, real `merge_group` aggregate `game-gate`, then protected
  `main` readback.

Direct merge, generic auto-merge, force/rebase, bypass, no-op CI-nudge commits and protection weakening
are forbidden substitutes.

### Future P0-P3 implementation candidate

On one unchanged exact final head the worker must provide:

- focused unit/schema/CLI tests for exact-ref resolution, identity validation, classification and replay;
- P0 hostile-target isolation regression proving the trusted controller/harness cannot be replaced and
  no credential/secret is exposed to target-controlled code;
- P1 bounded positive + controlled-negative proof and deterministic replay;
- P2 bounded raw + structured fuzz proof, controlled-negative detection and replay;
- static/semantic trigger test proving the discovery workflow excludes PR/push/Merge Queue events;
- ordinary workspace/PR/MQ negative-control evidence proving discovery heavy dependencies are not
  selected by the ordinary lane;
- exact dependency-fidelity evidence for the P1/P2 target(s);
- `cargo fmt`/strict Clippy/tests applicable to the discovery-owned Rust code without adding it to the
  ordinary root workspace;
- repository exact-head governance/architecture/Merge Gate checks selected by the canonical
  classifier, with no existing check weakened or removed;
- mandatory adversarial whole-diff self-review;
- genuinely independent exact-head semantic/security/supply-chain review because the change adds a
  workflow capable of resolving/checking out arbitrary selected refs and executing fuzz/property code;
- zero unresolved material findings/requested changes before integration;
- native Merge Queue `merge_group` aggregate `game-gate` and protected-main readback.

After protected implementation readback, one default-branch manual canary must exercise exact target
selection/replay from the trusted workflow before #162 marks the P0-P3 proof package complete. A cron
need not be artificially awaited, but the scheduled trigger/configuration must be validated and must
remain isolated from PR/MQ.

## Exact activation / launch condition

Protected integration of this document is necessary but **not sufficient** to start implementation.

After protected-main readback, #162 must perform a fresh reconcile and then explicitly bind all of the
following in Issue #628:

```text
protected_main: <current exact protected main SHA containing this allocation>
allocation_path: docs/agents/programs/OTV2_DEFECT_DISCOVERY_P0_P3_ALLOCATION_20260915.md
allocation_state: ACTIVE
worker_state: ADMITTED
worker_profile: OTV2_DEFECT_DISCOVERY_P0_P3_LEAD
worker_alias: Oteryn: defect discovery p0-p3
branch: agent/defect-discovery-p0-p3-628
task: docs/agents/tasks/active/OTV2-20260915-defect-discovery-p0-p3-628.md
owned_paths: exactly the five entries/subtrees in this allocation
shared_write_leases: NONE
```

Activation requires, at that moment:

1. the allocation bytes are read back from protected `main`;
2. the five exact worker-owned paths/subtrees are free of competing live ownership;
3. no conflicting `agent/defect-discovery-p0-p3-628` implementation lineage exists;
4. the current scheduler admits the worker without violating its mutating-writer concurrency policy;
5. P1/P2 target seams still exist and can remain read-only;
6. no newer protected architecture/governance record supersedes P0-P3 requirements;
7. all prerequisites above remain `PROVEN`, with no material `UNKNOWN`/`CONFLICT`.

Only after that explicit Issue #628 activation record may the implementation branch/task be created or
resumed and the operator invoke exactly:

```text
Oteryn: defect discovery p0-p3
```

If any condition fails, leave `ALLOCATION_STATE: NOT_ACTIVE` and return the exact blocker. Do not create
a replacement implementation branch, seize another lane's path, or widen this allocation implicitly.

## Completion boundary

This allocation is complete only when it is protected-integrated and read back from `main`. That
completion authorizes only the later activation preflight above; it does not claim P0-P3 are
implemented.

P4/P5 and any subsequent module/tool rollout require a new, separately reconciled #162 decision after
the protected P0-P3 implementation proof has been qualified and read back.