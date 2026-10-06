# ARCH-LIVE-READINESS-0: before external players (GM and support, threat model and live operations, economy incidents and privacy)

- Decision id: ARCH-LIVE-READINESS-0.
- Status: the rulings of §1–§3 and the packets are accepted on merge. The owner ruled on
  2026-10-06 (item 3b) that the "before external players" package is authored now, in parallel
  with the alpha package. Every contract amendment below is exact text marked pending: it is
  applied by the named packet, because the owning file is a candidate, because the control plane
  leases registry and event numbers, because protocol review must register a wire code, or
  because the target is an accepted cross-repository contract
  (`CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md`) that needs control-plane and Platform review.
  Platform proposals (§1 P1, §2 P1, §3 PLATFORM-PRIVACY-PROP-1 and PLATFORM-ECON-PROP-1) are
  routed by the control plane and bind nothing in Game until Platform accepts. Owner rulings of
  2026-10-06 on all eleven §4.2 items are recorded in each section's owner questions and in §4.2,
  and the body applies them. This remains a proposed architecture decision until merge; it grants
  no runtime or production authority.
- Origin: owner request (2026-10-06): review what exists and decide what still has to be fixed
  architecturally, because the owner had to point out errors by hand.
- Gaps: OPS-GM-01 (§23) in §1; PROD-LIVEOPS-01 (§22) and the system threat model (§27, §29) in
  §2; economy incident response and DATA-PRIVACY-01 (§28) in §3.
- Owning contracts: `docs/architecture/reviews/OTERYN_GAME_ARCH_ERROR_CODES_2026-10-05.md`
  (ERR-CODES; §1.9 release display), `FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md` and
  `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json` (wire codes 1200 and 1201, `STAFF_V1`,
  `SERVICE_NOTICE_V1`), `docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json` (event type
  `STAFF_ACTION`), ANL-01, ANL-03, DUR-03 (remediation and retirement causes), FND-04 and
  FND-04A (admission binding, Platform veto), OPS-NODE-BOOT-01 (configuration and secrets),
  CHAT-0, WRITE-0, SEC-CLIENT-01 and `CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md` (account erasure).
- Related: ARCH-ALPHA-OPS-0 (alpha operability). §3 ruling 18's erasure journal lives in its §3
  restore fence directory and is re-applied by its §3 ruling 9 step 7; node diagnostic logs use
  its §1 retention; §2 THREAT-DOS-1 uses its §2 measurement. ARCH-I18N-A11Y-CREATIVE-0 amendment 3
  replaces the localization item of ERR-CODES §1.9; §2 amendment A3 replaces the release-display
  item of the same section; the two edits touch different sentences.

## Implementation brief

1. No staff action creates value, ever. Every value correction is a §3 remediation record under
   the two-person rule; staff have no raw SQL (§1 ruling 1).
2. Alpha staff work is operator-only: `oteryn-game-ops staff …` and `reports …` on the node host,
   request file then control socket, idempotent by `operation_id`. Each request carries a
   detached signature by the operator's personal OpenSSH key, made off the host and verified
   against a reviewed allowed-signers roster before anything is written (§1 rulings 2–3; §2
   ruling 4). Signed revokes, mutes and fence raises survive a restore by re-apply (§1 ruling 2).
3. Kick is a typed terminal release `TerminalRelease::StaffKick`; the client gets protocol code
   1201 SESSION_STAFF_DISCONNECT (TERMINAL) and never resumes; the first terminal commit wins
   (§1 ruling 5).
4. Mute is a `game_character_sanctions` row, never a Character aggregate write; chat answers
   `MUTED {seconds}` from the later of auto-mute and staff mute (§1 ruling 6).
5. Account bans are Platform's; Game ends the live session by kick (alpha) or by Platform's
   typed `AccountSessionsTerminationRequestedV1` (proposal P1, EXT), Platform-signed and verified
   before any kick (§1 ruling 7).
6. EXT staff authority is a Game roster `game_staff_roles`, bound at admission and re-read by
   every staff command; MODERATOR and GAMEMASTER tiers; in-game commands use `STAFF_V1`, never
   chat text. A staff character is value-isolated for good: no item, gold or dust crosses its
   holdings in either direction, checked in the item fence, the bank fence and Inbox delivery
   after the root lock (§1 rulings 4, 8–10).
7. Every staff, operator and Platform-security action is one `STAFF_ACTION` DURABLE_AUDIT event
   in the same transaction as its mutation; inspect commits its audit before showing data
   (§1 ruling 11).
8. Bug reports go to GitHub Issues through a player bug-report issue form, exploits through
   GitHub private vulnerability reporting. EXT in-game reports are a typed `PLAYER_REPORT` intent
   with a closed reason and no free text; a harassment report may snapshot one chat line the
   reporter received, copied by the server (§1 ruling 12).
9. One system threat model, `docs/architecture/OTERYN_GAME_SYSTEM_THREAT_MODEL.md`, STRIDE per
   boundary B1–B8; any PR that changes a trust boundary updates it (§2 rulings 1–3).
10. Release builds keep showing the public wire code, subject to ERR-PUBLIC-REVIEW-1 before the
    first external player (§2 ruling 5).
11. Every node setting is restart-only; no reload signal, no flag service; the only kill switch
    is `[capabilities] withheld` (§2 rulings 6–8).
12. Maintenance is an announced graceful stop ending in protocol code 1200
    SESSION_SERVICE_MAINTENANCE (TERMINAL); a durable maintenance marker read at boot keeps a
    restarted node not ready; reopen is a new launch (§2 rulings 9–11, 15).
13. Secrets stay files read once; rotation is a node replacement (§2 rulings 12–14).
14. Economy incidents start as an ANL-03 case; containment is a typed
    `EconomyContainmentFence`. Every value transaction holds the fence root row FOR SHARE and a
    raise holds it FOR UPDATE, so no covered write commits after a raise; fail closed, never
    touching sessions; a raise is one signed request; a lift needs two `ECONOMY_REMEDIATOR`
    signatures, as a remediation approval does (§3 rulings 1–4).
15. Remediation is a compensating DUR-03 transaction under `IntegrityRemediationCause`, recorded
    in `game_economy_remediation_records` with a plan-hash-bound approval signed by two distinct
    `ECONOMY_REMEDIATOR` holders with personal keys; with fewer than two, remediation fails
    closed; an economy bug is never repaired by a restore (§3 rulings 7–13).
16. `docs/contracts/GAME_PERSONAL_DATA_INVENTORY.json` lists every personal-data store with a
    privacy class from the closed ANL-01 §15 set and an erasure action from a closed set, and is
    checked against migrations (§3 ruling 14).
17. Platform holds account PII and is the entry point for data subject requests; account erasure
    is a typed, idempotent, Platform-signed Platform→Game request, with one active erasure per
    AccountId; only the erasure handler's own database role, outside the runtime and
    control-plane groups, can start, retire or substitute an erasure; Game retires value and tombstones
    the roots, then one guarded transaction substitutes a random account deletion marker or
    deletes, in a fixed per-table order, every row keyed by the AccountId, changing no value
    column or chain link, proves by a catalog scan that none is left, and writes a denial
    digest that refuses every later fact for that AccountId (§3 rulings 15–17).
18. Erasures are journalled, by the signed Platform request only, in the restore fence directory:
    a durable `PENDING` record before the first database transition and `COMPLETE` before the
    Platform acknowledgement. A request the database would refuse is never journaled. After any
    restore, every journaled request is re-applied, which also restores its denial digest, once
    the journal is checked against Platform's signed erasure head; a gap, an unverifiable record
    or a torn or unreadable journal stops the restore until Platform's copy of the request is
    appended. The journal is itself an inventory row (§3 rulings 14 and 18; ARCH-ALPHA-OPS-0 §3
    ruling 9 step 7).
19. Packet order: OPS-GM-AUDIT-1 (with the request signature module), THREAT-MODEL-0,
    ECON-FENCE-1 (after that module) and PRIVACY-INVENTORY-1 first;
    OPS-GM-SANCTION-2 after LOGOUT-WIRE-1; LIVEOPS-MAINT-1 after protocol review of 1200;
    PRIVACY-ERASURE-1 after PRIVACY-INVENTORY-1, GUILD-ERASURE-0 (the guild-leader transition,
    §3 U3), PLATFORM-PRIVACY-PROP-1 (the signed request) and DATA-RESTORE-OPS-2.
20. Numbers leased by the control plane at packet time: event type 4 `STAFF_ACTION`, the
    `STAFF_V1` and `SERVICE_NOTICE_V1` capability and domain ids, the new 6xxx ops codes and
    the `ACCOUNT_ERASED` SQLSTATE.
    Protocol codes 1200 and 1201 are fixed here and registered through protocol review.

## 1. GM, moderation, staff audit and player support

Gap: OPS-GM-01. Scope: the alpha minimum, and the "before external players" package (owner ruling 2026-10-06).

### Facts

| # | Fact | Locator | Class |
|---|---|---|---|
| F1 | OPS-GM-01 is REGISTERED_UNRESOLVED. The open items are role boundaries; mute, ban, kick, teleport, inspect and recovery; audited corrections; impersonation and account access; case management, appeals and review; dual control; immutable admin audit; and emergency rollback. | docs/architecture/ARCHITECTURE_ANALYSIS_GAP_REGISTER.md §23 | PROVEN |
| F2 | OPS-GM-01 is REQUIRED_FOR_ALPHA operational completeness. It must keep "no hidden unaudited GM mutation path" and "stale administrators cannot bypass session, item or revision fences". | docs/architecture/GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md (OPS-GM-01, line 554, line 684); FOUNDATION_DECISION_BACKLOG.md lines 578, 644, 661 | PROVEN |
| F3 | No staff authority exists in Game. The ops binary has issue/reconcile/revoke and assignment commands only. Migration 0043 pins group_id=1 and account_type=1 and says privileged Control authority is separate. | apps/game-server/src/bin/oteryn-game-ops.rs; apps/game-server/migrations/0043_spell_familiar_group.sql | PROVEN |
| F4 | The node has a uid-0 Unix control socket. It serves one bounded request at a time, and today only bootstrap by operation id. | apps/game-server/src/node/serve.rs:984-1024 | PROVEN |
| F5 | Platform defines no staff roles. Its role is account authority, UX/commercial orchestration and read projections. | docs/contracts/CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md:359 | PROVEN |
| F6 | The pre-admission grant has an exact claim set, and an unknown claim is rejected. A staff claim would need a new grant profile version, owned jointly with Platform. | docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md §5, §5.2 | PROVEN |
| F7 | Platform security evidence (account disabled, generation floor, freshness of 5 s or less) is a pre-admission veto only. Platform has no post-admission GameSession authority. Ending a live session needs a game-domain control contract. | FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md §10 (line 228); docs/architecture/FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md §9.4; FND-04_PLATFORM_PRE_ADMISSION_RECONCILIATION_REFINEMENT.md §3.4 | PROVEN |
| F8 | A recovery grant binds account_security_generation. A disabled or revoked account gets RECOVERY_GRANT_SECURITY_STATE_REVOKED. | docs/contracts/FND-04_REAUTHENTICATED_RECOVERY_GRANT_PROFILE_V1.md lines 140, 241, 265, 390 | PROVEN |
| F9 | So a Platform ban blocks fresh login and resume, but it does not end a live session. | F7, F8 | DERIVED |
| F10 | OPS-REVOKE-REPORT-1 reports channel scope revocation to Platform. It does not cover account bans or session termination. | docs/architecture/reviews/OTERYN_GAME_ARCH_REVOKE_REPORT_2026-10-05.md | DERIVED |
| F11 | ADR-0003 lets the game server do final checks "such as … ban/disabled policy where contracted". | docs/architecture/ADR-0003-platform-identity-game-gateway-and-admission-boundary.md §6 (line 106) | PROVEN |
| F12 | Support and admin tools must use audited domain commands, never raw SQL. | docs/architecture/ADR-0012-character-authority-and-platform-lifecycle-boundary.md:264 | PROVEN |
| F13 | A correction is a new compensating transaction with a new TransactionId and causation. Mint, burn and transform need a typed authorized cause. | docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md §2 item 7, §26 | PROVEN (CANDIDATE contract) |
| F14 | Detectors never sanction. Enforcement and GM policy are outside ANL-03. | docs/architecture/ANL-03_ECONOMY_INTEGRITY_SECURITY_ANALYTICS_CONTRACT_CANDIDATE.md lines 30, 213, 291, 297 | PROVEN |
| F15 | ANL-01: DURABLE_AUDIT is atomic with the mutation. Event types register owner, schema, durability, privacy floor, a finite retention profile and atomic evidence. Investigation tools cannot mutate or sanction. | docs/architecture/ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md §10, §12, §15-§17 | PROVEN |
| F16 | Registered event types are 1-3, so the next free id is 4. Audit retention today is P90D (character, one-item) and P30D (economy, market, guild). | docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json | PROVEN |
| F17 | SEC-CLIENT-01 retention precedents: a reviewer audit is kept 1 year (RL-15), and identity/access audit 2 years (RL-16). Reviewer identity is staff data. In alpha there are no ban waves. Before open beta, signals and player reports open cases in "the OPS-GM-01 queue". | docs/architecture/reviews/OTERYN_GAME_SEC_CLIENT01_CLIENT_INTEGRITY_AND_ANTI_BOT_DECISION_2026-10-01.md §5.1, §6 | PROVEN (CANDIDATE) |
| F18 | WRITE-0 has a text report store readable by the moderation role, with states OPEN → CLOSED or EXPIRED by conditional update, kept 180 days (RL-03). Text removal is keyed by a moderation action id. Close/clear authority and any result beyond OK/STALE are left to "the GM tools decision". | docs/architecture/reviews/OTERYN_GAME_WRITE0_BOOKS_SCROLLS_AND_BLACKBOARDS_DECISION_2026-10-01.md §6 | PROVEN (CANDIDATE) |
| F19 | CHAT-0 keeps an auto-mute row `game_character_chat_mutes`, written under the session fence. It is read at admission, reconnect and channel transfer, and the answer is `MUTED {seconds}`. Chat text is not stored. | docs/architecture/reviews/OTERYN_GAME_CHAT0_PLAYER_CHAT_DECISION_2026-09-30.md §6, §7; apps/game-server/src/chat/ | PROVEN |
| F20 | A second client cannot kick a healthy incumbent that is in combat, in a PZ or logout-locked. No logout command exists yet. LOGOUT-WIRE-1 adds `TerminalRelease::Logout` with the settle handshake and `session_state = 3`. | FND-ID-01_ACCOUNT_SINGLE_ONLINE_CHARACTER_OWNER_BASELINE.md:224; reviews/OTERYN_GAME_ARCH_KILL_REWARD_LOGOUT_PACKETS_2026-10-04.md; apps/game-server/src/gameplay_transport/mod.rs:794 | PROVEN |
| F21 | The F12 report line has no player-linked ids and no free text, and nothing is uploaded. Crash privacy: an allowlist, no private chat, and opting out is not suspicion evidence. | reviews/OTERYN_GAME_ARCH_ERROR_CODES_2026-10-05.md §1.10 item 3; CLIENT_CRASH_DIAGNOSTICS_PRIVACY_OWNER_BASELINE.md | PROVEN |
| F22 | §2 ruling 4 allows no GM path on the wire until OPS-GM-STAFF-5, and admin actions go through `oteryn-game-ops` with a retained request file. §3 has a remediation record with a two-person rule, and §3 owner ruling Q1 b names the approver role. | this document §2 ruling 4; §3 rulings 7, 8, 11 | DERIVED |
| F23 | Ops error codes 6001-6006 are taken. Protocol wire codes 1100-1116 are admission and 1117 is taken by ARCH-ALPHA-OPS-0 §4; §2 takes 1200 for maintenance. | docs/contracts/OTERYN_GAME_ERROR_CODE_REGISTRY.json; docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json; §2 amendment A1 | PROVEN |
| F24 | Content has a teleporter item. A generic in-session relocation primitive was not confirmed. | apps/game-server/src/content/encounter_map_item.rs:912 | UNKNOWN |

### Rulings

Normative. Phases: **ALPHA** (the minimum for alpha operational completeness) and **EXT** (required before external players).

