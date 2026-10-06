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
last_progress: "review round 2: Codex findings 4196009699, 4196009718, 4196009733, 4196009753 and 4196009764 fixed; §3 Q4 PROPOSED a pending the owner; local validation passed"
acceptance_criteria:
  - "§1-§4 rulings, packets and contract amendments are exact enough for each named packet to start without a new decision"
  - "§4 ruling 9 defines a durable, monotonic client trust-record rollback floor (build floor plus persisted trust-floor.v1 outside the install root), the order the updater checks it in, and its tests (Codex 4195138078)"
  - "§2 ruling 12 selects the bench job by the oteryn-game-server local dependency closure from cargo metadata, Cargo build inputs, vendor/ and the workflow, failing closed, with a classifier test (Codex 4195138089)"
  - "§1 ruling 6 rotates the active log while the node runs (64 MiB or 24 h), prunes hourly by age and a 1 GiB cap, states the bounds and the tests (Codex 4195138097)"
  - "§3 ruling 9 step 4 bridges a restore across several recovery generations from write-once retained fence records, refusing a missing or broken chain, with a two-recovery test (Codex 4196009699)"
  - "§3 ruling 9 step 3 selects the retained release whose embedded migration digest equals the restored ledger and fails closed otherwise; backups carry build and ledger annotations (Codex 4196009718)"
  - "§3 ruling 7 proves archiving with a pgbackrest check probe every 120 s and two alerts; the 5 min RPO holds only while they are clear (Codex 4196009733)"
  - "§2 ruling 9 measures the World limit with its own multi-Channel run (PERF-WORLD-1) and never multiplies; until then it equals the Channel value (Codex 4196009753)"
  - "§2 ruling 11 applies the 30% headroom to the highest passing N, never the first failing N (Codex 4196009764)"
  - "every owner question carries its owner ruling of 2026-10-06 in its section and in §5.2, and the body follows it; §3 Q2 b is testing-phase only with a re-decision gate before external players"
  - "the document stays a proposed architecture decision with no runtime authority"
owned_paths: [docs/architecture/reviews/OTERYN_GAME_ARCH_ALPHA_OPERABILITY_2026-10-06.md, docs/agents/tasks/archive/OTV2-20261006-arch-alpha-ops-0.md]
public_contracts: []
depends_on: []
blocks: [OBS-LOG-1, OBS-CORR-2, OBS-METRICS-3, OBS-DEPLOY-4, OBS-BUNDLE-5, TIME-CLOCK-1, TIME-SKEW-1, PERF-ALPHA-1, PERF-CI-1, PERF-ACCEPT-1, PERF-WORLD-1, DATA-MIGRATION-GUARD-1, DATA-RESTORE-OPS-2, DATA-BACKUP-PITR-3, DATA-RESTORE-DRILL-4, DATA-CONTENT-REF-5, VERSION-FLOOR-1, VERSION-ASSET-PIN-2, RELEASE-MANIFEST-3, CLIENT-UPDATE-4, CLIENT-RELEASE-5]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome
- Answers the owner's request of 2026-10-06 to review what exists and fix what still needs an architectural decision. Owner ruling 1a: the alpha operability package is one decision set.
- §1 Observability: log levels, `parent` and `attempt` correlation, `traceparent` to Platform, a log sink that rotates the active log while the node runs and prunes by age and size, with a lifecycle file per incarnation (fixes the log lost after two restarts), retention 14/30 days, loopback Prometheus metrics with a fixed label set, alert rules and `diagnose --bundle`.
- §2 Time and performance: no global tick; registered timers; one `OwnerClock` per Channel; each rule names one of four clocks; durable time is a remaining duration or a database-time deadline; chrony and a skew alarm; capacity is measured per channel by the harness load mode; iai-callgrind benches, selected by the game-server dependency closure, report before they gate.
- §3 Data continuity: one migration ledger and a guard, one schema version per release, stop-the-world for releases with a migration, `{family, key, revision}` content references, pgBackRest with RPO 5 min, RTO 4 h and 14-day retention, the restore fence directory (including the erasure journal of ARCH-LIVE-READINESS-0; on a NAS volume for the testing phase only), a twelve-step re-entrant restore and Platform reconciliation by operation identity. Records that PR #1874's deploy runs the migration before the stop and assigns the reorder to DATA-BACKUP-PITR-3.
- §4 Client version: `oteryn-client/<semver>+<sha12>`, `client_build_floor` and wire code 1117, a latest-only signed feed before login, assets in the build, Velopack on GitHub Releases, and an Ed25519 release manifest with offline keys and a persisted client trust-record rollback floor.
- Contract amendments are exact text, pending and applied by the named packets. The ten first-round owner items of §5.2 carry the owner rulings of 2026-10-06; §3 Q4 (review round 2) is PROPOSED a until the owner rules.

