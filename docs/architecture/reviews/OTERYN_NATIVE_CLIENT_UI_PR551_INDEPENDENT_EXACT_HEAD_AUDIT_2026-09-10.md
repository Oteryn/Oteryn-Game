# Oteryn native client UI PR #551 independent exact-head audit

- Date: 2026-09-10
- Repository: `Oteryn/Oteryn-Game`
- Audited pull request: `#551`
- Audited base: `main`
- Audited base SHA: `7144c0b9ec8691e481df058c85d890ac88d32461`
- Audited exact head: `0f26f4bfe03618f7faa8409f3f678d34a6acb886`
- Audit classification: **KEEP**
- Material findings: **NONE**
- Runtime/client/server/protocol/content implementation authority: **NONE**
- Merge authority: **REPOSITORY CONTROL PLANE ONLY**

## 1. Purpose

Persist the independent exact-head audit result for PR #551, `docs(ui): apply PR 549 audit corrections`, without changing the audited PR head.

This record is bound only to:

```text
repository: Oteryn/Oteryn-Game
pull_request: #551
base: main
exact_head: 0f26f4bfe03618f7faa8409f3f678d34a6acb886
```

Any material change to PR #551 after that head invalidates this audit target and requires a fresh exact-head review.

## 2. LIVE state verified

At audit completion:

- PR #551 was open and unmerged;
- `base=main`;
- exact head was `0f26f4bfe03618f7faa8409f3f678d34a6acb886`;
- protected `main` was `7144c0b9ec8691e481df058c85d890ac88d32461` and already contained PR #549;
- PR #551 changed exactly two Markdown files;
- current exact-head workflow evidence included successful Architecture semantic audit, Agent governance and latest Merge gate runs;
- no unresolved inline review threads were present.

This audit performed no repository, branch, PR, runtime or merge mutation before the audit result was reached.

## 3. Sources read and cross-checked

The audit read and cross-checked:

### PR #551 artifacts

- `docs/architecture/reviews/OTERYN_NATIVE_CLIENT_UI_PR549_INDEPENDENT_AUDIT_AND_REQUIRED_CORRECTIONS_2026-09-10.md`;
- `docs/architecture/OTERYN_NATIVE_CLIENT_UI_ARCHITECTURE_CORRECTION_ADDENDUM_2026-09-10.md`.

### PR #549 baseline documents

- `docs/architecture/OTERYN_NATIVE_CLIENT_UI_ARCHITECTURE_BASELINE_2026-09-10.md`;
- `docs/architecture/OTERYN_NATIVE_CLIENT_UI_ARCHITECTURE_IMPLEMENTATION_PLAN_2026-09-10.md`;
- `docs/architecture/OTERYN_NATIVE_CLIENT_VIEWPORT_AB_EXPERIMENT_PLAN_2026-09-10.md`.

### Governing instructions/policy

- root `AGENTS.md`;
- `docs/agents/META_AGENT_POLICY_BINDING.json`;
- bound `OTERYN_ORGANIZATION_AGENT_POLICY 3.1.0` at `Oteryn/Oteryn@3b39e0be05aef008f1bd442821daefa898a201dd`;
- `docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md`.

### Current implementation boundaries

- `apps/client` including `ClientBootstrap` and the Windows interactive shell;
- `crates/client-runtime`;
- `crates/client-domain`;
- `crates/client-simulation`;
- `crates/input-actions`;
- `crates/input-platform`;
- `crates/renderer`;
- `tools/synthetic-client-harness`;
- root workspace boundary policy in `workspace-boundaries.toml`;
- LIVE shared-workspace ownership around Issue #351 / Draft PR #356.

## 4. Finding closure verification

### 4.1 Workspace/Cargo ownership and P1 entry gate — CLOSED

The correction addendum expands the mechanically complete P1 workspace surface to:

```text
crates/ui-core/**
Cargo.toml
workspace-boundaries.toml
Cargo.lock  # only when produced by the exact package/dependency delta
```

It requires package identity, exactly one release role, declared internal dependency edges, valid production closure, `cargo metadata --locked`, architecture-check proof and repository exact-head checks.

