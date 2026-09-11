# Oteryn native client UI source-audit continuation

- Date: 2026-09-11
- Repository: `Oteryn/Oteryn-Game`
- Task: `OTV2-NATIVE-UI-SOURCE-AUDIT-CONTINUATION-20260911`
- Protected base at branch creation: `main@6db1e95dcd0377d3258c045ea0b95f5d620c53f7`
- Integrated predecessor: PR `#557`, merge commit `6db1e95dcd0377d3258c045ea0b95f5d620c53f7`
- Original architecture PR: `#549`, exact historical head `83314bcbf2978b20f07ca539dd697900dcdb410f`
- Integrated first correction: PR `#551`
- Disposition: **FIX / FOLLOW-UP CORRECTION REQUIRED**
- Runtime implementation authority: **NONE**
- Final FOV: **UNDECIDED / EVIDENCE-GATED**
- Addons/mods/plugins: **DEFERRED / FUTURE CONCEPT**

- Expanded audit initial target: PR `#560`, `base=main`, pre-amendment head `db502e473bda60f7cdea2498f5138705c2cf0bea`
- Expanded evidence: Sections 9-12 below; correction bytes require new exact-head qualification

## 1. Purpose and source boundary

PR #557 is now integrated. This continuation records the remainder of the owner-requested line-by-line review of the bounded native-client UI source surface and turns the additional findings into explicit architecture requirements. It does not rewrite historical PRs and does not claim that every source file in the repository or every transitive dependency was audited.

The complete scoped source boundary is the eight requested package subtrees plus root workspace metadata:

- `apps/client/**`
- `crates/client-runtime/**`
- `crates/client-domain/**`
- `crates/client-simulation/**`
- `crates/input-actions/**`
- `crates/input-platform/**`
- `crates/renderer/**`
- `tools/synthetic-client-harness/**`
- root `Cargo.toml`
- root `workspace-boundaries.toml`

All 29 files in those eight package subtrees, plus the two root files, were read completely: **31/31 scoped files**. The three architecture/plan documents introduced by #549 and the integrated correction addendum were also read completely. Contextual contract reads outside that set remain contextual evidence rather than a repository-wide exhaustive audit claim.

## 2. Exact source inventory

| Exact file | Audited Git blob |
| --- | --- |
| `Cargo.toml` | `fb4134748560219c5a4cb80f58ad98ac7d1c22a2` |
| `apps/client/Cargo.toml` | `305272fa694d23f9aa5db196c38f41b3cf249f58` |
| `apps/client/src/lib.rs` | `8e4b3d0bbadfc1c05cccdaf667d63056fe73d709` |
| `apps/client/src/main.rs` | `e026dd98b8a1f9875d3812a6192fdbc9eca7bb27` |
| `apps/client/src/windows_shell.rs` | `0dc912a313fb035967b1578886986c163223c9f4` |
| `crates/client-domain/Cargo.toml` | `8a3d284b75ffe30ccb51c3320b85a9883d4be7e1` |
| `crates/client-domain/src/lib.rs` | `9cb4fb57493a0ff01d7514e47bb730ea7ca4b7f9` |
| `crates/client-runtime/Cargo.toml` | `28350a2f665fb39dc5d0bba4a91f5fb620ca659e` |
| `crates/client-runtime/src/lib.rs` | `bd908a2c6e2d418dc09e39b7f42d9ac58cfd78a0` |
| `crates/client-simulation/Cargo.toml` | `6d9819b5fe3374f05bb653351d318c9a11747b83` |
| `crates/client-simulation/src/lib.rs` | `53218533d094ea74f0adf5ec153ca0f57f574af8` |
| `crates/input-actions/Cargo.toml` | `9d7138e63d686f5b0bfedac0d0d7a8608a5950c8` |
| `crates/input-actions/src/error.rs` | `eeaeb18e57868218d1bd8a25f17422db87fb1ba7` |
| `crates/input-actions/src/lib.rs` | `ae5a3fec1db556f664284bddac9e4a4337e1286a` |
| `crates/input-actions/src/physical.rs` | `ffb0d4570f56c6df506d3d7ec34142931fbd6df8` |
| `crates/input-actions/src/router.rs` | `8655c5bfb6afd36d4ab5a131fd0871961f3e5ee1` |
| `crates/input-actions/src/semantic.rs` | `489c926c48f795f56b2f08c7defb94fdf9b93f0b` |
| `crates/input-actions/src/text.rs` | `36e1cd57ff1f478696cec9e22f98446afa0fe46a` |
| `crates/input-platform/Cargo.toml` | `23a15999fa845fa9913f6d5af3aaf52992ca311d` |
| `crates/input-platform/src/adapter.rs` | `5b715a926b7792eb9ff494bf22aca2ba6a31e012` |
| `crates/input-platform/src/error.rs` | `b1f2f5b2bce28117dd89f3495d6de400fbe79228` |
| `crates/input-platform/src/lib.rs` | `3663ff6a8800a0b2c098d0b6de21125278744312` |
| `crates/input-platform/src/tests.rs` | `0c193b94c88c95557d9959ba5436e311b59846cb` |
| `crates/input-platform/src/winit_adapter.rs` | `03e6453be0d5d9c21816803dc9abe254c659f1e3` |
| `crates/renderer/Cargo.toml` | `d76249a3d0a602d259189658aa9421ded543f575` |
| `crates/renderer/src/lib.rs` | `42cd8425eef9e6bbe5011f9287461c3c762602c2` |
| `crates/renderer/src/resources.rs` | `50523a17d1dcd72bd004faa688c05d598574ef2f` |
| `crates/renderer/src/windows.rs` | `f7e8a5be69b696f98343b3d9cd4ebf834ef83221` |
| `tools/synthetic-client-harness/Cargo.toml` | `ca0af51692916102ca886e99747e1ef18dec4f15` |
| `tools/synthetic-client-harness/src/main.rs` | `1a8592fffb681e34960da545f965baecb00752d2` |
| `workspace-boundaries.toml` | `6af6c9f9e05151afa603cd3d86f4fb3261abaa3c` |

