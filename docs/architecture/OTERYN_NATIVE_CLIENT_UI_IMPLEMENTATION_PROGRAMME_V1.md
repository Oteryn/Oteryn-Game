# Native client UI implementation programme V1

Status: AUTHOR_CANDIDATE / NOT_ACTIVE. Prepared 2026-09-11.

This is a conditional implementation architecture, delivery plan and qualification matrix, not a runtime allocation or a replacement control plane. The companion is `docs/agents/programs/OTERYN_NATIVE_UI_AGENT_PROGRAMME_V1.md` (sections 11-16). These two documents form one packet. Existing architecture and audit records remain intact. Only an independently reviewed, protected successor may adopt this candidate; conflicts retain the existing protected authority until resolved.

## 1. LIVE STATE

FACT (observed snapshot, not a permanent current-state assertion): repository `Oteryn/Oteryn-Game`, protected `main@fc0ecb064b1d4a23a87dbba8070eb91a5142be03`, tree `fa4c025cfed3725c4b501b42292e8f846d6d5a18`. Initial source reads used `5ec6ca6369e98a6f66f679cfc7fc1248fb5992fb`; fresh pre-publication readback observed #558 integration at 2026-09-11T07:51:33Z. Re-read GitHub before execution. All PR tuples below have `repository=Oteryn/Oteryn-Game`, `base=main`.

| PR | Exact observed head | Observed state | Protected merge commit |
|---|---|---|---|
| #549 | 83314bcbf2978b20f07ca539dd697900dcdb410f | MERGED | 7144c0b9ec8691e481df058c85d890ac88d32461 |
| #551 | 0f26f4bfe03618f7faa8409f3f678d34a6acb886 | MERGED | 5025be6cf3f5140cf94708f8e6ddc9ab3f40d99f |
| #557 | c76858b23298e9f012b72ce834b3f826722940e5 | MERGED | 6db1e95dcd0377d3258c045ea0b95f5d620c53f7 |
| #560 | db502e473bda60f7cdea2498f5138705c2cf0bea | MERGED | 663bd35a5196a925fc6eb0318381ad0b97f4cc2c |
| #562 | af8ed70960a1d262ab6e9f93b920fc6512d95df0 | MERGED | 5ec6ca6369e98a6f66f679cfc7fc1248fb5992fb |
| #558 | 4de101473cad4a886b7cd1559f324b33ab4c6e0b | MERGED | fc0ecb064b1d4a23a87dbba8070eb91a5142be03 |

#558's source base SHA was `5ec6ca6369e98a6f66f679cfc7fc1248fb5992fb`. Its earlier PR test-merge SHA was not integration evidence. The actual merge-group SHA and protected merge commit are now both `fc0ecb064b1d4a23a87dbba8070eb91a5142be03`. Preserve the completed branch lineage; do not reopen or create replacement #558 work.

Exact-head #558 evidence, GitHub Actions Merge gate run `34574214586`:

| Requirement | Observed result | Evidence / limitation |
|---|---|---|
| Formatting | PASS | policy/metadata job 103182995787, Verify formatting |
| Workspace build | PASS | Linux job 103182995740, locked workspace all-targets |
| Strict Clippy | PASS | same job, all-targets, `-D warnings` |
| Workspace tests | PASS | same job, locked workspace tests |
| Focused input regressions | PASS within workspace run | input-platform 17/17, including six `audit_regressions`; input-actions 11/11. No separate focused invocation asserted. |
| Focused renderer regressions | PASS within workspace run | renderer 15/15, including four acquisition-boundary cases; not hardware observation |
| Windows lane | PASS | job 103182995730; existing client build/Clippy/smoke and synthetic/SIM steps, not native UI qualification |
| Supply chain | PASS | job 103182995774 |
| Aggregate game-gate | PASS | job 103185060906 |
| Other workflows | SUCCESS | Agent governance 34574214549; Architecture semantic audit 34574214476. Workflow success alone does not establish a Native UI profile. |
| Independent review activity | COMPLETED, coordinator accepted | Codex summary comment 5631012973 binds `4de1014`; #162 comment 5631116708 records exact-head acceptance with no finding/thread. `reviews=[]` means no formal review submissions, not absence of bot review activity. |
| Queue / protected-main integration | PASS / MERGED | actual `merge_group` run 34575603814, game-gate job 103189466858 SUCCESS; protected main readback equals fc0ecb064b1d4a23a87dbba8070eb91a5142be03 |

Fresh source reconciliation: compare `5ec6ca6...` to `fc0ecb0...` is ahead by one commit and exactly the nine #558 owned files. Governance, the five architecture source documents, root Cargo and workspace-boundaries are unchanged. This packet is based on the new protected main; it does not claim to have performed that merge.

Evidence scope: the existing Linux log proves six modifier/lifecycle regression cases and four acquire/preflight cases, not future IME, UI, device recovery or display latency. Historical failures on older heads remain historical. Never transfer a run to another head.

## 2. AUTHORITY / GOVERNANCE

Read root `AGENTS.md`, applicable `docs/agents/AGENTS.md`, `docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md`, `docs/agents/BUILD_TEST_MATRIX.md`, task template and current ownership before any mutation. This packet does not rewrite them.

Bound META: `docs/agents/META_AGENT_POLICY_BINDING.json` selects `Oteryn/Oteryn@3b39e0be05aef008f1bd442821daefa898a201dd`, policy 3.1.0. Consult its organization policy, prompting standard, prompt-evaluation standard and `docs/governance/AI_REVIEW_POLICY.md`. Public safety instructions in the companion are operational constraints, not copied hidden instructions.

LIVE ruleset `20991995` is active: required `game-gate`, Merge Queue/ALLGREEN/SQUASH, non-fast-forward and deletion protection, resolved review threads, no bypass actors; `current_user_can_bypass=never`. Required approving-review count is zero. This does not waive the independent review explicitly required for this programme. External AI is advisory, not a new required check or integration authority.

Coordination is existing issue #162 and protected `docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md`, reconciled with current authoritative coordinator checkpoints. Comment `5630945976` retains #351/#356's Cargo custody; later `5631070425` explicitly preserves WP3 authority despite an executor transient. Comment `5631116708` qualified #558 before its now-observed protected integration. No P1 Cargo release/allocation was found in these readbacks. Old allocation prose, earlier heads or elapsed time never release leases. The Native UI Lead is subordinate to the unique active control-plane profile, not its replacement.

Open-PR census identified #536 touching `docs/agents/PROMPT_LIFECYCLE.json` and `docs/agents/README.md`, and #520 touching the architecture index. Do not take over those shared surfaces. New aliases in this packet are candidate definitions, NOT_REGISTERED and NOT_DISPATCHABLE; activation is CP-A below. Native tools and repository CI are the execution route; Remote Desktop is not authorized for this work.

### Protected source inventory and precedence

The following complete architecture documents were read at initial main `5ec6ca6369e98a6f66f679cfc7fc1248fb5992fb` and are unchanged in the reconciled protected main above. Paths share `docs/architecture/`:

