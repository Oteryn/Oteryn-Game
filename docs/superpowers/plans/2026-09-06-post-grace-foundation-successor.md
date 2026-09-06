# Post-grace Foundation Successor Implementation Plan

**Admitted exact allocation, Issue #338 / PR343.** Canonical branch: `agent/post-grace-foundation-successor-338`; immutable protected admission `4f35ec5a56f5e8b0c32db4503d2bd3503b8828ee` under Work5558312039.

**Goal:** Implement accepted post-grace recovery timing as an additive Foundation semantic successor with verified reauthentication, independently current actor/continuity authority, immutable attempt timing and generation-one replacement. Preserve V1/V2 and B329 independence.

**Accepted authority:** FND-DUR-POST-GRACE-TIMING-V1, #332/#334, protected merge `1bcdc951e90a56310d24dfb5f3953ec0f86e1695`, Merge Queue `34022052840` PASS; #332 closed completed 2026-09-06T08:39:48Z verified by Work. This is architecture acceptance, not worker admission. Preserve its parent FND04B/recovery-grant/reconnect/terminal-replacement contracts and #326 owner-claim semantics. Immutable admission remains `4f35ec5a56f5e8b0c32db4503d2bd3503b8828ee`; later upstream advances do not reset admission.

## Exact allocated paths after protected integration

| Path | Responsibility |
|---|---|
| `apps/game-server/src/foundation/admission_recovery_inner.rs` | Closed timing/record/request/flow successor; current actor and retained continuity sources; immutable deadline, generation and replay semantics |
| `apps/game-server/src/foundation/fnd04_verifier.rs` | Additive sealed authenticated recovery evidence/verification successor; preserve existing verifier interfaces/classification |
| `apps/game-server/src/foundation/admission_authority_publication.rs` | Additive owner-sealed successor claim transition bound to exact new recovery operation |
| `apps/game-server/src/foundation/post_grace_recovery_tests.rs` | New independent source/mutation and direct/reconciled semantic tests |
| `docs/agents/tasks/active/OTV2-20260906-post-grace-foundation-successor-338.md` | Technical checkpoint/evidence |
| `docs/superpowers/plans/2026-09-06-post-grace-foundation-successor.md` | This implementation plan |

No facade/export/lib edits are currently demonstrated necessary: recovery declarations are included in `admission`, re-exported by existing `pub use admission::*`; verifier/publication modules are public. Include new tests from an owned source file. Keep the synchronous compatibility facade unchanged. No SQL/schema/migration/Cargo/workflow/source-provider/production/Server Seam writes.

## Execution/custody

One writer, one branch/worktree, one 60-minute execution window. Shared semantic types require serial mutation; independent read-only review may run in parallel. Technical milestones below do not grant multiple hours. At the window boundary checkpoint the exact durable head and one next action; Work controls continuation/rotation. Preserve admission, history, frozen candidate and cumulative failure/repair/CI counters; no reset through worker rotation or metadata-only commits. GitHub lifecycle and canonical gates remain authoritative.

## Milestone 1 — sealed verified recovery and current actor prerequisites

- [x] RED: caller-filled history/flag or reconnect proof cannot select live post-grace recovery; missing registered evidence/current actor source fails closed.
- [x] Add an opt-in recovery-scoped sealed source and private verified successor carrying verified-at, credential timing, authenticated security/trust provenance and deadlines/uncertainty. Existing `Fnd04EvidenceAuthority`, `RecoveryTrustContext`, `verify_recovery_grant*` behavior remains intact; no required new methods on existing external implementers.
- [x] Authenticate scope/key/profile and signed bindings with existing classification. Existing V1 verified facts lack source provenance/deadlines: do not fill those from compatibility DTOs or FreshAdmission-scoped evidence. Preserve all applicable accepted credential bounds using checked arithmetic; introduce no new duration.
- [x] Add sealed owning current actor/continuity resolution: exact terminal predecessor, same present uncontrolled actor and placement, account/character/world, current lease/runtime/revisions, no controller, retained original epoch/grace, protection and complete retained budget. Raw DTOs are observations/history, not source registration.
- [x] Tests independently control these sources; changed actor/presence/placement/fences never derives its allegedly current value from the candidate record.

## Milestone 2 — closed versioned timing and immutable operation

- [x] Add a mandatory closed successor timing discriminator: SameSession retaining old semantics; TerminalSessionPostGrace requiring newly verified recovery plus current owning facts. Unknown/missing versions reject; no automatic upgrade or default conversion from V1.
- [x] Preserve `ReconnectContinuityV1` prepared<=original-grace and `ReconnectConnectionFenceV1` candidate=predecessor+1. New-session initial generation is exactly 1 in the successor, with predecessor generation stored separately; never force it through V1 by inventing zero/predecessor history.
- [x] At eligibility require trusted now strictly greater than exact historical predecessor grace. Equality remains governed by existing paths; pre-grace terminal replacement remains V1/V2.
- [x] Freeze finite attempt deadline as the minimum of accepted credential and security/trust bounds, optionally a separately accepted shorter attempt bound. Exclude historical grace from the new variant's upper-bound minimum, but require deadline>historical grace and usable at preparation. Overflow/unprovable/expired bounds reject.
- [x] Private live authorization produces the immutable operation/request; historical representation retains variant, deadline, source evidence, predecessor/candidate/attempt/transport and exact actor/epoch bindings. Historical restoration never produces live PREPARE. Same attempt cannot change deadline or variant after refreshed evidence or ambiguity.

