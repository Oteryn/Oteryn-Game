# PVP-1b

```yaml
task_id: PVP-1b
title: "PVP-1b: skull evaluation, World cleanup job, pvp/party rulesets (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/pvp-1b
issue: 1622
pr: null
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md App. 7 PARTY/PVP PVP-1b; wave 2"
class: "hard, persistence and security review"
migration: maybe
registry: no
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/combat/pvp/
  - apps/game-server/src/durability/pvp_state.rs
  - rulesets/pvp/ (new rows)
  - rulesets/party/ (new rows)
  - apps/game-server/tests/pvp_1b_*.rs (new)
slot_only_paths:
  - migration number (if any)
  - mod.rs (cleanup job composition)
  - ruleset pins
public_contracts: []
depends_on:
  - "PVP-1a: MERGED #2032 (migration 0081)"
  - "DEATH-1: MERGED"
  - "PARTY-1: BLOCKING: not merged"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

Skull evaluation in the death transaction (points and revenge-mark inserts), the node World cleanup job calling the 1a cleanup function, `rulesets/pvp/` and `rulesets/party/` rows (`pvp_type` OPTIONAL).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| PVP-1a | MERGED #2032 (migration 0081) |
| DEATH-1 | MERGED |
| PARTY-1 | BLOCKING: not merged |

**BLOCKING:** PARTY-1. The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- `apps/game-server/src/combat/pvp/`
- `apps/game-server/src/durability/pvp_state.rs`
- rulesets/pvp/ (new rows)
- rulesets/party/ (new rows)
- apps/game-server/tests/pvp_1b_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- migration number (if any)
- mod.rs (cleanup job composition)
- ruleset pins
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `durability/character_death.rs` death transaction
- `durability/pvp_state.rs` 1a cleanup fn

## Inseparable bundles

migration+writer+durability.

## Acceptance criteria

- [ ] Skull points written atomically with the death.
- [x] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue'); inspection used the allocated head documented below.

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

PARTY_PVP0 l.33, §6, §9. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` App. 7 PARTY/PVP PVP-1b.

## Builder instructions

- Work on branch `skeleton/pvp-1b`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: detached skull-window evaluator prepared locally; source-bound checks passed
status: waiting
branch: skeleton/pvp-1b
blocker: PARTY-1
next_action: CP reviews the detached local diff; runtime remains blocked
```

## Detached local preparation (2026-10-15, unpublished)

Draft #2203, `skeleton/pvp-1b`, initial/unchanged head
`aa9fc367f4d5159f1d4d5e98e8468982eb36db9e`. This preparation is restricted to
`combat/pvp/skull.rs`, `combat/pvp/skull_tests.rs` and this task record. No commit,
publication, production registration, death writer, database/schema change, ruleset, pin or
registry edit is authorized. The broader skeleton-owned paths above are not this allocation.

Residue checked before implementation: `combat/pvp/` contains legality and lock helpers, but
no skull-window evaluator. PVP-1a already supplies `durability/pvp_state.rs` reads, deadline
write-ahead and bounded expiry cleanup; those are reused as evidence, not reimplemented.
The new helper imports the existing canonical `PvpSkull` enum and remains unregistered.

The lead supplied the previously accepted #1771 basis; the decision document's embedded
CANDIDATE heading remains unchanged. Only PARTY-PVP-0 §6.2/§9/RL-14..16 is implemented here,
without using newer unmerged gap-closure proposals. Numerical provenance remains
`PARITY_PENDING`: thresholds reproduce the accepted manual lower bounds, not a claim of
independently demonstrated current Tibia Global parity.

The evaluator totals rolling 24-hour/7-day/30-day points including one positive new increment,
uses red 3,000/5,000/10,000 and black 6,000/10,000/20,000 milli thresholds, preserves an
unexpired higher skull and resets the resulting red/black deadline to now +30/+45 days.
Stored skulls expire at `until <= now` (§6.2). Zero/oversized increments, malformed states or
point rows, future points and arithmetic deadline overflow are refused.

**Endpoint assumption, not settled contract:** rolling windows use `age < duration`, or
`(now - duration, now]`. §9 does not specify the lower-bound equality. The lead permits this
only as a detached preparation assumption; architect acceptance and database-time precision
binding are still required before integration. Exact-edge tests expose the choice.

The input convention separates history from the new increment (1..=1,000 milli); the helper
adds the latter once to all three totals. It cannot prove occurrence uniqueness, classification,
World/contributor scope, snapshot completeness, current authority or that a new point row was
committed. A future adapter must supply those facts under the victim's death transaction and
current fences; arithmetic output is not mutation permission. Only qualified Open-PvP
unjustified points belong on that path. No runtime gate or Optional/Hardcore behavior changes.

Runtime remains blocked on PARTY/death-transaction dependencies and allocation of the actual
writer/cleanup/ruleset/wiring work. The full atomic-death acceptance checkbox stays open.
This detached evaluator does not bypass those gates, award points, write revenge marks,
run cleanup or activate PvP. Newer unmerged splits are not treated as release gates satisfied.

Validation is intentionally small: an isolated no-dependency harness outside the repository
path-includes the actual `skull.rs` and its actual `skull_tests.rs`. Its only type dependency is
the verbatim canonical `PvpSkull` definition extracted from `durability/pvp_state.rs`, with
the full source and exact excerpt hashed. This tests arithmetic, not DB/session authority or
production composition. No surrounding durability implementation is stubbed or compiled.

- `TEST_FIRST_COMPILE_FAILURE`: before implementation, the harness failed with 20 missing
  `evaluate_skull` errors. This is not an executed product-behavior RED claim.
- Actual-source harness GREEN: 12 tests passed, 0 failed/ignored, covering all threshold edges,
  rolling endpoints, new-point inclusion/reset, active-skull preservation/upgrades, expiry,
  malformed/future input, time overflow and deterministic reorder/repeat behavior.
- Harness `cargo clippy --all-targets -- -D warnings`: passed; explicit owned-file
  `rustfmt --check` and corrected workspace `cargo fmt --all --check`: passed.
- Source binding: eight inputs verified before/after checks, including actual evaluator/test
  sources, the owning decision and verbatim canonical enum/full durability source hashes.
- Governance validator: passed (22 documents, 9 lanes); corrected unit run: 124 passed.
- Initial setup failures are retained separately: one governance test first errored because
  sparse checkout omitted `experiments/world-vfx-real-content/README.md`; workspace fmt first
  could not load the omitted `tests/security/auth/Cargo.toml`. The lead restored baseline
  experiments/tests/vendor from this same head, with no unowned source diff. Corrected runs
  passed; these omissions were not product defects.
- Whole-diff adversarial self-review, owned-path/unchanged-head/source-hash checks and
  `git diff --check`: passed. Two new Rust files plus this task record only; no commit or
  external write. No Rust build/job remains queued by this worker.
- Full-server/all-target production compilation, PostgreSQL transaction validation and live
  PvP E2E are not claimed; they require the later integration allocation.

Evidence is local at `/workspace/oteryn-prep/pvp-1b-evidence/`.
