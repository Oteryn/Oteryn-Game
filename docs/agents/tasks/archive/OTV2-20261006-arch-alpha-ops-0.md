# OTV2-20261006-arch-alpha-ops-0

```yaml
task_id: OTV2-20261006-arch-alpha-ops-0
title: "ARCH-ALPHA-OPS-0: alpha operability (observability, time and performance, data continuity, client version)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: cand/arch-alpha-ops
issue: 162
pr: 1878
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01WQyZ8BUWVpmDLpSTpXHvn1 (Sol Supervising Architect)
created_at: 2026-10-06
updated_at: 2026-10-06
execution_policy: continuous_progress
owned_paths: [docs/architecture/reviews/OTERYN_GAME_ARCH_ALPHA_OPERABILITY_2026-10-06.md, docs/agents/tasks/archive/OTV2-20261006-arch-alpha-ops-0.md]
public_contracts: []
depends_on: []
blocks: [OBS-LOG-1, OBS-CORR-2, OBS-METRICS-3, OBS-DEPLOY-4, OBS-BUNDLE-5, TIME-CLOCK-1, TIME-SKEW-1, PERF-ALPHA-1, PERF-CI-1, PERF-ACCEPT-1, DATA-MIGRATION-GUARD-1, DATA-RESTORE-OPS-2, DATA-BACKUP-PITR-3, DATA-RESTORE-DRILL-4, DATA-CONTENT-REF-5, VERSION-FLOOR-1, VERSION-ASSET-PIN-2, RELEASE-MANIFEST-3, CLIENT-UPDATE-4, CLIENT-RELEASE-5]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome
- Answers the owner's request of 2026-10-06 to review what exists and fix what still needs an architectural decision. Owner ruling 1a: the alpha operability package is one decision set.
- §1 Observability: log levels, `parent` and `attempt` correlation, `traceparent` to Platform, per-incarnation node logs (fixes the log lost after two restarts), CANDIDATE retention, loopback Prometheus metrics with a fixed label set, alert rules and `diagnose --bundle`.
- §2 Time and performance: no global tick; registered timers; one `OwnerClock` per Channel; each rule names one of four clocks; durable time is a remaining duration or a database-time deadline; chrony and a skew alarm; capacity is measured per channel by the harness load mode; iai-callgrind benches report before they gate.
- §3 Data continuity: one migration ledger and a guard, one schema version per release, stop-the-world for releases with a migration, `{family, key, revision}` content references, pgBackRest, the restore fence directory (including the erasure journal of ARCH-LIVE-READINESS-0), a twelve-step re-entrant restore and Platform reconciliation by operation identity. Records that PR #1874's deploy runs the migration before the stop and assigns the reorder to DATA-BACKUP-PITR-3.
- §4 Client version: `oteryn-client/<semver>+<sha12>`, `client_build_floor` and wire code 1117, a latest-only signed feed before login, assets in the build, Velopack and an Ed25519 release manifest with offline keys.
- Contract amendments are exact text, pending and applied by the named packets. Ten owner items are listed in §5.2 with recommendations.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