| File | Exact Git blob |
|---|---|
| OTERYN_NATIVE_CLIENT_UI_ARCHITECTURE_BASELINE_2026-09-10.md | 7ad2dfc233980c1a424e6e129816596b99bd758b |
| OTERYN_NATIVE_CLIENT_UI_ARCHITECTURE_IMPLEMENTATION_PLAN_2026-09-10.md | 36aaaa191b9bdd34d6a8bc9c15dc2a2418a839c3 |
| OTERYN_NATIVE_CLIENT_VIEWPORT_AB_EXPERIMENT_PLAN_2026-09-10.md | ea35bf121ecab4fa0f580313cb99b8b09e0cd935 |
| OTERYN_NATIVE_CLIENT_UI_ARCHITECTURE_CORRECTION_ADDENDUM_2026-09-10.md | 7c661000ab361604e1cb64241818ccca51933e94 |
| OTERYN_NATIVE_CLIENT_UI_ARCHITECTURE_CORRECTION_CONTINUATION_2026-09-11.md | 1341ac9cc73a3c13514ffcbc0b7cf1f6598f81ec |

No new historical files are proposed. The continuation/addendum govern their explicit corrections, not wholesale erasure of the baseline. Parent ALPHA client contracts, ADR-0007 tier obligations and existing resource/settings authority are consumed through these corrections; this planning pass is not a new exhaustive audit of all parent/third-party source.

Directly inspected current package authority: root `Cargo.toml` blob `fb4134748560219c5a4cb80f58ad98ac7d1c22a2`, `workspace-boundaries.toml` blob `6af6c9f9e05151afa603cd3d86f4fb3261abaa3c`; selected checker source confirms local workspace/role/internal-edge validation. P1 requires additional external closure proof, not a claim that the existing checker already supplies it.

## 3. CURRENT PROGRAMME STATUS

`UI-P0_RUNTIME_PREREQUISITES=PROTECTED_INTEGRATED`; `UI-P0_ADMISSION=BLOCKED_SHARED_LEASE`; `UI-P1=PREPARE / NOT_ADMITTED`; subsequent implementation is NOT_ACTIVE. #558 is no longer an integration blocker. Planning and bounded read-only design can proceed while P1 Cargo/allocation and candidate-adoption gates remain open. No P2/P3 start follows from document completion.

This candidate remaps historical stage names explicitly:

| Protected historical responsibility | New programme responsibility |
|---|---|
| P0 prerequisite closure, P1 foundation, P2 input, P3 renderer | same responsibility, smaller slices |
| P4 text/resources | P4-T; widgets/layout additionally P4-L |
| P5 full synthetic HUD | P5-F, clearly labelled native fixture evidence |
| P6 A/B experiment | P7-A; new P6-N/P6-J separate native and Tier-2 qualification |
| P7 FOV decision | P7-D; fixed disposition is evidence/owner gated, while responsive is pending-only until separate relevance proof plus applicable independent/owner acceptance |
| P8 production adapters | P5-R; new P8-F is feature-driven higher surfaces |
| P9 polish/hardening | P9-H/P9-R, with explicit Tier-3/recovery/rollout gates |

This crosswalk changes planning labels, not accepted obligations or readiness. No final visual style is selected.

## 4. FINAL ARCHITECTURE

### 4.1 Boundaries and production closure

```text
OS/winit -> input-platform -> apps/client arbitration
                              |-> ui-core (eligible UI ownership)
                              `-> input-actions (eligible gameplay routing)