## Review findings

- Codex 4195138078 (P1, §4 ruling 9): a revoked release key could be replayed through an older signed trust record. Fixed: SEC-CLIENT-01 §3 (lines 113-169) defines the trust-record floor for the node only and `main` has no client updater (`apps/client/src/`). §4 ruling 9 now adds a client build floor and a persisted `trust-floor.v1` (highest applied revision, revoked set, highest checkpoint) outside the Velopack install root, written atomically and never lowered before any manifest is verified, with sequential record application and checkpoint checks. Tests are in RELEASE-MANIFEST-3 and CLIENT-UPDATE-4; amendment 7 and the release order carry it.
- Codex 4195138089 (P2, §2 ruling 12): the bench selector missed hot-path dependencies. Fixed: the selector is the `oteryn-game-server` local dependency closure computed from `cargo metadata` by `tools/repository/classify_pr_test_lanes.py` (`graph`, lines 231-268), plus Cargo build inputs (lines 30-33), `vendor/**` and the workflow, failing closed. The edges are cited from code (`gameplay_transport/world_map.rs:36`, `ai_think.rs:47,54`, `monster_ai_cycle.rs:15`, `movement/speed.rs:13`, `gameplay_transport/mod.rs:363`). PERF-CI-1 owns the classifier and tests view.rs, movement.rs, world_runtime.rs, owner_timer.rs, ai_think.rs, each dependency crate, `Cargo.lock` and `vendor/`.
- Codex 4195138097 (P2, §1 ruling 6): logs were pruned only at start. Fixed: `oteryn-game-server log-sink` rotates the active segment at 64 MiB or 24 h while running, prunes hourly by 14-day age and a 1 GiB cap (never the active segment), and keeps a lifecycle file for health and the deploy revision read. Bounds: at most 1 GiB plus 64 MiB on disk; each line kept at least 14 days and removed within 15 days 1 hour. OBS-DEPLOY-4 lists the tests.- Codex 4196009699 (P1, §3 ruling 9 step 4): a PITR to before an earlier recovery left the database below the external generation G, and reconcile accepts only G or G+1 (`character_authority.rs:411-417`; contiguous rows, `0005:40`). Fixed: `begin_recovery` keeps every superseded record write-once as `character-recovery-fence-v1.<generation>.record`; reconcile verifies the digest chain from the database's row H and inserts H+1 to G+1 in one transaction, refusing a missing or broken record. DATA-RESTORE-OPS-2 tests two recoveries and a PITR before both; DATA-RESTORE-DRILL-4 adds the case.
- Codex 4196009718 (P1, §3 ruling 9 step 3): the current binary's exact gate stopped every restore to a pre-migration point. Fixed: backups carry `oteryn-build` and `oteryn-ledger` annotations, releases record the digest of their embedded `migrate!` set, and the restore runs the retained release whose digest matches the restored ledger, failing closed otherwise; it never migrates. DATA-RESTORE-OPS-2 and DATA-BACKUP-PITR-3 own it.
- Codex 4196009733 (P1, §3 ruling 7): `archive_timeout` does not prove archiving. Fixed: a `pgbackrest check` probe every 120 s exports its last success through the `node_exporter` textfile collector; `OterynWalArchiveBehind` (over 300 s) and `OterynWalArchiveProbeMissing` alert, and a release with a migration does not start while either fires. The 5 min RPO holds only while both are clear. Admission during an alert is owner question §3 Q4, PROPOSED a.
- Codex 4196009753 (P1, §2 ruling 9): the World limit was a multiplication. Fixed: PERF-WORLD-1 measures it with C ≥ 2 Channels on one PostgreSQL; until then it equals the Channel value.
- Codex 4196009764 (P2, §2 ruling 11): headroom used the first failing N. Fixed: the accepted value is floor(highest passing N / 1.3); the evidence records every step.

## Owner rulings

- 2026-10-06, given by the owner to the architect: every owner question is ruled **a**, except §3 Q2, ruled **b** with a refinement: the fence directory on the NAS is used only for the testing phase, and the choice is decided again before external players are admitted. Recorded in each section's owner questions and in §5.2; §3 rulings 8 and 12, amendments A1 and A2, DATA-RESTORE-OPS-2 and U7 carry the testing-phase scope and the re-decision gate.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
