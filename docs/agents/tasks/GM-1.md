# GM-1 — GM tools, mute, kick, staff audit and reports (consolidated task card)

```yaml
task_id: GM-1
title: "GM-1: signed staff requests with atomic STAFF_ACTION audit; mute, kick, inspect, item-text reports, restore replay"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 6 row GM-1 (row 136); App. 7 GM-1; §5 Wave 5"
brief: "docs/architecture/reviews/OTERYN_GAME_ARCH_LIVE_READINESS_2026-10-06.md §1 (rulings 2, 8, 11; packet table OPS-GM-AUDIT-1 ALPHA first); ALPHA-OPS ruling 8; WRITE0 §6"
class: "hard (child graph), security, persistence and authority review"
base_branch: main
branch: null
pr: null
needs_migration: true   # next free number at allocation time (only one migration PR in the queue); draft had two units (staff audit, staff moderation)
needs_registry: true    # GAME_EVENT_FOUNDATION_REGISTRY STAFF_ACTION (+ retention profile), RESOURCE_LIMITS_REGISTRY staff wire bounds
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **OPEN**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: none. Existing logout is reusable; no staff-action owner, sanction/mute store or staff desk from GM-1 is retained on main. Evidence: `crates/gameplay/dispatch/src/gameplay_transport/logout.rs`.

Remaining: Implement staff audit/sanctions and real mute/kick/report consumers within the accepted owner contracts. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2181, branch skeleton/gm-1

## Goal

Game side of OPS-GM-01: every staff action is an off-host OpenSSH-signed request,
verified against a restart-only reviewed roster and a hash-chained control log, and
commits atomically with its `STAFF_ACTION` audit event. Delivered mechanics: Mute,
Unmute, Kick, Inspect, ReportsList, ReportClose, ReportClearText (item-text reports,
REPORT-3) and a restore replay. **No value-creating staff action.** Sanctions are staff
records, not Character aggregate writes.

Children (D1045 / OWNER-E): OPS-GM-AUDIT-1 (first), SANCTION-2, REPORT-3, ROSTER-4 (EXT),
STAFF-5 (staff panel wire), PREPORT-6 (player reports). D1045 gave each its own PR and
migration; the draft (D1437) bundled AUDIT-1 + minimal SANCTION-2/REPORT-3 writers
because WRITE-MOD-1 was not on main. CP decides the split at allocation; ROSTER-4,
STAFF-5 and PREPORT-6 stay out of this card.

## Dependencies (re-verified against origin/main 59785d76)

| Dependency | State |
|---|---|
| LOGOUT-WIRE-1 (fenced logout; kick uses it) | MERGED (#2127) |
| `crates/ntp-clock` (`read_synchronized`) | on main |
| WRITE-MOD-1 (moderation writers) | not on main; this task owns minimal writers (D1437) unless CP allocates WRITE-MOD-1 first |
| WRITE-WIRE-1 (player-facing report intent) | not this task; only the storage writer exists here |
| ARCH-CONTROL-ANCHOR-0 (off-host signed head) | external; EXT phase only |
| DATA-RESTORE-OPS-2 (full restore sequence, `restore resolve`) | follow-up; ambiguous links stop replay until it lands |
| OPS-GM-P1 (Platform) | external; EXT phase only |
| Canonical PG mount order (D1353/D1354): #2181 → #2166 → #2187 | #2187 MERGED; #2166 consolidation (not in this batch) — mount additively |
| D1447-style owner bootstrap (`game_staff_rosters` rows per activation) | owner action after merge; moderation fails closed until done |

## Accepted decisions

- **D1040 / D1029 / D1448b:** canonical bytes are JCS (closed typed object with every
  parameter, environment, roster digest, `issued_at` as uint64 ms decimal string,
  `control_seq`, previous-link digest). Upstream `ssh-keygen -Y sign/verify`; namespaces
  `oteryn-ops-request-v1` and `oteryn-ops-submission-receipt-v1` (D1437's `oteryn-staff`
  superseded). Verify before decoding, then reject any non-identical re-encoding.
- **D1041:** restart-only roster config pinned to the allowed-signers SHA-256 and newest
  activation digest; submitted previous-link checked against the DB head; no runtime
  activation in ALPHA (ROSTER-4 is EXT); retained revisions support history checks.
- **D1042:** trusted time from `ntp-clock::read_synchronized` (`None` fails closed); first
  execution window 10 min, future skew ≤5 min; identical bytes return the stored outcome
  at any age, different bytes under one operation id refuse; retry never re-executes.
- **D1044 / D1045 / OWNER-E:** SANCTION-2 owns its session-generation fences; owner holds GM
  signing keys and the initial roster off host.
- **D1085–D1088:** AUDIT-1 allocation; proposed schema = immutable links, operations/outcomes,
  signed receipts, ANL outbox, same-transaction constraints, payload ceilings, finite expiry.
- **D1087:** shared JCS crate (`crates/release-manifest`, JCS only, upstream
  `serde_json_canonicalizer = 0.4.1`, RFC 8785 vectors); SEC-REL-1 later owns release schemas.
- **D1091:** `workspace-boundaries.toml` gets only the new member and the
  server → release-manifest edge.
- **D1160/D1161:** only own module registration lines and test forwarding lines in shared
  files (CP note 2026-10-19 covers the four wrapper re-export lines).
- **D1188 / D1343(a–e):** `STAFF_ACTION` event type id 5, schema revision 1, 365-day audit
  retention (`STAFF_ACTION_AUDIT_RETENTION_V1`); proto `docs/contracts/game-events/v1/staff_action.proto`;
  restart-only optional `[staff]` config; the UID-0 control socket keeps its 30 s deadline and
  64-byte bootstrap route, staff framing bounds prefix and payload; staff PG cases use public
  production types and `Contract::registered()`; wire bounds stay CANDIDATE until max/overflow
  tests run. Re-verify id 5 is still free at allocation.
- **D1328:** generated Cargo.lock, pins and registries have no exclusive slot; regenerate after merge-up.
- **D1374:** reverse receipt→link constraint (same environment, control sequence, RECEIPT kind,
  exact signed bytes/principal/fingerprint/signature, same transaction; `23514`). Accepted residual
  (ARCH_LIVE_READINESS §1 ruling 2): misattributed receipt/link written directly with runtime
  credentials in one transaction is caught by the Rust writer and `audit verify`, not by SQL.
- **D1393:** ordinary requests require a distinct WorldId and ChannelId (`world_id == channel_id`
  refuses); only typed `RosterActivate` has `None` scope.
- **D1412:** the pending-receipt fence releases only for a receipt that matches its RECEIPT link byte
  for byte, verifies under the activation in effect at its own `control_seq` (`History::receipt_at`),
  and binds the stored operation. Trust input: `[staff] activation_history_file` + `[staff.trust]`,
  read once at boot; nothing read from the DB for trust.
- **D1437:** JCS stays; roster carries exactly one `# oteryn-staff-role <principal> GM|SENIOR_GM|ADMIN`
  line per signer (boot fails closed on missing/duplicate/unknown). Tiers: GM = Inspect, ReportsList,
  Kick, Mute, Unmute, ReportClose; SENIOR_GM adds ReportClearText; ADMIN all.