1. **No value-creating staff action, in either phase.** Item, gold or dust grants, character stat or skill edits, position edits on offline characters, and "compensation" are forbidden as staff actions. Every value correction is a §3 remediation record: a DUR-03 §26 compensating transaction with a typed cause and the two-person rule (F13, F22). Staff have no raw SQL (F12). This closes "audited domain transactions for corrections" and "dual control" for OPS-GM-01. Dual control applies only to value.
2. **ALPHA: operator-only, no GM path on the wire.** All alpha staff actions run as `oteryn-game-ops staff …` and `oteryn-game-ops reports …` on the node host as root, on the §2 ruling 4 pattern (F22). The flow is: request file, then the control socket (F4), then the node. The `operation_id` is the idempotency key, and an ambiguous outcome is reconciled from the retained file. The actor is `OPERATOR`. No in-game staff role exists in alpha. Root can edit host login audit (auditd, journald), so the human is attributed by a personal signature on each request, not by host logs:
   - Signing. `oteryn-game-ops staff … --prepare` (and `reports …`) writes the canonical request bytes: one canonical encoding of the action's complete typed request. That is operation_id, action, typed target, reason code, scope (WorldId, ChannelId), every action parameter the typed request carries (for example a mute's duration, a roster grant's role tier and a legal hold's expiry), environment id, the roster SHA-256 and `issued_at`. No field of the request is outside the signed bytes. The operator signs exactly those bytes on their own device with a personal OpenSSH key (`ssh-keygen -Y sign`, upstream). The key is never on the node host, and agent forwarding is not used. `submit` sends the bytes with the detached signature; the tool sends nothing unsigned.
   - Roster. The signers are the allowed-signers roster of §3 ruling 8: one principal per person, each line limited by `namespaces=`. Namespaces: `oteryn-ops-staff@v1` (staff actions, reports, roster grant and revoke); `oteryn-ops-econ-raise@v1` (fence raise, §3 ruling 4); `oteryn-ops-econ-lift@v1`, `oteryn-ops-econ-remediation@v1` and `oteryn-ops-econ-hold@v1` (case holds, §3 ruling 5), only on `ECONOMY_REMEDIATOR` lines; `oteryn-ops-privacy@v1` (privacy export and privacy legal holds, §3 rulings 19-20, only on `PRIVACY_OFFICER` lines). The roster path and SHA-256 are restart-only node and ops config keys (§2 ruling 6), logged at start and checked against the reviewed revision (§2 ruling 7).
   - Verification. The process that commits verifies with `ssh-keygen -Y verify` before any write: the node for a control-socket action, the ops tool for roster, fence and remediation commits (§3 rulings 4 and 8). The verifier rebuilds the signed bytes with its own environment id (a restart-only config key, never read from the database, so a database restored into another environment keeps that environment's id) and the SHA-256 of the reviewed roster it has loaded. It executes only the typed request decoded from the verified bytes, never a field sent beside them; an unknown or missing field, or bytes that are not the canonical encoding, refuse. A bad signature, an unknown principal, a wrong namespace, another environment id or another roster SHA-256 is refused with the new ops code `OPS_STAFF_SIGNATURE_INVALID` (6xxx, leased at packet time), and nothing is written. Every request built by `--prepare` carries both fields, and so do the economy lift and remediation tuples (§3 rulings 4 and 8).
   - Freshness and replay. A first execution needs `issued_at` within 10 minutes (CANDIDATE) of the verifier's clock. The verifier stores the request digest with the outcome. The same bytes again return the stored outcome at any age; other bytes under the same operation_id are refused. A stale request that never ran is refused, and the operator signs a new one under a new operation_id, so nothing runs twice.
   - Evidence. The `STAFF_ACTION` row stores the principal, key fingerprint, signature and roster SHA-256 (ruling 11). `oteryn-game-ops audit verify` re-verifies every OPERATOR row against the roster revision it names, and checks that every sanction, release and report transition names an action_id whose row verifies. An unsigned or failing row is an incident.
   - Restore. A database restore rolls back every row after T, so a revoke, mute, fence raise or hold placement committed after T would be lost. The signed request files of a roster revoke, a staff mute, a fence raise and a case or privacy legal hold placement (§3 rulings 5 and 20) are therefore also written, before submit, to the restore fence directory, and the submitting tool appends each one's final outcome record there. Their replay follows the one shared rule, ARCH-ALPHA-OPS-0 §3 ruling 8 "Replay after restore". `oteryn-game-ops restore` replays, before any authority opens (amendment 5 in §3), only a request with a retained `COMMITTED` outcome record and no stored outcome in the restored database. It verifies the signature against the roster revision the request names, skips the freshness window, and is idempotent by operation_id. A request with a stored outcome changes nothing, so a control unmuted, lifted or released before T is not revived. A request file with no outcome record (never submitted, or a crash before the record) stops the restore until a signed `restore resolve` applies or abandons it. Selection never compares `issued_at` or any other clock with T. A grant, unmute, lift or hold release after T is not re-applied and is signed again, so a restore never widens authority or drops evidence. A missing or unreadable directory refuses the restore; a testing-phase reseed records that these requests are lost, as for the erasure journal.
   - Residual. Root can still read the node's runtime database credential and write without the node or the tool. It cannot forge a signature, so such a write has no verifying row and `audit verify` finds it afterwards. If the database owner credential is on the host, root can also disable triggers; THREAT-MODEL-0 records where that credential lives (recommended: off the node host). Host login audit stays a secondary record only.
3. **ALPHA action set.**
   - `staff inspect --character` is a read-only snapshot of character state, position, session and active sanctions. The audit event is committed before the data is returned. If the audit write fails, nothing is shown.
   - `staff kick --character | --account` ends the live session (ruling 5).
   - `staff mute --character --for <dur> --reason <code>` and `staff unmute --sanction <id>` (ruling 6).
   - `reports list | close` and `reports clear-text` act on the WRITE-0 report store (F18).
   - Teleport and roster administration are not in alpha.
4. **EXT: Game-owned staff roster.** New table `game_staff_roles`: account_id, character_id (one designated staff character), tier, granted_at, revoked_at, revision. It is written only by `oteryn-game-ops staff grant | revoke`, each a signed request (ruling 2), and each change is audited. Rows are never deleted: revoke sets `revoked_at` by compare-and-set on the revision, and a revoke survives a restore by re-apply (ruling 2). A grant locks the character root FOR UPDATE before it inserts the row (ruling 10). It then requires the locked root to be live and its `account_id` to equal the grant's AccountId; otherwise it refuses with the new ops code `OPS_STAFF_GRANT_TARGET_MISMATCH` (6xxx, leased at packet time) and writes no roster row and no audit row, so no character of another account is ever staff-marked. The table enforces the same pairing: a composite foreign key `(character_id, account_id)` references a new unique key `(character_id, account_id)` on `game_character_roots`, DEFERRABLE INITIALLY IMMEDIATE like the other substituted keys of §3 ruling 17, so the erasure function changes both columns in one transaction.
   - The final admission transaction reads the roster row and binds `{tier, roster revision}` to the session.
   - Every staff command re-reads the row in its own transaction (`FOR SHARE`) and refuses `ROLE_REVOKED` if it is revoked or its revision changed. A stale admin acts on nothing (F2).
   - Platform is not asked for a staff claim (F5, F6; owner ruling §1 Q1 a).
   - Each environment has its own roster, because each environment has its own database.
   - Impersonation is forbidden. Staff never log in to a player's account, never receive player credentials and act only through their own staff character. Account access is Platform's.
5. **Kick is an immediate, typed terminal release.** It adds `TerminalRelease::StaffKick(StaffActionRef)` to the release enum from LOGOUT-WIRE-1, with the same settle handshake and `session_state = 3`.
   - It ignores the logout lock, because a player cannot trigger it. That is not an escape path.
   - The client gets the new protocol wire code 1201 SESSION_STAFF_DISCONNECT (amendment A7), progression TERMINAL, so it does not resume.
   - The server refuses a resume because the session is released.
   - It serializes with the player's own logout, channel transfer or control-loss expiry on the session's single terminal transition. The first commit wins, and a late kick returns `ALREADY_RELEASED`.
6. **Game sanctions are staff records, not Character aggregate writes.** New table `game_character_sanctions`: sanction_id (UUIDv7), character_id, kind (closed enum: `MUTE` only for now), starts_at, ends_at (finite), reason_code (the closed set of ruling 11), actor, status ACTIVE or LIFTED, revision, and `basis_refs` (typed `{family: WRITE0_REPORT | PLAYER_REPORT, id}`).
   - Mute inserts a row. Unmute changes ACTIVE to LIFTED by compare-and-set on the revision.
   - Two concurrent mutes make two rows, and the latest active `ends_at` applies.
   - No session ever writes this table, so it does not contend with the CHAT-0 fenced auto-mute row (F19).
   - The chat path checks the auto-mute and the active staff mute, and answers `MUTED {seconds}` from the later of the two.
   - Live delivery: the owning node commits the row and the audit together, then updates its in-memory session. If the target is offline, the row is read at the next admission, reconnect or channel transfer.
   - Restart-safe: the rows are durable, and the in-memory copy is rebuilt from them at admission.
7. **Account bans are Platform's. Game ends the live session.** A ban sets the Platform account to disabled, which blocks fresh login and resume (F9). For a live session:
   - ALPHA: the operator runs `staff kick --account`. Platform disable plus Game kick is the ban procedure.
   - EXT: proposal P1 (routed by the control plane) is a typed, idempotent Platform→Game request `AccountSessionsTerminationRequestedV1 {operation_id, account_id, account_security_generation}`. Platform signs the request bytes, which include the Game environment id, with a Platform request-signing key; the node verifies the detached signature against the restart-only verification key file of the §2 secret table before any kick, refuses an unsigned or failing request with nothing done, and stores the signature in the `STAFF_ACTION` row. Game then kicks every session of that account with `StaffKick` and actor `PLATFORM_SECURITY`, and reports completion. This is the game-domain control contract that F7 requires. Platform gains no other post-admission authority.
8. **EXT: tiers.**
   - Two tiers, a closed enum: `MODERATOR` (1) and `GAMEMASTER` (2).
   - MODERATOR: inspect (session, position, sanctions only), mute, unmute, kick, report triage and close, and text clear.
   - GAMEMASTER: everything a moderator has, plus full inspect, teleport self to a position or a character, and teleport a target to self or to the temple.
   - The roster, configuration and maintenance stay operator-only. TUTOR is not created.
9. **EXT: in-game staff commands use a typed capability, not chat text.**
   - A new capability `STAFF_V1` with one command type `STAFF_COMMAND`. Each command has a closed kind enum and typed targets.
   - It is offered only to a session whose admission bound a roster row. Ordinary and older clients never see it, so no gating change is needed.
   - Results: OK, ROLE_REVOKED, TIER_INSUFFICIENT, TARGET_NOT_FOUND, TARGET_NOT_IN_CHANNEL, STALE, ALREADY_RELEASED and RATE_LIMITED.
   - In-game commands act only on targets in the staff session's own (WorldId, ChannelId). The node that owns the target's session does the work, so the target's session-generation fence applies.
   - Cross-channel actions use the ops tool against the owning node. A cross-channel teleport is forbidden; the only cross-channel move is the GAME-CHANNEL-01 §12 evacuation.
   - Teleport is a server-authoritative position change in the target's session loop (F24).
10. **EXT: staff value isolation, a holder boundary enforced from OPS-GM-ROSTER-4.** A check at intent dispatch alone is not enough: value also moves by drop, pickup, loot, bank and Inbox paths that are not trade intents.
    - Staff-marked. A character is staff-marked when any `game_staff_roles` row names it, revoked or not. Rows are never deleted (ruling 4), so the mark is permanent.
    - Boundary. A staff-marked character's own holdings (equipment, the backpack tree, its Inbox and its forge dust) are closed: no item, gold or dust crosses them in either direction. The (Account, World) bank is outside the boundary. Allowed: moves inside the holdings, NPC trade, fee burns, and system grants that land inside (a reward claim, a spell item that lands in the own backpack).
    - Item paths. `character_item_fence_is_current` (`apps/game-server/src/durability/item_transfer.rs:1044`) is the one item fence; it locks the character root FOR UPDATE (`:1151`). It gains a required argument `ItemBoundary::{Internal, Crossing}` with no default, so every caller must classify itself. For a staff-marked character, `Crossing` returns `STAFF_VALUE_ISOLATED` and the caller writes nothing. Internal callers today: equipment (`character_equipment.rs:225`), reward claim (`reward_claim_mint.rs:854`), and a spell item that lands in the backpack (`spell_item_transaction.rs:1122`). Crossing callers today: Ground pickup and corpse loot (`item_transfer.rs:1181`), map item pickup (`map_item_mint.rs:695`), and a spell item that overflows to the Ground (`ItemGrantOverflow::DropOnCasterTile`, `apps/game-server/src/spell/native_items.rs:88`, through the same `:1122` call). Drop and throw (ITEM-MOVE-2b; `ItemTransferDestination` has no Ground variant yet, `item_transfer.rs:164-169`) are Crossing when they land.
    - Bank. `acting_fence` (`apps/game-server/src/durability/bank.rs:614`) runs with the acting root FOR UPDATE (`lock_acting_root`, `:716`). It refuses a staff-marked actor with `STAFF_VALUE_ISOLATED` for `bank_deposit`, `bank_withdraw` and `bank_transfer` (`:405`, `:417`, `:429`). A transfer also refuses a staff-marked recipient, read after the recipient root FOR SHARE (`:1714-1729`).
    - Inbox. `game_character_inbox_deliver` (`apps/game-server/migrations/0076_character_inbox.sql:215`), the only delivery entry for HOUSE-1b and MARKET-1 (`character_inbox.rs:4-8`), refuses a staff-marked recipient with the new SQLSTATE `OTI06` after its recipient root lock FOR KEY SHARE (`:236`); `OTI01`-`OTI05` are taken (`:209-213`).
    - Rewards, death and party. A kill whose reward principal is staff-marked mints no loot and grants no experience or bestiary progress (`settle_creature_death_rewards` and `settle_creature_death_rewards_with_bestiary`, `apps/game-server/src/combat/death_reward.rs:641, 746`). DEATH-3 moves nothing out of a staff-marked character (`character_death.rs:68`). A party refuses a staff-marked member (`world_party.rs`, migration 0046), and reward settlement skips a member that was marked after joining, so no shared reward reaches one.
    - Future writers. Trade, mail send, market, guild bank and house bid packets call the same boundary check. None has a writer on main today (grep).
    - Ordering. Every check reads the roster in a later statement after its root lock (the §3 ruling 2 pattern). A grant locks the root FOR UPDATE before it inserts the roster row. FOR UPDATE conflicts with the writers' FOR UPDATE, FOR SHARE and FOR KEY SHARE, so only two orders exist: a writer that locked first commits first, and its move is a fact from before the grant; a writer that locks after the grant sees the row and is refused. The ROSTER-4 migration gives the control-plane role UPDATE on one inert root column, which PostgreSQL needs for FOR UPDATE; the root guards still refuse any real change.
    - A grant on an online character applies from that character's next writer transaction; no session state changes. The OPS-GM-STAFF-5 dispatch check is only an early refusal.
11. **Staff audit is a single ANL-01 event type, `STAFF_ACTION`** (amendment A1).
    - DURABLE_AUDIT. The privacy floor is RESTRICTED_PLAYER_LINKED.
    - For a mutation, it is in the same transaction as the sanction, release or report change. For inspect, it is committed before the read result is released.
    - Payload: action_id, actor kind (OPERATOR, STAFF, PLATFORM_SECURITY), the staff character and roster revision if any, action kind, typed target, reason code, result, basis refs and trace id. Closed sets: action kind `INSPECT`, `KICK`, `MUTE`, `UNMUTE`, `REPORT_CLOSE`, `REPORT_CLEAR_TEXT`, `ROSTER_GRANT`, `ROSTER_REVOKE`, `TELEPORT`, `ECON_FENCE_RAISE`, `ECON_FENCE_LIFT` (§3 ruling 4); reason code `BOT`, `HARASSMENT`, `NAME`, `CHEATING`, `BUG_ABUSE` (the ruling 12 reasons), `SUPPORT`, `PLATFORM_SECURITY`, `ROSTER`, and the economy case origins `DETECTOR_SIGNAL`, `INTEGRITY_ECODE`, `SUPPORT_REPORT` (§3 ruling 1); result the ruling 9 set. An unknown value is refused before any write. For a Platform-security action the payload also carries Platform's signature (ruling 7), and for an OPERATOR action the signer principal, key fingerprint, signature and roster SHA-256 (ruling 2). It never contains player text or chat.
    - Roster grant and revoke use the same event type.
    - Economy containment uses it too. For `ECON_FENCE_RAISE` and `ECON_FENCE_LIFT` the typed target is the fence (fence_id, fence revision and scope: `ITEM_TYPE_SET`, `MARKET`, `CHARACTER_VALUE` or `BANK` with its ids), the basis refs carry the economy case id, and the reason code is the case origin. A raise carries one signer (principal, key fingerprint, signature, roster SHA-256); a lift carries both signers of §3 ruling 4. The event commits in the same transaction as the fence row change.
    - Retention is a new profile, `STAFF_ACTION_AUDIT_RETENTION_V1`, of 1 year (owner ruling §1 Q3 a). Rows are never edited. A mistake is reversed by a new action.
12. **Reports** (owner rulings §1 Q2 a, refined, and §1 Q4 b).
    - ALPHA: text reports already exist in WRITE-0. No new Game queue in alpha.
    - Bug reports, in both phases, go to GitHub Issues only, through a player bug-report issue form. The form asks for the pasted F12 line, the client version and the steps, and for no account name, email or chat text (F21).
    - Exploits go only through GitHub private vulnerability reporting, never a public Issue. The issue chooser already links there (`.github/ISSUE_TEMPLATE/config.yml:2-5`), and `SECURITY.md` names it.
    - Private vulnerability reporting is reported enabled by the control plane; this decision has not verified it. It is a precondition that OPS-GM-REPORT-3 verifies on the live repository setting before it delivers the form; if it is not enabled, the packet stops and reports.
    - Discord is added only when external testers arrive, by a later decision.
    - EXT: in-game reports are rule-violation reports only. A new `PLAYER_REPORT` intent creates a row in `game_player_reports` with target CharacterId and a closed reason (BOT, HARASSMENT, NAME, CHEATING, BUG_ABUSE). There is no free text.
    - The server attaches world, channel, position, time and the reporter's trace.
    - It uses the WRITE-0 state machine, dedup and admission order (F18) and the restricted store. The limits reuse WRITE0-RL-04/05 values as CANDIDATE, and RL-03 sets retention to 180 days (CANDIDATE).
    - Chat evidence: the node keeps, in memory only, the last 50 chat lines (CANDIDATE) delivered to each session, keyed by the CHAT state-domain revision of the `CHAT_LINE_DELTA_V1` that carried each line (the revision is per GameSessionId and strictly increasing; `PROTOCOL_OTERYN_V1_REGISTRY.json`, state domain 12). A HARASSMENT report may carry one `chat_line_revision`. The server copies that line's text, speaker CharacterId, room and time from its own buffer into the report row; the client never supplies the text. An unknown or evicted revision refuses the report with `CHAT_LINE_UNAVAILABLE` and writes nothing.
    - The evidence is bound to the target. The buffer records each line's speaker CharacterId as the server routed it. The report is refused with `CHAT_LINE_TARGET_MISMATCH`, and nothing is written, when that speaker is not the report's target CharacterId, including a system or NPC line that has no player speaker. A `chat_line_revision` on any reason other than HARASSMENT is refused the same way. The report row stores `reason`, `target_character_id`, `chat_line_revision`, `chat_speaker_character_id` and the copied `chat_line_text`, `chat_room` and `chat_line_at`, and the table enforces `CHECK ((chat_line_revision IS NULL AND chat_line_text IS NULL AND chat_speaker_character_id IS NULL AND chat_room IS NULL AND chat_line_at IS NULL) OR (chat_line_revision IS NOT NULL AND chat_line_text IS NOT NULL AND chat_speaker_character_id IS NOT NULL AND chat_room IS NOT NULL AND chat_line_at IS NOT NULL AND chat_speaker_character_id = target_character_id AND reason = 'HARASSMENT'))`: the evidence columns are all absent or all present, and present evidence names the target, so a row whose evidence names another player, or evidence with no speaker, cannot exist. Erasure deletes the line text (§3 ruling 14 row) by clearing all five evidence columns in the same statement that substitutes `target_character_id`, so the check holds at every row state. Dedup, report counts and a sanction's `basis_refs` therefore always count against the player who spoke the line.
    - The buffer is lost at disconnect, reconnect and transfer, since lines are never replayed from storage. The snapshot shares the report's 180-day retention and never enters logs or audit.
    - SEC-CLIENT-01 signals open rows here (F17).
13. **Not a GM tool: case management, appeals and rollback.**
    - A sanction's `basis_refs` are the case link. Appeals go to the owner out of band, and the result is an audited unmute or lift.
    - World or economy rollback is never a GM action. Use §2 maintenance or the kill switch, the §3 containment fence or the ARCH-ALPHA-OPS-0 §3 restore.
    - Ban waves stay a Platform decision before open beta (F17).
14. **Before-freeze check.**
    - Serialization: release (ruling 5), sanction compare-and-set (ruling 6), report compare-and-set (F18), roster re-read (ruling 4), and a roster grant against value writers under the root lock (ruling 10).
    - Authority: every OPERATOR request is signed by a personal key and every Platform termination request by Platform, each verified before any write; root bypass is detected by `audit verify` (rulings 2 and 7).
    - Restart and restore: the rows are durable, a kick needs no resume state, and an ambiguous ops outcome is reconciled from the request file; a restore replays the signed revokes and mutes with a retained committed outcome record and no stored outcome in the restored database, and never replays a grant or unmute (ruling 2; ARCH-ALPHA-OPS-0 §3 ruling 8).
    - Typed references: `basis_refs`, `StaffActionRef` and the roster revision.
    - Older clients: `STAFF_V1` is never offered to them. An old client that gets SESSION_STAFF_DISCONNECT uses the unknown-code fallback (DERIVED).
    - Multi-component: the node commits row and audit in one transaction before any in-memory or wire effect.

### Contract amendments

- **A1. docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json, `event_types`.** Append `{"id": 4, "name": "STAFF_ACTION", "owner_gate": "OPS_GM_01_STAFF_ACTIONS", "payload_schema": "docs/contracts/game-events/v2/staff_action.proto", "payload_message": "oteryn.events.v2.StaffActionV1", "current_schema_revision": 1, "durability_class": "DURABLE_AUDIT", "privacy_class_floor": "RESTRICTED_PLAYER_LINKED", "retention_profile_id": "STAFF_ACTION_AUDIT_RETENTION_V1", "atomic_mutation_evidence": true}`. The id is 4 unless another packet takes it first; the control plane leases it.
- **A2. Same file, `retention_profiles`.** Append `STAFF_ACTION_AUDIT_RETENTION_V1`:
  - purpose "Prove every staff, operator and Platform-security action on players";
  - privacy_class RESTRICTED_PLAYER_LINKED;
  - finite_retention_duration_or_ceiling "P1Y (owner ruling 2026-10-06, §1 Q3 a)";
  - permitted_roles ["owner", "authorized security"];
  - export_redaction_policy, deletion_or_anonymization_policy and legal_hold_policy copied verbatim from CHARACTER_AUTHORITY_DURABLE_AUDIT_RETENTION_V1;
  - policy_revision 1.
- **A3. reviews/OTERYN_GAME_WRITE0_BOOKS_SCROLLS_AND_BLACKBOARDS_DECISION_2026-10-01.md §6.** Replace each "GM tools decision" with "ARCH-LIVE-READINESS-0 §1 (OPS-GM-01)". Add: "Close and clear are allowed for OPERATOR in alpha and for MODERATOR or above after that. Results: OK, STALE, ROLE_REVOKED, TIER_INSUFFICIENT. Each is a STAFF_ACTION event."
- **A4. reviews/OTERYN_GAME_CHAT0_PLAYER_CHAT_DECISION_2026-09-30.md §6.** Add: "A character is also muted while an ACTIVE staff MUTE sanction covers now (ARCH-LIVE-READINESS-0 §1 ruling 6). `MUTED {seconds}` uses the later end time." Add to the chat storage rule: "Exception: one line a player reports for harassment is copied by the server from its in-memory buffer into the restricted player report store for 180 days (ARCH-LIVE-READINESS-0 §1 ruling 12). No other chat text is stored."
- **A5. reviews/OTERYN_GAME_SEC_CLIENT01_CLIENT_INTEGRITY_AND_ANTI_BOT_DECISION_2026-10-01.md §6.** Replace "the OPS-GM-01 queue" with "`game_player_reports` (ARCH-LIVE-READINESS-0 §1 ruling 12)".
- **A6. docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json.** Add capability `STAFF_V1` and command type `STAFF_COMMAND`, with `"offered": false` and offer_gate "offered only to a session whose admission bound a game_staff_roles row; not before OPS-GM-STAFF-5". Also add intent `PLAYER_REPORT` (target, closed reason, optional `chat_line_revision`, HARASSMENT only), with the refusals `CHAT_LINE_UNAVAILABLE` and `CHAT_LINE_TARGET_MISMATCH`, not offered before OPS-GM-PREPORT-6. The control plane leases the numbers.
- **A7. Error codes.**
  - `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json`, `error_codes`, after §2's 1200 (through FND-02 registration and protocol review): `{"code": 1201, "name": "SESSION_STAFF_DISCONNECT", "category": "SESSION_REJECTED", "default_disposition": "TRANSPORT_FATAL", "progression": "TERMINAL", "public_class": "SESSION_UNAVAILABLE"}`. All values come from the registry's existing closed sets; no new public class is added. The player sees the SESSION_UNAVAILABLE text with code 1201 (§2 ruling 5), which support recognises.
  - `docs/contracts/OTERYN_GAME_ERROR_CODE_REGISTRY.json`, 6xxx block: ops codes `OPS_STAFF_NODE_UNREACHABLE`, `OPS_STAFF_TARGET_NOT_ONLINE` and `OPS_STAFF_SIGNATURE_INVALID` (ruling 2), numbered from the next free 6xxx code the control plane leases at packet time.
  - `STAFF_VALUE_ISOLATED` (ruling 10) is a domain outcome, not a wire code: the client sees the existing generic refusal, so no protocol change is needed.
- **A8. docs/architecture/ARCHITECTURE_ANALYSIS_GAP_REGISTER.md §23.** Status: "Decided for alpha and EXT by ARCH-LIVE-READINESS-0 §1. Residual: name lock, offline unstuck."
- **No change** to the FND-04 grant profiles (ruling 4).

### Packets

| Id | Owned paths | Tests | Order |
|---|---|---|---|
| OPS-GM-AUDIT-1 | docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json; docs/contracts/game-events/v2/staff_action.proto (new); apps/game-server/src/staff/audit.rs (new); apps/game-server/src/staff/signature.rs (new; request bytes, allowed-signers roster and namespaces, `ssh-keygen -Y verify`, stored request digest); apps/game-server/src/node/serve.rs (verify before a control-socket action); apps/game-server/src/bin/oteryn-game-ops.rs (`--prepare`, `submit`, `audit verify`, the copy of signed revoke, mute, raise and hold placement requests and their outcome records to the restore fence directory and their replay, called at restore step 7 by amendment 5 of §3); the `OPS_STAFF_SIGNATURE_INVALID` code (A7) | registry check passes; an audit insert rolls back with its mutation; a failed inspect audit shows nothing; an unsigned request, a signature by a principal not on the roster, a staff-namespace signature presented as a remediation, another environment id, and `issued_at` older than the window are each refused with nothing written; the same bytes replayed after the window return the stored outcome; other bytes under the same operation_id are refused; a roster whose SHA-256 differs from the configured one stops the tool; `audit verify` flags an OPERATOR row with a bad signature and a sanction row with no verifying `STAFF_ACTION`; an unknown action kind, reason code or result is refused; the `ECON_FENCE_RAISE` and `ECON_FENCE_LIFT` kinds and the three case-origin reason codes are accepted; a restore to a T before a revoke, a mute, a fence raise and a hold placement re-applies all four once, a second run changes nothing, and a grant, unmute, lift or hold release after T is not re-applied; with the issuer's clock 10 min behind, a mute committed after T with an `issued_at` before T is still re-applied, and with it 10 min ahead, a mute committed and unmuted before T stays unmuted; a request file with no outcome record stops the restore until a signed `restore resolve` applies or abandons it, and a `REFUSED` record is not replayed; a crash between the node's commit and the outcome record is reconciled by submitting the same bytes, which appends the `COMMITTED` record; a mute whose duration, a roster grant whose tier or a hold whose expiry differs from the signed bytes is refused with nothing written | ALPHA, first |
| OPS-GM-SANCTION-2 | apps/game-server/migrations/NNNN_character_sanctions.sql (new); apps/game-server/src/staff/{mod,sanction}.rs (new); apps/game-server/src/chat/ (mute check); apps/game-server/src/node/serve.rs (control commands); apps/game-server/src/bin/oteryn-game-ops.rs; gameplay_transport release enum; the protocol and error code registries (A7) | kick vs logout race has one winner; kick in combat releases; no resume after a kick; mute on a live and an offline target; concurrent mutes; unmute compare-and-set; mute survives a restart; ops replay is idempotent | ALPHA, after AUDIT-1 and LOGOUT-WIRE-1 |
| OPS-GM-REPORT-3 | apps/game-server/src/bin/oteryn-game-ops.rs (`reports`); WRITE-0 moderation module; .github/ISSUE_TEMPLATE/player_bug_report.yml (new; the existing `bug_report.yml` is the developer defect form) | precondition: the live repository setting shows private vulnerability reporting enabled, else stop and report; the issue form is valid and asks for no account name, email or chat text; list, close, compare-and-set, stale close; clear-text is keyed by action id and audited | ALPHA; the issue form first, the `reports` commands after WRITE-MOD-1 |
| OPS-GM-ROSTER-4 | migrations (new) `game_staff_roles`, the `OTI06` refusal in `game_character_inbox_deliver`, and the control-plane column privilege on the root; apps/game-server/src/staff/roster.rs; admission binding; ops `staff grant / revoke`; the ruling 10 boundary in apps/game-server/src/durability/{item_transfer.rs, character_equipment.rs, reward_claim_mint.rs, map_item_mint.rs, spell_item_transaction.rs, bank.rs, world_party.rs, character_death.rs} and apps/game-server/src/combat/death_reward.rs | a revoke mid-session refuses the next command; a revision change gives ROLE_REVOKED; grant and revoke are signed and audited; a roster row cannot be deleted; a grant pairing Account A with a character of Account B, or with a finalised character, is refused with `OPS_STAFF_GRANT_TARGET_MISMATCH` and writes no roster or audit row, and a direct insert of such a pair fails the composite foreign key; for a staff-marked character each of these is refused with `STAFF_VALUE_ISOLATED` and writes nothing: Ground pickup, corpse loot, map item pickup, spell overflow to the Ground, bank deposit, withdraw, transfer from it and transfer to it, and Inbox delivery to it (`OTI06`); a staff kill mints no loot and grants no experience or bestiary progress; a party refuses a staff member; Internal moves (equip, reward claim, a spell item into the own backpack) still commit; a revoked row keeps the mark; a grant racing a Ground pickup and a bank transfer on two connections with a deterministic interleaving: the writer that locked the root first commits, and the one that locks after the grant is refused; a fence caller without an `ItemBoundary` does not compile | EXT |
| OPS-GM-STAFF-5 | protocol registry (A6); apps/game-server/src/staff/command.rs; intent-dispatch isolation check; native client staff panel | not offered to a non-staff or older client; tier matrix; same-channel only; teleport goes through the target's fence; the dispatch check refuses a staff character's trade early (the ROSTER-4 boundary is the guarantee) | EXT, after ROSTER-4 |
| OPS-GM-PREPORT-6 | migrations (new) `game_player_reports`; apps/game-server/src/staff/report.rs; apps/game-server/src/chat/ (per-session line buffer); protocol registry | dedup; limits; expiry; restricted read; no free text accepted; a reported line is copied from the server buffer, never from the client; an evicted or unknown revision gives `CHAT_LINE_UNAVAILABLE` and writes nothing; a received line spoken by character B in a report naming character A, a system line, and a revision on a non-HARASSMENT reason each give `CHAT_LINE_TARGET_MISMATCH` and write nothing; a direct insert whose speaker differs from the target, a revision with a NULL speaker, a speaker with no revision, partial evidence columns or evidence on a non-HARASSMENT reason each fail the table check; no chat text in logs or audit | EXT |
| P1 (proposal, Platform) | none in Game until accepted; Game handler in OPS-GM-SANCTION-2 follow-up | idempotent replay; stale account_security_generation ignored; an unsigned request, a bad signature or a signature for another environment id kicks nothing and writes nothing | EXT; routed by the control plane |

### Owner questions

1. Where does staff authority come from? a) A Game roster written by the operator with `oteryn-game-ops` (ruling 4). b) A Platform staff claim in a new grant profile version (cross-repo). **Recommend a.** It is local, immediate to revoke and needs no Platform change.
   - **Owner ruling 2026-10-06: a.** Ruling 4 applies it.
