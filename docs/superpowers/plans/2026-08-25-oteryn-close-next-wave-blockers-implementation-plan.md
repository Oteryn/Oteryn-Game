# Close Next-Wave Blockers Implementation Plan

> Governing authorization: Issue #128 and `docs/agents/prompts/OTV2_CLOSE_NEXT_WAVE_BLOCKERS.md`.

**Goal:** terminally close Oteryn-Game blockers #93, #115, #116 and #123, merge every required bounded change, and recompute lawful next-wave allocations without starting gameplay, listener, durability or client implementation.

**Architecture:** one coordinator issue (#131) owns decomposition only. Evidence-backed first-slice decisions are reviewed separately from the serialized resource registry. The FND-04 trust boundary is implemented in a dedicated Foundation PR with TDD and a fresh non-authoring exact-head security review. Terminal readiness/ownership reconciliation is a final docs-only PR.

**Technology:** Rust 1.94, repository Python governance tools, GitHub Actions `game-gate`, local Ollama non-authoring review (`qwen2.5-coder:14b`), JSON resource registry.

---

## Task 1: Establish coordinator allocation and durable checkpoint

**Files:**
- Create: `docs/agents/tasks/active/OTV2-20260825-close-next-wave-blockers.md`
- Create: `docs/superpowers/plans/2026-08-25-oteryn-close-next-wave-blockers-implementation-plan.md`
- Modify: `docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md`

1. Record Issue #131, base `9cc23cdbfe68d0a0f13df054874929b5e5dbe418`, branch ownership and a 120-minute foreground budget justified by four blocker lifecycles.
2. Allocate only the three coordinator paths above; do not pre-lease registry/Cargo/code paths.
3. Run `python tools/agents/validate_governance.py` and `git diff --check`.
4. Whole-diff self-review, push, open one docs-only PR, require exact-head governance/architecture/merge-authority/merge-gate and `game-gate`, squash-merge with expected-head protection.

## Task 2: Produce executable evidence and first-slice decisions for #93, #116 and #123

**Files:**
- Create: `tools/next-wave-limit-evidence/main.rs`
- Create: `tools/next-wave-limit-evidence/README.md`
- Create: `docs/agents/evidence/OTV2-20260825-next-wave-limit-evidence.json`
- Create: `docs/architecture/reviews/OTERYN_GAME_NEXT_WAVE_FIRST_SLICE_LIMITS_DECISION_2026-08-25.md`
- Create: one active task record, later archived by its own closeout PR

1. Freeze exact non-shipping first slices:
   - Ability: one explicit target, one immediate typed damage/heal occurrence, bounded staged plan; no area/chain/retarget/future/conditions/reactions/cross-domain/script work.
   - Interaction: one root, immediate bounded children only; no grandchildren, foreign delegated operations or automatic retries.
   - AI: fixed compiled acquire-or-idle representation, bounded perception and one path proposal per actor; no spawn/timers/memory/repath/controlled backlog/scripts.
   - Durability: Foundation authority-journal/reconciliation receipt substrate only; item/value/transform/container/workflow cardinalities excluded fail-closed.
   - Listener: TCP/TLS admission/backpressure/drain resource profile only; no socket bind, port, certificate, key, deployment topology or listener code.
2. Write boundary tests first in `main.rs` for every candidate maximum and `max+1` rejection. Compile/run and record expected RED because the decision model is absent.
3. Implement the smallest checked accounting model, rerun GREEN, then run optimized deterministic stress fixtures and emit the evidence JSON.
4. Accept only conservative safety ceilings supported by the executable model and explicit cost equations. State that they are not product balance, production sizing or Reference parity.
5. For every inventory row, record `REGISTERED_CANDIDATE` or `NOT_APPLICABLE_TO_FIRST_SLICE` with a fail-closed reason. Do not leave an exercised row unclassified.
6. Create a dedicated successor issue for unresolved Movement-only rows and remove them from the current #93 release gate; do not allocate Movement.
7. Validate the harness tests, evidence schema, governance and diff; review and merge one decision/evidence PR.

## Task 3: Serialize accepted resource limits into the canonical registry

**Files:**
- Modify: `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`
- Create: one active task record, later archived by its own closeout PR

1. Rebase from the merged decision PR and acquire the sole registry lease in live allocations.
2. Add exact accepted entries with unit, immutable hard maximum, configurable range, failure category, pre-allocation/commit impact, visibility and boundary tests.
3. Add no wire/error numeric identifier and no production tuning default.
4. Run a Python registry assertion that checks uniqueness, required fields, exact accepted values and JSON round-trip. Observe RED before entries, GREEN after entries.
5. Run governance, architecture checks and diff check; whole-diff review, exact-head CI and squash-merge.
6. Close #93, #116 and #123 with links to decision evidence, registry PR/merge and the Movement successor where applicable.

## Task 4: Implement the bounded FND-04 verifier/consumer seam for #115 with TDD

**Files:**
- Create: `apps/game-server/src/foundation/fnd04_verifier.rs`
- Modify: `apps/game-server/src/foundation/mod.rs`
- Modify: `apps/game-server/Cargo.toml`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Create: `docs/architecture/reviews/OTERYN_GAME_FND04_VERIFIER_CONSUMER_DELIVERY_2026-08-25.md`
- Create: one active task record, later archived by its own closeout PR

1. Freeze the API before production code:
   - fixed verifier-selected admission/recovery contexts;
   - bounded JWS Compact/JSON parser and exact `Ed25519` policy;
   - caller-provided authenticated key-trust, account-security and current game facts with source-age/anti-rollback floors;
   - fresh success maps to existing `FreshAdmissionFacts` only after every profile/current-fact check;
   - recovery success maps to a new typed non-authoritative `ReauthenticatedRecoveryFacts`; it cannot create/revive/rebind a GameSession;
   - replay/consume remains the existing durable authority/journal responsibility.
2. Add one focused failing test at a time and run the exact test to observe RED before production implementation:
   - outer/JWS/base64/JSON/header bounds and duplicate members;
   - exact algorithm, fixed trust set and signature-first classification;
   - post-signature exact schema/binding/profile/time classification;
   - fresh/recovery purpose separation;
   - trust/security source age <=5 seconds and non-rollback floors;
   - independent revision/current account/character/world/route/runtime/lease/scope checks;
   - replay-key mapping and proof that verification alone mutates no session authority.
3. Add only exact direct dependencies required for standards-conformant base64url, JSON and Ed25519 verification; pin versions and update lockfile.
4. Refactor only after every focused GREEN. Run package tests, workspace tests, strict Clippy, rustfmt, architecture check, governance and diff check.
5. Freeze the exact head after the last semantic/test/task change.

## Task 5: Obtain independent exact-head security review and merge #115

1. Build the review packet only from exact Git blobs at frozen PR head: complete changed production/test code, Issue #115/#128 scope, FND-04 profiles, error precedence, resource registry and dependency diff.
2. Start a fresh non-authoring local `qwen2.5-coder:14b` Ollama session. Require structured P0/P1/P2 findings, exact path/line and a SHA-bound verdict. Persist packet/response SHA-256 in PR evidence without moving the frozen head.
3. Any material finding triggers TDD repair, a new frozen head, full local gate, fresh CI and a fresh independent review. Never reuse a superseded verdict.
4. Require exact-head `game-gate`, zero unresolved threads, whole-diff self-review and independent verdict with zero material findings. Squash-merge with expected-head protection.
5. Verify merged `main`, then close #115 with exact PR/head/merge/CI/review evidence.

## Task 6: Terminal closeout and readiness recomputation

**Files:**
- Modify: `docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md`
- Create/modify/archive task records under `docs/agents/tasks/**`
- Create: `docs/architecture/reviews/OTERYN_GAME_NEXT_WAVE_BLOCKERS_CLOSEOUT_2026-08-25.md`

1. Re-read current main, Issues #93/#115/#116/#123/#131, successor Movement issue, merged PRs, branches, checks and review threads.
2. Record lawful readiness separately for Ability, Interaction, AI, Movement, Durability, Server Seam, Client and QA. A merged decision/registry permits later allocation; it does not itself start implementation.
3. Release all shared path leases, archive completed task records and delete merged branches through repository lifecycle automation/manual verified fallback.
4. Record exact next aliases for every newly lawful lane and every remaining explicit blocker.
5. Run governance/diff checks, self-review, exact-head CI, squash-merge, verify main and close #131 only when all acceptance boxes are true.


## Terminal execution result — 2026-08-25

- #93, #115, #116 and #123 are completed.
- First-slice limit decisions merged through PR #140 and canonical registry mutation through PR #144.
- FND-04 allocation PR #145 and verifier/consumer PR #151 are merged; Issue #115 is completed.
- PR #151 exact-head CI is green. The missing durable independent-review evidence was reconciled post-merge on exact PR head `7a61d0347fbc73501951d28e43182b3394df9ab1` with fresh non-authoring `qwen2.5-coder:14b`: PASS, no findings.
- Movement-only resource work remains outside this completed blocker wave under Issue #139.
- No Server Seam listener, Durability, Ability, Interaction, AI, Movement, Combat or Client implementation is started by this closeout.
- All blocker-wave task records are archived and the serialized Foundation/Cargo lease is released/unassigned by the terminal closeout delivery.
- Local `validate_repository_policy.py` on Windows reports a checkout-only LICENSE hash mismatch because CRLF conversion changes the working-tree blob to `3d73aee29999ccd34b9495745d08be6c4b613712`; the committed/index LICENSE blob is `d0a1fa1482eea82e19510e7920cbe3a03e41f691`, exactly the validator-pinned canonical MPL-2.0 blob, and `LICENSE` has zero closeout diff.
