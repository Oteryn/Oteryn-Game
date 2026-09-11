# Oteryn native client UI architecture correction continuation

- Date: 2026-09-11
- Repository: `Oteryn/Oteryn-Game`
- Protected base at branch creation: `main@6db1e95dcd0377d3258c045ea0b95f5d620c53f7`
- Predecessor correction: `docs/architecture/OTERYN_NATIVE_CLIENT_UI_ARCHITECTURE_CORRECTION_ADDENDUM_2026-09-10.md`, integrated by PR `#557`
- Evidence: `docs/architecture/reviews/OTERYN_NATIVE_CLIENT_UI_SOURCE_AUDIT_CONTINUATION_2026-09-11.md`
- Amendment source baseline: `main@663bd35a5196a925fc6eb0318381ad0b97f4cc2c`, after integration of #560
- Amendment evidence: `docs/architecture/reviews/OTERYN_NATIVE_CLIENT_UI_CONTRACT_ENFORCEMENT_AUDIT_2026-09-11.md`
- Status: **CONTRACT/ENFORCEMENT AMENDMENT CANDIDATE / FRESH EXACT-HEAD REVIEW REQUIRED**
- Runtime implementation authority: **NONE**
- Production/client/server/protocol/content mutation authority: **NONE**
- Final FOV policy: **UNDECIDED / EVIDENCE-GATED**
- Addon/mod/plugin architecture: **DEFERRED / FUTURE CONCEPT / OUT OF CURRENT IMPLEMENTATION SCOPE**

## 1. Purpose, precedence and bounded supersession

PR #557 is integrated and remains the primary native UI correction addendum. This continuation supplements it after the remaining requested input/renderer source files were read completely. It supersedes only the bounded clauses listed here; all unaffected #549/#551/#557 requirements remain binding.

The initial version of this continuation was integrated by #560. Its Git history and source-audit record remain historical evidence; the additional cross-contract clauses in this amendment require their own exact-head review and integration. They do not retroactively qualify earlier candidates.

This continuation changes no runtime code and grants no worker allocation.

Bounded supersession topics:

- P2 normalized-event ownership needed for native UI input;
- P2 native IME host wiring and privacy/range rules;
- P2 wheel/scroll unit and fractional semantics;
- P3 loss-domain classification and device-recovery evidence;
- future qualification matrix expansion from 15 to 19 grouped cases;
- exact existing-code repair requests to the repository control plane;
- P1 external dependency-neutrality evidence;
- P5 inherited settings scope, precedence and privacy;
- P3 surface-frame/callback lifetime and presentation evidence;
- the distinction between native synthetic fixtures, ADR-0007 E2E tiers and automated CI coverage.

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
- supplied preedit cursor/selection/range is validated before indexing and cannot panic or address outside the payload;
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

For the pinned `winit 0.30.13` contract, preedit offsets are UTF-8 byte offsets: validate payload bounds and character boundaries before indexing. Empty preedit clears composition; an absent cursor is not an invented zero-position caret. Tests must reflect the selected Windows IME event sequence rather than assume that enabling IME leaves the ordinary keyboard-text stream unchanged.

IME host commands are application/platform integration, not a new gameplay or protocol authority.

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

A surface-recreation test is not a GPU-device-recovery test.

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

### 6.1 Physical frame ownership at the pinned API boundary

P3 must coordinate world/UI submission, outstanding surface-frame ownership and surface configuration at the renderer-owned boundary. At `wgpu 30.0.0`, a previous surface texture still alive during reconfiguration is an API precondition violation. Release or consume each acquired frame before replacement configuration, including early-return, skip and recovery paths. Do not carry that frame through unbounded asynchronous UI work. Validate supported configuration and non-zero dimensions before the call; a Rust `Result` return elsewhere does not convert API precondition violations into a recovery proof.

Record acquisition, submission, presentation request, GPU completion and physical observation as distinct facts. `Queue::present` schedules presentation; neither that call nor a queue-completion callback alone proves pixels reached a display. Existing counters named `presented` must state their measured boundary. Pixel/interaction qualification still requires a named observation oracle; synthetic state counters do not become screenshots or hardware evidence.