- **CP 2026-10-19:** roster provisioning accepted (owner-only `game_staff_rosters` rows); alpha kick =
  D1437 fenced logout answering IN_FIGHT; no protocol change in this task.

## Reference behaviour from the draft (re-implement, do not copy)

- **Signature/chain:** bounded raw OpenSSH verification before typed decode; pinned allowed-signers
  bytes; streaming retained-chain verification from the earliest independently retained activation;
  closed action/reason/result taxonomy of ruling 11 reused in full (no fallback string, no opaque JSON).
- **Executor** (`control_loop` → `ControlInput::Staff` → `serve_staff_control`): verify, check node
  World/Channel scope, role tier and mute bound; action + sanction/report rows + operation row + outbox
  row commit in one transaction; audit failure rolls back everything, no live effect. Trusted time
  sampled after all awaited prerequisites; exact replay does not consult the clock.
- **Schema guards (all from review findings, keep as requirements):**
  - member LOGIN guard on activations (session and effective, incl. indirect membership; SECURITY INVOKER);
  - `game_staff_head_successor`: head moves only to the exact successor link written in the same
    transaction; only ACTIVATION changes `activation_sha256`; only a no-head activation at seq 1 with zero
    predecessor bootstraps; later activations lock the current head;
  - activation refused while a receipt-requiring operation lacks its receipt (`23514`);
  - head link re-verified under current trust (digest, roster, kind, signature, activation, operation row
    via `verify_stored_operation`, outbox via `verify_stored_event`) before any extension;
  - only a COMMITTED receipt of a COMMITTED mutation releases the fence; `requires_receipt` derived from
    the signed action and outcome; refused mutations await no receipt;
  - `game_character_sanction_bound` (deferred): sanction row names a same-transaction COMMITTED/OK signed
    MUTE with same Character, duration, start and signer; a lift names a signed UNMUTE at `lifted_at`
    (`lifted_at > 0`, not `>= starts_at`, so a behind-clock unmute lifts);
  - `game_item_text_report_bound` (deferred): state change or snapshot clear names its same-transaction
    signed REPORT_CLOSE/REPORT_CLEAR_TEXT; snapshot clear refused while `expires_at` is ahead of the DB clock;
  - `game_staff_signer_allows(environment, principal, roster, kind)` enforces signer tier in both bound
    triggers from the owner-only immutable `game_staff_rosters` table;
  - INSERT/UPDATE on sanctions only for the executor runtime role; control SELECT only (column SELECT on
    reports without `snapshot_text`); unknown/duplicate envelope fields rejected before prost decode.
