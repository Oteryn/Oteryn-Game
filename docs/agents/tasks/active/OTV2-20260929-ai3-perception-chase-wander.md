# OTV2-20260929-ai3-perception-chase-wander

```yaml
task_id: OTV2-20260929-ai3-perception-chase-wander
title: GAME-AI-01 §5 AI-3 -- compile ai/ into the server; perception, chase, wander (D115)
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/ai3-perception-chase-wander
issue: 162
pr: 1196
allocation: "GAME-AI-01 §5 AI-3 row; D115 in #162 comment 5879404970"
base_sha: b6bb50dce51cc2468a5ab3cca570863bc8607185
head_sha: pending_push
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: impl ai (claude-code-session-01U1WRHgL9X8RbuiG1pwczrF)"
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/lib.rs                 # mod ai; mod ai_think; declarations
  - apps/game-server/src/ai/mod.rs               # doc comment only; no behavior change
  - apps/game-server/src/ai/perception.rs        # one accessor, CandidateId::get
  - apps/game-server/src/ai_think.rs             # new: the §4.4 composition root
  - apps/game-server/src/foundation/mod.rs       # owner_timer visibility widened to pub(crate)
  - docs/agents/tasks/active/OTV2-20260929-ai3-perception-chase-wander.md
public_contracts:
  - GAME-AI-01
depends_on:
  - OTV2-20260928-ai2-spawn-envelope
blocks:
  - "AI-4 (typed AI issuer via Ability; HP floor D54; real bite committer)"
cross_repository_coordination_id: null
external_repositories: []
```

## Plan (written before coding)

1. Read §4.4/§4.9/§5, the `ai/` bootstrap, AI-1/AI-2 records, `owner_timer.rs`,
   `runtime_actor_carrier.rs`, `movement.rs`, D115 (`#162` issuecomment-5879404970).
2. Add `mod ai;` to `lib.rs` (§5: "Compile `ai/` into the server"), reusing the perception/
   path-proposal primitives from a new composition root.
3. Build the composition (attack-intent/chase/wander/idle, §4.4) driven by an occurrence-bound
   SIM RNG, acting through `movement::step_cardinal` for chase/wander only; attack decided, never
   executed. Supply `TimerFamily`/occurrence types for AI-1's `OwnerTimerLane` (items a/b).
4. Implement, test, validate, push. No PR, no GitHub comment.

## Outcome

