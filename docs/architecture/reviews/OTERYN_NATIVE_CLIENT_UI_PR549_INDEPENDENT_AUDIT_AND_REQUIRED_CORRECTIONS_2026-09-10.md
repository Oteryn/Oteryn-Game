# Oteryn native client UI PR #549 independent audit and required corrections

- Date: 2026-09-10
- Repository: `Oteryn/Oteryn-Game`
- Audited pull request: `#549`
- Audited base: `main@149e1e5cc3dd09b4bbf53e9213b92901233da210`
- Audited exact head: `83314bcbf2978b20f07ca539dd697900dcdb410f`
- Post-audit protected main containing the same three document blobs: `7144c0b9ec8691e481df058c85d890ac88d32461`
- Audit classification: **FIX**
- Runtime/client/server/protocol/content implementation authority: **NONE**
- UI-P1 execution status: **BLOCKED_PENDING_DOCUMENT_CORRECTION_AND_SHARED_WORKSPACE_LEASE**
- Final FOV policy: **UNDECIDED / EVIDENCE-GATED**
- Addon/mod/plugin architecture: **DEFERRED / FUTURE CONCEPT / OUT OF CURRENT IMPLEMENTATION SCOPE**

## 1. Purpose

Persist the complete independent audit result for the native UI architecture and implementation programme delivered by PR #549.

PR #549 was merged while the independent audit was still in progress. The merge did not change the audited document content: the three document blobs on protected `main@7144c0b9ec8691e481df058c85d890ac88d32461` match the blobs reviewed at exact PR head `83314bcbf2978b20f07ca539dd697900dcdb410f`.

This record therefore preserves the `FIX` result and defines the smallest corrections required before any `UI-P1` runtime allocation may begin. It does not itself implement those corrections, create `ui-core`, change Cargo/workspace state, activate a renderer/UI path, select a FOV policy, or authorize a merge.

## 2. Sources verified by the audit

The audit read and cross-checked:

- `docs/architecture/OTERYN_NATIVE_CLIENT_UI_ARCHITECTURE_BASELINE_2026-09-10.md`;
- `docs/architecture/OTERYN_NATIVE_CLIENT_UI_ARCHITECTURE_IMPLEMENTATION_PLAN_2026-09-10.md`;
- `docs/architecture/OTERYN_NATIVE_CLIENT_VIEWPORT_AB_EXPERIMENT_PLAN_2026-09-10.md`;
- root `AGENTS.md`;
- `docs/agents/META_AGENT_POLICY_BINDING.json` and bound `OTERYN_ORGANIZATION_AGENT_POLICY 3.1.0` at `Oteryn/Oteryn@3b39e0be05aef008f1bd442821daefa898a201dd`;
- `docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md`;
- accepted ALPHA-CLIENT and QA-E2E architecture;
- graphics/VFX and Visual World Slice architecture/evidence boundaries;
- current implementations and boundaries of `apps/client`, `client-runtime`, `client-domain`, `client-simulation`, `input-actions`, `input-platform`, `renderer`, and `tools/synthetic-client-harness`;
- current workspace validator and CI behavior;
- LIVE shared-workspace ownership around Issue #351 / Draft PR #356;
- legacy viewport reference at `blakinio/otclient@53646cfa1957ce75f18547424a9e9177c4ad8cc2`.

## 3. Findings that are correct and should remain unchanged

The following parts of PR #549 passed the independent audit and should not be reopened merely while repairing the blockers below.

### 3.1 Authority and composition

Keep:

- `apps/client` as the production native-client composition root;
- `client-runtime` narrow and application-owned;
- UI as non-authoritative presentation/interaction state;
- typed UI intents flowing through normal application/game command validation;
- renderer ownership limited to physical GPU/surface/resource concerns;
- world and UI rendering logically separate and independently measurable;
- GPU-local handles/resources excluded from UI/domain authority state;
- `client-domain` and `client-simulation` synthetic-only until a separately accepted production projection exists.

