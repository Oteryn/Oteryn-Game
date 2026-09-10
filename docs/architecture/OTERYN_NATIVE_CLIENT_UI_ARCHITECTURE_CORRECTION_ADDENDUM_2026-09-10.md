# Oteryn native client UI architecture correction addendum

- Date: 2026-09-10
- Repository: `Oteryn/Oteryn-Game`
- Corrects/supersedes bounded clauses from PR `#549`
- Audit source: `docs/architecture/reviews/OTERYN_NATIVE_CLIENT_UI_PR549_INDEPENDENT_AUDIT_AND_REQUIRED_CORRECTIONS_2026-09-10.md`
- Audited PR head: `83314bcbf2978b20f07ca539dd697900dcdb410f`
- Protected baseline containing the audited documents: `main@7144c0b9ec8691e481df058c85d890ac88d32461`
- Follow-up source audit: `main@5025be6cf3f5140cf94708f8e6ddc9ab3f40d99f`; evidence in `docs/architecture/reviews/OTERYN_NATIVE_CLIENT_UI_SOURCE_AUDIT_2026-09-10.md`
- Status: **CORRECTION CANDIDATE / FRESH EXACT-HEAD REVIEW REQUIRED BEFORE INTEGRATION**
- Runtime implementation authority: **NONE**
- Production/client/server/protocol/content mutation authority: **NONE**
- Final FOV policy: **UNDECIDED / EVIDENCE-GATED**
- Addon/mod/plugin architecture: **DEFERRED / FUTURE CONCEPT / OUT OF CURRENT IMPLEMENTATION SCOPE**

## 1. Purpose and supersession boundary

This addendum applies the independent `FIX` findings recorded after PR #549. It does not reopen the native Rust + `wgpu` direction, the ALPHA-CLIENT authority boundary, the Visual World Slice authority model, or the deliberate deferral of final FOV and addon-platform decisions.

When this addendum conflicts with the three documents delivered by PR #549, this addendum supersedes only these topics:

- UI-P1 workspace/Cargo ownership and entry conditions;
- input routing/arbitration and IME semantics;
- active-action cancellation when UI ownership changes;
- shell/bootstrap composition ownership;
- P2/P3 shared-path serialization;
- P5/P6 qualification-host and physical-world seam;
- stage-specific physical/Tier-2 evidence;
- interaction-critical fail-closed behavior;
- Variant-B experiment definition and A/B population identity;
- P8 dependency shape relative to P7;
- retained-tree decision analysis completeness;
- Cargo dependency direction versus presentation data flow;
- physical-input cleanup versus semantic-action admission;
- geometry/hit-test/draw coherence and bounded resource admission;
- renderer generation validation and actual presentation evidence;
- retained-node and session-observation lifetime boundaries;
- local-layout persistence/rollback safety;
- P4 prerequisite precision and A/B measurement accounting.

All unaffected requirements in the original UI baseline, implementation plan, and viewport experiment remain binding according to their current repository status.

The original document versions remain preserved; unaffected requirements remain applicable as stated above. In particular, baseline Section 26 is a presentation data-flow sketch, not permission for a Cargo edge from `ui-core` to a renderer. Section 2.2 here is the compile-time authority. Implementation-plan Sections 5.2, 10, 11 and 20 must be read with the corrected input, DAG and workspace clauses here; viewport-experiment Section 6 must be read with the exact primary-B policy in Section 16. Neither green checks nor an earlier review of different correction bytes qualifies this revision.

This addendum grants no worker allocation and no runtime authority.

The correction lifecycle is fail-closed and ordered:

```text
correction candidate exact head
  -> fresh independent exact-head architecture review
  -> KEEP with zero unresolved material findings
  -> repository-required exact-head checks
  -> normal Merge Queue only
  -> merge_group game-gate
  -> protected-main readback
  -> fresh shared-workspace ownership census/allocation
  -> UI-P1 may be admitted
```

A correction candidate MUST NOT be integrated merely because documentation CI is green. If review returns `FIX` or `NEEDS_DECISION`, repair/review continues on the correction branch and the previous review target becomes historical.

## 2. Decisions preserved unchanged

### 2.1 Product/authority boundary

```text
server/gameplay authority
        |
        v
client projection / view-safe state
        |
        +----------------------+
        |                      |
        v                      v
world render input          UI view model
        |                      |
        v                      v
world renderer          retained UI semantics
                               |
                               v
                           UiDrawList
                               |
                               v
                         physical UI renderer
```

The UI remains presentation and interaction. It never becomes gameplay authority.

`apps/client` remains the sole production native-client composition root unless a later explicit accepted architecture decision changes that boundary.

The production dependency closure must not reach synthetic-only client-domain/simulation/assets/harness packages.

### 2.2 Renderer boundary

`crates/renderer` remains the physical GPU/surface/resource owner. It may depend on a framework-neutral UI draw contract. `ui-core` must not depend on `wgpu`, `winit`, platform window APIs, protocol/server crates, gameplay-authority crates, or product-specific HUD panels.

Data-flow arrows are not Cargo dependency arrows. The compile-time shape for the first foundation is expected to permit:

```text
apps/client -> ui-core
renderer    -> ui-core       # when P3 consumes renderer-neutral UiDrawList/types

ui-core     -X-> renderer
ui-core     -X-> wgpu
ui-core     -X-> winit
ui-core     -X-> synthetic-only packages
```