2. Where do player bug reports go? a) Alpha: an out-of-band channel you name, with the pasted F12 line; EXT: the same, plus in-game rule-violation reports only. b) An in-game bug report into the Game queue. c) The Platform support desk (needs a Platform proposal). **Recommend a.**
   - **Owner ruling 2026-10-06: a**, refined: player reports go through GitHub Issues only, via a bug-report issue form; exploits go through GitHub private vulnerability reporting; Discord is added only when external testers arrive. The control plane reports private vulnerability reporting already enabled; this decision has not verified it, so OPS-GM-REPORT-3 verifies it as a precondition. The issue form is delivered by OPS-GM-REPORT-3. Ruling 12 applies it.
3. How long is staff action audit kept? a) 1 year (like the SEC-CLIENT-01 reviewer audit). b) 2 years (like the identity and access audit). c) 90 days (like the character audit). **Recommend a.** It covers appeals and staff-abuse review.
   - **Owner ruling 2026-10-06: a.** Ruling 11 and amendment A2 apply it.
4. Can a harassment report carry the reported chat line? a) No. Staff act on report counts and live observation. b) Yes. The node keeps the last lines a reporter received in memory only, and a report snapshots the one line chosen into the restricted store for 180 days. **Recommend b for EXT.** Without it, chat moderation has no evidence.
   - **Owner ruling 2026-10-06: b.** Ruling 12, amendments A4 and A6, and the §3 ruling 14 inventory apply it.

### Rejected options

- GM item or gold grants with an audit trail: these create value outside DUR-03 and the two-person rule (ruling 1).
- Chat-prefix "/" commands parsed out of CHAT_INTENT: this mixes privileged commands into a broadcast path with spam limits, and it is untyped.
- A staff claim in the FND-04 grant now: it needs a cross-repo profile version, and revocation only reaches the session at the next admission.
- Staff mute in the CHAT-0 auto-mute row: a staff write would contend with the session fence.
- Kick as a plain transport close: the player could resume, or the character would stay in the world during grace.
- Letting Platform end sessions through grant validation: FND-04 §3.4 forbids it.
- A TUTOR tier, a case management system, ban waves and GM rollback: not needed before external players.

### Open unknowns

- U1 (UNKNOWN): an in-session relocation primitive for teleport (F24). OPS-GM-STAFF-5 confirms it or adds the smallest one.
- U2 (UNKNOWN): how the ops tool finds the owning node's control socket when nodes are on different hosts. Today it is local only (F4).
- U3 (CANDIDATE): sanction and report limits and durations and the chat line buffer size (ruling 12). Staff audit retention is fixed at 1 year by owner ruling §1 Q3 a (A2); the rest are fixed by the RL rows.
- U4: an offline "unstuck" relocation and a character name lock are not decided. A name lock may touch Platform lifecycle.
- U5: WRITE-0, CHAT-0, SEC-CLIENT-01, DUR-03, ANL-01 and FND-04A are CANDIDATE. Rulings that amend them land with or after their acceptance. No CONFLICT was found.

## 2. System threat model and live-ops controls

Scope: one living threat model, the boundary inventory, the deferred §1.9 display item from
ARCH-ERROR-CODES-0, and gap PROD-LIVEOPS-01 (runtime config, flags, maintenance, secrets).
Builds on ARCH-ERROR-CODES-0 (`docs/architecture/reviews/OTERYN_GAME_ARCH_ERROR_CODES_2026-10-05.md`)
and OPS-NODE-BOOT-01 (`docs/architecture/reviews/OTERYN_GAME_NODE_BOOT_COMPOSITION_DECISION_2026-09-24.md`).
No secret values appear here.

### Facts

F1. PROVEN. No whole-system threat model exists. Partial ones: DUR-03 analysis §24 "Threat review"
(`docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_ANALYSIS.md`), FND-04B §24
(`docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md`), the SEC-CLIENT-01 decision
(`docs/architecture/reviews/OTERYN_GAME_SEC_CLIENT01_CLIENT_INTEGRITY_AND_ANTI_BOT_DECISION_2026-10-01.md`, CANDIDATE),
the registration bootstrap decision and the revoke-report decision.
F2. PROVEN. The owner-accepted refinements require "a maintained threat model covering client, Gateway,
GameNode, admin plane and update pipeline" before external alpha
(`docs/architecture/ARCHITECTURE_REVIEW_REFINEMENTS_2026-08-10.md:168`). The same requirement appears in
`docs/architecture/ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_CONTRACT_CANDIDATE.md:582`.
F3. PROVEN. Gap register: PROD-LIVEOPS-01 §22 (lines 413–426) REGISTERED_UNRESOLVED; OPS-GM-01 §23 unresolved;
§26 lists secrets; SEC-CLIENT-01 §27; EXP-SECURITY-01 §29 lists supply chain, signing keys, secret
management and incident response (`docs/architecture/ARCHITECTURE_ANALYSIS_GAP_REGISTER.md`).
F4. PROVEN. Node configuration is one closed-schema TOML read once at start. Sections: listener, scope,
database, character, control, readiness, platform, launch, world_bundle
(`apps/game-server/src/node/config.rs:182-191`). No reload or SIGHUP path exists in `apps/game-server/src`
(grep). Changes take effect only by stop-then-start replacement with a new NodeId (OPS-NODE-BOOT-01 D3/D5).
F5. PROVEN. Secrets arrive only as file references, opened without following symlinks, owner = reading user,
mode 0600/0400, parent not group/other writable (`apps/game-server/src/node/secure_file.rs`; OPS-NODE-BOOT-01 D1).
Secret file keys: `database.password_file`, `database.root_ca_file`, `listener.private_key_file`,
`platform.client_key_file`, `platform.runtime_status.client_key_file`, `platform.trust_roots_file`,
`launch.authorization_file` (`config.rs:45-135`).
F6. PROVEN. Two PostgreSQL group roles (runtime, control-plane); login roles are members; the control-scope
grant table is writable only by the DB owner (OPS-NODE-BOOT-01 D2). `oteryn-game-ops` runs only as root,
holds only the control-plane credential, and writes each request file durably before submitting it
(`apps/game-server/src/bin/oteryn-game-ops.rs:1-7`). Exit classes 2–7 (`oteryn-game-ops.rs:58-70`).
F7. PROVEN. Shutdown on SIGTERM/SIGINT withdraws readiness first within a 10 s budget, then stops the loops.
In-flight admissions and durability passes run to their own outcome (`apps/game-server/src/node/serve.rs:70,1076,1520-1556`).
Connected sessions are not told why they close. DERIVED from the same lines: they see a transport close.
F8. PROVEN. Wire codes reach the player through `ProtocolError` (message type 14, phase ANY) and
`CommandResult` (`docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json:42`). No maintenance, drain or
shutdown code exists. 1100–1116 are admission; 1117–1199 are reserved to admission; 1200–1999 are future
wire codes through FND-02 registration (ARCH-ERROR-CODES-0 §1.2).
F9. PROVEN. FND-04A §3 gives Platform the "configured world/channel/login/maintenance/entitlement policy",
offer/route orchestration and grant signing. Game owns final admission, readiness, presence and session
(`docs/architecture/FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md:68`).
F10. PROVEN. A ready=false report or a stale heartbeat makes Platform stop routing to the scope. Heartbeat 5 s
and freshness 15 s are proposed, not accepted (`docs/contracts/OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md` §8).
mTLS, TLS 1.3, one client certificate per purpose (§3). Key separation §16.2 is pending
(`docs/architecture/reviews/OTERYN_GAME_ARCH_REVOKE_REPORT_2026-10-05.md`).
F11. PROVEN. FND-04B §22: after node replacement the same session continues only with fenced recoverable
evidence; a process restart is not a loss epoch.
F12. PROVEN. Connection bounds: `MAX_CONNECTIONS = 256`, `MAX_HANDSHAKE_UNITS = 64`,
`MAX_ENTRY_DEADLINE_MS = 60_000` (`config.rs:13-19`). No per-source rate limit exists (grep, DERIVED).
F13. PROVEN. CI runs cargo-deny 0.20.2 pinned by commit SHA (`.github/workflows/rust.yml:425`,
`.github/workflows/merge-gate.yml:1225`; `deny.toml` denies yanked crates and restricts licenses) and CodeQL
(`.github/workflows/codeql.yml`). No release signing, SBOM or build provenance workflow exists (grep).
F14. PROVEN. Capabilities carry `offered` and an `offer_gate`; a capability not selected is never sent and its
commands are refused (`PROTOCOL_OTERYN_V1_REGISTRY.json:78-86`, CHAT_V1). 1116
ADMISSION_CAPABILITY_REQUIRED exists (`:691`).
F15. PROVEN. ARCH-ERROR-CODES-0 §1.8 item 2 and §1.9: the player sees the code now; whether release builds keep it
is decided "after a security review of the `public_class` codes, before the first public release"; the
catalogue keeps display in one place. Live log-level change is deferred (§1.9); `OTERYN_LOG` is read once (§1.10).
F16. CONFLICT. `docs/architecture/GAME-CHANNEL-01_CHANNEL_PRODUCT_POLICY_CONTRACT.md` header says CANDIDATE;
`docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md:42` says ACCEPTED/LIFECYCLE_CLOSED. This draft relies
on its §12 (typed maintenance evacuation) and §27 (DRAINING class) only as the register states them.
F17. UNKNOWN. How the native client trusts the gameplay TLS certificate (pin in grant, private CA or public CA).
F18. UNKNOWN. Which admission code a not-ready node returns today.

### Rulings

1. **One threat model.** Path: `docs/architecture/OTERYN_GAME_SYSTEM_THREAT_MODEL.md` (new). Owner: the
   supervising architect role. Method: STRIDE per trust boundary, one table per boundary with threat,
   control + locator, or gap + packet. It replaces no partial model; it links them.
2. **Update trigger.** Any PR that adds or changes a trust boundary, credential, secret file key, wire
   authentication, ops privilege, CI publication path or content ingest path must update the threat model in
   the same PR. Enforcement: the PR template checkbox plus architect review; no new CI rule in this packet.
3. **Initial inventory.** The model starts from this table. A gap blocks external players unless marked "later".