session/projection -> apps/client typed view-model -> ui-core
ui-core intent -> apps/client validation -> existing production application seam
ui-core draw extraction -> renderer -> GPU/surface
apps/client -> client-runtime (bounded async/cancellation only)
synthetic-client-harness -> production-safe contracts (one-way test consumption)
```

`apps/client` remains the sole production native-client composition root. `input-actions` owns framework-neutral normalized physical/value/action contracts and semantic action routing, never widgets or a generic UI bus. `input-platform` extracts/normalizes OS events, never owns native widget state. Proposed `crates/ui-core/` (`oteryn-ui-core`) owns retained UI model, layout, hit testing, focus/capture semantics and draw descriptions. It must not depend on winit, wgpu, protocol/server authority, uncontrolled async runtimes or synthetic-only packages. `renderer` owns physical GPU/surface/resources.

`client-runtime` stays lifecycle/cancellation support, not the composition root. `client-domain` is a projection boundary, not platform/UI authority; importantly, current `workspace-boundaries.toml` classifies BOTH client-domain and client-simulation as synthetic. Neither may silently enter production closure. P5-R must use actually admitted production contracts, or request a separate reviewed production-admission change. A name or copied projection type does not admit a package. The synthetic harness remains a support tool, never the production entry point.

### 4.2 Retained state, geometry and draw semantics

DECISION: retained semantic identity contains a node slot/identity plus lifetime generation. Generation overflow, removed nodes, recycled virtual rows and stale async/resource identities fail closed; no wrapping into apparent validity. Concrete arena/library remains evidence-gated. Bound nodes, depth, dirty work, draw items, event queues and resources before allocation; reject cycles, non-finite geometry and over-budget work without partially publishing a tree.

Separate OS physical, platform logical, user-scaled UI, renderer/world coordinates. For aligned origins, `physical = ui * dpi_scale * user_scale`; all transforms include explicit origin and clipping. Apply each scale once. Use half-open edges, a documented rounding rule and bounded physical scissor conversion. Layout, hit test and draw extraction share one immutable geometry epoch. Resize/DPI/scale changes invalidate relevant snapshots; zero-size suppresses acquisition, division and invalid world hits. Input must not target new geometry while old pixels remain the only presented geometry; deny affected targeting until the presentation contract is coherent.

Extract framework-neutral ordered primitives: quads, borders/nine-slice, images, shaped text, clips and transforms. Preserve alpha/painter order rather than sorting transparently across layers. Draw IDs reference logical resources; GPU handles stay in renderer. Bounds/cache keys include content/style/font/scale and applicable device generation. Virtual rows invalidate focus, hover, drag and tooltip bindings on rebinding.

### 4.3 Input, IME and scroll

Arbitration happens in apps/client after platform normalization. UI ownership cannot suppress physical/router cleanup. Test left/right Ctrl/Shift/Alt/Super, modifier snapshots, repeat, consumed press/release, modal/context transitions, focus/capture/device loss and duplicate loss signals. A blocked or cancelled held press remains disarmed until release plus a fresh eligible press. `ContextKind::Global` cannot bypass UI/gameplay ownership.

Native IME is a real platform contract: preedit, optional UTF-8 byte cursor/range validated for bounds and character boundaries, clear/cancel, exactly-once commit, focus-loss cancellation, enable/disable and coherent candidate-area placement. Prevent KeyEvent text plus IME double insertion. Do not log committed/preedit text. Qualify Polish diacritics, replacement, focus changes and DPI/user-scale transitions on the native host.

UI scroll retains line versus pixel units and finite bounded fractional magnitude before any gameplay conversion. UI consumption precedes gameplay impulses. P2-S decides whether accumulation is needed; if used, bound it and specify reset on focus/context/device/target changes. Do not retrofit smooth UI scrolling into a discrete wheel impulse without a typed contract and tests.

### 4.4 Renderer and asynchronous lifetime

Check renderer generation before avoidable acquisition, queue submission, present and reconfiguration effects; rejected stale work produces zero such effects. Preserve phase-denial ordering and failure truthfulness. Consume/drop an acquired surface frame before reconfiguration. No cross-frame/callback frame ownership leak.

Keep input-device loss, surface loss/outdated/suboptimal/reconfiguration, and GPU device/queue loss separate. A surface recreation is not GPU-device recovery. Define bounded progress/cancellation for callbacks and device polling. Unqualified GPU recovery is explicit terminal behavior, not a false recovered state.

Evidence states remain distinct: acquired, submitted, present-requested, GPU-complete, display-observed. CPU timing or zero must never replace unavailable GPU timing. A successful `Result<()>`, synthetic callback or `present()` request is not display observation.

Delayed work carries session, projection, node lifetime and resource/device generation as applicable. apps/client owns bounded scheduling and validates stamps again before applying a result. Widgets create no runtimes, threads or unbounded tasks; dropping a handle or timing out is not proof of task termination.

### 4.5 Product surfaces, persistence and FOV

Baseline HUD: world viewport; HP/mana/status; minimap; battle list; equipment; at least one container; chat; action bars; tooltip; drag/drop; modal; docking; scaling at 100/125/150/200%. UI emits intents, never authoritative gameplay state. Missing/stale session/projection disables affected actions without fabricating data. P5-F proves a labelled fixture; P5-R proves real production integration. Feature-driven later surfaces do not create addon/plugin architecture.

Persist layout/preferences in the owning application settings boundary, not ui-core. Inherit the accepted per-field scope contract: permitted ephemeral overrides, DEVICE, OS_USER, ACCOUNT, then product defaults; INSTALLATION is not a universal overlay. Preserve privacy opt-outs and restrictive precedence; do not create account-sync authority. Bound/version/validate before failure-atomic replacement; interrupted writes retain a recoverable old or new value. Unknown future schema/downgrade preserves original bytes and refuses destructive overwrite. Corruption uses safe defaults without erasing recoverable evidence. Migration, backup retention and rollback are bounded and tested separately in P5-P.

`RESPONSIVE_FOV_POLICY = UNDECIDED / EVIDENCE-GATED`; after P7-A it may advance only to `RESPONSIVE_FOV_PREFERRED_PENDING_SERVER_RELEVANCE_PROOF` until the separate server/network/fairness relevance spike succeeds.
`FIXED_FOV_POLICY = UNDECIDED / EVIDENCE-GATED`.

Resource budgets, renderer limits and larger windows are not FOV decisions. Separate render extent, available projection, gameplay visibility and server relevance. Fixed-B experiment: identical aspect, uniform fit, centered letterbox/pillarbox, no crop; bars are not world input. Same world/session/content/HUD population and gameplay zoom for A/B. P7-A produces reproducible viewport A/B evidence only and MUST NOT claim server relevance, network behavior or fairness truth. P7-D may accept a fixed-FOV policy from its accepted evidence and owner gates. It may not record terminal responsive acceptance from A/B: `VIEWPORT_RESPONSIVE_FOV_ACCEPTED_AFTER_SERVER_RELEVANCE_PROOF` additionally requires the separate server/network/fairness relevance evidence, applicable independent review and owner decision. These gates block only responsive-FOV-sensitive work; non-FOV-sensitive P8/P9 work remains parallel-safe. Existing #502/resource registry own resource authority; no new numeric gameplay budget is invented here.

### 4.6 P1 workspace/dependency admission

Proposed exact package: `crates/ui-core/Cargo.toml`, package `oteryn-ui-core`, ordinary workspace edition/lints/license/publish=false. Initial normal dependencies: std only; initial build dependencies: none; initial dev dependencies: none. Any later helper/library requires explicit category, feature, target, licensing and closure review; a dev-only fixture must not become a normal/build edge or contaminate the production artifact.

Serialize root `Cargo.toml`, `Cargo.lock` (only resolver-produced changes), `workspace-boundaries.toml` and relevant package manifests. Register ui-core exactly once as production-safe, not a new production root; declared internal edges and metadata must agree. No app activation in P1. Initial public surface is bounded identity/geometry/tree/input-neutral/draw contracts, not a general framework.

P1-E evidence algorithm (design; not implemented by this packet):
1. Parse all declared dependency categories, optional dependencies, target predicates and features, including disabled declarations.
2. Resolve locked Cargo metadata WITH external packages/resolve graph, not `--no-deps` only. Walk package IDs and dependency kinds, not substring matches in cargo tree.
3. Check default, no-default and each relevant supported feature combination/target; enumerate supported combinations explicitly, with Windows target closure. An untested combination is unsupported or gated, not implicitly safe. Check optional/target-only declared edges even when the current host does not resolve them.
4. Traverse normal and build closures separately; forbidden wgpu/winit, synthetic, server/protocol authority or disallowed runtime edges fail with an exact dependency witness. Dev-only allowance is explicit and verified not reachable in production normal/build closure.
5. Negative fixtures independently inject direct forbidden, transitive forbidden, build-only, optional-feature-only and Windows-target-only forbidden edges. Include valid positive and allowed-dev-only controls; missing/malformed metadata and incomplete target coverage fail closed.
6. Keep the existing workspace checker. Add the missing proof under its owning tools/CI allocation, not a cosmetic PASS script. P1 exit requires both old checks and the new closure evidence on the exact integrated candidate.

## 5. DEPENDENCY DAG

```text
P0 protected prerequisites + allocation/lease admission
  -> P1 admission + external closure proof
       +-> P2 normalized values / arbitration / IME / scroll --+
       +-> P3 draw + native world/UI presentation host --------+-> P5 full HUD
       `-> P4 layout (parallel); P3 joins visual/text work -----+     | fixture / real projection
                                                                  v
                                                         P6 native / real Tier 2
                                                          |               |
                                                          v               v
                                                 P7-A A/B -> P7-D   P8 feature-driven
                                                  fixed decision;   (FOV-independent first)
                                                  responsive pending
                                                     | separate server relevance proof
                                                     | + independent review + owner decision
                                                     \ responsive-FOV-sensitive joins /
                                                           P9 selected-scope hardening,
                                                              Tier 3 and rollout
```

Machine-friendly edge list below describes stage-level constraints; `condition` restricts the edge to the named sub-slice. Hard edges are acyclic. `parallel_safe` and `soft` are planning relations, not permission to ignore a shared-file lease. The detailed slice table is the execution-level entry authority after adoption/allocation.

