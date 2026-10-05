# OTV2-20261004-scope-handoff-1

```yaml
task_id: OTV2-20261004-scope-handoff-1
title: "SCOPE-HANDOFF-1 house scope handoff (migration 0074)"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
lane_id: house
base_branch: main
branch: claude/scope-handoff-1-20261004
pr: 1782
base_sha: ba8b8df
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-01HCmuoFBu8ew5HMXiFva99Z (oteryn-hard-worker)
control_plane: claude-code-session-013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-04
updated_at: 2026-10-04
packet: "docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_PREMIUM_SOCIAL_PACKETS_2026-10-04.md §2.3; HOUSE-RUNTIME-0 §4 with its ADMIT-0 amendment; HOUSE-CUSTODY-0 §3.4; ADR-0001 §10; control plane answers Q1 a, Q2 a, Q3 a; amendment: game_house_access stub (OTERYN_GAME_HOUSE_RT_INBOX_PACKETS_2026-10-04 §1.2)"
leases: migration 0074 (control plane lease)
owned_paths:
  - apps/game-server/migrations/0074_house_scope_handoff.sql
  - apps/game-server/src/durability/house_scope_handoff.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/src/house_scope/mod.rs
  - apps/game-server/src/house_scope/handoff.rs
  - apps/game-server/src/lib.rs
  - apps/game-server/tests/character_authority_postgres.rs
  - apps/game-server/tests/support/house_scope_handoff_postgres_cases.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/archive/OTV2-20261004-scope-handoff-1.md
depends_on: []
public_contracts: []
external_repositories: []
```

## Outcome

- **Migration 0074.** Runtime scope assignments gain `scope_kind` (1 Channel, 2 house) and
  `house_key`; a house assignment key is `\x02 || world || house key`, `channel_id` is NULL.
  Exact-house control grants (`game_control_house_scope_grants`) feed the replaced grant and
  effect guards. Sessions gain `runtime_scope_house_key` and `origin_channel_id` (a house session
  is instance-kind with the derived instance id). `game_house_scope_handoffs` records an entry
  PREPARED (tile reserved, one open per Character, unique tile per house) and COMMITTED
  (immutable; a deferred trigger proves the source session terminal and the house session
  matching). Only the entry direction is admitted. `game_house_access(world_id, house_key,
  character_id)` is created as a stub returning no row (`role TEXT, content_fenced BOOLEAN,
  acl_revision NUMERIC(20,0), guild_revisions JSONB`); the runtime gets EXECUTE only.
- **Writer** (`durability/house_scope_handoff.rs`). `assign_house_scope` on the shared assignment
  writer high water and receipt history. `prepare_house_entry`: the live Channel session at the
  caller's fence, the Channel held by the proving node, the house assigned at the expected
  generation; refusals `HOUSE_CLOSED`, `BUSY`, `NO_ROOM` (`HOUSERT0-RL-03`, tile) write nothing.
  `commit_house_entry`: the node holds the house scope, the source session unchanged, then
  `game_house_access` is called in the commit transaction and compared with the pre-check (no
  row, another role, a changed ACL or guild revision: `NO_ACCESS`; content fence set:
  `HOUSE_CLOSED`; both abort the handoff); the source session is terminalized, the session
  use committed and a fresh house session admitted in the same transaction. `abort_house_entry`
  and `reconcile_house_entries` delete PREPARED rows only.
- **Access (Q2 a, amended).** The stub admits nobody; HOUSE-1a replaces its body. The tests
  replace the body from test code with one reading a fixture table `FOR SHARE`.
- **Exit (Q3 a).** The exit into a Channel scope and the §4.3 fallback are typed refusals
  (`ExitNotAdmitted`).
- **`house_scope`.** Re-exports, the §4.1 precheck order and entry tile selection.
- **Not here (Q1 a).** Admission guards, `admission_journal.rs` and actor wiring
  (HOUSE-RUNTIME-1) are untouched.
- **Codex round 1 (owner-approved path grant).** `runtime_scope_assignment.rs` reads a receipt
  whose assignment is a house scope as an operation conflict instead of decoding its NULL
  `channel_id`; `character_authority.rs` reads only Channel assignments (`scope_kind = 1`). The
  audit found no other world-level consumer: the others match an exact `channel_id`.
- **Codex round 2.** A session of a house instance id without its house key and origin Channel
  is refused (`23514`, 0074 `BEFORE INSERT` trigger); the replacement inheritance added here was
  removed in round 5. Revoking a house scope that is not assigned is `NotAssigned`, like the Channel writer.
