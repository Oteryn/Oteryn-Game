# ACH-1 — achievements, badges, titles (skeleton task record)

```yaml
task_id: ACH-1
title: "ACH-1: achievements, badges, titles"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 6 row ACH-1 (rows 125, 129); §7 'ACH-1'"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_ACHIEVEMENT0_ACHIEVEMENT_GRANT_RUNTIME_DECISION_2026-10-03.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/ach-1
needs_migration: true
needs_registry: false
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Achievement grant runtime: quest-transition grant, encounter-outcome grant with durable receipt, batch grant API and counter thresholds, coverage report. ACH-NOTIFY-1 is MERGED.

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

ACH-QUEST-1, ACH-ENC-1 (hard), ACH-COUNTER-1, ACH-COVERAGE-1 (content tooling). ACH-NOTIFY-1 MERGED.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| LOOP-PROGRESS-1 (family entry gate) | BLOCKING (Wave 1 sequential) |
| QUEST-STATE-1 | MERGED |
| ENC-OUTCOME-1 (ACH-ENC-1) | BLOCKING |
| XP receipt; SKILLS-0 build receipts (ACH-COUNTER-1) | verify |
| FND-02 / ACH-NOTIFY-1 | MERGED |
| acceptance of ACHIEVEMENT0 | ACCEPTED by ACCEPT-BATCH-2026-10-15 §1 (#2209, integrated main `e2da9805d296a27b06b32335bf67394509d4910b`, control-plane verified) |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

ACH-COVERAGE-1 has **no dependencies** in the accepted brief's child graph. The
family/runtime dependencies above do not block this separately allocated tooling
slice and remain applicable to their respective runtime children.

## owned_paths (builder edits freely)

- `apps/game-server/src/achievement/grant.rs`
- `apps/game-server/src/achievement/counters.rs`
- `apps/game-server/src/achievement/grant_tests.rs`
- `apps/game-server/src/durability/achievement_outcome_receipt.rs`
- `apps/game-server/tests/achievement_grant_postgres.rs`
- `tools/content-schema/achievement-authoring/validate_achievements.py and test_validate_achievements.py (ACH-COVERAGE-1 coverage report)`
- `docs/agents/tasks/ACH-1.md` (this record: status and notes only)

To compile on the branch the builder may add only the module registration lines in shared `mod.rs`/`lib.rs` files; they are re-applied in the slot commit after merging main, together with every other slot-only change.

## slot_only_paths (ONLY in the final commit at the merge-queue turn, after merging main)

- tools/content-migration/regenerate_content.py outputs
- content/project.json
- content/manifest.json, content/world/manifest.json
- content/content.lock.json, content/world/content.lock.json
- content/interactions/index.json
- content/world/pins/*
- Rust pins (pinned registries / fixtures regenerated from content)
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring
- crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants
- apps/game-server/src/durability/mod.rs, domain/mod.rs, lib.rs module registration

## Wiring points (verified on origin/main 5742f30f3)

- apps/game-server/src/durability/account_achievement.rs (single writer of the account fact; call its grant entry point only)
- apps/game-server/src/achievement_catalogue.rs
- quest transition transaction (quest/)
- crates/protocol-oteryn/src/account_achievements.rs and achievement_notices.rs (existing; no new capability)

## Inseparable bundles (never split across PRs)

- migration (outcome receipt table) + writer + durability
- each granter calls the one grant entry point from its own transaction

## Acceptance criteria

- [ ] Grant idempotent per (account, achievement).
- [ ] Encounter outcome receipt prevents double grant after restart.
- [ ] Notice delta emitted after `Granted` commit only.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server ach
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
python3 tools/content-migration/regenerate_content.py   # slot commit only; runs --check tools
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_ACHIEVEMENT0_ACHIEVEMENT_GRANT_RUNTIME_DECISION_2026-10-03.md §3-§5
- apps/game-server/src/durability/account_achievement.rs
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## ACH-COVERAGE-1 local preparation (2026-10-15, draft #2154)

- One writer, branch `skeleton/ach-1`, starting HEAD
  `1c8d0dbdd9ac7e2ecd6c7e03eb189b4b74996d46`; only the two allocated Python
  files and this record changed. No runtime, migration, registration, generated
  content, catalogue or pin changes; publication remains with the control plane.
- Effective authority: ACHIEVEMENT0 §4, accepted through #2209. Its report
  "lists every catalogue key with no granter in any admitted content (claim,
  transition, encounter binding or threshold)"; "Such a key is
  `UNEARNABLE_PENDING`"; "No admin, script or migration grant substitutes for
  content." §2 names quest/quest-step, boss/encounter, counter, and one-time-action
  earning families. The accepted source was read on main; this older draft's
  historical CANDIDATE header does not override the integrated acceptance.
- The CLI now always adds `coverage`, scoped explicitly to the supplied supported
  inputs. Existing record counts, validation errors, slug/explicit-owner bindings,
  chest-ref counts and failure exit semantics remain. Missing content bindings
  are reportable pending records and do not fail an otherwise valid catalogue.
- `bound_keys` means a matching RewardClaim placement reference only, never runtime
  earnability or world admission. `retired_keys` remains explicit even when a key
  binds: retired grants remain no-ops under the existing contract. Every key with
  no supplied binding, including a retired key, is listed as `UNEARNABLE_PENDING`.
  An invalid/duplicate-key catalogue reports `INVALID_CATALOGUE` instead of
  positive coverage. Sorted keys and deduplicated bindings make output stable.
- **Scope limit / cross_domain_findings:** the current CLI admits catalogue and
  RewardClaim documents only. The existing quest transition schema has no
  achievement field; the Tirecz manifest has name-only `outcome_evidence`, not a
  keyed admitted achievement consumer; this validator has no admitted threshold
  input. Transition, encounter-binding and counter-threshold granters therefore
  remain explicitly unsupported. No new schemas, counter owners, thresholds,
  event maps or name-derived bindings were invented.
- The catalogue schema carries no earning-family/condition field. Descriptions,
  names, Premium/secret flags and external provenance cannot supply that authority.
  Pending reasons are explicitly `NO_ADMITTED_GRANTER;
  EARNING_FAMILY_UNCLASSIFIED`, with all four §2 families listed in the report.
  Exact per-key family reasons and whole-world coverage need the owning admitted
  content/family facts; they are not claimed complete by this supported-input slice.
- Local evidence and reproducible commands live outside the repository in
  `/workspace/oteryn-prep/ach-1-evidence/`. No Rust build is needed for this slice.


## ACH-COVERAGE-1 invalid-key repair (2026-10-16)

Returned to AUTHORING for a concrete defect in the allocated coverage tooling:
with `--chest-claims`, an invalid catalogue key represented as a JSON list or
object raised `TypeError` before the CLI could emit its invalid-catalogue report.
Only string keys now enter reference binding, matching catalogue key validation.
Schema rejection, unbound-reference failure and `INVALID_CATALOGUE` coverage
remain intact; malformed values cannot count as content coverage.

The CLI regression reproduced both crashes before the fix. The complete offline
validator suite now passes 23 tests, including both malformed-key shapes. This
change requires no runtime grant writer, schema/content admission, CP slot,
PostgreSQL job or Rust source change. Existing runtime ownership blockers above
remain applicable to their separate children.
