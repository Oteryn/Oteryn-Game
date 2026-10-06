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
blocks: [OPS-GM-AUDIT-1, OPS-GM-SANCTION-2, OPS-GM-REPORT-3, OPS-GM-ROSTER-4, OPS-GM-STAFF-5, OPS-GM-PREPORT-6, THREAT-MODEL-0, ERR-PUBLIC-REVIEW-1, LIVEOPS-CONFIG-AUDIT-1, LIVEOPS-MAINT-1, LIVEOPS-NOTICE-2, LIVEOPS-KILL-2, THREAT-DOS-1, SEC-ROTATION-1, SUPPLY-SIGN-1, ECON-FENCE-1, ECON-CASE-HOLD-1, ECON-TRACE-1, ECON-REMEDIATION-1, PRIVACY-INVENTORY-1, PRIVACY-LOG-REDACT-1, PRIVACY-ERASURE-1, PRIVACY-DSR-EXPORT-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome
- Owner ruling 3b of 2026-10-06: the "before external players" package is authored now.
- §1 GM and support: a Game staff roster written with `oteryn-game-ops`; typed sanctions with compare-and-set; staff kick as protocol code 1201 SESSION_STAFF_DISCONNECT; staff character isolation; one `STAFF_ACTION` event type for staff audit; player reports; no in-game GM commands until `STAFF_V1`.
- §2 Threat model and live operations: one living threat model and boundary inventory; the deferred ERR-CODES §1.9 display item; config audit; maintenance drain as protocol code 1200 SESSION_SERVICE_MAINTENANCE (TERMINAL); `SERVICE_NOTICE_V1` countdown; capability kill switch; secrets as files read once; rotation as node replacement; compromise response.
- §3 Economy incidents and privacy: an ANL-03 case and a typed `EconomyContainmentFence`; remediation as a compensating DUR-03 transaction with two-person approval; never a restore; `GAME_PERSONAL_DATA_INVENTORY.json`; the Platform/Game PII split; typed account erasure from Platform; an erasure journal in the restore fence directory of ARCH-ALPHA-OPS-0 §3.
- Contract amendments are exact text, pending and applied by the named packets. Platform proposals bind nothing. Eleven owner items are listed in §4.2 with recommendations.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
