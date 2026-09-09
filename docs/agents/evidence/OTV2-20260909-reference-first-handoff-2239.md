# OTV2 Reference-first continuation handoff — 2026-09-09 22:39 CEST

Status: `HISTORICAL_CONTINUATION_EVIDENCE_ONLY`

This handoff preserves work coordinates for the next agent. It grants **no** write, allocation, review, merge, production, external-repository, or live-environment authority. GitHub LIVE state, current protected `main`, current repository instructions, and accepted architecture/contracts always outrank this file. Re-read the listed live locators before any material action.

## Capture point

- Repository: `Oteryn/Oteryn-Game`
- Protected main observed at handoff creation: `247818666bbbcb13fa27e24ec90fee24c793a416`
- Parent allocation/control plane: #162
- Global Reference programme: #486
- Official-first evidence lane: #483
- Immutable first external target: `global-tibia-observable-2026-07-28-post-server-save`

## Binding Reference rule

The delivery order remains:

1. observable Global Tibia target;
2. evidence at its actual strength;
3. Oteryn Reference composition;
4. Oteryn Evolved only after an explicit later activation decision.

Use official Tibia/CipSoft evidence first. Current official pages may support discovery or continuity, but do not automatically prove the 2026-07-28 target cut. Canary, Crystal and other OTS repositories remain `OTS_HYPOTHESIS_ONLY`: useful for edge-case/test discovery, never sufficient to prove Global behavior or values. `UNKNOWN` / `CONFLICT` stays fail-closed. Do not fill gaps from current Global, OTS convenience values, or Evolved design.

## Current control-plane / ownership boundary

- #162 remains the sole current Game product allocation/integration control plane.
- #486 is a coordination programme, not a second scheduler or writer authority.
- This handoff creates no worker, branch lease, product-path ownership, or implementation admission.
- Existing canonical workers/PRs must be resumed rather than replaced. Before any write, refresh current open PRs/branches/path ownership and applicable nearest `AGENTS.md`.
- Read-only evidence/readiness work may continue only where it remains live-state path-disjoint.
- Server Seam #247 and the WP dependency chain remain the production admission/composition dependency; independent Reference evidence/architecture preparation must not claim production Reference readiness around that chain.

## Canonical active gates at capture

### R2 CONTENT / first-production closure

- #54 is open/reopened. First-production CONTENT is not terminally accepted.
- #490 is the canonical post-merge P1 repair tracker after #481.
- #492 is the **same sole repair lineage**, still Draft. Live branch head at capture: `7a0d3ecf1f618ddce2571a8f15c36b8719a3b8ee`.
  - P1-1: controller/catalog ownership repair is implemented on that lineage.
  - P1-2: canonical cross-boundary `WorldId` remains held on architecture protection.
- #494 is the architecture decision selecting `crate::foundation::WorldId` as the canonical Game-side cross-boundary Rust representation. Exact head at capture: `e7cdf36d1a9e5dbef580bd7169f128e1c5e8da0c`; Agent Governance, Architecture Semantic Audit and Merge Gate are green. It is still open and is **not** protected-main integration evidence.
- #504 is the `REFERENCE_PLAYABLE_CONTENT_PROFILE/v1` successor gate. Do not mutate #54/#492 into this successor. #504 remains gated by terminal first-production repair/ownership release plus its own world/evidence/resource dependencies.

### R4 Ability / R6 Creature AI

- #508 is the canonical production targeting successor gate.
- Phase A is `ABILITY_EXACT_ACTOR_RESOLUTION_V1`: one server-authoritative semantic actor reference/generation lookup, no geometry scan.
- Phase B later adds only the evidenced range/LoS/floor/geometry legality required by the selected Reference mechanic and reopens the corresponding bounded geometry resource work.
- Never bridge fixture identities by convention such as `CandidateId(u64) -> String -> TargetId(String)`. Player, AI and other permitted origins must converge on the same authoritative resolver/legality path.
- Do not create a second actor identity, resolver authority, or spatial index if another accepted owner introduces the required seam first.

### R5 Combat / death / loot / XP

- #506 is the resource/architecture gate for `REFERENCE_COMBAT_DEATH_WORKFLOW_V1`.
- Preserve accepted ownership: Ability owns committed effects; runtime owns the one death occurrence/corpse projection; DUR-03 owns durable item/value materialization and custody; Character/DUR-02 owns persistent XP/progression.
- Loot and XP are separate descendant workflows, not one distributed transaction.
- #506 must not implement replacement Ability, Character progression, or DUR-03 transaction engines.

