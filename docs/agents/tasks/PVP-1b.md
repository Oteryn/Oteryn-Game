# PVP-1b

```yaml
task_id: PVP-1b
title: "PVP-1b: skull evaluation, World cleanup job, pvp/party rulesets (skeleton)"
mode: IMPLEMENT
status: NOT_FULLY_QUALIFIED
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/pvp-1b
issue: 1622
pr: 2203
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md App. 7 PARTY/PVP PVP-1b; wave 2"
class: "hard, persistence and security review"
migration: maybe
registry: no
owner: family_f_pvp
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-18
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/combat/pvp/
  - apps/game-server/src/durability/pvp_state.rs
  - rulesets/pvp/ (new rows)
  - rulesets/party/ (new rows)
  - apps/game-server/tests/pvp_1b_*.rs (new)
  # Widened by the lead for the death-transaction wiring (CP D1443 allocation, 2026-10-18).
  - apps/game-server/src/durability/character_death.rs
  - apps/game-server/src/durability/character_revision_sequencer.rs
  - apps/game-server/src/gameplay_transport/source_item_cycle.rs
  - apps/game-server/migrations/0123_pvp_death_consequences.sql (number slot-only)
  - apps/game-server/tests/support/pvp_state_postgres_cases.rs
  - rulesets/pvp/skulls/
  - rulesets/party/shared-experience/
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
- Widened (lead, D1443 allocation): `durability/character_death.rs`,
  `durability/character_revision_sequencer.rs`, `gameplay_transport/source_item_cycle.rs`,
  migration `0123_pvp_death_consequences.sql`, `tests/support/pvp_state_postgres_cases.rs`,
  `rulesets/pvp/skulls/`, `rulesets/party/shared-experience/`. `combat/pvp/mod.rs` stays
  untouched (PVP-DEATH-1 #2205 adds `mod death;` there).

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

- [x] Skull points written atomically with the death (`commit_character_pvp_death` /
  `settle_player_pvp_death`; PG case
  `unjustified_kills_raise_a_white_then_red_skull_that_the_world_job_clears`).
- [x] World cleanup job registered in the Channel owner cycle (`drain_source_pvp_expiry`,
  1 s pass with the party deadlines), including the expired-skull normalization.
- [ ] Live PvP (attack, killer classification, skull wire/broadcast): PVP-RT-1b, PVP-DEATH-1,
  PVP-WIRE-1, PVP-CLIENT-1 (PVP-0 §8 go-live gate).
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
last_progress: death-transaction adapter, World job registration, rulesets and PG case
status: waiting
branch: skeleton/pvp-1b
blocker: "no skull wire/broadcast path (PVP-WIRE-1/PVP-CLIENT-1); PvP attack and killer classification (PVP-RT-1b/PVP-DEATH-1); PARTY-1"
next_action: Codex review of the pushed head; CP decides the go-live split
```

## D1443 integration (2026-10-18)

- **D1443 (CP):** frag windows roll from each unjustified kill's committed timestamp
  (`committed_at`, stamped by the 0081 trigger), with an exclusive end: a point counts while
  `now - committed_at < window`. The previous "endpoint assumption" is settled by D1443.
- Skull module registered through `durability/pvp_state.rs` (`#[path]` mount; test crates hold
  the durability tree without the combat tree). The adapter
  `apply_pvp_death_in_transaction` runs in the victim's death transaction after the receipt:
  contributors must equal the victim's ledger within 5 minutes (compact kind must match); state
  rows locked in CharacterId order; kill block +15 min for every contributor; for each
  unjustified one a point row (1000 / 500 assist / floor(5000/n) beyond five damage
  contributors), a 7-day revenge mark, the white skull at least to the kill block (PVP-0 §6.1,
  D971) and the skull evaluation; then the victim's ledger is deleted. The contributors are
  part of the death command binding (empty for PvE, so PvE bindings are unchanged).
- Migration 0123: runtime grants for skull/kill block/points/marks; an UPDATE guard that admits
  a skull or kill-block change only in a death transaction of the row's World naming the
  Character in the victim's ledger, except clearing an expired skull; the expiry index.
- Ruleset rows: `rulesets/pvp/skulls/skull-policy.json`, pinned by a unit test against the Rust
  constants; `rulesets/party/shared-experience/shared-experience.json` (no runtime consumer).
- `OTS_HYPOTHESIS_ONLY` (Canary values; the manual gives only rule-of-thumb ranges): red
  thresholds 3/5/10 kills per day/week/month, black thresholds 6/10/20, the 5/N damage split.
  `OTERYN_DECISION` (`PARITY_PENDING`): assist 500 milli-points, 45-day point retention, at most
  80 contributors, party activity window 2 minutes. Official (manual): red 30 d, black 45 d,
  revenge 7 d, kill block 15 min.
- Not wired here (go-live gate PVP-0 §8): no skull field exists on the creature wire or its
  broadcast (PVP-WIRE-1/PVP-CLIENT-1, protocol slot-only); players cannot attack players
  (`attackable_kind` creature-only); the runtime killer classification and mark avenging
  belong to PVP-RT-1b/PVP-DEATH-1, which call `settle_player_pvp_death`.

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

## D1122 draft checkpoint (2026-10-16)

The coherent component at `1fda2fbebeb27d5767baab6ee9ba0b9f551657cc`
preserves a later unexpired red/black deadline when new points would otherwise
shorten it. Native 14 cases over the exact owned Rust and strict all-target
Clippy passed on the canonical PC/V2 lane with private native registration in
`combat/pvp/mod.rs` (`product_registration=false`). This qualifies that private
component assembly, not raw product registration or full production compilation.
The full CLOUD package run passed the library (3164 passed, 0 failed, 29 ignored)
and failed four `character_authority_postgres` claim/quest-obligation cases:
`claim_mints_one_item_into_a_new_entry_and_a_once_claim_never_repeats`,
`every_fence_operator_rejects_at_freeze_and_at_commit`,
`a_claim_at_rl_07_open_obligations_is_refused_writing_nothing`, and
`a_quest_obligation_rides_on_its_claim_and_the_quest_writer_consumes_it`.
The retained errors are Ambiguous, Unavailable(Database(Database(None))), and
Unavailable(CommitOutcomeUnknown). Their cause remains unresolved; no failure
independence or full PASS is claimed and no blind rerun is authorized.

The exact raw result is `evidence/pvp-1b-cloud-package-validation-results.json`;
independent failure classification is
`evidence/2203-claim-failure-independent-review.json`, outside the checkout.
This documentation checkpoint preserves those RED results under CP D1122,
without changing Rust source or activating PvP. The D1128 ledger/points adapter
in `durability/pvp_ledger.rs` follows #2204; current skull endpoints/parity,
atomic death/points/revenge/kill-block, World cleanup and runtime wiring remain
open. No ready, merge, queue change or review trigger is included.