At P1 do not add speculative renderer/app edges before the corresponding real dependency exists. `workspace-boundaries.toml` must match exact Cargo metadata at every candidate.

### 2.3 FOV and addons

The product FOV remains:

```text
RESPONSIVE_FOV_POLICY = UNDECIDED
FIXED_FOV_POLICY = UNDECIDED
```

Responsive FOV may be exercised as client/synthetic experiment behavior, but product acceptance still requires separately authorized server relevance/network/fairness evidence if the client wants more authoritative world information.

Addon/mod/plugin/community customization remains:

```text
DEFERRED / FUTURE CONCEPT / OUT OF CURRENT IMPLEMENTATION SCOPE
```

No addon runtime, scripting VM, manifest/package format, community widget API, sandbox/capability system, hot reload, marketplace, signing, or distribution implementation is authorized by this UI programme.

## 3. Corrected UI-P1 workspace scope

The original P1 two-path statement is superseded.

### 3.1 Mechanically complete surfaces

The first implementation remains one framework-neutral UI foundation, but the complete workspace surfaces are:

```text
NEW/MODIFY: crates/ui-core/**
MODIFY:     Cargo.toml
MODIFY:     workspace-boundaries.toml
MODIFY:     Cargo.lock                    # only when produced by the exact package/dependency delta
```

No server/gameplay/protocol/production-networking/client-domain semantic/art/final-HUD work is added to P1.

### 3.2 Required workspace declaration

P1 must register the selected package name, expected to be `oteryn-ui-core` unless exact implementation review selects another non-conflicting name, in the repository's enforced workspace policy.

The package must receive exactly one release role. For the native product foundation the expected role is **production**, because production client and renderer may consume it. A different role requires explicit architecture justification and production-closure proof.

The policy must declare exact internal dependency edges for the package and for any existing package newly depending on it.

Expected direction:

```text
ui-core -> only framework-neutral production foundations proven necessary
apps/client -> ui-core
renderer -> ui-core       # only once P3 actually introduces this dependency
```

### 3.3 P1 mechanical proof

At minimum:

```text
cargo +1.94.0 metadata --locked --format-version 1
cargo +1.94.0 fmt --all --check
cargo +1.94.0 run --locked -p oteryn-architecture-check -- workspace .
cargo +1.94.0 test --locked -p <ui-core-package>
cargo +1.94.0 clippy --locked -p <ui-core-package> --all-targets -- -D warnings
repository-selected exact-head gate
```

Current protected repository CI remains authoritative. No policy/validator/protection weakening is permitted.

### 3.4 P1 shared-workspace entry gate

Immediately before allocation/admission, the coordinator must perform a fresh live ownership census for:

```text
Cargo.toml
Cargo.lock
workspace-boundaries.toml
```

P1 may start only when:

```text
P1_WORKSPACE_ENTRY_READY =
    correction integrated and read back from protected main
AND correction's pre-integration exact-head independent review was KEEP
AND no conflicting active writer owns root Cargo/Cargo.lock
AND workspace-boundaries.toml ownership is explicitly available/allocated
AND exact P1 branch/base/owned paths are recorded
```

At the original audit snapshot, Issue #351 / Draft PR #356 held the serialized root Cargo lease. That is historical/live execution evidence, not a permanent dependency. The start gate must use current GitHub ownership.

## 4. UI-P1 size and sub-slicing

P1 must remain a minimal coherent foundation rather than a requirement to land every generic UI facility in one oversized PR.

If reviewability requires it, the semantic P1 programme may be split into bounded serial sub-slices without creating speculative crates. A reasonable order is:

```text
P1a: node identity/tree + geometry + deterministic layout + draw primitives
P1b: focus/modal/capture + interaction-state transitions
P1c: minimal docking/virtualization/text interfaces/diagnostics required by P2-P5
```

This is not a mandatory three-PR topology. One P1 PR remains valid if its public API and review surface stay bounded.

Do not add broad product panels, market/quest/social frameworks, theme marketplaces, extension APIs, or unrelated UI systems in P1.

### 4.1 Node lifetime and bounded foundation inputs

Stable identity includes lifetime, not just a reusable array index. Removing/replacing a node must invalidate its focus, hover, capture, drag, tooltip, queued callback and measurement references before storage can identify another node. A recycled virtualized row must retain the semantic item identity separately from its visual slot. Choose the smallest implementation that rejects stale references; no generational-arena library or storage algorithm is selected here.

Before allocation, traversal or expansion, P1 must define checked, finite limits for its implemented surfaces: live nodes, tree/dock/clip depth, dirty work, draw primitives, visible rows/overscan and queued interaction work. Reject cycles, non-finite geometry/scales, invalid extents and arithmetic overflow before mutation. Limit values belong to the owning implementation and qualification fixture, not a speculative global UI framework. A consumed critical event or budget denial must not become gameplay pass-through.

### 4.2 One geometry contract

Keep OS physical coordinates, platform-logical coordinates and user-scaled UI layout units distinguishable. Apply each scale exactly once. The conversion between a UI-local point and physical client coordinates must include its layout origin, platform DPI and user UI scale; the inverse used by hit testing must match. World zoom and Variant-B fit scale remain separate transforms.

