# OTV2-20260927-r7-p03-character-xp-commit

```yaml
task_id: OTV2-20260927-r7-p03-character-xp-commit
title: R7 P03 durable Character XP commit
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/r7-p03-character-xp-commit
issue: 162
pr: null
base_sha: 5ecf841c7b849a252ec239226361e6e76ac05239
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: /root
created_at: 2026-09-27T14:25:00Z
updated_at: 2026-09-27T15:28:55Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/durability/character_progression.rs
  - apps/game-server/src/durability/character_authority.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/migrations/0009_character_progression.sql
  - apps/game-server/tests/character_progression_postgres.rs
  - apps/game-server/tests/character_authority_postgres.rs
  - apps/game-server/tests/support/character_progression_postgres_cases.rs
  - docs/agents/tasks/active/OTV2-20260927-r7-p03-character-xp-commit.md
  - docs/agents/evidence/OTV2-20260927-r7-p03-character-xp-commit.json
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
allocation_comment: 5856703694
base_refresh_comment: 5857130016
routing_allocation_comments: [5857090934, 5857102866]
jira_story: KAN-12
```

## Outcome

Add the production Character/DUR-02 component that commits one positive XP award
to typed Character-owned state. One stable reward occurrence either advances the
global `CharacterRevision` exactly once or returns its retained result. Changed
reuse conflicts, lost success reconciles by occurrence, and a restart reads the
same state. P03 does not wire Combat to XP and is not a playable milestone.

## Architecture and source of truth

- `PROVEN`: GAME-CHAR-01 Stage B assigns persistent progression and its explicit
  interpretation context to Character.
- `PROVEN`: DUR-02 requires one global `CharacterRevision` increment per semantic
  transaction, exact-retry receipts, stale/conflicting rejection and restart readback.
- `PROVEN`: FND-04 connection, lease and scope generations are separate fences
  and must be revalidated in the XP transaction.
- `PROVEN`: `domain::progression::calculate_progression` is the existing pure,
  persistence-neutral calculator and is reused without a second formula.
- `PROVEN`: before freeze, AUTHORING fast-forwarded from allocated base
  `56e5c8a39bf2d899cbbc24735d2d7440d4ecaec1` through `0d01e9841f9c7bbc6895d6bd928f8521e6f205f3`
  and `0c7098eb5086fb8274527ec3a419fd4a2f862f38` to protected
  `main@5ecf841c7b849a252ec239226361e6e76ac05239`; every integrated path is
  disjoint from all nine P03 paths and migration `0009` remained free.
- `UNKNOWN`: Reference/Global formula parity, low-level bonuses, modifiers,
  party/shared XP, death loss commit and the runtime death-to-XP producer.
- `NOT_APPLICABLE`: Canary, CrystalServer, TibiaWiki BR and Fandom do not establish
  database fencing or idempotency. The R7 checkpoint's level-50 `+5` Rat vector is
  only a bounded test input with modifiers inactive.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: CharacterRecovery_x_FND04GameplayFence_x_AwardExperienceCommit
authority_invariants:
  - sealed recovery admission is current
  - reward occurrence is canonical UUIDv7
  - exact retry binding is unchanged
  - GameSession is current and nonterminal
  - connection generation is current
  - Character lease generation and holder session are current
  - Channel scope generation and assigned holder are current
  - node incarnation is current and secret-proven
  - Character lifecycle and expected global revision are current
  - Character interpretation and typed progression context are current
consumer_boundaries:
  - new award commit revalidates all current authority
  - retained exact result reconciles without reacquiring gameplay authority
  - current progression read requires sealed Character recovery authority
mutation_operators:
  applicable:
    - missing progression state
    - stale CharacterRevision
    - stale connection generation
    - stale lease generation
    - stale scope generation
    - terminal session
    - ended node incarnation
    - context or policy substitution
    - exact concurrent duplicate
    - distinct concurrent occurrences with one predecessor
    - post-update receipt failure and transaction rollback
  considered_not_applicable:
    - source reward eligibility, owned by later Combat composition
    - progression initialization, owned by a separate bootstrap/population gate
    - audit/outbox publication, no accepted mandatory event family exists
one_invariant_per_negative_case: covered by focused PostgreSQL target
independent_current_fact_sources:
  - game_character_recovery_admissions
  - game_durability_reconnect_sessions and admission guards
  - game_runtime_scope_assignments and game_node_registrations
  - game_character_roots and game_character_interpretations
record_derived_matching_helper:
  allowed_for_positive_happy_path: false
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: commit, reconcile, current read
  protocol_versions: typed v1 command and policy digests
  direct_and_reconciled_paths: covered
  fenced_durable_writes: covered
  restart_retry_replay_concurrency_pg_reload: focused target present; configured PostgreSQL run pending
  evidence:
    - apps/game-server/tests/character_progression_postgres.rs
    - apps/game-server/tests/support/character_progression_postgres_cases.rs
    - apps/game-server/tests/character_authority_postgres.rs
