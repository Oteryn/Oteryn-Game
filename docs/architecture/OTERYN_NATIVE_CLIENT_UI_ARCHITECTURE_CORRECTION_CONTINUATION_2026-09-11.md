# Oteryn native client UI architecture correction continuation

- Date: 2026-09-11
- Repository: `Oteryn/Oteryn-Game`
- Original continuation protected base at branch creation: `main@6db1e95dcd0377d3258c045ea0b95f5d620c53f7`
- Predecessor correction: `docs/architecture/OTERYN_NATIVE_CLIENT_UI_ARCHITECTURE_CORRECTION_ADDENDUM_2026-09-10.md`, integrated by PR `#557`
- Evidence: `docs/architecture/reviews/OTERYN_NATIVE_CLIENT_UI_SOURCE_AUDIT_CONTINUATION_2026-09-11.md`
- Expanded-amendment publication base: `main@663bd35a5196a925fc6eb0318381ad0b97f4cc2c`, after #560 integration
- Status: **EXPANDED CORRECTION CANDIDATE / FRESH EXACT-HEAD REVIEW REQUIRED**
- Runtime implementation authority: **NONE**
- Production/client/server/protocol/content mutation authority: **NONE**
- Final FOV policy: **UNDECIDED / EVIDENCE-GATED**
- Addon/mod/plugin architecture: **DEFERRED / FUTURE CONCEPT / OUT OF CURRENT IMPLEMENTATION SCOPE**

## 1. Purpose, precedence and bounded supersession

PR #557 is integrated and remains the primary native UI correction addendum. This continuation supplements it after the remaining requested input/renderer source files were read completely. It supersedes only the bounded clauses listed here; all unaffected #549/#551/#557 requirements remain binding.

This continuation changes no runtime code and grants no worker allocation.

Bounded supersession topics:

- P2 normalized-event ownership needed for native UI input;
- P2 native IME host wiring and privacy/range rules;
- P2 wheel/scroll unit and fractional semantics;
- P3 loss-domain classification and device-recovery evidence;
- future qualification matrix expansion from 15 to 19 grouped cases;
- exact existing-code repair requests to the repository control plane;
- external dependency and Windows-test proof at UI slice exits;
- component qualification versus ADR-0007 end-to-end evidence;
- presentation-request versus display-observation evidence;
- interpretation of selected CI profiles and inherited settings/privacy scope.

The following remain unchanged:

```text
apps/client = sole production client composition root
ui-core -> no wgpu / winit / server / protocol / gameplay-authority / synthetic-only deps
renderer = physical GPU/surface/resource owner
production closure -X-> client-domain/client-simulation/synthetic harness
RESPONSIVE_FOV_POLICY = UNDECIDED
FIXED_FOV_POLICY = UNDECIDED
ADDONS = DEFERRED / FUTURE CONCEPT
```

## 2. Corrected P2 normalized-event ownership

The prior correction wording that `input-actions/**` may change only for missing semantic-action primitives is too narrow for the existing code ownership. `NormalizedInputEvent` and its framework-neutral physical values are currently owned under `crates/input-actions`.

P2 may therefore make the **minimum framework-neutral extension to the existing normalized-event/value contract** when a native UI input fact cannot otherwise be preserved without loss or duplication.

Allowed examples, exact type names implementation-local:

```text
IME preedit payload + optional range/cursor
IME enabled/disabled lifecycle fact when required by normalized flow
unit-aware/fraction-preserving UI wheel delta
```

This does not move widget state into the input crates.

Ownership remains:

```text
input-platform
    winit/OS extraction and platform normalization

input-actions
    framework-neutral normalized physical/value contracts,
    binding atoms, action contexts and semantic router

ui-core
    widget focus/modal/capture/drag/text-composition semantics

apps/client
    product composition and arbitration, including native host commands
```

No second normalized-event registry is permitted in `apps/client` or `ui-core`.

## 3. Native IME contract and host wiring

P2 must preserve the full native composition sequence required by the focused editor.

Required behavior:

- preedit text is retained as bounded transient UI state;
- supplied preedit cursor/selection/range is validated as byte offsets, including UTF-8 boundaries, before indexing and cannot panic or address outside the payload;
- committed text is inserted exactly once;
- `KeyEvent::text` and IME commit cannot double-insert one logical input;
- focus loss, IME disable, modal replacement and session/widget replacement cancel or clear transient composition safely;
- application diagnostics, errors and tracing must not reveal committed or preedit text merely to prove routing;
- Polish diacritics and a real preedit-before-commit sequence remain mandatory qualification cases.

`apps/client` owns the native host wiring:

```text
focused editable widget
    -> application arbitration
    -> request native IME enabled
    -> place native candidate/composition area at current editor caret/selection geometry
```

