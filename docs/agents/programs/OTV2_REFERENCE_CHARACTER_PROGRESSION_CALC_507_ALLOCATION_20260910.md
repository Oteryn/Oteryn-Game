# OTV2 Reference Character Progression Calc #507 — allocation

Refs #162 #486 #507 #506 #483 #514.

```yaml
classification: COORDINATOR_ALLOCATION
allocation_state: NOT_ACTIVE_CONDITIONAL
repository: Oteryn/Oteryn-Game
programme: 486
control_plane: 162
active_control_plane_profile: OTV2_WORK_DELIVERY_COORDINATOR
admission_main_sha: 1e993fb62cb9912c833f466a021070d9d93d3892
lane_id: R7_CHAR_ITEM_PROGRESSION
worker_task_id: REFERENCE_CHARACTER_PROGRESSION_CALC_507
first_child: REFERENCE_CHARACTER_PROGRESSION_CALC_V1
worker_branch: agent/reference-character-progression-calc-507
persistence_authority: NONE
character_revision_mutation_authority: NONE
runtime_authority: NONE
production_authority: NONE
live_deployment_authority: NONE
```

## Purpose

Allocate exactly one future pure fixed-shape Character progression calculator component. Phase A calculates one staged progression outcome from one bounded current snapshot, one typed operation, one exact revision context and one injected finite policy/table.

This allocation deliberately separates scalar calculation from later durable Character/DUR-02 commit. It does not activate WP5 Character Authority, SQL, persistence, CharacterRevision mutation, Combat composition, production runtime or Reference target-parity claims.

## Activation gate

This allocation remains **NOT ACTIVE** until:

1. this exact allocation candidate receives producer whole-diff self-review and genuinely independent exact-head control-plane review with `P0=0 / P1=0 / P2=0`;
2. applicable exact-head Agent Governance, Architecture Semantic Audit and Merge Gate are successful;
3. protected integration occurs only through META 3.1 native exact-head Merge Queue and the real `merge_group` aggregate `game-gate` succeeds;
4. protected-main readback proves this allocation canonical;
5. #162 performs a fresh overlap/custody readback and explicitly activates the SAME worker/branch.

No direct worker alias, branch existence, issue comment, CI result or merge by itself activates Phase A.

## Exact future owned paths

After activation, the worker may mutate only:

```text
apps/game-server/src/domain/progression.rs
apps/game-server/src/domain/mod.rs
apps/game-server/tests/reference_character_progression_calc.rs
docs/agents/tasks/active/OTV2-20260910-reference-character-progression-calc-507.md
```

`apps/game-server/src/domain/mod.rs` is a serialized shared hook: immediately before worker activation #162 must prove no active WP5/domain writer owns or is changing it. If overlap exists, keep this worker inactive or serialize that exact hook; do not widen paths.

Everything else remains read-only. In particular:

```text
Cargo.toml
Cargo.lock
crates/simulation-determinism/**
apps/game-server/src/durability/**
apps/game-server/src/content/**
apps/game-server/src/gameplay_transport/**
apps/game-server/tests/durability_postgres.rs
migrations/**
docs/contracts/**
docs/agents/programs/**
RESOURCE_LIMITS_REGISTRY.json
.github/workflows/**
```

Platform/external repositories, production environments and live data remain out of scope.

## Exact first child

`REFERENCE_CHARACTER_PROGRESSION_CALC_V1` is a pure component:

```text
one bounded current progression snapshot
+ one typed progression operation
+ exact profile/ruleset/content/SIM/evidence/declaration revisions
+ one injected finite progression policy/table
-> checked deterministic arithmetic
-> one complete staged progression outcome
```

The first child must not own durable application, replay queues or source-attribution policy.

## Required typed surface

Exact Rust naming may follow existing Domain style, but the semantic surface must remain closed and typed.

Minimum operation families:

```text
AwardExperience {
  source_occurrence,
  reward_revision,
  amount_or_reward_key,
}

ApplyDeathExperienceLoss {
  death_occurrence,
  death_policy_revision,
  declared_difference_revision,
}
```

The calculation context binds exact interpretation revisions. The injected finite policy/table supplies the threshold/reward/loss oracle. The result is one fixed-shape complete staged outcome.

Do not expose a generic signed `progression_delta` capable of mutating arbitrary skill/counter families.

## Authority invariants

The Phase-A calculator must be structurally unable to:

- mint, increment or commit `CharacterRevision`;
- mutate Character ownership/lifecycle/session/lease state;
- open SQL connections or call DUR-02/DUR-03;
- persist receipts/outbox/audit state;
- enqueue pending/retry/reconciliation work;
- decide Combat reward attribution or eligibility;
- mutate Combat, corpse, loot or item state;
- infer an unproven Global progression formula from current Global or OTS code.

The later `REFERENCE_CHARACTER_PROGRESSION_COMMIT_V1` remains a separate consumer under existing WP5 Character/DUR-02 ownership after its own prerequisites and explicit allocation.

## Evidence discipline

Structural tests must use **synthetic, explicitly non-Reference finite policies/tables** unless an exact target field is independently admissible under #483/#514.

Do not hard-code current official level-threshold examples, Canary/Crystal formulas or convenient OTS values as the immutable `2026-07-28` Global target while continuity remains unknown.

The component may prove the accepted Oteryn Reference declared-difference transform structurally against synthetic tables:

```text
DeathXPBasis = LevelXPSpan(current_level)
DeathSkillLoss = 0
DeathMagicLevelLoss = 0
```

Such tests prove the Oteryn-side declared-difference mechanism only. Their names/evidence must not claim Global parity.

## Required RED/GREEN matrix

At minimum prove:

1. award below next synthetic threshold changes XP but not projected level;
2. exact threshold crossing produces the deterministic higher staged level;
3. multi-threshold award is deterministic if supported, otherwise explicitly rejected by the first-child contract;
4. death XP loss can cross a lower threshold and delevel in staged output;
5. two same-level snapshots with different within-level progress use the same full injected `LevelXPSpan` basis for the declared-difference death transform;
6. death operation leaves skill/magic progression fields unchanged by construction;
7. mismatched profile/ruleset/evidence/policy/declaration revisions fail closed;
8. missing/unknown threshold oracle fails closed and never guesses;
9. checked add/subtract/threshold arithmetic overflow or underflow rejects with no partial result;
10. unsupported operation/skill/proficiency mutation fails closed;
11. identical scalar inputs deterministically reproduce an equivalent staged outcome;
12. no structural test or component success promotes target parity without admissible external evidence.

Use existing checked numeric/domain primitives where already available through current Game dependencies. Do not mutate `crates/simulation-determinism/**` or add a second numeric library.

## Resource disposition

If the implementation remains exactly one bounded input snapshot, one operation, one finite injected policy and one fixed-shape result, it introduces no new variable-cardinality production resource and needs no new `RESOURCE_LIMITS_REGISTRY` row.

This disposition is invalidated by any arbitrary collection, unbounded policy table, multi-character batch, retained pending/retry queue, variable retained diagnostics/history or multi-principal attribution. If any appear necessary, stop before that mutation and return a resource/allocation escalation.

## Phase-A acceptance

A successful Phase-A worker may become component-ready independently of WP3/#356 and WP4/#335. Phase-A completion **does not** close #507 terminally and does not release #506 XP composition.

The later durable progression apply path remains blocked until the real prerequisite chain is true, including:

```text
WP3/#356 terminal protected
+ WP4/#335 protected/released
+ existing WP5 Character Authority/audit ownership activated/applied
+ exact progression durable carrier/schema/apply hook
+ applicable PostgreSQL routing/custody
+ durable idempotency/restart/audit qualification
+ fresh exact allocation/review/CI/MQ/readback
```

Do not pull any of those responsibilities into Phase A.

## Collision and anti-duplication rule

Immediately before activation, re-read open PRs, branches and active tasks. If another canonical progression/domain worker overlaps any owned path, bind/serialize against that lineage rather than creating a competing writer.

Pre-allocation readback at the admission main found no active branch matching `progression` and no open material PR for this first child.

## Completion handoff

A successful return must contain:

```yaml
worker_task_id: REFERENCE_CHARACTER_PROGRESSION_CALC_507
exact_head_sha: <sha>
changed_paths: []
focused_validation: []
whole_diff_review: PASS|BLOCKED
phase_a_component_result: PASS|BLOCKED_MATERIAL_FINDING
synthetic_policy_only: true|false
target_threshold_parity: PROVEN|UNKNOWN|CONFLICT
character_revision_mutation: false
persistence_mutation: false
resource_registry_change: false
production_authority: NONE
next_action: <one exact coordinator action>
```

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
