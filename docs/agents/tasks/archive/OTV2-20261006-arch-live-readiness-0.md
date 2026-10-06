# OTV2-20261006-arch-live-readiness-0

```yaml
task_id: OTV2-20261006-arch-live-readiness-0
title: "ARCH-LIVE-READINESS-0: before external players (GM and support, threat model and live operations, economy incidents and privacy)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: cand/arch-live-readiness
issue: 162
pr: 1879
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01WQyZ8BUWVpmDLpSTpXHvn1 (Sol Supervising Architect)
created_at: 2026-10-06
updated_at: 2026-10-06
execution_policy: continuous_progress
owned_paths: [docs/architecture/reviews/OTERYN_GAME_ARCH_LIVE_READINESS_2026-10-06.md, docs/agents/tasks/archive/OTV2-20261006-arch-live-readiness-0.md]
public_contracts: []
depends_on: []
blocks: [OPS-GM-AUDIT-1, OPS-GM-SANCTION-2, OPS-GM-REPORT-3, OPS-GM-ROSTER-4, OPS-GM-STAFF-5, OPS-GM-PREPORT-6, THREAT-MODEL-0, ERR-PUBLIC-REVIEW-1, LIVEOPS-CONFIG-AUDIT-1, LIVEOPS-MAINT-1, LIVEOPS-NOTICE-2, LIVEOPS-KILL-2, THREAT-DOS-1, SEC-ROTATION-1, SUPPLY-SIGN-1, ECON-FENCE-1, ECON-CASE-HOLD-1, ECON-TRACE-1, ECON-REMEDIATION-1, GUILD-ERASURE-0, PRIVACY-INVENTORY-1, PRIVACY-LOG-REDACT-1, PRIVACY-ERASURE-1, PRIVACY-DSR-EXPORT-1]
cross_repository_coordination_id: null
external_repositories: []
last_progress: "2026-10-06 review round 6: Codex findings 4197873469 (erasure without a guard row), 4197873478 (signatures bound to environment and roster) and 4197873488 (grant account pairing) fixed, with the deferred P2 4196701864 (drain order) and 4196701873 (report evidence check); review round 4: Codex findings 4197087993 (staff value boundary), 4197088008 (ordered per-table erasure), 4197088013 (signed operator requests) and 4197088022 (one active erasure per account) fixed; round 3: Codex finding 4196701855 fixed (erasure journal PENDING before the first database transition, COMPLETE before the Platform ack); rounds 1 and 2 fixed earlier; owner rulings of 2026-10-06 on all eleven §4.2 items recorded and applied"
```

## Outcome
- Owner ruling 3b of 2026-10-06: the "before external players" package is authored now.
- §1 GM and support: a Game staff roster written with `oteryn-game-ops`; typed sanctions with compare-and-set; staff kick as protocol code 1201 SESSION_STAFF_DISCONNECT; staff character isolation; one `STAFF_ACTION` event type for staff audit; player reports; no in-game GM commands until `STAFF_V1`.
- §2 Threat model and live operations: one living threat model and boundary inventory; the deferred ERR-CODES §1.9 display item; config audit; maintenance drain as protocol code 1200 SESSION_SERVICE_MAINTENANCE (TERMINAL); `SERVICE_NOTICE_V1` countdown; capability kill switch; secrets as files read once; rotation as node replacement; compromise response.
- §3 Economy incidents and privacy: an ANL-03 case and a typed `EconomyContainmentFence`; remediation as a compensating DUR-03 transaction with two-person approval; never a restore; `GAME_PERSONAL_DATA_INVENTORY.json`; the Platform/Game PII split; typed account erasure from Platform; an erasure journal in the restore fence directory of ARCH-ALPHA-OPS-0 §3.
- Contract amendments are exact text, pending and applied by the named packets. Platform proposals bind nothing. The eleven owner items of §4.2 carry the owner rulings of 2026-10-06.
- The document stays a proposed architecture decision until merge and grants no runtime or production authority.