Complete subtree identities at the scoped readback:

| Subtree | Git tree |
| --- | --- |
| `apps/client` | `8fb4f420971a3bb6c3e215b4c65dde861f4bb80a` |
| `crates/client-runtime` | `b173994007c0ab6cb10cc98c0828131e4fce0ae7` |
| `crates/client-domain` | `67b010d180fd0e6b155d3780ee740cb53d2a0fce` |
| `crates/client-simulation` | `4524b6310c4b5021bd30be8aafb81b5b2823a2b7` |
| `crates/input-actions` | `9f76b8efd2ce3f74b907a3d9932bfdb0dbc9d15e` |
| `crates/input-platform` | `ca6c66b27a1bbaf929ceae3bc0ed2da598e48126` |
| `crates/renderer` | `481b58c4ddca94e82992021590d6c71d1e718daa` |
| `tools/synthetic-client-harness` | `6c3d8144a4362ead052ffca288d0593a1b0156f0` |

## 3. Findings retained from #557

The integrated correction remains valid for: semantic cancellation versus physical cleanup; blocked-press re-arming; `Global` binding non-bypass; full modifier-stream qualification; renderer generation validation before physical effects; truthful attempted/submitted/presented/skipped/recovery accounting; coherent geometry/DPI/hit-test/draw transforms; stale node/row/resource identity; bounded work admission; painter-order/cache/shader qualification; stale observation/result rejection; non-destructive layout persistence; source precedence; corrected P4/P8 dependency shape; FOV evidence gates; production/synthetic separation.

This continuation does not reopen those accepted directions. It adds the following gaps discovered by completing the remaining scoped files.

## 4. Additional findings

### F8 — normalized-event ownership and native IME host were underspecified

**FACT:** `NormalizedInputEvent` is defined in `crates/input-actions/src/physical.rs`, while the existing correction wording permitted `input-actions/**` changes only for missing semantic-action primitives. A real preedit payload therefore cannot be added solely in `input-platform` without either losing it or duplicating the shared normalized-event contract. The Windows shell currently does not provide the future UI composition with explicit IME enable/disable and candidate-area placement.

**CORRECTION:** P2 may make the minimum framework-neutral normalized-event/value extension at the existing owning contract when required for IME or unit-aware UI input. Widget focus/composition state remains in `ui-core`/`apps/client`, not in the input crates. `apps/client` owns wiring native IME enable/disable and candidate-area placement to the focused editor. Preedit ranges must be validated before indexing; commit remains exactly-once and diagnostic output must not reveal committed or preedit text.

### F9 — wheel normalization destroys information required by UI scrolling

**FACT:** `process_wheel()` reduces native line and pixel deltas to the same integer `WheelDelta`; sufficiently small values can round to zero. `WheelDelta::directions()` is intentionally a discrete gameplay-direction abstraction, not a smooth UI-distance contract.