Layout, clipping, hit testing and draw extraction must use a coherent geometry snapshot. Specify rounding and half-open edge rules, intersect nested clips, and clamp physical scissors to the target before integer conversion. Empty/minimized viewports produce no world hit and no division by zero. Reject unsupported/non-finite/overflowing dimensions before traversal, tile-extent expansion or GPU configuration. Bounds are resource-safety constraints, not a selected product FOV.

## 5. Corrected input architecture

The original diagram routing all UI input through `input-actions` is superseded because current `input-actions` is not a general UI event bus.

### 5.1 Ownership

```text
input-platform
    platform/winit normalization and lifecycle facts

input-actions
    framework-neutral physical binding atoms, semantic action contexts,
    held-action lifecycle and gameplay/action routing contracts

ui-core
    framework-neutral UI hit/focus/modal/drag/text-interaction state

apps/client
    application-level arbitration/composition between UI and gameplay/action routes
```

### 5.2 Correct event flow

```text
winit / OS event
      |
      v
input-platform normalization
      |
      v
normalized event / bounded UI-relevant primitive
      |
      v
apps/client input arbitration
      |
      +--> ui-core interaction/text/focus route
      |       |
      |       `--> UiIntent / consume / reserve / local UI transition
      |
      `--> input-actions semantic gameplay/action route
              physical cleanup always reconciled; semantic emission only when eligible
```

This must not duplicate physical normalization in `apps/client` or `ui-core`. The eligibility condition applies to semantic action admission, not to dropping the router's physical lifecycle stream. Section 7 specifies the cleanup obligation.

### 5.3 Deterministic arbitration

For every normalized event, the app must determine UI ownership/reservation before unintended gameplay action emission.

At minimum:

- text focus reserves text-producing keyboard behavior from gameplay;
- modal scope reserves interaction outside the modal route;
- UI pointer capture receives the matching pointer lifecycle until completion/cancel;
- UI drag capture and confirmed OS pointer capture remain distinct facts;
- failure/absence of interaction-critical arbitration does not default to permissive gameplay pass-through;
- one physical event cannot produce both a UI action and an unintended gameplay command.

## 6. Full IME correction

P2 must support the minimum bounded IME semantics required for Windows text entry. Existing `Ime::Preedit(text, cursor)` data cannot be discarded by the final implementation.

Required framework-neutral semantics, exact names implementation-local:

```text
IME enabled/disabled
preedit text update
preedit selection/cursor/range when supplied
commit text
composition cancellation/clear
focus-loss cancellation/reset
```

Requirements:

- preedit is local presentation state, never gameplay authority;
- committed text is emitted exactly once;
- keyboard `KeyEvent::text` and `Ime::Commit` do not double-insert one logical text input;
- disabling IME or losing focus clears/cancels transient composition safely;
- composition text/ranges are bounded as untrusted UI text;
- tests cover Polish diacritics and a real preedit-before-commit sequence.

`input-platform/**` may change in P2 only for demonstrated missing normalization primitives. `input-actions/**` may change only for demonstrated missing semantic action primitives. Widget focus/composition ownership does not move into either crate.

## 7. Active gameplay action cancellation on UI ownership change

Input eligibility changes must reconcile already-active semantic actions, not only future key presses.

When UI state makes an existing gameplay/action route ineligible, the app/input layer must deterministically emit/cause `Ended` or `Cancelled` according to the owning `input-actions` contract.

Required cases:

```text
gameplay key held -> text field gains focus
gameplay action active -> modal opens
pointer action active -> UI drag/modal capture takes ownership
application focus lost while action is held
OS pointer capture lost
device reset/loss while UI interaction is active
IME composition starts while a text-producing gameplay binding is otherwise eligible
```

No stuck gameplay action may survive these transitions.

Cancellation and physical release accounting are different obligations. At the source-audit snapshot, `InputRouter::set_context_active()` calls `cancel_ineligible()` without clearing `held`; a release removes the atom only through `process_button()`. Therefore the app must not implement `if UI consumed { return; }` before required release/focus/capture/device cleanup is reconciled by every state owner. Cleanup must remain idempotent and cannot emit a new gameplay `Started`/`Repeated` action.

P2 must define re-arming: a press consumed while the route is ineligible cannot become a gameplay press merely because a modal closes, a widget disappears, a different key is pressed or a platform repeat arrives. Require a fresh eligible press after release, unless an explicitly reviewed action contract specifies another behavior. Reuse/extend the owning semantic router only as needed; do not fabricate OS focus/device-loss facts or introduce a second physical-key registry to achieve this.

`ContextKind::Global` remains eligible under text/modal contexts in the current `BindingMap`. Global is not a gameplay-authorization bypass. App-owned mappings must explicitly distinguish permitted cross-mode local actions from gameplay intents; modal/text ownership still blocks unintended gameplay commands, regardless of binding priority. When a modal supersedes a captured background target, cancel or transfer ownership before routing the next semantic event.

## 8. P2 client composition correction

P2 must reconcile existing pre-native composition instead of adding a third owner.

Required ownership:

```text
apps/client
    composition and ordering of runtime, input arbitration,
    UI semantics, window lifecycle, renderer and product view adapters

client-runtime
    async runtime/cancellation lifecycle only

input-platform
    platform normalization only

input-actions
    semantic action contracts/router only

ui-core
    framework-neutral UI semantics only

renderer
    physical GPU/surface/resource state only
```