### 3.2 Input ownership

Keep the existing split:

```text
input-platform -> physical/platform normalization
input-actions  -> semantic action contracts/router
ui-core        -> UI-local focus/capture/modal/drag interaction state
apps/client    -> application-level arbitration/composition
```

Do not create a second physical-key vocabulary or panel-local global input router.

### 3.3 FOV status

The final FOV policy remains correctly:

```text
RESPONSIVE_FOV_POLICY = UNDECIDED
FIXED_FOV_POLICY = UNDECIDED
```

Variant A may be implemented first only as client/synthetic qualification behavior. Responsive FOV must not silently change server/protocol/relevance semantics. If client evidence favors responsive FOV, a later separately authorized server/relevance/fairness spike remains mandatory before product acceptance.

### 3.4 Addons/mods/plugins

The current UI programme must continue to treat addon/mod/plugin/community customization as:

```text
DEFERRED / FUTURE CONCEPT / OUT OF CURRENT IMPLEMENTATION SCOPE
```

No current UI slice may add a scripting VM, WASM/Lua choice, addon runtime, package/manifest format, third-party widget API, capability/permission system, hot-reload system, marketplace, signing flow, or distribution system solely for hypothetical future extensions.

## 4. Blocking corrections required before UI-P1

### BLOCKER A — P1 workspace scope is incomplete

The current P1 slice names only:

```text
Cargo.toml
crates/ui-core/**
```

That is not sufficient for the repository's enforced workspace contract.

The architecture validator requires every workspace package to have matching package/path registration, exactly one release role, declared internal dependency edges, an acyclic dependency graph, and a valid production closure. Cargo metadata is executed with `--locked`.

The corrected P1 owned/serialized surfaces must therefore account for:

```text
crates/ui-core/**
Cargo.toml
workspace-boundaries.toml
Cargo.lock                    # when changed by the exact workspace/dependency delta
```

The correction must explicitly state:

- exact `oteryn-ui-core` package identity or the selected final package name;
- production/synthetic/test/tool role;
- exact internal dependency edges permitted at P1;
- production-closure expectation;
- `cargo metadata --locked` and architecture-check proof.

No workspace-policy weakening is permitted to make P1 pass.

### BLOCKER B — active serialized root Cargo ownership conflicts with immediate P1 start

LIVE Issue #351 / Draft PR #356 currently owns the serialized root `Cargo.toml` / `Cargo.lock` lease for the SQLx/rustls/Tokio durability work.

Repository coordination policy forbids two active writers on root/app Cargo manifests, `Cargo.lock`, `workspace-boundaries.toml`, shared composition roots, registries, or governance surfaces.

Therefore P1 requires a new entry condition:

```text
P1_WORKSPACE_ENTRY_READY =
  P0 corrections accepted
  AND fresh live ownership census complete
  AND root Cargo/Cargo.lock lease is released or explicitly serialized/transferred
  AND workspace-boundaries ownership is explicitly allocated
```

A green documentation audit alone is not sufficient to admit the P1 writer while another active worker retains shared workspace custody.

### BLOCKER C — the planned input flow overstates the current `input-actions` contract

The current plan depicts:

```text
winit / OS
  -> input-platform
  -> input-actions / semantic contexts
  -> UI route
  -> gameplay when not consumed
```

The current implementation does not support that flow for every UI event family:

- `InputRouter` consumes semantic key/mouse/wheel action state but does not produce a UI pointer/text stream;
- pointer movement and committed text are normalized but ignored by `InputRouter`;
- `Ime::Preedit(text, cursor)` is currently collapsed to a marker with its text/cursor information discarded.

The P2 contract must be corrected to an application-owned arbitration boundary, conceptually:

```text
winit / OS
   -> input-platform normalized events
   -> apps/client input arbitration
        -> ui-core interaction/text/focus route
        -> input-actions semantic gameplay route when eligible
```

This does not require duplicating physical normalization. It requires accurately routing the existing event families and adding only demonstrated missing primitives.