A selected callback/mapping/timing path must have bounded progress and delivery ownership. Renderer/device callbacks must not directly mutate the retained UI; deliver bounded, generation-checked results to the application owner. Device replacement invalidates old completion/resource identities. Qualify the polling/submission path that actually drives callbacks, without adding an unrelated runtime or blocking the UI loop on unbounded GPU waits. Requested device features/limits, timestamp support and units belong in the measurement cell; adapter support alone does not enable a device feature. Unsupported timing is `UNAVAILABLE`, not a zero-duration success.

The implementation evidence must bind the exact lockfile, target, enabled features and backend. The audited lockfile resolves `wgpu 30.0.0` with `wgpu-core`, `wgpu-hal` and `wgpu-types 30.0.1`; the top-level version is not the identity of the entire renderer dependency closure. Library upgrades require rechecking the affected API contracts, not relabelling old evidence.

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

These are **19 grouped baseline future boundary/negative cases**. Sections 6.1, 10.1, 10.2 and 11.1 refine the applicable cross-contract and evidence oracles; the baseline count is not a claim of exhaustive or executed runtime tests.

## 9. Existing-code repair requests to #162

This document requests allocation; it does not self-issue a lease. At the amendment census, Draft PR #558 already carries an input/renderer prerequisite repair lane. Reconcile the existing lane through #162 before issuing any further work; do not create a replacement writer or treat its unmerged candidate as a protected-main fix.

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

These boundary decisions are required at their owning stages: dependency neutrality at P1, event/frame ownership at P2/P3 and durable setting scope at P5. Postponing them would permit incompatible public contracts, duplicate ownership or false qualification evidence.

The realistic alternatives are to leave each integration to infer these invariants or to state the minimum cross-owner contract now. The latter is selected: it prevents incompatible P1/P2/P3 interfaces and destructive P5 migration without selecting storage algorithms, libraries or a new framework. Its cost is explicit negative evidence at the affected slice, not an unrelated platform programme. Superseding changes require measured counterevidence and a replacement contract preserving authority, privacy, bounded lifetime and reproducible proof. `Must decide now? YES` for these invariants; implementation mechanisms remain evidence-selected.

The following remain deliberately undecided until implementation evidence exists:

- exact normalized-event variant/type names;
- exact scroll accumulation constants and numeric resource ceilings;
- retained-tree storage implementation;
- shader language/text library;
- final responsive versus fixed FOV policy;
- addon/mod/plugin platform design.

No architecture-as-product expansion is authorized.

### 10.1 P5 settings inherit the client contract

UI persistence must consume Section 15 of `ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_CONTRACT_CANDIDATE.md`, not create an independent settings policy. Each durable layout/preference field declares its semantic scope and permitted override behavior. Keep device geometry/scale applicability distinct from portable preferences. Ordinary precedence remains ephemeral, then DEVICE, OS_USER, ACCOUNT and product default where the field permits those scopes; INSTALLATION is not an unrestricted extra overlay.

The most restrictive valid privacy setting remains effective. A layout reset, migration, downgrade or account switch must not re-enable diagnostics disabled by OS-user/installation policy or overwrite unrelated identity/security settings. Treat an unavailable accepted account-settings service as an absent account layer, not permission to invent local account synchronization. Scope changes require source/destination scope, conflict rules and rollback/recovery in a versioned migration. Section 23 of the original correction addendum still supplies bounded failure-atomic persistence.

P5 qualification must cover conflicting permitted scopes, missing account layer, device change, privacy opt-out across reset/migration, downgrade and preservation of unrelated fields. Synthetic fixtures can prove local policy logic but cannot qualify an unimplemented account service.

### 10.2 P1 dependency neutrality needs the right graph

