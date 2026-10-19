# GM-1 — GM tools, mute, kick, staff audit and reports (skeleton task record)

```yaml
task_id: GM-1
title: "GM-1: allocated OPS-GM-AUDIT-1 source authoring"
mode: IMPLEMENT
status: NOT_FULLY_QUALIFIED
active_child_task_id: OTV2-20261016-gm-audit-1
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/gm-1
base_sha: 5742f30f3
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
created_at: 2026-10-15
updated_at: 2026-10-19
owner: "chat I / root lead; admission_2187, gmaudit_2181 and sec_2188 collaborators"
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 6 row GM-1 (row 136); App. 7 GM-1; §5 Wave 5"
class: "hard (child graph)"
needs_migration: yes
needs_registry: yes
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "apps/game-server/src/staff/{mod,audit,signature}.rs (new)"
  - "apps/game-server/src/durability/staff_action.rs (new)"
  - "apps/game-server/tests/staff_audit_postgres.rs (new)"
  - "apps/game-server/tests/support/staff_audit_postgres_cases.rs (D1343: canonical production-type cases)"
  - "apps/game-server/tests/durability_postgres.rs (D1343: additive staff case mount only)"
  - "apps/game-server/migrations/0133_staff_audit.sql (D1343a: current migration lease)"
  - "apps/game-server/src/node/{config,serve}.rs (D1343d: exact staff config/control hunks; D1352: test-only spawn correction)"
  - "apps/game-server/Cargo.toml (D1343e: staff_audit_postgres test=false target entry only)"
  - "docs/contracts/game-events/v1/staff_action.proto (D1343b/c: STAFF_ACTION ID 5, schema revision 1)"
  - "docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json (D1343: additive STAFF_ACTION event and retention)"
  - "docs/contracts/RESOURCE_LIMITS_REGISTRY.json (D1343: additive CANDIDATE staff wire bounds)"
  - "crates/release-manifest/ (new; shared JCS only, D1087)"
  - "workspace-boundaries.toml (D1091: exact member/path/production and two edge hunks only)"
  - "apps/game-server/src/bin/oteryn-game-ops.rs (existing executable, signed requests)"
  - "docs/agents/tasks/GM-1.md"
  - "apps/game-server/src/{lib.rs,durability/mod.rs} (D1160: own module registration line only)"
  - "apps/game-server/tests/*.rs (D1161: 26 exact production node/staff forwarding lines only)"
  - "content/world/pins/oteryn.json (D1328: compiler-derived inputs_digest correction only)"
  - "apps/game-server/migrations/0134_staff_moderation.sql (D1437: minimal moderation writers; WRITE-MOD-1 not on main)"
  - "apps/game-server/src/durability/staff_moderation.rs (D1437: staff executor, sanctions, item-text reports)"
  - "apps/game-server/src/staff/live.rs (D1437: live staff desk, kick and mute fan-out)"
  - "apps/game-server/src/gameplay_transport/{chat_intent,connection,mod,qualification,capabilities_tests}.rs (D1437: staff mute refusal, kick-to-fenced-logout and desk registration hunks only)"
  - "apps/game-server/src/node/serve.rs (D1437: staff executor and desk wiring hunks)"
slot_only_paths:
  - "Cargo.toml and further apps/game-server/Cargo.toml integration outside the exact D1343e test-target hunk (CP grant required; D1328 integration already landed in 647053e9)"
  - "migrations: game_staff_roles, game_player_reports (D1437: sanctions and item-text reports landed in 0134; roles stay in the signed roster)"
  - "docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json (staff panel)"
  - "apps/game-server/src/gameplay_transport/capabilities.rs, crates/protocol-oteryn/src/lib.rs (connection.rs and mod.rs GM-1 hunks are owned under D1437)"
  - "Other handwritten wiring and contract changes require the current CP allocation. D1328 removes exclusive slots for generated Cargo.lock, pins and registries; regenerate those outputs after merge-up."
```

## Current checkpoint — local Codex review of `628415ce`: restore tail, replay effects, staff audit expiry (2026-10-19)

Parent `628415ce`, then `origin/main` merged with a merge commit. Findings
from the pre-push Codex review, all accepted:

- P1 `staff_moderation.rs` restore: retained links above the restored tip
  that had no outcome were dropped silently once the chain stopped extending.
  A link past a gap was lost without an operator decision. Every leftover
  candidate above the extended tip now returns `RestoreError::Ambiguous`
  with its operation ids, and nothing is applied. Test:
  `staff_pg_restore_refuses_a_retained_tail_past_a_gap_without_outcomes`.
- P2 `staff_moderation.rs` replay: an identical retry of a COMMITTED request
  returned the stored outcome with no live effect. If the node had lost the
  effect (for example after a restart), it was never restored. A committed
  replay now re-syncs the desk from database truth:
  - Mute and Unmute set the live mute to the latest ACTIVE sanction end.
    This is idempotent and needs no clock.
  - Kick applies nothing on replay. A kick is bound to the session present
    at first execution, so a retry never kicks a reconnected session. A lost
    kick needs a fresh signed request.
  - A refused replay applies nothing.

  Test: `staff_pg_executor_identical_retries_resync_committed_mutes_and_never_rekick`.
- P2 `staff_action.rs` outbox retention: envelopes carried `expires_at` but
  nothing ever deleted them. Migration 0133 adds the SECURITY DEFINER
  `game_staff_expire_action_audit(p_batch)`, modelled on
  `game_character_expire_audit`:
  - batches are bounded to 1..64 and it reads the database clock;
  - it deletes only after `expires_at + 900000`, where 900000 is
    EXECUTION_WINDOW_MS + FUTURE_SKEW_MS (the lib test
    `staff_audit_expiry_grace_matches_signature_window` pins this);
  - it never deletes an envelope whose mutation still awaits its receipt.

  Operations, links and receipts are never deleted. Runtime has EXECUTE
  only, and still no DELETE on the outbox. `DurabilityRoot::expire_staff_action_audit`
  is called from `serve.rs` `audit_expiry_loop` (`event=staff_audit_expiry`).
  Test: `staff_pg_staff_audit_envelopes_expire_after_retention_and_verification_still_passes`.
  It also shows that the full evidence walk and the orphan check still
  pass after expiry.

Second pre-push review round (lock order, cross-node mutes, kick):

- Report admission no longer deadlocks with `REPORT_CLEAR_TEXT`. It takes
  the world-scoped item KEY SHARE lock before it shares the text row. The text writers use item instance,
  then revision, then text. Test:
  `staff_pg_report_admission_locks_the_item_before_its_text`.
- A mute or unmute committed on another node now reaches this node's live
  chat. Each node re-syncs its live Characters' staff mutes from the
  database every 5 s (`staff_mute_refresh_loop` in `node/serve.rs`), in
  statements of at most 256 Characters; chat stays a map lookup.
  Per-Character sequences keep a stale refresh from overwriting a newer
  local mute or unmute, and a mute chat admission loads before its
  connection registers is kept. Test:
  `staff_pg_mute_refresh_resyncs_other_nodes_and_scopes_the_environment`.
  Unit tests: `mute_refresh_sets_and_clears_live_characters`,
  `mute_refresh_ignores_a_stale_snapshot_after_set_mute`,
  `mute_refresh_drops_non_live_entries` and
  `mute_refresh_keeps_a_mute_loaded_before_its_registration`.
- Both PostgreSQL tests run in the `durability_postgres` job in
  `pr-postgres-harness.yml` and `merge-group-gate.yml`.

Third pre-push review round (CP-approved, bounded batches):

- Report expiry no longer stops after 64 reports. One hourly
  `expire_item_text_reports` call loops over batches of
  `REPORT_EXPIRY_BATCH` (each report in its own transaction) until none is
  left, then clears expired snapshots in statements of at most 1024 rows
  until none is left. Test:
  `staff_pg_report_expiry_drains_all_batches_in_one_call` (70 reports).
- Restore no longer applies the whole retained tail in one transaction
  under the two-second deadline. A fully verified tail is applied in
  batches of `RESTORE_BATCH` (16), each in its own transaction that checks
  the head left by the previous batch. A retry after a stop or a failed
  batch skips what committed and applies the rest. Test:
  `staff_pg_restore_batches_commit_and_retry_idempotently` (19 links, a
  prefix retry and a failure in batch two that rolls back only that batch).
- Both tests run in the `durability_postgres` job in
  `pr-postgres-harness.yml` and `merge-group-gate.yml`.

Fourth pre-push review round (receipt signer, disconnected mutes):

- Restore checks, before any batch commits, that each mutation's receipt
  binds the trust's environment and roster and is signed by the request's
  principal and key; `receipt_in_transaction` would otherwise reject it in a
  later batch after the mute had committed. A batch also ends only where
  every mutation in it has its receipt. Tests:
  `staff_pg_restore_refuses_a_receipt_from_another_signer_before_any_batch_commits`
  and `staff_pg_restore_keeps_a_mutation_and_its_receipt_in_the_same_batch`.
- The mute re-sync no longer evicts the mute of a Character whose
  connection dropped. A same-session resume does not reload the durable
  mute, because chat still knows the session, so the eviction let a muted
  Character chat. The desk now keeps a cached mute until it ends, and the
  re-sync also covers cached, non-live Characters, so a mute lifted on
  another node is cleared. The cache holds only unexpired mutes. Unit test:
  `mute_refresh_keeps_a_disconnected_mute_until_it_ends_or_lifts` (it
  replaces `mute_refresh_drops_non_live_entries`, which asserted the
  eviction); the PostgreSQL case
  `staff_pg_mute_refresh_resyncs_other_nodes_and_scopes_the_environment`
  now drops desk_b's connection before the refresh and the unmute.
- The PostgreSQL tests run in the `durability_postgres` job.

Fifth pre-push review round (unmute clock skew, sanction verification):

- An unmute no longer fails when the unmuting node's clock is behind the
  node that wrote the mute. The lift time stays the unmute's own signed
  execution time, which the bound trigger requires, so migration 0134 now
  checks `lifted_at>0` instead of `lifted_at>=starts_at`. Before, the CHECK
  rolled back a valid signed unmute and left the Character muted. Test:
  `staff_pg_executor_unmute_on_a_node_clock_behind_the_mute_start_still_lifts`.
- `oteryn-game-ops staff audit-verify` and the restore preflight now
  compare every sanction row with its retained signed operations, the same
  binding the `game_character_sanction_bound` trigger checks at write time
  (without the same-transaction and roster parts, which do not hold for
  history). A row that a database owner rewrote with the triggers off, for
  example a longer `ends_at`, now fails verification. Test:
  `staff_pg_sanction_check_detects_rows_rewritten_past_their_signed_operation`;
  `staff_pg_staff_audit_envelopes_expire_after_retention_and_verification_still_passes`
  also asserts that a genuine history passes the check.
- The PostgreSQL tests run in the `durability_postgres` job.

Sixth review round (Codex threads on `a4af837c`):

- P1 `staff_moderation.rs` restore, a tail MUTE then UNMUTE: verified and
  kept by contract. ALPHA-OPS ruling 8 (*Replay after restore*, *Order and
  scope*) says no unmute, lift or hold release is replayed; one committed
  after T is lost and is signed again. The replayed mute gets a new
  sanction id, so the lost unmute's `UNMUTE{sanction_id}` refuses
  `TargetNotFound`, and a fresh signed unmute of the restored id lifts it.
  The restore now records the skipped unmute (below), so the gap is
  visible to the operator. Test:
  `staff_pg_restore_keeps_a_replayed_mute_until_a_fresh_signed_unmute`.
  Open owner question below.
- P1 `staff_moderation.rs` and `oteryn-game-ops.rs`: a COMMITTED MUTE with
  no sanction row went undetected, because the check ran only from
  sanction rows to operations. Migration 0134 adds the immutable
  provenance table `game_staff_restore_unapplied` (operation id,
  environment, `NOT_REAPPLIED` or `REFUSED_NOW`), written only by restore in
  the operation's own transaction and bound to it by a deferred trigger.
  The check, now `staff_moderation_has_unbound_rows`, also runs the
  reverse way: every COMMITTED, OK MUTE, UNMUTE, REPORT_CLOSE and
  REPORT_CLEAR_TEXT in the environment must have exactly one of its effect
  or a valid provenance row, and an orphan or invalid provenance row fails.
- P2 `oteryn-game-ops.rs` audit-verify and the restore preflight: report
  rows are now compared with their signed REPORT_CLOSE and
  REPORT_CLEAR_TEXT operations, the binding `game_item_text_report_bound`
  checks at write time: kind, report id, world scope, CLOSED state with
  `ended_at`, and the cleared text revision. The control role gets a
  column SELECT on `game_item_text_reports` without `snapshot_text`.
- Tests: `staff_pg_moderation_check_detects_missing_effects_and_rewritten_reports`
  and `staff_pg_restore_records_each_unapplied_effect_and_stays_verifiable`.
  The test `restore_to` helper now also reverts report closes, text clears
  and lifts made above the restored head, as a point-in-time restore would.
- The PostgreSQL tests run in the `durability_postgres` job. There is no
  protocol or wire change.

Wiring is unchanged. Production staff requests reach the executor through
`control_loop`, then `ControlInput::Staff`, then `serve_staff_control`. The
e2e case is `staff_pg_executor_mute_inspect_unmute_and_kick_follow_the_committed_audit`,
run by the `durability_postgres` job (`pr-postgres-harness.yml` and
`merge-group-gate.yml`). There is no protocol or wire change.

CP decisions (2026-10-19):

- Roster provisioning: accepted. Moderation writes need a
  `game_staff_rosters` row for the active roster, and only the owner can
  insert it. Until the owner bootstrap inserts those rows, moderation
  fails closed.
- D1161: CP-granted D1161 note 2026-10-19, for the four integration-test
  wrapper re-export lines.
- Kick: alpha keeps the D1437 fenced-logout kick, which answers IN_FIGHT
  for an in-fight session. This PR has no protocol change.

Open owner questions:

- Kick of an in-fight session. An immediate StaffKick that ignores the
  combat lock needs TerminalRelease::StaffKick and wire code 1201 (A7), a
  protocol change outside GM-1. Options: (a) keep the D1437 fenced-logout
  kick with the IN_FIGHT answer for alpha; (b) allocate a follow-up packet
  with protocol review. Recommendation: (a) now, (b) before external
  players.
- Restore of a tail MUTE then UNMUTE. ALPHA-OPS ruling 8 never replays an
  unmute, so the restored mute stays active until a fresh signed unmute of
  its new sanction id; the restore records the skipped unmute as
  `NOT_REAPPLIED`. Options: (a) keep ruling 8; (b) amend ruling 8 to
  replay a tail unmute of a mute replayed in the same restore. Option (b)
  needs the replayed sanction id carried into the unmute, which is a
  contract change. Recommendation: (a).

## Previous checkpoint — merge-up, Codex review of `4b23100d`, renumbering to 0133/0134 (2026-10-18)

Parent `4b23100d`. In this round:

- `origin/main` was merged with a merge commit, and the conflicts in
  `durability/mod.rs`, `node/config.rs`, `tests/durability_postgres.rs`
  and `RESOURCE_LIMITS_REGISTRY.json` were resolved. Both sides are kept.
- D1453 renumbered the migrations: `0116_staff_audit.sql` is now
  `0133_staff_audit.sql` (`version=133`) and `0123_staff_moderation.sql`
  is now `0134_staff_moderation.sql`. Every reference was updated.
- The integration-test wrappers re-export `production_server::{node, staff}`
  (wave1 fix). These four lines exceed the D1161 line count, so they need a CP note.

Open findings on `4b23100d`, all accepted:

