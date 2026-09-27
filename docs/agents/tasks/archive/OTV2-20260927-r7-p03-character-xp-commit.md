# OTV2-20260927-r7-p03-character-xp-commit

```yaml
task_id: OTV2-20260927-r7-p03-character-xp-commit
title: R7 P03 durable Character XP commit
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/r7-p03-character-xp-commit
issue: 162
pr: 998
base_sha: 5ecf841c7b849a252ec239226361e6e76ac05239
head_sha: 0f7b74370fbc07bb5d894cf142b61f9a07611225
final_head_sha: 0f7b74370fbc07bb5d894cf142b61f9a07611225
final_head_frozen_at: 2026-09-27T16:47:31Z
completed_at: 2026-09-27T17:20:06Z
owner: /root
created_at: 2026-09-27T14:25:00Z
updated_at: 2026-09-27T17:20:06Z
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
- `PROVEN`: the existing pure `calculate_progression` is reused without a second formula.
- `PROVEN`: before freeze, AUTHORING refreshed to protected
  `main@5ecf841c7b849a252ec239226361e6e76ac05239`; integrated paths were
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
  restart_retry_replay_concurrency_pg_reload: focused target and configured hosted PostgreSQL passed
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
- [x] Configured PostgreSQL 17.6 execution passes on the frozen exact head.
- [x] Independent exact-head GPT-6 Luna review and hosted checks pass before MQ.

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
- Protected `character_authority_postgres` discovers the four shared P03 cases: PASS.
- WSL strict Clippy for `character_progression_postgres` with `--no-deps -D warnings`: PASS.
- WSL strict Clippy for the registered protected target: PASS after each fixture repair.
- GPT-6 Luna SQL/trigger/concurrency audit: PASS, no material findings.
- Pure binding regressions: present in `character_progression.rs`.

### Component/integration

- Focused PostgreSQL wrapper: `apps/game-server/tests/character_progression_postgres.rs`.
- Protected PostgreSQL 17.6 routing: shared cases are included by registered `character_authority_postgres`.
- Local configured PostgreSQL was unavailable in the authoring environment; no
  `OTERYN_TEST_POSTGRES_ADMIN_URL` was configured and the installed Docker Desktop
  engine failed to initialize. Hosted PostgreSQL is the terminal execution proof.
- Hosted PostgreSQL 17.6: `36329970211` exposed the insecure `/tmp` parent;
  `36331164655` proved its `0700` repair, then rejected the direct scope insert.
  `36332161320` exposed raw-guard assignment ambiguity. On `1ce4eefa`, run
  `36333519659` passed build, Clippy and workspace tests; the real PG target passed
  replay/restart and stale-fence cases. Both concurrent cases committed exactly
  once, while their simultaneous loser correctly hit the root's max-one-holder
  backpressure. The final successor retries after holder release and proves exact
  replay or stale-predecessor rejection.
- Frozen source `0f7b74370fbc07bb5d894cf142b61f9a07611225`: exact-head run
  `36334520143` passed all required source checks. Its registered PostgreSQL target
  passed all four P03 cases and reported `464 passed; 0 failed`.
- Actual merge-group run `36335855136`, job `108666618739`: PostgreSQL 17.6 passed
  all four P03 cases again on composed SHA `650dd6484a443fbc39e4b5dfb0686b24c72cbcd5`.

### E2E

`NOT_APPLICABLE`: P03 has no Combat occurrence producer or gameplay composition.

### Exact-head CI

- final head: `0f7b74370fbc07bb5d894cf142b61f9a07611225`
- trigger source: PR #998 `synchronize`
- workflow/run: canonical Merge gate `36334520143`
- decisive jobs: Rust Linux `108662817793`, validate `108665037846`,
  `game-gate` `108665053000`
- runner assignment: hosted Linux; required source jobs completed
- classification: server + PostgreSQL
- result: PASS; exact-head checks terminal without pending or failure

## Self-review

- exact head: `0f7b74370fbc07bb5d894cf142b61f9a07611225`
- method/reviewer: `/root`, complete nine-path delta and authority-family review
- material findings: immutable root/bootstrap-receipt assumptions and all later
  exact-head fixture/authority defects were repaired before the final freeze
- verdict: PASS; no unresolved material finding

## Independent review

- required: YES; durable Character mutation and recovery/session fencing
- exact head: `0f7b74370fbc07bb5d894cf142b61f9a07611225`
- method/auditor: independent GPT-6 Luna subagent
- material findings: terminal-session coverage, readiness ordering, stale task-head
  metadata and the max-one-holder fixture expectation were repaired in superseded
  candidates; fresh final-head review found no P0/P1/P2
- verdict: PASS

## PR and closeout

- changed-file review: complete for all nine allocated source paths
- unresolved review threads: none
- related/superseded PRs: none adopted
- protected integration: native exact-head `merge-async` with
  `merge_action=merge_queue`; UUID `5a9a98cb-ec7f-48e9-8cba-90573c03dbc4`,
  receipt sequence 1 and same-target readback sequence 2
- actual Merge Queue: run `36335855136`, aggregate `game-gate` job
  `108668316671` SUCCESS
- merge commit/result: PR #998 MERGED at `2026-09-27T17:20:06Z` as
  `650dd6484a443fbc39e4b5dfb0686b24c72cbcd5`
- protected readback: `main` equals the successful merge-group SHA; eight allocated
  blobs equal the frozen source and `durability/mod.rs` preserves the complete P03
  delta plus only the independent reconnect linkage from the composed base
- ownership release: terminal after protected archive readback; the lifecycle
  receipt is recorded externally on Issue #162 to avoid self-referential closeout data

## Context checkpoint

```yaml
last_progress: "PR #998 merged through successful Merge Queue and protected main readback; task archived"
status: completed
branch: codex/r7-p03-character-xp-commit
head_sha: 0f7b74370fbc07bb5d894cf142b61f9a07611225
pr: 998
final_head_sha: 0f7b74370fbc07bb5d894cf142b61f9a07611225
final_head_frozen_at: 2026-09-27T16:47:31Z
ci_trigger_source: pull_request/synchronize
ci_check_generation: 36334520143
ci_checks_for_current_head: 14
ci_run_ids: [36334520143]
ci_job_ids: [108662817793, 108665037846, 108665053000]
runner_assignment_state: success
merge_group_run_id: 36335855136
merge_group_sha: 650dd6484a443fbc39e4b5dfb0686b24c72cbcd5
merge_group_game_gate_job: 108668316671
merge_group_postgres_job: 108666618739
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 8
ci_recovery_actions_for_current_head: 6
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: "none for P03; excluded gameplay, initialization and Reference-parity work require separate allocations"
```