- **Mute/Unmute:** `game_character_sanctions`; `StaffDesk` refuses chat after commit; chat admission reads
  `staff_active_mute_until` (environment-scoped). A mute also holds spell casts of sessions without chat
  state (desk fallback; `staff_presence` loads the durable mute). Cross-node: 5 s `staff_mute_refresh_loop`
  in statements of ≤256 Characters, per-Character sequences prevent stale overwrite, disconnected mutes
  stay cached until they end or lift.
- **Kick:** reaches the connection registered on the desk; runs the real fenced logout release — accepted
  → logout; in-fight/busy → keeps playing (IN_FIGHT); unknown → disconnect. Replay never re-kicks.
- **Committed replay** re-syncs the desk from DB truth (mute = latest ACTIVE end); refused replay applies nothing.
- **Reports (REPORT-3, WRITE0 §6):** item-text reports admitted with dedup, then hourly (10), account (20)
  and World capacity limits, then current-text check; locks item KEY SHARE before the text row (text
  writers lock item, revision, text). ReportsList pages by `(entry_seq, report_id)` with a signed cursor,
  fresh `entry_seq` on each insert/state change under the worlds row lock, answer byte bound (snapshot ≤7,996
  bytes so every page holds one report). Close ends at `GREATEST(now, created_at)`. Hourly expiry loops
  `REPORT_EXPIRY_BATCH` until empty, then clears snapshots in ≤1,024-row statements. Map-object reports out of scope.
- **Audit expiry:** SECURITY DEFINER `game_staff_expire_action_audit(p_batch)` (batch 1..64, DB clock,
  deletes only after `expires_at + 900000` = execution window + future skew, never an envelope awaiting its
  receipt); operations, links and receipts never deleted; runtime has EXECUTE only; called from the node
  audit expiry loop.
- **Restore replay** (`oteryn-game-ops staff restore-replay`, own login, node stopped; ALPHA-OPS ruling 8):
  verify restored chain against retained `staff-link-*` files (gap/fork/changed link → apply nothing);
  verify request evidence for every link below the head; preflight that each receipt binds environment,
  roster, principal and key; re-run COMMITTED receipted MUTEs once with their own checks (no freshness;
  failing check → `RefusedNow`); store REFUSED receipts; a mutation without a verifying receipt or a link
  without its outcome is `Ambiguous` (any leftover above the extended tip too) and applies nothing; other
  mutations recorded, not reapplied, in immutable `game_staff_restore_unapplied` (`NOT_REAPPLIED`/`REFUSED_NOW`);
  apply in batches of 16, each checking the previous head, a batch ends only where every mutation has its
  receipt; idempotent retry.
- **Verification:** `staff audit-verify` and restore preflight compare every sanction and report row with its
  signed operation and the reverse (`staff_moderation_has_unbound_rows`: each COMMITTED OK MUTE/UNMUTE/
  REPORT_CLOSE/REPORT_CLEAR_TEXT has exactly one effect or provenance row); missing event excused only after
  the signed request's latest execution time plus retention.
- **Ops executable:** prepare signed bytes, submit detached signatures, prepare/store receipts
  (`prepare-receipt` refuses reads), audit-verify, `historical-reconcile` (read-only); never accepts or
  generates private keys. Audit build id accepts 1-128 visible ASCII bytes (`oteryn-game-server/<v>` has `/`).

### Qualification contract (18 boundary rows, all must have tests)