`ClientBootstrap` and the interactive Windows shell may be refactored/combined internally, but P2 must not leave contradictory production ownership paths.

The current pre-native gameplay-entry fail-closed behavior remains unchanged unless a separate authorized gameplay/client-runtime programme changes it.

## 9. P2/P3 concurrency correction

P2 and P3 may proceed semantically in parallel after stable P1 contracts only when their actual writable paths are disjoint.

Shared surfaces requiring explicit serialization as applicable:

```text
Cargo.toml
Cargo.lock
workspace-boundaries.toml
apps/client/Cargo.toml
apps/client/src/ui/** composition glue
public ui-core draw/input contract files
```

Allowed execution patterns:

- serial P2 then P3;
- serial P3 then P2; or
- parallel disjoint workers plus separately serialized shared-path prerequisite/lease.

This is execution planning, not a new architecture decision.

## 10. P3 renderer seam

P3 remains responsible for a bounded physical UI draw path:

```text
UiDrawList -> renderer-owned extraction/batching/resources -> wgpu submission
```

Requirements:

- `ui-core` exposes renderer-neutral primitives only;
- renderer owns pipelines, bind groups, buffers, atlases/caches and device/surface resources;
- renderer failure/loss does not mutate semantic UI/game state;
- UI and world timing remain logically separable;
- the first physical fixture proves actual rendered primitives/pixels, not only successful surface creation.

A physical pass may later be merged for optimization only if semantic separation and observability remain intact.

### 10.1 Presentation result and generation fence

At `main@5025be6cf3f5140cf94708f8e6ddc9ab3f40d99f`, `WindowsRenderer::render()` returns `Result<(), RendererError>` and `execute()` returns `Ok(())` for `SkipTimeout` and `SkipOccluded`, as well as successful reconfiguration without presentation. `Ok(())` is not evidence that the requested UI/world frame was presented. A suboptimal frame can also be presented before subsequent reconfiguration fails. P3 must expose or derive truthful per-attempt presentation/recovery facts; exact result type names remain implementation-local.

The current `present()` performs queue submission/presentation before `SurfaceState::apply(Presented)` validates the supplied generation. Before P3 qualifies this path, validate generation and relevant lifecycle/input preconditions before surface acquisition or other physical side effects. A stale generation must not submit, present, reconfigure or consume resources on behalf of the current generation. This source finding is not repaired by this documentation change.

Surface retry/skip, successful presentation, recovery and terminal failure must remain distinguishable in tests/counters. Preserve existing bounded recovery and fatal behavior; do not add unbounded retry or weaken the generation fence. A new hit-test layout cannot silently become actionable against an old displayed layout: route against coherent presented geometry or fail closed for affected targeting until coherence is restored. Decorative frame failure need not block unrelated safe local actions.

### 10.2 Shader, ordering and cache proof

The source workspace enables `wgpu` with `default-features = false` and only `std`/`dx12`. P3 must explicitly qualify the chosen shader representation, required feature/dependency delta, pipeline format/blend conventions and Windows backend. Clear-pass success proves none of those. Serialize any resulting Cargo/lockfile delta under Section 9; this clause does not choose a shader language or enable unrelated default features.

Batching must preserve painter order for overlapping transparent primitives and clip/transform boundaries. Sorting the entire draw list by texture is not generally order-preserving. Use overlapping-primitive and nested-clip fixtures as an independent oracle, not only primitive counts. Renderer-local glyph/image cache identities must include the relevant content/font/style/scale and device/resource lifetime; eviction or reconstruction must not leave a draw list pointing at recycled incompatible resources.

## 11. P4 text/resource boundary

P4 may select a reversible shaping/rasterization implementation behind the semantic boundary.

Required behavior:

- Unicode and Polish diacritics measure and render correctly;
- glyph/fallback caches are reconstructible after renderer loss;
- untrusted text has bounded size/work behavior;
- app/domain/panel logic owns no GPU glyph resources;
- exact library choice remains implementation evidence unless a later stable/public contract requires it.

## 12. P5 shared HUD composition and synthetic boundary

P5 must define one reusable production-safe HUD composition seam before synthetic qualification can claim representative coverage.

Allowed dependency direction:

```text
production-safe UI semantics/composition API
          ^
          |
synthetic qualification adapters/harness
```

Forbidden production direction:

```text
apps/client / ui-core / renderer
          -> client-domain synthetic-only
          -> client-simulation synthetic-only
          -> synthetic-assets
          -> synthetic-client-harness
```

Synthetic fixtures may adapt synthetic state into the same presentation-safe view-model/input contracts used by the HUD, but production packages never depend on synthetic-only packages.

Before P5 is physically demonstrated, the selected qualification host must instantiate the same UI semantics and drive an actual physical renderer path, not merely calculate `SurfaceState` transitions or print state.

The host may be an explicitly test-only native-client mode/profile, a dedicated native qualification binary with production-safe dependencies, or another bounded host accepted during implementation review. It must not become a second production client composition root.

Any test-only profile must still satisfy the repository's actual dependency validator; a disabled feature or friendly profile name is not an exemption from production-closure rules. Select the host/package edges against current Cargo metadata before allocation. Do not weaken workspace validation to legalize a synthetic dependency from a production package.

