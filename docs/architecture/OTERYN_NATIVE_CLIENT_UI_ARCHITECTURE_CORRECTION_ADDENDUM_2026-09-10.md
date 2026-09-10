# Oteryn native client UI architecture correction addendum

- Date: 2026-09-10
- Repository: `Oteryn/Oteryn-Game`
- Corrects/supersedes bounded clauses from PR `#549`
- Audit source: `docs/architecture/reviews/OTERYN_NATIVE_CLIENT_UI_PR549_INDEPENDENT_AUDIT_AND_REQUIRED_CORRECTIONS_2026-09-10.md`
- Audited PR head: `83314bcbf2978b20f07ca539dd697900dcdb410f`
- Protected baseline containing the audited documents: `main@7144c0b9ec8691e481df058c85d890ac88d32461`
- Status: **CORRECTION CANDIDATE / FRESH EXACT-HEAD REVIEW REQUIRED**
- Runtime implementation authority: **NONE**
- Production/client/server/protocol/content mutation authority: **NONE**
- Final FOV policy: **UNDECIDED / EVIDENCE-GATED**
- Addon/mod/plugin architecture: **DEFERRED / FUTURE CONCEPT / OUT OF CURRENT IMPLEMENTATION SCOPE**

## 1. Purpose and supersession boundary

This addendum applies the independent `FIX` findings recorded after PR #549. It does not reopen the native Rust + `wgpu` direction, the ALPHA-CLIENT authority boundary, the Visual World Slice authority model, or the deliberate deferral of final FOV and addon-platform decisions.

When this addendum conflicts with the three documents delivered by PR #549, this addendum supersedes only the exact topics named below:

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
- retained-tree decision analysis completeness.

All unaffected requirements in the original UI baseline, implementation plan, and viewport experiment remain binding candidates/accepted content according to their current repository status.

This addendum grants no worker allocation and no runtime authority. Before UI-P1 starts, this correction must be protected-integrated, read back from protected `main`, pass its required fresh exact-head architecture review, and satisfy the shared-workspace ownership entry gate defined below.

## 2. Decisions preserved unchanged

The following findings from PR #549 are explicitly retained.

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
renderer    -> ui-core       # to consume renderer-neutral UiDrawList/types when needed