`ai/` now compiles into the server for the first time (`lib.rs`'s new `mod ai;`; previously only
`tests/ai_bootstrap.rs` built it standalone). A new composition root, `ai_think.rs`, implements
the real §4.4 per-think policy (attack intent / chase / wander / idle), reusing
`ai::canonicalize_perception` for target selection and `ai::build_path_proposal` to bound the
chase tier's single greedy step; attack is decided (adjacency, cooldown, a seeded chance draw)
but only surfaced as `ThinkOutcome::AttackIntent`, never executed (AI-4's Ability boundary).
Chase/wander act through the real `movement::step_cardinal`. D115's four constants are pinned.
`ThinkFamily`/`ThinkOccurrence` are the first concrete `TimerFamily` for AI-1's `OwnerTimerLane`.

## Architecture and source of truth

- PROVEN: GAME-AI-01 §4.4 (Behaviour/D53), §4.5 (Movement, live since AI-2), §4.9 (D57 rows,
  unchanged here), §5 (AI-3: depends on AI-2; `lib.rs`, `ai/**`).
- PROVEN (live, not yet in `docs/`): `#162` issuecomment-5879404970, D115: think every 1000 ms,
  perception 7 tiles, wander 25%/radius 2 (bite chance/cooldown/interval are content inputs,
  §4.8, not D115 values -- never hardcoded here).
- PROVEN: `movement.rs`'s own module doc already names AI-3 as `step_cardinal`'s intended caller.
- PROVEN (discovered mid-task): `foundation::owner_timer` was private with no re-export --
  nothing outside `foundation` could reach `OwnerTimerLane`, despite AI-1's own comment
  anticipating AI-2/AI-3 wiring it in. Widened to `pub(crate) mod owner_timer;` (pure visibility
  change) so `ai_think.rs` can supply `ThinkFamily`/`ThinkOccurrence`.
- PROVEN (discovered mid-task): `tests/ai_bootstrap.rs` path-includes `ai/mod.rs` as its own
  standalone crate root (no `foundation`/`content`/`movement`). Composing against those from
  inside `ai/`'s tree would break that build, so `ai_think.rs` is a sibling, not a child.
- PROVEN (discovered mid-task): `ScopeRuntimeFence`'s scope-bound constructor is private to
  `foundation` -- production wiring never happened. No outside caller can construct a fence, so
  `schedule_next_think` is real and type-checked but untestable end-to-end here. Reported.
- DERIVED: perception/adjacency/wander-radius use Chebyshev distance on `MovementLocalPosition`
  (same floor required); §4.4/D115 name no metric.
- DERIVED: `ExactActorRef` is opaque by design, so this module cannot derive a numeric
  perception identity from one, or compare two refs' generations; perceived players carry a
  caller-supplied `PerceivedPlayerId`, and `ThinkSequenceTracker` is bounded by an explicit
  `retire` the owner calls (its generation identity), not by generation comparison here.

## Acceptance criteria

- [x] `ai/` compiles into the server; `tests/ai_bootstrap.rs`'s standalone build is unaffected.
- [x] `decide` implements all four §4.4 tiers: attack intent (adjacent, legal, off cooldown, a
      successful chance draw), chase (nearest target, one bounded `PathProposal` step), wander
      (D115's 25%/radius 2, home-anchored), idle.
- [x] A failed attack-chance draw ends the think idle -- no chase/wander fallback, tested.
- [x] A re-entry-protected adjacent target is chased, never attacked.
- [x] Randomness is a pure function of `(decision_root, occurrence, purpose, draw_index)`: a
      repeated call for the same occurrence never redraws, tested; no wall clock anywhere.
- [x] Chase/wander act through the real `movement::step_cardinal`; a rejected step mutates
      nothing and is returned as a typed error.
- [x] Attack intent never reaches Movement or any mutation surface; only surfaced.
- [x] `ThinkFamily::registered_maximum` equals AI-1's `AI01_PENDING_TIMERS_PER_ACTOR`; lane
      construction admits the maximum, rejects maximum+1 (no new registry row).
- [x] `ThinkSequenceTracker` never re-issues an `(actor, sequence)` identity, and `retire` bounds
      its size across many respawn generations of one slot (binding item a).
- [x] `schedule_next_think` forwards only a caller-issued `RuntimeWorkStamp` (binding item b).
- [ ] Exact-head CI and required independent review (control plane, after freeze).

## Excluded scope

Bite/attack execution through Ability, the player HP floor (D54), and the typed AI issuer -- all
AI-4 (§4.6, §4.7), which also needs spell P3b-2 first. No production caller wires the composed
per-creature think into `ChannelRuntimeV1`'s live owner cycle (same still-open state AI-1/AI-2
left for their own code): complete, tested and uncalled, ready for that wiring. `durability/**`,
migrations, `content/**`, `gameplay_transport/**`, `ability/**` untouched.

## Content gap (§4.8: bite/behaviour values are content inputs, not code constants)

D115's think/perception/wander values are pinned constants. Bite interval/chance/magnitude
(§4.8) do not exist as runtime content data yet; `AttackReadiness` is a caller-supplied fact.

## Binding items carried over from AI-1

- **(a) bounded, monotonic occurrence identity.** `ThinkSequenceTracker` is the only place a
  `ThinkOccurrence`'s sequence is derived from: it advances monotonically per actor and never
  re-issues a value, so `(actor, sequence)` can never repeat in a tracker's lifetime -- the same
  pattern AI-2 applied to `SpawnCellState.attempt`. `OwnerTimerLane::schedule` independently
  refuses a duplicate `(family, occurrence)` pair as a second guarantee. `retire` (Codex P2 on
  `#1196`, repair generation 1 below) bounds the tracker's own size across respawns.
- **(b) unforgeable stamp provenance.** Unchanged: `OwnerTimerLane::schedule` itself already
  enforces this (AI-1); `schedule_next_think` fabricates no `RuntimeWorkStamp` of its own, only
  forwards one the caller's `&ScopeRuntimeFence` already issued.

## Implementation / findings

- `lib.rs`: `mod ai;` (compiles the bootstrap for the first time) and `mod ai_think;` (the
  composition root), both `#[allow(dead_code)]` -- no production caller yet, same pattern as
  `movement`/`spell`.
- `ai/mod.rs`: doc comment only. No `Decision`/`DecisionUnit`/`resolve`/`AiSnapshot`/`ActorId`
  change: the bootstrap mechanism/population there does not fit attack/chase/wander/perceived
  players (a semantic mismatch, not genuine reuse).
- `ai/perception.rs`: one new accessor, `CandidateId::get`, `#[allow(dead_code)]`'d for
  `tests/ai_bootstrap.rs`'s standalone build.
- `ai_think.rs` (new): `PerceivedPlayerId`, `AttackReadiness`, `PerceivedPlayer`,
  `CreatureThinkInput`, `ThinkOutcome`, `decide`, `ThinkAction`, `act_step`, `ThinkFamily`,
  `ThinkOccurrence`, `ThinkSequenceTracker`, `schedule_next_think`, D115 constants.
- `foundation/mod.rs`: `mod owner_timer;` to `pub(crate) mod owner_timer;` -- pure visibility
  widening, the only way to make `OwnerTimerLane` reachable outside `foundation`.

## Repair generation 1 (Codex review, PR #1196, frozen head `1c8c9db`)

- **P2 fixed:** `ThinkSequenceTracker` grew one entry per distinct `ExactActorRef` ever seen,
  never removing a dead generation's entry. Added `retire(actor)`: the owner calls it on
  death/despawn, bounding the table to currently live/pending actors, not total respawn history.
  `ExactActorRef`'s fields stay private to `foundation` (no world/channel/generation accessor),
  so the slot-grouping alternative (replace-on-newer-generation) could not be built from this
  task's owned paths; `retire` is Codex's offered simpler, bounded alternative. 3 new tests.
- **P2 deferred, not coded (Codex disposition):** Binding for production wiring: carry the
  `PathProposal` binding into the action stage and call `PathProposal::revalidate` against
  current provenance before movement commits (Codex P2 on #1196).

## Validation

- `cargo fmt -p oteryn-game-server -- --check`: clean (after one `cargo fmt` pass).
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`: clean, including
  `tests/ai_bootstrap.rs`'s standalone crate and the Foundation standalone test crates.
- `cargo test -p oteryn-game-server --lib`: 834 passed, 0 failed, 2 ignored (whole-crate
  regression; `ai::`/`ai_think::` lib tests are new since `ai/` was not part of `lib.rs` before).
- `... --lib ai::tests`: 6 passed (pre-existing bootstrap). `... --lib ai_think::`: 20 passed
  (17 from the original head, 3 new from repair generation 1).
- `python3 tools/agents/validate_governance.py`: passed (22 policy docs, 9 lanes).
- `git diff --check`: clean.

## Self-review

- exact head: local candidate before freeze (repair generation 1)
- method/reviewer: implementing agent; one independent (Codex) review round on frozen head
  `1c8c9db` (PR #1196), one P2 fixed, one P2 deferred per its own disposition (above)
- material findings: none open
- verdict: READY_FOR_FREEZE (pending push and control-plane routing)

## Independent review

Required: YES. Round 1 (Codex, frozen head `1c8c9db`, PR #1196): disposed above. Further
rounds: pending.

## PR and closeout

Per instruction, this worker opens no PR and does not comment on GitHub; it pushes a new commit
to `claude/ai3-perception-chase-wander` (no force-push) and stops. Control plane owns freeze,
review and integration.

## Context checkpoint

```yaml
last_progress: repair generation 1 (Codex P2 fix + deferral, PR #1196 frozen head 1c8c9db) implemented, tested and validated; pushing now
status: waiting
pr: 1196
head_sha: pending_push
final_head_sha: null
blocker: "ScopeRuntimeFence has no accessible constructor outside foundation, so schedule_next_think cannot be exercised end-to-end from this task's own tests; live wiring into ChannelRuntimeV1's owner cycle needs a foundation-owned follow-up"
next_action: "control plane: freeze the pushed SHA, route independent re-review, allocate the ChannelRuntimeV1/ScopeRuntimeFence wiring follow-up and AI-4 (which also inherits the deferred PathProposal::revalidate binding item)"
```