- **Codex round 3 (owner-approved extra P1 round).** Commit re-reads the origin Channel
  assignment `FOR SHARE` at the source session's generation and holder before terminalizing the
  source; a replaced or revoked origin is `AuthorityRejected`. The bare-session refusal derives
  house instance ids from the house assignments (expression index
  `game_runtime_scope_house_instances`), so it holds for the first session of an assigned house.
- **Codex round 4 (architect ruling, D573).** Entry is same-node only: prepare requires the
  house scope held by the proving node, and a house assigned to another node is `BUSY` before
  anything is written (`HOUSE_CLOSED` stays for a house not assigned at the generation); commit
  keeps both same-node proofs.
- **Codex round 5 (owner decision 1b, narrowed scope).** A house session is never replaced: the
  0074 trigger refuses a candidate whose replacement receipt names a house session predecessor
  (`23514`); a reconnect into a house is a fresh entry handoff from the Channel. The inheritance
  arm and its `FOR SHARE` check are removed. `reconcile_house_entries` takes the recovering
  node's incarnation proof and aborts only the PREPARED handoffs whose origin Channel that proven
  current incarnation holds at the source generation (assignment locked `FOR SHARE`); there is
  no global cleanup.
- **Codex round 6 (D615).** A house is not assigned while a live kind-2 session holds its
  instance id without naming it (`AssignmentRejection::ScopeInUse`, and the 0074 assignment
  admission trigger); the assignment and a bare session insert take one transaction advisory
  lock on the instance id before they check. Abort of a PREPARED handoff requires the proven
  current incarnation holding the origin Channel at the source generation.
  `recover_stale_house_entries` (any proven current incarnation) deletes PREPARED handoffs
  whose origin Channel generation is no longer current and releases COMMITTED handoffs whose
  house generation is no longer current; generations are monotonic and assignment rows are
  never deleted, so the verdict is final. A COMMITTED handoff keeps its tile reserved until
  `release_house_entry_tile` by the house holder moves it to RELEASED (3), retained and
  immutable.
- **Codex round 7.** A deferred 0074 constraint trigger proves the reverse direction: a
  nonterminal session that names a house commits only as the destination of its COMMITTED or
  RELEASED handoff of the same Character, account, World, house, origin Channel and scope
  generation (`23514` otherwise). `release_house_entry_tile` requires persisted placement
  evidence: the destination session was bound to a transport (generation above the admission
  generation 1) or is terminal; a live unbound destination returns `NotPlaced` and writes
  nothing. Stale-generation cleanup stays in `recover_stale_house_entries`.

## Not in scope

- **Cross-node entry (SCOPE-HANDOFF-2).** Entering a house held by a node other than the
  origin Channel's needs a connection transfer/redirect contract that no contract owns yet.
  The "any node" fallback is deferred to the follow-up child SCOPE-HANDOFF-2.

## Tests

- Unit: house id grammar, scope key and instance id, refusal names, precheck order, tile
  selection and capacity.
- PostgreSQL 17.6 (`character_authority_postgres`, `house_scope_handoff_postgres_cases`): a
  crash after prepare (backend terminated before COMMIT) recovers the source session; a crash
  after commit (restarted root) admits once with a fresh GameSessionId; a revocation committed
  first refuses `NO_ACCESS`; a racing revocation waits on the `FOR SHARE` row and finds the
  Character inside; the stub and a changed guild revision refuse `NO_ACCESS`; exit, stale fence, closed house, busy, replay, grant, revoked house scope
  and committed-row immutability; a house assignment key reused for a Channel revoke is an
  operation conflict, and with the Channel revoked and the house still assigned bootstrap is
  refused; replacing a house session is refused (`23514`) and a bare candidate without a
  receipt is refused; a restart reconcile by another node keeps a live PREPARED handoff, the
  owning node's reconcile aborts it; a revoked house scope revoked again is `NotAssigned`; an origin Channel revoked after prepare
  fails the commit with the source session live; a bare first session of an assigned house is
  refused (`23514`); a house assigned to another node is `BUSY` at prepare with no handoff row
  or reservation; a bare session racing a house assignment makes the assignment wait on the
  instance lock and return `ScopeInUse`, and the assignment row itself is refused (`23514`);
  another node's abort is refused; with the origin Channel revoked, reconcile and abort leave
  the handoff and recovery deletes it; a committed tile stays reserved (`23505`) until the
  holder releases it, a release before placement is `NotPlaced` and keeps the handoff
  COMMITTED, the release replays and is immutable; a directly inserted house session without
  its handoff is refused at commit (`23514`); recovery releases the tile of a
  replaced house generation.

## Validation

- `cargo fmt --all --check`: pass
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server` with PostgreSQL 17.6: pass (24432 passed, 0 failed)
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass (54 tests)

## Review

Hard, security and durability review on the final frozen head; the control plane requests it.