When editable focus is lost, native IME is disabled or redirected according to the selected platform contract. Candidate-area coordinates must use the same coherent DPI/user-scale geometry snapshot as hit testing and draw extraction; scale may be applied exactly once.

IME host commands are application/platform integration, not a new gameplay or protocol authority. The pinned `winit 0.30.13` contract distinguishes an empty preedit (clear) and an absent cursor range (hide); neither is an invalid committed-text value. During enabled preedit, ordinary `KeyboardInput` events are not delivered. Cancel/reconcile the affected semantic route before changing IME ownership rather than depending on a later ordinary key event to do it. A native qualification case must exercise this event gap and safe re-arming. Use actual platform outcomes for capture/IME integration; do not fabricate successful OS ownership from a UI request.

## 4. Corrected wheel/scroll contract

The existing discrete `WheelDelta`/direction route is adequate for bounded gameplay impulses but is not sufficient evidence for smooth UI scrolling because native line and pixel deltas are collapsed to a common integer and small values may disappear.

Before that information loss, P2 must expose a bounded UI-consumable scroll fact that preserves:

```text
unit = line | pixel (or an equivalent explicit representation)
fractional magnitude needed by the selected UI policy
axis/direction
finite/bounded value
```

Requirements:

- UI panel scrolling is arbitrated before unintended world/gameplay wheel action;
- gameplay may retain its existing discrete impulse semantics after UI ownership is resolved;
- tiny or fractional native UI deltas cannot be silently represented as proof of zero physical motion merely because the gameplay impulse rounds to zero;
- bounded accumulation, if used, must have explicit reset/overflow behavior;
- no wheel event may be applied both to a consuming UI surface and an unintended gameplay action.

P2 exit must include native line-delta and pixel-delta cases, sub-unit values, sign/axis behavior, modal/pointer-capture arbitration and scale changes.

## 5. Loss domains are separate contracts

The following must not be conflated:

```text
A. input-device reset/loss
B. presentation-surface loss/outdated/suboptimal/reconfiguration
C. GPU-device/queue loss or unusable device
```

### 5.1 Input-device reset

The input layer owns reconciliation of held atoms/modifiers/capture-related input facts and cancellation of ineligible semantic actions. It does not imply any renderer recovery.

### 5.2 Surface loss

The renderer may recreate/reconfigure a presentation surface while retaining the current device/queue when the selected backend supports that path. A successful surface recreation proves only the surface path recovered.

### 5.3 GPU-device loss

GPU-device loss requires evidence that the selected `wgpu`/Windows path can safely detect and rebuild every device-bound resource and queue dependency used by the product. If that recovery contract is not implemented and qualified, the supported behavior must be explicit terminal/fail-closed rather than described as recovered.

A surface-recreation test is not a GPU-device-recovery test. Device-loss and uncaptured-validation notification are separate from surface acquisition results in the pinned API. Any chosen callback/observation route must be bounded, tied to its device lifetime and reconciled by the app/renderer owner; an old-device notification cannot reset a replacement device. Keep callbacks short and do not mutate UI from an arbitrary callback or introduce an unbounded GPU wait on the event loop. Exact notification types and recovery strategy remain implementation choices.