**CORRECTION:** P2 must preserve a bounded unit-aware and fractional UI scroll representation before discrete gameplay impulse conversion. Panel scrolling and world/gameplay wheel actions are separate semantic consumers. Tests must cover line units, pixel units, sub-unit accumulation/bounds and arbitration without claiming smooth UI distance correctness from the discrete impulse oracle.

### F10 — input loss, surface loss and GPU-device loss are distinct domains

**FACT:** input `DeviceLost` resets input adapter state; `WindowsRenderer` surface recreation reuses the current device/queue; `ResourceCache` fences insertion by process generation. Surface recreation is therefore not proof of GPU-device recovery.

**CORRECTION:** P3 qualification must distinguish input-device reset, presentation-surface loss/recreation and actual GPU-device loss/rebuild. If the selected `wgpu`/backend path has no supported in-process device-loss recovery contract, the behavior must be explicit terminal/fail-closed rather than reported as recovered. Resource reconstruction, cache identity and bounded admission consume the existing renderer resource authority (#502/#162); this audit does not mint a competing cache budget.

## 5. Existing-code repair prerequisites

These are repair requests for the coordinator/control plane, not leases granted by this documentation task.

| Repair | Requested ownership | Acceptance oracle |
| --- | --- | --- |
| Modifier/lifecycle prerequisite | `crates/input-actions/src/physical.rs`, `router.rs`, `lib.rs`; regression seam in `crates/input-platform/src/tests.rs` | Ctrl/Shift/Alt/Super match without physical modifier atoms contaminating non-modifier chords; left/right release and repeat semantics explicit; consumed release/context changes leave no stale action. |
| Renderer generation prerequisite | `crates/renderer/src/windows.rs :: WindowsRenderer::render/present` plus an allocated renderer regression seam | Reject stale generation before surface acquisition, queue submission or presentation; preserve same-generation lifecycle and bounded recovery. Pure `SurfaceState` tests alone are insufficient evidence for physical ordering. |

No product keymap, world renderer, cache budget, Cargo dependency, server/protocol behavior or runtime activation is authorized here.

## 6. Qualification delta

The 15 grouped boundary cases already recorded by #557 remain. The completed source audit adds four mandatory groups before the affected slices can claim exit:

1. native IME preedit sequence with bounded/invalid byte ranges, exactly-once commit and redacted diagnostics;
2. native IME enable/disable plus candidate-area movement across focus, scale and window lifecycle changes;
3. line versus pixel wheel input, fractional/sub-unit values, bounded accumulation and UI-versus-gameplay arbitration;
4. separate input-device reset, surface-loss recreation and GPU-device-loss/resource-rebuild or explicit terminal behavior.

These are future implementation tests. They are not represented as executed runtime evidence by this documentation PR.

## 7. Decision discipline and invariants

The additional decisions are required now because P2/P3 implementation would otherwise encode incompatible contracts at shared boundaries. Numeric resource ceilings, exact retained-tree storage, shader language/text library and final product FOV remain implementation/evidence decisions and are not selected here.

Preserved invariants:

- `apps/client` is the sole production native-client composition root;
- `client-runtime` owns async runtime/cancellation lifecycle only;
- `input-platform` owns platform normalization only;
- `input-actions` owns framework-neutral normalized physical/action contracts and semantic routing, not widget state;
- `ui-core` remains framework-neutral and must not depend on `wgpu`, `winit`, server/protocol/gameplay-authority or synthetic-only packages;
- `renderer` owns physical GPU/surface/resource behavior;
- `client-domain`, `client-simulation` and the synthetic harness remain outside production closure unless separately accepted;
- `RESPONSIVE_FOV_POLICY = UNDECIDED` and `FIXED_FOV_POLICY = UNDECIDED`;
- addons/mods/plugins remain deferred/out of current implementation scope.

## 8. Validation, review and integration boundary

This continuation is documentation-only. It records source-derived defects and architecture requirements; it does **not** repair the Rust defects, execute physical Windows/GPU qualification, run the future P2/P3 tests, authorize a worker lease, or activate runtime behavior.

A fresh independent review must bind the follow-up PR, `base=main` and its exact head. Required repository-native exact-head checks, normal Merge Queue, successful `merge_group` aggregate gate and protected-main readback remain mandatory before integration. Author review or automated semantic checks cannot be relabelled independent acceptance.

Rollback is documentation-only and must preserve already integrated #551/#557 evidence. Do not weaken protection, tests, workspace policy or Merge Queue to land this continuation.

```text
SOURCE_AUDIT_SCOPE = 31_OF_31_REQUESTED_SOURCE_FILES_READ
SOURCE_AUDIT = FIX
FOLLOW_UP_ARCHITECTURE_CORRECTION = REQUIRED
RUNTIME_FIXES = NOT_IMPLEMENTED_BY_THIS_DOCUMENTATION_TASK
INDEPENDENT_REVIEW = REQUIRED_ON_EXACT_HEAD
FOV = UNDECIDED / EVIDENCE-GATED
ADDONS = DEFERRED / FUTURE CONCEPT
MERGE_AUTHORITY = REPOSITORY_CONTROL_PLANE_ONLY
```

## 9. Expanded parent-contract and enforcement audit

The next pass reads the following **20 additional complete files** at protected `6db1e95dcd0377d3258c045ea0b95f5d620c53f7`. Together with the previous 31-file source boundary, this is an explicit finite read set, not every recursively referenced server contract or every third-party implementation. Previously read original UI documents and governing AGENTS/META/decision-discipline remain part of the authority basis, not newly counted files.

| Exact additional file | Audited Git blob |
| --- | --- |
| `docs/architecture/ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_ANALYSIS.md` | `05e43dcca1da9c5b36b50727d64160f4c14dfc55` |
| `docs/architecture/ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_CONTRACT_CANDIDATE.md` | `a846cb093d84033e0cd9f37e88c6d00538529a43` |
| `docs/architecture/OTERYN_GRAPHICS_PRESENTATION_VFX_ARCHITECTURE_BASELINE_2026-09-09.md` | `f9a6086f51d5ae6f6eb689bea4178b8c3f2361e5` |
| `docs/architecture/OTERYN_VISUAL_WORLD_SLICE_ARCHITECTURE_AND_EVIDENCE_GATE_2026-09-09.md` | `8dc9963bf70c785cf2df45ff6eec139d749498df` |
| `docs/architecture/ADR-0007-native-end-to-end-test-platform.md` | `764448e57feed94f59fe18e659412c6f483b08bd` |
| `docs/architecture/CLIENT_CRASH_DIAGNOSTICS_PRIVACY_OWNER_BASELINE.md` | `b1fd280fe5a23066b350b7cc83f51591e259048c` |
| `docs/architecture/GRAPHICS_APPEARANCE_ANIMATION_AND_SEASONALITY_HORIZON_NOTE.md` | `e68aa8a5e0dcc3c89d8bfdaa7f034896af24d396` |
| `docs/architecture/ADR-0016-gameplay-transport-client-mode-runtime-readiness.md` | `efe8b0f78689cfce59562ecedf31b870f80513d5` |
| `tools/architecture-check/Cargo.toml` | `e381aeb95cba8279300dad9bc5cd633948b8d00e` |
| `tools/architecture-check/src/main.rs` | `7138c4d4af708fc64228d626f3987d818c05d9f1` |
| `tools/architecture-check/src/lib.rs` | `3e4284235bee11588b6f00afb582789f13bd6373` |
| `.github/workflows/architecture-semantic-audit.yml` | `44f79456e85e682a4b710b51394330302a97526f` |
| `.github/workflows/merge-gate.yml` | `98c56c64c9dac75cd821b478fdc6baf585e70861` |
| `.github/workflows/merge-group-gate.yml` | `c59b30fde7538e738346eec03a602081dc4ac2d6` |
| `.github/workflows/rust.yml` | `dc2ea01a08d971735b382082caeae330349183ca` |
| `tools/architecture/semantic_contract_audit.py` | `09654066975466368f8f3c596c59eaac3457acbe` |
| `tools/repository/classify_pr_test_lanes.py` | `805496393d949fb1206e30e92c8c01167786b8f1` |
| `tools/repository/validate_repository_policy.py` | `ee166d0e45bbc1fd68a40d2e392547701691d7af` |
| `tools/repository/validate_repository_policy_core.py` | `86a55a1b7542305c19d8a35e9f7cc9731f5ef525` |
| `docs/agents/BUILD_TEST_MATRIX.md` | `b9b4f073537bb00f6dfd7645797d2333ee2d8c8e` |

The full-file set covers direct UI parent analysis/contract, graphics/world-slice boundaries, essential E2E/privacy/readiness parents, workspace enforcement, PR/queue/post-merge workflow wiring, semantic profile selection and repository-policy pins. Other referenced FND-02/FND-04/DUR-04/Platform contracts are inherited through these parents; this pass does not claim a fresh whole-file audit of their complete downstream implementation or recursively of every ADR. The graphics horizon remains a deferred note, not authority to freeze formats, animation engines or quality tiers.

Selected `Cargo.lock` graphics/input entries and pinned public API/source documentation were also inspected. This is dependency/API-boundary verification, not a full source audit of all transitive crates, OS libraries, drivers or supply-chain advisories. No dependency version, feature, budget, policy pin or workflow was changed.

## 10. Expanded findings and disposition

### F11 — successful semantic workflow did not evaluate UI

**FACT:** semantic workflow run `34562444300`, job `103147774477`, at `db502e473bda60f7cdea2498f5138705c2cf0bea` completed successfully. Its domain result was `NOT_APPLICABLE`, `profiles: []`, `checks: []`. Dispatch regressions passed, but `select_profiles()` / `main()` expose only the named ALPHA-client, analytics and Foundation-reconnect profiles; the two #560 UI documents selected none.

**CORRECTION:** continuation Section 11 and the build matrix require conclusion plus selected-profile/verdict/check evidence. Earlier wording that this workflow “passed” is true only about job completion, not semantic UI acceptance. The historical run is preserved, not rewritten. A generic green workflow, author review or an empty domain profile cannot replace required independent review. No cosmetic UI keyword checker is added merely to manufacture a PASS.

### F12 — workspace role checks are not the whole framework-neutrality proof

**FACT:** `tools/architecture-check` validates workspace membership/path mapping, release roles, local dependency edges, cycles and declared production closure. Its local-edge graph is not a resolved transitive registry-dependency proof. The client/server closure snippets in the three Rust workflows use `cargo tree --edges normal,build` on their execution target/default configuration; `cargo-deny` serves advisory/license/ban/source policy, not UI neutrality or runtime correctness.

**INFERENCE, high confidence:** a green current generic boundary check alone does not establish the planned `ui-core` framework/GPU/platform prohibition for every relevant feature/target. **CORRECTION:** continuation Section 8.2 assigns the minimum package-specific declaration/resolution and negative evidence to P1 and consuming slices, through the existing enforcement owner. No speculative package or new registry is introduced now.

### F13 — Windows-only adapter tests are not executed by canonical lanes

**FACT:** `crates/input-platform/src/lib.rs` gates `winit_adapter` with `#[cfg(windows)]`. Its `stable_keyboard_subset_uses_usb_hid_usage_codes` and `named_mouse_buttons_are_stable_and_other_is_unsupported` tests therefore do not run in the Linux workspace population. The inspected Windows lanes build/lint the client and run shell/harness/simulation commands, but do not run the input-platform package's unit tests on MSVC. Compiling a dependency is not executing its tests.

**CORRECTION:** continuation Section 8.3 states the focused Windows command and requires named/countable target execution plus appropriate native interaction proof. This audit did not run that missing population, and the documentation patch does not alter pinned CI jobs.

### F14 — release compilation is followed by development-profile smoke

**FACT:** PR, Merge Queue and post-merge Windows jobs build `oteryn-client --release`, then call `cargo run ... -- --smoke` without `--release`. The already-audited shell smoke returns before constructing `WindowsRenderer`.

**CORRECTION:** continuation Section 8.3 and the build matrix label the evidence accurately. A later release claim must execute the exact artifact/hash and the required journey. Neither the existing smoke nor a future release flag alone proves physical UI or ADR-0007 Tier 3.

### F15 — local synthetic physical proof cannot substitute for Tier 2

**FACT:** ADR-0007 and the ALPHA-client contract distinguish synthetic component fixtures from the real headless/native/production-binary service journeys. The UI addendum's `Tier-2-equivalent` phrase can be read more broadly than that parent contract permits.

**CORRECTION:** continuation Section 8.1 explicitly supersedes only that ambiguity. Local physical HUD evidence remains useful for P3-P6 without blocking pure foundations on the entire server stack, but is not Tier 2, native gameplay readiness or a new E2E tier. Parent service, cleanup and exact-revision obligations remain intact. Section 8.4 also binds layout/settings recovery to the inherited scope/privacy rules rather than inventing local account authority.

### F16 — active build matrix lagged executable protected CI

**FACT:** protected #556 (`1f2781c9c52e4231c9638553bcdd6014291e109a`) activated the reviewed architecture-document-only Merge Queue path classifier. Current queue code may skip heavy lanes for that proven path family; the matrix still said queue qualification was always FULL/unconditional. Separately, the current PR PostgreSQL target oracle reads immutable exact-path base/head trees and compares the checkout blob, whereas the matrix still described the older comparison-file-list oracle and its cap.

**CORRECTION:** update `docs/agents/BUILD_TEST_MATRIX.md` to those exact source contracts. Preserve #556's accepted optimization, the distinct PR/post-merge classifier, fail-closed selected-gate composition and all reviewed pins. No workflow/protection change is made. A queue path classification is not evidence about arbitrary source-level document consumption; future consumer changes require their existing owner review.

### F17 — a presentation request is not a display acknowledgement

**FACT:** the pinned `wgpu 30.0.0` `Queue::present` API schedules presentation and returns unit; the submitted-work callback reports GPU completion, not displayed pixels. `Surface::configure` also has explicit outstanding-texture/configuration preconditions. Source `WindowsRenderer::present` updates its state counter after the request, not after an independent display observation.

**INFERENCE, high confidence:** treating that counter as confirmation of visible pixels or input-to-display latency would overstate evidence. **CORRECTION:** continuation Sections 5.3/6 require truthful present-requested/display-observed distinction, bounded device-lifetime notifications and safe frame/configuration ordering. This is a proof/lifetime contract, not a claim that a hardware failure was reproduced or fixed here.

## 11. Pinned upstream contract references

The selected public references were checked for the workspace's pinned API versions, not substituted with a latest-version example:

- `winit 0.30.13` IME source: <https://docs.rs/winit/0.30.13/src/winit/event.rs.html> — byte-indexed optional cursor ranges, empty-preedit clearing and commit/disable sequence.
- `winit 0.30.13` Window: <https://docs.rs/winit/0.30.13/winit/window/struct.Window.html> — IME enable and candidate-area geometry; ordinary keyboard events are absent during preedit.
- `wgpu 30.0.0` Queue: <https://docs.rs/wgpu/30.0.0/wgpu/struct.Queue.html> — submission versus scheduled presentation, callback completion and unsupported timestamp period.
- `wgpu 30.0.0` Surface: <https://docs.rs/wgpu/30.0.0/wgpu/struct.Surface.html> — acquired-frame lifetime and configuration preconditions.
- `wgpu 30.0.0` Device: <https://docs.rs/wgpu/30.0.0/wgpu/struct.Device.html> — separate loss/error notification and polling surfaces.

These checks refine requirements, not select a new library or grant raw backend/unsafe access. In particular, the top-level `wgpu 30.0.0` pin does not imply every resolved backend crate is `30.0.0`: the inspected lock entries include `wgpu-core 30.0.1` and `wgpu-hal 30.0.1`. A complete target-resolved dependency/code audit remains a different, explicitly bounded activity.

## 12. Publication, validation and remaining proof

The expanded candidate changes exactly the two existing #560 documents and the active build matrix. During this pass #560 integrated as `663bd35a5196a925fc6eb0318381ad0b97f4cc2c`, so the candidate is prepared as a normal follow-up from that protected base, not a write to its closed branch or a history rewrite. A complete comparison from the audited `6db1e95...` source adds only #559's unrelated WP3 allocation and the two #560 documents. Exact three-file blob readback confirms the prepared patch still applies unchanged; the audited runtime and enforcement sources did not drift. The original 31-path/blob inventory is preserved. Local verification binds exact baseline blobs, the three-file patch, document structure, preserved invariants and patch application/reversal. Those checks are document/provenance checks, not missing Rust/platform/hardware tests. Repository-native qualification must execute against the newly published exact head; prior green `db502e...` results do not qualify new bytes.

The source-derived input/renderer repairs already have a separate Draft #558 at live-read head `b2b6e5be610ae03642eeeec7cdd0ccd777d0d20d`. Do not duplicate its writer, presume its pending generation fix is delivered, or modify it from this documentation task. #502 resource authority and #162 allocation remain unchanged.

The local execution surface has no Rust toolchain or authorized Windows/GPU qualification host. No new physical IME/DPI/capture/device-recovery run, A/B measurement, release journey, complete third-party-code audit, independent review or protected integration is claimed. These limits do not invalidate the completed finite source/contract/enforcement review; they bound what it can prove. FOV remains evidence-gated and addons remain deferred.