### 12.1 Observation and asynchronous lifetime

P1 owns generic stale-node rejection; the app/view-model adapter owns session/projection freshness in P5/P8. Do not put protocol or synthetic session types into `ui-core`. Use the owning production-safe observation contract when available; synthetic fixtures may model the same lifetime behavior without becoming that contract.

A drag/drop, selected row, pending command preview or delayed result must retain sufficient semantic target and observation context to reject a replaced/reused target. Session change, disconnect/resynchronization, projection replacement or node removal must cancel/revalidate affected pending interactions before dispatch. Do not replay them into a newer session, infer server success from an animation, or invent a new command retry/idempotency protocol. Existing admission/command contracts remain authoritative.

The event-loop/app owner applies UI mutations. Widgets may not create their own runtimes, threads or unbounded tasks. Any offloaded shaping/loading work must have bounded admission, an owned lifetime, cancellation and stale-result rejection before applying its result. Reuse `client-runtime` only at the app-owned boundary; a `JoinHandle` or timeout alone is not proof that work stopped. No new scheduler framework is required by this clause.

## 13. P6 physical world + HUD qualification gate

P6 requires a real physical world-render seam before claiming an A/B result representative of intended product presentation.

Current isolated World+VFX experiment evidence may inform or supply a later promotion/reuse plan, but UI work must not silently migrate broad experiment code under an unspecified `renderer/world presentation paths` allowance.

Before P6 admission, one bounded allocation must establish how the physical qualification host obtains:

```text
representative RenderSnapshot / PresentationEvents / EnvironmentState
        -> physical world presentation
        + representative HUD/UI draw path
        -> one actual wgpu frame composition
```

The source may be a production-ready world renderer by then or an explicitly bounded qualification-only bridge. Either way:

- provenance/evidence classification is explicit;
- synthetic world inputs remain labelled synthetic where applicable;
- production client gains no synthetic-only dependencies;
- server/gameplay authority does not move into renderer/UI;
- reuse/promotion from `experiments/world-vfx-prototype` requires explicit path allocation/review.

P1-P4 are not blocked on full World+VFX completion. This is a P5/P6 physical-proof gate.

## 14. Stage-specific test/evidence correction

General green CI is necessary but not sufficient for physical client/UI proof.

### 14.1 P1

Require deterministic foundation tests, workspace metadata/boundary proof, package strict Clippy/tests, and repository exact-head gate. Physical GPU proof is not required merely to establish pure `ui-core` semantics.

### 14.2 P2

Required deterministic/client cases:

- resize small -> large -> small;
- scale-factor transition;
- zero/minimized and restore;
- app focus loss/regain;
- OS capture gain/loss;
- UI pointer/drag capture transition;
- text focus/modal reservation;
- IME enable/preedit/commit/cancel/disable;
- active semantic action cancellation on UI ownership changes;
- pre-native gameplay unavailability remains fail-closed.

Where platform behavior cannot be simulated faithfully, retain a named Windows-native qualification case rather than replacing it with a pure unit claim.

### 14.3 P3/P4

Require actual physical `wgpu` evidence for primitives, clipping/scissor, text/glyph rendering, z/order, resize/reconfigure, resource reconstruction where applicable, and independent UI timing/counters.

### 14.4 P5/P6

Require a named Windows native/hardware/scene qualification cell and Tier-2-equivalent interaction/render evidence where applicable to user-observable UI behavior. A console/headless synthetic harness alone cannot be reported as native-client UI/render proof.

### 14.5 Release

Tier 3 production-binary evidence remains a later release/product gate. It does not block `ui-core` or the qualification HUD.

### 14.6 Source-driven negative and boundary matrix

These are required future test cases, not tests claimed executed by the architecture audit. Each owning slice must bind actual test names, commands, runner/target, fixture limits and exact evidence SHA before claiming its exit.

| Owner | Required case | Passing oracle |
| --- | --- | --- |
| P1 | Remove/reuse focused/captured node; recycle a virtual row | Stale references rejected; no action retargets to the replacement |
| P1/P4/P5, for each implemented surface | Cyclic/deep dock tree; oversized draw/text/row work; non-finite or overflowing geometry | Bounded rejection before allocation/partial mutation; safe state retained |
| P1/P2 | DPI and UI scale varied independently; nested clips; exact edges; zero viewport | Forward/inverse geometry agrees; invisible/clipped regions do not hit |
| P2 | Press -> modal -> release while consumed -> close -> fresh press | One cancellation, released atom absent, exactly one new eligible start |
| P2 | Press during modal -> close -> repeat/other key before release | No synthetic re-arming or unintended gameplay action |
| P2 | Global high-priority binding while text/modal owns input | Only explicitly permitted cross-mode local action; no gameplay bypass |
| P2 | Focus/capture/device loss while UI consumes events; duplicate cleanup | All relevant state owners reconcile; no stuck hold/capture or duplicate action |
| P2/P4 | Empty/multibyte preedit, invalid/out-of-bounds range, commit, focus loss | Bounded valid text state, explicit cursor/range units, exactly-once insertion |
| P3 | Stale generation at physical render entry | No surface acquisition, queue submission, presentation or reconfiguration |
| P3 | Timeout/occluded/outdated/lost and present-then-reconfigure failure | Actual presentation distinguished from skip/recovery; bounded recovery preserved |
| P2/P3 | Resize/scale changes followed by skipped/failed UI frame | No targeting against mismatched displayed/hit-test geometry |
| P3/P4 | Overlapping alpha primitives, nested clips, cache eviction/device rebuild | Painter-order pixels and current resource identity preserved |
| P5/P8 | Delayed drag/result after node, projection or session replacement | Old work rejected/cancelled; no replay into replacement target/session |
| P5 | Interrupted settings write, corrupt/future schema, migration failure | Old or complete new layout survives; safe defaults do not destroy recoverable data |
| P6 | Missing GPU timer or presentation; paired-frame accounting | Unavailable evidence labelled unavailable, never a zero or fabricated PASS |