### BLOCKER D — full IME behavior is not currently representable

P2 currently promises text/IME routing, but the existing platform adapter loses IME preedit text and cursor/range state.

P2 must explicitly allow the minimum bounded semantic extension required for:

- preedit/update;
- commit;
- cancel/disable;
- composition range/cursor representation where needed;
- focus loss during composition;
- no duplicate committed text from keyboard text plus IME commit.

The tests must prove that text-entry focus never leaks text/composition keystrokes into gameplay actions.

### BLOCKER E — active gameplay actions need cancellation semantics when UI takes ownership

The existing semantic router retains held inputs and active actions and emits `Started`, `Repeated`, `Ended`, and `Cancelled`.

It is not enough for the UI to start consuming new input after a modal/focus transition. If an already-started gameplay action becomes ineligible because UI focus/modal/capture ownership changes, the route must deterministically emit/cause the appropriate cancellation/ending behavior.

Required P2 negative tests include at minimum:

- gameplay key held -> text field gains focus;
- gameplay action active -> modal opens;
- pointer action active -> drag/modal capture changes;
- application loses focus while action is held;
- OS pointer capture is lost;
- device-loss/reset while UI interaction is active.

No stuck gameplay action may survive the ownership transition.

### BLOCKER F — P5/P6 do not yet have a concrete shared physical qualification host

Current facts:

- `tools/synthetic-client-harness` is a synthetic qualification program and is not a production-native interactive UI host;
- production `renderer` currently supplies surface lifecycle and a clear pass, not the full World+VFX scene;
- the physically proven world/VFX implementation exists in an isolated non-production experiment that deliberately does not link the production client/renderer/workspace.

The plan must not hide this gap behind broad wording such as `renderer/world presentation paths needed only for measured viewport extraction`.

Before P5/P6 admission, define one explicit composition seam where:

- production-safe UI APIs remain in production-safe crates/app code;
- synthetic fixtures depend on those APIs, never the reverse;
- production client dependency closure never reaches `client-domain`, `client-simulation`, synthetic assets, or the synthetic harness;
- the qualification host can actually render both the representative world scene and the HUD through the selected physical `wgpu` path;
- any reuse/promotion from the isolated World+VFX prototype receives its own bounded allocation rather than being smuggled into a UI PR.

P1 itself does not need the world renderer completed. This is an entry/ownership gate for P5/P6.

## 5. Delivery/DAG corrections

### 5.1 P2 and P3 are not freely parallel on all surfaces

P2 and P3 may be semantically parallel after P1, but their implementation PRs can overlap on:

- root/workspace dependency wiring;
- `Cargo.lock`;
- `workspace-boundaries.toml` edges;
- `apps/client/src/ui/**` composition glue.

The plan must state that these shared surfaces are separately serialized. Parallel execution is permitted only when the actual allocated path sets are disjoint.

### 5.2 P8 must remain dependency-driven rather than globally gated by FOV

The current visual DAG shows `P7 -> P8`, but many production view-model adapters do not depend on the FOV decision.

Correct the plan to distinguish:

- viewport/FOV-sensitive production integration: blocked by P7 where applicable;
- independent surfaces such as connection/status/chat or other projection adapters: may proceed once their owning production-safe client contracts exist and shared ownership permits.

No P8 feature may bypass a genuine FOV dependency, but P7 must not become an artificial blocker for unrelated client/UI integration.

### 5.3 P1 should remain minimal enough to review

P1 currently combines retained tree, geometry/layout, invalidation, focus/capture/modal, docking, drag/drop, virtualization, draw extraction, text interfaces, and diagnostics.

A correction should explicitly allow bounded P1 sub-slices if the public surface becomes too large for one independently reviewable PR. The invariant is one coherent minimal foundation, not one oversized commit.

Do not create speculative extra UI crates merely to split work.

## 6. Client composition correction

The current repository has two relevant pre-native client shapes:

- `ClientBootstrap` owns `ClientRuntime`, `SurfaceState`, and `InputPlatformAdapter` for non-gameplay/pre-native composition tests;
- the Windows interactive shell owns the actual window/event-loop/physical renderer lifecycle.

P2 must deliberately reconcile these responsibilities under `apps/client` instead of adding a third parallel runtime/input/render owner.

Required invariant:

```text
apps/client remains the sole production composition root
client-runtime owns async runtime lifecycle only
input-platform owns platform normalization only
renderer owns physical GPU/surface resources only
ui-core owns framework-neutral UI semantics only
```

## 7. Physical test and CI corrections

Current PR `game-gate` can prove build/lint/synthetic compatibility but its Windows `--smoke` path does not prove rendered UI interaction.

The implementation plan must bind evidence to stages instead of treating general green CI as sufficient physical proof.

### P1

Require pure deterministic tests and repository-required exact-head checks.

### P2

Require deterministic shell/input/DPI tests including:

- resize and scale changes;
- focus loss/regain;
- pointer capture gain/loss;
- modal/text ownership;
- IME preedit/commit/cancel;
- active gameplay action cancellation;
- minimized/zero-size transitions.

### P3/P4

Require physical `wgpu` evidence for:

- draw-list rendering;
- clipping/scissor;
- text/glyph rendering;
- cache/resource reconstruction;
- surface/device recovery where the owning renderer can exercise it.

### P5/P6

Require a named physical Windows/hardware/scene qualification cell plus Tier-2-equivalent native interaction/render evidence where applicable. A synthetic console harness alone is not a physical native-client UI proof.

### Later release

Tier 3 remains a release/product-artifact gate and is not required merely to create `ui-core`.

## 8. Fail-safe and rollback corrections

Keep the existing rule that optional visual resources may degrade safely, but distinguish decorative degradation from interaction-critical failures.

Required fail-safe behavior should include:

```text
optional decorative UI asset missing
  -> bounded visual fallback/degraded presentation

critical input/focus/modal arbitration unavailable or inconsistent
  -> fail closed for affected interaction; do not pass through gameplay input by default

UI renderer unavailable
  -> never fabricate gameplay state; preserve semantic client/game state

corrupt local layout
  -> safe default layout

surface/device loss
  -> rebuild physical UI resources from semantic UI state
```

Every runtime slice remains independently revertible unless a later separately accepted server/protocol contract says otherwise.

## 9. A/B viewport experiment corrections

### 9.1 Variant B must be one deterministic experimental policy

The phrase `fit/crop within safe constraints, or another measured presentation policy` is too broad for a direct A/B comparison.

The experiment must choose and record one exact B fixture policy before measurement, including:

- baseline gameplay tile extent fixture;
- preserved aspect ratio rule;
- letterbox/pillarbox/crop policy;
- centering rule;
- `world_zoom` definition;
- derived presentation/fit scale definition;
- pointer-to-world mapping through the fitted/cropped viewport;
- zero/minimum-size behavior.

The fixture value is not automatically a production FOV constant.

### 9.2 Separate world zoom from viewport fit scale

Both A and B must use the same semantic world zoom for direct comparison.

Variant B may need an additional presentation fit scale to place a fixed tile extent inside different viewport shapes. That derived scale must not be mislabeled as the player's world zoom.

### 9.3 Freeze experiment identity before performance claims

Each A/B evidence population must bind at minimum:

- exact code SHA;
- exact deterministic scene/fixture digest;
- seed/script identity;
- baseline tile-extent fixture;
- presentation family and resource-density configuration;
- hardware/GPU/driver/OS identity;
- resolution/window/DPI/UI-scale cell;
- warm-up procedure;
- measured frame count and repeat population;
- same event/workload timing for A and B.

A different scene or timing invalidates a direct A/B cell.

### 9.4 Control non-viewport information surfaces

Fairness comparison must explicitly control or record information delivered through:

- minimap;
- battle list;
- target selection/acquisition;
- names/health bars;
- UI alerts or other synthetic HUD projections.