## Milestone 3 — PREPARE, final authorization and independent adoption

- [x] Implement additive split-phase flow/request/completion family and bounded semantic persistence port. No SQL, waits or alternate synchronous production route.
- [x] Provide bounded pure adapter predicates over independently current supplied source/session facts and trusted time for PREPARE and final COMMIT authorization. Both require post-grace ordering and the frozen deadline; later deny/restriction/controller/actor/fence changes reject. PREPARE is not authorization escrow.
- [x] Normalized final revalidation requires authenticated current recovery/security/trust evidence, not a stored proof enum. Later observations may restrict but never extend the immutable attempt deadline. Keep operation deadline separate from any stricter final authorization bound.
- [x] Typed completion/reconcile matches exact original operation; known failure retains terminal disposition, ambiguous outcome reconciles before a new candidate. Historical commit can classify success without granting current authority.
- [x] Adoption independently verifies exact current candidate generation1/controller/physical transport binding, current actor/placement/lease/scope/revisions/security/trust and absence of supersession. Never reinstall from receipt-only facts.

## Milestone 4 — retained epoch, budget, protection and sealed claims

- [x] Add validated retained-budget evidence/restoration, bound to actor/epoch and preserved attempt identities/dispositions/high-water with explicit completeness. Do not use `ReconnectAttemptBudgetV1::new(epoch)` to infer empty restarted state. Missing or compacted unprovable state closes admission.
- [x] Preserve eight-distinct-attempt limit across sessions; same-attempt retry consumes no new slot; old terminal attempts stay terminal, old prepared candidates remain noncommittable after predecessor terminality. No new epoch just to recover.
- [x] Preserve entitlement and consumption/activation/rearm evidence, not merely a convenient unused/fenced enum. No minting/reset/rearm; existing eligible unused entitlement follows accepted once-only activation. Actor retirement and successful restoration preserve existing epoch finality; later real control loss uses existing rules.
- [x] Add a separate successor owner-sealed claim capability/evidence that binds exact new record, independently current claims/session, source/CAS and effect identity. Preserve existing lifecycle enum variants/public signatures consumed by B; do not add a new variant to an exhaustively matched existing enum by convenience.
- [x] Claim effects remain inert until the matching canonical replacement transaction. Ordinary publication cannot apply them. Pure locked validators expose exact owner-authored effects for later SQL child; no source provenance invention.

## Milestone 5 — acceptance matrix and compatibility

Apply AuthorityInvariant x ConsumerBoundary x MutationOperator; each negative changes one invariant with a valid independently sourced control.

| Family | Mandatory semantic evidence |
|---|---|
| Positive | Newly verified recovery after grace; old terminal/session distinct; same uncontrolled actor/placement; generation1; unchanged lease/epoch/protection; direct and reconciled restoration |
| Timing | Equality unchanged; same-session after grace rejected; wrong historical grace; frozen variant/deadline replay conflict; queue-delay simulation; overflow/missing/future/nonmonotonic/source-expired bounds; refresh cannot extend |
| Auth/source | Fast reconnect proof, raw DTO/flag, unregistered source, wrong scope/key/profile, missing provenance, accepted-source rollback/contradiction and deny rejected; compile-fail historical-to-live |
| Current authority | Active/reconnectable predecessor, healthy controller, absent/different actor, wrong placement/account/character/world/lease/scope/generation/revision; independently rechecked at preparation/final/adoption |
| Continuity | Attempts 8/9 across sessions, exact retry, retained collision/terminal outcomes, missing budget/protection evidence, consumed entitlement unchanged, no epoch/reset or stale prepared COMMIT |
| Recovery | Ambiguous original binding retained; historical-only restore; post-success replacement/terminality/actor disappearance prevents adoption; no heal/respawn/relocation semantics |
| Claims | Stale owner predecessor/CAS/holder, wrong successor source/effects, ordinary-publication bypass, lifecycle history forgery; exact accepted successor floors |
| Compatibility | V1/V2 signatures/behavior and early-terminal regressions; unknown/missing successor version; no V1 downcast; B source-included target compiles unchanged |

## Final qualification and handoff

- [ ] Focused RED/GREEN, Foundation/game-server tests and compile-fail doctests; fmt, strict all-target Clippy, architecture/task governance and full selected canonical checks.
- [ ] Compile `cargo +1.94.0 test --locked -p oteryn-game-server --test durability_postgres --no-run` against the actual B-integrated source when available. Before B integration use current protected target and record that later merge-up requires actual source-inclusion compile proof. Do not edit B's worktree or claim its uncommitted compile compatibility.
- [ ] Whole-diff adversarial review and genuinely independent exact-head high-risk review; material findings dispositioned and sibling sweeps complete. Normal CI/Merge Queue/protected readback required.
- [ ] Return exact successor API/history/current-source contract and semantic evidence to Work. No actual PostgreSQL post-grace proof or production provider readiness is claimed. Later SQL child must serialize with B, inspect its released schema and obtain a fresh explicit forward migration allocation; select no migration number here.
- [ ] Preserve old readers/writers/records; later SQL rollout enables the new path only after supported reader/writer/version compatibility. Rollback disables new attempts and retains supported historical reconciliation, never deletes/downcasts history or revives predecessor.