```json
{
  "schema_version": 1,
  "stages": ["UI-P0", "UI-P1", "UI-P2", "UI-P3", "UI-P4", "UI-P5", "UI-P6", "UI-P7", "UI-P8", "UI-P9"],
  "edges": [
    {"from":"UI-P0","to":"UI-P1","type":"hard","condition":"protected prerequisites and admission"},
    {"from":"UI-P1","to":"UI-P2","type":"hard","condition":"all input implementation"},
    {"from":"UI-P1","to":"UI-P3","type":"hard","condition":"all renderer UI integration"},
    {"from":"UI-P1","to":"UI-P4","type":"hard","condition":"pure model/layout can start"},
    {"from":"UI-P3","to":"UI-P4","type":"hard","condition":"visual/text integration only"},
    {"from":"UI-P2","to":"UI-P3","type":"parallel_safe","condition":"disjoint files; serialize composition/manifests"},
    {"from":"UI-P2","to":"UI-P4","type":"soft","condition":"interactive acceptance joins at P5; pure layout does not wait"},
    {"from":"UI-P2","to":"UI-P5","type":"hard","condition":"complete HUD interaction contracts"},
    {"from":"UI-P3","to":"UI-P5","type":"hard","condition":"native world/UI host and draw contract"},
    {"from":"UI-P4","to":"UI-P5","type":"hard","condition":"required widgets and text"},
    {"from":"UI-P5","to":"UI-P6","type":"hard","condition":"fixture for P6-N; real projection for P6-J"},
    {"from":"UI-P6","to":"UI-P7","type":"evidence_only","condition":"native host qualified before comparative A/B"},
    {"from":"UI-P5","to":"UI-P8","type":"hard","condition":"P5-R real integration for product features"},
    {"from":"UI-P6","to":"UI-P8","type":"evidence_only","condition":"applicable native interaction acceptance"},
    {"from":"UI-P7","to":"UI-P8","type":"owner_decision","condition":"fixed-FOV-sensitive features require accepted fixed P7-D disposition; responsive-FOV-sensitive features require VIEWPORT_RESPONSIVE_FOV_ACCEPTED_AFTER_SERVER_RELEVANCE_PROOF after separate server/network/fairness evidence, applicable independent review and owner decision; non-FOV-sensitive features do not wait"},
    {"from":"UI-P6","to":"UI-P9","type":"evidence_only","condition":"native and Tier-2 acceptance before rollout"},
    {"from":"UI-P8","to":"UI-P9","type":"hard","condition":"only the explicitly selected release feature set"},
    {"from":"UI-P7","to":"UI-P9","type":"owner_decision","condition":"fixed-FOV-sensitive release scope requires accepted fixed P7-D disposition; responsive-FOV-sensitive release scope requires VIEWPORT_RESPONSIVE_FOV_ACCEPTED_AFTER_SERVER_RELEVANCE_PROOF after separate server/network/fairness evidence, applicable independent review and owner decision; non-FOV-sensitive hardening and release scope do not wait"}
  ],
  "serialization_points": ["Cargo.toml", "Cargo.lock", "workspace-boundaries.toml", "apps/client composition", "ui-core public exports", "renderer public exports", "prompt lifecycle/index", "physical evidence host"]
}
```

P3-H host discovery/design may proceed read-only before implementation admission. CP-S and CP-D are separate control-plane proposals; neither is a mandatory dependency of pure layout progress. CP-A gates new alias dispatch, not read-only review of this packet.

## 6. UI-P0..UI-P9 PLAN

| Stage | Entry | Exit (all applicable clauses) | Current disposition |
|---|---|---|---|
| P0 | live governance and existing writers known | corrections and required #558 repairs protected-integrated/read back; no overlapping writer; current baseline and exact leases granted | RUNTIME_READY / ADMISSION_BLOCKED |
| P1 | P0; Cargo/manifest/checker lease; admitted writer | package admitted, normal/build neutrality and negatives proven, boundaries/tests green, protected readback; no product activation | PREPARE |
| P2 | P1 contracts; input and composition leases | focus/modal/capture, cleanup, IME and scroll implemented; stale/repeat/loss negatives; affected Windows package evidence; native acceptance remains P6 | EVIDENCE_GATED |
| P3 | P1; renderer/composition leases; shader decision before affected work | draw integration, generation-before-effects, coherent frame ownership, truthful outcome states; actual native world/UI host path exists and is exercised | EVIDENCE_GATED |
| P4 | P1 for pure layout; P3 for visual/text integration | required bounded layout/widgets/virtualization and shaped text/resources; deterministic geometry and draw tests; no final art freeze | EVIDENCE_GATED |
| P5 | necessary P2/P3/P4 joins | full native fixture HUD explicitly labelled; real session/projection-backed product adapter separately proven; bounded settings persistence if included | EVIDENCE_GATED |
| P6 | runnable native host/full HUD; real contracts for Tier 2 | physical matrix recorded with pass/fail/missing cells; Tier-2 journeys actually completed separately; no synthetic relabel | EVIDENCE_GATED |
| P7 | comparable qualified native A/B host | P7-A records reproducible A/B observations without server relevance/network/fairness truth claims; P7-D may accept fixed FOV with sufficient accepted evidence and owner gates, but responsive remains UNDECIDED or `RESPONSIVE_FOV_PREFERRED_PENDING_SERVER_RELEVANCE_PROOF` until separate server/network/fairness evidence, applicable independent review and owner decision permit `VIEWPORT_RESPONSIVE_FOV_ACCEPTED_AFTER_SERVER_RELEVANCE_PROOF` | EVIDENCE_GATED |
| P8 | P5-R; applicable P6; feature-specific contracts | individually requested higher surfaces accepted; fixed-FOV-sensitive work waits for its accepted P7-D fixed disposition and responsive-FOV-sensitive work waits for `VIEWPORT_RESPONSIVE_FOV_ACCEPTED_AFTER_SERVER_RELEVANCE_PROOF`; non-FOV-sensitive work does not wait; addons stay deferred | FEATURE_DRIVEN |
| P9 | selected release features and applicable P6 outcomes; applicable fixed or final responsive P7 disposition only for FOV-sensitive scope | bounded resources/performance/recovery/persistence; exact release Tier 3; rollback exercised; explicit rollout admission and protected readback; non-FOV-sensitive hardening/release work does not wait for responsive proof | EVIDENCE_GATED |

## 7. PR SLICE TABLE

Every row inherits the following mandatory fields: `base=live protected main`, one exact admitted writer, no protected-main mutation, no unrelated server/protocol/synthetic/CI/governance writes, exact-head tests/CI/review and normal queue readback before downstream production admission. Owned paths are proposed ceilings, NOT current leases. A coordinator grant must narrow them to exact files before implementation. Future files/modules are labelled proposed; no current binary is invented.

Legend (part of every row): CI `G/U/I/R/W/P/H/C` is defined in section 9. Windows `W0`=no native execution required for this docs slice; `W1`=affected package tests plus existing Windows lane; `W2`=W1 plus physical native cells. Rollback `RB0`=docs-only revert preserving historical records; `RB1`=no-activation package/contract revert together with all new consumers; `RB2`=disable/revert UI consumer through explicit safe uninitialized state while preserving cleanup; `RB3`=restore validated renderer path or explicit terminal/no-present state, never stale generation; `RB4`=disable product UI actions/feature without forging session state; `RB5`=preserve persisted bytes, bounded prior schema recovery, never overwrite future schema; `RB6`=invalidate evidence/revert policy choice, no runtime claim. Lease codes are section 8. `after` is both dependency and required merge ordering. Physical `none` means not claimed, not automatically qualified.

