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
last_progress: "review round 4: merged origin/main 6560803cf (merge 2a2026c5a); Codex 4197124781 (restore selects the build of the newest build activation row, checked against the restored ledger digest) and 4197124797 (ruling 5 content check uses the bundles of every activated build, or the live database with all nodes stopped under the ops lock; never a drill database) fixed; adversarial self-review fixes; local validation passed"
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
  - "§3 ruling 10 selects post-restore re-requests by the Game recovery generation and an id-set difference, never by comparing Platform and Game clocks, with clock-behind, clock-ahead and duplicate-replay tests (Codex 4196648601)"
  - "§3 ruling 9 step 3 selects the build named by the newest build activation row in the restored database (written only by `release activate` after migration), verifies the restored ledger digest against it, and fails closed on no row, a mismatch, a build missing from the never-pruned release index or a partial ledger; no selection by digest alone or by backup annotation (Codex 4197124781)"
  - "§3 ruling 5 checks content references against the bundles of every activated build, and otherwise against the live database with every node stopped under the exclusive ops lock; a drill database is never the gate (Codex 4197124797)"
  - "§3 rulings 3, 8, 9 and 12 serialise deploy, restore and drill with the ops lock, keep the supervisor from restarting nodes, and make the restore resumable from a versioned journal without a second CAS, a lost epoch raise or a shortened 35 s gate"
  - "no item in the accepted scope of §1-§4 is open or PROPOSED; §3 Q4 is DECIDED a, accepted by control plane under D607 (D838) (Codex 4196648606)"
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
- Contract amendments are exact text, pending and applied by the named packets. The ten first-round owner items of §5.2 carry the owner rulings of 2026-10-06; §3 Q4 (review round 2) is DECIDED a, accepted by control plane under D607 (D838).

## Review findings

