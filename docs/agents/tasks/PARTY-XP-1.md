# PARTY-XP-1 — party shared experience and party spells (skeleton task record)

```yaml
task_id: PARTY-XP-1
title: "PARTY-XP-1: party shared experience and party spells"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row PARTY-XP-1; §7 'PARTY-XP-1' (PARTY_PVP0 ACCEPTED)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_PARTY_PVP0_PARTIES_AND_PVP_DECISION_2026-09-30.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/party-xp-1
needs_migration: false
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Parties share experience under PARTY_PVP0 rules (level range, activity) and party spells work with feedback through CHAT-2.

Children (allocate from the brief's child graph; the plan row is only the family entry gate; no child of a CANDIDATE brief is allocated before the brief is ACCEPTED):

Single row; feeds BOSS-REWARD-1 (MONSTER-META-1).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| PARTY-1 | BLOCKING (Wave 2, not merged) |
| D118 XP lane (LOOP-PROGRESS-1) | BLOCKING (Wave 1, not merged) |
| D3-4 (LOOP-DAMAGE-1) | BLOCKING (Wave 2, not merged) |
| CHAT-2 (party spell feedback) | BLOCKING (CHAT-2B #2128 open) |
| PARTY_PVP0 | ACCEPTED (plan §7 status list) |
| DEATH-1, ATTACK-1b, COND-1, SPEED-1, VIS-2, ATTACK-WIRE-1 | MERGED (archive records, #1798, #2122) |
| DEATH-3 | BLOCKING (death module batch C, not merged) |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/durability/world_party.rs`
- `apps/game-server/src/durability/world_party_tests.rs`
- `apps/game-server/src/gameplay_transport/party_spell_owner.rs`
- `apps/game-server/src/gameplay_transport/party_xp.rs`
- `apps/game-server/src/gameplay_transport/party_xp_tests.rs`
- `apps/game-server/tests/party_xp_postgres.rs`
- `docs/agents/evidence/PARTY-XP-1-*.md`
- `docs/agents/tasks/PARTY-XP-1.md` (this record: status and notes only)

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

- apps/game-server/src/gameplay_transport/kill_reward.rs (XP split hook)
- D118 XP lane writer (LOOP-PROGRESS-1)
- party_spell_owner.rs spell registration
- GAME_EVENT_FOUNDATION_REGISTRY.json

## Inseparable bundles (never split across PRs)

- XP split + kill reward + audit in one transaction
- party spells + bundle data + tests

## Acceptance criteria

- [ ] Shared XP split exactly once per kill, only to eligible active members.
- [ ] Ineligible members (range/activity) receive solo-rule XP or none per brief.
- [ ] Party spells affect only party members; feedback via CHAT-2.
- [ ] Concurrent kills serialize per Character XP lane.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server party_xp
cargo test -p oteryn-game-server world_party
cargo test -p oteryn-game-server kill_reward
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
python3 tools/content-migration/regenerate_content.py --check   # slot commit only
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_PARTY_PVP0_PARTIES_AND_PVP_DECISION_2026-09-30.md
- docs/reference/spells/r21-local-candidate/bundles/instant-*_party/
- PR #1798
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 3), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