| ID / objective | Entry and after | Exit | Owned paths / expected package-target | Required tests; CI; Windows; physical | Lease; rollback |
|---|---|---|---|---|---|
| P0-Q reconcile completed prerequisite and admission | #558 already protected; existing coordinator only for new grant | preserve #558 evidence; explicit new admission/lease decision before P1 | read-only #558 history; allocation writes only by existing authorized coordinator, not this packet | section 1 matrix and actual queue/main evidence; G,I,R,W; W1; no UI physical claim | no inherited custody; RB1/RB3 for a separately authorized repair only |
| P1-E external closure enforcement | P0; separate tools lease | direct/transitive/build/optional/target negatives and positives executed | `tools/architecture-check/` bounded new checker module/tests; explicit wiring allocation only | dependency algorithm 4.6; G,C,U,W; W1; none | L-CHECK/L-CARGO as needed; RB1 |
| P1-A admit ui-core foundation | P0; P1-E available for exit; Cargo released | one admitted package, bounded primitives, no activation | proposed `crates/ui-core/`; root manifests/boundaries exact edits; `oteryn-ui-core` | identity/geometry/bounds/unit and closure; G,U,W; W1; none | L-CARGO/L-CORE; RB1 |
| P2-N normalized value contracts | P1 | neutral typed pointer/text/scroll values, loss/modifier preservation | bounded files `crates/input-actions/`, `crates/input-platform/`; existing packages | units/range/modifiers/repeat/loss; G,I,W; W1; later P6 | L-INPUT; RB1 |
| P2-A app arbitration/focus | P2-N; ui-core semantics | consumed cleanup, modal/global/fresh-press invariants | proposed app UI arbitration module, ui-core focus/capture module; existing router seam only if leased | Q01,Q04-Q07,Q11; G,I,W; W1; later P6 | L-COMPOSE/L-CORE/L-INPUT; RB2 |
| P2-I native IME | P2-A; platform behavior evidence | exactly-once commit, UTF-8/range, focus cancel and candidate placement | leased input-platform extraction + app IME bridge + ui-core edit model | Q08,Q16,Q17; G,I,W; W2; PH-IME | L-INPUT/L-COMPOSE/L-CORE/L-HOST; RB2 |
| P2-S fractional scroll | P2-N/P2-A; accumulation decision | typed units, bounded fractions, UI-first routing, reset tests | leased input values/platform and app scroll bridge | Q18 plus consumed gameplay negatives; G,I,W; W2; PH-SCROLL | L-INPUT/L-COMPOSE; RB2 |
| P3-D renderer draw integration | P1; shader evidence | ordered UI primitives, bounded resources, stale generation before effects | `crates/renderer/` leased draw/resources modules; `oteryn-renderer` | Q09,Q10,Q12,Q19; G,R,W; W1; later host | L-RENDER/L-CARGO for feature change only; RB3 |
| P3-H native world/UI host | P3-D; explicit host design/allocation | actual native target renders world+UI, frame/outcome stamps and progress exercised | apps/client native composition and renderer seam; support harness may consume one-way only under separate lease | Q03,Q09-Q11,Q15,Q19; G,R,W; W2; PH-FRAME | L-COMPOSE/L-RENDER/L-HOST; RB3 |
| P4-L layout/widgets | P1; visual part joins P3-D | bounded row/column/stack/docking/modal/virtual lists and coherent hit tests | proposed ui-core layout/widgets modules, no OS/GPU deps | Q01-Q03,Q11; G,U; W1; physical later | L-CORE; RB1 |
| P4-T text/resources | P1; library/license decision; P3-D for integration | shaping/measure/draw agree; bounded font/glyph/image cache | ui-core text contracts + renderer text/resource adapters, manifests only under lease | Polish/fallback/bidi cases as supported; Q03,Q12; G,U,R,W; W2; PH-TEXT | L-CORE/L-RENDER/L-CARGO; RB1/RB3 |
| P5-F complete native fixture HUD | P2 complete; P3-H; P4 complete | all baseline surfaces in one native host; synthetic data explicitly labelled | proposed `apps/client/src/ui/` shared product-safe composition; synthetic fixture adapter outside production closure | full HUD, Q01-Q13; G,P,W; W2; PH-HUD-FIXTURE | L-COMPOSE/L-CORE/L-HOST; RB2 |
| P5-R real session/projection HUD | P5-F; actual production contracts admitted | real projection, validated intents, stale/missing session fail-closed; no synthetic dependency | proposed app UI view-model/intents adapters only; no protocol/server changes | Q13, session reset/reconnect/missing projection, intent denial; G,P,W; W2; PH-HUD-REAL | L-COMPOSE; RB4 |
| P5-P layout/settings persistence | P5-F; parent scopes/storage evidence | bounded atomic/migration/downgrade/privacy cases pass | proposed app settings/UI persistence adapter, not ui-core filesystem | Q14, crash/size/future-schema/opt-out tests; G,P,W; W2; PH-PERSIST | L-COMPOSE; RB5 |
| P6-N physical native qualification | P5-F plus P2/P3/P4 integrated | complete applicable native matrix, real host IDs and truthful failures | allocated evidence report paths, no implementation files by default | all physical rows; G,H; W2; native fixture labelled | L-EVIDENCE/L-HOST; RB6 |
| P6-J real Tier-2 journeys | P5-R, applicable P5-P and P6-N | actual parent-required Tier-2 journeys, normal production contracts | separate evidence records, approved host/data scope | PH-TIER2; G,H,P; W2; real journeys | L-EVIDENCE/L-HOST; RB6 |
| P7-A viewport A/B | P6-N; identical population/world host; owner-approved experimental scope | reproducible measurements/all attempts; no policy inference from budgets and no server relevance, network or fairness truth claim | isolated experiment/evidence adapter; no server relevance mutation | paired aspect/no-crop/bar-hit/information-surface observations/Q15; G,H,R,W; W2; PH-AB; evidence is not server relevance/fairness proof | L-EVIDENCE/L-HOST/L-COMPOSE only if granted; RB6 |
| P7-D policy decision | P7-A sufficient evidence; applicable owner decision | fixed policy may be accepted when its evidence/owner gates pass; responsive may remain UNDECIDED or record only `RESPONSIVE_FOV_PREFERRED_PENDING_SERVER_RELEVANCE_PROOF` before the separate spike; terminal `VIEWPORT_RESPONSIVE_FOV_ACCEPTED_AFTER_SERVER_RELEVANCE_PROOF` requires successful separate server/network/fairness evidence, applicable independent review and owner decision | reviewed decision record only; separate relevance spike and implementation remain outside this slice | A/B evidence/decision review for fixed; separate server/network/fairness evidence plus applicable independent review and owner decision for final responsive; G; W0 | L-EVIDENCE; RB6 |
| P8-F one requested higher surface | P5-R; needed P6; accepted fixed P7-D disposition only if fixed-FOV-sensitive; `VIEWPORT_RESPONSIVE_FOV_ACCEPTED_AFTER_SERVER_RELEVANCE_PROOF` only if responsive-FOV-sensitive; no P7 wait if non-FOV-sensitive | named feature and its negative cases accepted, not generic framework; responsive-sensitive acceptance cannot use pending preference as final policy | exact app UI feature submodule and necessary ui-core primitive only | feature matrix plus Q01/Q03/Q13; G,P,W; W2; affected journey | L-COMPOSE/L-CORE; RB4 |
| P9-H hardening/recovery/performance | integrated relevant slices; can progress before FOV choice | bounded resource limits, cancellation, error taxonomy, loss and persistence recovery; measured budgets | separately leased app/renderer/UI files for one finding family per PR | Q01-Q19 applicable; soak/resource/latency; G,U,I,R,P,W,H as touched; W2 | relevant non-overlapping leases; RB2/RB3/RB5 |
| P9-R exact release qualification/rollout | P9-H, P6-J, selected P8; accepted fixed P7-D disposition for fixed-FOV-sensitive scope; `VIEWPORT_RESPONSIVE_FOV_ACCEPTED_AFTER_SERVER_RELEVANCE_PROOF` for responsive-FOV-sensitive scope; no P7 wait for non-FOV-sensitive scope | Tier-3 exact release, no test adapters; rollback exercised; explicit owner rollout admission; pending responsive preference cannot qualify responsive-sensitive release | release evidence and approved rollout configuration only | PH-TIER3 plus full selected suite; G,H,P,W; W2; exact release | L-EVIDENCE plus explicit release custody; RB4/RB5 |
| CP-A activate prompt pack | packet reviewed/protected; lifecycle/index lease released; explicit owner/control-plane grant | eight canonical prompts registered reusable, evaluation recorded, no new control plane | proposed `docs/agents/prompts/OTV2_SOL_NATIVE_UI_*.md`, lifecycle and prompt index only after lease | schema/alias/authority negatives, paired prompt canary; G,C; W0; no product evidence | L-REGISTRY; RB0 |
| CP-S Native UI semantic profile | independent control-plane allocation | genuine invariants/negative fixtures, explicit unsupported/manual checks, no cosmetic PASS | exact existing semantic-audit tools/tests/config selected after fresh source inventory | boundary/DAG/authority mutations, source witnesses; G,C; W0 unless implementation changes | L-CHECK; RB0 for spec; tested checker revert for implementation |
| CP-D reviewed docs-only routing | separately reviewed versioned consumer closure | only proven inert docs may skip heavy lanes; unknown/mixed/control-plane stays FULL | exact classifier/tests/workflow wiring under separate allocation | rename/delete/symlink/truncation/new-consumer/stale-base negatives; G,C; W0 | L-CHECK; fail back to FULL routing |