## 15. Interaction-critical fail-closed behavior

Optional presentation resources may degrade through bounded compatible fallback.

Interaction-critical ownership failure must not default to gameplay pass-through.

Examples:

```text
invalid/inconsistent modal ownership
unknown focus ownership
lost UI capture lifecycle
interaction router failure
corrupt hit-test state affecting ownership
```

For these cases the affected interaction fails closed until coherent state is restored. The client must not invent authoritative outcome or silently reinterpret an unavailable UI ownership decision as a gameplay command.

UI renderer failure may degrade presentation but must not mutate authoritative game/client state; physical resources remain reconstructible caches.

## 16. Corrected Variant B definition

The primary A/B comparison must use one deterministic Variant-B policy. The original broad `fit/crop/or another measured presentation policy` wording is superseded for the primary cell.

### 16.1 Primary B fixture

```text
fixed baseline GameplayFovExtent fixture
preserve world aspect ratio
uniform presentation scaling
center fitted world image
letterbox/pillarbox unused viewport area as necessary
no non-uniform stretching
no crop in primary B cell
```

Crop-based or alternative fixed-FOV presentation may be tested later as a distinct experiment identity. The baseline tile extent remains a fixture, not a final product constant.

### 16.2 World zoom versus fit scale

Record separately:

```text
world_zoom
    semantic player/world presentation zoom used by direct A/B comparison

viewport_fit_scale
    derived uniform scale used by fixed-FOV B to fit the fixed world extent
```

`viewport_fit_scale` must not be reported as changed player world zoom.

### 16.3 Pointer mapping

Variant-B pointer-to-world mapping must invert the same fitted transform used for rendering. Pointer positions in letterbox/pillarbox regions resolve to no world tile. Mapping must remain deterministic across resize/DPI transitions.

## 17. A/B evidence population identity

Every direct A/B cell must record at minimum:

```text
client/code exact SHA
world/presentation fixture digest
synthetic seed/script identity where applicable
baseline fixed-FOV tile extent fixture
world_zoom
presentation family/quality/resource density
HUD fixture identity
OS + target triple
GPU + driver + adapter/backend identity
window physical size
window logical size
platform DPI scale
user UI scale
warm-up procedure
measured frame count
repeat/attempt population identity
event/workload timing identity
```

Any material difference in scene state, workload timing, HUD information fixture, or presentation family invalidates a direct pair unless explicitly treated as another matrix dimension.

No hidden retry until a preferred result appears is permitted.

Record presentation mode/frame cap, measurement units, timer support and A/B run order; use a predeclared paired/counterbalanced schedule where order could bias the comparison. Retain failed/skipped attempts and label their exclusion reasons. Distinguish attempted, submitted and actually presented frames, including resize/recovery intervals. Unsupported GPU timing is `UNAVAILABLE`, not zero, CPU submission time or GPU PASS.

Compute the baseline `UI update + layout + extraction <= 1.0 ms p95` target from the per-frame sum for the same measured frame population. Do not add independently computed component percentiles. Keep CPU work, CPU submission and GPU elapsed time separately named; preserve frame samples/counts and the declared percentile method so the result can be independently recomputed. No numerical performance or fairness result is supplied by this architecture document.

## 18. Fairness information-surface controls

FOV fairness cannot be measured only from world pixels. The A/B experiment must control or explicitly record information available through:

- minimap;
- battle list;
- target acquisition/selection;
- names/health bars;
- alerts/markers;
- any other HUD surface revealing entities/positions outside the rendered viewport.

Direct A/B fixtures should keep these equivalent unless a separate test intentionally measures their interaction with viewport policy.

Responsive FOV still requires assessment of monitor-dependent world awareness, target opportunity, PvE avoidance, and PvP scouting/reaction effects.

## 19. P7/P8 dependency correction

The original unconditional `P7 -> P8` edge is refined. P8 remains feature/dependency-driven.

FOV-sensitive production adapters remain blocked by the applicable P7 decision and later server/relevance proof where responsive FOV is selected.

FOV-independent adapters such as connection/session presentation, chat, player status, or another feature whose owning contract does not depend on FOV may proceed after their own production-safe source projection, allowed intent/command boundary, required UI foundation, path ownership, review, and CI exist.

P7 must not become an artificial blocker for unrelated adapters. This addendum does not authorize bulk P8 work.

For responsive outcome, P7 may record only:

```text
RESPONSIVE_FOV_PREFERRED_PENDING_SERVER_RELEVANCE_PROOF
```