### R7 Character progression / item transactions

- #507 defines `REFERENCE_CHARACTER_PROGRESSION_APPLY_V1`; Character owns progression application and DUR-02 remains physical persistence owner. Exact target formulas/rounding remain evidence-gated.
- Existing accepted Oteryn Reference death differences remain explicit overlays, not guessed Global truth: `DeathXPBasis = LevelXPSpan(current_level)`, `DeathSkillLoss = 0`, `DeathMagicLevelLoss = 0`. Any exercised Global modifier/rounding still requires admissible evidence.
- #513 is the shared DUR-03 resource gate for first Reference item transactions:
  - `REFERENCE_LOOT_MATERIALIZE_V1`;
  - `REFERENCE_ITEM_PICKUP_V1`.
- Close DUR03 transaction resource dimensions once at the Durability owner. Combat, Item and NPC consumers must not invent their own transaction budgets or duplicate durable value authority.

### R8 NPC / service boundary

- PR #500 defines the first Reference NPC/service boundary on exact head `85e7bda6bb2d4c23a33c1284c5e98da6c502153e`.
- Agent Governance, Architecture Semantic Audit and Merge Gate are green at capture; no submitted PR review was present at the capture readback. The PR remains open and therefore is not protected architecture yet.
- Selected shape is a logical `GAME-NPC-SERVICE` role inside current runtime ownership, not a new process/business authority.
- First non-mutating trader child is `NPC_DIALOGUE_TRADE_WIDGET_V1`; later `NPC_SINGLE_TRADE_COMMIT_V1` must consume the proper GAME-ITEM / DUR-03 / value owner rather than making NPC or client authoritative.

## Work already prepared but not allocation authority

The prior Reference preparation established useful minimal successor shapes that must be reconciled with current live gates rather than restarted:

- movement: one local cardinal step + static collision was the deliberately minimal R3 component idea; no diagonal/speed/discovery/occupancy/visibility/Interaction/pathfinding was implied;
- creature AI: stationary retaliation was deliberately bounded to acquisition -> at most one transient Ability intent, without Movement/timers/memory/spawn/backlog ownership;
- R4: one representative offensive and one healing ability remain the intended minimal Reference proof shape, but exact values/formulas/geometry must come from #483 evidence and current Content/targeting gates;
- R5/R7: deterministic creature death, XP, corpse, durable loot materialization and pickup have been decomposed so ownership does not collapse into one generic gameplay/economy worker;
- R8: NPC dialogue/service is separated from item/value mutation;
- world/client/presentation work may continue only under its live path-disjoint authority and cannot move gameplay truth client-side.

These are reconstruction coordinates only. If current accepted architecture or live ownership has advanced, consume the newer state instead of preserving these shapes mechanically.

## Safe continuation sequence

1. Fresh-read protected `main`, root/nearest instructions, #162, #483, #486, #54/#490/#492/#494, #500, and #504/#506/#507/#508/#513. Refresh active PR/path ownership before every mutation.
2. If #494 has become protected-integrated, resume the **same #492 repair lineage** for the exact authorized P1-2 consequence only. Do not create another CONTENT branch/worker.
3. Drive #492 through its remaining focused validation, required independent exact-head review, canonical CI, FULL Merge Queue and protected-main readback. Close/release #490/#54 only when their current live acceptance is truly terminal.
4. Only after overlapping first-production CONTENT ownership is released may #162 allocate #504 / Reference CONTENT implementation or the dependent R4 Content-to-Ability slice.
5. While mutation is dependency-held, continue only legal path-disjoint official-first evidence/resource/architecture work. Prefer closing the smallest live gate rather than broadening scope.
6. R4/R6 must consume #508 Phase A before production-shaped target execution; geometry comes later under evidence/resource bounds.
7. R5 loot/pickup must consume #513; XP/progression must consume #507; #506 composes those owners rather than replacing them.
8. R8 runtime work waits for protected NPC-service architecture and a fresh #162 allocation; mutating trade waits for the proper Durability/value transaction child.
9. Do not claim `REFERENCE_PLAYABLE` from local component success. The physical native journey still needs the real Server Seam/WP admission chain plus client/world/gameplay/durability composition.
10. Keep Evolved closed until a reviewed Reference readiness checkpoint and explicit later activation decision.

## Refresh-on-resume rule

Every SHA/status in this handoff is a capture coordinate, not current authority. If any referenced main/PR/Issue/head/ownership/blocker/gate has changed, treat that field as expired and rebuild the affected step from GitHub LIVE state without discarding unrelated valid work.