finding_dispositions:
  p0_p1_accepted_and_repaired:
    - replaced immutable-root trigger with identity-preserving exact revision successor guard
    - decoupled bootstrap receipt revision one from current CharacterRevision integrity
    - added deferred cross-relation state/receipt/root consistency constraint
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred:
    - added direct new-occurrence rejection after the GameSession becomes terminal
    - made all-target Clippy consume error payloads and scoped expect_used only to unit tests
```

## Acceptance criteria

- [x] Typed progression state contains level, total XP and explicit revision context.
- [x] No JSON state bag, signed generic delta, private retry queue or direct Combat SQL.
- [x] Existing finite progression calculator recomputes the staged result in transaction.
- [x] Stable occurrence exact replay returns the original result; changed reuse conflicts.
- [x] New writes revalidate recovery, session, connection, lease, scope and node fences.
- [x] Root revision, typed state and immutable receipt commit atomically with a deferred DB constraint.
- [x] Missing initial progression remains valid but XP commit fails closed.
- [x] Focused target covers replay, stale facts, rollback, concurrency and restart readback.
- [ ] Configured PostgreSQL 17.6 execution passes on the frozen exact head.
- [ ] Independent exact-head GPT-6 Luna review and hosted checks pass before MQ.

## Excluded scope

Combat reward eligibility, occurrence production, spawn/death wiring, progression
initialization, Reference formula selection, low-level/modifier/party behavior,
death loss commit, loot coupling, protocol/client output, playable integration,
audit/outbox invention and Global parity.

## Implementation / findings

Migration `0009` adds typed state and immutable reward-occurrence receipts without
backfill. It narrows root updates to identity-preserving `revision + 1`, narrows
state updates to one positive XP successor with unchanged context, and defers a
cross-relation consistency check to commit. The historical bootstrap receipt stays
at revision one and is not rewritten.

`commit_character_experience` checks the sealed recovery record, locks admission
relations, short-circuits an exact retained receipt, then validates the current
session/guards, assigned Channel holder, node incarnation, Character root,
interpretation and stored progression context. It calls `calculate_progression`,
updates root/state, inserts the receipt and reports success only after commit.

The command binding is a versioned SHA-256 binding of occurrence, Character,
expected revision, award, all revision identities and the full finite policy.
Attempt-local session/node/lease/scope facts are deliberately excluded so a lost
success remains reconcilable after reconnect. They remain mandatory for every
previously unseen occurrence.

## Validation

### Focused

- WSL `cargo check -p oteryn-game-server --locked`: PASS.
- WSL focused and registered PostgreSQL target compile: PASS.
- Protected `character_authority_postgres` target discovers the shared P03 cases: PASS.
- WSL strict Clippy for `character_progression_postgres` with `--no-deps -D warnings`: PASS.
- WSL full `cargo clippy --workspace --all-targets -- -D warnings`: PASS after exact-head repair.
- GPT-6 Luna pre-freeze SQL/trigger/concurrency static audit: PASS, no material findings.
- Pure binding regressions: present in `character_progression.rs`.

### Component/integration

- Focused PostgreSQL wrapper: `apps/game-server/tests/character_progression_postgres.rs`.
- Protected PostgreSQL 17.6 routing: shared cases are included by registered `character_authority_postgres`.
- Local configured PostgreSQL: pending; no `OTERYN_TEST_POSTGRES_ADMIN_URL` and
  the installed Docker Desktop engine failed to initialize.
- Hosted PostgreSQL 17.6 lane: container/classification reached on superseded SHA; execution pending final repaired exact head.

### E2E

`NOT_APPLICABLE`: P03 has no Combat occurrence producer or gameplay composition.

### Exact-head CI

- final head: pending remote freeze
- trigger source: pending PR
- workflow/run/job: pending
- runner assignment: pending
- classification: server + PostgreSQL
- result: pending

## Self-review

- exact head: pending remote freeze
- method/reviewer: `/root`, complete allocated-delta and authority-family review
- material findings: immutable root/bootstrap-receipt assumptions repaired before freeze
- verdict: local authoring candidate remains under validation

## Independent review

- required: YES; durable Character mutation and recovery/session fencing
- exact head: pending remote freeze
- method/auditor: independent GPT-6 Luna subagent
- material findings: successor review found one P2 terminal-session coverage gap; fixed before final freeze
- verdict: pending

## PR and closeout

- changed-file review: pending final staged candidate
- unresolved review threads: pending PR
- related/superseded PRs: none adopted
- protected auto-merge: pending native Merge Queue
- merge commit/result: pending
- ownership release: pending protected-main readback and archive packet

## Context checkpoint

```yaml
last_progress: full-workspace Clippy and terminal-session negative coverage repaired
status: implementing
branch: codex/r7-p03-character-xp-commit
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 3
ci_recovery_actions_for_current_head: 1
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: refresh main, freeze repaired SHA, rerun exact-head review and PostgreSQL 17.6 CI
```