P1 admission now requires correction integration/readback plus a fresh LIVE ownership census and explicit allocation for shared workspace paths. This correctly preserves the current serialized Cargo ownership held by #351/#356 as a live execution dependency rather than permanent architecture.

`UI-P1` is therefore **not authorized to start by this audit**.

### 4.2 Input arbitration — CLOSED

The original over-broad flow through `input-actions` is superseded by the correct split:

```text
input-platform -> platform/winit normalization
apps/client    -> application-level UI/gameplay arbitration
ui-core        -> UI focus/modal/capture/drag/text state
input-actions  -> semantic gameplay/action routing
```

The app must resolve UI ownership/reservation before gameplay emission, and one physical event cannot produce both an intended UI action and an unintended gameplay command.

This matches current code evidence: pointer/text normalized events are not general UI outputs from `InputRouter`, and current UI arbitration does not yet exist in the production shell.

### 4.3 IME semantics — CLOSED

P2 now explicitly requires:

- IME enabled/disabled state;
- preedit text updates;
- preedit selection/cursor/range where supplied;
- commit;
- cancel/clear;
- focus-loss reset;
- exactly-once committed text with no `KeyEvent::text` plus `Ime::Commit` duplication;
- bounded untrusted composition text;
- Polish-diacritic and real preedit-before-commit tests.

This closes the current implementation gap where `Ime::Preedit(text, cursor)` is collapsed to a marker and its payload is discarded.

### 4.4 Active-action cancellation — CLOSED

The correction requires already-active semantic gameplay actions to be reconciled when UI ownership changes, including:

- held gameplay key -> text focus;
- active gameplay action -> modal;
- pointer action -> UI drag/modal capture;
- application focus loss;
- OS pointer capture loss;
- device reset/loss;
- IME composition beginning while a text-producing gameplay binding would otherwise be eligible.

No stuck gameplay action may survive these transitions.

### 4.5 `ClientBootstrap` / Windows shell ownership — CLOSED

P2 now explicitly requires reconciliation of the existing pre-native `ClientBootstrap` and interactive Windows shell under one `apps/client` production composition root.

The invariant is preserved:

```text
apps/client       -> composition/order
client-runtime    -> async runtime/cancellation lifecycle
input-platform    -> platform normalization
input-actions     -> semantic action contracts/router
ui-core           -> framework-neutral UI semantics
renderer          -> physical GPU/surface/resources
```

No third production composition owner is introduced, and current pre-native gameplay-entry fail-closed behavior remains protected.

### 4.6 P2/P3 serialization — CLOSED

P2/P3 semantic parallelism is now conditional on path disjointness. Shared surfaces, including root Cargo/lock, `workspace-boundaries.toml`, `apps/client/Cargo.toml`, app UI composition glue and public `ui-core` contracts, require explicit serialization.

### 4.7 P5/P6 physical qualification host — CLOSED

The correction no longer treats the synthetic console harness as sufficient physical native UI evidence.

P5 requires one production-safe HUD composition seam with synthetic dependencies only pointing toward production-safe semantics, never the reverse.

P6 requires one bounded qualification host/cell that physically composes representative world presentation plus representative HUD/UI through an actual `wgpu` frame path.

This matches current source boundaries: production `renderer` owns the real DX12/`wgpu` surface but currently performs only a clear pass, while `tools/synthetic-client-harness` currently exercises synthetic state plus `SurfaceState` rather than a physical world+HUD render.

### 4.8 Physical / Tier-2 evidence — CLOSED

Evidence is now stage-specific:

- P1: deterministic foundation/workspace proof;
- P2: deterministic shell/input/DPI lifecycle plus named Windows-native cases where pure simulation is insufficient;
- P3/P4: actual physical `wgpu` primitives/text/clipping/recovery evidence;
- P5/P6: named Windows native/hardware/scene qualification and Tier-2-equivalent interaction/render evidence where applicable;
- Tier 3 remains a later product/release gate.

General green CI is explicitly not treated as sufficient physical UI proof.

### 4.9 Fail-closed semantics — CLOSED

The correction distinguishes optional visual degradation from interaction-critical ownership failure.