Forbidden paths for each slice are the complement of its granted exact owned files. In particular all slices except CP-D forbid workflow changes; CP-S may change only its specifically allocated semantic wiring, not global routing. CP-A alone may propose registry changes, never active coordinator allocation files. No slice obtains server, protocol, root Cargo or physical-host authority merely because it is useful. A module/package ceiling must be narrowed before a writer starts.

## 8. OWNERSHIP / WRITE-LEASE TABLE

| Lease | Surface / custodian | Serialization rule |
|---|---|---|
| L-CARGO | root Cargo.toml, Cargo.lock, workspace-boundaries.toml, affected manifests | currently #351/#356 custody; P1 waits explicit release and new exact grant; one resolver/manifest writer |
| L-CORE | proposed ui-core public exports and exact modules | P1 owner establishes seams; later P2/P4 distinct modules only; public API changes serialized |
| L-INPUT | exact input-actions/input-platform files | #558 protected integration is observed; verify old custody closeout and obtain a new exact P2 grant before writing; one shared router/value-export writer |
| L-RENDER | exact renderer resources/windows/draw exports | #558 protected integration is observed; new exact P3 grant still required; serialize P3/P4 resources and generation contracts |
| L-COMPOSE | apps/client entry/event loop/native UI wiring, proposed modules | one composition writer at a time; other workers hand off patches/contracts, not concurrent writes |
| L-CHECK | architecture/semantic/classifier tooling and allocated wiring | one independent control-plane allocation per change family; no CI optimization hidden in UI |
| L-REGISTRY | PROMPT_LIFECYCLE.json and relevant prompt/index files | preserve #536; activation only after fresh overlap resolution/protected allocation |
| L-EVIDENCE | unique proposed evidence report per candidate/run | sole report writer; immutable old evidence; independent reviewer does not author implementation |
| L-HOST | approved native hardware/window/capture resource | physical evidence runs serialized to prevent input/display contamination; does not authorize Remote Desktop |

A lease records exact files/symbols, owner, branch, admission main SHA, candidate head, dependencies, terminal state and explicit release event. Unpublished local state, an old status or idle time cannot transfer custody. At a missing lease, finish disjoint allowed work, then report `SHARED_LEASE_REQUIRED = path :: symbol/resource :: reason`. The current packet owns only its two new documents and no active implementation/control-plane state.

## 9. TEST / CI / PHYSICAL QUALIFICATION MATRIX

### 9.1 Intended lanes versus actual routing

`G`: existing governance/policy/metadata/dependency review/CodeQL and aggregate game-gate selected by the repository. `U`: pure Rust ui-core and dependency/boundary tests. `I`: input-actions/platform plus application arbitration tests. `R`: renderer/resource/lifecycle and host integration tests. `W`: Windows build/strict Clippy plus explicit affected-package tests. `P`: application/session/projection/product integration and impacted server/PG tests only as actually selected. `H`: approved physical evidence, not a GitHub-hosted-unit-test synonym. `C`: enforcement/classifier/prompt-policy tests and reviewed trusted-base selection.

| Change class | Intended risk-proportionate qualification | Current obligation |
|---|---|---|
| Proven inert architecture docs | G + document/contract/link checks | skip heavy lanes only through reviewed consumer closure; not assumed for this packet |
| Control-plane/enforcement/prompts | G+C and all impacted implementation lanes | fail-closed FULL when closure is not proved; Markdown extension is insufficient |
| ui-core pure Rust | G+U and target/feature closure incl. Windows | use current classifier, do not locally exempt unrelated required checks |
| Input | G+I+W; H for native acceptance | Linux alone does not qualify Windows event behavior |
| Renderer | G+R+W; H for physical truth | pure callbacks do not prove actual surface/GPU/display effects |
| Windows-native | G+affected packages+W+H | existing smoke exits before full native UI; require actual scenario |
| Product integration | G+P+I/R/W as impacted; H and Tier-2 journeys | no synthetic-only dependency and no replacement of real projection by fixture |
| Physical qualification | G for record plus H exact binaries/host; Tier 2/3 distinct | missing hardware/host is NOT_RUN, not PASS |