## Acceptance criteria

- Each of §1-§3 has facts with evidence, rulings, exact contract amendment text, packets with owned paths, tests and order, owner questions, rejected options and open unknowns; §4 has the before-freeze checklist and the owner items.
- Containment serializes with value writers under READ COMMITTED through the fence root row lock, and a deterministic two-connection test proves that a covered write reaching the root after a raise cannot commit (§3 ruling 2).
- Remediation approval needs two distinct authenticated people, by personal signing keys on a reviewed roster, never a shared root or database credential, and fails closed while fewer than two exist (§3 ruling 8).
- PRIVACY-ERASURE-1 depends on GUILD-ERASURE-0 (U3) in the brief order and the packets table (§3 U3).
- The maintenance marker is durable in Game PostgreSQL, read at boot before readiness, and an open or unreadable marker keeps the node not ready (§2 ruling 9).
- Lifting a fence needs the same two personal signatures as a remediation approval, under its own namespace, and fails closed with fewer than two principals (§3 ruling 4).
- Erasure leaves no AccountId in any Game table except audit envelopes until expiry: bank balances are retired to 0, then one guarded function substitutes a random marker or deletes, in a fixed per-table order, every row that holds the AccountId, with a before/after digest of the untouched columns and a catalog scan that proves none is left (§3 ruling 17, amendment 4).
- Each AccountId table has one inventory row with one exact action and step, and the inventory check fails for a table or embedded column with no ordered step (§3 rulings 14 and 17).
- At most one erasure per AccountId is open; the start takes the admission relation fence, so two starts or a start and an admission never overlap, and a crash after `PENDING` resumes under the same operation_id (§3 ruling 17).
- The erasure guard branches work only inside the erasure role's definer function for an open `SUBSTITUTING` row (§3 ruling 17).
- A staff-marked character's value never crosses its own boundary: Ground drop and pickup, corpse loot, map pickup, spell overflow, bank, Inbox, kill rewards and party settlement are refused or skipped at the writer, under the root lock that a roster grant also takes (§1 rulings 4 and 10).
- Every alpha staff, roster, fence and remediation request carries a personal OpenSSH signature under its own namespace, verified by the committing process and stored for `audit verify`; host login audit is not the attribution (§1 ruling 2, §2 row B5).
- The erasure journal holds AccountId and operation_id only and is an inventory row that the inventory check requires (§3 rulings 14, 18).
- The erasure journal records a durable `PENDING` entry before the first database erasure transition and `COMPLETE` after the last and before the Platform acknowledgement; a restore re-applies every journaled AccountId idempotently and fails closed on a torn or unreadable journal; PRIVACY-ERASURE-1 tests a crash at each point (§3 ruling 18).
- A reported chat line must have been spoken by the report target, enforced in the command and by a table check (§1 ruling 12).
- An account with no 0005 guard row runs the full erasure path and completes only after the catalog scan; premium and spell premium writers refuse an open erasure under the admission relation fence (§3 ruling 17).
- Staff, lift and remediation signatures cover the environment id and the roster SHA-256, and the verifier uses its own restart-only environment id (§1 ruling 2, §3 rulings 4 and 8).
- A staff grant requires a live root of the grant's own account, enforced in the tool and by a composite foreign key (§1 ruling 4).
- Maintenance close stops reading commands, lets in-flight passes finish and deliver their results, and only then sends 1200 (§2 ruling 9).
- Every owner ruling of 2026-10-06 is recorded in its section and in §4.2, and the body is consistent with it.

## Review findings