- [P1 PRRT_kwDOT8SzxM6tscmJ](https://github.com/Oteryn/Oteryn-Game/pull/2181):
  signer tiers were not enforced on direct moderation writes. A member LOGIN
  could commit a REPORT_CLEAR_TEXT or an ADMIN-only sanction signed by a GM.
  The fix has three parts:
  - Migration 0134 adds the owner-only, immutable table `game_staff_rosters`,
    which holds the roster bytes for each `(environment, roster_sha256)`.
  - It adds `game_staff_signer_allows(environment, principal, roster, kind)`.
    This function resolves the active roster from the head's ACTIVATION
    link, takes the principal's one `# oteryn-staff-role` line and applies
    the same tier table as `Role::allows`.
  - The sanction and report bound triggers now refuse any write whose signer
    tier does not allow the action.

  Test: `staff_pg_member_logins_cannot_commit_a_moderation_write_above_the_signer_tier`.

  **Provisioning:** the owner must now insert the roster row along with each
  activation. Without it, every moderation write fails closed at commit.
- [P1 PRRT_kwDOT8SzxM6tscmL](https://github.com/Oteryn/Oteryn-Game/pull/2181):
  restore checked operation evidence only at the head. The restore loop now
  runs `staff_action::verify_request_evidence` for every request link in the
  restored prefix. The ops `audit-verify` command uses the same check.
  Test: `staff_pg_restore_verifies_operation_evidence_below_the_head`.
- [P2 PRRT_kwDOT8SzxM6tscmN](https://github.com/Oteryn/Oteryn-Game/pull/2181):
  active-mute lookups were not scoped to the staff environment. The Mute,
  Unmute and Inspect paths and the live chat mute check now filter on the
  environment. `StaffDesk` carries the trust environment for this. Test:
  `staff_pg_active_mutes_are_scoped_to_the_staff_environment`.

Wiring: production staff requests reach the executor through `control_loop`,
then `ControlInput::Staff`, then `serve_staff_control`. The e2e case is
`staff_pg_executor_mute_inspect_unmute_and_kick_follow_the_committed_audit`,
run by the `durability_postgres` job (`pr-postgres-harness.yml` and
`merge-group-gate.yml`). Follow-ups: DATA-RESTORE-OPS-2 and ARCH-CONTROL-ANCHOR-0.

## Historical checkpoint — Codex review of `7e2f70bc` repairs and staff restore replay, REPORT_ONLY (2026-10-18)

Parent `7e2f70bc`. This round has three parts.

**Already resolved before this round.** The four cited P2s were fixed and
resolved earlier: `PRRT_kwDOT8SzxM6tffQV`, `PRRT_kwDOT8SzxM6tf6Xp`,
`PRRT_kwDOT8SzxM6thcai` and `PRRT_kwDOT8SzxM6tgnXA`. The earlier P1/P2
findings are also resolved, each with a test that still exists:

| Thread | Finding | Fixed in | Test |
| --- | --- | --- | --- |
| `tkDg-` | member LOGINs forge request links | `39fa9d3b` | `staff_pg_member_logins_cannot_roll_back_head_or_get_forged_links_extended` |
| `tkDhA` | head monotonicity | `39fa9d3b` | the same case |
| `tkDhD` | null scope on scoped actions | `39fa9d3b` | lib `ordinary_requests_require_signed_world_and_channel_scope` |
| `tl8ns` | genuine links with forged rows | `aeaf11f1` | `staff_pg_member_logins_cannot_get_genuine_links_with_forged_rows_extended` |
| `tl8nt` | activation before the pending receipt | `aeaf11f1` | `staff_pg_activation_waits_for_the_pending_mutation_receipt` |
| `tqCoi` | REPORTS_LIST pagination | `7e2f70bc` | `staff_pg_executor_reports_list_pages_past_one_page_by_signed_cursor` |
| `tffQU` | member LOGINs insert activations | earlier | `staff_pg_member_logins_cannot_insert_activations` |

**Open findings on `7e2f70bc`, all accepted:**

- [P1 PRRT_kwDOT8SzxM6tq-Vs](https://github.com/Oteryn/Oteryn-Game/pull/2181):
  a report transition was not bound to its signed action. A member LOGIN
  could close, dismiss or clear a report directly. The deferred constraint
  trigger `game_item_text_report_bound` now requires that each state change
  or snapshot clear names an operation that:
  - was committed in the same transaction and environment;
  - has a COMMITTED/OK outcome;
  - signs that transition for this report.

  Otherwise it raises `23514`, `report transition is not its signed staff
  action`.
- [P1 PRRT_kwDOT8SzxM6tq-Vu](https://github.com/Oteryn/Oteryn-Game/pull/2181):
  a snapshot clear had no retention check. The transition trigger now
  refuses to clear `snapshot_text` while `expires_at` is still ahead of the
  database clock (`immutable staff evidence`). The executor's expiry also
  uses the database clock.
- [P2 PRRT_kwDOT8SzxM6tq-Vy](https://github.com/Oteryn/Oteryn-Game/pull/2181):
  a report that changed state after a page was read could fall behind the
  cursor and be skipped. Every insert or state change now draws a fresh
  `entry_seq` from a sequence, under the worlds row lock. The list pages by
  `(entry_seq, report_id)`, and the signed cursor is `{entry_seq,
  report_id}`. A report that changes state after a page moves past the
  cursor, so the next page still reaches it.
- `to430` (receipt disposition): a receipt now always carries its outcome's
  own disposition. Only a COMMITTED receipt of a COMMITTED mutation releases
  the fence. An optional REFUSED receipt records a refusal for restore, as
  ALPHA-OPS ruling 8 asks. `staff prepare-receipt` now refuses only reads.

**Restore path (boundary rows 13-15).** The repo's durability restores follow
one pattern. An operator-only `oteryn-game-ops` command runs while the node is
stopped, through its own `[section]` login, and checks before it writes. GM
data now follows it with `oteryn-game-ops staff restore-replay` and
`DurabilityRoot::staff_restore_replay`. It applies ALPHA-OPS ruling 8 to the
retained `staff-link-*` files:

1. It verifies the restored chain against the retained links. A gap, fork
   or changed link stops the replay and applies nothing.
2. It walks the links after the restored head:
   - A COMMITTED MUTE with a verifying receipt is re-run once with its own
     checks (role, duration, target in the signed world), without
     freshness. A check that now fails applies nothing (`RefusedNow`).
   - A REFUSED receipt stores the refusal. A resubmit returns the stored
     refusal bytes.
   - A mutation without a verifying receipt, or a link without its outcome,
     is ambiguous. The command stops and applies nothing.
3. Other COMMITTED mutations are recorded but not re-applied; only mutes are
   replayed.
4. Everything is written in one semantic-pass transaction. It first checks
   that the locked head is still the verified one.

New cases:

| Row | Test |
| --- | --- |
| 9 (restore side) | `staff_pg_restore_stops_on_a_gap_a_fork_or_a_changed_link` |
| 13 | `staff_pg_restore_replays_receipted_mutes_once_and_rechecks_them` |
| 14 | `staff_pg_restore_stores_receipted_refusals_and_resubmits_return_them` |
| 15 | `staff_pg_restore_without_a_verifying_receipt_applies_nothing` |
| Vs | `staff_pg_member_logins_cannot_end_or_clear_reports_without_their_signed_action` |
| Vu | `staff_pg_report_snapshots_stay_until_retention_ends_on_the_database_clock` |
| Vy | `staff_pg_reports_closed_after_a_page_stay_reachable_by_its_cursor` |
| to430 | `staff_pg_executor_receipts_match_their_outcome_and_committed_ones_release_fences` |
| ops | bin `staff_restore_replay_connects_only_the_restore_login` |

The restore path does not yet cover these. Each is named with its owner, and
none blocks this PR:

- **DATA-RESTORE-OPS-2** (no PR yet): the full `oteryn-game-ops restore`
  sequence (lock, journal, fence) and `restore resolve` for an ambiguous
  link. Until it lands, an ambiguous link stops the replay and needs the
  owner.
- **ARCH-CONTROL-ANCHOR-0**: an off-host signed head. Until then, a restore
  that lost both the database head and the retained files cannot detect a
  truncated tail.

Validation on WSL `Ubuntu` (Rust 1.94, `cargo fetch --locked` first, PG
through the unchanged `in-pg-netns.sh` against shared PostgreSQL 17.6, D1450):

| Check | Result |
|---|---|
| `cargo fmt --all --check` | pass |
| `tools/agents/validate_governance.py` | pass |
| `cargo clippy -p oteryn-game-server --all-targets -- -D warnings` | pass |
| lib `staff chat_intent logout_wire config` | 80 passed, 0 failed |
| bin `oteryn-game-ops` | 8 passed, 0 failed |
| `--test staff_audit_postgres` | 1195 passed, 0 failed |
| `--test durability_postgres staff_pg_` | 35 passed, 0 failed |

Runtime remains REPORT_ONLY.

## Previous checkpoint — Codex review of `adc0e0a5` repairs, REPORT_ONLY (2026-10-18)

Parent `adc0e0a5`. All three findings were accepted:

- [P1 PRRT_kwDOT8SzxM6tqCoh](https://github.com/Oteryn/Oteryn-Game/pull/2181#discussion_r4306858434):
  a member LOGIN could insert an ACTIVE mute that named any staff action id,
  for example an INSPECT. Migration 0134 now binds every sanction row to its
  own signed action through the deferred constraint trigger
  `game_character_sanction_bound` (`23514`, `sanction is not its signed staff
  action`). The trigger requires that the named operation row:
  - was committed in the same transaction (`created_xact_id`) and environment;
  - has a COMMITTED/OK outcome;
  - carries a signed MUTE for this Character, with the same duration, start
    (`occurred_at`) and signer;
  - for a lift, carries a signed UNMUTE of this sanction at `lifted_at`.

  INSERT and UPDATE on sanctions are now granted only to the executor's
  runtime role; control keeps SELECT only.
- [P2 PRRT_kwDOT8SzxM6tqCoi](https://github.com/Oteryn/Oteryn-Game/pull/2181#discussion_r4306858437):
  REPORTS_LIST had no cursor past its first 32 reports. The signed action now
  carries `after: {created_at, report_id} | null`. The executor pages by the
  `(created_at, report_id)` keyset, and the answer returns `truncated` and the
  `next` cursor. A page also ends early at the answer byte bound. Because a
  snapshot is at most 7996 bytes, every page holds at least one report.
- [P2 PRRT_kwDOT8SzxM6to430](https://github.com/Oteryn/Oteryn-Game/pull/2181#discussion_r4306375624):
  a REFUSED receipt released the fence. A receipt must now be COMMITTED and
  acknowledge a COMMITTED stored outcome, both in the fence and in the receipt
  writer. `staff prepare-receipt` refuses any other receipt. Only a COMMITTED
  mutation sets `requires_receipt` (`audit::requires_receipt`, derived from the
  signed action and outcome). A refused mutation changed nothing, so it awaits
  no receipt, and its audit evidence expires like a read's. Residual risk: a
  forged unsigned REFUSED outcome row is not fenced. It cannot carry an
  effect, because no sanction binds to a non-COMMITTED operation.

CP decisions on the earlier owner questions:

- **D1448b, namespace.** The signing namespaces stay `oteryn-ops-request-v1`
  and `oteryn-ops-submission-receipt-v1`. D1437's `oteryn-staff` is superseded.
- **Migration number.** `0123` is renumbered at the queue turn. #2203 also
  holds `0123` provisionally, so whichever PR merges later renumbers.

New canonical cases:

- `staff_pg_executor_only_committed_receipts_release_mutation_fences`;
- `staff_pg_member_logins_cannot_forge_sanctions_from_other_actions`, for
  runtime and control LOGINs;
- `staff_pg_executor_reports_list_pages_past_one_page_by_signed_cursor`, with
  70 open reports paged 32/32/6 and large closed reports paged at the byte
  bound.

There is also a new lib case, `reports_list_cursor_names_a_report_uuid`.
The existing mute/inspect/unmute/kick case now expects 4 receipts instead of 6,
because its refused UNMUTE and KICK no longer take one.

Validation on WSL `Ubuntu` (Rust 1.94, `cargo fetch --locked` first, PG
through the unchanged `in-pg-netns.sh` against shared PostgreSQL 17.6, D1450):

| Check | Result |
| --- | --- |
| `cargo fmt --all --check` | pass |
| strict Clippy `-p oteryn-game-server --all-targets -D warnings` | pass |
| strict Clippy `--test staff_audit_postgres -D warnings` | pass |
| lib `-- staff chat_intent logout_wire` | 57 passed, 0 failed |
| `staff_audit_postgres` | 1188 passed, 0 failed |
| canonical `durability_postgres staff_pg_` | 28 passed, 0 failed |
| `python3 tools/agents/validate_governance.py` | passed (22 policy documents, 9 project lanes) |

Runtime remains REPORT_ONLY.

## Previous checkpoint — D1437 mechanic complete and wired, REPORT_ONLY (2026-10-18)

Parent `aeaf11f1`, merged with `main` `64d5e4da` (merge `bf88a534`). CP D1437
resolved the architect UNKNOWNs; where the PR already implied a choice it is kept:

- **Signed bytes.** The request and receipt rows already hash JCS
  (`oteryn_release_manifest::jcs`), so JCS stays the canonical encoding. The
  `ssh-keygen -Y` namespaces stay `oteryn-ops-request-v1` and
  `oteryn-ops-submission-receipt-v1`. D1437 named `oteryn-staff`; see the owner question below.
- **Roster.** The server-side signed roster loaded at boot (`[staff.trust]`
  plus `activation_history_file`) now also carries exactly one role comment
  line per signer: `# oteryn-staff-role <principal> GM|SENIOR_GM|ADMIN`. The
  roster digest covers that line and OpenSSH ignores it. Boot fails closed
  (`trust.roles()`) on a role that is missing, duplicated, malformed or unknown. The role decides
  the action: GM may Inspect, ReportsList, Kick, Mute, Unmute and ReportClose;
  SENIOR_GM may also ReportClearText; ADMIN may run every action.
- **Writers.** WRITE-MOD-1 is not on `main`, so this PR owns the minimal
  writers: migration `0134_staff_moderation.sql` (sanctions, item-text reports
  and the WRITE0-RL-04..06 counters) and `durability/staff_moderation.rs`.

What runs in the game now:

- `serve_staff_control` routes every signed request through the audited
  executor. It verifies the request, then checks the scope (node world and
  channel), the role tier and the mute bound. The action, its sanction or
  report rows, the operation row and the outbox row all commit in one
  transaction. A failed audit insert rolls back everything, and no live
  effect runs.
- **Mute / Unmute** write `game_character_sanctions`. After the commit, the
  node's `StaffDesk` refuses that Character's chat. Chat admission also reads
  `staff_active_mute_until`, so the mute survives a restart or reconnect.
- **Kick** reaches the connection registered on the desk at admission. It
  runs that session's real fenced logout release (LOGOUT-WIRE-1): an accepted
  release ends the session as a logout; an in-fight or busy session keeps
  playing; an unknown release ends it as a disconnect.
- **Inspect** and **ReportsList** are audited reads.
- **REPORT-3 (WRITE0 §6):** item-text reports are admitted with dedup, then
  the hourly (10), account (20) and world capacity limits, then the
  current-text check. Staff can close a report or clear its text with a
  revision fence. Expiry runs on the node's maintenance cadence.

- **Audit build id (found by the executor tests).** `audit::encode` checked
  the server build id with the signed-field `token` alphabet. The node's
  default `oteryn-game-server/<version>` carries a `/`, so every live staff
  action failed as `Invalid` before its audit row. The check now takes 1-128
  visible ASCII bytes; lib
  `the_running_server_build_id_is_an_accepted_envelope_build` covers it.
- **Report close clock (found by the executor tests).** A close stamped
  `ended_at` with the staff request clock, which may precede the report's
  `created_at` from the reporter's node clock. The row CHECK then refused the
  close as a database error. A close now ends at `GREATEST(now, created_at)`.

The 18 boundary rows map to tests as follows. All 18 rows are covered; rows
13-15 (restore) were added in the `7e2f70bc` repair checkpoint above.

| Row | Covered by |
| --- | --- |
| 1 | lib `missing_signature_unknown_principal_and_wrong_namespace_are_independent` |
| 2 | `environment_roster_and_parameter_binding_have_no_unsigned_sibling` |
| 3 | `authenticated_unknown_duplicate_missing_and_noncanonical_fields_refuse` |
| 4 | `first_execution_clock_bounds…` and PG `staff_pg_stale_future_clock…` |
| 5-6 | PG `staff_pg_identical_expired_retry…` |
| 7 | PG `staff_pg_stale_future_clock_sequence_previous_and_roster…` and `old_loaded_config…` |
| 8 | PG `staff_pg_same_predecessor_two_connections_have_one_typed_loser` |
| 9 | lib `chain_gaps_forks_digest_substitution_and_unrecorded_activation_fail` |
| 10 | lib `removed_key_cannot_append…` and `receipts_cannot_replace_recorded_activation_anchors` |
| 11 | lib `event_kind_target_reason_parameters_and_missing_bytes_each_fail_binding` |
| 12 | PG `staff_pg_receipt_requires_exact_digests…` and lib `receipt_signature_namespace…` |
| 13 | PG `staff_pg_restore_replays_receipted_mutes_once_and_rechecks_them` |
| 14 | PG `staff_pg_restore_stores_receipted_refusals_and_resubmits_return_them` |
| 15 | PG `staff_pg_restore_without_a_verifying_receipt_applies_nothing` |
| 16-17 | PG `staff_pg_failed_audit_insert_rolls_back…` and `staff_pg_executor_audit_failure_rolls_back_the_action_and_its_effect` |
| 18 | lib `retained_operation_mirrors_verify_without_any_outbox_row` and `unsigned_read_timestamp_cannot_excuse_missing_unexpired_audit` |

New integration cases go through the real dispatch path
(`serve_staff_request`, the runtime LOGIN and the receipt chain):

- `staff_pg_executor_mute_inspect_unmute_and_kick_follow_the_committed_audit`;
- `staff_pg_executor_refuses_scope_bounds_tier_and_foreign_targets`;
- `staff_pg_executor_without_a_roster_role_fails_closed_before_any_write`;
- `staff_pg_executor_audit_failure_rolls_back_the_action_and_its_effect`;
- `staff_pg_item_text_reports_admit_close_clear_and_expire_under_write0_limits`.

New connection cases in `capabilities_tests::logout_wire` cover a kick that
releases, a kick of an in-fight or busy session, and a kick whose release is
unknown.

Remaining:

- A mute or unmute committed on another node reaches live chat on the 5 s
  desk re-sync, subject to a successful database refresh. Chat open also
  loads the active mute through `staff_active_mute_until`.
- The player-facing report intent is WRITE-WIRE-1; only the storage writer
  exists here.
- Map-object reports are out of scope.

Owner questions:

- **Namespace.** D1437 says `oteryn-staff`, but the PR signs with
  `oteryn-ops-request-v1` / `oteryn-ops-submission-receipt-v1`.
  Recommendation: keep these, because the existing chain and evidence verify
  under them.
- **Migration number.** `0123` is free on `main` `64d5e4da` today. It is
  renumbered at the queue turn if taken.

Validation of Rust tree `64102214`, before this card's results table (Ubuntu/mole,
shared `~/oteryn-cache`, one CP D1429 `slot-run.sh` slot per run,
`apps/game-server/src/lib.rs` touched before every cargo run, `cargo fetch
--locked` first, PG through the unchanged `in-pg-netns.sh` against shared
PostgreSQL 17.6):

| Check | Result |
| --- | --- |
| RED: the executor cases before the build-id fix | 4 failed (`Rejected(Invalid)` before any audit row) |
| RED: the reports case before the close-clock fix | 1 failed (`game_item_text_reports_check1` on close) |
| `cargo fmt --all --check` | pass |
| strict Clippy `-p oteryn-game-server --all-targets -D warnings` | pass |
| strict Clippy `--test staff_audit_postgres -D warnings` | pass |
| lib `-- staff chat_intent logout_wire` | 56 passed, 0 failed |
| `staff_audit_postgres` | 1185 passed, 0 failed |
| canonical `durability_postgres staff_pg_` | 25 passed, 0 failed |
| `python3 tools/agents/validate_governance.py` | passed (22 policy documents, 9 project lanes) |

Runtime remains REPORT_ONLY.

## Previous checkpoint — Codex review of `39fa9d3b` repairs, REPORT_ONLY (2026-10-18)

Parent `39fa9d3b`, not behind `main` `afa97ba0`, so no merge. Both findings were accepted:

- [P1 4305145537](https://github.com/Oteryn/Oteryn-Game/pull/2181#discussion_r4305145537):
  a member LOGIN could pair a genuinely signed REQUEST head link with forged
  unsigned rows (`requires_receipt=false` for a mutation, or a forged audit
  event). Before a request, `staff prepare` or a receipt extends a REQUEST head,
  `verify_head_operation_in_transaction` now binds its operation row through
  `audit::verify_stored_operation`, which derives the receipt requirement from
  the signed action. It also binds its outbox event through
  `audit::verify_stored_event`. The event may be absent only after its signed
  retention expiry.
- [P2 4305145538](https://github.com/Oteryn/Oteryn-Game/pull/2181#discussion_r4305145538):
  migration 0133 refuses an ACTIVATION link (`23514`, `staff activation awaits
  pending receipt`) while any receipt-requiring operation lacks its receipt, so
  a rotation cannot strand the pending receipt.

New canonical cases `staff_pg_member_logins_cannot_get_genuine_links_with_forged_rows_extended`
(runtime and control LOGINs, both forgeries) and
`staff_pg_activation_waits_for_the_pending_mutation_receipt`. Both fail on the
parent and pass on this tree. Integration-rest run 39080056876 belongs to
#2167, not this PR: its head has every check green.

Validation of the four source files at this commit (Ubuntu/mole, shared `~/oteryn-cache`, one
CP D1429 `slot-run.sh` slot per run, `apps/game-server/src/lib.rs` touched
before every cargo run, PG through the unchanged `in-pg-netns.sh` against
shared PostgreSQL 17.6, `PRE-ROUTING` skips 0):

| Check | Result |
| --- | --- |
| RED: the two new cases on the parent's `src` and migration | 2 failed (all 8 forged-row probes admitted; activation stranded the receipt) |
| `cargo fmt --all --check` | pass |
| strict Clippy `-p oteryn-game-server --all-targets -D warnings` | pass |
| lib `-- staff` | 18 passed, 0 failed |
| `staff_audit_postgres` | 1169 passed, 0 failed |
| canonical `durability_postgres staff_pg_` | 20 passed, 0 failed |

Runtime remains REPORT_ONLY.

## Previous checkpoint — Codex review 5559897716 repairs, REPORT_ONLY (2026-10-18)

Parent `c05cd337`, merged with `main` `f094e032` and `afa97ba0` (resource registry: main
plus the two GM1 staff entries; World pin `inputs_digest` recomputed). All
three findings were accepted:

- [P1 4304344319](https://github.com/Oteryn/Oteryn-Game/pull/2181#discussion_r4304344319):
  a member LOGIN could commit a forged link plus the head advance without a
  receipt-requiring operation, and the next request chained after it.
  `verify_head_in_transaction` now re-verifies the stored head link (digest,
  roster, kind, signature, activation binding) under the current trust before
  a request, `staff prepare` or a receipt extends it. The head's
  `activation_sha256` is not writable by those LOGINs, so the current
  activation covers every link from it to the head.
- [P2 4304344321](https://github.com/Oteryn/Oteryn-Game/pull/2181#discussion_r4304344321):
  migration 0133 adds `game_staff_head_successor`, a BEFORE UPDATE trigger: the
  head may only move to the exact successor link written in the same
  transaction, and only an ACTIVATION link may change `activation_sha256`.
- [P2 4304344325](https://github.com/Oteryn/Oteryn-Game/pull/2181#discussion_r4304344325):
  a roster activation must carry no scope, and every other action must carry
  one, at both prepare and verify.

New canonical case
`staff_pg_member_logins_cannot_roll_back_head_or_get_forged_links_extended`
covers the runtime and control LOGINs: head rollback and stay are refused with
`23514`, and a committed forged head link is refused by the next receipt,
`staff prepare` and the next request. Counts and head stay unchanged. The
owner activation fixture in `staff_pg_member_logins_cannot_insert_activations`
now also advances `activation_sha256`, as every lawful activation must. The
`[staff.trust]` + `activation_history_file` shape is unchanged (CP decision).

Validation of tree `0034b7de`, before this card edit (Ubuntu/mole, shared `~/oteryn-cache`, one
CP D1429 `slot-run.sh` slot per run, `apps/game-server/src/lib.rs` touched
before every cargo run, PG through the unchanged `in-pg-netns.sh` against
shared PostgreSQL 17.6, `PRE-ROUTING` skips 0):

| Check | Result |
| --- | --- |
| `cargo fmt --all --check` | pass |
| strict Clippy `-p oteryn-game-server --all-targets -D warnings` (tree `63714405`) | pass |
| lib (tree `63714405`) | 3413 passed, 0 failed (29 ignored, pre-existing) |
| `staff_audit_postgres` | 1167 passed, 0 failed (18 `staff_pg_` ok) |
| canonical `durability_postgres staff_pg_` | 18 passed, 0 failed |

Tree `63714405` differs from `0034b7de` only by the one-line fixture SQL above;
its first PG run failed exactly that owner fixture, which the new trigger
correctly refused. Runtime remains REPORT_ONLY.

## Previous checkpoint — D1412 verified receipt releases the fence, REPORT_ONLY (2026-10-17)

[D1412](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6238311581)
returns this lane to AUTHORING for
[P2 4303796136](https://github.com/Oteryn/Oteryn-Game/pull/2181#discussion_r4303796136)
on parent `cf0bf62ad333df8091b6c536f5829d96a124856d`. The finding is accepted:
the pending-receipt fence released on row existence alone, and migration 0133
checks only structure, so a runtime or control LOGIN could commit a
forged-signature receipt, its RECEIPT link and the head advance in one valid
transaction and reopen staff control.

### Owned paths for this delta

- `apps/game-server/src/durability/staff_action.rs`
- `apps/game-server/src/bin/oteryn-game-ops.rs`
- `apps/game-server/tests/support/staff_audit_postgres_cases.rs`
- `apps/game-server/src/node/{config,serve}.rs` (already owned as D1343d; the
  trust input strictly requires the restart-only history to reach the server)
- this card

No SQL, migration, registry, Cargo or contract file changes.

### Repair

`receipt_fence_in_transaction` runs in `begin_with_clock`, and in the
`staff_head` preparation path, after `lock_head` and in the same transaction.
The existence check stays. The receipt of the latest `requires_receipt`
mutation must also:

- match its RECEIPT link byte for byte (bytes, digest, principal, fingerprint,
  signature), at a sequence later than the mutation;
- pass the existing verifier, `History::receipt_at`, which picks the reviewed
  signed activation in effect at that receipt's own `control_seq`, never the
  current roster;
- bind the stored operation: environment, operation ID, request and outcome
  digests, signer and fingerprint, disposition, and the link predecessor.

Any miss refuses with the verifier error or `Receipt` and keeps the fence.
Earlier receipts were checked when their own fence was released, so a request
does not re-verify the whole history. `staff_receipt_head` (receipt
publication) keeps its existing path; the fence there would be circular.

Trust input: the reviewed activation history already used by
`staff audit-verify` (a JSON `ActivationConfig` list ending at the current pin,
checked by `History::load`). Server config becomes `[staff]
activation_history_file` plus `[staff.trust]` (the former `[staff]` keys). The
file is read once at boot under the same trusted-file rule as the roster.
`staff prepare` takes `--activation-history`. Nothing is read from the
database for trust. This config shape is an assumption reported to CP.

### RED/GREEN

New canonical case
`staff_pg_forged_receipt_commit_cannot_release_fence_for_member_logins`, run
for real member LOGIN roles of `oteryn_game_runtime` and `oteryn_game_control`.
Each LOGIN commits, through plain SQL, a RECEIPT link, the receipt row and the
head advance for a lawful pending mutation. The COMMIT must succeed. There are
two forgeries:

- a genuine rostered signature over other bytes attached to the bound bytes
  (refusal `Signature`);
- a validly signed receipt binding another outcome (refusal `Receipt`).

Each one then makes two attempts at preparation and at the next valid request.
The identical retry of the mutation must return its stored outcome, with
counts and head unchanged.

Control
`staff_pg_valid_receipt_across_rotation_releases_fence_under_its_own_activation`
covers a lawful roster-A receipt at 3 followed by rotation to an independent
roster B at 4:

- Preparation and the next request under B are admitted with history
  `{1:A, 4:B}`.
- Current-roster-only history is refused with `Signature`.
- History lacking the receipt's activation is refused with `MissingMaterial`.
- Identical retries return the stored outcomes unchanged.

RED is the GREEN source with only the body of `receipt_fence_in_transaction`
reverted to the existence check, with identical test bytes.

### Validation (source-bound, 2026-10-17)

Source: `cf0bf62a` plus the four code files above, patch
`5f567292b2648923122997acb31a702e985ca98150a48eefdc2d3c67410cefcf`, GREEN
`staff_action.rs` `fb7e3192548e27668a7d344de7fb3f49a71b1f416c2b74f40be1d1b0d70d691a`.

The run took the outer lock (D1415 `flock -w 600`, Ubuntu/mole
`heavy.lock`) once, and touched `apps/game-server/src/lib.rs` before every
cargo run (D1420). Earlier runs without the touch were discarded. PG used the
D1417 route: test binaries were built in Ubuntu/mole, copied out of the
shared target, and run under Ubuntu-24.04 root through its existing
`in-pg-netns.sh` and shared-pg.env (PostgreSQL 17.6). `PRE-ROUTING` skips: 0.

| Check | Result |
| --- | --- |
| `cargo fmt --all --check` | pass |
| strict Clippy `-p oteryn-game-server --all-targets -D warnings` | pass |
| lib | 3259 passed, 0 failed (29 ignored, all pre-existing) |
| lib `staff::` | 16 ok |
| lib `node::config::tests::staff` | 1 ok |
| lib `node::serve::tests::control_` | 2 ok |
| canonical `durability_postgres staff_pg_` | 17 passed, 0 failed |
| `staff_audit_postgres` staff cases | 17 ok (15 existing + 2 new) |
| card: `validate_governance.py`; `tools/agents/tests`; `git diff --check` | pass; 124 OK; clean |

The full `staff_audit_postgres` binary also runs 5 source-scanning lib tests
(`character_revision_sequencer`, `item_mint_audit` ×3,
`charm_desc_fence_gate_tests`). They failed only with `NotFound`, because the
binary ran in a distro without the source tree; the same tests passed in the
lib run.

RED (fence body reverted to the existence check, test bytes identical,
`staff_action.rs` `466e9397…`):

- The forged case fails with all 16 admissions: preparation and the next valid
  request, for the runtime and control LOGINs, for both forgeries, on both
  attempts. Every forged COMMIT succeeded.
- The rotation case fails with `current-roster history released the fence`.

GREEN source restored to `fb7e3192…` and passes both. Runtime remains
REPORT_ONLY.

## Previous checkpoint — D1393 distinct signed scope, REPORT_ONLY (2026-10-17)

[D1393](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6235846045)
returns this lane to AUTHORING for
[P2 4303224869](https://github.com/Oteryn/Oteryn-Game/pull/2181#discussion_r4303224869).
FAMILY-I is the sole writer for one complete two-file successor on
`1bea35ca1d4edf8fb50648378fa17fd6ec8d176b`: exactly
`apps/game-server/src/staff/signature.rs` and this GM card. CP explicitly
requires no main merge first. The parent is the published D1374 receipt
successor, sole parent `63ad6c3afb37aafd4cd6cde76dcaa4696622ef25`, tree
`a2a3755dad78efe7f5ecf319fa8c5e8eb6aea15d`. Root remains the sole publisher.

### Accepted identity invariant and minimal repair

The finding is accepted: the old validator allowed an invalid World/Channel
pair when both fields contained the same syntactically valid UUID. Both
envelope fields were present; the defect was acceptance of equal identities,
not loss of the channel field. The owning contract is supported by
`docs/contracts/game-events/v1/staff_action.proto:13`, the existing
`0008_native_entry_content_activation.sql:48` check `world_id <> channel_id`,
and the production scope-assignment inequality and independent equal-ID
negatives in `tests/native_scope_assignment.rs`.

The approved +13/−2 patch adds `scope.world_id == scope.channel_id` to the
existing `Some(scope)` guard. UUID decoding already requires canonical
lowercase text, so this comparison binds exact identities without a second
parser. The same validator covers canonical preparation and verification of
received, genuinely signed bytes. `audit_staff_read` verifies before acquiring
its database connection; historical chain verification uses the same checked
request constructor. The typed, independently pinned `RosterActivate` exception
with `None` remains valid. Existing scoped activations retain the checked
distinct pair. Runtime remains REPORT_ONLY.

### Regression operators and frozen source

The existing test
`staff::signature::tests::ordinary_requests_require_signed_world_and_channel_scope`
adds the operator `equal world/channel` for each of ten ordinary action
variants. It copies the valid world UUID into the channel field while retaining
all other valid facts. Each mutation checks both `canonical_bytes()` and
verification of adversarial JCS signed directly with the existing real OpenSSH
fixture. The original 30 negative variants, ten distinct-scope positive controls
and typed `None` activation remain unchanged.

The genuine RED executed one failing test, exit 101, with **20 acceptance
labels**: ten actions × `prepare` and `verify` for `equal world/channel`. The
raw log confirmed the exact ordered list of all twenty; the original negative
variants were not reported as accepted. GREEN exercises **40 negative variants**, the ten ordinary positive
controls and global activation inside the existing single test. These are
variants, not additional test functions: the staff family remains 16 and the
selected combined inventory remains **16 staff + 15 canonical PG + 4 node = 35**.

| Frozen item | SHA256 |
| --- | --- |
| Published-parent signature source | `661b3797dd1404a1d1fcbb7796aa32a1e08716abb4e450f21ac2672060d7e74e` |
| RED signature source, test change only | `13ba347902a30e44b394d0ea1305b5e2eaf613ea71c4fae73defb6ba036ad3fb` |
| GREEN signature source, approved guard and test | `0ba9b12deb3f9a0595dd29d7f0d0cefac46b1b963ffec3d229844d1d549ac424` |
| Independently reviewed proposal patch | `b68da6fa4a665c217a50acff0e112d519bdba20bc19cc7401914cfff5c1d4780` |
| Native source packet | `3ceed4da04244a891e41cdc2d63ab6634cdae24c90439f596f14bf9dbc1da0d9` |

The RED production prefix matches the parent byte-for-byte. The GREEN
production change is only the equality condition; the regression bytes are
identical between phases. Independent proposal review and exact-parent
apply/whitespace check passed. No D1393 native result is claimed by those checks.

### Actual D1393 execution record

These results come from the newly executed D1393 reports. Root downloaded
and byte-verified all **21 raw files**, including every command log, in the
GREEN checkpoint archive; seven complete source inventories remain retained
at their original native paths. All phases preserved **18,923 source paths**.

| Check | Expected execution | Observed result |
| --- | --- | --- |
| Genuine scope RED | 0 passed, 1 failed, exit 101; all 20 equal-ID labels | **CONFIRMED:** 0 passed, 1 failed, zero ignored; exit 101; exact 20 labels |
| Staff unit family GREEN | 16 passed, zero failed/ignored | **16 passed**, zero failed/ignored |
| Canonical staff PG GREEN | 15 passed, zero failed/ignored | **15 passed**, zero failed/ignored; shared PG helper, no skipped routing |
| Node controls GREEN | 4 passed, zero failed/ignored | **4 passed**, zero failed/ignored |
| Workspace formatting | Rust 1.94 workspace fmt check | **PASS**, exit 0 |
| Strict Clippy | Selected server/release-manifest packages, all targets, `-D warnings` | **PASS**, exit 0 |

- RED finished: `2026-10-17 15:55:48 UTC`; raw result
  `ccffecf5878431822b9237fe53fa008014205b75537eec38b617c8e3ca548034`; regression log `8a9e07d73600de24c19d1ffe7941a5cbb6d89dfe5004f902d719616375a38e2f`.
  Full source before/after: `538f892bf1fa0ea4cd205e60cbfe587b740e2a06a078b66dd134ba675e08f211`.
- GREEN finished: `2026-10-17 16:03:05 UTC`; raw result
  `fd445829e3107238426cbaeb6cf983937de5fc1330ad9ad242decdf921ea55f1`; full source before/after
  `80a261d307fba769cd57589c93a52c24acb56882cc760aab1e5102065d7eb5a0`.
- Downloaded, byte-verified raw native archive: `7a044aa4949b274c3d5c9164a8b57467e9bf842bf8ce667c6ba5bae24569db67`.

D1329/D1342 remain applicable: shared PostgreSQL 17.6 via the existing
`in-pg-netns.sh` after locked fetch, one allocated build slot and prescribed
environment, jobs=4, MemAvailable at least 12 GiB, and cleanup limited to owned
fixtures. Runtime authority, SQL, registry, Cargo, envelope encoding and the
accepted D1374 direct-database attribution residual remain outside this delta.

### Relationship to published D1374 and the next commit

D1374 is complete and remains a separate historical qualification. Its source
GREEN passed 35 tests, fmt and strict Clippy on inventory
`c262033964d4b3c762efc18b2a2018378387492c3c98298dfb9de7a4fcf25c60`.
The actual fetched `1bea35ca` commit then independently passed the same 35 tests
and fmt at **2026-10-17 15:22:01 UTC**, on unchanged 18,923-path inventory
`629d1b3865053040acea7c4faf37d95fa45b6d172823491170f68906a7c559dc`.
Published raw result:
`bd9f68091d4156c64de55881e96c5172a856fb56e60eb7fce17c24e3ed492400`;
final 33-file archive:
`ac7f8441653f483e8bfaa8cfb79aa9237de2c1543497b683e38eae606f6aac80`.
Clippy was not rerun in that published phase; its source-bound GREEN execution
used identical code. The receipt/link thread and four earlier findings are
resolved; [D1374 completion](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234761362)
records archive verification and cleanup. Those passing tests lacked the new
equal-ID operator and do not prove this D1393 repair.

The D1393 source run precedes the completed card and eventual API-created
commit. Its results stay bound to the exact source inventory recorded above.
Actual published-head verification, current CI and the one deduplicated review
will be reported separately in the PR description and CP receipt. D1393
requires one non-force publication, a reply and resolution of 4303224869, then
one review of the new head. Ready, queue and merge remain CP-owned.

## Previous checkpoint — D1374 receipt-to-link constraint, REPORT_ONLY (2026-10-17)

[D1374](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234779578)
allocates one complete three-file successor for
[review 4302869631](https://github.com/Oteryn/Oteryn-Game/pull/2181#discussion_r4302869631).
CP first merged main into the task branch: authoring predecessor
`63ad6c3afb37aafd4cd6cde76dcaa4696622ef25`, parents
`b120cfc5a0d056ec47487ab544be7a90414ba9e8` and
`ce7bccab973625d5d3d0d0c566426903229d2373`, tree
`5b21979b90dc5c1412dc94ff61bf5aa699c811da`. Root verified the fetched ancestry,
complete source inventory and unchanged preimages of both owned code leaves.
The package contains only migration `0133_staff_audit.sql`, the existing
`tests/support/staff_audit_postgres_cases.rs` and this task card. Existing
shared mounts, registries and node/control behavior are unchanged by this delta.

### Receipt integrity and remaining trust boundary

The old deferred SQL guard checked RECEIPT link to receipt row only. A direct
runtime/control INSERT could point a receipt row at an existing REQUEST link
and commit without a new receipt link, discharging the row-presence fence in
the subsequent request path. The added reverse constraint checks every inserted
receipt at transaction completion: same environment and control sequence,
RECEIPT kind, exact signed bytes, principal, key fingerprint and signature, and
the same transaction as the link. Missing binding raises SQLSTATE `23514` with
`staff receipt has no same-transaction link`. Existing link digest constraints,
operation foreign keys, membership guards and all prior DDL bytes are preserved.
The change is exactly eleven additive lines, including the deferred trigger
and explicit PUBLIC revoke. It does not compare receipt transaction identity
with the earlier operation transaction; normal later receipts and identical
retries remain valid.

This is a structural SQL invariant. Links have no operation_id column, so a
misattributed receipt/link written directly with runtime database credentials
in one transaction remains possible outside the verified writer. The signed
operation_id, request/outcome digests and signer are checked by the existing
Rust writer and by `audit verify`, including after outbox expiry. This is the
accepted later-audit residual in
`docs/architecture/reviews/OTERYN_GAME_ARCH_LIVE_READINESS_2026-10-06.md`,
section 1 ruling 2, Residual, explicitly accepted for this repair by D1374.
No SQL JSON parser or cryptographic verification claim is added. Runtime
continues as REPORT_ONLY, with the existing mutation and activation holds.

### Native regression and source-bound qualification

The additional canonical test
`staff_pg_member_logins_cannot_commit_receipts_without_matching_links` uses
two independent fixture databases and real LOGIN members of runtime and
control. Each first commits a lawful receipt in a transaction later than its
operation, verifies transaction provenance and retries identical bytes. Each
then performs the faulty direct INSERT and a real COMMIT. RED records both
accepted writes, proves the retained wrong-kind row and confirms a fresh begin
would bypass the missing-receipt fence. Both fixtures and generated roles are
cleaned before the final accumulated assertion. GREEN requires the exact
constraint error, unchanged head/counts and the preserved fence, followed by
a lawful receipt, retry and resumed request.

All 14 prior case bodies and helpers remain the exact 59,011-byte prefix
(SHA256 `91eb131c6403d0297fd689e4c653bdb24ff10862851494c4879d07b103359c86`).
Only the appended case was formatted; independent SQL and formatted-test
review PASS. The family now has 15 cases.

Genuine RED on the CP-merged base: one actually executed failing regression
(exit 101), both runtime/control direct-COMMIT acceptance markers and complete
fixture cleanup. Full source before/after:
`5c50401d93f6bb34b5bb2423bf29b177afb5d281c0b42d272ada0ed7a85a1d8d`.
Raw RED result: `6bdb1dc96f8cb07155e63d614a42fff0da80896cd8d7d49643e8b342aa828564`;
raw regression log: `977b110539413dc8dcea8b24ffc2ead012b1bee7c50b25e1e7253a0cb2db0840`.

GREEN completed 2026-10-17 15:04:56 UTC
on Molehill slot I: **16 staff + 15 canonical PG + 4 node = 35 PASS,
zero failed or ignored**, workspace format check PASS and strict all-target
Clippy PASS. The sole RED-to-GREEN source change is the eleven-line SQL repair.
Full 18,923-path GREEN source before/after:
`c262033964d4b3c762efc18b2a2018378387492c3c98298dfb9de7a4fcf25c60`.
Raw GREEN result: `4ff367e26fcd155c33fa5677d6c30f6f6e5cdaf81b6db2536f9efe9ab3bfba8b`.
PG family log: `d584451c76c60a9017c9a1ebcc187b1d4f58979a0b8fffd71d07458dda1db37e`.
Clippy log: `984a0050d7b0091cd2c5701c8ec6e6b74b8587417704637c594d8849234b24a1`.
The root downloaded and byte-verified all 21 raw logs/results in archive
`12bdac76ad2deb15b701f941aa0264ce112d46b455f8fd2ce31832b7ae9b8a20`; all source inventories remain
in their original native evidence directories.

Final migration SHA256:
`67f145e81069112f6db55ddb3339aea30ddb3cbdaecd76ff1c325d9fa13fadb6`.
Final formatted PG source SHA256:
`b8dec62cfaab73e928dd57227039d873dd2f99e205cad65d82dcd25903ee824a`.

Molehill execution follows D1329/D1342: locked fetch before PG, the existing
shared-PG namespace helper, one held build slot, pinned shared build environment,
jobs=4 and MemAvailable at least 12 GiB. No private PG, foreign database/server
or peer process/lock is used. Test executables remain only inside the held
slot's canonical target directory. The initial prepare stopped before Cargo/PG
because its non-login shell lacked the installed Cargo bin directory; its
failure is retained separately, not counted as RED. The corrected launcher
uses the verified directory and a new immutable evidence namespace.

The candidate's actual published-head verification and live CI/review results
are reported in the PR description and
[#2131](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234761362)
after publication. Source qualification recorded here is not relabelled as
already executed on the future API-created commit. Ready, queue and merge
remain CP-owned. Native checkouts are removed after source/evidence recovery
and published-head verification, while reports and shared targets are retained.


## Previous checkpoint — D1343 installed audit contract and review repairs, REPORT_ONLY (2026-10-17)

Authoring predecessor is published draft #2181
`df1d39389eabe030ac7dc2a37b92d202aaedae62`. The root lead is the sole branch
publisher. [D1343](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233583959)
and [D1343d/e](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233694200)
allocate the following complete package. This checkpoint supersedes the migration,
event, canonical PG and node/control holds recorded in older checkpoints below;
it does not turn their historical tests into evidence for this package.

### Installed contract and composition

- Migration `0133_staff_audit.sql` is the single DDL source, embedded by the real
  `sqlx::migrate!("./migrations")` ledger. Historical 0092 is not installed or
  reused, and the old public proposal string is removed. The inherited LOGIN
  membership guard from P1 4302394369 remains intact.
- `STAFF_ACTION` has the current D1343 lease for event ID 5, schema revision 1
  and a 365-day ordinary audit expiry. The v1 proto and the production registry
  binding strings agree. Existing event/resource rows remain unchanged; the
  staff rows are additive. Payload/envelope wire limits remain CANDIDATE until
  native max/overflow tests execute on the published successor head.
- All staff PG cases use the public production staff types, the installed
  migration and `Contract::registered()`. The support fixture owns its test
  OpenSSH keys and loads trust through the public checked-file API. It does not
  expose production signing or rely on a private fixture-only contract.
- `durability_postgres.rs` gains only the two-line staff case mount; all its
  previous bytes are preserved. Ten of the twelve existing GM case blocks
  remain byte-identical. Two replace fixture event IDs with the registered ID
  and a distinct wrong-route value, preserving each assertion and negative
  operator. The new activation regressions bring the staff family to 14 cases.
  The standalone wrapper remains explicitly callable; its `test = false` entry
  routes default execution through the canonical aggregate without duplicating
  cases.
- Node config accepts an optional restart-only staff trust section and still
  parses existing configs without it. Boot loads pinned trust before authority
  registration. The UID-0 control socket preserves its 30-second deadline and
  64-byte bootstrap route; staff framing separately bounds prefix and payload.
  It passes opaque signed bytes to the existing verified durability owners.
  Missing trust refuses the staff route, and every non-read request remains
  refused by `audit_staff_read`. Runtime stays REPORT_ONLY under D1343d; no
  activation endpoint, live roster change, signer distribution or new gameplay
  mutation capability is introduced.

### Review findings in this package

The prior df1 review completed with two P2 findings, both accepted for repair in
this single successor:

- [4302571603](https://github.com/Oteryn/Oteryn-Game/pull/2181#discussion_r4302571603):
  ordinary signed requests require both WorldId and ChannelId. The existing
  typed `RosterActivate` environment-link exception is preserved. Tests exercise
  ten ordinary variants through preparation and real signature verification,
  with missing scope, invalid WorldId and invalid ChannelId as separate cases.
- [4302571606](https://github.com/Oteryn/Oteryn-Game/pull/2181#discussion_r4302571606):
  later activations must lock and validate the independently current head.
  Only a no-head activation at sequence 1 with the zero predecessor may
  bootstrap. PostgreSQL regressions use real signatures, isolated predecessor
  mutations, valid bootstrap/rotations and an actual concurrent INSERT blocked
  on the head before a competing valid rotation commits.

The earlier P1 member-role guard and P2 exact retained-envelope validation remain
in the package. No case, signature check or authorization fence is disabled.

### Qualification and resource rules

The native source assembly is the named published predecessor plus this complete
14-file package. Its results remain distinct from qualification of the eventual
API-created successor. All three collaborators reviewed separate source portions;
root reviewed the complete package and remains the sole native executor and
publisher.

The original node/control proposal is patch SHA256
`8f55229e824663df1a8f94e0b49db07b88b5f1c29ae24800a78897aa2051b9e1`.
Attempt 1 stopped before tests with E0433 because the new socket test used
`tokio::join!` without the macros feature. [D1352](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234008098)
accepts the exact test-only `tokio::spawn` writer plus awaited completion
correction, patch `2dff1cb5d047262966d057a2f4dbcc0bb0fe1e682e996902c6b4e2831d60b32f`.
It preserves the concurrent socket test, assertions and production bytes without
changing Cargo features. The compile-only failure is retained, never counted as
defect RED.

Attempt 2 executed genuine RED against the preserved scope and activation SQL:
all three selected regressions exited 101 with exactly one failure each. Scope
reported all 20 missing-scope prepare/verify markers; binding reported six
independent predecessor failures; the actual concurrent INSERT showed both the
missing lock and stale-predecessor acceptance. The two PG cases cleaned their
owned fixtures before their final assertions. All 18,917 source paths were
unchanged, snapshot
`b787968a4025d056f0618b5b0adfe6910ad2978bbfe7f5348de5683ca9b40b97`;
the raw result SHA256 is
`e5e67dcfaecd2f9cc117659a7dc283df8395b33f50810731229ffb010da92fc8`.

Attempt 2 GREEN then compiled both real targets and produced 15 passing staff
tests plus one genuine payload-boundary failure: the new borrowed-length visitor
used envelope field 27 (digest), while payload is field 26. Producer bounds,
the outer envelope limit and whole regenerated-event comparison remained intact.
[The retained failure and correction](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234225406)
change only that one visitor byte; tests and limits are unchanged. Its before/after
source was
`ece0ad58c19f5c9ebcb0f929378428e4dc55c163521aaf5a68e03496556f310b`;
raw result SHA256
`2cd3e160676e45584c0ccdb838286a6fffa222c9a73a7a74af6299fe493e5fd8`.
All 22 completed reports and full raw logs from the failed stages are preserved
in archive SHA256
`89db5a0b5e7025b8e85587bd75527003ad19a2e2d8672e8eaa96d4f97ebb2e91`.

Attempt 3 executed all 16 staff unit tests, all 14 canonical staff PG cases and
four selected node tests: 34 passed, zero failed or ignored. Workspace fmt passed.
Strict all-target Clippy then failed on the direct `panic!` in the newly added
parser test. [The test-only assertion correction](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234386607)
uses `Option<Vec<u8>>` plus `assert_eq!` to preserve the required Staff variant
and byte-for-byte Request/Receipt round trip. No production code, test selection,
limit or lint allowance changes. The retained attempt 3 source snapshot is
`52282e7bd3a4f736c9b7f441b35f6190bb81039dd33dc0fcce9d4a09f236dd4d`,
packet SHA256
`1869935ad0efcdad502d724e7e40797c6b7b55e21f5ebee97a17cbd7f19d51b5`.
Preparation verified exactly one changed audit byte and zero rustfmt changes.
The raw Clippy-failed result is
`e7d65093696608761d68166bf161ffc34bb8b625467d83fc0bab4910acde28ae`;
all 37 completed reports and full raw logs through this stage are preserved in
archive `35ec6e73c7730b270e2dc35a67ca366955e43aadda594f09166f5dfa109fbab6`.

Corrected attempt 4 GREEN completed on Molehill-PC at 13:34:54 UTC: all 16 staff
unit tests, all 14 canonical staff PostgreSQL cases and four node tests passed,
with zero failures or ignored cases. Workspace fmt and strict all-target Clippy
for game-server/release-manifest also passed. Its sole source correction
relative to attempt 3 is the parser test assertion, patch
`33c89ba15d7a3d329d628ece99d9fd849f4a923dd8c47ccbfacf0ac1484d48a0`;
source packet SHA256
`2ff8fe149a126d09f13704f02a7cb00b3588839226dc31cc675debc5283c3fec`.
The complete 18,917-path source snapshot remained unchanged before and after:
`1c8d874c818f211533456753336eba99a70bc8b2018296ce3aeef2d996c44079`.
The raw passing result SHA256 is
`9f3e30b0f5aac607b84a3bf0785ff6adee35b55a98df60540a698ef23f0c9f46`.
All 52 completed reports and full raw logs through this passing stage are
preserved and verified in archive
`b6282fb5da1a852ff7461a3d9e4730970a21343906d42bfe63c75cec02fd9102`.
The original three RED executions and the payload-boundary RED retain their
separate source identities and are not relabelled or repeated.
At preparation of this publication, native qualification of the API-created
successor and the original PG target-aggregation guard remain PENDING until
that successor exists and is checked out cleanly. Their later receipts belong
in the PR and control-plane discussion; this source-assembly evidence is not
relabelled as execution of the future commit.

[D1342](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233555077)
requires the prescribed sourced sccache environment, an exclusively held free
build-slot lock, its own `/root/oteryn-target-<slot>`, jobs=4, incremental=0 and
MemAvailable >= 12 GiB before Cargo. D1329 requires `cargo fetch --locked` before
tests through the shared PG 17.6 namespace helper. No private PG server or
container is started. Each fixture cleans only its own database/roles.
[The retention clarification](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233732824)
requires a separate lock only for keeping/deleting ELF files outside the slot
target; this run keeps none there.

[The Cargo correction](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233644124)
confirms that D1328 remains binding and #1942 is not a gate. Only the exact
D1343e target hunk changes Cargo in this package. #2197 precedes #2181.
D1353 orders the canonical PG mount #2181 → #2166 → #2187; D1354 orders the
shared transport mount #2197 → #2145 house line → QUEST (#2198) → A2 (#2187).
Ready/queue actions stay with CP. D1343d/e
requests responses and resolution for both new findings with the fixing commit,
then one new-head review request after live de-duplication.

## Published predecessor checkpoint — D1328/D1337 review repairs and World pin (2026-10-17)

Current authoring base is published draft #2181 head
`647053e929694a9dceb639bd42e4cdceb3ded479`, sole parent
`3342d23605d817ca08506e29bdc33303a5d41afa`. [CP's D1328 integration](https://github.com/Oteryn/Oteryn-Game/pull/2181#issuecomment-6232918885)
installed the root Cargo member, game-server dependency and lock entries for
`crates/release-manifest`. The earlier #1942 Cargo hold is superseded. The old
public 3342d236 member-count and five unresolved-import CI errors belong to
that predecessor; CP reports locked metadata and architecture inventory PASS
at 647053e9, with no build, Clippy or tests in its integration run.

[The general D1328 decision](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6232899745)
removes the #1942 gate and exclusive slots for generated Cargo.lock, pins and
registries. The second lander merges main and regenerates these outputs;
handwritten ownership and ordering remain in force.

[CP's follow-up assignment](https://github.com/Oteryn/Oteryn-Game/pull/2181#issuecomment-6233003106)
requires P1 4302394369, P2 4302394370 and the compiler-derived World pin correction
in **one non-force successor of 647053e9**. The root lead is the sole publisher.
The earlier activation-anchor repair is already published in 3342d236 and
retained unchanged here; its source SHA-256 is
`833065648b8b37de987277c4177307f394630e370a352674669c329dbeedd6fe`.
This checkpoint governs current evidence; the private native history below is
not reassigned to the new candidate.

### Review repairs — independent source review PASS

- [P1 4302394369](https://github.com/Oteryn/Oteryn-Game/pull/2181#discussion_r4302394369), accepted:
  the proposed activation trigger now checks both session and effective
  non-superuser membership in runtime/control, including indirect membership.
  Explicit `SECURITY INVOKER` preserves the context; the superuser distinction
  retains owner/bootstrap provisioning. A new regression uses four actual
  LOGIN connections (runtime/control, direct/indirect), checks inherited
  INSERT/head UPDATE, and compares the same signed activation against the
  owner control. It rolls back attempts and removes only its database and six
  random roles before asserting the result.
- [P2 4302394370](https://github.com/Oteryn/Oteryn-Game/pull/2181#discussion_r4302394370), fix prepared:
  the audit verifier rejects unknown and duplicate envelope/transaction fields
  before prost decoding while retaining ANL-01 field-order tolerance. The
  regression has two reordered positive controls and eight wire mutations with
  recomputed SQL hashes.

Separate agents independently reviewed both source deltas and regression
baselines. This is static review evidence, not a native test result. Current
authoring bytes after the lead's final P2 assertion formatting:

| File under `apps/game-server/` | SHA-256 |
|---|---|
| `src/durability/staff_action.rs` | `19c8e52352d6a6b127861b1a4f62aa1952fe7a7f40a7023934e6959c3c13da7a` |
| `tests/staff_audit_postgres.rs` | `2a679d3d957ca601c170ccfab8865366da8ffdbd77fdfc17bcbe999f507c043b` |
| `src/staff/audit.rs` | `2bfbc62f28b3f67f0360810b4e80dae40f8f398b67af7f39af82b2f858c7814d` |

The original P2 review used pre-format SHA-256
`298357ed9f9225847f6cfbb4dd511d2ef6ba0aa65dc0b3a74111808de3b48e1c`.
An independent follow-up review also accepts final formatted `2bfbc62f…`;
only the final assertion layout changed, without a logic or vector change.

### World pin — PROVEN native execution

Lead evidence `gm-d1328-pin-result.json`, also summarized in the
[native slot request](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233237407),
binds execution to 647053e9 on Molehill-PC with Rust 1.95.0. After
`cargo +1.95.0 fetch --locked` (exit 0), repository
`cargo +1.95.0 run --locked --release --quiet -p oteryn-world-bundle-compiler -- pin-check <repo> <output>`
first reported the stale input digest (exit 1). The compiler-derived update
changes only `content/world/pins/oteryn.json`'s `inputs_digest`, from
`0264c82e66279f2473a353a2bf8a2da22178586557d5f7304608ca112f139ce8`
to `c1b0bbc004182f5c13bb4544b817b04451b8fbe7a21ebb94ba98d88837e0d2cd`.
The final pin-check returned exit 0 and its `build_twice` check produced two
identical **25,089,899-byte** World bundles with digest
`03f64820d8584b9de2b388ac7db6b7e19af16d5da1e57e44b727a262390072f6`.
Generated pin SHA-256:
`e554e36a567cdf519833859d61b9e1ab63068e9d9365238c83c3db2bb73054b0`.
The compiler input index was unchanged before/after, SHA-256
`ad667faa0aa9810accd022023074e92146d17a6e41f4cb4b3be5587e8103c90d`;
the assigned staff Rust repairs are outside those inputs.

### Current native validation status

As recorded in the slot request at `2026-10-17T11:14:58Z`, the regression runner
stopped before source mutation, Cargo or PG: PID 102226 held `build-slot-A.lock`
and `native-pg-runtime.lock`. That slot-A blockage is historical: [CP D1337](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233242761),
at `2026-10-17T11:15:37Z`, grants a separate nonblocking `build-slot-I.lock`,
`jobs = 2`, and a task-owned Cargo home and target. At least 8 GB of free RAM
must be checked before each stage; lock or resource refusal stops execution
without retry. D1337 does not require `native-pg-runtime.lock`. The lead is
responsible for execution under this grant.

The subsequent native attempt acquired slot I but the lead's stricter
`min(MemFree, MemAvailable) >= 8 GiB` guard refused the start:
`MemFree = 7384854528` bytes (about 6.88 GiB), while
`MemAvailable = 42363576320` bytes (about 39.45 GiB). It stopped before Cargo,
tests, source/cache or PG mutation and did not retry. [The clarification request](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233298494)
asks CP to confirm the MemAvailable interpretation and authorize a new start.
[CP clarified the guard](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233326230)
at 11:25:28Z: only `MemAvailable >= 8 GiB` counts, and one new guarded
RED→GREEN attempt is authorized. The new run uses an independent evidence
directory and the same slot I, with final P2 `2bfbc62f…` bound in its GREEN
packet. Original production plus test-only bytes remains separately preserved
for RED; a prepared expectation is never recorded as an executed result.

| Current phase | Evidence at this checkpoint |
|---|---|
| P2 exact lib RED; P1 exact LOGIN/PG RED | **CONFIRMED**: each executed test returned 0 passed / 1 failed, Cargo exit 101, with all eight P2 and all four P1 negative vectors named in the expected final assertion. |
| P2 exact lib GREEN | **PASS**: 1 passed / 0 failed, 0.02 s. |
| P1 exact LOGIN/PG GREEN | **PASS**: 1 passed / 0 failed, 1.30 s; completed at 11:37:45Z. |
| Repaired staff unit family | **PASS**: 12 passed / 0 failed / 0 ignored, 0.77 s. |
| All 12 staff PG cases | **PASS**: 12 passed / 0 failed / 0 ignored, 14.43 s; completed at 11:38:01Z. |
| Owned rustfmt check; whole-workspace `cargo +1.94.0 fmt --all -- --check` | **PASS** for both executed checks. |
| Strict all-target Clippy | **PASS**, exit 0: game-server and release-manifest, `--all-targets -- -D warnings`; completed at 11:41:10Z. |
| Authoring governance, diff check and agent suite | **PASS**: 22 required policy documents / 9 lanes, `git diff --check`, and 124 governance tests. The finalized five-file tree is checked again before publication and on the fetched API candidate. |

The completed RED run on Molehill-PC used Rust 1.94.0 at 11:27:56–11:33:08Z.
P2 accepted all eight altered envelopes under the original verifier; P1
accepted all four real LOGIN member attempts under the original trigger.
The expected P1 assertion occurred after cleanup of its own database and six
roles. All **18,914** tracked source files were unchanged during execution;
the before/after snapshot SHA-256 is
`7dee471a97ffd0f38cad84f59e59bb51d2c20dc1faa51a9a3c28d5c145da524f`.
GREEN began at 11:33:47Z as a separate run, requiring this successful RED
evidence and matching source snapshot before installing the reviewed final
bytes. The reported GREEN results above bind to base 647053e9 plus the three
exact source hashes and generated pin recorded in this checkpoint. The native
checkout contained the baseline task card during these runs. No API candidate
existed at execution time; candidate-specific CI and publication verification
are tracked separately on #2181. The complete GREEN runner returned exit 0.
All **18,914** native tracked inputs were unchanged from before to after
GREEN, snapshot SHA-256
`f99f4f93bb532ef5d384ca85d0f3b5f943cc3ac7539206fd0f20c537b8b22fa4`.
Strict Clippy completed at 11:41:10Z with exit 0; the unrelated existing
vendored Tokio missing-documentation warning was visible in dependency output
and did not fail the selected-package strict gate.

D1329 permits only shared PostgreSQL 17.6 via the supplied
`~/.oteryn/in-pg-netns.sh`, following `cargo +1.94.0 fetch --locked` for the
repository-pinned native regression toolchain. No private server/container or
host-5432 change is part of this route. Cleanup is restricted to the fixture's
own database and randomized roles; skipped/unconfigured PG is not PASS.

### Remaining shared contracts and canonical PG plan

Production node config/control, migration, `staff_action.proto`, STAFF_ACTION
event/retention registration and canonical PG aggregation remain held. D1328's
source repairs and Cargo integration do not allocate these contracts. Historical
0092 and D1188/event ID 5 references below remain recovery evidence, not current
leases. The shared order remains `tests/wp5_s3b_composition.rs`: #2181 then A2,
and `tests/durability_postgres.rs`: #2181 then #2187; CP writes Cargo/lock.

Read-only analysis supports keeping canonical `production_server::staff`,
which already supplies the staff types consumed by source-included durability.
The proposed route moves the PG cases into one `support/*_cases.rs` file, with
a canonical mount and a thin focused wrapper plus CP-written `test = false`.
Independent OpenSSH fixtures use public trust-file loaders, without copying or
exposing `Signer`. Public `Contract::registered()` requires the actual CP event
allocation; it currently fails with `MissingMaterial`. The future cases must
derive IDs from that contract instead of fixture ID 4 or hardcoded 4/5 mirrors.
The existing checker exemption for canonical source modules means a staff
source mount is not by itself a proven duplicate-check failure, but it would
repeat the staff unit tests; the public-staff route avoids that duplication
without a new allowlist. This plan is static analysis, not implemented or run.

The earlier **11/11 PG PASS** below used private assembly
`715041517515314e4e10c63a405281cd37e72d14`, explicit standalone execution and
`Contract::fixture()` event ID **4**. It qualifies that participant fixture,
not the registered contract, held node route or canonical aggregate. The
private proposal's ID 5 does not change what those tests executed.

## Historical execution checkpoint — activation-anchor repair, NOT_FULLY_QUALIFIED (2026-10-17)

This checkpoint supersedes the validation statements below for the repaired
signature verifier. Existing draft #2181 remains draft. The source repair is
based on public head `921d7d1c51230595e0a3aa664fea97bee97a464d`; native execution
uses recovered private assembly `715041517515314e4e10c63a405281cd37e72d14`
with the same signature source. Held Cargo, node, migration and event-contract
inputs in that private assembly remain separate from the public source tree.

The final remote read found an independently published fast-forward successor,
`6b9941a1608a2b8b87e38d95927c53d445bec761`, which merged main into this task
branch. The local candidate was synchronized to that existing branch head before
freeze; this continuation created no merge commit. Its `signature.rs`, this
record's predecessor, all staff modules, the staff durability/PG sources and
release-manifest crate are byte-identical to the reviewed predecessor. Both
real cursor consumers are unchanged, and the complete two-file repair passes
an isolated-index apply check at the new head. This is source compatibility
evidence only: the native runs below remain bound to private `71504151`, and
do not qualify the independently updated branch or current main.

- **P1 accepted and repaired:** `HistoryCursor::push` previously allowed a
  correctly signed receipt to replace an independently recorded activation at
  either the initial anchor or roster rotation. The common guard now requires
  activation sequence, link kind and the independently pinned activation digest
  to agree before dispatching to receipt or request verification. The remaining
  request guard checks the signed action against the link kind.
- **RED proven:** the new
  `receipts_cannot_replace_recorded_activation_anchors` regression, using real
  OpenSSH signatures, failed on the original production implementation with
  `receipts replaced independently recorded activations at sequences [1, 2]`.
  Its genuine activation controls and ordinary receipts after each activation
  passed. Cargo returned 101: 1 failed, 0 passed, 3265 filtered out. Test-only
  signature source SHA-256:
  `abc140c7ff28e53753c576f9d643daecf99053101751b5fd2bdc388f84e0c982`.
- **GREEN proven:** native Rust 1.94.0
  `cargo test --offline --locked -p oteryn-game-server --lib staff:: --quiet`
  passed all 12 staff tests, with 0 failures, 0 ignored and 3254 filtered out.
  The formatted repaired source had the same SHA-256 before and after execution:
  `833065648b8b37de987277c4177307f394630e370a352674669c329dbeedd6fe`.
- **Independent review PASS:** the separate SEC agent reviewed the production
  guard and regression against the public head, then verified the final
  formatted source SHA-256 above. The `audit-verify` and
  `historical-reconcile` consumers propagate cursor failures; both consume the
  repaired common guard. No additional production bypass was confirmed in the
  sibling sweep.
- **PostgreSQL PASS:** native
  `cargo test --offline --locked -p oteryn-game-server --test staff_audit_postgres
  staff_pg_ --quiet -- --test-threads=1` executed all 11 staff families:
  11 passed, 0 failed, 0 ignored, 1135 filtered out (9.07 s). Execution held the
  PostgreSQL qualification lock and used the dedicated container's network
  namespace with the unchanged loopback-5432 guard and PostgreSQL 17.6
  (`server_version_num = 170006`). The repaired source hash remained unchanged.
- **Strict Clippy PASS:** native
  `cargo clippy --offline --locked -p oteryn-game-server -p oteryn-release-manifest
  --all-targets --quiet -- -D warnings` passed on the same private assembly;
  the repaired source hash remained unchanged.
- Whole-workspace `cargo +1.94.0 fmt --all --check`, `git diff --check`, governance
  validation (22 policy documents, 9 project lanes), and the agent unit suite
  (124 tests, 0 failures) PASS. The lead owns the final candidate freeze and
  existing-draft publication.

**Cross-domain finding, deferred to the shared-slot owner:** the private Cargo
proposal declares the standalone `staff_audit_postgres` target with
`test = false`; the authored staff PostgreSQL cases have no aggregate
registration in that proposal. This integration finding is reported to the
lead. The explicit target run above qualifies those cases; default test
discovery requires its own shared-slot resolution before CI coverage can be
claimed.

## Historical checkpoint — NOT_FULLY_QUALIFIED (2026-10-16)

This checkpoint supersedes earlier validation and shared-registration statements
below. Existing draft #2181 remains draft. [D1122](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6224930201)
permits one coherent existing-draft checkpoint with explicit RED/NOT_RUN evidence
before full checks; it does not authorize ready, review triggers or CI restarts.
[D1160](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6224912485)
authorizes exactly `pub mod staff;` in `src/lib.rs` and
`pub mod staff_action;` in `src/durability/mod.rs` now.
[D1161](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6224978258)
authorizes exactly the production `node`/`staff` forwarding insertion in 26
source-included durability fixture roots; the dedicated staff fixture already
has its own staff module and production node and remains unchanged.

- **PROVEN, compile-only:** cloud `cargo check --locked -p oteryn-game-server
  --all-targets --quiet` PASS on the private assembly bound to `d11297a7`.
  This assembly includes held shared-path integration inputs; this is neither
  standalone Git-tree qualification nor a runtime/test result.
- **RED, repaired but not yet requalified:** strict Game Server Clippy at
  `d11297a7` failed `large_enum_variant` for `Begin::Fresh(Pending)` in lib and
  libtest. `e34d4f44` boxes the pending state without changing transaction,
  clock or request checks; no replacement Clippy PASS is claimed.
- **PROVEN, compile-only:** release-manifest strict Clippy PASS in that cloud
  assembly. Three previously passed JCS tests are reused historical evidence.
- **NOT_RUN:** all 11 authored staff pure tests and all 11 PostgreSQL test
  families. Compilation does not execute either group; PostgreSQL qualification
  is deferred to the CP-directed frozen ready-PR CI path under D1161.
- Exact changed-file formatting and `git diff --check` PASS for this repair.
  Lead owns independent review, remaining qualification and draft publication.

Root Cargo membership/dependency/lock, node configuration/control, proto,
migration and STAFF_ACTION registration slots remain unchanged under
D1086/D1091. No root manifest, lock, node, proto or migration slot is installed
by this checkpoint. No new runtime authority, full qualification, ready or merge
claim follows.

## Goal

Game side of OPS-GM-01: OPS-GM-AUDIT-1, SANCTION-2, REPORT-3, ROSTER-4, STAFF-5, PREPORT-6. No value-creating staff action.

## Dependencies (CP D1044, 2026-10-16)

| Dependency | State | Gate |
|---|---|---|
| LOGOUT-WIRE-1 | MERGED (#2127); former SANCTION-2 gate removed | cleared |
| WRITE-MOD-1 (REPORT-3 commands only) | merge required for REPORT-3 | **BLOCKING REPORT-3 ONLY** |
| OPS-GM-AUDIT-1 | D1085 allocated on existing #2181, task OTV2-20261016-gm-audit-1 | source authoring |
| OPS-GM-P1 (Platform), ARCH-CONTROL-ANCHOR-0 | external gate for EXT phase only | ok |

WRITE-MOD-1 gates only REPORT-3. SANCTION-2 owns its session-generation
fences. Child implementation requires its own CP allocation; the corrected
family path inventory below does not allocate a child or authorize slot work.

## Owned paths (family inventory; child allocation required)

- Current AUDIT-1 allocation: apps/game-server/src/staff/{mod,audit,signature}.rs
- apps/game-server/src/durability/staff_action.rs
- apps/game-server/tests/staff_audit_postgres.rs
- crates/release-manifest/ (JCS module only; SEC-REL-1 later owns release schemas)
- workspace-boundaries.toml (D1091's exact inventory/edge additions only)
- apps/game-server/src/bin/oteryn-game-ops.rs (existing executable, signed requests)
- docs/agents/tasks/GM-1.md
- D1437 additions: apps/game-server/migrations/0134_staff_moderation.sql,
  apps/game-server/src/durability/staff_moderation.rs,
  apps/game-server/src/staff/live.rs, the GM-1 hunks of
  apps/game-server/src/gameplay_transport/{chat_intent,connection,mod,qualification,capabilities_tests}.rs
  and apps/game-server/src/node/serve.rs

## Slot-only paths (ONLY in the final commit at the merge-queue turn, after merging main)

- apps/game-server/src/node/serve.rs and apps/game-server/src/node/config.rs
- migrations: game_character_sanctions, game_staff_roles, game_player_reports
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json (staff panel)
- docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json (`staff_action`)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs, crates/protocol-oteryn/src/lib.rs
- tools/content-migration/regenerate_content.py outputs; content/project.json; content/manifest.json, content/world/manifest.json; content/content.lock.json, content/world/content.lock.json; content/interactions/index.json; content/world/pins/*; Rust pins
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring; crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants; module registration in lib.rs / durability/mod.rs / domain/mod.rs

`docs/contracts/game-events/v2/staff_action.proto` and the `STAFF_ACTION`
registration require a CP contract allocation. D1160/D1161 now permit only
the exact own registration/forwarding lines described in the current checkpoint;
all other shared wiring remains slot-held.

## Wiring points (verified on origin/main 5742f30f3)

- apps/game-server/src/chat/ (mute)
- logout path (LOGOUT-WIRE-1)
- connection.rs command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (USE_INTENT l.2481, CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` table l.165, offered sets l.76/104/132 (new capability not offered until CP decides)

## Inseparable bundles (never split across PRs)

- migration + writer + durability per child
- wire + registry + capability + handler + tests (STAFF-5)

## Acceptance criteria

- [ ] every staff action emits an audited signed `staff_action` event
- [ ] sanctions are staff records, not Character aggregate writes
- [ ] no value-creating staff action
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.

## Validation commands

```sh
cargo test -p oteryn-game-server staff
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_ARCH_LIVE_READINESS_2026-10-06.md §1
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 5), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- After CP allocates a child, fill only its owned_paths until CP gives the queue turn; then merge main and add allocated slot-only paths in ONE final commit.
- Push rarely and batched: one push per round of fixes (heavy builds per push); no status-only commits.
- Never mark the PR ready, never enable auto-merge, never post `@codex review`; CP does that.
- Max 2 fix rounds; after that CP closes and splits the task.
- Confirm every BLOCKING dependency on main before starting code that needs it.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Local preparation — OPS-GM-AUDIT-1 (2026-10-15)

**Historical packet:** retained as evidence of the preparation at its stated
source SHA. The 2026-10-16 checkpoint below supersedes its UNKNOWN codec,
roster/head and clock bindings, guessed executable ownership, unresolved
OWNER-E, and LOGOUT/SANCTION gate. Its validation is historical, not validation
of this checkpoint; no child implementation was delivered by that packet.

This is a prerequisite and qualification packet, not delivery, acceptance or runtime
authority. Existing draft #2181 remains a draft. Work is local only on
`skeleton/gm-1` at `81e5a0f9263ff5cfac4628afa8e4fc54365164f0`; no commit,
publication, registration, schema, key generation or shared wiring is authorized.
The lead supplied META 3.1.0 binding
`1bfb5ff98c8aa156e73669a14e083a1d464c29fb`: one authoring writer, independently
current authority at each consuming boundary, no new production grants or trust.

### Accepted scope and source evidence

- **PROVEN:** ARCH-LIVE-READINESS-0 §1 rulings 2 and 11 and its packet table
  define OPS-GM-AUDIT-1, including the request-signature module, as **ALPHA,
  first**. The lead independently verified the owning record on authoritative
  main. LOGOUT-WIRE-1 gates SANCTION-2 and WRITE-MOD-1 gates REPORT-3 commands;
  neither blocks this first child's preparation. No pending acceptance batch
  is asserted for ARCH-LIVE-READINESS-0.
- **PROVEN:** the accepted verifier is upstream `ssh-keygen -Y verify`, with
  personal off-host OpenSSH signing, the reviewed namespace-limited
  allowed-signers roster, restart-only environment/roster configuration, and
  independently recorded roster activations. A local key list, self-declared
  request environment or stored request alone cannot substitute for them.
- **PROVEN at the source SHA above:** `apps/game-server/src/staff/` and
  `docs/contracts/game-events/v2/staff_action.proto` are absent; the event
  registry has types 1–3 and no `STAFF_ACTION` or its retention profile.
  Bounded searches of `apps/game-server/src`, `crates`, `docs/contracts` and
  Cargo manifests found no reusable staff signing implementation or contract.
- **PROVEN:** `node/operator_files.rs` uses pretty JSON plus newline and
  ordinary JSON decoding for existing unsigned operations. It defines no staff
  signed-byte format or canonical-byte equality check. `NodeConfig` and
  `OpsConfig` in `node/config.rs` expose no staff environment, signer-roster or
  activation-history binding. `node/serve.rs::handle_control` accepts a bounded
  bootstrap operation UUID, not a signed staff request.
- **PROVEN:** ANL-01 §§2, 9 explicitly reject protobuf serialization as semantic
  canonicalization. Its exact event-byte retention rule is not a canonical
  operator-request signing format.

### Prerequisites before signature or audit implementation

1. **Canonical bytes (UNKNOWN binding):** §1 specifies the signed semantic
   fields, but does not bind a concrete encoding, canonical rejection rules,
   field representations or shared test vectors. Resolve the accepted codec
   and version for complete typed requests, per-signer tuples, chain links and
   submission receipts; bind timestamp units, digest encodings and digest
   inputs. Requests must include every parameter, environment, roster digest,
   `issued_at`, `control_seq` and previous-link digest. The receipt domain is
   `oteryn-ops-submission-receipt-v1`. Bind account-ref component encoding for
   the accepted SHA-256 domain formula without retaining AccountId. Do not
   choose an independent JSON/protobuf envelope or handwritten cryptography.
2. **Independent authority source (UNKNOWN interface):** bind the restart-only
   verifier environment and reviewed roster bytes/digest, plus an off-host
   private-deployment checkout's recorded activation sequence/digests. Define
   how the verifier obtains the current chain head independently from the
   submitted link. New submissions must name the newest activation; historical
   verification needs retained roster revisions and exact recorded links.
   No live keys or private deployment access is requested by this packet.
3. **Clock and operation outcome binding (UNKNOWN interface):** the accepted
   10-minute first-execution window is marked CANDIDATE; bind its limit and
   future-time treatment. Resolve the durable atomic operation-id/request-
   digest/outcome lookup and chain-append serialization. Identical bytes return
   the stored outcome at any age; different bytes under that operation id
   refuse. Stored signed evidence never supplies independently current facts.
4. **Audit transaction contract (slot work):** lease event/schema and ops-error
   identifiers through CP, register `STAFF_ACTION` and
   `STAFF_ACTION_AUDIT_RETENTION_V1`, then bind the existing durable event writer
   and authoritative mutation transaction. An audit failure must roll back its
   mutation; inspect/list/trace must commit their audit before exposing data.
   Retained evidence and signed `COMMITTED` receipts must outlive audit expiry
   whenever a surviving domain row relies on them. No schema is assumed here.

These are implementation-binding gaps, not a claim that the accepted security
semantics are undecided. No Rust files are created while the exact signing and
authority contracts are unbound. The accepted action/reason/result closed sets
in §1 ruling 11 must be reused in full, including economy/privacy/restore kinds;
no fallback string, value-minting command or unsigned bypass is allowed.

### Boundary qualification required when the prerequisites land

Each negative case must change one invariant while keeping unrelated facts
valid; expected refusal means no authoritative mutation, audit insertion or data
disclosure. This table is a test contract, **not executed runtime evidence**.

| Consumer boundary | Invariant / single negative operator | Required observation |
|---|---|---|
| First submit | Identity: missing signature; unknown principal; wrong namespace, independently | Each refuses before a write |
| First submit | Binding: other environment; other roster; altered typed target, reason or parameter, independently | Each refuses; execute only decoded verified bytes |
| Decode before verify/execute | Provenance: missing/unknown field; alternate noncanonical bytes, independently | Refuse without accepting an unsigned sibling field |
| First submit | Temporal: stale never-executed request; future request outside the bound policy | Refuse without applying; resolve exact future bound first |
| Retry lookup | Replay: same operation id and same bytes after expiry | Return exact stored outcome once; freshness cannot force re-execution |
| Retry lookup | Replay: same operation id and different bytes | Refuse without replacing stored outcome |
| Chain append | Current authority: stale sequence; wrong previous digest; obsolete roster after activation, independently | Refuse with chain and database unchanged |
| Concurrent submit | Replay/concurrency: two writers extend one predecessor | One ordered winner; typed loser, no fork or duplicate effect |
| Historical audit/restore | Provenance: missing middle link; duplicate sequence/fork; changed previous digest; unrecorded activation, independently | Failing verification; restore applies nothing |
| Historical audit/restore | Revocation: removed key signs old-roster bytes before/after activation | Recorded activation fixes history; flag and refuse replay |
| Audit verification | Binding: event kind/target/reason/one parameter differs from signed bytes; missing bytes, independently | Flag each failing row |
| Mutation verification | Provenance: missing receipt; valid REFUSED receipt; wrong request/outcome digest; changed receipt signer/namespace, independently | Row fails; intent never proves execution |
| Restore | Replay: valid COMMITTED receipt and absent outcome | Re-run own checks once; a current refusal applies nothing |
| Restore | Replay: valid REFUSED receipt whose original checks now pass | Preserve refusal; never execute or allow resolve-apply |
| Restore | Missing provenance: no verifying receipt | No replay; restore stops pending signed resolution |
| Read result | Audit commit fails | Inspect/list/trace expose no data |
| Mutation transaction | Audit insert fails | Domain mutation rolls back atomically |
| Retained domain row | Expired audit versus missing retained evidence/receipt/roster | Expired audit alone passes; missing verification material fails |

Session-generation mutation tests are **NOT_APPLICABLE to this no-runtime-change
packet**; future kick/sanction consumers must preserve their own current session
fences. The signature, chain, clock and receipt cases above were BLOCKED in
this preparation. They now run as real tests; the row map in the D1437
checkpoint names each one, and rows 13-15 (restore) are in the `7e2f70bc`
repair checkpoint.

### Cross-domain findings and next allocation

**REPORT_ONLY:** the packet's real executable integration paths are
`apps/game-server/src/bin/oteryn-game-ops.rs` and `node/serve.rs`, with restart
configuration in `node/config.rs`; they are outside this local worker's owned
paths. Do not create a guessed `tools/oteryn-game-ops` replacement. CP must route
those owners, migrations, proto, registry, error numbers and module registration
at a later authorized slot. EXT off-host-head closure remains
ARCH-CONTROL-ANCHOR-0; its absence does not cancel ALPHA first-child preparation.

Whole-diff self-review: this change only records the accepted semantics,
evidenced binding gaps and next qualification work. It introduces no new
authority, signed envelope, trust root, production behavior or readiness claim.

Local validation: `python tools/agents/validate_governance.py` PASS (22 policy
documents, 9 project lanes); `python -m unittest discover -s tools/agents/tests`
PASS (124 tests, 0 failures); `git diff --check` PASS. Owned-path inspection
confirms only this task record changed and HEAD remains the source SHA above.
No Rust/runtime tests ran, because there is no runtime change; the negative
signature/authority/restore cases remain unqualified as stated above.

## CP implementation-binding checkpoint (2026-10-16)

**PROVEN decision evidence:** [CP D1040–D1045](https://github.com/Oteryn/Oteryn-Game/pull/2181#issuecomment-6216592423)
and [OWNER-E acceptance](https://github.com/Oteryn/Oteryn-Game/pull/2181#issuecomment-6216812069).
Initial authoring head: `0be3b3f2673544916228bd6e14db6cf7b45ace7b`;
the only file in this correction is `docs/agents/tasks/GM-1.md`.
These resolve the historical packet's stated gaps as follows; accepted
architecture, closed action/reason/result sets, atomic audit, retained evidence
and restore requirements continue to apply.

- **D1040:** canonical bytes use JCS (D1029), a closed typed object containing
  every parameter, environment, roster digest, `issued_at` (uint64 milliseconds
  encoded as a decimal string), `control_seq` and previous-link digest. Use
  upstream `ssh-keygen -Y sign` / `-Y verify`; the request namespace is
  `oteryn-ops-request-v1` and receipt namespace is
  `oteryn-ops-submission-receipt-v1`. Verify before decoding, then reject any
  non-identical canonical re-encoding. This namespace binding supersedes the
  historical preparation's request-namespace assumptions.
- **D1041:** pin the restart-only roster configuration to the allowed-signers
  file SHA-256 and newest activation digest. Check the submitted previous-link
  against the DB head. ALPHA has no runtime activation; ROSTER-4 remains EXT.
  Retained revisions and recorded activations still support historical checks.
- **D1042:** use `crates/ntp-clock::read_synchronized`; `None` fails closed.
  First execution has a 10-minute window and permits at most 5 minutes future
  skew. Identical request bytes return the stored outcome; different bytes
  under the same operation id refuse. Retry never becomes a second execution.
  The clock interface is present in `crates/ntp-clock/src/lib.rs` at checkpoint
  source HEAD `0be3b3f2673544916228bd6e14db6cf7b45ace7b`.
- **D1043–D1044:** the active path inventory and dependency table above replace
  the guessed tools path and stale gates. `staff_action.proto` / registration
  remain a CP contract allocation, and SANCTION-2 carries session fences.
- **D1045 / OWNER-E:** AUDIT-1 is the first PR. SANCTION-2, REPORT-3, ROSTER-4
  and PREPORT-6 each receive a separate PR with their own migration. The owner
  accepted custody of GM signing keys and the initial roster off the host.

**Next boundary:** after this record/owned-path correction, CP allocates
AUDIT-1. No child has been allocated by this checkpoint; implementation,
fixtures, migration/event/error leases and runtime qualification remain
undelivered. The negative-case table remains a qualification contract, with
the bound clock windows and trusted DB head above replacing its open questions.
This local work changes only this record and grants no readiness, merge or
production authority.

Checkpoint validation: `python tools/agents/validate_governance.py` PASS (22
policy documents, 9 project lanes); `python -m unittest discover -s
tools/agents/tests` PASS (124 tests, 0 failures); `git diff --check` PASS.
No Rust build or runtime test was needed for this record-only correction.

## Allocated AUDIT-1 authoring checkpoint (2026-10-16)

[CP D1085–D1088](https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6218747873)
supersedes the preceding checkpoint's pending allocation. Task
`OTV2-20261016-gm-audit-1` uses existing draft #2181; AUTHORING resumed from
`a4d84863c556fcde127a047c11318c7423f44913`. One writer authors the exact paths
above. This allocation does not start SANCTION-2, REPORT-3, ROSTER-4 or PREPORT-6.

Authored components (source evidence, not runtime qualification):

- `staff/signature.rs`: bounded raw OpenSSH verification before closed typed
  decoding/JCS equality, pinned copied allowed-signers bytes, independent
  environment/roster/activation bindings, exact request/receipt namespaces,
  uint64 decimal strings, synchronized-clock windows, and streaming retained
  chain verification against independently loaded reviewed activations.
- `staff/audit.rs`: complete accepted action/reason/result taxonomy and proposed
  protobuf payload in the ANL envelope. The initial typed request codec covers
  eight ALPHA variants; future allocated owners add complete typed variants for
  their economy/privacy/restore commands. No opaque JSON or fallback string
  accepts an unimplemented command. Registry admission fails closed while the
  actual STAFF_ACTION allocation/retention profile is absent.
- `durability/staff_action.rs`: participant in the existing semantic transaction,
  per-environment locked head, exact-byte retry before freshness, typed conflicting
  operation/predecessor refusal, atomic link/outcome/audit, and signed receipt
  reconciliation. Later domain owners must use its same-physical-transaction
  participant and preserve their independent current fences. No pool getter or
  second domain writer was added. The public root currently offers the audit
  boundary for reads and receipt storage; it refuses domain mutations.
  Trusted time is sampled after all awaited prerequisites; exact terminal replay
  does not consult the clock. A receipt's outcome digest now covers the
  authoritative execution timestamp, and the event/source row must match it.
- Existing ops executable: prepare signed bytes, submit detached signatures,
  prepare/store submission receipts, and bounded streaming audit verification.
  Private keys are never accepted or generated by production commands. Exact
  evidence is retained exclusively before output; an unsigned transport outcome
  never substitutes for an operator-signed COMMITTED receipt.
  `historical-reconcile` is a separate read-only path: the complete independently
  anchored chain seals an exact old request, then a real READ ONLY semantic
  transaction returns its existing outcome or absent/conflict. Missing outcomes
  never enter execution. Current submit still requires the current roster; the
  historical proof grants no current signer or domain authority.
- `crates/release-manifest`: JCS only, upstream
  `serde_json_canonicalizer = 0.4.1`, with RFC8785 ECMAScript number/UTF16 vectors,
  closed duplicate/unknown-field and byte-equality checks. No release manifest
  schema, key custody or SEC authority is implemented here.

D1088's table/DDL proposal lives in the owned transaction source as
`PROPOSED_MIGRATION_0092`; runtime never installs it. It proposes immutable links,
operations/outcomes, signed receipts, ANL outbox, cross-row same-transaction
constraints, payload ceilings, finite audit expiry metadata, and restricted
grants. Seeding an initial head must import independently reviewed signed
activation evidence; no runtime activation or genesis-zero shortcut exists.
Ordinary audit expiry/publication and retained-proof prefix pruning still need
their concrete slot-installed lifecycle; no production retention completeness
is claimed by the schema proposal alone.

D1160/D1161 supersede the historical registration hold for the two own module
lines and 26 fixture forwarding lines only. D1086 still holds node config/control
hunks after Admission, manifests/lock sequentially after
#1942, and the reserved 0092/proto/STAFF_ACTION slots. An explicit private native
assembly recipe is outside this checkout; it is validation input, never a
published shared-path patch or event-number allocation. Real node dispatch and
full operator inspect/report data delivery remain unqualified until those
slots; an audit-only result is not a delivered GM command implementation.

The earlier workspace inventory finding is **resolved by authoring allocation**
[D1091](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6219643346):
`workspace-boundaries.toml` now includes exactly the new member/path/production
entry, `oteryn-game-server` to `oteryn-release-manifest`, and the new crate's empty
internal dependency list. No Client or release-sign edge was added. Root Cargo
membership and lock remain D1086 slots after #1942, so this inventory change
does not claim the unregistered checked-in workspace already passes architecture
or Rust qualification.

Forensic completeness repairs: verification begins at the earliest independently
retained activation, checks every retained operation/outbox/receipt by anti-join
as well as traversing the chain, and compares SQL event routing/schema/retention,
identity, timestamp/expiry, hash and publication-state mirrors to the actual
envelope. The outcome hash and receipt-requirement mirrors are checked too.
For a read lacking an optional signed receipt, a missing event is excused only
after the signed request's latest permitted execution time plus retention. This
fails closed during the ambiguous expiry interval (at most 15 minutes); it does
not trust a database owner to backdate the unsigned read outcome.

Qualification inventory: security tests cover independent missing/unknown/wrong
signature namespace, environment/roster/parameter substitution, authenticated
unknown/duplicate/missing/noncanonical fields, clock bounds/unsynchronized clock,
chain gaps/forks/previous digest, unrecorded activation, removed signer, receipt
namespace/principal, and event kind/target/reason/parameter/missing-byte changes.
Eleven native PostgreSQL families use actual OpenSSH fixtures and the production
participant/proposed DDL: old identical retry and changed operation bytes;
clock/sequence/predecessor/roster refusals; forced audit-insert failure with
domain/head/link/outcome rollback; two-connection predecessor contention;
receipt digests/disposition/chain/idempotency; actual runtime-role grant refusals.
Additional cases cover independently advanced activation/current-roster historical
read-only lookup, clock sampling after an observed real head-lock wait, orphan
outbox detection after owner FK bypass, valid-envelope SQL mirror forgery, and
operation identity forgery after removal of its expired outbox row.
An audit-failure fixture increments a nontransactional sequence before raising,
so an unrelated early error cannot masquerade as audit-insert rollback proof.
The staff pure tests and PostgreSQL families are authored and **NOT_RUN**
at this checkpoint. Missing PG
emits PRE-ROUTING/NONCANONICAL and is not qualification. Restore application,
surviving sanction/roster rows and session-generation mutations need their
actual allocated child consumers; no fixture claims those are passed.

Current cheap checks: governance validator PASS (22 policy documents, 9 lanes),
agent unit suite PASS (124 tests, zero failures), and `git diff --check` PASS.
Native Rust 1.94 formatting PASS for the six server Rust files and the frozen
JCS module; the sole writer verified every before/after file hash and applied
the exact formatting-only patch. Independent v3 source review PASS for all
eleven owned files, with no remaining material finding in that targeted review.
The compile-only cloud results and remaining RED/NOT_RUN scopes are recorded
in the current checkpoint above; staff unit and PG execution remain NOT_RUN. No ready, merge, production authority or runtime-pass
claim follows from this authoring checkpoint.


Durable-column evidence map (D1088 proposal; checked independently of surviving
outbox):

| Proposed table/columns | Independent binding / bounded contract |
| --- | --- |
| links: environment, control_seq, previous_link_sha256, roster_sha256 | Verified signed request/receipt and independently pinned activation history; gapless complete prefix starts at earliest reviewed activation. |
| links: link_sha256, signed_bytes, principal, key_fingerprint, signature | SHA-256 of exact verified bytes; OpenSSH verification using offhost reviewed roster, exact namespace and actual signer fingerprint. |
| links: link_kind | Decoded signed request action distinguishes REQUEST/ACTIVATION; receipt namespace distinguishes RECEIPT. Reverse joins require retained operation/receipt for every nonactivation link. |
| heads: environment, control_seq, link_sha256, activation_sha256 | Complete verified chain terminates at this head; activation digest equals independent current restart config. Same locked head serializes first execution. |
| operations: environment, operation_id (composite PK), request_sha256, request_bytes, principal, key_fingerprint, control_seq | Exact signed request and its verified link; checked even when the outbox has expired. |
| operations: outcome_bytes, outcome_sha256, action_id, transaction_id, occurred_at | Closed canonical outcome, exact SHA-256, canonical UUID identities and execution timestamp; timestamp constrained by signed request execution window and additionally covered by mutation receipt outcome digest. |
| operations: requires_receipt | Derived from signed action kind, never a database authority flag. Outcome disposition/result must obey the committed/OK relation. |
| outbox: environment, operation_id, event_id (PK), transaction_id | Exact signed request and canonical outcome identities, plus reverse operation association. |
| outbox: event_type_id, schema_revision, retention_profile_id, occurred_at, expires_at, envelope, envelope_sha256 | Registered STAFF_ACTION contract, full decoded ANL envelope/payload, outcome timestamp, exact 365-day expiry and exact envelope hash. No event allocation is guessed. |
| outbox: publication_state | Only contract states 1/2 are legal; this mutable lifecycle marker is not a signature or proof of publication. Publication/pruning implementation remains slot-held. |
| receipts: environment, operation_id (composite PK), receipt_bytes, principal, key_fingerprint, signature, control_seq | Verified historical receipt, exact matching receipt chain link and original request signer, request/outcome digests and COMMITTED disposition; reverse scan detects orphan receipts. |
| links/operations/outbox/receipts: created_xact_id | Insert-time current physical transaction check and same-group joins (request/link/operation/outbox; receipt/link). This unsigned local database metadata proves enforced transaction participation at insertion, not an independently signed transaction identity against database-root rewriting. |

The complete operation verifier is shared by ordinary stored-outcome replay,
historical read-only reconciliation and audit verification. Its thirteen-field
negative matrix has no outbox input. The native expiry-boundary case removes the
event, verifies retained operation evidence, then changes only its SQL operation
UUID while retaining signed request/outcome bytes and original transaction stamp;
verification must refuse. Additional negative matrices cover eleven event mirrors
and signed link-kind substitution. Native execution remains pending.


## Recoverable frozen source backup — 2026-10-17

User-requested code backup in this existing draft. This section contains the exact frozen private source bundle and readable integration patch; it is material source recovery data, not a new runtime/ready claim. The published task-branch runtime files keep their allocated integration order. Unified diffs below are encoded as JSON arrays of exact lines (including newlines); concatenate `patch_lines` to recover the byte-identical patch without Markdown trailing-whitespace changes.

Prepared composition: all-target compilation PASS ata4a446f; registered-ID5 PG target compile/link --no-run PASS atfaa906ca. Final715 changes onlya proto comment; Rust/registry/migration bytes unchanged. RuntimePG NOT_RUN; prior11pure and3JCS PASS were not rerun. Full integrated qualification remains separate.

```json
{
  "private_candidate": "715041517515314e4e10c63a405281cd37e72d14",
  "bundle_file": "gm-private-71504151-current-1942.bundle",
  "bundle_bytes": 9525,
  "bundle_sha256": "ceb722363a511d774f139bfded6274790c8dc3608dfef0ad048741f18c37d40b",
  "integration_patch_sha256": "e88e277ddada0acb0e01873b7b3825e608e79de1339f8a11e548464ddca55045",
  "source_manifest_sha256": "b70cc19999436311166d018e8b105e6464400a17c8f1b37c8b0ee9ff627dce1a",
  "restoration": "Base64-decode the bundle block, verify SHA256, then git bundle verify against the published prerequisites below; fetch its HEAD into an isolated checkout. The bundle preserves private commits and merge resolutions.",
  "shared_runtime_integration": "Private source backup only; the allocated runtime/shared-file slot still applies.",
  "status": "NOT_FULLY_QUALIFIED",
  "published_prerequisites": [
    "bbdbb689dad872074fb12befccc3b47e5eb2ce4b",
    "fdc20be476e6d38913d254209ccfa710743d6c59"
  ]
}
```

<details><summary>GM D1086 — exact Git bundle (base64)</summary>

```text
IyB2MiBnaXQgYnVuZGxlCi1mZGMyMGJlNDc2ZTZkMzg5MTNkMjU0MjA5Y2NmYTcxMDc0M2Q2YzU5
IGZpeChjbGllbnQpOiBiaW5kIEhVRCBhbmQgc2V0dGluZ3MgdG8gYWN0dWFsIHJ1bnRpbWUgc3Rh
dGUKLWJiZGJiNjg5ZGFkODcyMDc0ZmIxMmJlZmNjYzNiNDdlNWViMmNlNGIgY2hvcmUoZ20pOiBy
ZWdpc3RlciBhdWRpdCBtb2R1bGVzIGFuZCByZWNvcmQgcXVhbGlmaWNhdGlvbiBsaW1pdHMKNzE1
MDQxNTE3NTE1MzE0ZTRlMTBjNjNhNDA1MjgxY2QzN2U3MmQxNCBIRUFECgpQQUNLAAAAAgAAAC6T
EnicpY9NTsMwEIX3PsXskao48U+CKtQCqsQGkNo9mrEn1DSOkeP0ANyNe+Gegbf7vic96ZXMDMY4
75HYjqgYUVUidIPx2hKOPLaN7ho0LL4x81xgRBwa41B15KRizVZKj0oT9sYYIqc6S51GgWs5pwyP
mMvvT4qBv+AIW6rIl6Ukd57CvFzC7jNimDYuxQeQdmhl3+u+hbumRlQbQyn8zxnxnsMVC8OzrP4e
fHJrvJ2ZGBf2cDztD4eP/dPp5e0V+Hpr5jUSZ/EHHi9em5YSeJylj0FuwjAQRfc+xewrkJ2MY6eq
qlIqpG4oEuyrcTIGQ0KQMyB1y916r4Yz9O/eWzzpS2YGF6yzusTWRvSBsI6BdemYvA6mjL7xum6R
UV0o81mAkBCrWGAbOaIlG2xdoqaoval85TxFis6RoqschgzvlOX3PvSJj7CFlzAhn0YZmkOXzuMp
ve17St28GfpXMK4ujPdYGHjS09Rk+yTC/8yoTU43EoYPM/lnyLxP46O63Mw6ppFb2O4Wq9X3Yrn7
/FoD3x5P5efCYNUf/Mxcz5UTeJylj01OwzAQhfc+xeyRkGfyY1sgRAuqxAaQ2j2aOJPGbRwjx+EA
3I17EXEFnt7me4tPeiWLgEXn0feEhFpXXdNz3RunB916rqtWm6bDrqm8+uQscwHmFq0hGmoyJORc
raUf3MDoxQppR77TxKh4LWPKsOdcfr5TDHKBI9x3G8p1KcmPU5iXa3g8Rw7TrU/xAdA4QmvRNHCj
t6htjaEU+adGvefwxUXgGbVtIcxFzplLSDNs9Wv+e4aupjsY09TD8bQ7HD52T6eXt1eY1yg5eJiE
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
IiV+/KN8
```
</details>

<details><summary>GM D1086 — exact frozen integration patch lines (JSON)</summary>

```json
{
  "patch_lines": [
    "diff --git a/Cargo.lock b/Cargo.lock\n",
    "index 68194af2..bd6fb7d6 100644\n",
    "--- a/Cargo.lock\n",
    "+++ b/Cargo.lock\n",
    "@@ -2723,6 +2723,15 @@ dependencies = [\n",
    "  \"serde_json\",\n",
    " ]\n",
    " \n",
    "+[[package]]\n",
    "+name = \"oteryn-release-manifest\"\n",
    "+version = \"0.1.0\"\n",
    "+dependencies = [\n",
    "+ \"serde\",\n",
    "+ \"serde_json\",\n",
    "+ \"serde_json_canonicalizer\",\n",
    "+]\n",
    "+\n",
    " [[package]]\n",
    " name = \"oteryn-client\"\n",
    " version = \"0.1.0\"\n",
    "@@ -2847,6 +2856,7 @@ dependencies = [\n",
    "  \"oteryn-foundation\",\n",
    "  \"oteryn-ntp-clock\",\n",
    "  \"oteryn-protocol-oteryn\",\n",
    "+ \"oteryn-release-manifest\",\n",
    "  \"oteryn-simulation-determinism\",\n",
    "  \"oteryn-world-bundle\",\n",
    "  \"oteryn-world-bundle-compiler\",\n",
    "@@ -3697,6 +3707,12 @@ version = \"1.0.23\"\n",
    " source = \"registry+https://github.com/rust-lang/crates.io-index\"\n",
    " checksum = \"9774ba4a74de5f7b1c1451ed6cd5285a32eddb5cccb8cc655a4e50009e06477f\"\n",
    " \n",
    "+[[package]]\n",
    "+name = \"ryu-js\"\n",
    "+version = \"1.0.3\"\n",
    "+source = \"registry+https://github.com/rust-lang/crates.io-index\"\n",
    "+checksum = \"04d056b875a9d2e6cb9a61d127afee9ac5999b9f87bcb32079d1318e505be714\"\n",
    "+\n",
    " [[package]]\n",
    " name = \"same-file\"\n",
    " version = \"1.0.6\"\n",
    "@@ -3824,6 +3840,17 @@ dependencies = [\n",
    "  \"zmij\",\n",
    " ]\n",
    " \n",
    "+[[package]]\n",
    "+name = \"serde_json_canonicalizer\"\n",
    "+version = \"0.4.1\"\n",
    "+source = \"registry+https://github.com/rust-lang/crates.io-index\"\n",
    "+checksum = \"87f7bb64f8c7e9f783fdfe883b5fe93a90bee1597c63705baa91842c5f698620\"\n",
    "+dependencies = [\n",
    "+ \"ryu-js\",\n",
    "+ \"serde\",\n",
    "+ \"serde_json\",\n",
    "+]\n",
    "+\n",
    " [[package]]\n",
    " name = \"serde_repr\"\n",
    " version = \"0.1.21\"\n",
    "diff --git a/Cargo.toml b/Cargo.toml\n",
    "index 1af3eed0..967c484d 100644\n",
    "--- a/Cargo.toml\n",
    "+++ b/Cargo.toml\n",
    "@@ -7,6 +7,7 @@ members = [\n",
    "     \"crates/diagnostics\",\n",
    "     \"crates/error-codes\",\n",
    "     \"crates/ntp-clock\",\n",
    "+    \"crates/release-manifest\",\n",
    "     \"crates/client-runtime\",\n",
    "     \"crates/platform-contracts\",\n",
    "     \"crates/platform-client\",\n",
    "diff --git a/apps/game-server/Cargo.toml b/apps/game-server/Cargo.toml\n",
    "index 959727d0..028940c6 100644\n",
    "--- a/apps/game-server/Cargo.toml\n",
    "+++ b/apps/game-server/Cargo.toml\n",
    "@@ -19,6 +19,7 @@ ed25519-dalek.workspace = true\n",
    " oteryn-error-codes = { version = \"=0.1.0\", path = \"../../crates/error-codes\" }\n",
    " oteryn-foundation = { version = \"=0.1.0\", path = \"../../crates/foundation\" }\n",
    " oteryn-ntp-clock = { version = \"=0.1.0\", path = \"../../crates/ntp-clock\" }\n",
    "+oteryn-release-manifest = { version = \"=0.1.0\", path = \"../../crates/release-manifest\" }\n",
    " oteryn-protocol-oteryn = { version = \"=0.1.0\", path = \"../../crates/protocol-oteryn\" }\n",
    " oteryn-simulation-determinism = { version = \"=0.1.0\", path = \"../../crates/simulation-determinism\" }\n",
    " oteryn-world-bundle = { version = \"=0.1.0\", path = \"../../crates/world-bundle\" }\n",
    "@@ -166,3 +167,7 @@ test = false\n",
    " \n",
    " [lints]\n",
    " workspace = true\n",
    "+\n",
    "+[[test]]\n",
    "+name = \"staff_audit_postgres\"\n",
    "+test = false\n",
    "diff --git a/apps/game-server/migrations/0092_staff_audit.sql b/apps/game-server/migrations/0092_staff_audit.sql\n",
    "new file mode 100644\n",
    "index 00000000..28db4ab1\n",
    "--- /dev/null\n",
    "+++ b/apps/game-server/migrations/0092_staff_audit.sql\n",
    "@@ -0,0 +1,118 @@\n",
    "+\n",
    "+CREATE TABLE game_staff_control_links (\n",
    "+ environment text NOT NULL CHECK(length(environment) BETWEEN 1 AND 128),\n",
    "+ control_seq numeric(20,0) NOT NULL CHECK(control_seq BETWEEN 1 AND 18446744073709551615),\n",
    "+ link_sha256 text NOT NULL CHECK(link_sha256 ~ '^[0-9a-f]{64}$'),\n",
    "+ previous_link_sha256 text NOT NULL CHECK(previous_link_sha256 ~ '^[0-9a-f]{64}$'),\n",
    "+ roster_sha256 text NOT NULL CHECK(roster_sha256 ~ '^[0-9a-f]{64}$'),\n",
    "+ link_kind text NOT NULL CHECK(link_kind IN ('REQUEST','RECEIPT','ACTIVATION')),\n",
    "+ signed_bytes bytea NOT NULL CHECK(octet_length(signed_bytes) BETWEEN 1 AND 32768 AND encode(sha256(signed_bytes),'hex')=link_sha256),\n",
    "+ principal text NOT NULL CHECK(length(principal) BETWEEN 1 AND 128),\n",
    "+ key_fingerprint text NOT NULL CHECK(length(key_fingerprint) BETWEEN 1 AND 128),\n",
    "+ signature text NOT NULL CHECK(octet_length(signature) BETWEEN 1 AND 8192),\n",
    "+ created_xact_id bigint NOT NULL CHECK(created_xact_id=txid_current()),\n",
    "+ PRIMARY KEY(environment,control_seq), UNIQUE(environment,link_sha256), UNIQUE(environment,control_seq,link_sha256));\n",
    "+CREATE TABLE game_staff_control_heads (\n",
    "+ environment text PRIMARY KEY, control_seq numeric(20,0) NOT NULL,\n",
    "+ link_sha256 text NOT NULL, activation_sha256 text NOT NULL CHECK(activation_sha256 ~ '^[0-9a-f]{64}$'),\n",
    "+ FOREIGN KEY(environment,control_seq,link_sha256) REFERENCES game_staff_control_links(environment,control_seq,link_sha256));\n",
    "+CREATE TABLE game_staff_operations (\n",
    "+ environment text NOT NULL, operation_id uuid NOT NULL CHECK(game_character_is_uuid_v7(operation_id)),\n",
    "+ request_sha256 text NOT NULL, request_bytes bytea NOT NULL CHECK(octet_length(request_bytes) BETWEEN 1 AND 32768 AND encode(sha256(request_bytes),'hex')=request_sha256),\n",
    "+ outcome_bytes bytea NOT NULL CHECK(octet_length(outcome_bytes) BETWEEN 1 AND 4096),\n",
    "+ outcome_sha256 text NOT NULL CHECK(encode(sha256(outcome_bytes),'hex')=outcome_sha256),\n",
    "+ principal text NOT NULL, key_fingerprint text NOT NULL,\n",
    "+ action_id uuid NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(action_id)), transaction_id uuid NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(transaction_id)),\n",
    "+ control_seq numeric(20,0) NOT NULL, requires_receipt boolean NOT NULL,\n",
    "+ occurred_at bigint NOT NULL CHECK(occurred_at>0),\n",
    "+ created_xact_id bigint NOT NULL CHECK(created_xact_id=txid_current()),\n",
    "+ PRIMARY KEY(environment,operation_id), UNIQUE(environment,request_sha256),\n",
    "+ FOREIGN KEY(environment,control_seq) REFERENCES game_staff_control_links(environment,control_seq));\n",
    "+CREATE TABLE game_staff_action_outbox (\n",
    "+ environment text NOT NULL, operation_id uuid NOT NULL,\n",
    "+ event_id uuid PRIMARY KEY CHECK(game_character_is_uuid_v7(event_id)), transaction_id uuid NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(transaction_id)),\n",
    "+ event_type_id bigint NOT NULL CHECK(event_type_id>0), schema_revision bigint NOT NULL CHECK(schema_revision>0),\n",
    "+ retention_profile_id text NOT NULL CHECK(retention_profile_id='STAFF_ACTION_AUDIT_RETENTION_V1'),\n",
    "+ occurred_at bigint NOT NULL CHECK(occurred_at>0), expires_at bigint NOT NULL CHECK(expires_at=occurred_at+31536000000),\n",
    "+ envelope bytea NOT NULL CHECK(octet_length(envelope) BETWEEN 1 AND 50176),\n",
    "+ envelope_sha256 text NOT NULL CHECK(encode(sha256(envelope),'hex')=envelope_sha256),\n",
    "+ publication_state smallint NOT NULL CHECK(publication_state IN (1,2)),\n",
    "+ created_xact_id bigint NOT NULL,\n",
    "+ FOREIGN KEY(environment,operation_id) REFERENCES game_staff_operations(environment,operation_id),\n",
    "+ FOREIGN KEY(event_id) REFERENCES game_staff_operations(action_id),\n",
    "+ FOREIGN KEY(transaction_id) REFERENCES game_staff_operations(transaction_id));\n",
    "+CREATE TABLE game_staff_action_receipts (\n",
    "+ environment text NOT NULL, operation_id uuid NOT NULL,\n",
    "+ receipt_bytes bytea NOT NULL CHECK(octet_length(receipt_bytes) BETWEEN 1 AND 32768),\n",
    "+ principal text NOT NULL, key_fingerprint text NOT NULL,\n",
    "+ signature text NOT NULL CHECK(octet_length(signature) BETWEEN 1 AND 8192),\n",
    "+ control_seq numeric(20,0) NOT NULL,\n",
    "+ created_xact_id bigint NOT NULL CHECK(created_xact_id=txid_current()),\n",
    "+ PRIMARY KEY(environment,operation_id),\n",
    "+ FOREIGN KEY(environment,operation_id) REFERENCES game_staff_operations(environment,operation_id),\n",
    "+ FOREIGN KEY(environment,control_seq) REFERENCES game_staff_control_links(environment,control_seq));\n",
    "+CREATE FUNCTION game_staff_outbox_insert_stamp() RETURNS trigger LANGUAGE plpgsql AS $$\n",
    "+BEGIN\n",
    "+ IF NEW.created_xact_id<>txid_current() THEN RAISE EXCEPTION 'staff insert transaction mismatch' USING ERRCODE='23514'; END IF;\n",
    "+ RETURN NEW;\n",
    "+END $$;\n",
    "+CREATE TRIGGER game_staff_outbox_insert_stamp BEFORE INSERT ON game_staff_action_outbox\n",
    "+ FOR EACH ROW EXECUTE FUNCTION game_staff_outbox_insert_stamp();\n",
    "+CREATE FUNCTION game_staff_immutable() RETURNS trigger LANGUAGE plpgsql AS $$\n",
    "+BEGIN RAISE EXCEPTION 'immutable staff evidence' USING ERRCODE='23514'; END $$;\n",
    "+CREATE FUNCTION game_staff_operation_has_audit() RETURNS trigger LANGUAGE plpgsql AS $$\n",
    "+BEGIN\n",
    "+ IF NOT EXISTS(SELECT 1 FROM game_staff_action_outbox a WHERE a.environment=NEW.environment AND a.operation_id=NEW.operation_id\n",
    "+ AND a.event_id=NEW.action_id AND a.transaction_id=NEW.transaction_id AND a.created_xact_id=NEW.created_xact_id)\n",
    "+ THEN RAISE EXCEPTION 'missing same-transaction staff audit' USING ERRCODE='23514'; END IF;\n",
    "+ IF NOT EXISTS(SELECT 1 FROM game_staff_control_links l WHERE l.environment=NEW.environment AND l.control_seq=NEW.control_seq\n",
    "+ AND l.link_kind='REQUEST' AND l.signed_bytes=NEW.request_bytes AND l.link_sha256=NEW.request_sha256\n",
    "+ AND l.principal=NEW.principal AND l.key_fingerprint=NEW.key_fingerprint AND l.created_xact_id=NEW.created_xact_id)\n",
    "+ THEN RAISE EXCEPTION 'missing same-transaction staff link' USING ERRCODE='23514'; END IF;\n",
    "+ RETURN NULL;\n",
    "+END $$;\n",
    "+CREATE FUNCTION game_staff_link_extends_head() RETURNS trigger LANGUAGE plpgsql AS $$\n",
    "+DECLARE h game_staff_control_heads%ROWTYPE;\n",
    "+BEGIN\n",
    "+ IF NEW.link_kind='ACTIVATION' THEN\n",
    "+   IF current_user IN ('oteryn_game_runtime','oteryn_game_control') THEN\n",
    "+     RAISE EXCEPTION 'runtime activation forbidden' USING ERRCODE='23514';\n",
    "+   END IF;\n",
    "+   RETURN NEW;\n",
    "+ END IF;\n",
    "+ SELECT * INTO h FROM game_staff_control_heads WHERE environment=NEW.environment FOR UPDATE;\n",
    "+ IF NOT FOUND OR NEW.control_seq<>h.control_seq+1 OR NEW.previous_link_sha256<>h.link_sha256 THEN\n",
    "+   RAISE EXCEPTION 'staff predecessor conflict' USING ERRCODE='23514';\n",
    "+ END IF;\n",
    "+ RETURN NEW;\n",
    "+END $$;\n",
    "+CREATE FUNCTION game_staff_link_has_outcome() RETURNS trigger LANGUAGE plpgsql AS $$\n",
    "+BEGIN\n",
    "+ IF NEW.link_kind='REQUEST' AND NOT EXISTS(SELECT 1 FROM game_staff_operations o WHERE o.environment=NEW.environment\n",
    "+ AND o.control_seq=NEW.control_seq AND o.request_bytes=NEW.signed_bytes AND o.created_xact_id=NEW.created_xact_id)\n",
    "+ THEN RAISE EXCEPTION 'staff link has no outcome' USING ERRCODE='23514'; END IF;\n",
    "+ IF NEW.link_kind='RECEIPT' AND NOT EXISTS(SELECT 1 FROM game_staff_action_receipts r WHERE r.environment=NEW.environment\n",
    "+ AND r.control_seq=NEW.control_seq AND r.receipt_bytes=NEW.signed_bytes AND r.principal=NEW.principal\n",
    "+ AND r.key_fingerprint=NEW.key_fingerprint AND r.signature=NEW.signature AND r.created_xact_id=NEW.created_xact_id)\n",
    "+ THEN RAISE EXCEPTION 'staff receipt link has no receipt' USING ERRCODE='23514'; END IF;\n",
    "+ IF NOT EXISTS(SELECT 1 FROM game_staff_control_heads h WHERE h.environment=NEW.environment AND h.control_seq>=NEW.control_seq)\n",
    "+ THEN RAISE EXCEPTION 'staff head not advanced' USING ERRCODE='23514'; END IF;\n",
    "+ RETURN NULL;\n",
    "+END $$;\n",
    "+CREATE TRIGGER game_staff_link_predecessor BEFORE INSERT ON game_staff_control_links\n",
    "+ FOR EACH ROW EXECUTE FUNCTION game_staff_link_extends_head();\n",
    "+CREATE CONSTRAINT TRIGGER game_staff_link_outcome AFTER INSERT ON game_staff_control_links\n",
    "+ DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_staff_link_has_outcome();\n",
    "+CREATE CONSTRAINT TRIGGER game_staff_operation_audit AFTER INSERT ON game_staff_operations\n",
    "+ DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_staff_operation_has_audit();\n",
    "+CREATE TRIGGER game_staff_link_immutable BEFORE UPDATE OR DELETE ON game_staff_control_links FOR EACH ROW EXECUTE FUNCTION game_staff_immutable();\n",
    "+CREATE TRIGGER game_staff_link_no_truncate BEFORE TRUNCATE ON game_staff_control_links FOR EACH STATEMENT EXECUTE FUNCTION game_staff_immutable();\n",
    "+CREATE TRIGGER game_staff_operation_immutable BEFORE UPDATE OR DELETE ON game_staff_operations FOR EACH ROW EXECUTE FUNCTION game_staff_immutable();\n",
    "+CREATE TRIGGER game_staff_operation_no_truncate BEFORE TRUNCATE ON game_staff_operations FOR EACH STATEMENT EXECUTE FUNCTION game_staff_immutable();\n",
    "+CREATE TRIGGER game_staff_receipt_immutable BEFORE UPDATE OR DELETE ON game_staff_action_receipts FOR EACH ROW EXECUTE FUNCTION game_staff_immutable();\n",
    "+CREATE TRIGGER game_staff_receipt_no_truncate BEFORE TRUNCATE ON game_staff_action_receipts FOR EACH STATEMENT EXECUTE FUNCTION game_staff_immutable();\n",
    "+REVOKE ALL ON game_staff_control_links,game_staff_control_heads,game_staff_operations,game_staff_action_outbox,game_staff_action_receipts FROM PUBLIC;\n",
    "+GRANT SELECT,INSERT ON game_staff_control_links,game_staff_operations,game_staff_action_outbox,game_staff_action_receipts TO oteryn_game_runtime,oteryn_game_control;\n",
    "+GRANT SELECT ON game_staff_control_heads TO oteryn_game_runtime,oteryn_game_control;\n",
    "+GRANT UPDATE(control_seq,link_sha256) ON game_staff_control_heads TO oteryn_game_runtime,oteryn_game_control;\n",
    "+REVOKE ALL ON FUNCTION game_staff_immutable(),game_staff_operation_has_audit(),game_staff_link_extends_head(),game_staff_link_has_outcome() FROM PUBLIC;\n",
    "diff --git a/apps/game-server/src/node/config.rs b/apps/game-server/src/node/config.rs\n",
    "index b944b6ec..ca67a7df 100644\n",
    "--- a/apps/game-server/src/node/config.rs\n",
    "+++ b/apps/game-server/src/node/config.rs\n",
    "@@ -246,6 +246,8 @@ pub struct BootConfig {\n",
    " #[derive(Debug, Clone, PartialEq, Eq, Deserialize)]\n",
    " #[serde(deny_unknown_fields)]\n",
    " pub struct NodeConfig {\n",
    "+    /// D1041 restart-only reviewed independent staff trust; absent disables staff control.\n",
    "+    pub staff: Option<crate::staff::signature::TrustConfig>,\n",
    "     pub listener: ListenerConfig,\n",
    "     pub scope: ScopeConfig,\n",
    "     pub database: DatabaseConfig,\n",
    "diff --git a/apps/game-server/src/node/serve.rs b/apps/game-server/src/node/serve.rs\n",
    "index 150ab680..46843748 100644\n",
    "--- a/apps/game-server/src/node/serve.rs\n",
    "+++ b/apps/game-server/src/node/serve.rs\n",
    "@@ -85,7 +85,7 @@ const LAST_POSITION_FLUSH_BUDGET: Duration = Duration::from_secs(3);\n",
    " const AUDIT_INTERVAL: Duration = Duration::from_secs(60 * 60);\n",
    " const AUDIT_BATCH: u16 = 64;\n",
    " const AUDIT_RUN_CAP: u32 = 64;\n",
    "-const CONTROL_REQUEST_BYTES: usize = 64;\n",
    "+const CONTROL_REQUEST_BYTES: usize = crate::staff::signature::MAX_CONTROL_BYTES;\n",
    " const CONTROL_DEADLINE: Duration = Duration::from_secs(30);\n",
    " const MAX_GAMEPLAY_KEY_BYTES: usize = 16 * 1024;\n",
    " \n",
    "@@ -1904,14 +1904,26 @@ fn parse_operation_id(request: &[u8]) -> Option<[u8; 16]> {\n",
    "     (bytes[6] >> 4 == 7 && bytes[8] & 0xc0 == 0x80).then_some(bytes)\n",
    " }\n",
    " \n",
    "-async fn handle_control(stream: &mut UnixStream) -> Option<[u8; 16]> {\n",
    "-    let mut request = Vec::with_capacity(CONTROL_REQUEST_BYTES);\n",
    "+enum ControlInput {\n",
    "+    Bootstrap([u8; 16]),\n",
    "+    Staff(crate::staff::signature::ControlMessage),\n",
    "+}\n",
    "+async fn handle_control(stream: &mut UnixStream) -> Option<ControlInput> {\n",
    "+    let mut request = Vec::with_capacity(64);\n",
    "     let mut limited = (&mut *stream).take(CONTROL_REQUEST_BYTES as u64 + 1);\n",
    "     limited.read_to_end(&mut request).await.ok()?;\n",
    "     if request.len() > CONTROL_REQUEST_BYTES {\n",
    "         return None;\n",
    "     }\n",
    "-    parse_operation_id(&request)\n",
    "+    if let Some(bytes) = request.strip_prefix(crate::staff::signature::CONTROL_PREFIX) {\n",
    "+        let message =\n",
    "+            oteryn_release_manifest::jcs::decode(bytes, crate::staff::signature::MAX_CONTROL_BYTES)\n",
    "+                .ok()?;\n",
    "+        // Only the bounded transport envelope is decoded here. The executable\n",
    "+        // inner request remains opaque until OpenSSH verifies its exact bytes.\n",
    "+        return Some(ControlInput::Staff(message));\n",
    "+    }\n",
    "+    parse_operation_id(&request).map(ControlInput::Bootstrap)\n",
    " }\n",
    " \n",
    " /// One request at a time, bounded size and deadline; only uid 0 is served.\n",
    "@@ -1923,6 +1935,7 @@ async fn control_loop(\n",
    "     proof: &NodeIncarnationProof,\n",
    "     evidence: &FreshEvidenceSource,\n",
    "     intents: &ProducerDescriptor,\n",
    "+    staff_trust: Option<&std::sync::Arc<crate::staff::signature::VerifierTrust>>,\n",
    "     stop: &CancellationToken,\n",
    " ) {\n",
    "     let capacity = TransientCapacity::new();\n",
    "@@ -1942,14 +1955,46 @@ async fn control_loop(\n",
    "         let connection = uuid_v7().ok().map(|bytes| uuid_text(&bytes));\n",
    "         let connection = connection.as_deref();\n",
    "         let served = tokio::time::timeout(CONTROL_DEADLINE, async {\n",
    "-            let answer = match handle_control(&mut stream).await {\n",
    "-                Some(operation) => {\n",
    "+            let input = handle_control(&mut stream).await;\n",
    "+            if let Some(ControlInput::Staff(message)) = input {\n",
    "+                let answer = if let Some(trust) = staff_trust {\n",
    "+                    match message {\n",
    "+                        crate::staff::signature::ControlMessage::Request(signed) => root\n",
    "+                            .audit_staff_read(std::sync::Arc::clone(trust), signed)\n",
    "+                            .await\n",
    "+                            .ok()\n",
    "+                            .map(|outcome| outcome.bytes),\n",
    "+                        crate::staff::signature::ControlMessage::Receipt(signed) => root\n",
    "+                            .store_staff_receipt(std::sync::Arc::clone(trust), signed)\n",
    "+                            .await\n",
    "+                            .ok()\n",
    "+                            .map(|()| b\"receipt-stored\".to_vec()),\n",
    "+                    }\n",
    "+                } else {\n",
    "+                    None\n",
    "+                };\n",
    "+                event_traced(\n",
    "+                    if answer.is_some() {\n",
    "+                        \"event=staff_audit result=recorded\"\n",
    "+                    } else {\n",
    "+                        \"event=staff_audit result=rejected\"\n",
    "+                    },\n",
    "+                    connection,\n",
    "+                );\n",
    "+                let _ = stream\n",
    "+                    .write_all(answer.as_deref().unwrap_or(b\"rejected\"))\n",
    "+                    .await;\n",
    "+                let _ = stream.shutdown().await;\n",
    "+                return;\n",
    "+            }\n",
    "+            let answer = match input {\n",
    "+                Some(ControlInput::Bootstrap(operation)) => {\n",
    "                     bootstrap_one(\n",
    "                         root, authority, proof, evidence, intents, &capacity, operation,\n",
    "                     )\n",
    "                     .await\n",
    "                 }\n",
    "-                None => \"rejected\",\n",
    "+                _ => \"rejected\",\n",
    "             };\n",
    "             event_traced(\n",
    "                 &format!(\"event=character_bootstrap result={answer}\"),\n",
    "@@ -2396,6 +2441,13 @@ async fn boot_and_serve(\n",
    "     signalled: &CancellationToken,\n",
    " ) -> Result<(), BootError> {\n",
    "     let config = &material.config;\n",
    "+    let staff_trust = config\n",
    "+        .staff\n",
    "+        .as_ref()\n",
    "+        .map(|staff| staff.load())\n",
    "+        .transpose()\n",
    "+        .map_err(|_| BootError::Startup(StartupError::Invalid { key: \"staff\" }))?\n",
    "+        .map(std::sync::Arc::new);\n",
    "     let scope = RuntimeScopeRefV1::channel(material.world, material.channel);\n",
    "     let Some(proof) = first(register(root, &material.launch), signalled.cancelled()).await else {\n",
    "         stopped_before_ready();\n",
    "@@ -2712,6 +2764,7 @@ async fn boot_and_serve(\n",
    "         &proof,\n",
    "         evidence,\n",
    "         &material.intents,\n",
    "+        staff_trust.as_ref(),\n",
    "         &shutdown,\n",
    "     ));\n",
    "     let mut expiry = pin!(audit_expiry_loop(root, &authority, &loops_stop));\n",
    "diff --git a/docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json b/docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json\n",
    "index 41465e49..f282609c 100644\n",
    "--- a/docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json\n",
    "+++ b/docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json\n",
    "@@ -132,6 +132,20 @@\n",
    "       \"deletion_or_anonymization_policy\": \"Delete the retained player-linked event envelope and payload at ordinary expiry; do not replace deletion with indefinite anonymized or pseudonymous retention. Expiry never weakens the authoritative guild, membership and guild bank tables, which this profile never deletes.\",\n",
    "       \"legal_hold_policy\": \"Explicit case/legal authorization must record reason, authorizing actor, start time and exact affected records or scope, with audited placement, access, scope change and release; release returns to the original expiry without resetting the clock.\",\n",
    "       \"policy_revision\": 1\n",
    "+    },\n",
    "+    {\n",
    "+      \"id\": \"STAFF_ACTION_AUDIT_RETENTION_V1\",\n",
    "+      \"purpose\": \"Prove every staff, operator and Platform-security action on players\",\n",
    "+      \"privacy_class\": \"RESTRICTED_PLAYER_LINKED\",\n",
    "+      \"finite_retention_duration_or_ceiling\": \"P1Y (owner ruling 2026-10-06, \u00a71 Q3 a)\",\n",
    "+      \"permitted_roles\": [\n",
    "+        \"owner\",\n",
    "+        \"authorized security\"\n",
    "+      ],\n",
    "+      \"export_redaction_policy\": \"Case-scoped authorized export redacts unrelated player-linked fields.\",\n",
    "+      \"deletion_or_anonymization_policy\": \"Delete the retained player-linked event, envelope, and payload at ordinary expiry; do not convert it into an indefinite pseudonymous analytics record.\",\n",
    "+      \"legal_hold_policy\": \"Explicit case/legal authorization must record reason, authorizing actor, start time, affected records or scope, and audited access; release returns the record to ordinary expiry and deletion.\",\n",
    "+      \"policy_revision\": 1\n",
    "     }\n",
    "   ],\n",
    "   \"event_type_rules\": {\n",
    "@@ -183,6 +197,18 @@\n",
    "       \"privacy_class_floor\": \"RESTRICTED_PLAYER_LINKED\",\n",
    "       \"retention_profile_id\": \"ECONOMY_LEDGER_RETENTION_V1\",\n",
    "       \"atomic_mutation_evidence\": true\n",
    "+    },\n",
    "+    {\n",
    "+      \"id\": 5,\n",
    "+      \"name\": \"STAFF_ACTION\",\n",
    "+      \"owner_gate\": \"OPS_GM_01_STAFF_ACTIONS\",\n",
    "+      \"payload_schema\": \"docs/contracts/game-events/v2/staff_action.proto\",\n",
    "+      \"payload_message\": \"oteryn.events.v2.StaffActionV1\",\n",
    "+      \"current_schema_revision\": 1,\n",
    "+      \"durability_class\": \"DURABLE_AUDIT\",\n",
    "+      \"privacy_class_floor\": \"RESTRICTED_PLAYER_LINKED\",\n",
    "+      \"retention_profile_id\": \"STAFF_ACTION_AUDIT_RETENTION_V1\",\n",
    "+      \"atomic_mutation_evidence\": true\n",
    "     }\n",
    "   ],\n",
    "   \"notes\": [\n",
    "diff --git a/docs/contracts/game-events/v2/staff_action.proto b/docs/contracts/game-events/v2/staff_action.proto\n",
    "new file mode 100644\n",
    "index 00000000..d55b4823\n",
    "--- /dev/null\n",
    "+++ b/docs/contracts/game-events/v2/staff_action.proto\n",
    "@@ -0,0 +1,28 @@\n",
    "+syntax = \"proto3\";\n",
    "+\n",
    "+package oteryn.events.v2;\n",
    "+\n",
    "+// OPS-GM-AUDIT-1 / ARCH-LIVE-READINESS-0 \u00a71 ruling 11.\n",
    "+// Schema revision 1; CP D1188 leases STAFF_ACTION event_type_id = 5.\n",
    "+// Exact signed JCS and outcome bytes are retained; protobuf is not semantic JCS.\n",
    "+// Payload <= 49152 bytes; the ANL envelope <= 50176 bytes.\n",
    "+// Fields mirror apps/game-server/src/staff/audit.rs::StaffActionV1.\n",
    "+message StaffActionV1 {\n",
    "+  bytes action_id = 1;\n",
    "+  string actor_kind = 2;\n",
    "+  string action_kind = 3;\n",
    "+  bytes target_jcs = 4;\n",
    "+  string reason = 5;\n",
    "+  string result = 6;\n",
    "+  bytes signed_request = 7;\n",
    "+  string principal = 8;\n",
    "+  string key_fingerprint = 9;\n",
    "+  string signature = 10;\n",
    "+  string roster_sha256 = 11;\n",
    "+  string request_sha256 = 12;\n",
    "+  bytes parameters_jcs = 13;\n",
    "+  string trace_id = 14;\n",
    "+  bytes basis_refs_jcs = 15;\n",
    "+  bytes outcome_jcs = 16;\n",
    "+  string environment = 17;\n",
    "+}\n"
  ]
}
```
</details>