The current PR merge-gate has no blanket documentation exemption. BUILD_TEST_MATRIX records an existing merge-group architecture-path-only exception; that exception is not proof of reviewed document-consumer closure and is neither extended nor newly approved here. This packet includes agent instructions, so expect current trusted routing, possibly FULL Rust/Windows/PostgreSQL. Do not cancel/skip/disable lanes to make the planning PR cheaper. CP-D is the separate fix design.

CP-S design: select `NATIVE_CLIENT_UI` by a versioned contract manifest and actual owned source/document consumers. Validate admitted production graph, ui-core forbidden closure, sole composition-root ownership, normalized-value ownership, generation-before-effect call paths, stale identity/geometry fencing, stage edges/leases, evidence-tier requirements and the distinct fixed versus responsive FOV gates, including the no-A/B-shortcut prerequisite for responsive-sensitive P8/P9 acceptance. Structural facts use parsed metadata/source or executable negative fixtures. Semantic properties requiring runtime/physical/independent judgment remain explicitly NOT_VERIFIED until that evidence exists. A text keyword scan, empty check list or generic NOT_APPLICABLE must never become Native UI PASS. Reject unknown schema/consumer, missing witness or modified trusted selector; never add a second required AI gate.

CP-D design: version the complete document-consumer closure against exact protected base/head trees, not a capped changed-file API. Include scripts, workflow consumers, prompt/lifecycle dispatch, generated inputs, linked contract dependencies, path/type/mode changes and renamed/deleted files. Allowed inert-document families need positive proof and negative fixtures. Unknown consumers, overflow/truncation, mixed changes, symlinks, malformed metadata or stale base select FULL. Separate review of classifier implementation, fixtures and trusted workflow wiring; no workflow path-filter that suppresses required game-gate creation.

### 9.2 Automated negative-case catalogue

| ID | Required family / acceptance |
|---|---|
| Q01 | stale/remove/reuse focus, hover, capture, drag, tooltip, virtual row and async identities rejected |
| Q02 | bounds/depth/cycle/overflow/non-finite inputs denied before partial publication/allocation |
| Q03 | coordinate origin/DPI/user-scale/clip/rounding/half-open/zero-size coherence |
| Q04 | consumed release and modal transition still clear physical/router state |
| Q05 | blocked press cannot rearm until release plus fresh eligible press |
| Q06 | Global context cannot bypass modal/UI/gameplay authority |
| Q07 | both modifier sides, changed snapshots, repeat, device/focus/capture loss and duplicates |
| Q08 | UTF-8 preedit/range, exactly-once commit, cancel/focus loss, no duplicate text |
| Q09 | stale generation rejected before acquisition/submission/present/reconfiguration effects |
| Q10 | timeout/occluded/outdated/lost/suboptimal and present-then-reconfigure failure preserve truthful outcome |
| Q11 | skipped/old displayed frame cannot enable unseen new hit geometry |
| Q12 | painter order, resource identities, font/glyph caches, bounds/eviction/recovery |
| Q13 | delayed stale session/projection/node/resource results cannot mutate active UI or emit valid intents |
| Q14 | corrupted/oversize/future schema, crash, migration/downgrade and privacy scopes preserve safe bytes |
| Q15 | paired measured per-frame CPU sum, missing GPU timing, all attempts, display evidence truthfulness |
| Q16 | IME None/empty/range byte-boundary semantics and redacted diagnostics |
| Q17 | IME enable/disable/candidate area under DPI/user-scale/resize/focus transitions |
| Q18 | bounded line/pixel/fractional scroll, UI consumption and accumulator/reset negatives |
| Q19 | input-device versus surface versus GPU-device loss; bounded completion/terminal handling |

For authority-bearing application/persistence work also apply the task-template invariant x consumer-boundary x mutation-operator matrix: one broken invariant per negative, independent current source facts, and sibling/replay/restart/concurrency sweeps where applicable. A generated matching expected record is not independent negative authority evidence.

### 9.3 Physical evidence cells (future required, NOT_RUN by this packet)

Each record binds repo/base/exact head, binary/profile/hash, target, feature set, host OS/build, GPU/driver/backend, display/refresh/DPI/user scale, input/IME method, fixture or real world/session/projection identity, seed/content/HUD, steps, attempt order, warmup/sample policy, raw artifact hashes, result and failure/skip reason. Redact user text/tokens and sensitive session data. Missing required field means NOT_QUALIFIED. Do not publish private host credentials or private runtime metadata.

| Cell | Scenario and observable acceptance |
|---|---|
| PH-IME | native preedit/commit/cancel/replacement, Polish diacritics, focus/modal transitions; exactly one insertion and correctly placed candidate area |
| PH-SCROLL | real line/pixel/fractional input, UI over gameplay, capture/focus reset; finite magnitude and no hidden gameplay action |
| PH-GEOMETRY | 100/125/150/200% UI scale, DPI/monitor move, resize/minimize/restore/zero-size; displayed and hit geometry agree |
| PH-INPUT | capture release/loss, keyboard focus, device reset/loss, held press and left/right modifiers; no stuck action or magical rearm |
| PH-FRAME | stale generations, surface lost/outdated/suboptimal, acquired-frame disposal before configure; no denied-generation physical effects |
| PH-DEVICE | real supported GPU loss scenario or explicit terminal policy; bounded callbacks/progress; surface-only test is not GPU recovery |
| PH-TEXT | supported shaping/fallback/diacritics, font scale, clipping and cache pressure; measure/draw consistency |
| PH-DISPLAY | distinguish present request, GPU completion and actual display; input-to-display requires a stated external observation method, not CPU submit latency |
| PH-PERSIST | interrupted write, corruption, bounded migration and downgrade/future schema; safe restore and unchanged recoverable bytes |
| PH-HUD-FIXTURE | all baseline surfaces on real native window/GPU with labelled fixture data; useful component evidence, NOT Tier 2 |
| PH-HUD-REAL | actual application session/projection and validated intents, reconnect/stale projection/missing authority; no simulated product authority |
| PH-AB | same world/session/content/HUD, zoom and workload; 1280x720, 1920x1080, 2560x1440, 3440x1440, 3840x2160 plus resize; centered uniform fixed-B/no crop, fair population |
| PH-TIER2 | parent-required real journeys through normal production contracts; explicitly identify any bounded test adapter; synthetic component fixture is insufficient |
| PH-TIER3 | exact release artifact with no test adapters, required real journeys/recovery/rollback and production settings; earlier release-build smoke is insufficient |

Input-to-display instrumentation/host remains UNKNOWN until an approved actual method is demonstrated. GPU timing may be UNSUPPORTED/NOT_MEASURED, never zero or CPU-derived. The inherited fixture engineering target is p95 of each frame's update+layout+extract SUM <=1 ms, not sum of separate p95 values and not a product SLO. Define sampling and hardware before claiming that target met. No new numeric resource limits override #502 or the protected registry.

## 10. DECISION REGISTER

Statuses below are candidate recommendations unless already preserved from protected architecture. Each row answers timing, blocked slice, coupling, replacement evidence and deliberately undecided scope; no unsupported library choice is made.