Identity (missing signature, unknown principal, wrong namespace); binding (environment, roster, typed
parameter); provenance (unknown/duplicate/missing/noncanonical fields); temporal window; identical expired
retry returns stored outcome; changed bytes refuse; stale sequence/predecessor/roster; two writers on one
predecessor → one typed loser; historical gap/fork/unrecorded activation; removed key; event mirror binding;
receipt digest/namespace/disposition; restore rows 13–15; read audit commit failure exposes no data;
mutation audit failure rolls back; expired audit vs missing retained evidence.

## Open owner questions (carried from the draft)

1. **In-fight kick.** An immediate StaffKick ignoring the combat lock needs `TerminalRelease::StaffKick`
   and wire code 1201 (A7) — a protocol change. (a) keep fenced-logout kick with IN_FIGHT for alpha;
   (b) follow-up packet with protocol review. Recommendation: (a) now, (b) before external players.
2. **Restore of a tail MUTE then UNMUTE.** Ruling 8 never replays an unmute; the restored mute stays until a
   fresh signed unmute of its new sanction id (recorded `NOT_REAPPLIED`). (a) keep ruling 8; (b) amend to
   replay the tail unmute (contract change). Recommendation: (a).

## Target layout (ADR-0022)

ADR-0022 does not place ops/staff code; confirm placement with CP at allocation. Proposal:
- Signed-request codec, roster, chain verifier, audit envelope: `apps/game-server/src/staff/` (new; ops, not
  gameplay) — `signature.rs`, `audit.rs`, `roster.rs`, one mechanic per file, tests in child files.
- JCS: `crates/release-manifest/` (new crate, JCS only); `workspace-boundaries.toml` (member + edge only).
- Durability: `crates/gameplay/durability/src/durability/staff_action.rs` (new),
  `staff_moderation.rs` (new), `staff_restore.rs` (new), `staff_reports.rs` (new); `durability/mod.rs` wiring-only.
- Live desk (mute cache, refresh, kick fan-out): `crates/gameplay/social/src/chat/staff_mute.rs` (new) and
  `crates/gameplay/dispatch/src/gameplay_transport/staff_desk.rs` (new); hunks in
  `gameplay_transport/chat_intent.rs` (1,635 lines), `connection.rs` (8,261 lines), `logout.rs` call-site only.
- Node/ops: `apps/game-server/src/node/serve.rs`, `apps/game-server/src/node/config.rs` (1,484 lines),
  `apps/game-server/src/bin/oteryn-game-ops.rs` (1,836 lines) — wiring/config hunks; command bodies in new modules.
- Contracts: `docs/contracts/game-events/v1/staff_action.proto` (new), `GAME_EVENT_FOUNDATION_REGISTRY.json`,
  `RESOURCE_LIMITS_REGISTRY.json`.
- Tests: `apps/game-server/tests/staff_audit_postgres.rs` (new, `test = false` focused wrapper),
  `apps/game-server/tests/support/staff_audit_postgres_cases.rs` (new), two-line mount in
  `apps/game-server/tests/durability_postgres.rs`.
- Migrations: next free number at allocation time (only one migration PR in the queue). The draft's 0133/0134
  are not leases.

## Acceptance criteria

- [ ] Every staff action emits an audited signed `STAFF_ACTION` event in the same transaction as its effect.
- [ ] Sanctions are staff records, not Character aggregate writes; no value-creating staff action.
- [ ] All schema guards above enforced against real runtime/control member LOGINs.
- [ ] Mute holds chat and casts across restart, reconnect and nodes; kick uses the fenced logout.
- [ ] Restore replay follows ALPHA-OPS ruling 8 and stays verifiable.
- [ ] All 18 qualification rows covered; PG cases run in the `durability_postgres` job (PR and merge-group harness).
- [ ] Every concurrent transition serialized (named lock or deterministic order) and the losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Structure: one mechanic per module file in its ADR-0022 layer.
- [ ] Structure: new tests live in their own child files, not appended to existing files.
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines (`connection.rs`, `chat_intent.rs`, `serve.rs`, `config.rs`, `oteryn-game-ops.rs`).

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p oteryn-game-server staff
cargo test -p oteryn-game-server --bin oteryn-game-ops staff
cargo test -p oteryn-gameplay-dispatch staff chat_intent logout
OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test -p oteryn-game-server --test durability_postgres staff_pg_
python3 tools/agents/validate_governance.py
```