Otherwise a purported FOV advantage/disadvantage may actually be caused by a different HUD information fixture.

### 9.5 Responsive acceptance remains server-evidence-gated

Client-side A may record projection deficit when the desired render extent exceeds available client projection.

It must not fabricate outer-world data and must not turn client window size into gameplay authority.

A product outcome of `VIEWPORT_RESPONSIVE_FOV_ACCEPTED` still requires separate evidence for server relevance/interest cost, network volume, fairness, bounds/clamping, resize-rate behavior, and the behavior when server-approved projection is smaller than the requested render extent.

## 10. Architecture Decision Discipline correction

The retained-tree decision has a credible rationale but should be brought fully into the repository's required analysis shape before treating the implementation architecture as audited `KEEP`.

The corrected architecture record should explicitly contain:

- **Problem** — persistent MMO HUD state, interaction, docking, focus and virtualization;
- **Constraints** — deterministic ownership, framework neutrality, `wgpu` separation, Windows-first shell, future browser semantic reuse without browser distortion;
- **Options** — retained production tree vs immediate-mode production UI, with immediate helpers remaining permitted for bounded debug tooling;
- **Trade-offs** — state complexity/invalidation cost vs stable identity/focus/docking/virtualization and reduced rebuild churn;
- **Risks** — over-engineering, stale/invalidation bugs, memory retention, implementation complexity;
- **Recommendation** — retained tree for production gameplay UI;
- **Future impact** — test automation, browser reuse of semantics, migration/supersession cost;
- **Decision timing** — why the next HUD proof needs this now and what measured evidence may reopen it.

This is a documentation correction, not an invitation to reopen the already evidence-backed custom Rust + `wgpu` renderer foundation.

## 11. Legacy viewport evidence verified

At exact reference `blakinio/otclient@53646cfa1957ce75f18547424a9e9177c4ad8cc2`, the audit directly verified:

- `MapView` initializes visible dimension to `15 x 11`;
- `data/setup.otml` has `map.viewport: 8 6`;
- `Map::resetAwareRange()` produces `left=8`, `right=9`, `top=6`, `bottom=7` from that setting;
- `UIMap::updateVisibleDimension()` lets width follow map-panel aspect ratio when `keepAspectRatio=false` while height remains based on zoom;
- aware range and visible dimension are separate concepts;
- `ProtocolGame::sendChangeMapAwareRange()` exists and is guarded by `GameChangeMapAwareRange`.

These facts support keeping Oteryn's render extent, available projection, gameplay visibility, and server relevance as separate concepts. They do not authorize copying OTClient's protocol, Lua/OTUI architecture, or legacy constants as Oteryn product policy.

## 12. Required repair sequence

The smallest safe continuation is:

```text
this independent audit record
  -> docs-only correction of the three UI architecture/plan documents
  -> fresh exact-head independent audit of the corrected documents
  -> resolve/release serialized root workspace ownership
  -> allocate UI-P1 only
```

The corrected documents may receive `KEEP` only after every blocking item in Section 4 is resolved and the material corrections in Sections 5-10 are reconciled.

Do not implement `UI-P1` merely because this audit record itself is merged.

## 13. Terminal audit result

```text
AUDITED_REPOSITORY = Oteryn/Oteryn-Game
AUDITED_PR = #549
AUDITED_EXACT_HEAD = 83314bcbf2978b20f07ca539dd697900dcdb410f
AUDIT_RESULT = FIX
UI_P1 = BLOCKED_PENDING_CORRECTION_AND_WORKSPACE_OWNERSHIP
FOV = UNDECIDED_EVIDENCE_GATED
ADDON_PLATFORM = DEFERRED_FUTURE_CONCEPT
```

No owner-sensitive unresolved choice discovered in this audit requires `NEEDS_DECISION`. The findings are repairable within existing accepted architecture and repository governance.

`IMPLEMENTATION_AUTHORITY: NONE`

`RUNTIME_ACTIVATION_AUTHORITY: NONE`

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