**Single current next action:** Work publishes/reviews the window3 material checkpoint on PR343, then returns sole-writer custody for remaining exact-source qualification and any separately protected amendment. No raw history or successful local test is production source registration.

### Window3 implementation readback

Work5559305170 resolves the bounded representation question without changing the common V1 guard: unchanged nested Fresh historical provenance is carried alongside independent Recovery authorization/current shared floor. The complete claim evidence stores immutable original admission operation and separate immutable claim-time audit. The positive originalRecoveryN / retainedFreshN+1 / currentRecoveryN+2 matrix has actual RED/GREEN evidence; final decisions chain from the claim-time audit without rewriting the original operation.

Private split flow, sealed durable completion, typed terminal outcomes, historical-only reconcile, freshly reauthorized PREPARED restart, and common direct/reconciled current adoption are implemented. Adoption requires exact current committed claim successors and clears projection on every failed current fence. It can occur after original credential expiry only with independently current scoped Recovery/trust and original committed decision time. The retained restored epoch contains exactly one committed candidate attempt and preserves prior terminal/collision/attempt histories; eligible protection activates once at original commit time, without reset/rearm.

Checkpoint evidence:30 focused tests, full locked package library320/integration targets/18 doctests, strict all-target Clippy and fmt/governance. PostgreSQL tests are unconfigured and prove compilation only. Independent exact-head review, canonical selected CI/MQ, actual B-integrated source compile and Work integration remain required. The accepted unexpected-control-loss policy may need a separately protected additive implementation amendment; this checkpoint has not implemented that future amendment.


### Mandatory semantic matrix qualification after checkpoint4

Milestones1–4 are implemented and locally qualified by the following independently sourced controls; checked items do not assert SQL or registered production owners.

| Obligation | Direct evidence in post_grace_recovery_tests.rs |
|---|---|
| Recovery scope, authenticated profile/bindings and source expiry/uncertainty | `post_grace_recovery_source_deadline_is_scoped_conservative_and_checked`, signed credential matrix (13 mutations), source replay/denial/generation tests |
| Independent actor, canonical predecessor and original finite deadline | current actor/FND02/timing tests; PREPARE/final locked matrix runs12 actor,8 source,8 canonical changes and missing/stale claims, unavailable source and queue expiry at both boundaries |
| Original operation/history cannot become current authority | immutable-operation/unknown-version tests, sealed-source/claim/receipt compile-fail doctests; historical restore cannot prepare or commit before sealed PREPARED reconciliation and fresh authorization |
| Retained budget and protection span replacement | complete8-entry mixed terminal/collision/own-prepared history commits and adopts; previously consumed protection remains unchanged; dropping one retained entry rejects; existing8/9, retry and final-epoch tests |
| Claim ownership and selected common floor | stale Fresh history remains unchanged; independent Recovery7/Fresh8/currentRecovery9 positive and current-purpose rollback/substitution/CAS/holder negatives |
| Split final outcome and independent current adoption | direct and reconciled positives, absent/mismatched/ambiguous/typed terminal results, late adoption with newly current Recovery,16 current-fence changes and canonical candidate initial-origin negative |

The signed time matrix initially tried nbf101 at now100; existing accepted NumericDate tolerance is+5, so the fixture was corrected to nbf106. This was an incorrect test expectation, not a runtime repair or a changed accepted bound. Known runtime repair counter remains3 plus priorUNKNOWN.

Checkpoint4 published `f2ccc7de74ed58bd47c7c71c0d1e62c8a5fce331` passed independent/root review (Work3435559531198) and canonical selected CI34036149928, Linux101494526536/game101495093630, governance34036149942 and semantic34036149966 (Work3435559562859). Expanded matrix/docs after that head require fresh exact-content publication/review and affected selected gates; no carried-forward exact-head claim.

Final local checkpoint5 evidence: full locked package library324 and20 doctests,34 post_grace family cases; strict all-target Clippy6.05s, fmt/diff/governance PASS. Explicit current-checkout durability_postgres --no-run PASS. Exact completion-operation/transport/attempt/claim/deadline and mixed SameSession timing negatives also pass. Later actual B-integrated source still requires its own compile after normal merge-up.


Checkpoint5 review disposition: accepted P2 canonical-negative masking repaired. Both locked boundaries now first accept an unchanged canonical observation with coherent new actor revision12/accepted12/decision/time101, then reject each single canonical mutation using that valid refreshed baseline. The original unchanged revision11 control remains, and same-revision contradiction has its own separate tests. Focused repaired matrix passes; runtime unchanged. Known cumulative repairs4, priorUNKNOWN retained. Prior full-package evidence remains explicitly before this test-only repair.