`tools/architecture-check` proves declared workspace-local edges and release-role closure. Its internal-edge projection does not by itself prove absence of external registry/git dependencies or all their transitive dependencies. Keep that existing check, and qualify `ui-core` neutrality against the actual resolved normal/build closure for the selected targets and feature sets, including optional and target-specific activation. Use package identities rather than dependency aliases or a substring-only tree search.

Inspect declared normal/build/optional/target-specific dependencies for forbidden platform/GPU/gameplay coupling, and bind the resulting evidence to manifests, lockfile and resolved metadata. State separately the development/test dependencies and what graph was checked. A host-default Linux `cargo tree` for `oteryn-client` is not an exhaustive neutrality proof for `ui-core` on Windows. Workspace-wide `unsafe_code = "forbid"` is not a source audit of third-party crates.

P1 exit requires a positive allowed-foundation case and rejection evidence for a direct forbidden package, a transitive forbidden package and a relevant target/feature-only forbidden edge. Add or extend enforcement only under an explicitly allocated implementation/control-plane slice, with its own regression evidence; this document neither weakens the existing checker nor claims that these future cases have run.

## 11. CI, rollback and review

This is a documentation-only correction candidate. It must be qualified by repository-selected checks for its exact head. Runtime E2E is not evidence generated by these Markdown changes; future P2/P3 tests remain unperformed until their implementation slices exist.

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

### 11.1 Evidence classification and stage admission

The current `docs/agents/BUILD_TEST_MATRIX.md` and the actual exact-head workflow bodies govern CI selection. A successful workflow can report semantic `NOT_APPLICABLE`: the #560 semantic-audit job did so with an empty selected-profile/check list. Existing dispatcher regression tests and workflow success must not be presented as an independent UI architecture KEEP. Any future UI profile must document its real coverage; no new required status or paid reviewer is selected here.

The phrase "Tier-2-equivalent" in Section 14.4 of the original correction addendum is superseded by this distinction:

| Evidence | What it can establish | What it does not establish |
| --- | --- | --- |
| Native UI qualification with labelled synthetic inputs | The exercised native input, layout and observed physical UI behavior in the named cell | ADR-0007 Tier 2, real admission/transport/server/persistence behavior or final FOV fairness |
| ADR-0007 Tier 2 | The declared real native-client E2E journey through its required production contracts with a bounded test adapter | Exact production-default release binary behavior |
| ADR-0007 Tier 3 | The declared journey using identified release artifacts without that in-process adapter | Exhaustive UI, fault or performance coverage |

P1-P6 may use appropriately labelled native synthetic fixtures without waiting for full server E2E. Supported product journeys still require their actual ADR-0007 evidence before product acceptance. `NOT_APPLICABLE` phase labels cannot erase the defining boundary of a claimed tier.

The inspected Windows jobs build the release client but invoke shell smoke through a separate `cargo run` without `--release`; they do not launch the identified release artifact as Tier 3. They run simulation-determinism tests, not the Windows unit/integration suites of every client/input/renderer dependency. P2/P3 owners must name and execute the affected package suites on `x86_64-pc-windows-msvc` and qualify native IME/DPI/capture/render paths with the required fixtures. Compilation or `--all-targets` Clippy does not execute those tests. Machine/driver/adapter and pixel-observation evidence must be retained for physical claims.

Historical ALPHA `#263` references denote `blakinio/Oteryn-v2#263`, not `Oteryn/Oteryn-Game#263`. Legacy issue status is provenance, not new Game execution or acceptance authority. Current Game contracts, live allocation and exact-head evidence remain controlling.

Rollback is documentation-only. Preserve historical #549/#551/#557/#560 evidence and revert only the bounded amendment if required; do not remove the valid earlier corrections.

```text
FOLLOW_UP_CORRECTION = CANDIDATE_AUTHORED
RUNTIME_IMPLEMENTATION_AUTHORITY = NONE
INDEPENDENT_REVIEW = REQUIRED_ON_EXACT_HEAD
FOV = UNDECIDED / EVIDENCE-GATED
ADDONS = DEFERRED / FUTURE CONCEPT
MERGE_AUTHORITY = REPOSITORY_CONTROL_PLANE_ONLY
```
