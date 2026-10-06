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
last_progress: "2026-10-06 review round 2: Codex findings 4195977709, 4195977719, 4195977731 and 4195977740 fixed after round 1 (4195149410, 4195149418, 4195149435, 4195149446); owner rulings of 2026-10-06 on all eleven §4.2 items recorded and applied"
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
- Erasure leaves no AccountId in any Game table except audit envelopes until expiry: bank balances are retired to 0 and `account_id` in root and bank rows is replaced by a random marker inside one guarded function, with a before/after value digest (§3 ruling 17, amendment 4).
- The erasure journal holds AccountId and operation_id only and is an inventory row that the inventory check requires (§3 rulings 14, 18).
- A reported chat line must have been spoken by the report target, enforced in the command and by a table check (§1 ruling 12).
- Every owner ruling of 2026-10-06 is recorded in its section and in §4.2, and the body is consistent with it.

## Review findings

- 4195149410 (P1, §3 ruling 2): fence reads alone did not serialize with a raise under READ COMMITTED. Fixed: a singleton fence root row, held FOR SHARE by every covered writer after the recovery fence and FOR UPDATE by a raise or lift, modelled on the recovery admission row; tests (a)-(c) are deterministic interleavings on two connections.
- 4195149418 (P1, §3 ruling 8): the two-person rule compared values typed into a root tool. Fixed: personal OpenSSH signing keys per `ECONOMY_REMEDIATOR` holder, an allowed-signers roster in the reviewed deployment location with its digest recorded, verification at approval and before every step, distinct principals and keys, and `OPS_REMEDIATION_SECOND_PERSON_UNAVAILABLE` with fewer than two principals.
- 4195149435 (P1, §3 packets): the guild-leader case was a test, not a dependency. Fixed: decision packet GUILD-ERASURE-0 (a GUILD-0 §3.5 amendment) precedes PRIVACY-ERASURE-1 in the brief order, the packets table, U3 and `blocks`.
- 4195977709 (P1, §3 ruling 4): a single holder could lift a fence. Fixed: a lift needs two `ECONOMY_REMEDIATOR` signatures over `(fence_id, fence revision, case_id)` in a lift namespace, verified with the roster digest, committed by compare-and-set under the root FOR UPDATE, fail closed; ECON-FENCE-1 tests.
- 4195977719 (P1, §3 ruling 17): bank rows keyed by AccountId had no erasure rule. Fixed: `ERASURE_RETIRE` under a new `AccountErasureCause`, then one SECURITY DEFINER substitution of a random marker for `account_id` in root and bank rows with no value or chain change and an equal value digest; BANK-0 amendment 4; inventory rows for every AccountId-keyed table; PRIVACY-ERASURE-1 tests.
- 4195977731 (P2, §3 rulings 14, 18): the erasure journal was not inventoried. Fixed: journal reduced to AccountId and operation_id, inventory row with classification, access and backup-bound retention, and the check requires it; `game_account_erasures` row added.
- 4195977740 (P2, §1 ruling 12): a report could attach a line by another speaker. Fixed: the buffer records the speaker, `CHAT_LINE_TARGET_MISMATCH` refuses a line not spoken by the target or a non-HARASSMENT revision, and a table check binds the stored speaker to the target; OPS-GM-PREPORT-6 tests.
- 4195149446 (P2, §2 ruling 9): drain state lived only in process memory. Fixed: a durable `game_scope_maintenance` marker committed before the drain request, read at boot before readiness, fail closed when unreadable, and ended by compare-and-set in `maintenance end`.

## Owner rulings

- 2026-10-06: §1 Q1 a; §1 Q2 a, refined (GitHub bug-report issue form, private vulnerability reporting verified by OPS-GM-REPORT-3, Discord only with external testers, the form delivered by OPS-GM-REPORT-3); §1 Q3 a; §1 Q4 b; §2 Q1 a; §2 Q2 a; §2 Q3 a; §3 Q1 b; §3 Q2 a; §3 Q3 a, subject to legal review; §3 Q4 a. Recorded in each section and in §4.2.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