Resource cache/budget/lifetime requirements continue to consume existing renderer resource authority (#502) and product allocation control (#162). This continuation creates no competing numeric cache budget and does not promote prototype capacity to a product ceiling.

## 6. Renderer generation ordering remains a prerequisite

The source-audited generation defect recorded by #557 remains an implementation prerequisite: stale generation must be rejected before physical acquisition/submission/presentation effects that are avoidable by validation.

P3 must prove at minimum:

```text
stale generation -> no surface acquisition/queue/present side effect attributable to the rejected call
same generation -> normal lifecycle preserved
presented suboptimal frame -> presentation accounting remains truthful even if later recovery fails
skipped/recovery-only success -> not counted as presented
```

Pure `SurfaceState` tests are insufficient to prove physical `WindowsRenderer` ordering.

The `wgpu 30.0.0` `Queue::present` call schedules presentation and returns no display acknowledgement. Record acquisition, submission, **present-requested**, skip and recovery outcomes separately; define any legacy `Presented` counter at that observable level. GPU-work completion is not proof that a compositor displayed the expected pixels. Claim display/pixel or input-to-display evidence only with a named observation method and frame/geometry identity. Geometry safety still requires coherent extraction/hit testing and conservative rejection of affected targeting when coherence is unknown; this does not require a new display-feedback API or blocking unrelated safe actions.

Before `Surface::configure`, retire old acquired surface textures and serialize incompatible submission/reconfiguration work. Validate non-zero dimensions and supported configuration before the call. These are physical lifetime/precondition requirements, not a replacement for generation fencing. Qualify resize/recovery with an outstanding-frame case, and retain explicit terminal behavior when safe recovery is unavailable.

## 7. Modifier/lifecycle prerequisite remains explicit

P2 must qualify the complete adapter -> arbitration -> router stream. Physical modifier key events must not accidentally become extra non-modifier chord atoms when modifiers are already represented as modifier state.

Required cases:

- Ctrl/Shift/Alt/Super press + ordinary key;
- left/right modifier release semantics;
- repeat handling;
- consumed UI press followed by modal/focus changes;
- release while UI owns the event;
- focus/capture/device reset with held action;
- fresh eligible re-press after the prior press was blocked.

`ContextKind::Global` remains a routing priority/context property, not permission for a gameplay intent to bypass modal/text ownership.

## 8. Expanded qualification matrix

The 15 grouped negative/boundary cases already integrated by #557 remain mandatory where their owning slice applies. Add these four groups:

| New group | Minimum oracle | Owning slice |
| --- | --- | --- |
| Native IME payload/range/privacy | preedit retained; invalid/out-of-range indices rejected safely; commit exactly once; diagnostics redact text | P2 |
| Native IME host placement | enable/disable and candidate area follow editable focus, DPI/UI scale and window lifecycle without double scaling | P2 |
| Native line/pixel/fractional scroll | unit and needed fraction preserved for UI; bounded; UI consumption cannot leak to gameplay | P2 |
| Loss-domain separation | input reset, surface recovery and GPU-device loss/rebuild-or-terminal behavior are independently observable and truthfully reported | P3 |

Therefore the current architecture qualification plan contains **19 grouped future boundary/negative cases**. This number describes planned test groups, not completed runtime tests.

### 8.1 Physical component proof is not another E2E tier

Any earlier `Tier-2-equivalent` wording in the UI addendum is boundedly superseded: a synthetic world/HUD rendered in a native window is **physical component qualification**, not ADR-0007 Tier 2. It may qualify the applicable P3-P6 local rendering/interaction requirement without waiting for a complete gameplay stack, but cannot qualify Platform/Gateway/admission/protocol/server/persistence or production gameplay readiness. Tier 2 must traverse the real accepted client/service path with its bounded observation/input adapter. Tier 3 must use the identified production artifacts and release journey. The parent ADR remains unchanged; no fourth tier is created.

### 8.2 Dependency proof belongs to the actual package and target

P1 must supplement the current workspace role/local-edge validator with explicit evidence that the **actual `ui-core` package**, including relevant optional, build, dev and target-specific declarations and resolved transitive dependencies, cannot acquire the prohibited framework/platform/GPU/gameplay/synthetic coupling. Distinguish declaration constraints from each supported build's resolved graph. A Linux/default-feature client dependency tree or a supply-chain license/advisory check is not that proof. Bind package IDs/versions, feature sets and target triples; missing or incomplete evidence cannot satisfy the exit.

Use the existing validator owner for the smallest needed executable negative checks when the package is introduced. Cases must reject a forbidden outward local edge and direct/transitive external framework coupling without relabelling a forbidden dependency as production-safe. Do not add unused packages, a parallel policy registry or speculative generic enforcement in this documentation slice. P2/P3 must reconcile feature-unification changes against the same contract before claiming neutrality.

### 8.3 Execute platform tests and the artifact actually claimed

Canonical Windows lanes currently build/lint the client, run a pre-native shell smoke and synthetic harness, and test simulation determinism. They do not execute the Windows-only `input-platform::winit_adapter` unit tests. The Linux workspace job cannot execute code under `#[cfg(windows)]`; client Clippy is not dependency unit-test execution. At P2 exit, require Windows execution of the owning package tests, with observed names/counts, followed by the applicable native IME/DPI/capture cases. A filtered command running zero relevant tests is not qualification.

For the current package, the focused command is `cargo +1.94.0 test --locked -p oteryn-input-platform --target x86_64-pc-windows-msvc`. It is a required future qualification command, not an execution claim made here. Any integration into pinned canonical jobs requires the existing CI owner's reviewed change; this document changes no workflow.

The current `--release` build is followed by a `cargo run ... -- --smoke` command without `--release`. Report that as release-build evidence plus development-profile pre-native smoke, not execution of the release artifact. Release qualification must run the exact identified built artifact, retain its hash and satisfy the named journey; adding a release flag alone does not supply rendering, Tier-2 or Tier-3 evidence.

### 8.4 Inherited settings and privacy authority

P5 layout persistence and P8 settings adapters must use the ALPHA-CLIENT contract's semantic setting scopes and restrictive privacy precedence. An absent account service is not permission to invent account-wide state. Resetting a layout, moving to another device or recovering a corrupt settings file cannot silently re-enable diagnostics. The existing privacy baseline continues to exclude private chat/credentials from diagnostics and keeps privacy opt-out independent of gameplay permission or suspicion scoring. This is an inherited acceptance obligation, not a new account API, legal-compliance claim or telemetry implementation.

## 9. Existing-code repair requests to #162

This document requests allocation; it does not self-issue a lease. The expanded live census found the existing Draft PR #558, branch `agent/native-ui-audit-prerequisites-20260910`, at `b2b6e5be610ae03642eeeec7cdd0ccd777d0d20d`. Reconcile these requests with that writer and its current ownership before any allocation; do not create duplicate repairs from this document. Draft code, its earlier negative population and a future green run are different evidence states; protected integration/readback remains necessary before declaring a prerequisite delivered.

### 9.1 Input modifier/lifecycle prerequisite

Requested paths:

```text
crates/input-actions/src/physical.rs
crates/input-actions/src/router.rs
crates/input-actions/src/lib.rs
crates/input-platform/src/tests.rs   # regression seam only as needed
```

Acceptance:

- conventional modifier chords match the complete native-adapter event stream;
- physical modifier atoms do not corrupt non-modifier chord identity;
- left/right release and repeat behavior are explicit;
- UI-consumed release/context transitions leave no stale semantic action;
- no product keymap, UI tree, Cargo or server change is bundled into the prerequisite.

### 9.2 Renderer generation prerequisite

Requested paths:

```text
crates/renderer/src/windows.rs :: WindowsRenderer::render/present
allocated regression seam under crates/renderer
```

Acceptance:

- stale generation is rejected before avoidable physical surface/queue effects;
- same-generation behavior and bounded recovery remain correct;
- the test distinguishes state-machine rejection from physical renderer ordering;
- no world renderer, cache budget, Cargo or runtime activation change is bundled into the prerequisite.

## 10. Decision timing

These boundary decisions are required before P2/P3 implementation because postponing them would permit incompatible public event contracts, duplicate ownership or false recovery evidence.

The following remain deliberately undecided until implementation evidence exists:

- exact normalized-event variant/type names;
- exact scroll accumulation constants and numeric resource ceilings;
- retained-tree storage implementation;
- shader language/text library;
- final responsive versus fixed FOV policy;
- addon/mod/plugin platform design.

No architecture-as-product expansion is authorized.

For the expanded evidence corrections, **Must decide now? YES** at the proof/ownership level: P1 could otherwise pass with an unexamined framework dependency, P2 with unexecuted platform tests, and P3-P6 with misclassified presentation/E2E evidence. The realistic alternatives are accepting broad green job labels or requiring the named observable contract. The latter is selected because it avoids false readiness while permitting local component proof before full gameplay exists. Costs are explicit target tests and evidence bookkeeping, not a new UI framework. The risk of over-blocking is bounded by the component/E2E distinction above. Superseding evidence requires a reviewed validator, target-executed test, artifact-observation mechanism or parent-contract change that proves an equivalent boundary. Exact libraries, numeric budgets, storage and scheduling algorithms remain undecided.

## 11. CI, rollback and review

This is a documentation-only correction candidate. It must be qualified by repository-selected checks for its exact head. Runtime E2E is not evidence generated by these Markdown changes; future P2/P3 tests remain unperformed until their implementation slices exist.

Record both the workflow conclusion and the actual selected profile/verdict/check list. The prior #560 semantic workflow run `34562444300` succeeded but emitted `NOT_APPLICABLE`, with `profiles: []` and `checks: []`; it did not semantically qualify the UI clauses. Its dispatch regressions are not a UI-specific review. A missing profile, a skipped lane, a compile-only result and independent architecture acceptance must never be conflated.

Use the current build matrix's distinct PR, Merge Queue and post-merge selection rules. Protected #556 introduced an architecture-document-only queue lane; it did not change PR/post-merge classification or make a green aggregate physical UI proof. Preserve that accepted routing and all required selected gates. Changes to these candidate bytes invalidate the previous exact-head qualification, even when previously audited source files are unchanged.

Before integration:

```text
fresh independent review bound to repository + follow-up PR + base=main + exact head
-> zero unresolved material findings / accepted disposition
-> repository-required exact-head checks
-> normal Merge Queue only
-> real merge_group aggregate game-gate
-> protected-main readback
```

Do not direct-merge, bypass, force-push, weaken protection, suppress tests or use a no-op commit to obtain new checks.

Rollback is documentation-only. Preserve historical #549/#551/#557 evidence and revert only this bounded continuation if required.

```text
FOLLOW_UP_CORRECTION = CANDIDATE_AUTHORED
RUNTIME_IMPLEMENTATION_AUTHORITY = NONE
INDEPENDENT_REVIEW = REQUIRED_ON_EXACT_HEAD
FOV = UNDECIDED / EVIDENCE-GATED
ADDONS = DEFERRED / FUTURE CONCEPT
MERGE_AUTHORITY = REPOSITORY_CONTROL_PLANE_ONLY
```