until the separate server/network/fairness spike succeeds. The product terminal `VIEWPORT_RESPONSIVE_FOV_ACCEPTED_AFTER_SERVER_RELEVANCE_PROOF` remains unavailable before that proof.

## 20. Retained-tree decision analysis completion

### Problem

The gameplay HUD requires stable focus, text composition, docking, scrolling/virtualization, drag/drop, modal state, and deterministic automation across long-lived panels. The implementation needs one ownership model rather than panel-specific cross-frame state mechanisms.

### Constraints

- UI is non-authoritative;
- `ui-core` is framework/GPU/platform neutral;
- Windows-first native client requires DPI/IME behavior;
- deterministic tests and stable widget identity are required;
- MMO HUD density and long-lived panels are expected;
- final styling and physical renderer remain replaceable.

### Realistic options

**A — retained production UI tree:** long-lived nodes retain identity; relevant state changes invalidate/update portions; renderer-neutral draw extraction follows interaction/layout resolution.

**B — immediate-mode production HUD:** logical widget descriptions are rebuilt each frame and all required cross-frame interaction state is externalized.

Immediate helpers remain acceptable for debug/engineering overlays that do not define production gameplay UI ownership.

### Trade-offs

Retained UI gives direct stable ownership for focus, composition, docking, drag/drop, automation identity, and invalidation. Costs are lifecycle/invalidation complexity and over-engineering risk.

Immediate UI reduces explicit node lifecycle bookkeeping for simple surfaces but shifts complexity into stable IDs, external text/focus/docking/drag state, large-list behavior, and reconciliation of long-lived MMO panels. Without discipline it can also encourage full-frame rebuild work.

### Risks

Retained: building a general widget framework beyond HUD Slice 01 needs.

Immediate: fragmented state mechanisms effectively recreate retained semantics in less coherent form.

### Recommendation

Use a retained tree for production gameplay UI, restricted to primitives required by representative HUD proof. Permit implementation-local immediate helpers for engineering/debug overlays only.

### Superseding evidence / future impact

Reopen only if measured implementation evidence shows material failure in frame-time, memory, correctness, development complexity, platform text/input, accessibility, or reuse and a realistic alternative preserves authority/testability better. Framework preference alone is insufficient.

### Decision timing

`Must decide now? YES` at the ownership-model level because P1 public state/interaction contracts otherwise risk incompatible assumptions. Exact storage, invalidation algorithm, and helper style remain implementation details.

### 20.1 Timing of the source-driven safety corrections

**Problem:** an implementation can satisfy broad phrases such as stable identity, cancellation, successful rendering and bounded text while still retaining stale holds/targets, accepting mismatched geometry, or counting skipped frames as evidence.

**Constraints:** preserve one app composition owner, inward dependency direction, existing input/renderer lifecycles, non-authoritative UI, bounded work and independently reproducible proof.

**Realistic options:** defer these semantics to each integration PR, or define the minimum cross-owner invariants now while leaving algorithms and numeric tuning to the owning slice.

**Trade-offs and risks:** deferral makes P1/P2/P3 public contracts capable of incompatible interpretations; excessive specification would delay the first vertical slice. The selected correction fixes only the observable safety/evidence contract, not a new framework or broad implementation package.

**Recommendation / Must decide now? YES** for stale-reference rejection, physical cleanup versus semantic admission, geometry coherence, pre-side-effect generation validation, truthful presentation accounting and non-destructive layout recovery. They constrain P1 interfaces, P2 arbitration, P3 submission and P5 persistence. Concrete limits, shader/text libraries, storage/layout algorithms and scheduling strategy remain **NO / implementation-evidence decisions** at their owning slice.

**Future impact:** these invariants avoid retargeted user commands and misleading qualification evidence without freezing a product FOV or public UI API. A later change requires a named counterexample/measurement and replacement contract preserving the same authority, boundedness and observable safety. Mere convenience or a faster unqualified screenshot is not superseding evidence.

## 21. Corrected execution DAG

```text
UI-P0 correction candidate
  |
  v
fresh independent exact-head correction audit
  |
  v
KEEP + protected integration/readback + LIVE workspace ownership
  |
  v
UI-P1 workspace-ready ui-core foundation
  |
  +----------------------+
  |                      |
  v                      v
UI-P2 shell/input/DPI   UI-P3 physical UiDrawList renderer path
  |                      |
  |                      v
  |                    UI-P4 minimum text/assets/theme
  |                      |
  +----------+-----------+
             v
UI-P5 representative HUD on shared production-safe UI seam
             |
             v
UI-P6 physical world+HUD viewport A/B qualification
             |
             v
UI-P7 FOV decision checkpoint

P8 adapters branch from the first stage that supplies their actual
foundation/source dependencies. Only FOV-sensitive P8 is obligatorily
downstream of P7.

P9 high-fidelity polish follows proven foundation and representative
density for affected surfaces and cannot pre-empt unresolved FOV or
gameplay-authority decisions.
```

P2/P3 parallelism is conditional on exact path disjointness and serialized shared workspace/composition surfaces. P4's text/resource work requires P1 interfaces and P3 draw support, not completion of all P2 input work. Integrated text editing and HUD proof at P5 still require both P2 and minimum P4. This is a prerequisite correction, not permission for concurrent writes to shared paths.

