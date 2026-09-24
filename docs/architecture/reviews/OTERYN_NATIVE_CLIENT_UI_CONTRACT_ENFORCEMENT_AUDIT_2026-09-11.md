# Native UI parent-contract, enforcement and pinned-API audit

- Task: `OTV2-NATIVE-UI-CONTRACT-ENFORCEMENT-AUDIT-20260911`
- Repository: `Oteryn/Oteryn-Game`
- Initial protected source: `6db1e95dcd0377d3258c045ea0b95f5d620c53f7`
- Reconciled publication base: `663bd35a5196a925fc6eb0318381ad0b97f4cc2c`
- Original UI architecture: #549, historical exact head `83314bcbf2978b20f07ca539dd697900dcdb410f`
- Integrated predecessors: #551, #557 and #560
- Source disposition: **FIX**
- Amendment disposition: **AUTHOR-CHECKED CANDIDATE / INDEPENDENT EXACT-HEAD REVIEW REQUIRED**
- Runtime implementation/activation authority: **NONE**
- Final FOV: **UNDECIDED / EVIDENCE-GATED**
- Addons: **DEFERRED / FUTURE CONCEPT**

## 1. Scope and source freshness

The owner requested the previously missing parent-contract, enforcement/CI and pinned-library boundary audit. This pass supplements the prior 31-file native-client source inventory; it does not replace that historical inventory with a new repository-wide claim.

The four directly named parent architecture documents of #549, its three original UI documents and ADR-0007 were read completely. The executable workspace checker, semantic audit selector/checker, PR and queue workflows, post-merge routing and named repository-policy modules below were also read completely, rather than inferred from their green statuses. The library review is of the selected versioned public API contracts and lockfile entries, not every internal line of every transitive crate or graphics driver. References from a parent to other domain programmes are not an assertion that those entire programmes were re-audited.

While this pass was reading protected `6db1e95...`, #560 was integrated as `663bd35...`. The complete comparison is ahead by two commits and adds only the #560 correction/report plus `docs/agents/programs/OTV2_WP3_RUSTLS_TLS13_POST_CERTIFICATE_TRANSCRIPT_CALLERS_AMENDMENT_20260911.md`. None of the inspected runtime, parent-contract, Cargo or enforcement inputs changed. The new amendment therefore starts after #560 instead of writing to its closed branch.

Authority was resolved from root `AGENTS.md`, the pinned META 3.1.0 binding to `Oteryn/Oteryn@3b39e0be05aef008f1bd442821daefa898a201dd`, applicable `docs/agents/AGENTS.md`, `ARCHITECTURE_DECISION_DISCIPLINE.md`, contribution rules and the current build matrix. No root Cargo, workflow, checker, shared runtime or protection lease is inferred. The write scope is exactly:

1. `docs/architecture/OTERYN_NATIVE_CLIENT_UI_ARCHITECTURE_CORRECTION_CONTINUATION_2026-09-11.md`;
2. `docs/agents/BUILD_TEST_MATRIX.md`;
3. this report.

The live named-work search found Draft PR #558 for runtime prerequisites and #552 for historical independent review, not another owner of this amendment. A separate open-PR search for `BUILD_TEST_MATRIX.md` returned no match. Searches are discovery evidence, not a lease grant. #558 remains untouched and is not reclassified as integrated or qualified here.

## 2. Exact complete-read inventory

These blobs are identified at the initial protected source and unchanged in the reconciled comparison, except the #560 continuation first consumed after its integration. A path here means the complete file was read; directories and transitive dependencies are not implied.