Unknown/inconsistent focus, modal, capture, hit-test or interaction-router state must fail closed for the affected interaction rather than defaulting to gameplay pass-through. UI renderer failure cannot fabricate or mutate authoritative gameplay/client state.

### 4.10 Variant B determinism and A/B fairness — CLOSED

Primary Variant B is now one deterministic fixture:

```text
fixed baseline GameplayFovExtent
preserved world aspect ratio
uniform presentation scaling
centered fitted world image
letterbox/pillarbox as needed
no non-uniform stretching
no crop in the primary B cell
```

`world_zoom` and derived `viewport_fit_scale` are distinct. Pointer mapping must invert the exact fitted transform, and letterbox/pillarbox coordinates map to no world tile.

Direct A/B evidence freezes code SHA, world fixture digest, seed/script, fixed-FOV fixture, world zoom, presentation family/resource density, HUD fixture, OS/target, GPU/driver/backend, logical/physical size, DPI/UI scale, warm-up, measured frames, repeat population and event/workload timing.

Fairness controls explicitly include minimap, battle list, target acquisition/selection, names/health bars, alerts/markers and other information surfaces that can expose off-viewport information.

### 4.11 P7/P8 DAG — CLOSED

The unconditional `P7 -> P8` dependency is correctly refined.

FOV-sensitive production adapters remain downstream of the applicable P7 outcome and, for responsive FOV, later server/relevance proof. FOV-independent adapters may proceed once their own production-safe sources, intent boundaries, UI foundations, path ownership, review and CI are available.

This does not authorize bulk P8 implementation.

### 4.12 Retained-tree decision analysis — CLOSED

The correction brings retained-vs-immediate UI ownership into the required `ARCHITECTURE_DECISION_DISCIPLINE` shape:

- Problem;
- Constraints;
- realistic Options;
- Trade-offs;
- Risks;
- Recommendation;
- superseding evidence / Future impact;
- Decision timing.

The retained production tree is selected only at the ownership-model level needed before P1 public state/interaction contracts. Exact storage and invalidation mechanisms remain implementation details and may be reopened by measured evidence.

## 5. FOV and addon guardrails

The audit confirms that PR #551 preserves:

```text
RESPONSIVE_FOV_POLICY = UNDECIDED
FIXED_FOV_POLICY = UNDECIDED
ADDON_PLATFORM = DEFERRED_FUTURE_CONCEPT
```

Responsive client-side behavior may be qualified experimentally, but product acceptance remains gated by separate server relevance/network/fairness proof where additional authoritative world projection is required.

No addon/mod/plugin runtime, scripting VM, manifest format, community API, sandbox/capability system, hot reload, marketplace, signing or distribution implementation is authorized.

## 6. Execution and integration guardrail

The exact-head audit result `KEEP` removes the independent-review content blocker for PR #551 at the audited head only. It does **not** itself merge #551 and does **not** authorize UI-P1.

The correction lifecycle remains:

```text
PR #551 exact-head KEEP
  -> repository-required exact-head checks
  -> normal Merge Queue
  -> successful merge_group aggregate gate
  -> protected-main readback of the correction
  -> fresh LIVE shared-workspace ownership census/allocation
  -> UI-P1 may be considered for admission
```

Any failed required gate fails closed. No bypass, direct merge, force push, protection weakening or alternate integration authority is implied.

## 7. Terminal audit result

```text
AUDITED_REPOSITORY = Oteryn/Oteryn-Game
AUDITED_PR = #551
AUDITED_BASE = main
AUDITED_BASE_SHA = 7144c0b9ec8691e481df058c85d890ac88d32461
AUDITED_EXACT_HEAD = 0f26f4bfe03618f7faa8409f3f678d34a6acb886
AUDIT_RESULT = KEEP
MATERIAL_FINDINGS = NONE
UI_P1 = BLOCKED_PENDING_CORRECTION_INTEGRATION_READBACK_AND_LIVE_WORKSPACE_ALLOCATION
FOV = UNDECIDED_EVIDENCE_GATED
ADDON_PLATFORM = DEFERRED_FUTURE_CONCEPT
```

`IMPLEMENTATION_AUTHORITY: NONE`

`RUNTIME_ACTIVATION_AUTHORITY: NONE`

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