| # | Boundary | Top threats (STRIDE) | Existing control (locator) | Gap → packet |
|---|---|---|---|---|
| B1 | client ↔ GameNode | S grant replay; T/I on wire; D connection floods; E modified client | one-shot grant and admission (FND-04A); evidence age ≤ 5 s (FND-04A:39); TLS 1.3 + ALPN (registry:16); 256/64 bounds (config.rs:13-14); server authority; public_class (ERR §1.8) | per-source connection rate limit → THREAT-DOS-1; network DDoS → host-included mitigation chosen with the hosting decision (owner ruling Q2 a); client integrity → SEC-CLIENT-01 |
| B2 | Platform gateway ↔ node (route, grant) | S forged grant or route; T stale route; R who routed | Platform-signed grants; 1108–1110 stale codes; runtime generation check | grant verification key and Platform request-signing key (§1 ruling 7, §3 ruling 16) delivery and rotation unknown (U4) → SEC-ROTATION-1 |
| B3 | node ↔ PostgreSQL | S stolen credential; T cross-scope writes; E runtime role doing control work | TLS with pinned CA (config.rs:45-52); two group roles + owner-only grant table (D2); session-generation fence; DUR-03 §24 | backup encryption and access → ARCH-ALPHA-OPS-0 §3 ruling 7; credential rotation → SEC-ROTATION-1 |
| B4 | node ↔ Platform (registration, status, projection) | S node impersonation; T false ready; I identity leak | one-shot launch authorization, NodeId never a credential (registration decision §2-§4); mTLS per purpose (RS §3); revocation report carries no identity | key separation §16.2 pending → OPS-KEY-SEPARATION-2 (existing) |
| B5 | ops tooling ↔ node/DB | S non-root caller; S/R which human acted; T replayed or forged request; T root editing host logs | root only; control socket SO_PEERCRED uid 0; request file before submit; idempotent replay (D2, ops.rs:1-7); a personal OpenSSH signature on every staff, report, roster, fence, case hold, remediation and privacy request, verified against the reviewed allowed-signers roster before any write and stored with its audit row (§1 ruling 2; §3 rulings 4, 5, 8); account erasure only through the erasure handler's own database role, in neither group, for a Platform-signed request journalled first (§3 rulings 17-18) | root bypass of on-host checks with the node's database credential → detected afterwards by `oteryn-game-ops audit verify` (OPS-GM-AUDIT-1); root using the erasure role's credential → an erasure row with no verifying Platform request and journal record is flagged by `audit verify` and never re-applied, so a restore to before it recovers the rows (§3 ruling 17); location of the database owner credential, which can disable triggers → THREAT-MODEL-0 (recommended off the node host); infrastructure commands (maintenance, assignment, authorization, content activation) move no player value and stay root-attributed with host login audit as a secondary record → THREAT-MODEL-0 lists them |
| B6 | CI / supply chain | T malicious dependency or action; S unsigned release | cargo-deny pinned by SHA; CodeQL; Merge Queue + game-gate | release signing, SBOM, provenance → SUPPLY-SIGN-1 (later for closed alpha, required before external players) |
| B7 | content pipeline | T corrupt or hostile bundle; I redistribution of third-party files | bundle digest pinned in config (`world_bundle.digest`, config.rs:144-156); activation via ops request | bundle provenance is the CI build that produced the digest → SUPPLY-SIGN-1 covers bundles |
| B8 | admin / GM | E any player gaining GM power; R unaudited GM action | none: no GM path exists (gap §23) | ruling 4; staff tooling and audit → §1 (OPS-GM-01) |

4. **No GM path on the wire** until OPS-GM-STAFF-5 delivers the typed `STAFF_V1` capability (§1 ruling 9). Any admin action meanwhile goes through
   `oteryn-game-ops` with a retained request file; a staff, report, roster, fence, case hold, remediation or
   privacy request also carries a personal signature verified before any write (§1 ruling 2). This
   is a control, not a deferral.
5. **Release-build code display (closes ARCH-ERROR-CODES-0 §1.9 item).** Release builds keep showing the
   public wire code next to the text. Reason: §1.8 already forbids a code that reveals more than its public
   text, and the number is in every client binary anyway. Condition: ERR-PUBLIC-REVIEW-1 passes before the
   first external player. If the review finds a code that reveals hidden state, that code changes its
   public_class or merges; the display switch stays on. Codes without public_class never reach the player (§1.8).
6. **Runtime configuration.** For alpha and the first external players every node setting is restart-only
   (F4). Live change happens only through existing control-plane state: assignment, authorization, content
   activation and readiness, each through `oteryn-game-ops` with request files. No reload signal, no remote
   config channel. Live log level stays deferred (ERR §1.9).
7. **Config provenance.** At `process_start` the node logs the SHA-256 of the exact config bytes, the
   `platform.descriptor_revision` and the world bundle digest. The deployment keeps config files in a private
   version-controlled location; the logged digest must match a reviewed revision. The deployment tool is
   the gap §26 decision, not this one.
8. **Feature flags.** No flag service. Three mechanisms only: (a) protocol capabilities with `offered` and
   `offer_gate` (F14); (b) closed-schema config keys, added only by a decision; (c) content activation.
   Kill switch: one new config key `[capabilities] withheld = [ids]` (LIVEOPS-KILL-2). A withheld capability
   is not offered; a client that requires it gets 1116. Staged rollout = replacing one channel's node at a time.
9. **Maintenance mode = announced graceful stop.** Sequence for one `RuntimeScopeRefV1` (WorldId, ChannelId):
   (1) Platform sets its maintenance policy and stops offers and grant signing for the scope (Platform-owned,
   F9; proposal P1). (2) The operator runs `oteryn-game-ops maintenance --world W --channel C --close-at T`
   (new) on the Game operator's own decision (owner ruling Q1 a); it writes the request file, then commits a
   durable maintenance marker for the scope in Game PostgreSQL with the control-plane role (new table
   `game_scope_maintenance`: scope, operation_id, close_at, state OPEN or ENDED, revision; an OPEN row is
   replayed idempotently by its operation_id), then sends the request over the control socket. (3) The node publishes
   ready=false (CompareAndSet, as at shutdown) and reports it; Platform stops routing (F10). New admissions use
   the existing refusal codes; RETRY_LOGIN returns the player to Platform, which shows the maintenance state.
   (4) Clients that selected SERVICE_NOTICE_V1 get one notice with `close_at`. (5) At `close_at` the node stops reading
   command frames on every session of the scope; a frame it has not read was never acknowledged and never ran.
   (6) It waits for each session's in-flight durability pass to end, within the pass's own deadline, and writes
   that command's result frame. A pass whose commit outcome is unknown at its deadline ends the session as any
   durability failure does today, and the state after the next admission is authoritative. (7) Only then does
   it send `ProtocolError` code 1200 SESSION_SERVICE_MAINTENANCE to that session and close it. No session is
   closed while it has a pass in flight. Every acknowledged Character write is already durable and
   session-generation fenced, so there is no separate "save all" step. (8) Reopen is a new launch:
   the operator ends the marker with `oteryn-game-ops maintenance end --world W --channel C` (compare-and-set
   OPEN → ENDED, its own operation id and retained request file), then stop-then-start, new NodeId, new launch
   authorization, then Platform clears its policy. No in-place reopen of the same incarnation.
   Boot reads the marker: before its first readiness publication (today `readiness.publish(true, None)`,
   `apps/game-server/src/node/serve.rs:1633`) the node reads the marker of its own scope. An OPEN marker keeps
   it ready=false, so it admits nobody; a node restarted mid-countdown or mid-drain has no sessions left to
   drain and stays not ready until `maintenance end`. If the marker cannot be read, boot treats it as OPEN
   (fail closed). The request file stays the operator's replay record; the marker is the node's.
10. **Maintenance close is terminal.** Code 1200 has progression TERMINAL: the client does not resume; the
    player logs in fresh. A cross-channel evacuation stays the typed, audited GAME-CHANNEL-01 §12 exception, not
    this path.
11. **Older clients.** A client without SERVICE_NOTICE_V1 gets no countdown. An older client that does not
    know 1200 shows the generic text of its block (ERR §1.6). No version gate is needed for the close code.
12. **Secrets stay files** with the F5 checks. No secret service in alpha. Delivery for production is part of the
    gap §26 deployment decision; recommended upstream: systemd credentials (`LoadCredential=`) if
    SEC-ROTATION-1 shows they pass the `secure_file` checks, else the config-management tool's file deploy.
    HashiCorp Vault/OpenBao is rejected for now (rejected options).
13. **Rotation means restart.** Every secret is read once. Rotating any of them is a node replacement, run as a
    maintenance (ruling 9) or as a rolling replacement per channel. Live sessions continue only through the
    FND-04B fenced path (F11); otherwise they receive 1200 and log in fresh.

| Secret (file key) | Holder | Rotation | Live-session effect |
|---|---|---|---|
| DB runtime password (`database.password_file`) | node | two login roles in the runtime group; switch the inactive one, replace nodes, then disable the old | node restart |
| DB control-plane password (`OpsConfig.database`) | ops tool | same pattern, control-plane group | none |
| DB erasure password (`privacy.erasure_database_password_file`, §3 ruling 17) | node erasure handler only; never the ops tool. It authorizes nothing without a Platform tag that the database verifies | two login roles in neither group, both granted only the erasure functions; switch the inactive one, replace nodes, then disable the old | node restart |
| Platform request verification key (`platform.request_verification_key_file`, §1 ruling 7, §3 ruling 16) | node | overlap old and new keys across one replacement (U4) | restart |
| Erasure authorization key (table `game_account_erasure_auth_keys`, §3 ruling 17; never a file) | database only, owned by `oteryn_game_erasure`; Platform | the database owner adds a new `ACTIVE` key, Platform switches, the old key becomes `RETIRED`; deleted on compromise | none |
| DB CA (`database.root_ca_file`) | node, ops | add new CA alongside old, replace, remove old | node restart |
| Gameplay TLS key (`listener.private_key_file`) | node | new chain + key, replace node | restart; trust path F17 |
| Platform client keys (`platform.client_key_file`, `runtime_status.client_key_file`) | node, ops | new cert per purpose, new `descriptor_revision` issued (D3 step 4) | restart |
| Platform trust roots (`platform.trust_roots_file`) | node | overlap old and new roots across one replacement | restart |
| Launch authorization (`launch.authorization_file`) | node | one-shot, consumed; never rotated, revoke/supersede only | none |

14. **Compromise response.** On suspected key compromise: revoke with `oteryn-game-ops` authorization/assignment
    revoke (revocation report, ARCH-REVOKE-REPORT), rotate per the table, record in the threat model change log.
    Scheduled rotation periods are CANDIDATE: TLS and mTLS leaf 90 days, DB passwords 180 days, fixed by
    SEC-ROTATION-1 after one timed rehearsal on the disposable preprod stack.
15. **Before-freeze check for ruling 9.** Concurrent transitions: maintenance, signal and assignment revoke all
    feed the one shutdown token; the first wins and the rest are no-ops. Restart/resume: the request file and
    operation id make the ops command replay-safe; the durable maintenance marker, read at boot before the
    first readiness publication and fail closed, makes a node that died mid-drain restart not ready. Typed
    references: the command carries `RuntimeScopeRefV1`, checked against the node's own scope; a mismatch is
    rejected (6xxx). Older peers: ruling 11; Platform needs no wire change. Multi-component: Platform policy and
    Game drain are independent; if Game acts first, ready=false still stops routing; if Platform acts first,
    no new grants arrive.

### Contract amendments

A1. `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json`, `error_codes`, after 1116 (through FND-02 registration
and protocol review):
`{"code": 1200, "name": "SESSION_SERVICE_MAINTENANCE", "category": "INTERNAL_UNAVAILABLE", "default_disposition": "TRANSPORT_FATAL", "progression": "TERMINAL", "public_class": "TEMPORARILY_UNAVAILABLE"}`
A2. Same file, `capabilities`: `SERVICE_NOTICE_V1`, one state domain carrying `{kind: MAINTENANCE, close_at_unix_ms}`,
`"offered": false`, `offer_gate`: "not offered before LIVEOPS-MAINT-1 composes the notice". Capability id and
state-domain id are leased by the #162 control plane; no number is chosen here.
A3. `docs/architecture/reviews/OTERYN_GAME_ARCH_ERROR_CODES_2026-10-05.md` §1.9, replace the release-display
sentence with: "Release builds keep showing the public wire code (ARCH-LIVE-READINESS-0 §2 ruling 5), subject
to ERR-PUBLIC-REVIEW-1 passing before the first external player."
A4. `docs/architecture/ARCHITECTURE_ANALYSIS_GAP_REGISTER.md` §22: status "PARTIALLY_RESOLVED by
ARCH-LIVE-READINESS-0: static config, no flag service, maintenance sequence, secrets as files. Open: event and
rate schedules, live log level." Add the threat model path to §27 and §29.
A5. `docs/contracts/OTERYN_GAME_ERROR_CODE_REGISTRY.json`, 6xxx block: add
`OPS_MAINTENANCE_SCOPE_MISMATCH` for ruling 15 (number assigned by LIVEOPS-MAINT-1 from the next free 6xxx code).

### Packets

- **THREAT-MODEL-0.** Writes the threat model from ruling 3, with links to the partial models and the update
  trigger. Row B5 records the signed-request control, where the database owner credential lives, and the
  infrastructure commands that stay root-attributed. Owned: `docs/architecture/OTERYN_GAME_SYSTEM_THREAT_MODEL.md` (new), `.github/pull_request_template.md`
  (checkbox line). Tests: agent-governance and architecture-semantic-audit workflows pass. Depends: none.
- **ERR-PUBLIC-REVIEW-1.** Reviews every code with a public_class against its text; records one line per code.
  Owned: the threat model §B1, A3 edit. Tests: a registry check that every public_class code has a review row
  (`apps/game-server` registry tests or `tools/` validator, path decided by the packet). Depends: THREAT-MODEL-0.
- **LIVEOPS-CONFIG-AUDIT-1.** Logs config SHA-256, descriptor revision and bundle digest at `process_start`.
  Owned: `apps/game-server/src/node/serve.rs`, `apps/game-server/src/node/config.rs`. Tests: unit test that the
  digest equals SHA-256 of the input bytes and that no secret value appears in the line. Depends: none.
- **LIVEOPS-MAINT-1.** Code 1200 (A1), ops `maintenance` and `maintenance end` subcommands, the durable
  maintenance marker and its boot check, control-socket command, close of all sessions at `close_at`, A5
  code. Owned: `apps/game-server/migrations/NNNN_scope_maintenance.sql` (new),
  `apps/game-server/src/bin/oteryn-game-ops.rs`, `apps/game-server/src/node/serve.rs`,
  `apps/game-server/src/node/operator_files.rs`, the session close path in
  `apps/game-server/src/gameplay_transport/connection.rs`, the registry files.
  Tests: drain closes every session with 1200 and leaves no unacknowledged write; a command inside a durability
  pass at `close_at` commits, its result frame reaches the client before 1200, and a command frame sent after
  `close_at` is not run; replayed request is a
  no-op; scope mismatch rejected; signal during maintenance converges; a process killed during the
  countdown and restarted publishes ready=false and admits nobody until `maintenance end`; an unreadable
  marker keeps the node not ready. Depends: protocol review of A1.
- **LIVEOPS-NOTICE-2.** SERVICE_NOTICE_V1 server side and native client display. Owned: the registry,
  `apps/game-server/src/node/serve.rs`, client paths named by the client lane. Tests: unselected client never
  gets the domain. Depends: LIVEOPS-MAINT-1, A2 lease. Before external players, not alpha.
- **LIVEOPS-KILL-2.** `[capabilities] withheld` config key. Owned: `apps/game-server/src/node/config.rs`,
  capability offer site. Tests: withheld capability is not offered; unknown id rejects the config. Depends: none.
- **THREAT-DOS-1.** Per-source connection and handshake rate limit at accept. Numbers CANDIDATE, fixed by a
  load measurement. Owned: `apps/game-server/src/node/serve.rs`, `config.rs`. Tests: flood from one source
  does not exhaust handshake units. Depends: ARCH-ALPHA-OPS-0 §2 PERF-ALPHA-1 measurement.
- **SEC-ROTATION-1.** Rotation runbook for every row of the ruling 13 table, rehearsed once on the disposable
  preprod stack; checks systemd credentials against `secure_file`. Owned: `docs/operations/` runbook (new).
  Tests: the rehearsal log. Depends: preprod route authority items 3–4.
- **SUPPLY-SIGN-1.** Release artifacts and world bundles signed and attested: GitHub artifact attestations
  (Sigstore), SBOM by `cargo-cyclonedx`, `cargo-auditable` binaries. Owned: `.github/workflows/` (new
  release workflow). Tests: verification step fails on a tampered artifact. Depends: SEC-REL-1 of SEC-CLIENT-01.

### Owner questions

1. Who triggers the Game-side maintenance drain? Context: Platform owns the policy (F9); Game owns readiness.
   a) Game operator runs `oteryn-game-ops maintenance` locally; Platform operator sets its policy separately.
   b) Platform calls a new Game control endpoint. c) Both, Platform-first. Recommend a: no new remote control
   channel; proposal P1 to Platform covers only "stop offers and grants, show maintenance in login".
   - **Owner ruling 2026-10-06: a.** Ruling 9 applies it.
2. Network DDoS protection before external players. Context: only node-level bounds exist (F12); this is spending.
   a) Host-included network mitigation, chosen with the hosting decision. b) Paid L4 proxy in front of nodes.
   c) None until public release. Recommend a.
   - **Owner ruling 2026-10-06: a.** Ruling 3, row B1, applies it.
3. External security review before external players. Context: the threat model is internal; spending.
   a) Internal review only. b) One paid external review of B1, B4 and B5 before public release. c) Bug bounty.
   Recommend a.
   - **Owner ruling 2026-10-06: a.** Internal review only; no paid external review or bug bounty is planned.

### Rejected options

- Hot reload of node config (SIGHUP): adds restart/resume state to every setting; replacement already exists.
- Feature-flag service (LaunchDarkly, Unleash, flagd): a second authority for behaviour; capabilities suffice.
- Secret manager (Vault/OpenBao) in alpha: a new stateful service and credential to protect; files suffice.
- In-place reopen of the same incarnation after maintenance: new readiness transitions; stop-then-start is proven.
- New admission code for maintenance in 1117–1199: Platform already shows maintenance at login.
- Hiding codes in release builds: no security gain under §1.8; harms support.
- Generic "maintenance bypass" flag for evacuation: forbidden by GAME-CHANNEL-01 §12.

### Open unknowns

- U1. Gameplay TLS trust path in the native client (F17); needed by SEC-ROTATION-1. Ask the client lane.
- U2. Admission code a not-ready node returns (F18); LIVEOPS-MAINT-1 test records it.
- U3. Heartbeat and freshness values remain CANDIDATE (U-RS1); maintenance timing depends on F.
- U4. How the node learns Platform's grant verification key and its request verification key (§1 ruling 7, §3 ruling 16), and how they rotate (B2). Proposal to Platform.
- U5. GAME-CHANNEL-01 lifecycle CONFLICT (F16); the register owner must fix the header or the register.
- U6. Whether systemd credentials satisfy the `secure_file` owner and mode checks; SEC-ROTATION-1 measures.

## 3. Economy incident response and data privacy lifecycle

Scope A is the response to a duplication or value bug. Scope B is the data privacy lifecycle (gap `DATA-PRIVACY-01`).
Package: "before external players", authored in parallel with the alpha package (owner ruling, 2026-10-06).
Full disaster recovery restore belongs to ARCH-ALPHA-OPS-0 §3 (data continuity). It is referenced here, not decided.

### Facts