- 4195149410 (P1, §3 ruling 2): fence reads alone did not serialize with a raise under READ COMMITTED. Fixed: a singleton fence root row, held FOR SHARE by every covered writer after the recovery fence and FOR UPDATE by a raise or lift, modelled on the recovery admission row; tests (a)-(c) are deterministic interleavings on two connections.
- 4195149418 (P1, §3 ruling 8): the two-person rule compared values typed into a root tool. Fixed: personal OpenSSH signing keys per `ECONOMY_REMEDIATOR` holder, an allowed-signers roster in the reviewed deployment location with its digest recorded, verification at approval and before every step, distinct principals and keys, and `OPS_REMEDIATION_SECOND_PERSON_UNAVAILABLE` with fewer than two principals.
- 4195149435 (P1, §3 packets): the guild-leader case was a test, not a dependency. Fixed: decision packet GUILD-ERASURE-0 (a GUILD-0 §3.5 amendment) precedes PRIVACY-ERASURE-1 in the brief order, the packets table, U3 and `blocks`.
- 4195977709 (P1, §3 ruling 4): a single holder could lift a fence. Fixed: a lift needs two `ECONOMY_REMEDIATOR` signatures over `(fence_id, fence revision, case_id)` in a lift namespace, verified with the roster digest, committed by compare-and-set under the root FOR UPDATE, fail closed; ECON-FENCE-1 tests.
- 4195977719 (P1, §3 ruling 17): bank rows keyed by AccountId had no erasure rule. Fixed: `ERASURE_RETIRE` under a new `AccountErasureCause`, then one SECURITY DEFINER substitution of a random marker for `account_id` in root and bank rows with no value or chain change and an equal value digest; BANK-0 amendment 4; inventory rows for every AccountId-keyed table; PRIVACY-ERASURE-1 tests.
- 4195977731 (P2, §3 rulings 14, 18): the erasure journal was not inventoried. Fixed: journal reduced to AccountId and operation_id, inventory row with classification, access and backup-bound retention, and the check requires it; `game_account_erasures` row added.
- 4195977740 (P2, §1 ruling 12): a report could attach a line by another speaker. Fixed: the buffer records the speaker, `CHAT_LINE_TARGET_MISMATCH` refuses a line not spoken by the target or a non-HARASSMENT revision, and a table check binds the stored speaker to the target; OPS-GM-PREPORT-6 tests.
- 4196701855 (P1, §3 ruling 18): the journal was ordered only before the Platform acknowledgement, so a crash after substitution and before the append left a restorable backup with no journaled AccountId. Fixed: a `PENDING` record is appended and fsynced with its directory before the first database transition and `COMPLETE` after the last; retry resumes from the journal; restore re-applies every journaled AccountId per idempotent phase and refuses a torn or unreadable journal; PRIVACY-ERASURE-1 crash-injection tests at each point; inventory row and brief updated.
- 4197087993 (P1, §1 ruling 10): the dispatch check missed Ground drop and pickup. Fixed: isolation is a holder boundary at the writers. `ItemBoundary::{Internal, Crossing}` on the item paths (item_transfer.rs, equipment, reward claim, map and spell mints), `acting_fence` in bank.rs, `OTI06` in Inbox delivery, no loot or experience from a staff kill, and party settlement skips a member marked after joining; a roster grant locks the root FOR UPDATE; OPS-GM-ROSTER-4 tests, including a two-connection race and a compile-time boundary.
- 4197088008 (P1, §3 rulings 14 and 17): the action per AccountId table was an open choice. Fixed: one inventory row per table from the migrations with one action and step; one ordered transaction substitutes, blanks or deletes children first, then checks by catalog scan, digest and chain verify; guard branches limited to the erasure role; the inventory check fails a table or embedded column with no ordered step.
- 4197088013 (P1, §1 ruling 2, §2 B5): host audit was the only human attribution. Fixed: every request is signed off-host with a personal OpenSSH key under a per-action namespace, verified by the committing process and stored in `STAFF_ACTION`; `oteryn-game-ops audit verify` re-checks; refusal `OPS_STAFF_SIGNATURE_INVALID`; the residual root-with-database-credential case is in THREAT-MODEL-0.
- 4197088022 (P1, §3 ruling 17): two operation_ids could start on one account. Fixed: `game_account_erasures` allows one open row per account; the start takes the admission relation fence, refuses another operation with `ACCOUNT_ERASURE_IN_PROGRESS`, appends `PENDING` before commit, and resumes a same-operation crash; PRIVACY-ERASURE-1 tests.
- 4195149446 (P2, §2 ruling 9): drain state lived only in process memory. Fixed: a durable `game_scope_maintenance` marker committed before the drain request, read at boot before readiness, fail closed when unreadable, and ended by compare-and-set in `maintenance end`.
- 4197873469 (P1, §3 ruling 17): an account with no 0005 guard row was reported complete at once while premium rows (0029, 0040, 0057) still held its AccountId. Fixed: no shortcut; the start mints the marker and the substitution and catalog scan always run, guard steps only when the row exists; premium and spell premium writers take the admission relation fence and refuse `ACCOUNT_ERASURE_IN_PROGRESS`; amendment 2 binds Platform to emit no fact after the request; PRIVACY-ERASURE-1 tests.
- 4197873478 (P1, §1 ruling 2, §3 rulings 4 and 8): lift and remediation signatures were not bound to a deployment. Fixed: every signed tuple starts with `environment_id` and `roster_sha256`; the verifier uses its own restart-only environment id and loaded roster digest; a roster change after approval needs both to re-sign by compare-and-set; ECON-FENCE-1 and ECON-REMEDIATION-1 tests, including a preproduction signature on a production record.
- 4197873488 (P2, §1 ruling 4): a grant could pair Account A with a character of Account B. Fixed: the grant refuses with `OPS_STAFF_GRANT_TARGET_MISMATCH` unless the locked root is live and belongs to the grant's account, and a deferrable composite foreign key enforces it; OPS-GM-ROSTER-4 tests.
- 4196701864 (P2, §2 ruling 9, deferred from round 3): 1200 could close a session with a pass in flight. Fixed: at `close_at` the node stops reading commands, waits for each in-flight pass and its result frame, then sends 1200; the close path is in LIVEOPS-MAINT-1 owned paths; test.
- 4196701873 (P2, §1 ruling 12, deferred from round 3): the table check allowed evidence with a NULL speaker. Fixed: the evidence columns are all NULL or all present, present evidence names the target on a HARASSMENT reason, and erasure clears them together; OPS-GM-PREPORT-6 direct-insert tests.
- 4198571173 (P1, §3 rulings 16-18): the erasure functions ran on the control-plane role that the root ops tool holds, so root could erase an account without a Platform request. Fixed: Platform signs the request; only the node erasure handler's own login role, in neither group, executes begin, retire and substitute; `audit verify` flags an erasure row with no verified journal record; a restore re-applies only verified records; §2 secret table and B5 residual; tests.
- 4198571185 (P1, §3 ruling 17): after `COMPLETED` Game could not refuse a premium fact issued before the request. Fixed: step 7 writes a one-way AccountId digest to `game_account_erasure_denials`; a trigger on every `account_id` table and the guard publication refuse it with `ACCOUNT_ERASED`; the journal re-apply restores it; amendments 2 and 3; tests.
- 4198571195 (P2, §3 ruling 14): inventory classes were free text, not the closed ANL-01 §15 set. Fixed: every row uses one of the four classes and a closed erasure action; the check fails on an unknown class or action and on an `account_id` table without the denial trigger.

## Owner rulings

- 2026-10-06: §1 Q1 a; §1 Q2 a, refined (GitHub bug-report issue form, private vulnerability reporting verified by OPS-GM-REPORT-3, Discord only with external testers, the form delivered by OPS-GM-REPORT-3); §1 Q3 a; §1 Q4 b; §2 Q1 a; §2 Q2 a; §2 Q3 a; §3 Q1 b; §3 Q2 a; §3 Q3 a, subject to legal review; §3 Q4 a. Recorded in each section and in §4.2.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
