# GUILD-1 — guild core tables and transactions (skeleton task record)

```yaml
task_id: GUILD-1
title: "GUILD-1: guild core tables and transactions"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 5 row GUILD-1 (row 113); §7 'GUILD-*' row GUILD-1"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/guild-1
needs_migration: true
needs_registry: false
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Guild, rank, member, invitation and account-leadership tables; found, invite, join, leave, exclude, rank, resign and disband transactions; formation and vice World jobs (GUILD0 §3, §4).

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

GUILD-1 (hard, persistence and security review). GUILD-BANK-1, GUILD-CHAT-1 follow in Wave 4; GUILDHALL-1 and GUILD-WIRE-1 are Wave 5.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| PARTY-1 | BLOCKING (Wave 2, not merged) |
| NPC-ECON-1 (BANK-1) | BANK-1 MERGED (#1766); NPC-ECON-1 family BLOCKING |
| PREMIUM-1 / PREM-WIRE-1 | MERGED (#2110) |
| ECON-RET-0 / GUILD-RET-0 | MERGED (#1748) |
| GUILD0 acceptance | ACCEPTED per plan §7 status list (file Status line still reads CANDIDATE: CP to confirm) |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/domain/guild.rs`
- `apps/game-server/src/durability/guild.rs`
- `apps/game-server/src/durability/guild_audit.rs`
- `apps/game-server/src/durability/guild_tests.rs`
- `apps/game-server/src/node/guild_jobs.rs`
- `apps/game-server/tests/guild_postgres.rs`
- `docs/agents/tasks/GUILD-1.md` (this record: status and notes only)

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

- durability/mod.rs and domain/mod.rs module registration
- node World job scheduler registration (formation/vice jobs) in the existing node job list
- GAME_EVENT_FOUNDATION_REGISTRY.json (guild event, retention `GUILD_ACTIVITY_RETENTION_V1`)
- no wire in this task (GUILD-WIRE-1, Wave 5)

## Inseparable bundles (never split across PRs)

- migration + guild writer + durability tests
- World jobs + their durable checkpoints

## Acceptance criteria

- [ ] Every transaction of GUILD0 §4 has a durable test incl. concurrent invite/join and leader resign.
- [ ] Disband leaves no orphan rank/member rows; formation job is restart-safe.
- [ ] Premium gate via `premium_current(account)`.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server guild
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md §3, §4, §4.4
- docs/agents/tasks/archive/OTV2-20261004-bank-1.md (pattern)
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Local-only bounded preparation (2026-10-15)

This section records preparation on `skeleton/guild-1` at
`ac7f709b9596f4660aabf0523d0196595cf1d29e` for existing draft #2136. It does not
complete the task, freeze a candidate, allocate integration work, or change the historical
skeleton status and dependency table above. The lead supplied bound META 3.1.0 policy
`1bfb5ff98c8aa156e73669a14e083a1d464c29fb`, one writer, AUTHORING only. The
available GPT-6.1 high worker substitutes for the unavailable hard-worker model.

### Accepted source proof and scope

Before semantic code, read SOCIAL-MAP-PACKETS §2.2 and §1.2, ACCEPT-SOCIAL-MAP-0,
and GUILD-0 §3.1–§3.3 with its resource-limit rows. Fresh read-only GitHub PR metadata
confirmed these merged sources:

| PR | Merge commit | Relevance |
|---|---|---|
| #1773 | `48edf99b6b702dcf9d421f6c1930580df4e2cfd6` | Accepted packet and dependency correction |
| #1771 | `554960245f662321abe7855edc15c6397eb587ac` | Accepts GUILD-0 despite its unchanged CANDIDATE heading |
| #1766 | `9c8dadd2a8c04fc5b817361c0e325fcbf772a9ef` | BANK-1 prerequisite |
| #2110 | `d8b3191b2fc4d60e9f15c45fc0fe1e8c4fb009e6` | Premium prerequisite |
| #1748 | `19effa26e1c5e918d4603912149073b92f09361b` | Existing guild activity retention |

SOCIAL-MAP §2.2 has `candidate_bases: []` and
`depends_on: [BANK-1, PREM-WIRE-1]`. §1.2 explicitly removes the INBOX dependency;
PARTY-1 and the full NPC-ECON-1 family are not its prerequisites. The historical
aggregate holds above therefore do not govern this bounded pure preparation. No source
document or external status was rewritten. This work does not depend on the newly
accepted 41 briefs or pending #2207.

The only code write is the previously absent `apps/game-server/src/domain/guild.rs`:
reuse `CharacterName` grammar and canonical key for the separate guild namespace;
checked rank levels and rank count; the numeric rank-change and exclusion inequalities;
the ordinary-command state gate; and the strict
formation/vice-recovery deadline comparisons. Rank-name grammar is not inferred from
guild-name grammar. No duplicate name normalizer was introduced.

The deadline helper accepts explicit caller-supplied guild facts and database transaction
time. It neither reads a clock nor reconstructs current authority from immutable records.
It is only a pure predicate: session, membership, Premium, leadership uniqueness,
database locking, writes, jobs and durable recovery remain the future writer's duties.
Premium deficit observation stays job-owned and is absent from the transaction gate.

### Validation and integration limits

Rust 1.94.0 compiled `/tmp/oteryn-guild-1-pure-harness.rs` with
`rustc --test --edition 2024 -D warnings`, under `/tmp/oteryn-prep-rust.lock`.
Its `/tmp/oteryn-guild-1-pure-tests --quiet` run passed **10/10**: seven new guild tests
and the three existing character-name tests. They cover name/rank max and max+1,
Unicode name rejection and canonical collisions, strict rank inequalities, all states,
formation and 14-day vice grace at the deadline and one millisecond later, stateless
reconstruction and overflow-safe timestamp comparisons. The grace test independently
pins the accepted 14-day value, rather than deriving its expected boundary from the
implementation constant.

The external harness imports the real `character_name.rs` and `guild.rs`; it introduces
no tracked registration or substitute implementation. Repeated evaluation of supplied
times proves a stateless predicate, not durable restart recovery. SHA-256 evidence:

| Input | SHA-256 |
|---|---|
| `character_name.rs` | `a0f34a560d7f9af9de85247ec4460f6769933d12735e0789262d8e8acd2997b1` |
| `guild.rs` | `6e964f4408c348020c142e8fed672a0660207681d5490a728d76c536776b622d` |
| `/tmp/oteryn-guild-1-pure-harness.rs` | `ffd21d2fdff73887f8ec953f1a48d2aec9128716c85b47fd174f637bf00831ff` |

`cargo fmt --all --check` and direct `rustfmt --edition 2024 --check` of the
unregistered guild file passed. Governance validation passed; agent unit tests passed
**124/124**. Whole-diff self-review, tracked and untracked whitespace checks, and an
exact two-owned-path status check passed. A source-accurate standalone clippy-driver
metadata check passed under the same shared lock with `-D warnings` and the workspace
deny lints (`all`, `dbg_macro`, `expect_used`, `panic`, `todo`, `unimplemented`,
`unwrap_used`, `unused_must_use`, and forbidden `unsafe_code`). This is not a full crate
clippy run.

Module registration, migrations, registry leases, runtime wiring, persistence, jobs,
ledger/grants, and every full-task acceptance checkbox remain untouched. Cargo tests
cannot discover this deliberately unregistered file. CP and the architect must confirm
the full accepted packet and allocate integration; independent hard, persistence,
security and privacy reviews on the final frozen head, required repository validation
and protected integration remain outstanding. No commit, push, PR mutation, paid review,
regeneration, pin edit or merge was performed.