| # | Fact | Locator | Class |
|---|---|---|---|
| F1 | A correction is a new TransactionId with causation to the original, current authorization and full conservation evidence. "Raw row/audit rewrite is not compensation." | DUR-03 §26 | PROVEN |
| F2 | Analytics may detect. It may not mutate, auto-delete duplicates, mint compensation or sanction. Correction uses a new typed, authorized transaction under the owning gameplay/admin/security contract. | DUR-03 §42; ANL-03 §2 | PROVEN |
| F3 | Raw SQL or an admin mutation is not an ordinary correction. | DUR-03 §49; CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md "Security and privacy consequences" | PROVEN |
| F4 | An integrity failure keeps the affected mutation closed until an explicit, safe repair or compensation path exists. | DUR-03 §41 | PROVEN |
| F5 | The burn causes are a closed set: FeeBurnCause, NpcTradeCause, ItemUseCause and DECAY_RETIRE (CorpseDecay, WorldReset). Silent deletion or quantity=0 is not a sink. No cause covers remediation or character retirement. | DUR-03 §15 | PROVEN |
| F6 | No cross-world transfer. A burn and mint across worlds is laundering. The same occurrence cannot mint twice. | DUR-03 §19, §14 | PROVEN |
| F7 | Value outside items: bank gold per (AccountId, WorldId) with an immutable ledger (BANK-0, migration 0071); forge dust per Character (FORGE-1a, migration 0059). | DUR-03 §18; apps/game-server/migrations/0071_account_bank.sql, 0059 | PROVEN |
| F8 | ANL-03 defines a provenance projection, 11 integrity invariants, and finding classes INVARIANT_VIOLATION_SUPPORTED, ANOMALY_HYPOTHESIS and REPLAY_CORROBORATED_DEFECT, plus case dispositions with an immutable audit. | ANL-03 §6, §6.1, §10, §11 | PROVEN |
| F9 | Enforcement, GM and account remediation authority are outside ANL-03 and need an owning contract. | ANL-03 §22 ANL03-XD-02 | PROVEN |
| F10 | Store coin balance, payment flow and purchase ledger are Platform's. Delivery ownership, entitlement revocation, refunds, chargebacks and fraud correction are open. | OTERYN_STORE_CATALOG_OWNER_DECISION_2026-09-28.md §2, §3 | PROVEN |
| F11 | Platform decides grant, expiry and revocation. Game enforces through a durable consumer fence. Revocation does not force a logout. | PROD-ENTITLEMENTS-01_GAME_CONSUMER_ENFORCEMENT_CONTRACT_CANDIDATE.md §2.1, §6, §12 | PROVEN (contract is CANDIDATE) |
| F12 | Database restore/PITR needs an external recovery register successor. Restored rows are historical evidence until they are reconciled. | reviews/OTERYN_CHARACTER_RESTORE_NONROLLBACK_FENCE_DECISION_2026-09-23.md "Restore / PITR" | PROVEN |
| F13 | Character audit retention is P90D (RESTRICTED_PLAYER_LINKED). One-item audit retention is P90D. The economy, market and guild event profiles are P30D (CANDIDATE). Authoritative bank, ledger, offer, escrow and guild tables are game state and are never deleted by these profiles. | reviews/…CHARACTER_DURABLE_AUDIT_RETENTION_DECISION_2026-09-22.md; …DUR03_ONE_ITEM_…_2026-09-27.md; …ECON_RET0_…_2026-10-04.md | PROVEN |
| F14 | A legal hold table exists for character audit only: `game_character_audit_legal_holds` (reason, actor, start, release). There is no hold for item audit or economy events. | apps/game-server/migrations/0005_character_authority.sql:157 | PROVEN |
| F15 | ANL-01 privacy classes: INTERNAL_NON_PERSONAL, PSEUDONYMOUS_ANALYTICS, RESTRICTED_PLAYER_LINKED and SECURITY_SENSITIVE. Every event type needs a finite retention profile, and unlimited retention is forbidden. Pseudonym map access is audited. No player-linked ids appear in metric labels or logs. | ANL-01 §15, §16, §17, §18 | PROVEN |
| F16 | SEC-CLIENT-01 defines a DSR reader (PRIVACY_DSR purpose), a 30-day account deletion bound (RL-18) and a deletion marker for ids (IDRES-1). Only the privacy officer role named by DATA-PRIVACY-01 may place a hold. Until that role is named, no hold can be placed. | reviews/OTERYN_GAME_SEC_CLIENT01_…_2026-10-01.md §5.1 | PROVEN (CANDIDATE decision) |
| F17 | Crash upload is on by default with a global opt-out. The lawful basis must be reviewed before production release. | CLIENT_CRASH_DIAGNOSTICS_PRIVACY_OWNER_BASELINE.md | PROVEN |
| F18 | Chat text is never written to a table or to ordinary logs. Mail and book text is player content in a text store, never in logs or audit. | CHAT-0 decision; MAIL-0 decision; WRITE-0 decision | PROVEN |
| F19 | Deletion lifecycle is Game-owned. A Platform workflow row is not proof. A deleted CharacterId is never reused. A released name key is blocked for 30 days (D183). | CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md §6.1, §7, §8; ADR-0012 | PROVEN |
| F20 | Cross-system deletion goes through a domain API, never through direct Platform SQL. | ADR-0004 | PROVEN |
| F21 | A guild leader cannot be deleted until they resign. | GUILD-0 decision | PROVEN |
| F22 | Whether Game transport or logs persist client IP addresses. | apps/game-server/src (no match for logged peer addresses found) | UNKNOWN |
| F23 | The DUR-03 file header says CANDIDATE/NONBINDING. The programme status lists DUR-03, ANL-01 and ANL-03 as ACCEPTED. | DUR-03 header; FOUNDATION_PROGRAMME_CURRENT_STATUS.md:55-68 | CONFLICT |
| F24 | The DATA-PRIVACY-01 items are classification, retention/deletion/anonymisation/legal hold, account deletion, export/access, consent, pseudonymous analytics, backup/audit exceptions and diagnostics. | ARCHITECTURE_ANALYSIS_GAP_REGISTER.md §28 | PROVEN |

### Rulings

#### A. Economy incident response

1. **One entry point.** An economy incident starts as an ANL-03 case (F8). The case opens from a detector signal, an integrity E-code line (DUR-03 §43) or a support report, recorded as the closed origin `DETECTOR_SIGNAL`, `INTEGRITY_ECODE` or `SUPPORT_REPORT`. The case id is the reference that every later record carries. Detection never mutates (F2).
2. **Containment is a typed fence.** New Game state `EconomyContainmentFence` with these scopes: `ITEM_TYPE_SET` (WorldId, ItemType ids), `MARKET` (WorldId), `CHARACTER_VALUE` (CharacterId), `BANK` (AccountId, WorldId). Every DUR-03 value mutation (TRANSFER, SPLIT_MERGE_QUANTITY, MINT, BURN, TRANSFORM, CONVERSION, bank and dust ledger) is covered. A covered mutation is refused with a typed outcome and no partial write. The fence is stored in Game PostgreSQL and survives restart. A fence row's state is `RAISED` or `LIFTED` (closed); a lifted row never returns to `RAISED`, and a new raise is a new row. A raise after the restore point survives a restore by re-apply of its signed request (§1 ruling 2). If the fence state cannot be read, the mutation is refused (fail closed, F4). Closing a whole channel uses the existing scope and assignment control. No new channel mechanism is needed.
   - **Serialization is a row lock, not a read.** Game transactions run at PostgreSQL's default READ COMMITTED; no isolation level is set anywhere in `apps/game-server/src` (grep). There, reading fence rows alone does not serialize with a concurrent raise: a writer can see no fence, and commit after the raise commits.
   - One fence root row, `game_economy_containment_root` (a singleton with `containment_revision`), is created by the ECON-FENCE-1 migration. It plays the part that the latest recovery admission row plays for Character writes: every fenced Character write holds that row FOR SHARE (`assert_recovery_fence`, `apps/game-server/src/durability/character_authority.rs:533-537`), and a recovery admission takes it FOR UPDATE before it advances (`character_authority.rs:408`). The session row is held FOR SHARE at its current generation in the same way (`assert_gameplay_fence`, `apps/game-server/src/durability/character_progression.rs:655-685`).
   - Every covered writer takes the root FOR SHARE directly after the recovery fence and before its session, guard and root locks (the BANK-0 lock order, `apps/game-server/src/durability/bank.rs:11-16`). It then reads the fence rows that cover its assets in a later statement of the same transaction.
   - A raise or lift takes the root FOR UPDATE, writes the fence row and its audit event, advances `containment_revision` and commits, all in one transaction.
   - FOR SHARE and FOR UPDATE conflict, so only two orders exist. A writer that holds the root first commits first; the raise waits for it, and that write is a fact from before containment. A writer that reaches the root after the raise holds it waits until the raise commits; its later fence-row statement takes a new READ COMMITTED snapshot, sees the raise and is refused. When a raise commits, no covered writer that missed it is still open, so no covered write can commit after it.
   - PostgreSQL requires UPDATE privilege on at least one column for FOR SHARE. The runtime role gets that privilege on one inert column of the root row only, and no write privilege on fence rows; raise and lift run with the control-plane role through `oteryn-game-ops`.
   - SERIALIZABLE is rejected: no Game path uses it, and every value writer would need a retry loop.
   - Test (ECON-FENCE-1, two PostgreSQL connections with a deterministic interleaving): (a) the raise transaction holds the root; a covered transfer blocks on it; the raise commits; the transfer returns the containment refusal and leaves no receipt, ledger line, balance change or event. (b) A covered transfer holds the root; the raise blocks until the transfer commits; the transfer's receipt exists and the raise then commits. (c) A transfer outside the fence scope is not refused after the raise.
3. **Containment does not touch sessions.** `CHARACTER_VALUE` refuses value mutations only. Movement and combat continue. The session-generation fence is unchanged. An older client sees the existing generic refusal message, so the protocol does not change (DERIVED: the fence adds no wire message).
4. **Raise fast, lift carefully.** The on-call operator may raise a fence without approval, because a fence is reversible and destroys nothing. A raise is still one signed request under `oteryn-ops-econ-raise@v1` (§1 ruling 2), so the raising person is known. Lifting a fence is under the two-person rule (owner ruling §3 Q1 b), with the same mechanism as a remediation approval (ruling 8):
   - A lift needs two signatures by two different `ECONOMY_REMEDIATOR` principals with different keys, from the same allowed-signers roster. Each signs `(environment_id, roster_sha256, fence_id, fence revision, case_id)` (§1 ruling 2) under the lift namespace `oteryn-ops-econ-lift@v1`, distinct from the remediation namespace, so a remediation signature cannot be replayed as a lift, and a lift signature cannot be reused for a later raise of the same scope.
   - The tool verifies both with `ssh-keygen -Y verify` against the roster, then commits the lift with the root FOR UPDATE (ruling 2) and a compare-and-set on the fence revision. The fence row stores both principals, key fingerprints and signatures, and the roster SHA-256; a CHECK refuses a `LIFTED` row that lacks either signature or names one principal or one fingerprint twice, so a lift written with the control-plane credential alone fails in the database. These stored signatures, not host login audit, are the evidence of who lifted, and `oteryn-game-ops audit verify` re-checks them (OPS-GM-AUDIT-1).
   - It fails closed. One signature, one principal signing twice, one key under two principals, a signature over another fence revision, or a roster with fewer than two principals refuses the lift and leaves the fence raised. The last case uses `OPS_REMEDIATION_SECOND_PERSON_UNAVAILABLE`. While the owner is the only role holder, a raised fence therefore stays raised until a second holder is added by a reviewed roster revision.
   - Every raise and lift is one `STAFF_ACTION` DURABLE_AUDIT event (§1 ruling 11, amendment A1; ANL-01 §5), action kind `ECON_FENCE_RAISE` or `ECON_FENCE_LIFT`, in the same transaction as the fence row change. It carries the case id, the signer or signers, the reason (the case origin of ruling 1) and the fence scope, and is kept under `STAFF_ACTION_AUDIT_RETENTION_V1`. A raise or lift whose audit insert fails rolls back.
5. **Preserve evidence at case open.** The case places a case hold on the affected audit and economy event rows before anything expires (P30D/P90D, F13). New hold tables for item audit and economy events copy the shape of `game_character_audit_legal_holds` (F14). A case hold is not a privacy legal hold. An `ECONOMY_REMEDIATOR` places and releases it, each by one signed request under `oteryn-ops-econ-hold@v1` (§1 ruling 2) whose principal, fingerprint and signature the hold row stores for `audit verify`; the case close releases it. A hold never extends past case close plus the expiry the row had.
6. **Investigation reads authoritative state.** Lineage comes from receipts, mint receipts, ledgers and escrow rows (game state, never retention-deleted, F13) and the audit outbox, joined into the ANL-03 §6 provenance graph. Tool: a read-only `oteryn-game-ops economy trace` subcommand in the existing ops binary. The ops credential can write, so the subcommand runs every query in one `READ ONLY` transaction, and PostgreSQL refuses any write in it. Output: a lineage report per ItemInstance or ledger line, with each hop's TransactionId and holder CharacterId. Only INVARIANT_VIOLATION_SUPPORTED or REPLAY_CORROBORATED_DEFECT may lead to remediation. ANOMALY_HYPOTHESIS never does.
7. **Remediation is a compensating DUR-03 transaction only.** Each step is a normal DUR-03 transaction with a new TransactionId, a typed `CausationRef {family, key, revision}` to the original transaction and full conservation evidence (F1). A burn uses the new closed cause `IntegrityRemediationCause` (amendment 1). Restoring a victim uses a MINT under the same cause, with exactly-once identity `(remediation_id, step_no)`. Forbidden: restoring one table, raw SQL, rewriting audit or ledger rows, quantity=0, and a burn+mint across worlds (F3, F5, F6).
8. **Typed remediation record.** New table `game_economy_remediation_records`: remediation_id (UUIDv7), case_id, finding_ref, world_id, proposer and approver (principal, key fingerprint, signature), roster digest, the step plan, and a state of `PROPOSED -> APPROVED -> EXECUTING -> COMPLETED | ABORTED`. Transitions use compare-and-set on a revision. Approval binds the exact plan hash. A changed plan is a new record. A CHECK refuses an `APPROVED`, `EXECUTING` or `COMPLETED` row that lacks either signature or names one principal or one fingerprint twice.
   - Owner ruling §3 Q1 b: proposal and approval need two different holders of the `ECONOMY_REMEDIATOR` role (two-person rule). The owner holds the role until delegating it.
   - **Two people means two authenticated identities, not two field values.** `oteryn-game-ops` runs as root with the one control-plane database credential (`apps/game-server/src/bin/oteryn-game-ops.rs:4-7`), and root can edit host login audit (§2 ruling 3, row B5). Comparing `proposer` and `approver` values typed into that tool proves nothing, so no shared root or database credential may carry an approval.
   - Each role holder has a personal OpenSSH signing key (`ssh-keygen -Y sign`, upstream), kept on that person's own device, never on the node host and never shared. The PROPOSED transition stores the proposer's signature, and the APPROVED transition the approver's, each over `(environment_id, roster_sha256, remediation_id, plan hash)` under the namespace `oteryn-ops-econ-remediation@v1`. The environment id and roster SHA-256 are bound as in §1 ruling 2, so a signature given in preproduction never verifies in production, even for a record restored or copied there with the same ids and plan, and `audit verify` rebuilds the tuple with its own environment id.
   - The role roster is an OpenSSH allowed-signers file with one principal per person. It lives in the private version-controlled deployment location of §2 ruling 7 and changes only by a reviewed revision. The tool refuses a roster in which one key appears under two principals. Each line carries `namespaces=` (§1 ruling 2), and only a principal whose line lists the remediation namespace counts toward two. Each record stores the roster's SHA-256, which is checked against the reviewed revision as the config digest is.
   - The tool verifies both signatures with `ssh-keygen -Y verify` against the roster at approval and again before every step. If the reviewed roster revision changed after approval, the signed `roster_sha256` no longer matches and the next step is refused with `OPS_STAFF_SIGNATURE_INVALID`; both holders re-sign the same plan hash over the new revision, and the record stores the new signatures and roster digest by compare-and-set before execution resumes. The tool refuses unless both verify and the two principals and key fingerprints differ. A root operator holding only their own key cannot reach APPROVED. Root can bypass the tool with the database credential, but cannot forge a signature: `audit verify` re-checks every record's signatures and flags a step with none.
   - **Fail closed before two identities exist.** While the roster holds fewer than two principals with distinct keys, APPROVED is unreachable: the tool refuses with a new 6xxx ops code `OPS_REMEDIATION_SECOND_PERSON_UNAVAILABLE` (leased at packet time), and no remediation burn or mint runs. A raise still needs no approval (ruling 4), so containment works; a lift needs the same two signatures and fails closed the same way (ruling 4).
9. **Restart and partial completion.** Steps commit one by one. Each step reads its record FOR SHARE and commits only while it is `EXECUTING`; an abort takes the record FOR UPDATE, so no step commits after `ABORTED`. There is no hidden atomicity across steps or worlds. After a restart, the executor resumes at the first step that has no receipt. An `ABORTED` record keeps its committed steps as facts, and undoing them needs a new remediation. A step on a character commits only when no live session writes that character: it takes the character write fence at a new session generation, as a login does, so a stale session is fenced out.
10. **Items that moved to other players.** The trace finds every current holder of an excess ItemInstance or quantity, including market escrow and house custody. Excess quantity in a merged stack is burned by quantity, with split/merge conservation. A consumed item cannot be retired; it is recorded as unrecovered value on the record. Owner ruling §3 Q2 a: the item is retired wherever it is; the proceeds are taken back from the exploiter along the ledger; and an innocent holder who paid for it gets a MINT under `IntegrityRemediationCause` for any shortfall, up to the price the receipts show they paid. Each of these is a step of the same approved record. A holder is never sanctioned by remediation alone (F9).
11. **Sanctions are separate.** Account bans and GM actions belong to OPS-GM-01 and an owning enforcement contract (F9). The remediation record may reference a sanction case. It never applies one.
12. **Platform commercial value is Platform's.** Game never changes coin balances, purchase ledgers or entitlements (F10, F11). Game sends Platform a proposal (packet PLATFORM-ECON-PROP-1): a typed incident notice carrying the case id and the affected AccountIds, with Platform deciding reversal, refund or revocation. A revocation reaches Game through the PROD-ENTITLEMENTS-01 consumer fence. If Store delivery later becomes Game-owned, a delivered item is remediated by ruling 7 with the Platform purchase ref as its causation.
13. **Not a disaster restore.** An economy bug is never repaired by a database restore. A restore rolls back every player and needs the recovery register (F12). A full restore happens only when the store is lost or corrupt, under ARCH-ALPHA-OPS-0 §3 ruling 9. After such a restore, open economy cases are re-traced, because restored rows are historical evidence only, and every fence raised and case hold placed after the restore point is raised or placed again by re-apply before any authority opens (§1 ruling 2).

#### B. Data privacy lifecycle

14. **Inventory is a checked artefact.** New `docs/contracts/GAME_PERSONAL_DATA_INVENTORY.json`: one row per table, log stream, file store and event type, with privacy class, retention profile, location and erasure action. The privacy class is exactly one value of the closed ANL-01 §15 set: `INTERNAL_NON_PERSONAL`, `PSEUDONYMOUS_ANALYTICS`, `RESTRICTED_PLAYER_LINKED` or `SECURITY_SENSITIVE`. The erasure action is one or more values of the closed set `TOMBSTONE` (character root, ruling 16), `RETIRE` (value burned under a closed DUR-03 cause), `KEEP_UNLINKED` (kept with the tombstoned CharacterId only), `MARKER` (AccountId replaced by the account deletion marker, ruling 17 step 3), `BLANK` (named columns set to a fixed value, step 3), `DELETE` (step 4 or the character step), `CLEAR` (named columns cleared together), `EXPIRE` (own finite clock, no early delete), `EXCLUDE` (dropped from a derived projection) and `NONE`. Explanations live in the other columns, never in the class. A check in `tools/architecture-check/` fails when a row's class or action is outside its closed set, when a migration creates a table that has no row, when a table with an `account_id` column has no account erasure action, no place in the ordered substitute and delete steps of ruling 17 or no denial trigger of ruling 17, when a column that embeds an AccountId (such as `guard_key`, `change_json` or `operation_json`) has no entry, and when a row of the fixed list of non-table stores below (the erasure journal, node diagnostic logs, crash packages) is missing. Initial rows:

| Data | Where | Class | Retention candidate | Erasure action on account deletion |
|---|---|---|---|---|
| Character root, name, name_key, AccountId link | 0005, 0022 | RESTRICTED_PLAYER_LINKED | While the character lives | `TOMBSTONE`, `MARKER`. AccountId replaced by the account deletion marker (ruling 17 step 3). Name removed. name_key kept 30 days (D183), then deleted. |
| Inventory, items, house custody | 0010-0015, 0025, 0077 | RESTRICTED_PLAYER_LINKED | While the character lives | `RETIRE` with `CharacterRetirementCause` (amendment 1) |
| Receipts, economy ledgers, market and trade rows | 0011-0016, 0023, ECON-RET-0 tables | RESTRICTED_PLAYER_LINKED | While the character lives; game state | `KEEP_UNLINKED` with the tombstoned CharacterId only (ruling 17; owner ruling §3 Q3 a, subject to legal review) |
| Bank operations, ledger entries and balances, keyed by `account_id` | 0071 (`0071_account_bank.sql:52, 114, 145`), written also by 0072 | RESTRICTED_PLAYER_LINKED | Kept (BANK-0 §3) | `RETIRE`, `MARKER`. Balance retired to 0; `account_id` replaced by the account deletion marker (ruling 17 step 3); amounts, chain links and CharacterIds unchanged |
| Reconnect sessions | `0001_admission_reconnect_journal.sql:1-5` | RESTRICTED_PLAYER_LINKED | Kept as session history | `MARKER` (ruling 17 step 3) |
| Reconnect attempts, pending commands, control-loss continuity | `0001:136-144, 179, 76-82` | RESTRICTED_PLAYER_LINKED | While the session lives | `DELETE` (step 4) |
| Fresh admission receipts | `0002_fresh_admission_authority.sql:3-15` | RESTRICTED_PLAYER_LINKED | Kept for replay protection | `MARKER` in `account_id` and `operation_json` (step 3) |
| Admission guard history | `0002:58-67` | RESTRICTED_PLAYER_LINKED | Kept for replay protection | `MARKER` in `guard_key` and `change_json` (step 3) |
| Admission account and character guards | `0002:70-97` | RESTRICTED_PLAYER_LINKED | While the account lives | `DELETE` (step 4) |
| Admission lifecycle receipts | `0002:129-131` | RESTRICTED_PLAYER_LINKED | Kept for replay protection | `MARKER` in `operation_json` (step 3) |
| Character operation receipts | `0005_character_authority.sql:90-93` | RESTRICTED_PLAYER_LINKED | Kept; game state | `MARKER` (step 3) |
| Account achievements and grant requests | `0021_account_achievements.sql:26-47` | RESTRICTED_PLAYER_LINKED | While the account lives | `DELETE` (step 4) |
| Account character projection and outbox | `0024_account_characters_projection.sql:29-39` | RESTRICTED_PLAYER_LINKED | While the account lives | `DELETE` (step 4); the completion to Platform replaces undelivered rows |
| Premium evidence, account and entitlement fences | `0029_premium_entitlement_fence.sql:18-59` | RESTRICTED_PLAYER_LINKED | While the account lives | `DELETE` (step 4) |
| Premium conflict and premium security audit | `0057_premium_semantic_conflict.sql:16-42` | SECURITY_SENSITIVE | Conflict while the account lives; audit kept | `DELETE`, `MARKER`. Delete the conflict; marker in the audit (steps 3-4) |
| Spell premium accounts, entitlements, history and security audit | `0040_spell_premium_projection.sql:3-35` | RESTRICTED_PLAYER_LINKED | Accounts and audit kept; the rest while the account lives | `MARKER`, `BLANK`, `DELETE`. Marker in accounts and audit, with `evidence` and `fingerprint` blanked; delete entitlements and history (steps 3-4) |
| House scope handoffs | `0074_house_scope_handoff.sql:263-271` | RESTRICTED_PLAYER_LINKED | Kept; game state | `MARKER` (step 3); a prepared handoff blocks the step |
| Staff roster | `game_staff_roles` (§1 ruling 4) | RESTRICTED_PLAYER_LINKED | Kept; never deleted | `MARKER` (step 3) |
| `BANK` containment scope rows | ECON-FENCE-1 | RESTRICTED_PLAYER_LINKED | Kept with the case; game state | `MARKER` (step 3); a raised fence blocks the step |
| Functions and triggers that read `account_id` but store no row of their own | 0009, 0022, 0028, 0072 | INTERNAL_NON_PERSONAL | No stored row | `NONE`; listed so the check sees them |
| Account erasure operations `game_account_erasures` | PRIVACY-ERASURE-1 migration | RESTRICTED_PLAYER_LINKED | Kept, so a replayed operation_id returns its completion; no AccountId after completion | `CLEAR`. `account_id` cleared at completion (ruling 17 step 7); operation_id, marker, request digest and deleted-row counts remain |
| Account erasure denials `game_account_erasure_denials` | PRIVACY-ERASURE-1 migration | RESTRICTED_PLAYER_LINKED | Kept while Platform may still deliver a fact for the account; bound fixed by the §3 Q4 legal review | `NONE`. Written by the erasure itself (ruling 17 step 7); holds only the one-way AccountId digest and the operation_id, matchable only by a holder of the AccountId |
| Character, item and economy audit outboxes | 0005, 0010, 0034, 0035, 0079 | RESTRICTED_PLAYER_LINKED | P90D / P30D (F13) | `EXPIRE` on its own clock. No early delete. |
| Mail, inbox and book text | MAIL-TEXT-1 store, 0076 | RESTRICTED_PLAYER_LINKED | While the letter exists; player content | `DELETE` |
| Chat | Memory only (F18); never persisted | RESTRICTED_PLAYER_LINKED | Not persisted | `NONE` |
| Reported chat line | `game_player_reports` (§1 ruling 12) | RESTRICTED_PLAYER_LINKED | 180 days CANDIDATE (WRITE0-RL-03) | `CLEAR` the five evidence columns together (§1 ruling 12 check); the report row keeps the tombstoned CharacterId |
| Guild, VIP, party membership | GUILD-0, VIP-0, 0046 | RESTRICTED_PLAYER_LINKED | While the character lives | `DELETE` the membership. A guild leader follows GUILD-ERASURE-0 (open unknown U3). |
| Highscores | Derived public snapshots | RESTRICTED_PLAYER_LINKED | Current + previous | `EXCLUDE` at once |
| Anti-cheat telemetry, DSR responses | SEC-CLIENT-01 §5.1 | SECURITY_SENSITIVE | SEC-CLIENT-01 RL values (CANDIDATE) | `DELETE` by the SEC-CLIENT-01 purge |
| Crash packages, disconnect forensics | Crash and disconnect baselines | RESTRICTED_PLAYER_LINKED | CANDIDATE, fixed by the §3 Q4 legal review | `DELETE` by AnalyticsActorId |
| Analytics events | ANL-01 | PSEUDONYMOUS_ANALYTICS | Per profile | `DELETE` the pseudonym map entry (amendment 3) |
| Client IP addresses | Transport and logs (F22) | SECURITY_SENSITIVE | None in durable Game tables | `NONE`; packet PRIVACY-LOG-REDACT-1 checks this |
| Node diagnostic logs | Host volume `log/node-*.log` (ARCH-ALPHA-OPS-0 §1 rulings 6-7); no Game player id, but `attempt_ref` is linkable by Platform, so the class is conservative | RESTRICTED_PLAYER_LINKED | 14 days CANDIDATE (ARCH-ALPHA-OPS-0 §1 owner question 2) | `EXPIRE` on its own clock. Game holds no account link. |
| Erasure journal | Host file in the Game restore fence directory, outside the PostgreSQL restore unit (ruling 18; ARCH-ALPHA-OPS-0 §3 ruling 8). Root-only file access; read only by the erasure handler and the restore re-apply step; never exported, logged or copied into the database. Holds each signed erasure request (AccountId, operation_id, environment id, `issued_at`, `erasure_seq`) with Platform's signature, key id and erasure authorization tag (ruling 17), and a `PENDING` or `COMPLETE` state; linkable by Platform; no CharacterId, name or marker | RESTRICTED_PLAYER_LINKED | Each entry until no backup older than it remains and its `erasure_seq` is below the floor of Platform's signed erasure head (ruling 18), on the backup retention clock of ARCH-ALPHA-OPS-0 §3 owner question 1 | `EXPIRE`. It is the erasure record, written `PENDING` before the first database transition and `COMPLETE` after the last (ruling 18); each entry is deleted once its backup bound passes. Not exportable. |

15. **Split with Platform.** Platform holds account PII: email, credentials, payment data, web login IPs and the account profile. Game holds an opaque AccountId and character data only. Game stores no email, real name or payment data. A data subject request enters at Platform. Platform calls a Game domain API for the Game portion and never reads Game SQL (F20). This is proposal PLATFORM-PRIVACY-PROP-1, routed by the control plane.
16. **Account deletion propagation.** Platform sends a typed, idempotent `AccountErasureRequested {operation_id, account_id}` when its own grace ends. The request bytes also carry the Game environment id, `issued_at` and `erasure_seq`, Platform's gapless per-environment sequence number of the request (ruling 18), and Platform signs them with its request-signing key (amendment 2). The node's erasure handler verifies the detached signature against the restart-only verification key file (§2 secret table) before anything else; an unsigned or failing request, or one for another environment id, starts nothing and writes nothing. Only that handler runs an erasure; `oteryn-game-ops` has no erasure command. The erasure start of ruling 17 runs first. Before the first character step, Game ends every live session of the account with the §1 ruling 7 EXT kick and actor `PLATFORM_SECURITY`. Game then runs `ScheduleCharacterDeletion` and `FinalizeCharacterLifecycle` for each character (F19). Order per character: cancel market offers and return escrow; close trade and party state; remove guild and VIP membership, where a guild leader follows the transition that GUILD-ERASURE-0 fixes (open unknown U3); delete player text; retire held items and dust; tombstone the root. After the last character, the account step of ruling 17 retires every bank balance of the account and replaces the AccountId. Each step is idempotent and resumes after a restart from its receipt. Game reports a typed completion to Platform. The engineering bound is the SEC-CLIENT-01 RL-18 30 days (CANDIDATE). A deleted CharacterId is never reused.
17. **Integrity data is kept, not linked.** After finalisation, audit rows remain until their own finite expiry (F13). The economy ledger keeps the tombstoned CharacterId only, with no AccountId and no name, so counterparties' histories and conservation still balance. Owner ruling §3 Q3 a: these rows are kept this way, subject to the external legal review of §3 Q4. An open economy case hold or a privacy legal hold delays only the rows in its scope.
   - **AccountId-keyed rows get an account deletion marker; no value is rewritten.** Bank operations, entries and balances carry `account_id` as a foreign key to `game_character_account_guards` (`0071_account_bank.sql:52, 114, 145`), as the character root does (`0005_character_authority.sql:51`). Bank rows are immutable (`0071_account_bank.sql:1020-1027`), the balance guard refuses an `account_id` change (`0071_account_bank.sql:520`), and the per-(Account, World) chain runs through `previous_entry_id`, not through `account_id` (`0071_account_bank.sql:122`).
   - **One active erasure per AccountId.** New table `game_account_erasures`: operation_id (primary key), account_id, marker, state `RUNNING -> SUBSTITUTING -> COMPLETED`, revision and the deleted-row count per table. `CHECK ((state = 'COMPLETED') = (account_id IS NULL))` and a unique index on `account_id WHERE account_id IS NOT NULL` allow at most one open erasure per account. Transitions use compare-and-set on the revision.
   - **Erasure execution role.** `oteryn-game-ops` runs as root with the one control-plane credential (§2 F6), so no erasure function is granted to the control-plane or runtime group. The PRIVACY-ERASURE-1 migration revokes EXECUTE on `game_account_erasure_begin`, `game_account_erasure_retire` and `game_account_erasure_substitute` from PUBLIC and grants it only to a new login role `oteryn_game_erasure_exec`, a member of neither group, with no other privilege. Its password file (`privacy.erasure_database_password_file`, §2 secret table) is read only by the node's erasure handler. The handler calls the begin function only after it has verified the Platform signature (ruling 16), passed the database authorization check below and fsynced the journal `PENDING` record holding the signed request (ruling 18), and passes the request's SHA-256, Platform key id and the erasure authorization tag below, which the `RUNNING` row stores as NOT NULL columns.
     - **The database verifies the authorization, not the role.** Root can read the execution password file (§2 B5), so holding the role authorizes nothing. Platform attaches to each erasure request, besides its signature, an HMAC-SHA-256 tag under a per-environment erasure authorization key. The tag covers the canonical bytes `"oteryn-account-erasure-v1" || environment_id || operation_id || account_id || issued_at` (each UUID as 16 bytes, `issued_at` as 8-byte big-endian Unix milliseconds).
       - Game holds that key only in the table `game_account_erasure_auth_keys (key_id, environment_id, key, state ACTIVE | RETIRED)`, owned by `oteryn_game_erasure` (NOLOGIN). No other role has any privilege on it, including the execution role, and no host file holds it. The database owner inserts it from Platform's delivery (PLATFORM-PRIVACY-PROP-1, §2 U4); neither the node nor the ops tool ever handles it.
       - `game_account_erasure_begin(environment_id, operation_id, account_id, issued_at, key_id, tag)` recomputes the tag with PostgreSQL's upstream `pgcrypto` `hmac(…, 'sha256')` under the named key of this environment and compares it. A missing tag, an unknown key, a key of another environment or a mismatch raises, and nothing is written. `retire` and `substitute` act only for an operation_id whose `RUNNING` or `SUBSTITUTING` row exists, and only a verified begin creates one. So root holding the execution password alone cannot start, retire or substitute anything.
       - `game_account_erasure_authorize` takes the same arguments, runs the same tag check under the same key rows, and writes nothing (a STABLE SECURITY DEFINER function owned by `oteryn_game_erasure`, executable only by the erasure execution role). It answers only pass or a refusal. The start calls it before the journal `PENDING` record (start step 3), so a request with a bad tag or an unknown, deleted or foreign key is never journaled and blocks nothing.
       - Rotation adds a new `ACTIVE` key, Platform switches, and the old key becomes `RETIRED`; a retired key still verifies a retained journal record at restore re-apply. On suspected compromise the key row is deleted (§2 ruling 14). Platform then re-issues under the new key, with the same operation_id, AccountId and `erasure_seq`, every request it signed under the deleted key and has no completion acknowledgement for; Game appends the re-issue as a replacement record (ruling 18) and the erasure resumes. At restore, a journal record under a deleted key is re-applied only through Platform's current issue of the same request (ruling 18).
       - Residual: whoever holds the database owner credential or the database server's files can read the key. THREAT-MODEL-0 records where they live (recommended: off the node host, as in §1 ruling 2).
     - Detection stays as a second layer. Root cannot forge Platform's signature or tag. `oteryn-game-ops audit verify` (OPS-GM-AUDIT-1) checks every erasure row against a retained journal record whose bytes hash to the stored digest, name the same operation_id and AccountId and verify under the configured Platform key, and flags any other row as an incident. A restore re-applies only journal records whose signature verifies (ruling 18), so restoring a backup taken before an unrecorded erasure brings its rows back. The journal outlives every backup older than it, so such an erasure is detectable for as long as a restore can undo it.
   - **Start.** One transaction, run by the erasure handler on the erasure execution role:
     1. It takes the recovery fence FOR SHARE, then the admission relation fence (`lock_admission_relations`, `apps/game-server/src/durability/db.rs:2991-3020`). That fence is EXCLUSIVE table locks that every guard publication (`apps/game-server/src/durability/admission_authority_guards.rs:830`) and every value writer (`bank.rs:456-457`, `item_transfer.rs:620-621`) already takes first. Two starts, or a start and an admission, never overlap.
     2. It reads the account's erasure rows. A row with another operation_id refuses with `ACCOUNT_ERASURE_IN_PROGRESS {active operation_id}`. A row with the same operation_id resumes it.
     3. With the transaction still open, it calls `game_account_erasure_authorize`. A refusal ends the start with the typed refusal `ACCOUNT_ERASURE_AUTHORIZATION_INVALID` (SQLSTATE leased at packet time); nothing is journaled or written, and Platform may send a corrected request under the same operation_id. Then it appends the journal `PENDING` record, holding the signed request, Platform's signature and the tag, under an exclusive `flock` on the journal file and fsyncs it (ruling 18). If the journal already holds `PENDING` without `COMPLETE` for this account under another operation_id, it refuses with `ACCOUNT_ERASURE_IN_PROGRESS`. Under the same operation_id it appends nothing for the same request bytes, and one replacement record for a re-issue (ruling 18).
     4. It mints the marker (a random UUIDv7, never derived from the AccountId), inserts it as a `game_character_account_guards` row (`0005_character_authority.sql:45-46`), inserts the `RUNNING` row and commits. Both inserts run inside one SECURITY DEFINER function, `game_account_erasure_begin`, owned by `oteryn_game_erasure`, which also stores the request digest and key id. Only the erasure execution role may execute it.
     - **No shortcut on a missing guard row.** An account with no 0005 guard row (it never created a character, or an earlier erasure already ran) can still hold account-only rows that do not reference that guard: premium evidence, fences and conflict (0029, 0057), spell premium rows (0040), and reconnect and admission rows (0001, 0002). The start runs the same steps, mints the marker and inserts the `RUNNING` row. There is no character step and no retirement, and the substitution function runs in full: step 1 locks the original guard row only if it exists, step 5 deletes it only if it exists, and steps 3, 4 and the step 6 catalog scan always run. `COMPLETED` is written only by step 7, after check (a) finds no row. Game never infers completion from a missing row; a new operation_id for an account that holds nothing completes through the same scan with zero counts.
     - **Account-only writers serialize with erasure.** The premium evidence and conflict writers (`apps/game-server/src/durability/premium_fence.rs:198, 218`) and the spell premium projection (`apps/game-server/src/durability/spell_entitlements.rs:119, 153`) write rows that do not reference the 0005 guard, so the step 1 guard lock does not block them. Under PRIVACY-ERASURE-1 each takes the admission relation fence first and refuses an AccountId with an open erasure row with `ACCOUNT_ERASURE_IN_PROGRESS`, writing nothing. Since the start and the substitution hold the same fence, such a write either commits before the start, and is then erased, or is refused. Account achievements need a live character root (`account_achievement.rs:187-190`), which no erased account has. After `COMPLETED` the denial below refuses the same writes with `ACCOUNT_ERASED`, so a fact Platform issued before the request and Game consumes after completion writes nothing. Amendment 2 still binds Platform to emit no such fact once it has sent the request.
     - A crash after `PENDING` and before the commit leaves only the journal record. A retry or a node restart under the same operation_id resumes without a second `PENDING`. Another operation_id is refused until then. Platform retries with the same operation_id (ruling 16). If the key is deleted before the retry, the retry is refused by the authorization check and the record stays open until Platform's re-issue under the new key replaces it, which the compromise rule above requires.
     - The same marker serves every character and World of the account, so the per-account chain and BANK-0's "different accounts" transfer guard still hold.
   - **No new session during erasure.** The guard publication (`admission_authority_guards.rs:808`, upsert at :905-937) reads `game_account_erasures` and `game_account_erasure_denials` after the relation fence and refuses an Account or Character guard change for an account with an open row (`ACCOUNT_ERASURE_IN_PROGRESS`) or a denial (`ACCOUNT_ERASED`). Those reads are the only admission change. Because the start holds the same fence, an admission either commits before the start, and its session is ended by ruling 16, or sees the row and is refused. Each character step takes the character write fence at a new session generation (ruling 9). After completion no Game row holds the AccountId, the denial refuses any guard for it, and Platform never signs a grant for an erased account (a term of PLATFORM-PRIVACY-PROP-1).
   - **Retirement first.** After every character is finalised, for each World with a non-zero balance, a bank operation of the new kind `ERASURE_RETIRE` writes one entry that debits the whole balance to 0 under `AccountErasureCause` (amendments 1 and 4), exactly once per (operation_id, WorldId). Its acting character is the account's most recently created root in that World. That root always exists: a balance arises only from an operation of a character of that World (`acting_character_id` is a NOT NULL foreign key to the roots, `0071_account_bank.sql:51, 123`), and roots are tombstoned, never deleted. The operation carries the erasure operation_id in place of a channel and scope generation. Ordinary character deletion never retires bank gold: the balance belongs to the (Account, World), not to one character.
     - The debit runs only inside the SECURITY DEFINER function `game_account_erasure_retire(operation_id, world_id)`, owned by `oteryn_game_erasure` and executable only by the erasure execution role. The bank operation guard refuses the kind `ERASURE_RETIRE` unless `current_user = 'oteryn_game_erasure'` and a `RUNNING` erasure row exists for that account and operation_id, so neither the runtime nor the control-plane role can burn a balance as an erasure.
     - A raised `BANK` or `CHARACTER_VALUE` fence on the account blocks this debit like any covered write (ruling 2). The erasure waits and shows the fence in its status, and the case owner lifts it or remediates first. This is the case hold delay above, and it counts against the 30-day bound.
   - **Substitution last.** One SECURITY DEFINER function, `game_account_erasure_substitute(operation_id)`, owned by a new NOLOGIN role `oteryn_game_erasure`, does the account step in one transaction. Only the erasure execution role may execute it.
     1. Locks, in this order: the recovery fence FOR SHARE; the admission relation fence; the account's 0005 guard row FOR UPDATE when it exists, which also blocks any insert whose foreign key references it (a root, a bank row, a projection row or a `BANK` fence row); then the erasure row FOR UPDATE. The state moves `RUNNING -> SUBSTITUTING`. Only this transaction ever sees that state.
     2. Preconditions, each read in a later statement: every root of the account is finalised; every bank balance is 0; no reconnect session is nonterminal (`0002_fresh_admission_authority.sql:41-42`); no house handoff is still prepared (state 1, `0074_house_scope_handoff.sql:295`); no raised fence covers the account. Otherwise the function refuses and writes nothing, and the handler retries later.
     3. It replaces `account_id` with the marker, and changes no other column, in: character roots (`0005:51`; the SECURITY DEFINER projection trigger at `0024_account_characters_projection.sql:69-85` skips this change), character operation receipts (`0005:93`), bank operations, entries and balances (`0071_account_bank.sql:52, 114, 145`), reconnect sessions (`0001_admission_reconnect_journal.sql:5`), fresh admission receipts (`0002:6`, with the AccountId in `operation_json` too), guard history (`0002:58-67`, with `guard_key` and `change_json` re-encoded), admission lifecycle receipts (`operation_json`, `0002:129-131`), house handoffs in state 2 or 3 (`0074:271`), the premium security audit (`0057_premium_semantic_conflict.sql:30-31`), spell premium accounts and their security audit (`0040_spell_premium_projection.sql:3-11, 27-29`; the stored `evidence` is set to one fixed byte and `fingerprint` to 32 zero bytes, because both come from the Platform fact), the staff roster (§1 ruling 4) and `BANK` fence scope rows (ECON-FENCE-1).
     4. It deletes, children first: reconnect pending commands, then attempts, then control-loss continuity (`0001:179, 136, 76`); admission character guards, then the account guard (`0002:84, 70`); the projection outbox, then the projection (`0024:34, 29`; undelivered rows are dropped, and the completion to Platform replaces them); the premium entitlement fence, account fence and evidence (`0029_premium_entitlement_fence.sql:51, 43, 18`) and the premium conflict row (`0057:16`); spell premium entitlements and history (`0040:12, 19`); account achievements, then their grant requests (`0021_account_achievements.sql:41, 26`).
     5. It deletes the original 0005 guard row, when it exists.
     6. It checks, and aborts on any failure: (a) a catalog scan over every column named `account_id` in `information_schema.columns`, plus each embedded column the inventory declares (`guard_key`, `change_json`, `operation_json`), finds no row with the AccountId other than this erasure row; audit outbox payloads are excluded, as they expire on their own clock (F13); (b) a per-table SHA-256 over every column it did not substitute or blank, taken before step 3 and again now, is equal; (c) every bank chain of the marker verifies; (d) the deleted-row counts are stored on the erasure row.
     7. It inserts the account's denial row, sets `account_id` to NULL and the state to `COMPLETED`, and commits.
   - **Late facts stay refused after completion.** A `COMPLETED` row no longer holds the AccountId, so the open row cannot fence a fact that arrives later. Step 7 therefore inserts, in the transaction that clears `account_id`, one row into the new table `game_account_erasure_denials (account_digest bytea primary key, operation_id)`, where `account_digest = sha256('oteryn-game-erased-account-v1' || AccountId)`, with ON CONFLICT DO NOTHING. Rows are never updated or deleted, by a guard like the other immutable tables.
     - A BEFORE INSERT OR UPDATE OF `account_id` trigger on every table with an `account_id` column, except `game_account_erasures`, refuses a row whose AccountId digest is denied with the new typed refusal `ACCOUNT_ERASED` (SQLSTATE leased at packet time) and writes nothing. The marker never matches a digest, so the erasure's own writes pass. The writers of embedded AccountId columns (guard history, admission receipts) and the guard publication check the digest after the relation fence. The inventory check fails on an `account_id` table without the trigger (ruling 14).
     - So a premium, entitlement or spell fact that Platform issued before the request and Game consumes after `COMPLETED` writes nothing. Until `COMPLETED` the open row refuses it with `ACCOUNT_ERASURE_IN_PROGRESS`, so no window is left between the two.
     - A restore from a backup taken before step 7 gets the denial back by the journal re-apply, before any authority opens (ruling 18); the ARCH-ALPHA-OPS-0 §3 ruling 9 step 9 premium re-pull runs later, records `ACCOUNT_ERASED` for that account and goes on. A testing-phase reseed loses the denials with the journal (ruling 18).
   - **Guard branches.** The new migration gives every immutability, revision and no-delete guard on a listed table one branch. The branch allows only the action that the inventory lists for that table (`MARKER`, `BLANK`, `CLEAR` or `DELETE`), and only when a `SUBSTITUTING` row exists for that account and `current_user = 'oteryn_game_erasure'`. The guard functions are SECURITY INVOKER (`0005:147`, `0071:683`, `0002:135`, `0074:321` and the 0040 guards), so inside the definer function `current_user` is the erasure role; a runtime, control-plane or erasure execution session never is. The DEFINER projection trigger (0024) cannot use that test, so its branch may only skip work. Every other UPDATE or DELETE is refused as before. Every foreign key to a substituted key, and the balance-to-entry composite key (`0071:151-152`), becomes DEFERRABLE INITIALLY IMMEDIATE, and the function defers them for its one transaction.
   - **Not a ledger rewrite.** Amounts, balances, kinds, entry, transaction, counterpart and chain links and CharacterIds are untouched, so conservation and the chain verify the same afterwards (check b). It is not a correction or compensation (F1): it changes no value.
   - After completion no Game table holds the AccountId, except audit envelopes until their own finite expiry (F13); the only AccountId left in Game is the erasure journal entry (ruling 18), and the denial holds only its one-way digest. A replay of the same operation_id returns the stored completion without any AccountId.