- Codex 4195138078 (P1, §4 ruling 9): a revoked release key could be replayed through an older signed trust record. Fixed: SEC-CLIENT-01 §3 (lines 113-169) defines the trust-record floor for the node only and `main` has no client updater (`apps/client/src/`). §4 ruling 9 now adds a client build floor and a persisted `trust-floor.v1` (highest applied revision, revoked set, highest checkpoint) outside the Velopack install root, written atomically and never lowered before any manifest is verified, with sequential record application and checkpoint checks. Tests are in RELEASE-MANIFEST-3 and CLIENT-UPDATE-4; amendment 7 and the release order carry it.
- Codex 4195138089 (P2, §2 ruling 12): the bench selector missed hot-path dependencies. Fixed: the selector is the `oteryn-game-server` local dependency closure computed from `cargo metadata` by `tools/repository/classify_pr_test_lanes.py` (`graph`, lines 231-268), plus Cargo build inputs (lines 30-33), `vendor/**` and the workflow, failing closed. The edges are cited from code (`gameplay_transport/world_map.rs:36`, `ai_think.rs:47,54`, `monster_ai_cycle.rs:15`, `movement/speed.rs:13`, `gameplay_transport/mod.rs:363`). PERF-CI-1 owns the classifier and tests view.rs, movement.rs, world_runtime.rs, owner_timer.rs, ai_think.rs, each dependency crate, `Cargo.lock` and `vendor/`.
- Codex 4195138097 (P2, §1 ruling 6): logs were pruned only at start. Fixed: `oteryn-game-server log-sink` rotates the active segment at 64 MiB or 24 h while running, prunes hourly by 14-day age and a 1 GiB cap (never the active segment), and keeps a lifecycle file for health and the deploy revision read. Bounds: at most 1 GiB plus 64 MiB on disk; each line kept at least 14 days and removed within 15 days 1 hour. OBS-DEPLOY-4 lists the tests.
- Codex 4196009699 (P1, §3 ruling 9 step 4): a PITR to before an earlier recovery left the database below the external generation G, and reconcile accepts only G or G+1 (`character_authority.rs:411-417`; contiguous rows, `0005:40`). Fixed: `begin_recovery` keeps every superseded record write-once as `character-recovery-fence-v1.<generation>.record`; reconcile verifies the digest chain from the database's row H and inserts H+1 to G+1 in one transaction, refusing a missing or broken record. DATA-RESTORE-OPS-2 tests two recoveries and a PITR before both; DATA-RESTORE-DRILL-4 adds the case.
- Codex 4196009718 (P1, §3 ruling 9 step 3): the current binary's exact gate stopped every restore to a pre-migration point. Fixed: backups carry `oteryn-build` and `oteryn-ledger` annotations, releases record the digest of their embedded `migrate!` set, and the restore runs the retained release whose digest matches the restored ledger, failing closed otherwise; it never migrates. DATA-RESTORE-OPS-2 and DATA-BACKUP-PITR-3 own it.
- Codex 4196009733 (P1, §3 ruling 7): `archive_timeout` does not prove archiving. Fixed: a `pgbackrest check` probe every 120 s exports its last success through the `node_exporter` textfile collector; `OterynWalArchiveBehind` (over 300 s) and `OterynWalArchiveProbeMissing` alert, and a release with a migration does not start while either fires. The 5 min RPO holds only while both are clear. Admission during an alert is §3 Q4, DECIDED a, accepted by control plane under D607 (D838).
- Codex 4196009753 (P1, §2 ruling 9): the World limit was a multiplication. Fixed: PERF-WORLD-1 measures it with C ≥ 2 Channels on one PostgreSQL; until then it equals the Channel value.
- Codex 4196009764 (P2, §2 ruling 11): headroom used the first failing N. Fixed: the accepted value is floor(highest passing N / 1.3); the evidence records every step.
- Codex 4196648601 (P1, §3 ruling 10): selecting re-requests by Platform acknowledgement time after T compared two clocks, so a lagging Platform clock could drop a lost delivery. Fixed: every Game outcome carries the Character recovery generation it committed under (fence `character_authority.rs:533`, `0005:30-42`); after a restore Game reads its `delivery_operation_id` receipts, lists Platform's acknowledged deliveries with generation ≥ H by Platform's own cursor, and re-applies each absent id under the same id (Platform contract `OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md:218-237`; Inbox `0076:61,215-277`). Ownership operations use the same bound (`0005:90-111`). A3, A4, U6, ruling 9 step 12 and PLATFORM-RESTORE-RECONCILE-P1 tests (clock behind, clock ahead, duplicate replay) follow.
- Codex 4196648606 (P2, §3 Q4): the scope was accepted on merge while Q4 was PROPOSED. Fixed: Q4 is DECIDED a, accepted by control plane under D607 (D838), in the status line, ruling 7, §3 owner questions and §5.2.
- Codex 4197124781 (P1, §3 ruling 9 step 3): releases that share a migration digest were not told apart, and the backup annotations do not say which build ran at T. Fixed: a new append-only build activation table (`activation_revision, build_id, ledger_digest, bundle_digest`), with a high-water taken `FOR UPDATE` and a guard trigger as in `0006:254-263`, is written only by `oteryn-game-ops release activate` under the exclusive ops lock after migration. Nodes register only when a row names their build (`serve.rs:4-7,1442`); registrations carry no build id (`0003:40-52`, F31). The restore runs the build of the highest revision; WAL order decides, never clocks. It fails closed on no row, a digest mismatch, a build missing from the never-pruned `releases/` index (U8), an unsuccessful row or a partial ledger. Annotations are diagnostic only. This replaces the digest selection of 4196009718.
- Codex 4197124797 (P1, §3 ruling 5): the content check ran on a drill database, which can predate the gate, and the live stack could write a new reference after its snapshot. Fixed: the deploy runs a bundle check against the bundles of every activated build (no player rows read; F32: the only literal seed is `0070:127`, references come from server writes `0049:15,94`). If it fails or has no input, the live check runs on the live database with a read-only role, after every node is stopped, under the exclusive ops lock, and the release ships stop-the-world. A drill database is never the gate. DATA-CONTENT-REF-5 owns both, after DATA-RESTORE-OPS-2 and DATA-BACKUP-PITR-3.
- Self-review, round 4 (fixed in the same candidate): rows from builds before the activation table force the live check; activation comes before the domain migration; the supervisor keeps nodes stopped and the deploy verifies it; scheduled backups and expiry pause until restore step 3; an ops lock serialises deploy, restore and other ops writes; a crash after the CAS is detected by the event id on the fence record; the journal records each epoch target before the raise; the 35 s gate uses the monotonic clock and restarts in full on resume; pgBackRest restores with `--delta`; `--abandon` is refused after admission opens; the journal is versioned; a drill uses its own fence directory and ops lock and never notifies production Platform; a stop-the-world release cannot be rolled back by a deploy; the release index is never pruned in the alpha (U8).

## Rulings

- 2026-10-06, given by the owner to the architect: every owner question is ruled **a**, except §3 Q2, ruled **b** with a refinement: the fence directory on the NAS is used only for the testing phase, and the choice is decided again before external players are admitted. Recorded in each section's owner questions and in §5.2; §3 rulings 8 and 12, amendments A1 and A2, DATA-RESTORE-OPS-2 and U7 carry the testing-phase scope and the re-decision gate.

- 2026-10-06, control plane under D607 (D838): §3 Q4 decided **a**.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
