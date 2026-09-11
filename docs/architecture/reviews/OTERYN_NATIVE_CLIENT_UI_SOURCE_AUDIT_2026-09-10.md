# Oteryn native UI source audit and bounded architecture corrections

- Date: 2026-09-10
- Task: `OTV2-NATIVE-UI-SOURCE-AUDIT-20260910`
- Repository: `Oteryn/Oteryn-Game`
- Audited protected source: `main@5025be6cf3f5140cf94708f8e6ddc9ab3f40d99f`
- Original PR: `#549`, `base=main`, exact head `83314bcbf2978b20f07ca539dd697900dcdb410f`
- Original merge: `7144c0b9ec8691e481df058c85d890ac88d32461`
- Integrated first correction: `#551`, exact reviewed head `0f26f4bfe03618f7faa8409f3f678d34a6acb886`
- Follow-up branch: `agent/native-ui-source-audit-20260910`
- Source-audit classification: **FIX**
- Correction status: **AUTHOR-CHECKED CANDIDATE / FRESH INDEPENDENT EXACT-HEAD REVIEW REQUIRED**
- Runtime implementation and activation authority: **NONE**
- Final FOV: **UNDECIDED / EVIDENCE-GATED**
- Addons/mods/plugins: **DEFERRED / FUTURE CONCEPT**

## 1. Scope, authority and coverage

The owner requested a deeper source audit and architecture corrections. PR #549 and its first correction #551 were already merged. This pass therefore reviews the integrated source and changes the existing correction addendum on a separate branch. It does not rewrite the closed PRs or implement UI-P1 and later runtime slices.

The bounded write allocation contains exactly two documents:

- `docs/architecture/OTERYN_NATIVE_CLIENT_UI_ARCHITECTURE_CORRECTION_ADDENDUM_2026-09-10.md`;
- this audit/task record.

Read authority: root `AGENTS.md`; pinned META policy binding and `OTERYN_ORGANIZATION_AGENT_POLICY 3.1.0` at `Oteryn/Oteryn@3b39e0be05aef008f1bd442821daefa898a201dd`; `docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md`; `CONTRIBUTING.md`; applicable build/test-matrix sections and client contract sections. No nearer `AGENTS.md` exists at `docs/`, `docs/architecture/` or `docs/architecture/reviews/` in the audited source.

All three documents introduced by #549 were read from beginning to end, including their diagrams, non-goals, testing and exit criteria. The PR reports 2,395 added lines across those three files. The integrated 814-line correction addendum was also read completely. The code review covers the exact boundary files below, not every source line in the repository or every transitive dependency. No claim of a repository-wide exhaustive audit or mathematically guaranteed defect absence is made.

### Architecture source identities

Paths below are under `docs/architecture/`.

| File | Audited Git blob |
| --- | --- |
| `OTERYN_NATIVE_CLIENT_UI_ARCHITECTURE_BASELINE_2026-09-10.md` | `7ad2dfc233980c1a424e6e129816596b99bd758b` |
| `OTERYN_NATIVE_CLIENT_UI_ARCHITECTURE_IMPLEMENTATION_PLAN_2026-09-10.md` | `36aaaa191b9bdd34d6a8bc9c15dc2a2418a839c3` |
| `OTERYN_NATIVE_CLIENT_VIEWPORT_AB_EXPERIMENT_PLAN_2026-09-10.md` | `ea35bf121ecab4fa0f580313cb99b8b09e0cd935` |
| `OTERYN_NATIVE_CLIENT_UI_ARCHITECTURE_CORRECTION_ADDENDUM_2026-09-10.md` | `29006bffa2af1e5581b2eba4b331d102fe480e85` |

### Source-boundary inventory

These files were read completely at the audited protected SHA. Directory names do not imply that all other files in those directories were reviewed.