18. **Backups replay erasure.** Each erasure is recorded in an erasure journal in the Game restore fence directory (ARCH-ALPHA-OPS-0 §3 ruling 8), outside the Game PostgreSQL restore unit, next to the Character recovery fence (F12). The journal is append-only. Each `PENDING` record holds the Platform-signed request bytes (AccountId, operation_id, environment id, `issued_at`, `erasure_seq`), Platform's detached signature and key id, and the erasure authorization tag and its key id; a `COMPLETE` record holds the operation_id. No record holds a CharacterId, name or marker: a re-apply finds the characters from the restored roots by AccountId, resumes a restored `game_account_erasures` row with its stored marker, or mints a new marker when the restored database predates the erasure. The journal is an inventory row (ruling 14).
   - Journal first. Before the first database erasure transition (the guard-row marker mint of ruling 17, the character steps and root tombstones of ruling 16, `ERASURE_RETIRE` and the substitution function), Game appends a `PENDING` record and fsyncs the file and its directory. If that fails, the erasure does not start. A crash at any later point therefore leaves the AccountId in the journal.
   - Complete last. After every database phase has committed, Game appends a `COMPLETE` record and fsyncs it. Only then does it acknowledge the erasure to Platform.
   - Retry resumes. A retried request, or a node restart, reads the journal and resumes the erasure from its step receipts (ruling 16). An existing `PENDING` record is not appended again.
   - Replacement. A re-issue of a request, with the same operation_id, AccountId and `erasure_seq`, a later `issued_at` and a current key, that passes the signature and the authorization check is appended as a new `PENDING` record. The newest verifying record of an operation_id is the one a retry, a resume and a restore use; earlier records stay, because the journal is append-only. A re-issue that changes the AccountId or `erasure_seq` is refused and writes nothing.
   - Completeness is anchored at Platform. Each record's signature does not prove that no record was removed, and root controls the restore fence directory. Platform numbers every erasure request it signs for an environment with a gapless `erasure_seq`, and keeps each signed request, in its current issue, until no Game backup older than it remains on the backup retention clock (amendment 2). At restore the erasure handler sends Platform a fresh random nonce and receives a signed head over `"oteryn-erasure-head-v1" || environment_id || nonce || floor_seq || head_seq`, verified against the Platform verification key; a head for another environment or nonce fails. Every `erasure_seq` from `floor_seq` to `head_seq` must have a verifying record whose newest issue passes the authorization check. A gap, including a deleted `PENDING`/`COMPLETE` pair or a journal truncated at a record boundary, stops the restore like an unverifiable record (below). Because the key table rolls back with the database, the database owner first re-provisions from Platform's delivery any current key the restored table lacks (§2 U4).
   - Restore. A restore re-applies every journaled AccountId, `PENDING` or `COMPLETE`, at ARCH-ALPHA-OPS-0 §3 ruling 9 step 7, before any authority opens. The node's erasure handler runs it on the erasure execution role, with authority closed; `oteryn-game-ops restore` starts it and waits, and holds no erasure credential. It first checks completeness against Platform's signed head (above), then verifies each record's signature against the Platform verification key named by the record's key id; the key file keeps retired keys for journal records only, never for a new request. A gap, or a record that cannot be read, parsed or verified, stops the restore before any record is applied, with no node started and admission closed. The run resumes only after the audited repair of ARCH-ALPHA-OPS-0 §3 ruling 9 step 7 appends Platform's current issue of each missing or failing request. An erasure that no Platform request backs has no record, so the restore undoes it (ruling 17, erasure execution role). A non-empty journal refuses the restore if the build selected at step 3 lacks PRIVACY-ERASURE-1. The re-apply ends each erasure at step 7 of the substitution, which restores the denial row. Each phase is idempotent: a step whose receipt exists, a balance already 0 or a row that already holds the marker is skipped. A missing or unreadable journal, or one whose last record is torn, is a refusal: the restore fails closed. Backup copies expire on the backup retention clock of ARCH-ALPHA-OPS-0 §3 owner question 1. A journal entry may be dropped once no backup older than it remains and its `erasure_seq` is below Platform's signed `floor_seq`. A testing-phase reseed (ARCH-ALPHA-OPS-0 §3 ruling 8) loses the journal and the denials with it; the re-decision before external players covers them.
19. **Access and export.** `oteryn-game-ops privacy export` runs as the SEC-CLIENT-01 DSR reader (PRIVACY_DSR purpose). It runs only for a request id that Platform supplies in a request signed like ruling 16, and only on a `PRIVACY_OFFICER` request signed under `oteryn-ops-privacy@v1` (§1 ruling 2). The ops credential can write, so every query runs in one `READ ONLY` transaction. It writes one JSON document covering every inventory row marked exportable. Each run is DURABLE_AUDIT. Responses follow SEC-CLIENT-01 RL-17/RL-23.
20. **Privacy officer role.** DATA-PRIVACY-01 names the role `PRIVACY_OFFICER`. It places, extends and releases privacy legal holds, each by one signed request under `oteryn-ops-privacy@v1` (§1 ruling 2) whose principal, fingerprint and signature the hold row stores for `audit verify`, approves DSR exports, and owns the published disclosure. This unblocks the fail-closed hold rule in SEC-CLIENT-01 (F16). Owner ruling §3 Q4 a: the owner holds `PRIVACY_OFFICER` and obtains one external legal review of the lawful basis categories, the crash default and the retention candidates before external players.
21. **One disclosure.** Before external players, Platform publishes one privacy notice that includes a Game section. Game supplies the section text from the inventory: telemetry without opt-out (SEC-CLIENT-01 R2a), crash upload on by default with opt-out (F17), audit retention, and the fact that chat is not stored except a reported line, kept 180 days (§1 ruling 12). The owner chooses the lawful basis categories after the external legal review (§3 Q4 a).
22. **Before freeze check.** Concurrent transitions: every covered writer holds the fence root FOR SHARE and a raise or lift holds it FOR UPDATE, and records use compare-and-set with two verified signatures (rulings 2, 8). A raise is one signed request (ruling 4). An erasure start and the substitution take the admission relation fence, so one erasure per account runs and no admission overlaps it; the substitution locks the account guard row FOR UPDATE, so no root, bank or fence row can reference the AccountId after it (ruling 17). A staff roster grant locks the character root FOR UPDATE before value writers see the mark (§1 rulings 4, 10). Erasure functions execute only on the erasure execution role, in neither runtime nor control-plane group, and only for a Platform-signed request that the database authorized and the journal recorded first; journal completeness is checked against Platform's signed erasure head at restore (ruling 18); `audit verify` flags any other erasure row (rulings 16-18). A fact consumed after `COMPLETED` is refused by the denial row (ruling 17). States, origins and erasure actions are closed sets, and CHECK constraints refuse a lifted fence or an approved remediation without two distinct signatures (rulings 1, 2, 4, 8, 14). Restart and resume: step receipts (rulings 9, 16). Typed references: CausationRef, case_id and remediation_id. Older clients: no wire change (ruling 3). Commit and recovery across components: per-step commit, the erasure journal written before the first database transition and completed before the Platform completion ack (rulings 9, 16, 18). Restore: verified journal records and the signed raise and hold placement requests are re-applied before any authority opens, which restores denials and fences (rulings 2, 13, 18, amendment 5).

### Contract amendments

1. `docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md` §15, append to the closed cause list:
   > `IntegrityRemediationCause { remediation_id, step_no, case_id, causation: CausationRef }` — BURN or MINT executed only by an `APPROVED` economy remediation record; exactly-once per `(remediation_id, step_no)`; same World as the original transaction.
   > `CharacterRetirementCause { character_id, lifecycle_operation_id }` — BURN of items and dust held by a Character at `FinalizeCharacterLifecycle`; exactly-once per `(lifecycle_operation_id, asset ref)`.
   > `AccountErasureCause { erasure_operation_id, world_id }` — BURN of the whole account bank balance of one (Account, World) by an account erasure, after every Character of the Account is finalised; exactly-once per `(erasure_operation_id, world_id)`.
   > All three causes require full conservation evidence (§26). Neither permits raw deletion.
2. `docs/contracts/CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md` §8, append:
   > Account erasure: Platform sends `AccountErasureRequested {operation_id, account_id}` after its own grace period ends. The request bytes also carry the Game environment id and `issued_at`, and Platform signs them with its request-signing key and attaches an HMAC-SHA-256 erasure authorization tag under a per-environment key that it shares only with the Game database (ARCH-LIVE-READINESS-0 §3 ruling 17); Game verifies the signature, and its database the tag, before Game records or starts anything, and either failure refuses the request. Platform numbers each request with a gapless per-environment `erasure_seq` and keeps each signed request, in its current issue, until no Game backup older than it remains. When it deletes a compromised tag key, it re-issues under the new key, with the same operation_id, AccountId and `erasure_seq`, every request without a completion. At a Game restore it answers a fresh nonce with a signed erasure head (`floor_seq`, `head_seq`) and delivers the current issue of any requested sequence number (ARCH-LIVE-READINESS-0 §3 ruling 18). Game runs at most one erasure per AccountId; another operation_id for the same account is refused with `ACCOUNT_ERASURE_IN_PROGRESS` until the first completes. Game ends the account's live sessions, finalises every Character of that AccountId, and deletes or substitutes every Game row that holds the AccountId: retained Character, bank, receipt and audit rows get one random account deletion marker (values and chain links unchanged), and the rest are deleted. It removes the display name, keeps the name key blocked for the D183 period, and returns a typed completion. Platform shows the account as erased only after that completion, and never signs a pre-admission grant for an erased account. Once it has sent the request, Platform emits no premium, entitlement or achievement fact for that account. Game keeps a one-way digest of an erased AccountId and refuses any later fact for it with `ACCOUNT_ERASED`; Platform treats that refusal as final and discards the delivery.
3. `docs/architecture/ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md` §17, append:
   > On account erasure, the pseudonym map entry for each affected AnalyticsActorId is deleted. Events already recorded remain under their own retention profile, with no path back to the AccountId or CharacterId. The map refuses to create an entry for an AccountId that Game has denied (ACCOUNT_ERASED); an event for it is counted or dropped like any event without a pseudonym, never given a raw id.
4. `docs/architecture/reviews/OTERYN_GAME_BANK0_ACCOUNT_BANK_BALANCE_DECISION_2026-09-30.md` §3, append:
   > Account erasure: bank operation and entry kind `ERASURE_RETIRE`, written only inside `game_account_erasure_retire` for a running erasure, debits a whole (Account, World) balance to 0 under `AccountErasureCause` (DUR-03 §15). Bank operation, entry and balance rows stay immutable except one change: inside `game_account_erasure_substitute`, after every balance of the account is 0, `account_id` is replaced by that erasure's account deletion marker. No other column, amount or chain link changes. Only the erasure execution role can perform either, never the runtime or control-plane role.
5. ARCH-ALPHA-OPS-0 §3 (`docs/architecture/reviews/OTERYN_GAME_ARCH_ALPHA_OPERABILITY_2026-10-06.md`), routed by the control plane to that review's PR, append to ruling 8 (restore fence directory) and ruling 9 step 7:
   > The restore fence directory also holds the retained signed request files of every staff roster revoke, staff mute, economy fence raise and case or privacy legal hold placement (ARCH-LIVE-READINESS-0 §1 ruling 2, §3 rulings 2, 5 and 20). Step 7 first checks the erasure journal against Platform's signed erasure head and verifies the Platform signature of every record; if a sequence number is missing or any record cannot be read, parsed or verified, the restore stops before any record is applied, starts no node and keeps admission closed until an audited repair appends Platform's current issue of each missing or failing request (ARCH-ALPHA-OPS-0 §3 ruling 9 step 7), because a skipped erasure would bring erased state back. Step 7 then re-applies every erasure in the journal, idempotent by operation_id, and replays the retained revoke, mute, raise and hold placement requests by the replay rule of ARCH-ALPHA-OPS-0 §3 ruling 8: only one with a retained `COMMITTED` outcome record and no stored outcome in the restored database; a request file with no outcome record stops the restore until a signed resolution. No selection compares `issued_at` or any other clock with T. No grant, unmute, lift or hold release is re-applied. A missing or unreadable directory refuses the restore. Step 9 records an `ACCOUNT_ERASED` refusal for an account and continues.

### Packets