| Exact repository path | Git blob |
| --- | --- |
| `docs/architecture/OTERYN_NATIVE_CLIENT_UI_ARCHITECTURE_BASELINE_2026-09-10.md` | `7ad2dfc233980c1a424e6e129816596b99bd758b` |
| `docs/architecture/OTERYN_NATIVE_CLIENT_UI_ARCHITECTURE_IMPLEMENTATION_PLAN_2026-09-10.md` | `36aaaa191b9bdd34d6a8bc9c15dc2a2418a839c3` |
| `docs/architecture/OTERYN_NATIVE_CLIENT_VIEWPORT_AB_EXPERIMENT_PLAN_2026-09-10.md` | `ea35bf121ecab4fa0f580313cb99b8b09e0cd935` |
| `docs/architecture/ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_ANALYSIS.md` | `05e43dcca1da9c5b36b50727d64160f4c14dfc55` |
| `docs/architecture/ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_CONTRACT_CANDIDATE.md` | `a846cb093d84033e0cd9f37e88c6d00538529a43` |
| `docs/architecture/OTERYN_GRAPHICS_PRESENTATION_VFX_ARCHITECTURE_BASELINE_2026-09-09.md` | `f9a6086f51d5ae6f6eb689bea4178b8c3f2361e5` |
| `docs/architecture/OTERYN_VISUAL_WORLD_SLICE_ARCHITECTURE_AND_EVIDENCE_GATE_2026-09-09.md` | `8dc9963bf70c785cf2df45ff6eec139d749498df` |
| `docs/architecture/ADR-0007-native-end-to-end-test-platform.md` | `764448e57feed94f59fe18e659412c6f483b08bd` |
| `docs/architecture/OTERYN_NATIVE_CLIENT_UI_ARCHITECTURE_CORRECTION_CONTINUATION_2026-09-11.md` | `eeed9bb86c45024007f78ca08a4bf91a3c7bf7ee` |
| `docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md` | `570ac67c6269041589938eabe01cddc1866f4c06` |
| `docs/agents/BUILD_TEST_MATRIX.md` | `b9b4f073537bb00f6dfd7645797d2333ee2d8c8e` |
| `docs/agents/AGENTS.md` | `5de964ee04806bb81db63d532bf3b7461cce18b6` |
| `Cargo.toml` | `fb4134748560219c5a4cb80f58ad98ac7d1c22a2` |
| `tools/architecture-check/Cargo.toml` | `e381aeb95cba8279300dad9bc5cd633948b8d00e` |
| `tools/architecture-check/src/lib.rs` | `3e4284235bee11588b6f00afb582789f13bd6373` |
| `tools/architecture-check/src/main.rs` | `7138c4d4af708fc64228d626f3987d818c05d9f1` |
| `tools/architecture/semantic_contract_audit.py` | `09654066975466368f8f3c596c59eaac3457acbe` |
| `.github/workflows/architecture-semantic-audit.yml` | `44f79456e85e682a4b710b51394330302a97526f` |
| `.github/workflows/merge-gate.yml` | `98c56c64c9dac75cd821b478fdc6baf585e70861` |
| `.github/workflows/merge-group-gate.yml` | `c59b30fde7538e738346eec03a602081dc4ac2d6` |
| `.github/workflows/rust.yml` | `dc2ea01a08d971735b382082caeae330349183ca` |
| `tools/repository/classify_pr_test_lanes.py` | `805496393d949fb1206e30e92c8c01167786b8f1` |
| `tools/repository/test_classify_pr_test_lanes.py` | `3a09387e7e33e5ea7cc244f6891c6de22007bd45` |
| `tools/repository/validate_repository_policy.py` | `ee166d0e45bbc1fd68a40d2e392547701691d7af` |
| `tools/repository/validate_repository_policy_core.py` | `86a55a1b7542305c19d8a35e9f7cc9731f5ef525` |
| `tools/repository/validate_pr_gate_pg_sim.py` | `cc3461a26a9d3d2bf45ae4ec24fad38c70223753` |

The complete three-file `tools/architecture-check` subtree is `659d9ce75878c20fe5766312b74a625c3e0d29ae`. Other checker test directories are not claimed fully reviewed merely because the corresponding CI regression command ran.

Targeted additional evidence: `Cargo.lock` blob `452d2fea5e2df8a706f17e3cb9a168c9bb0bf0d9`, selected renderer/window dependency entries; the source ordering already recorded by #557; live #558 metadata at head `b2b6e5be610ae03642eeeec7cdd0ccd777d0d20d`; both repositories' Issue #263 identities; and the exact #560 semantic job log below. Full lockfile or full #558 implementation requalification is not claimed.

## 3. Findings and bounded dispositions

### CE-1 — workspace closure and framework neutrality are different proofs

**FACT:** `tools/architecture-check/src/lib.rs::internal_edges` projects path dependencies belonging to workspace members. The role/edge/cycle/production checks validate that internal graph. Registry/git and their transitive closures are not established by that projection. The separate workflow tree checks use selected roots, the host/default profile and a named forbidden list.

**INFERENCE — high confidence:** a green internal checker alone cannot certify a new `ui-core` free of external GPU/platform coupling across relevant targets/features. This is an evidence gap, not a claim that the not-yet-allocated UI crate currently contains such a dependency.

**CORRECTION:** continuation Section 10.2 and the matrix distinguish internal policy validation from declared/resolved external closure evidence. P1 must supply direct/transitive/target-feature negative cases through its allocated implementation. No checker or required status is weakened or invented by this patch.

### CE-2 — actual semantic UI verdict was NOT_APPLICABLE

**FACT:** run `34562444300`, job `103147774477`, on #560 head `db502e473bda60f7cdea2498f5138705c2cf0bea` ran 37 dispatcher regression tests successfully, then emitted:

```text
profile = NOT_APPLICABLE
profiles = []
checks = []
verdict = NOT_APPLICABLE
SEMANTIC_AUDIT_NOT_APPLICABLE
```

`semantic_contract_audit.py::select_profiles/main` selects ALPHA_CLIENT_01, ANL_02_ANL_03 or FOUNDATION_RECONNECT_DURABILITY_V1 by explicit paths; neither #560 file selects a UI profile. The workflow's successful exit is therefore consistent with the emitted non-applicability result.

**CORRECTION:** earlier receipts describing the workflow as successful must not be read as UI semantic PASS. This report, continuation Section 11.1 and the matrix explicitly correct the evidence interpretation. The historical run remains intact. Automated text/profile checks, including the 37 tool tests, are not independent architecture review of these new clauses.

### CE-3 — Windows builds and debug shell smoke do not execute every native suite

**FACT:** all three inspected Rust workflow paths build the release client but launch shell smoke using `cargo run` without `--release`. Windows `cargo test` is named for simulation-determinism, not the client/input/renderer suites. Linux workspace tests and Windows `--all-targets` Clippy cannot be described as executing every Windows-only dependency test. The previously audited shell exits smoke before renderer construction.

**CORRECTION:** P2/P3 must bind their actual package test commands/targets and native fixtures; Tier 3 must identify and launch the release artifact, not infer identity from an earlier build step. The matrix documents current behavior without changing workflows or pretending that future UI tests already exist.

### CE-4 — current queue and PostgreSQL routing differ from the matrix prose

**FACT:** pinned queue workflow `c59b30...` has an inline `architecture-docs` path-only exception that may omit Rust Linux, PostgreSQL, Windows and supply chain. The aggregate still requires candidate/governance, dependency review and CodeQL, and permits skipped results only for coherent unselected lanes. This is not the PR/post-merge consumer-snapshot classifier. The PR PostgreSQL target classifier now uses exact commit-tree/checkout-blob observations, not the capped comparison as target-presence authority.

**CORRECTION:** the matrix now describes these actual algorithms, labels earlier unconditional/full rollout descriptions as historical and retains required exact-head/queue controls. No CI behavior changes.

**REMAINING CONTROL-PLANE ASSUMPTION:** the queue exception does not verify the document-consumer snapshots used in the PR path. A path label alone is not proof that a later runtime consumer cannot depend on an architecture document. This pass does not certify that assumption for every future merge composition or silently fix the protected workflow. The existing control-plane owner must review it before relying on the exception for a relevant document-input dependency. This documentation correction is not a new approval of the exception.

### CE-5 — P5 persistence must inherit scope and privacy semantics

**FACT:** ALPHA contract Section 15 specifies ACCOUNT, OS_USER, INSTALLATION and DEVICE semantics, field-specific override precedence, absent account-layer behavior and restrictive privacy precedence. UI failure-atomic layout persistence alone does not establish these cross-scope guarantees.

**CORRECTION:** continuation Section 10.1 requires field scope, migration direction/conflicts/rollback, unrelated-field preservation and opt-out survival through layout reset, downgrade or account/device change. It introduces no account service, storage framework or cloud synchronization authority.

### CE-6 — a native synthetic fixture is not Tier 2 by resemblance

**FACT:** ADR-0007 defines Tier 2 as the declared native-client E2E journey using its real production contracts and a bounded observation adapter. Tier 3 binds exact production artifacts. The original correction's "Tier-2-equivalent" phrase could conflate that definition with a physical synthetic HUD fixture.

**CORRECTION:** continuation Section 11.1 supersedes that phrase and preserves a separately labelled native UI qualification category. P1-P6 retain useful independent progress; product E2E acceptance retains the actual ADR requirements. A clear pass, console harness, prototype frame or synthetic viewport benchmark cannot establish production gameplay or final FOV fairness.

### CE-7 — pinned frame, callback and dependency identities need an explicit contract

**FACT:** the inspected `wgpu 30.0.0` API distinguishes surface acquisition, submission and presentation scheduling. Its reconfiguration preconditions constrain outstanding surface-texture lifetime. The selected callback/timing APIs require progress and explicit device capabilities. The lockfile fixes `wgpu 30.0.0` together with core/hal/types `30.0.1`, while `winit` is `0.30.13`; the top-level version alone is incomplete evidence identity.

**CORRECTION:** continuation Section 6.1 adds coordinated renderer frame/configuration lifetime, bounded callback delivery and precise observation boundaries. Section 3 makes the pinned UTF-8 preedit range interpretation explicit. The amendment does not select shaders, a scheduler, numeric cache ceilings or a different library version. Existing #502 resource authority is retained.