| Exact file | Audited Git blob |
| --- | --- |
| `apps/client/src/lib.rs` | `8e4b3d0bbadfc1c05cccdaf667d63056fe73d709` |
| `apps/client/src/windows_shell.rs` | `0dc912a313fb035967b1578886986c163223c9f4` |
| `crates/client-runtime/src/lib.rs` | `bd908a2c6e2d418dc09e39b7f42d9ac58cfd78a0` |
| `crates/client-domain/src/lib.rs` | `9cb4fb57493a0ff01d7514e47bb730ea7ca4b7f9` |
| `crates/client-simulation/src/lib.rs` | `53218533d094ea74f0adf5ec153ca0f57f574af8` |
| `crates/input-actions/src/lib.rs` | `ae5a3fec1db556f664284bddac9e4a4337e1286a` |
| `crates/input-actions/src/router.rs` | `8655c5bfb6afd36d4ab5a131fd0871961f3e5ee1` |
| `crates/input-actions/src/semantic.rs` | `489c926c48f795f56b2f08c7defb94fdf9b93f0b` |
| `crates/input-actions/Cargo.toml` | `9d7138e63d686f5b0bfedac0d0d7a8608a5950c8` |
| `crates/input-platform/src/lib.rs` | `3663ff6a8800a0b2c098d0b6de21125278744312` |
| `crates/input-platform/src/adapter.rs` | `5b715a926b7792eb9ff494bf22aca2ba6a31e012` |
| `crates/input-platform/src/winit_adapter.rs` | `03e6453be0d5d9c21816803dc9abe254c659f1e3` |
| `crates/renderer/src/lib.rs` | `42cd8425eef9e6bbe5011f9287461c3c762602c2` |
| `crates/renderer/src/windows.rs` | `f7e8a5be69b696f98343b3d9cd4ebf834ef83221` |
| `crates/renderer/Cargo.toml` | `d76249a3d0a602d259189658aa9421ded543f575` |
| `tools/synthetic-client-harness/src/main.rs` | `1a8592fffb681e34960da545f965baecb00752d2` |
| `Cargo.toml` | `fb4134748560219c5a4cb80f58ad98ac7d1c22a2` |
| `workspace-boundaries.toml` | `6af6c9f9e05151afa603cd3d86f4fb3261abaa3c` |

Additional contextual evidence includes `crates/input-actions/src/physical.rs` lines 1-150 (blob `ffb0d4570f56c6df506d3d7ec34142931fbd6df8`), the earlier #549 audit and inspected ALPHA-CLIENT contract sections. Those contextual reads are not represented as whole-file re-audits. Legacy OTClient behavior remains earlier reference evidence; this pass did not independently requalify that external repository.

## 2. Existing corrections retained, not rediscovered as new blockers

#551 already corrects workspace membership/roles/edges, the root Cargo lease gate, UI/application input arbitration, missing IME preedit semantics, active-action cancellation, bootstrap/shell ownership, shared-path serialization, physical world/HUD host admission, staged physical evidence, fail-closed interactions, deterministic primary B, A/B identity/fairness surfaces and feature-driven P8 dependencies.

The fresh source inspection corroborates important limitations: the shell's `--smoke` path exits before constructing `WindowsRenderer`; the synthetic harness only exercises `SurfaceState`, not a physical frame; the production renderer presents a clear pass; `client-domain` and `client-simulation` remain synthetic-only in workspace policy. These are existing qualification gates, not newly implemented product capabilities.

PR #552 was open at the live census and records an independent KEEP for #551's older exact head. Its separate report is not modified. That review cannot qualify the materially changed addendum produced by this task.

## 3. Findings and corrections

### F1 — semantic cancellation is not physical cleanup

**FACT:** `InputRouter::set_context_active()` calls `cancel_ineligible()` without clearing `held`. `process_button(Released)` removes the atom; focus/capture/device lifecycle handlers have their own cleanup. `BindingMap::resolve()` and `is_context_eligible()` keep Global contexts eligible during text/modal input.

**INFERENCE, high confidence:** implementing the existing conditional input arrow as an early return for consumed UI events can swallow releases, retain stale physical state, or let a high-priority Global gameplay mapping bypass intended modal ownership. This is a risk in the planned composition, not a claim that a production gameplay UI currently exhibits it.