| Id | Owned paths | Tests | Deps / order |
|---|---|---|---|
| ECON-FENCE-1 | apps/game-server/migrations/NNNN_economy_containment_fence.sql (new); apps/game-server/src/durability/economy_fence.rs (new); fence checks in apps/game-server/src/durability/{item_fee_burn.rs, bank.rs, item_mint_audit.rs …}; apps/game-server/src/bin/oteryn-game-ops.rs (signed raise and lift, using the OPS-GM-AUDIT-1 signature module and roster digest) | The ruling 2 interleavings (a)-(c) on two connections: a transfer that reaches the root after a raise is refused and leaves nothing, a transfer that held it first commits before the raise; the fence persists across a restart; an unreadable fence refuses; an older client gets the existing refusal; a lift with one signature, one principal signing twice, one key under two principals, a remediation-namespace signature, a signature over an older fence revision, a signature made for another environment id or roster SHA-256, or a roster with fewer than two principals is refused and the fence stays raised and still refuses covered writes; a lift with two valid signatures from distinct principals commits once; an unsigned raise, or a raise signed under another namespace, is refused; every committed raise and lift has one `STAFF_ACTION` event (`ECON_FENCE_RAISE` or `ECON_FENCE_LIFT`) with the case id, scope and signers in the same transaction, and a failed audit insert rolls the fence change back; a direct UPDATE to `LIFTED` without two distinct signatures, and an UPDATE of a `LIFTED` row back to `RAISED`, are refused by the CHECK and the guard; a restore from a backup taken before a raise raises the fence again by re-apply, and a second re-apply changes nothing | First, after the OPS-GM-AUDIT-1 signature module |
| ECON-CASE-HOLD-1 | migration (new) for item and economy hold tables; apps/game-server/src/durability/ (new module) | A held row survives expiry; a release returns the row to its original expiry; an unsigned place or release, or one signed under another namespace or by a principal whose roster line lacks `oteryn-ops-econ-hold@v1`, is refused with nothing written; a restore to before a placement places the hold again | With FENCE-1 |
| ECON-TRACE-1 | apps/game-server/src/bin/oteryn-game-ops.rs (`economy trace`, read-only) | A fixture duplicate traces across trade, market escrow and stack merge; a write attempted inside the trace transaction fails (`READ ONLY`) | After FENCE-1 |
| ECON-REMEDIATION-1 | DUR-03 amendment 1; migration (new) `game_economy_remediation_records`; apps/game-server/src/durability/economy_remediation.rs (new); apps/game-server/src/bin/oteryn-game-ops.rs (approval, using the OPS-GM-AUDIT-1 signature module and roster digest) | Two-person rule: one principal signing both, one key under two principals, a signature over another plan hash, a signature made for another environment id (a preproduction signature on a production record with the same ids and plan, also in `audit verify`), a signature over another roster SHA-256 and a roster with fewer than two principals and a principal whose roster line lacks the remediation namespace are each refused with nothing written; `audit verify` flags a step with no valid signature; a changed plan is rejected; replaying a step mints nothing; a roster revision change after approval refuses the next step until both re-sign; resume after a crash between steps; a cross-world step is refused; a direct UPDATE to `APPROVED` without two distinct signatures is refused by the CHECK; a step racing an abort on two connections either commits before `ABORTED` or is refused | After TRACE-1 |
| PRIVACY-INVENTORY-1 | docs/contracts/GAME_PERSONAL_DATA_INVENTORY.json (new); tools/architecture-check/ | A migration table with no row fails the check; a table with an `account_id` column and no account erasure action fails; a table with an `account_id` column and no place in the ordered steps of ruling 17 fails; a missing entry for an embedded AccountId column fails; removing the erasure journal row fails; a class outside the four ANL-01 §15 classes, or an action outside the closed erasure action set, fails; a table with an `account_id` column and no `ACCOUNT_ERASED` denial trigger fails | Independent, first in B |
| PRIVACY-LOG-REDACT-1 | apps/game-server/src/gameplay_transport/, crates/diagnostics/ | A log-capture test asserts that no IP, AccountId or name appears (ANL-01 §18) | Independent |
| PRIVACY-ERASURE-1 | Amendments 1-4; apps/game-server/src/durability/character_authority.rs; apps/game-server/src/durability/bank.rs (`ERASURE_RETIRE`); the erasure refusal in apps/game-server/src/durability/{premium_fence.rs, spell_entitlements.rs}; migration (new) for the tombstone, `game_account_erasures`, `game_account_erasure_denials` and its `ACCOUNT_ERASED` trigger on every `account_id` table, the `ERASURE_RETIRE` kinds, the `oteryn_game_erasure` role, the `oteryn_game_erasure_exec` login role, the `pgcrypto` extension and `game_account_erasure_auth_keys`, the begin function's tag check and the non-mutating `game_account_erasure_authorize`, the begin, retire and substitution functions with EXECUTE revoked from PUBLIC, the guard branches and the deferrable foreign keys; the node erasure handler with Platform request verification and the `privacy.erasure_database_password_file` and `platform.request_verification_key_file` keys; the erasure and denial reads in the guard publication (apps/game-server/src/durability/admission_authority_guards.rs); the erasure row check in `oteryn-game-ops audit verify`; the erasure journal file in the restore fence directory (ARCH-ALPHA-OPS-0 §3 ruling 8), with replacement records and the completeness check against Platform's signed erasure head | Idempotent replay; resume mid-sequence; an erased guild leader follows the accepted GUILD-ERASURE-0 transition; a restore re-applies the journal; an account with balances in two Worlds and a transfer with another account: after completion no root, operation, entry or balance row and no guard row holds the AccountId, every balance is 0, both accounts' chains and the counterpart entries verify, and the before/after digest is equal; a runtime-role UPDATE of `account_id`, a substitution while a balance is non-zero or a character is not finalised, and a changed value column inside the function are each refused with nothing written; the journal holds no CharacterId; a failed journal append or fsync starts no database transition; crash injection at each point: after the `PENDING` fsync and before the first database phase, between each pair of database phases (marker mint, each character step and root tombstone, `ERASURE_RETIRE` per World, substitution), after the last phase and before `COMPLETE`, and after `COMPLETE` and before the Platform acknowledgement; in every case a later restore from a backup taken before the erasure leaves no AccountId linked in any Game table, and a retry converges to one completion with no second `PENDING` record; a restore with a torn last record or an unreadable journal is refused; two concurrent requests with different operation_ids for one account on two connections leave one open erasure row and one `PENDING` record, and the second gets `ACCOUNT_ERASURE_IN_PROGRESS`; an admission racing the start either commits first and its session is kicked, or is refused; a substitution with a live session, a prepared handoff or a raised fence is refused; the projection rows are not recreated by the substitution; a runtime-role or control-plane-role session cannot use a guard branch, even while a `SUBSTITUTING` row exists; a planted AccountId in an unlisted column fails the catalog scan and nothing is written; the deleted-row counts match; the 0040 `evidence` and `fingerprint` are blanked; an account that never created a character but holds premium evidence, a premium fence and spell premium rows (0029, 0040, 0057) has no 0005 guard row, and its erasure runs the substitution and the catalog scan and leaves none of those rows with the AccountId before `COMPLETED`; a premium evidence write racing the start on two connections either commits first and is erased, or is refused with `ACCOUNT_ERASURE_IN_PROGRESS`; a call to the begin, retire or substitution function from a runtime-role or control-plane-role session is refused, and an `ERASURE_RETIRE` without a `RUNNING` row for that operation_id is refused; an unsigned request, a bad signature, an unknown key id or another environment id writes no `PENDING` record and no row; a validly signed request with a wrong tag, or a tag under an unknown, deleted or foreign key, is refused with `ACCOUNT_ERASURE_AUTHORIZATION_INVALID`, writes no `PENDING` record and no row, and does not block a later request for the account; a key deleted between the authorization check and begin leaves `PENDING`, Platform's re-issue under the new key appends one replacement record and the erasure completes; a re-issue that changes the AccountId or `erasure_seq` is refused with nothing written; a restore after one `PENDING`/`COMPLETE` pair is deleted, or after the journal is truncated at a record boundary, stops at step 7 before any record is applied and, once the missing requests are appended from Platform, re-applies them before authority opens; a head for another environment, a replayed head with an old nonce, or an unreachable Platform stops the restore at step 7; a begin call on the execution role with no tag, a wrong tag, a tag under an unknown or deleted key or under another environment's key writes nothing, and a retire or substitute call for an operation_id with no `RUNNING` or `SUBSTITUTING` row writes nothing; the execution role cannot read `game_account_erasure_auth_keys`; a journal record under a `RETIRED` key is re-applied at restore; a premium fact, an entitlement fact and a spell premium fact issued before the request and consumed after `COMPLETED` are each refused with `ACCOUNT_ERASED` with nothing written, and so is a guard publication for the account; a restore from a backup taken before the erasure re-creates the denial before authority opens and the premium re-pull then writes nothing for the account; a journal record with a bad signature, or one that cannot be parsed, stops the restore before any record is applied, starts no node and keeps admission closed, and the run completes once an audited repair appends Platform's current issue of the request; `audit verify` flags an erasure row with no matching verified journal record | After INVENTORY-1, GUILD-ERASURE-0, PLATFORM-PRIVACY-PROP-1 and DATA-RESTORE-OPS-2 (journal re-apply) |
| PRIVACY-DSR-EXPORT-1 | apps/game-server/src/bin/oteryn-game-ops.rs (`privacy export`, signed privacy legal hold place, extend and release) | Export covers every exportable inventory row; refused without a Platform-signed request id or without a `PRIVACY_OFFICER` signature under `oteryn-ops-privacy@v1`; a write inside the export transaction fails (`READ ONLY`); an unsigned hold change is refused with nothing written; the run is audited | After INVENTORY-1 and the OPS-GM-AUDIT-1 signature module |
| GUILD-ERASURE-0 | Decision only: an amendment to GUILD-0 §3.5 (`docs/architecture/reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md:209-210`), written by that decision's owner | The amendment names one transition for an erased leader that completes within the 30-day erasure bound and never waits on the leader | Before PRIVACY-ERASURE-1; routed by the control plane |
| PLATFORM-PRIVACY-PROP-1 / PLATFORM-ECON-PROP-1 | Proposals only, routed by the control plane to Oteryn-Platform; PLATFORM-PRIVACY-PROP-1 includes the signed erasure and DSR requests, the request-signing key and its delivery (§2 U4), the erasure authorization tag and the delivery of its key to the Game database owner, the gapless `erasure_seq`, the retention and current-issue delivery of signed erasure requests, the re-issue after a tag key deletion and the signed erasure head (§3 ruling 18), and final handling of `ACCOUNT_ERASED` (amendment 2) | Platform side | Parallel; PLATFORM-PRIVACY-PROP-1 before PRIVACY-ERASURE-1 |

### Owner questions

1. Who may approve an economy remediation (lift a fence, burn or mint for repair)? a) The owner only. b) A named `ECONOMY_REMEDIATOR` role that the owner appoints, under the two-person rule, held by the owner until delegated. c) Any GM. **Recommend b.**
   - **Owner ruling 2026-10-06: b.** The owner holds the role until delegating it. Two people are two personal signing keys on a reviewed roster; with fewer than two, remediation fails closed (ruling 8).
2. An innocent player holds or paid for a duplicated item. a) Retire the item wherever it is, take back the proceeds from the exploiter along the ledger, and mint compensation for any shortfall up to what the holder paid. b) Retire it everywhere with no compensation. c) Retire it only from the exploiter. **Recommend a.**
   - **Owner ruling 2026-10-06: a.** Applied in ruling 10.
3. After account erasure, economy ledger and receipt rows: a) keep them with only the tombstoned CharacterId (no AccountId, no name) for integrity. b) delete them after a fixed period and record aggregate totals. **Recommend a, subject to legal review.**
   - **Owner ruling 2026-10-06: a, subject to the legal review of Q4.** Applied in ruling 17.
4. Privacy officer and lawful basis: a) the owner holds `PRIVACY_OFFICER` and gets one external legal review of the lawful basis categories, the crash default and the retention candidates before external players. b) the owner appoints someone else, with the same review. **Recommend a.**
   - **Owner ruling 2026-10-06: a.** Applied in rulings 20 and 21.

### Rejected options

- Restoring one table or a point in time to undo a duplicate: this breaks conservation and rolls back innocent players (F1, F12).
- Hand edits or raw SQL with a ticket: forbidden as correction (F3).
- Detectors that auto-remediate: forbidden (F2).
- Freezing by forcing logouts: a value freeze already contains the damage without touching sessions.
- The execution role as the only erasure authorization, or its password kept out of root's reach on the node host: root can read any host file, so only a check inside the database can refuse an unrequested erasure (ruling 17).
- Hard-deleting every row that carries a CharacterId on erasure: this breaks counterparties' ledgers and replay protection (DUR-03 one-item retention decision).
- Game handling coin refunds or entitlements: these are Platform's (F10).
- One atomic remediation across characters or worlds: no hidden cross-component atomicity, and no cross-world value (F6).

### Open unknowns

- U1 (CONFLICT, F23): DUR-03 and ANL-03 headers still say CANDIDATE. The control plane should reconcile the headers with the status register.
- U2 (UNKNOWN, F22): whether client IPs reach logs. PRIVACY-LOG-REDACT-1 settles it.
- U3 (CONFLICT, a dependency of PRIVACY-ERASURE-1): GUILD-0 §3.5 refuses to delete a guild leader until it resigns or disbands (F21; `docs/architecture/reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md:209-210`), but erasure cannot wait on the leader indefinitely. This document does not amend GUILD-0. Decision packet GUILD-ERASURE-0 resolves it, and PRIVACY-ERASURE-1 does not start before that amendment is accepted. Proposal for the GUILD-0 owner: erasure forces a resign to a vice by the GUILD-0 resign rule (§3.2), or, with no eligible vice, disbands the guild by §3.4.
- U4 (DERIVED tension): D183 keeps the name key for 30 days after deletion. The §3 Q4 legal review should confirm this.
- U5 (CANDIDATE): retention values for crash packages, forensic slices and SEC-CLIENT-01 RL values. Fixed by the §3 Q4 legal review.
- U6: SEC-CLIENT-01, ECON-RET-0 and PROD-ENTITLEMENTS-01 are CANDIDATE. These rulings depend on their shapes, not their final numbers.
- U7: Store delivery ownership is open (F10). Ruling 12 covers both outcomes.
- U8 (DERIVED): the erasure denial keeps a one-way digest of an erased AccountId for as long as Platform may deliver a fact for it (ruling 17). The §3 Q4 legal review should confirm the retention; until then the digest is kept, because dropping it lets a late fact relink the account.

## 4. Checklist and owner items

### 4.1 Before-freeze checklist

This list consolidates §1 ruling 14, §2 ruling 15 and §3 ruling 22.

- Concurrent transitions:
  - the release, sanction compare-and-set, report compare-and-set and roster re-read (§1);
  - maintenance, signal and assignment revoke all feed one shutdown token, and the first wins (§2);
  - every covered value writer holds the fence root FOR SHARE and a raise or lift holds it FOR
    UPDATE, so no covered write commits after a raise; records use compare-and-set (§3);
  - a roster grant locks the character root FOR UPDATE, so a value writer either commits before
    the mark or sees it and is refused (§1 rulings 4 and 10);
  - an erasure start and the substitution take the admission relation fence, so one erasure runs
    per account and no admission overlaps it; the substitution locks the account guard row FOR
    UPDATE, so no new row can reference the AccountId (§3 ruling 17);
  - an account-only fact is refused by the open erasure row until `COMPLETED` and by the denial
    row after it, so no late fact relinks an erased AccountId (§3 ruling 17).
- Restart and resume:
  - staff rows are durable, and a kick needs no resume state (§1);
  - the request file and operation id make every ops command replay-safe;
  - a node that died mid-drain restarts not ready, because it reads the durable maintenance
    marker at boot and treats an unreadable marker as open (§2);
  - remediation and erasure steps keep receipts (§3);
  - a restore replays the retained signed revoke, mute, raise and hold placement requests
    with a retained committed outcome record and no stored outcome in the restored database,
    and stops on a request file with no outcome record until a signed resolution, never a grant, unmute, lift or release, before any authority opens (§1 ruling 2, §3
    ruling 2, amendment 5).
- Typed cross-record references: `basis_refs`, `StaffActionRef`, the roster revision,
  `RuntimeScopeRefV1`, `CausationRef`, `case_id` and `remediation_id`.
- Older client and peer gating:
  - `STAFF_V1` and `SERVICE_NOTICE_V1` are never offered to older clients;
  - an older client receiving 1200 or 1201 uses the unknown-code fallback;
  - economy and privacy add no wire change.
- Multi-component commit and recovery:
  - the node commits a staff row and its audit in one transaction before any effect (§1);
  - Platform policy and the Game drain are independent, and either order stops new sessions (§2);
  - each remediation step commits on its own (§3);
  - the erasure journal records `PENDING` durably before the first database transition and
    `COMPLETE` after the last; Platform is acknowledged only after `COMPLETE`, and a restore
    re-applies every journaled AccountId, which restores its denial, after checking the journal
    against Platform's signed erasure head; it fails closed on a gap, an unverifiable record or a
    torn or unreadable journal, and a request the database would refuse is never journaled
    (§3 rulings 17 and 18).
- No new authority beyond the Game staff roster (§1 ruling 4) and the `ECONOMY_REMEDIATOR` and
  `PRIVACY_OFFICER` roles (§3); a remediation approval and a fence lift each need two personal
  signatures, never a shared root or database credential (§3 rulings 4 and 8).
- Every staff, roster, fence, case hold, privacy hold, export and remediation request is signed
  by a personal key under its own namespace, verified by the committing process and stored for
  `audit verify` (§1 ruling 2). Platform-security kicks, erasures and exports need a Platform
  signature over the request bytes, including the environment id (§1 ruling 7, §3 ruling 16).
- Erasure functions execute only on the erasure execution role, a member of neither the runtime
  nor the control-plane group, whose credential the ops tool never holds. The begin function
  starts nothing without a Platform tag it verifies under a key that only its owner role can
  read, so the execution password alone authorizes nothing; `audit verify` flags an erasure row
  with no verified journal record (§2 secret table, §3 ruling 17).
- Every stored state, origin, action kind, reason code, privacy class and erasure action is a
  closed set, and an unknown value is refused or fails the check (§1 ruling 11, §3 rulings 1, 2
  and 14).
- The erasure guard branches work only inside the erasure function's own role and only for an
  open `SUBSTITUTING` row; no runtime or control-plane session can use them (§3 ruling 17).

### 4.2 Owner items

The owner ruled on every item on 2026-10-06. Each ruling is recorded here and under the owner
questions of its section, and the body applies it.

| Item | Question | Options | Recommendation | Owner ruling |
|---|---|---|---|---|
| §1 Q1 | Source of staff authority | a Game roster via `oteryn-game-ops`; b Platform staff claim (cross-repo) | a | Owner ruling 2026-10-06: a |
| §1 Q2 | Where player bug reports go | a out-of-band channel, in-game rule-violation reports for EXT; b in-game bug queue; c Platform support desk | a | Owner ruling 2026-10-06: a, refined: bug reports via a GitHub bug-report issue form only; exploits via GitHub private vulnerability reporting, whose enabled state OPS-GM-REPORT-3 verifies first; Discord only when external testers arrive; OPS-GM-REPORT-3 delivers the form |
| §1 Q3 | Staff action audit retention | a 1 year; b 2 years; c 90 days | a | Owner ruling 2026-10-06: a |
| §1 Q4 | Reported chat line in harassment reports | a no; b one chosen line kept 180 days | b for EXT | Owner ruling 2026-10-06: b |
| §2 Q1 | Who triggers the maintenance drain | a Game operator locally; b Platform endpoint; c both | a | Owner ruling 2026-10-06: a |
| §2 Q2 | Network DDoS protection | a host-included; b paid L4 proxy (spend); c none until public release | a | Owner ruling 2026-10-06: a |
| §2 Q3 | External security review | a internal only; b one paid review; c bug bounty | a | Owner ruling 2026-10-06: a |
| §3 Q1 | Who approves economy remediation | a owner only; b `ECONOMY_REMEDIATOR`, two-person; c any GM | b | Owner ruling 2026-10-06: b |
| §3 Q2 | Innocent holders of duplicated items | a retire everywhere, compensate up to the price paid; b no compensation; c exploiter only | a | Owner ruling 2026-10-06: a |
| §3 Q3 | Ledger rows after erasure | a keep with tombstoned CharacterId; b delete after a period | a, subject to legal review | Owner ruling 2026-10-06: a, subject to the §3 Q4 legal review |
| §3 Q4 | Privacy officer | a owner, plus one external legal review; b appointee | a | Owner ruling 2026-10-06: a |