## 22. Entry/exit checkpoints

### Correction/P0 exit

Before any runtime work:

- the correction candidate receives fresh independent exact-head review;
- terminal review is `KEEP` with zero unresolved material findings;
- repository exact-head checks are green;
- correction is integrated only through normal Merge Queue;
- real `merge_group` aggregate gate succeeds;
- protected-main readback confirms the integrated correction bytes;
- FOV remains undecided and addons remain deferred;
- no runtime authority is inferred from docs CI.

### P1 entry

- LIVE shared-workspace ownership census clean/serialized;
- exact P1 branch/base/paths allocated;
- package role/edges planned against current `workspace-boundaries.toml`;
- no product panel/runtime/world renderer work bundled into P1.

### P1 exit

- deterministic foundation tests green;
- Cargo metadata matches boundary policy with `--locked`;
- no forbidden framework/GPU/gameplay/synthetic dependency from `ui-core`;
- API sufficient for bounded P2/P3 without speculative public surface;
- bounded lifetime/geometry/resource cases in Section 14.6 pass;
- exact-head repository gate green.

### P2 exit

- one coherent app-owned arbitration route exists;
- required IME/focus/capture lifecycle represented;
- active gameplay actions and physical cleanup reconcile when UI ownership changes;
- blocked presses cannot re-arm implicitly and global mappings cannot bypass UI ownership;
- DPI/resize/minimize deterministic;
- `apps/client` remains sole composition root;
- pre-native gameplay unavailability remains fail-closed.

### P3/P4 exit

- real physical `wgpu` draw evidence exists for required primitives/text;
- physical resources renderer-owned/reconstructible;
- semantic UI state survives/reconciles applicable renderer recovery;
- stale generations rejected before physical effects;
- presentation, skip and recovery accounting is truthful;
- geometry/painter-order/cache boundary tests pass;
- UI/world measurement boundaries observable.

### P5 exit

- HUD Slice 01 uses one production-safe semantic seam;
- synthetic adapters are one-way test dependencies;
- no production dependency reaches synthetic-only packages;
- required interaction/docking/virtualization/text behavior demonstrated;
- stale interactions/results and interrupted/corrupt layout persistence are safely handled.

### P6 exit

- world+HUD physically rendered in one named qualification host/cell;
- A/B uses frozen experiment identity and deterministic B policy;
- no unknown world state fabricated;
- performance/fairness evidence independently reviewable with truthful frame population and percentile accounting;
- implementation author has not selected final product FOV unilaterally.

### P7 exit

- fixed outcome may become accepted if the reviewed evidence supports it;
- responsive outcome is only `PREFERRED_PENDING_SERVER_RELEVANCE_PROOF` until separate server/network/fairness evidence succeeds;
- final responsive product acceptance occurs only after that proof and independent review;
- losing experimental path is removed/quarantined after the applicable decision;
- regression tests bind selected behavior.

## 23. Rollback and ownership

Every runtime slice remains independently revertible unless a later separately accepted contract explicitly introduces cross-domain rollback coupling.

Presentation-only rollback does not change server authority, protocol semantics, or gameplay truth.

P5 local persistence must be bounded and failure-atomic at the app-owned storage boundary. Validate schema, size/depth, unique panel identity, acyclic topology, finite scales/ratios and geometry before replacing active layout. Exact format and migration code remain implementation choices.

An interrupted write must leave a complete old or complete new saved layout, not a truncated replacement. An unsupported newer schema or failed migration may select in-memory safe defaults, but must not silently overwrite the recoverable original merely because an older build started. Define bounded recovery/backup retention and explicit replacement behavior. Keep filesystem work outside `ui-core` and outside the frame-critical mutation path; test denial, corrupt data, interruption and rollback without persisting transient interaction or gameplay state.

Shared path leases are execution authority, not architectural ownership. Historical #351/#356 Cargo custody does not permanently reserve the workspace, and no future UI worker may infer shared-path authority from this document.

## 24. Fresh review requirement

Because this addendum materially corrects merged #549 architecture, the original audit cannot qualify this new text.

Before integration, a fresh reviewer must bind the result to:

```text
repository: Oteryn/Oteryn-Game
correction_pr: <live PR containing this addendum>
base: main
exact_head: <live correction head>
```

Required terminal classification:

```text
KEEP
FIX
NEEDS_DECISION
```

A self-check by the author is useful but does not substitute for repository-required independent review.

Any material content change invalidates the previous correction review and requires fresh exact-head review before Merge Queue submission.

## 25. Terminal state

Until **all** of the following are true:

1. a fresh independent exact-head review returns `KEEP` with zero unresolved material findings;
2. the correction is integrated through the normal Merge Queue with the required `merge_group` aggregate gate successful;
3. protected-main readback confirms the integrated correction bytes;

the state remains:

```text
UI_ARCHITECTURE_CORRECTION = IN_REVIEW
UI_P1 = BLOCKED
RESPONSIVE_FOV_POLICY = UNDECIDED
FIXED_FOV_POLICY = UNDECIDED
ADDON_PLATFORM = DEFERRED_FUTURE_CONCEPT
```

After those three correction conditions are satisfied, the architecture blocker is removed. P1 still requires a fresh shared-workspace ownership census and explicit allocation before mutation.