**ADDITIONAL FACT:** `InputPlatformAdapter::process_key()` emits modifier key events, `KeyCode::new(224)` is valid, and `InputRouter::process_button()` includes all held atoms when creating a chord. The source trace ControlLeft-down -> A-down yields Control plus `[A, ControlLeft]`, not the conventional Control plus `[A]`. The inspected modified-key router test injects the letter with an already-set modifier bit instead of that complete adapter sequence.

**INFERENCE, high confidence:** directly composing these existing surfaces does not establish correct modifier-chord behavior. P2 must qualify the complete adapter/arbitration/router stream, including left/right modifier releases and repeat cleanup; shortened normalized-event tests are insufficient. This is a source-derived mismatch, not a claimed executed Rust regression.

**CORRECTION:** Sections 5 and 7 distinguish semantic admission from always-reconciled cleanup, require explicit re-arming, and prevent Global from becoming a gameplay bypass. P2 owns the minimal router/composition change and the negative tests; no duplicate physical-key registry or fabricated OS lifecycle is authorized.

### F2 — generation rejection occurs after physical effects

**FACT:** in `crates/renderer/src/windows.rs`, `render()` checks the phase but acquires the surface texture before checking the supplied generation. On successful acquisition, `present()` submits and presents before `SurfaceState::apply(Presented)` performs generation validation. The pure state test `stale_generation_is_transactional` does not exercise those physical operations.

**INFERENCE, high confidence:** a stale-generation call on that success path can produce physical effects before returning a generation error. This is established by source ordering; no physical reproduction was performed.

**CORRECTION:** Section 10.1 requires generation/lifecycle validation before physical effects and a no-side-effect stale-generation oracle before P3 exit. **The Rust defect remains unmodified by this P0 documentation patch.**

### F3 — success, presentation and measurement were insufficiently distinguished

**FACT:** `WindowsRenderer::execute()` returns `Ok(())` for timeout/occlusion skips and some recovery operations without presentation. A suboptimal frame can be presented before a later reconfiguration error. Existing A/B documents request timing/percentile evidence but do not specify this per-attempt accounting.

**INFERENCE, high confidence:** counting successful calls as presented frames can misstate both physical proof and the A/B population.

**CORRECTION:** Sections 10.1, 14.6 and 17 separate attempt/submission/presentation/recovery outcomes, preserve skipped/failed populations, label unavailable GPU timing, and compute the UI p95 target from same-frame sums rather than adding independent component percentiles. No measured performance result is claimed.

### F4 — geometry, retained identity and resource bounds need concrete cross-owner invariants

**FACT:** the baseline requires logical scaling, stable node identity, clipping, virtualization and bounded work. Those requirements do not define stale-node rejection, scale application count, shared hit-test/draw geometry, or admission-time overflow/depth handling. `input-platform` supplies bounded normalized coordinates, not the complete app/UI transform.

**RECOMMENDATION:** Sections 4.1/4.2 and 10.1 add minimum lifecycle/geometry oracles: no retargeting after node reuse, item identity separate from recycled row slots, checked finite limits, coherent forward/inverse transforms, nested clipping, and no targeting against mismatched displayed geometry. Numeric ceilings and storage algorithms remain implementation decisions. P1, P2 and P3 own only their implemented surfaces.

### F5 — the physical UI draw contract needs shader/order/cache qualification

**FACT:** root Cargo enables `wgpu` with only `std` and `dx12`, with default features disabled. The current physical path clears a target; it does not demonstrate UI shaders, blending, text or clipping. The baseline permits batching by compatible resources without a detailed ordering oracle.

**RECOMMENDATION:** Section 10.2 requires explicit qualification of the chosen shader representation/features, alpha painter order, clip boundaries and cache lifetime. It does not select a shader language, text library or new crate. Any future dependency change retains the serialized Cargo lease.

### F6 — pending interaction and persistence lifetime must survive replacement safely