**LIMIT:** these are source/API preconditions, not a hardware reproduction or proof that the current vendor internals and drivers are defect-free. Lockfile package presence alone is not a target/feature-resolved runtime graph.

### CE-8 — historical numeric references need repository identity

**FACT:** `blakinio/Oteryn-v2#263` is the historical ALPHA-CLIENT architecture task; `Oteryn/Oteryn-Game#263` is an unrelated governance task. The short numeric reference in migrated ALPHA documents is ambiguous across repositories.

**CORRECTION:** the continuation qualifies the historical reference and denies any inference of current execution/acceptance authority from an old closed issue. Historical parent documents and issue states are not rewritten.

## 4. Versioned public API evidence

The following primary references were inspected at the named versions. Scope is the listed API boundary, not an audit of the complete dependency implementation.

| Primary reference | Inspected boundary |
| --- | --- |
| [winit 0.30.13 event source](https://docs.rs/winit/0.30.13/src/winit/event.rs.html) | IME preedit/commit/reset payloads and byte ranges; line/pixel wheel representation |
| [winit 0.30.13 Window](https://docs.rs/winit/0.30.13/winit/window/struct.Window.html) | Native IME enablement and candidate-area host calls; platform-specific applicability |
| [wgpu 30.0.0 Surface](https://docs.rs/wgpu/30.0.0/wgpu/struct.Surface.html) | Configuration preconditions and acquired surface-frame lifetime |
| [wgpu 30.0.0 Queue](https://docs.rs/wgpu/30.0.0/wgpu/struct.Queue.html) | Submission, presentation scheduling, completion callbacks and timestamp units/support |
| [wgpu 30.0.0 Device](https://docs.rs/wgpu/30.0.0/wgpu/struct.Device.html) | Requested features/limits, polling/progress and device/error callback surfaces |

The rendering/error APIs are not treated as absent merely because an indexed search misses a method. No latest-version example is substituted for the pinned repository contract. Each future upgrade must requalify the affected boundary.

## 5. Validation, limits and follow-through

The two unmodified local input documents were reconstructed in an isolated workspace and verified against blobs `eeed9bb86c45024007f78ca08a4bf91a3c7bf7ee` and `b9b4f073537bb00f6dfd7645797d2333ee2d8c8e` before editing. The validation receipt records the resulting blobs and the exact three-path diff. Local document checks cover source identity, scope, formatting, fenced blocks, section continuity, source inventory and the required correction clauses; patch application and reverse application must reproduce the candidate and original bytes. These checks are not Rust tests, automated semantic certification or independent KEEP.

Repository-native checks must qualify the published exact PR head. Predecessor #560 CI is only evidence of that predecessor. The actual #560 semantic log is used narrowly for CE-2; neither its workflow success nor a future zero-profile run is presented as semantic approval of this report.

Local Rust tests were not run: this execution environment has no Cargo/rustc and its direct GitHub DNS route failed. The GitHub connector remains available for source and publication, and repository CI remains the execution route. No Remote Desktop, external paid AI reviewer, production service, private pixel asset or credential mutation is used.

Physical native UI qualification remains **NOT_RUN**. The missing dependencies are the allocated UI implementation and representative native HUD/world host, executable interaction/pixel-observation scenarios, and a named approved Windows/GPU/driver qualification cell. Their absence prevents a physical IME/DPI/capture/render/device-recovery or A/B PASS, not the source/contract corrections in this task. The existing #558 draft handles prerequisite code repairs; this pass neither replaces it nor declares its new regressions passing. Its live lifecycle must be read again before relying on it.

No new FOV decision, synthetic-to-production dependency, runtime activation, external write, queue bypass or protection weakening is introduced. The existing correction review/integration gate still applies to the new exact head. Documentation rollback must retain the valid #560 clauses and historical evidence. The control-plane consumer-assumption review in CE-4 and allocated P1/P2/P3/P5 implementation evidence remain explicitly unclosed; they are not hidden behind a numeric audit score.

```text
SOURCE_AUDIT = FIX
DOCUMENT_CORRECTIONS = AUTHOR_CANDIDATE
SEMANTIC_WORKFLOW_SUCCESS != UI_SEMANTIC_PASS
NATIVE_SYNTHETIC_FIXTURE != ADR0007_TIER2
RUNTIME_REPAIRS = EXISTING_PR558_NOT_REQUALIFIED_HERE
PHYSICAL_UI_QUALIFICATION = NOT_RUN
INDEPENDENT_REVIEW = REQUIRED_ON_NEW_EXACT_HEAD
FOV = UNDECIDED / EVIDENCE-GATED
ADDONS = DEFERRED / FUTURE CONCEPT
```
