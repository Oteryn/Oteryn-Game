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