ui-core     -X-> renderer
ui-core     -X-> wgpu
ui-core     -X-> winit
ui-core     -X-> synthetic-only packages
```

The exact minimal dependencies of `ui-core` remain implementation-local but must be declared in the workspace boundary registry and independently reviewed at P1.

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

### 3.1 P1 semantic implementation scope

The first implementation remains a single framework-neutral UI foundation, but the mechanically complete workspace surfaces are:

```text
NEW/MODIFY: crates/ui-core/**
MODIFY:     Cargo.toml
MODIFY:     workspace-boundaries.toml
MODIFY:     Cargo.lock                    # only when produced by the exact package/dependency delta
```

No server/gameplay/protocol/production-networking/client-domain semantic/art/final-HUD work is added to P1.

### 3.2 Required workspace declaration

P1 must register the selected package name, expected to be `oteryn-ui-core` unless the exact implementation review chooses another non-conflicting name, in the repository's enforced workspace policy.

The package must receive exactly one release role. For the native product foundation it is expected to be **production**, because both the production native client and production renderer may consume it. A different role requires explicit architecture justification and production-closure proof.

The policy must declare exact internal dependency edges for the package and for existing packages newly depending on it.

The expected minimal dependency direction is:

```text
ui-core -> only framework-neutral production foundations proven necessary
apps/client -> ui-core
renderer -> ui-core       # once P3 needs UiDrawList consumption
```

At P1, do not predeclare speculative renderer/app edges that do not yet exist merely for future convenience. The registry must match actual Cargo metadata at each exact candidate.

### 3.3 P1 required mechanical proof

At minimum:

```text
cargo +1.94.0 metadata --locked --format-version 1
cargo +1.94.0 fmt --all --check
cargo +1.94.0 run --locked -p oteryn-architecture-check -- workspace .
cargo +1.94.0 test --locked -p <ui-core-package>
cargo +1.94.0 clippy --locked -p <ui-core-package> --all-targets -- -D warnings
repository-selected exact-head gate
```

The exact repository CI selected by current protected policy remains authoritative. No validator or protection weakening is permitted to make P1 pass.

### 3.4 P1 shared-workspace entry gate

A successful architecture correction does not itself grant a P1 writer the root workspace.

Immediately before allocation/admission, the coordinator must perform a fresh live ownership census for:

```text
Cargo.toml
Cargo.lock
workspace-boundaries.toml
```

P1 may start only when:

```text
P1_WORKSPACE_ENTRY_READY =
    correction protected-integrated and read back
AND fresh exact-head correction audit is clean
AND no conflicting active writer owns root Cargo/Cargo.lock
AND workspace-boundaries.toml ownership is explicitly allocated
AND the P1 branch/base/owned paths are exactly recorded
```

At the time of the original audit, Issue #351 / Draft PR #356 held the serialized root Cargo lease. That fact is historical evidence, not a permanent dependency. The start gate must use LIVE ownership, not assume #351 still blocks or has released it.

## 4. UI-P1 size and sub-slicing

P1 must remain a minimal coherent foundation rather than a requirement to land every generic UI facility in one oversized PR.

The semantic P1 programme may be split into bounded serial sub-slices when independent reviewability improves, provided all sub-slices remain within the same frozen foundation architecture and do not create speculative crates.

Recommended minimum-to-HUD order inside P1 if splitting is needed:

```text
P1a: node identity/tree + geometry + deterministic layout + draw primitives
P1b: focus/modal/capture + interaction-state transitions
P1c: minimal docking/virtualization/text interfaces/diagnostics required by P2-P5
```

This is not a mandatory three-PR topology. A single P1 PR remains valid if its API and review surface stay bounded.

Do not implement broad market/quest/social widget frameworks, production panel models, theme marketplace abstractions, or general extension APIs in P1.

## 5. Corrected input architecture

The original diagram that routes all UI input through `input-actions` is superseded because current contracts do not make `input-actions` a general UI event bus.

### 5.1 Ownership

Keep these responsibilities distinct:

```text
input-platform
    platform/winit normalization and lifecycle facts

input-actions
    framework-neutral physical binding atoms, semantic action contexts,
    held-action lifecycle and gameplay/action routing contracts

ui-core
    framework-neutral UI hit/focus/modal/drag/text-interaction state

apps/client
    application-level arbitration and composition between UI and gameplay/action routes
```

### 5.2 Correct event flow

The application-level flow is:

```text
winit / OS event
      |
      v
input-platform normalization
      |
      v
NormalizedInputEvent / bounded UI-relevant normalized primitives
      |
      v
apps/client input arbitration
      |
      +--> ui-core interaction/text/focus route
      |       |
      |       `--> UiIntent / consume / reserve / local UI transition
      |
      `--> input-actions semantic gameplay/action route
              only when the current UI ownership state permits it
```

This must not duplicate physical normalization in `apps/client` or `ui-core`.

### 5.3 Deterministic arbitration rule

For each normalized input event, the application must deterministically decide whether UI owns/reserves the event before an unintended gameplay action can be produced.

At minimum:

- active text focus reserves text-producing keyboard behavior from gameplay;
- active modal scope reserves interaction outside the permitted modal route;
- active UI pointer capture keeps receiving the matching pointer lifecycle until completion/cancel;
- UI drag capture and OS pointer capture are distinct facts;
- absence/failure of interaction-critical arbitration must not default to permissive gameplay pass-through.

A single physical event must not produce both a UI action and an unintended gameplay command.

## 6. Full IME correction

P2 must support the minimum bounded IME semantics needed for correct Windows text entry.

The existing `Ime::Preedit(text, cursor)` information cannot be discarded by the final P2 implementation.

Required framework-neutral concepts may use different exact Rust names, but must represent:

```text
IME enabled/disabled
preedit text update
preedit selection/cursor/range when supplied
commit text
composition cancellation/clear
focus-loss cancellation/reset
```

Requirements:

- IME preedit is local presentation state and is not a gameplay command;
- committed text is emitted exactly once;
- keyboard `KeyEvent::text` and `Ime::Commit` must not double-insert the same logical text;
- disabling IME or losing focus clears/cancels transient composition safely;
- composition text and ranges are bounded under the same untrusted-text/resource principles as other UI text;
- tests must cover Polish diacritics and at least one composition sequence that exercises preedit before commit.

`input-platform/**` may be modified in P2 only for demonstrated missing platform normalization primitives. `input-actions/**` may be modified only if a demonstrated semantic action primitive is missing. Do not move widget focus/composition ownership into either crate.

## 7. Active gameplay action cancellation on UI ownership changes

Input eligibility changes must reconcile already-active semantic actions, not only future key presses.

When a UI transition makes an existing gameplay/action route no longer eligible, the app/input layer must deterministically end or cancel the active semantic action according to the existing `input-actions` lifecycle contract.

Required negative cases include:

```text
gameplay key held -> text field gains focus
gameplay action active -> modal opens
pointer action active -> UI drag/modal capture takes ownership
application focus lost while action is held
OS pointer capture lost
device reset/loss while UI interaction is active
IME composition starts while a text-producing gameplay binding is otherwise eligible
```

No stuck movement/action state may survive these transitions.

The exact distinction between `Ended` and `Cancelled` follows the owning `input-actions` contract and must be tested rather than redefined by UI widgets.

## 8. Client composition correction for P2

P2 must reconcile existing pre-native client composition instead of adding another parallel owner.

The production application remains `apps/client`.

Required ownership after P2:

```text
apps/client
    owns composition and ordering of runtime, input arbitration,
    UI semantic state, window lifecycle, renderer and product view adapters

client-runtime
    owns async runtime/cancellation lifecycle only

input-platform
    owns platform normalization only

input-actions
    owns semantic action contracts/router only

ui-core
    owns framework-neutral UI semantics only

renderer
    owns physical GPU/surface/resource state only
```

`ClientBootstrap` and the interactive Windows shell may be refactored/combined internally, but P2 must not leave two contradictory production ownership paths or introduce a third runtime/input/renderer composition root.

The pre-native gameplay entry fail-closed behavior must remain unchanged unless a separately authorized gameplay/client-runtime programme changes it.

## 9. P2/P3 concurrency correction

P2 and P3 may still proceed semantically in parallel after stable P1 contracts, but only if their actual writable paths are disjoint.

Shared surfaces that require explicit serialization include as applicable:

```text
Cargo.toml
Cargo.lock
workspace-boundaries.toml
apps/client/Cargo.toml
apps/client/src/ui/** composition glue
public ui-core draw/input contract files
```

No two active writers receive overlapping ownership merely because the high-level DAG draws P2 and P3 as parallel branches.

The coordinator may choose either:

- serial P2 then P3;
- serial P3 then P2; or
- parallel workers with a separately serialized shared-path prerequisite/lease.

The choice is execution planning, not a new architecture decision, as long as the ownership invariants stay unchanged.

## 10. P3 renderer seam

P3 remains responsible for a bounded physical UI draw path.

Requirements:

```text
UiDrawList -> renderer-owned extraction/batching/resources -> wgpu submission
```

- `ui-core` exposes renderer-neutral primitives only;
- renderer owns pipelines, bind groups, buffers, atlases/caches and surface/device resources;
- renderer failure/loss does not mutate semantic UI/game state;
- UI and world timing remain logically separable;
- the first physical fixture proves actual rendered pixels/primitives, not only successful surface creation.

A physical pass may later be combined for optimization only if semantic separation and observability remain intact.

## 11. P4 text/resource boundary

P4 may select a reversible shaping/rasterization implementation behind the already defined semantic boundary.

Required behavior:

- Unicode and Polish diacritics measure and render correctly;
- glyph/fallback caches are reconstructible after physical renderer loss;
- untrusted text has bounded size/work behavior;
- app/domain/panel logic owns no GPU glyph resources;
- exact library choice remains implementation evidence unless a later public/stable contract requires it.

P4 must not turn one chosen font library into a gameplay or protocol dependency.

## 12. P5 shared HUD composition and synthetic boundary

P5 must define one reusable, production-safe HUD composition seam before the synthetic qualification host can claim representative UI coverage.

### 12.1 Dependency rule

Allowed direction:

```text
production-safe UI semantics/composition API
          ^
          |
synthetic qualification adapters/harness
```

Forbidden direction:

```text
apps/client / ui-core / renderer
          -> client-domain synthetic-only
          -> client-simulation synthetic-only
          -> synthetic-assets
          -> synthetic-client-harness
```

Synthetic fixtures may adapt synthetic state into the same presentation-safe view-model/input contracts consumed by the HUD, but production packages must never depend on synthetic-only packages.

### 12.2 HUD host contract

Before P5 is considered physically demonstrated, the selected qualification host must be able to instantiate the same UI foundation/HUD semantics and drive an actual physical renderer path, not merely calculate `SurfaceState` transitions or print semantic state.

The host may be:

- an explicitly test-only mode/profile of the native client;
- a dedicated native qualification binary with production-safe dependencies;
- another bounded host accepted during implementation review.

The host must not become a second production client composition root.

The exact host executable name is deferred until the bounded P5 implementation package, but its ownership/dependency direction must satisfy this section.

## 13. P6 physical world + HUD qualification gate

P6 requires a real physical world-render seam before it can claim an A/B result representative of the intended product presentation.

Current isolated World+VFX experiment evidence may inform or supply a later promotion/reuse plan, but UI work must not silently copy/migrate broad experiment code under an unspecified `renderer/world presentation paths` allowance.

Before P6 admission, one bounded allocation must establish how the physical qualification host obtains:

```text
representative RenderSnapshot / PresentationEvents / EnvironmentState
        -> physical world presentation
        + representative HUD/UI draw path
        -> one actual wgpu frame composition
```

The source may be a production-ready world renderer by then or an explicitly bounded qualification-only bridge. Either way:

- provenance/evidence classification must be explicit;
- synthetic world inputs remain labelled synthetic where applicable;
- production client must not gain synthetic-only dependencies;
- no server/gameplay authority moves into the renderer/UI;
- reuse/promotion from `experiments/world-vfx-prototype` requires an explicit path allocation and review.

P1-P4 are not blocked on full World+VFX completion. This is a P5/P6 physical-proof gate.

## 14. Stage-specific test/evidence correction

General green CI is necessary but not sufficient for physical client/UI proof.

### 14.1 P1

Required proof:

- deterministic tree/layout/geometry/draw ordering tests;
- focus/modal/capture state-machine tests for any included interaction core;
- workspace metadata/boundary proof;
- package strict Clippy/tests;
- repository exact-head gate.

No physical GPU proof is required merely to establish pure `ui-core` semantics.

### 14.2 P2

Required deterministic/client tests include:

- resize small->large->small;
- scale-factor transition;
- zero/minimized and restore;
- app focus loss/regain;
- OS capture gain/loss;
- UI pointer/drag capture transition;
- text focus and modal reservation;
- IME enable/preedit/commit/cancel/disable;
- active semantic action cancellation on UI ownership changes;
- gameplay fail-closed pre-native state remains unchanged.

Where platform behavior cannot be simulated faithfully, retain a named Windows-native qualification case instead of replacing it with a pure unit claim.

### 14.3 P3/P4

Require actual physical `wgpu` evidence for:

- quad/border/image/text primitives;
- clip/scissor behavior;
- z/order semantics;
- resize/reconfigure;
- text/glyph cache behavior;
- surface/device reconstruction where the renderer exposes the applicable recovery boundary;
- independent UI timing/counters.

### 14.4 P5/P6

Require a named Windows native/hardware/scene cell and Tier-2-equivalent interaction/render evidence where applicable to user-observable UI behavior.

A console/headless synthetic harness alone cannot be reported as native-client UI/render proof.

### 14.5 Release stage

Tier 3 production-binary evidence remains a later release/product gate. It does not block creation of `ui-core` or the first qualification HUD.

## 15. Interaction-critical fail-closed behavior

The original general failure semantics are refined as follows.

### 15.1 Decorative degradation

For optional presentation resources:

```text
missing optional asset
unsupported optional effect
non-critical cache pressure
```

use a bounded compatible fallback/degraded presentation where available.

### 15.2 Interaction-critical failure

For state required to arbitrate player input safely:

```text
invalid/inconsistent modal ownership
unknown focus ownership
lost UI capture lifecycle
interaction router failure
corrupt hit-test state affecting ownership
```

do **not** default to permissive gameplay pass-through.

The affected interaction fails closed until a coherent safe state is restored. The client must not invent authoritative gameplay outcome and must not silently convert an unavailable UI ownership decision into a gameplay command.

### 15.3 Renderer/UI failure

A UI renderer failure may make presentation degraded/unavailable, but it must not mutate authoritative game/client state. Physical resources are reconstructible caches.

If a shipped interaction-critical control becomes unavailable, the product must surface a bounded degraded/error state appropriate to that implementation rather than pretending the action succeeded.

## 16. Corrected Variant B definition

The viewport experiment requires one deterministic Variant-B policy per comparison population. The broad phrase `fit/crop within safe constraints, or another measured presentation policy` is superseded for the primary A/B cell.

### 16.1 Primary B fixture

The primary B comparison uses:

```text
fixed baseline GameplayFovExtent fixture
preserve world aspect ratio
uniform presentation scaling
center the fitted world image
letterbox/pillarbox unused viewport area as necessary
no non-uniform stretching
no crop in the primary comparison cell
```

A separate crop-based or alternative fixed-FOV presentation may be evaluated later as a distinct experiment cell, but it must not be mixed into the same `Variant B` identity.

The baseline tile extent remains an experiment fixture, not a final product constant.

### 16.2 World zoom versus fit scale

Direct A/B comparison must record two separate concepts:

```text
world_zoom
    semantic player/world presentation zoom used to define the comparison

viewport_fit_scale
    derived uniform physical scale used by fixed-FOV B to fit the
    fixed world extent into the current viewport
```

`viewport_fit_scale` must never be reported as a changed player world zoom.

### 16.3 Pointer mapping

For Variant B, pointer-to-world mapping must invert the same fitted transform used by presentation.

Pointer positions in letterbox/pillarbox regions do not resolve to a world tile.

The mapping must be deterministic across resize/DPI transitions and covered by tests.

## 17. A/B evidence population identity

Every direct A/B cell must bind enough identity to prevent accidental comparison of different workloads.

At minimum record:

```text
client/code exact SHA
world/presentation fixture digest
synthetic seed/script identity where applicable
baseline fixed-FOV tile extent fixture
world_zoom
presentation family/quality/resource density
HUD fixture identity
OS + target triple
GPU + driver + relevant adapter/backend identity
window physical size
window logical size
platform DPI scale
user UI scale
warm-up procedure
measured frame count
repeat/attempt population identity
event/workload timing identity
```

Any material difference in scene state, workload timing, HUD information fixture or presentation family invalidates a direct A/B pair unless explicitly treated as a separate matrix dimension.

No hidden retry until a preferred result appears is permitted.

## 18. Fairness information-surface controls

FOV fairness cannot be measured only from rendered world pixels.

The A/B experiment must control or explicitly record information available through:

- minimap;
- battle list;
- target acquisition/selection;
- names/health bars;
- alerts/markers;
- any other synthetic or production HUD surface that can reveal entities/positions outside the directly rendered world viewport.

The direct A/B fixture should keep those information surfaces equivalent unless the purpose of a separate test is specifically to measure their interaction with viewport policy.

Responsive FOV cannot be accepted as fair merely because UI panels expose equivalent information; the review must still assess monitor-dependent world awareness, target opportunity, PvE avoidance and PvP scouting/reaction effects.

## 19. P7/P8 dependency correction

The original visual DAG's unconditional `P7 -> P8` edge is refined.

P8 remains feature/dependency-driven.

### 19.1 FOV-sensitive P8 work

Any production adapter whose semantics depend on selected world visibility/relevance/viewport behavior remains blocked by the applicable P7 decision and later server/relevance proof where responsive FOV is selected.

### 19.2 FOV-independent P8 work

Production-safe view-model adapters such as connection/session presentation, chat, player status, or another feature whose owning contract does not depend on FOV may proceed after:

- its production-safe source projection exists;
- its allowed intents/command boundary are accepted;
- UI foundation/input/render prerequisites for that feature are present;
- shared path ownership is available;
- normal exact-head review/CI requirements are satisfied.

P7 must not become an artificial blocker for unrelated adapters.

This does not authorize bulk P8 work or production gameplay implementation from this documentation addendum.

## 20. Retained-tree architecture decision analysis completion

The retained-tree direction remains recommended, but its decision record is completed here according to `ARCHITECTURE_DECISION_DISCIPLINE.md`.

### Problem

The gameplay HUD requires stable focus, text composition, docking, scroll/virtualization, drag/drop, modal state and deterministic automation across many long-lived panels. The implementation needs a state model that does not force these concerns to be reconstructed independently by each panel every frame.

### Constraints

- UI is non-authoritative;
- `ui-core` must be framework/GPU/platform neutral;
- Windows-first native client with DPI/IME requirements;
- deterministic tests and stable widget identity are required;
- representative MMO HUD density and long-lived panels are expected;
- final styling and physical renderer implementation remain replaceable.

### Realistic options

**Option A — retained UI tree**

Long-lived nodes/widgets retain stable identity; state changes invalidate/update only relevant portions; renderer-neutral draw extraction occurs after layout/interaction resolution.

**Option B — immediate-mode production HUD**

Rebuild the logical widget description each frame and retain only externalized interaction state needed across frames.

A hybrid remains possible for engineering/debug overlays, but production gameplay UI must have one coherent ownership model.

### Trade-offs

Retained UI provides direct stable ownership for focus, composition, docking, drag/drop and deterministic node identity, and can avoid full-tree work through invalidation. Its cost is lifecycle/invalidation complexity and risk of a heavier internal framework.

Immediate-mode UI can reduce explicit node lifecycle bookkeeping and make simple engineering surfaces concise. For this product it shifts complexity into stable IDs, external focus/text/docking state, large-list behavior and reconciliation of long-lived MMO panels. It can also encourage rebuilding more presentation state each frame unless carefully engineered.

### Risks

Retained risk: over-engineering a general widget framework before HUD Slice 01 proves the need.

Immediate-mode risk: hidden state machinery emerges separately for text/IME, docking, drag/drop and automation, effectively recreating retained semantics in fragmented form.

### Recommendation

Use the retained tree for the production gameplay UI foundation, restricted to primitives required by the representative HUD. Permit implementation-local immediate helpers for debug/engineering overlays that do not define production gameplay UI ownership.

### Future impact and superseding evidence

Reopen this choice only if measured implementation evidence shows the retained foundation materially fails frame-time, memory, correctness, development complexity, platform text/input requirements, accessibility, or future client reuse in a way a realistic alternative demonstrably solves better while preserving authority and testability.

Framework fashion or preference alone is not sufficient superseding evidence.

### Decision timing

`Must decide now? YES` at the ownership-model level because P1 public state/interaction contracts otherwise risk incompatible assumptions. Exact internal storage, invalidation algorithm and helper style remain implementation details.

## 21. Corrected execution DAG

The high-level programme becomes:

```text
UI-P0 / correction exact-head audit
  |
  v
UI-P1 workspace-ready ui-core foundation
  |
  +----------------------+
  |                      |
  v                      v
UI-P2 shell/input/DPI   UI-P3 physical UiDrawList renderer path
  |                      |
  +----------+-----------+
             v
UI-P4 minimum text/assets/theme
             |
             v
UI-P5 representative HUD on shared production-safe UI seam
             |
             v
UI-P6 physical world+HUD viewport A/B qualification
             |
             v
UI-P7 FOV decision checkpoint

Independent production view-model adapters (P8) may branch from the
first stage that provides their required foundation/source contracts.
Only FOV-sensitive P8 work is obligatorily downstream of P7.

UI-P9 high-fidelity polish follows proven foundation and representative
density for the affected surfaces; it must not pre-empt unresolved FOV
or gameplay-authority decisions.
```

P2 and P3 parallelism is conditional on exact path disjointness and serialized shared workspace/composition surfaces.

## 22. Corrected entry/exit checkpoints

### Correction/P0 exit

Before runtime work:

- this addendum and the audit record are integrated to protected `main`;
- fresh exact-head review returns no unresolved `FIX`/`NEEDS_DECISION` for the corrected architecture;
- final FOV remains undecided;
- addons remain deferred;
- no runtime authority is inferred from documentation CI.

### P1 entry

- LIVE shared-workspace ownership census clean/serialized;
- exact P1 branch/base/paths allocated;
- package role/edges planned against current `workspace-boundaries.toml`;
- no product panel/runtime/world renderer work bundled into P1.

### P1 exit

- deterministic foundation tests green;
- actual Cargo metadata matches boundary policy with `--locked`;
- no forbidden framework/GPU/gameplay/synthetic dependency from `ui-core`;
- API is sufficient for bounded P2/P3 without speculative public surface;
- exact-head repository gate passes.

### P2 exit

- one coherent app-owned input arbitration route exists;
- full required text/IME/focus/capture lifecycle is represented;
- active gameplay actions reconcile on UI ownership changes;
- DPI/resize/minimize behavior is deterministic;
- `apps/client` remains sole composition root;
- pre-native gameplay unavailability remains fail-closed.

### P3/P4 exit

- real physical `wgpu` draw evidence exists for the required primitives/text;
- physical resources remain renderer-owned/reconstructible;
- semantic UI state survives/reconciles renderer recovery as applicable;
- UI/world measurement boundaries remain observable.

### P5 exit

- HUD Slice 01 uses one production-safe UI semantic seam;
- synthetic adapters remain one-way test dependencies;
- no production dependency reaches synthetic-only packages;
- required interaction/docking/virtualization/text behavior is demonstrated.

### P6 exit

- world+HUD is physically rendered in one named qualification host/cell;
- A/B comparison uses the frozen experiment identity and deterministic B policy;
- no unknown world state is fabricated;
- performance and fairness evidence are independently reviewable;
- implementation author has not selected final product FOV unilaterally.

### P7 exit

- explicit reviewed FOV decision exists;
- responsive outcome has separately authorized server relevance/network/fairness evidence before product acceptance;
- losing experimental path is removed or quarantined according to the decision;
- regression tests bind the selected behavior.

## 23. Rollback and ownership

Every runtime slice must remain independently revertible unless a later separately accepted contract explicitly introduces a cross-domain dependency requiring coordinated rollback.

Presentation-only correction/rollback must not change server authority, protocol semantics or gameplay truth.

Shared path leases are execution authority, not architectural ownership. Historical #351/#356 Cargo custody does not permanently reserve the workspace, and a future UI worker cannot infer shared-path authority from this document.

## 24. Fresh review requirement

Because this addendum materially corrects the merged #549 architecture plan, the original audit classification cannot by itself qualify this new text.

Before P1 begins, a fresh reviewer must bind its result to:

```text
repository: Oteryn/Oteryn-Game
correction_pr: <live PR number containing this addendum>
base: main
exact_head: <live correction head>
```

Required terminal classification:

```text
KEEP
FIX
NEEDS_DECISION
```

A self-check by the author is useful but does not substitute for any repository-required independent review.

Any material content change after a qualifying review invalidates that review target and requires fresh exact-head readback.

## 25. Terminal state

Until the correction is protected-integrated, independently reviewed on its exact final head, and the LIVE shared-workspace gate is available:

```text
UI_ARCHITECTURE_CORRECTION = IN_REVIEW
UI_P1 = BLOCKED
RESPONSIVE_FOV_POLICY = UNDECIDED
FIXED_FOV_POLICY = UNDECIDED
ADDON_PLATFORM = DEFERRED_FUTURE_CONCEPT
```

After protected integration plus a clean exact-head correction audit, the architecture blocker is removed, but P1 still requires fresh workspace ownership allocation before mutation.