| Decision | Status | Decide now / blocked slice | Coupling and required evidence / supersession | Deliberately not decided |
|---|---|---|---|---|
| Retained tree semantics and lifetime generation | DECIDED_NOW | semantic contract needed for P1 | prevents stale identity; exact storage chosen by measured bounded implementation; replace only reviewed equivalent lifetime proof | arena/library/concrete indexing |
| Text shaping/library | EVIDENCE_REQUIRED_BEFORE_SLICE | P4-T | Polish/Unicode, measure/draw, license, normal/build/target closure and cache/performance prototype | library/font stack before evidence |
| Shader representation/features | EVIDENCE_REQUIRED_BEFORE_SLICE | P3-D | pinned wgpu features, Windows compilation and pipeline prototype; lease required for feature changes | WGSL/alternative until validated |
| UI scale transform | DECIDED_NOW | P1 geometry | separate DPI/user scale and coherent epoch; physical Q03 may require reviewed refinement | product default scale and final visual dimensions |
| UI scale product defaults | EVIDENCE_REQUIRED_BEFORE_PRODUCT_ROLLOUT | P9-R | readable layout/native DPI matrix and owner UX acceptance | final default preference |
| Scroll accumulation | EVIDENCE_REQUIRED_BEFORE_SLICE | P2-S | real devices, bounded units/fractions/reset negatives; unit preservation already required | whether accumulation is needed and numeric bounds |
| IME platform behavior | EVIDENCE_REQUIRED_BEFORE_SLICE | P2-I native integration | pinned platform behavior, UTF-8 semantics, focus/candidate experiments and privacy | untested platform quirks/host behavior |
| Persistence storage format | EVIDENCE_REQUIRED_BEFORE_SLICE | P5-P | parent scopes, atomicity/corruption/migration/downgrade proof; avoid independent authority | encoding/library/migration implementation |
| Resource/cache limits | EVIDENCE_REQUIRED_BEFORE_SLICE | each allocating P1/P3/P4 slice | inherit #502/registry; bound before allocation, measure native cells, reviewed values only | new numeric limits/FOV inference |
| GPU timing mechanism | EVIDENCE_REQUIRED_BEFORE_SLICE | P7-A performance cells | actual backend capabilities, calibrated validity; unsupported values explicit | invented CPU-to-GPU substitute |
| Physical evidence host | EVIDENCE_REQUIRED_BEFORE_SLICE | P3-H/P6-N | approved Windows/native world+UI binary and host access/instrumentation | unavailable host asserted executable |
| Responsive FOV policy | OWNER_DECISION_REQUIRED | responsive-FOV-sensitive P8/P9 only | P7-A can support only UNDECIDED or `RESPONSIVE_FOV_PREFERRED_PENDING_SERVER_RELEVANCE_PROOF`; terminal `VIEWPORT_RESPONSIVE_FOV_ACCEPTED_AFTER_SERVER_RELEVANCE_PROOF` requires successful separate server/network/fairness relevance evidence, applicable independent review and owner decision | final visibility/relevance policy before all terminal prerequisites |
| Fixed FOV policy | OWNER_DECISION_REQUIRED | fixed-FOV-sensitive P8/P9 only | P7-D may accept fixed policy when comparable P7-A evidence and applicable review/owner gates are satisfied; fixed-B experiment remains no-crop/uniform/centered | final tile count/visibility choice |
| Addon/mod/plugin architecture | DEFERRED | no baseline slice | separate product requirement, threat/trust boundaries and reviewed architecture | plugin ABI, scripting/runtime ecosystem |

## 17. OPEN BLOCKERS

1. Root Cargo/workspace custody remains with #351/#356 until explicit release. P1 needs its own exact protected allocation; protected #558 alone does not admit it.
2. This packet requires independent review and normal protected publication. Its author checks are not independent KEEP; no active P1 allocation is created here.
3. New aliases require CP-A, current lifecycle/index lease resolution and actual prompt evaluation; no workers have been launched by this packet.
4. P1 external dependency-neutrality enforcement, native world/UI host, real projection admission, shaping/shader/persistence decisions and hardware/display instrumentation are not supplied by documentation. They gate their named slices, not all planning.
5. Fixed and responsive FOV remain distinct owner/evidence gates only for their sensitive work. Responsive terminal acceptance additionally awaits the separate server/network/fairness relevance spike, applicable independent review and owner decision; an A/B preference remains pending and cannot qualify responsive-sensitive P8/P9 acceptance. Non-FOV-sensitive P8/P9 work remains parallel-safe. Addons stay deferred. Prompt canaries, native physical cells and Tier-2/Tier-3 product acceptance are NOT_RUN here.

## 18. RECOMMENDED NEXT ACTION

Highest-value action: obtain independent exact-head review of this completed planning candidate through the existing control plane. #558 is already protected-integrated; do not reopen it. P1 may start only after this architecture is adopted and the existing coordinator resolves the still-held Cargo/workspace lease and grants the exact P1 allocation. This planning author must not create that grant or enqueue/merge the candidate.

Planning terminal classification: `NATIVE_UI_PROGRAMME_PLAN_READY_WITH_EVIDENCE_GATES`. This classifies the authored planning packet, not independent review, active aliases, runtime readiness or protected integration.

### Embedded task record (task-template fields; documentation-only candidate)

```yaml
task_id: OTV2-20260911-native-ui-implementation-programme
title: Native UI architecture, delivery and agent candidate pack
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
base_sha: fc0ecb064b1d4a23a87dbba8070eb91a5142be03
branch: docs/native-ui-programme-v1-20260911
owner: native-ui-planning-author
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/OTERYN_NATIVE_CLIENT_UI_IMPLEMENTATION_PROGRAMME_V1.md
  - docs/agents/programs/OTERYN_NATIVE_UI_AGENT_PROGRAMME_V1.md
implementation_authority: NONE
runtime_activation_authority: NONE
allocation_state: NOT_ACTIVE
alias_dispatch_state: NOT_REGISTERED
independent_review: REQUIRED_NOT_EVIDENCED
physical_qualification: NOT_RUN
prompt_behavioral_evaluation: NOT_RUN
```

Outcome: one reviewable two-document packet; acceptance requires full architecture/DAG/slices/qualification/decisions and eight prompt definitions with safe launch/resume procedures. Excluded: implementation, active control-plane mutation, registry activation, #558 edits, workflows, protection, deployment and merge.

High-risk authority/recovery execution matrix: NOT_APPLICABLE to this candidate publication because it installs no controller and performs no production/session/lease/recovery mutation. Future CP-A and authority-bearing P2/P5/P9 implementation must assess the actual matrix before material freeze; documenting that duty is not a waiver.

Validation/closeout: local document checks, final commit/head, hosted CI, author self-review and any independent disposition belong in exact-head PR/check evidence after the commit exists. Do not create a self-referential/no-op commit to insert its own SHA or copy later CI status. Local source-subset checks are not a complete checkout build. Repository CI must run its actual selected lanes. Rollback this packet by a normal reviewed revert of its two new documents, preserving all historical corrections and existing allocations.