**FACT:** the plan requires stale/disconnect behavior and safe layout defaults but does not specify delayed drag/results after semantic target replacement or interrupted/future-schema layout writes. Client-runtime exposes app-owned spawning; it does not itself establish UI-result freshness.

**RECOMMENDATION:** Sections 12.1 and 23 require app-owned observation fencing/cancellation, bounded asynchronous work, stale-result rejection and failure-atomic/non-destructive layout recovery. Session/protocol types remain outside `ui-core`; command retry/idempotency authority is not redefined. P5/P8 own these integrations; this is not permission to implement production projection adapters in P0.

### F7 — source precedence and the P4 prerequisite needed precision

**FACT:** baseline Section 26 can be read as an outward Cargo dependency, although the existing addendum correctly gives inward edges. The previous corrected diagram joins P2 and P3 before P4, while implementation-plan Section 19 names P3 and P1 text interfaces as P4 prerequisites.

**CORRECTION:** Section 1 explicitly maps superseded clauses; Section 21 permits P4 text/resource work after P1/P3 without awaiting unrelated P2 work. Integrated P5 proof still joins P2 and P4. Shared paths remain serialized. P8 is still feature/dependency-driven; no FOV-independent feature is artificially blocked by P7.

## 4. Validation and evidence limits

The changed document was prepared in an isolated local source copy. Its unmodified input was verified against Git blob `29006bffa2af1e5581b2eba4b331d102fe480e85`, not inferred from a filename or moving branch.

Local document checks cover UTF-8, final newline/trailing whitespace, fenced blocks, table structure, top-level section continuity, preserved FOV/addon/dependency/review constraints, the 15-row future boundary matrix, corrected DAG text/acyclicity, full modifier-stream qualification and the exact two-file write scope. Whole-diff author review is not independent architecture review. These are document checks, not execution of the required future Rust/UI tests.

The local environment has no `cargo`/`rustc` or authorized Windows GPU execution surface, and its GitHub network route is unavailable. Repository reads/publication use the GitHub connector. No local Rust build/test, physical rendering, Tier-2 journey, A/B benchmark or production-closure execution is claimed. Runtime E2E for this two-document patch is `NOT_APPLICABLE` because no runtime behavior changes; future UI qualification remains unperformed, not PASS.

Repository-native governance and selected checks must qualify the actual correction PR head. Their run IDs and live status belong to the PR rather than a pre-publication self-referential commit claim. Unselected runtime jobs must not be relabelled physical evidence. No workflow, protection, Cargo file or required check is changed.

## 5. Acceptance, remaining dependencies and disposition

This task delivers source findings and corrected architecture clauses, not a self-issued KEEP. The correction candidate requires a fresh independent review bound to its live repository, PR number, `base=main` and exact head; prior #549/#551 review targets do not transfer. Required exact-head checks, normal Merge Queue, successful `merge_group` gate and protected-main readback remain mandatory before integration claims.

UI-P1 still needs fresh root Cargo/lockfile/workspace-boundary ownership and explicit allocation. P2 must implement the cleanup/re-arming contract; P3 must repair and qualify pre-side-effect generation validation and actual rendering; P5/P6 still need a named physical world/HUD host and evidence. None of those dependencies is reported complete here.

Rollback of this candidate is documentation-only and does not alter server/protocol/gameplay behavior. Do not erase historical evidence or revert the valid #551 corrections while revising the new clauses.

```text
SOURCE_AUDIT = FIX
ARCHITECTURE_CORRECTIONS = CANDIDATE_AUTHORED
INDEPENDENT_REVIEW = REQUIRED_ON_NEW_EXACT_HEAD
RUNTIME_FIXES = NOT_IMPLEMENTED_BY_THIS_P0_TASK
FOV = UNDECIDED / EVIDENCE-GATED
ADDONS = DEFERRED / FUTURE CONCEPT
MERGE_AUTHORITY = REPOSITORY_CONTROL_PLANE_ONLY
```
