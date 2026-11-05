# ADM-02: Admin console design, analytics catalog and delivery plan

- Status: Candidate plan under ADR-0023; `ImplementationStatus: NOT_STARTED`
- Date: 2026-11-05
- Related: ADR-0023, ADM-01, ADR-0006, ANL-01, ANL-02, ANL-03

## 1. Evidence: what exists today

Inventory of `origin/main` (static source research; file:line citations are in the inventory notes of the design PR). "Implemented" means a traced production caller and effect.

| Area | State |
|---|---|
| Analytics contracts | ADR-0006 accepted; ANL-01 contract with concrete `EventEnvelopeV1` used by item mint/transfer, bank, staff, house, stash writers; ANL-02 and ANL-03 are candidates, `NOT_STARTED` |
| Durable evidence | audit/outbox tables for character, item, spell items, equipment, party, bank, public projections, guild, house, staff, market, mail, stash, store receipts; receipts for item moves, death, quests |
| Publishers that run | account-characters projection, public projections (highscores, profile, online), character command consumer, store receipts, character and staff audit retention |
| Publishers that do not exist | any analytics consumer of item/bank/house/market/mail outboxes; bank and other retention jobs; pseudonym issuer and `AnalyticsActorId` mapping |
| Telemetry not emitted | login/logout, combat damage/healing, quest analytics, session analytics |
| Observability | structured stderr lines (`OTERYN_LOG`), `command_integrity` aggregate log, reset timing log; **no** Prometheus/OTel exporter, **no** HTTP health endpoint, no tick/queue/pool metrics |
| Admin surfaces | root-only `oteryn-game-ops` CLI (launch authorisation, node registration, channel assignment, content activation, recovery); root-only Unix control socket (bootstrap only); signed staff commands over gameplay (Inspect, Mute, Unmute, Kick-recorded-only); host `supervisor.sh` start/stop/status |
| Missing admin capabilities | ban, jail, namelock, notation; item grant/remove; stats edit; rename; teleport; house/market/guild moderation; broadcast; maintenance mode; save now; event/raid trigger; rates/ruleset change; rollback; general player search/dossier |
| DB topology | `oteryn_game` with `oteryn_game_runtime` / `oteryn_game_control` groups, highscore job role; no analytics database, no read-only admin role, no replica |
| Platform side | Blade admin with closed-list RBAC, mandatory TOTP, `admin_audit_events` without IP/correlation/outcome; no charts, queue, Redis or metrics; game projections are latest-only; next Platform ADR 0047 |
| Numbering on main at research time | next Game ADR 0023, next migration above 0916 (claim at implementation time), capability max 275, command type max 491, event families 1,2,3,4,5,8,9 |

## 2. Component architecture

```text
Game repository                                         Platform repository
---------------                                         -------------------
crates/admin/contracts   envelope, catalog, schemas  ── shared JSON schemas (exported, locked in CROSS_REPOSITORY_CONTRACT_LOCK)
crates/admin/agent       oteryn-admin-agent binary      app/Admin/GameControl/*   request store, signer, approvals
  ├─ channel client (mTLS, long-poll/WebSocket)          routes /internal/v1/admin-control/*
  ├─ verifier (signature, nonce, policy)                 Blade modules (§3), Chart.js (vendored)
  ├─ executors:                                          admin_control_requests / _results / _agents
  │    node admin socket  → GameNode runtime             admin_approvals, admin_audit_events (+cols)
  │    oteryn_game (admin procedures, admin_read views)  analytics_cache (rollups)
  │    host supervisor (systemd / supervisor.sh)
  │    backup tooling (pg_dump / pgBackRest)
  │    oteryn_analytics (read-only role)
  └─ ledger writer (admin_command_ledger)
crates/admin/analytics-worker  oteryn-analytics-worker (outbox → facts → rollups)
apps/game-server               admin socket listener + runtime admin command handlers
```

**GameNode admin socket**: a second Unix socket `admin.socket_path`, peer credential must be the agent's service user (not root, not the game user). It accepts only `crates/admin/contracts` node commands (kick, teleport, broadcast, save, drain, world state, event start/stop, live item/stat changes for online characters). Each handler runs inside the runtime's normal ordering and durability path, writes the ledger row and ANL event in the same transaction as the effect, and returns the effect list. The existing root bootstrap control socket is unchanged.

**Offline path**: when the target character is offline, the agent calls typed SQL procedures in schema `game_admin` (security definer, owned by a NOLOGIN role, granted to `oteryn_game_admin_agent` login role) that apply the same fencing/revision checks as the runtime and refuse if the character is online (`ADM_PRECONDITION_FAILED`, retry via the online path).

## 3. Console modules (Platform)

Navigation groups and pages. Every page shows data source (Game agent / Platform), freshness, and for mutating actions the risk class badge and required controls.

1. **Dashboard** — players online now and 24 h curve per world; registrations, logins, peak concurrent users, revenue today (Platform-sourced); node/world health tiles; DB size and backup age; open alerts; last 20 admin actions.
2. **Worlds** — list (state, players, node, revisions, rates); detail tabs: configuration (versioned diff and history), channels, live events, broadcast/MOTD, maintenance and restart controls with countdown, content bundle activation; wizard **Add world** (R4: id, type, ruleset, map/content bundle, rates → dry-run → approval → create → open); **Retire/Delete world** (R4 with export check).
3. **Nodes and hosts** — hosts, agents, builds, assignments, CPU/RAM/disk, tick time; actions drain/start/stop/restart; **Add host** (enrolment token), **Remove host**; **Update build** (dormant until deploy enabled).
4. **Database** — status (size, connections, locks, lag, bloat), slow queries, migrations (applied/pending/drift), backups (list, run, verify, restore wizard R4), maintenance jobs, retention jobs; **SQL read** page (curated views, results grid, saved queries, audit banner).
5. **Players** — search; character dossier (stats, position, equipment, containers, depot, stash, bank, house, guild, sanctions, sessions, history timeline); actions: message, kick, teleport, rename, stats edit, item grant/remove, currency adjust, restore from backup, GM grant; linked Platform account card (email masked, MFA state, premium, payments, tickets) with Platform-side actions.
6. **Sanctions** — queue of reports, active sanctions, ban/mute/jail/namelock forms using Enforcement Command semantics, appeal history.
7. **Economy** — gold supply and sinks, item supply by type, market volume and prices, top holders, admin mints/removals, item provenance explorer, escrow.
8. **Analytics** — the catalog in §5 as dashboards with filters (world, vocation, level band, date range, grain), cohort and funnel views, export.
9. **Security and integrity** — command-integrity counters, suspicious trades, multi-account signals (ANL-03), re-link requests (restricted), staff action review.
10. **Audit** — combined Platform `admin_audit_events` and Game ledger by request id; filters by admin, operation, target; R1 read audit.
11. **Approvals** — pending R4 requests awaiting a second admin, with diff/dry-run preview; approve/reject with step-up.
12. **Access** — roles and permissions (Platform RBAC), signer key status, connected agents, Game policy view.

Platform implementation notes: server-rendered Blade stays; charts via vendored Chart.js with server-supplied JSON; long operations show progress by polling Platform's own result table (no Redis required); every mutating form posts to a Platform controller that writes `admin_control_requests` with the signed envelope.

## 4. Analytics data path

```text
GameNode ──(same tx)──> domain outboxes + ANL-01 envelopes ──┐
GameNode ──(best effort, bounded queue)──> telemetry spool ──┤
agent heartbeat / node metrics exporter ─────────────────────┤
                                                             v
                                 oteryn-analytics-worker (checkpointed, idempotent by event_id)
                                    ├─ pseudonymise actors (AnalyticsActorId, epoch keyed)
                                    ├─ fact tables (partitioned by day): fact_session, fact_combat_kill,
                                    │   fact_death, fact_item_flow, fact_currency_flow, fact_market_trade,
                                    │   fact_quest, fact_level, fact_admin_action, fact_node_metric, fact_store_delivery
                                    └─ rollups: minute (7 d), hour (400 d), day (forever)
                                                             v
                                 oteryn_analytics (PostgreSQL, separate DB, roles: writer, reader, curated)
                                                             v
                       agent executes ADM-01 analytics.* queries ──> Platform console (+ rollup cache)
```

- **Telemetry to add** (Game): `SESSION_STARTED/ENDED` (world, node, client version, duration), `CREATURE_KILLED` (creature type, level band, party size, vocation), `PLAYER_DIED` (cause class, level), `LEVEL_CHANGED`, `SKILL_ADVANCED`, `QUEST_STAGE_CHANGED`, `SPELL_CAST_AGG` (per minute aggregate per spell, not per cast), `LOOT_DROP_AGG`. Best-effort families go through a bounded spool with drop counters; durable families stay in outboxes.
- **Pseudonymisation**: the worker holds the epoch key; facts store `AnalyticsActorId`; the mapping table lives in `oteryn_analytics.identity_map` readable only by the re-link procedure.
- **Retention**: raw facts 90 days (sessions/combat), 400 days (economy/admin), rollups per above; legal holds respected.
- **Thresholds for a columnar store**: move raw telemetry to ClickHouse/Timescale only when measured raw insert rate > 5 000 rows/s sustained, raw facts > 500 GB, or p95 of catalog queries > 3 s on hour rollups. Query catalog and ADM-01 operations stay unchanged.

## 5. Named analytics catalog (v1)

| Group | Query ids | Source |
|---|---|---|
| Activity | `players_online` (series), `ccu_peak`, `dau_wau_mau`, `sessions_length_dist`, `logins_by_hour`, `new_characters`, `retention_cohort` (D1/D7/D30), `churned_players`, `client_versions` | fact_session, Platform registrations |
| Progression | `level_distribution`, `level_up_rate`, `time_to_level` (bands), `vocation_mix`, `skill_progress`, `quest_completion_funnel`, `quest_stage_dropoff` | fact_level, fact_quest |
| Combat and balance | `kills_by_creature`, `exp_per_hour_by_band`, `deaths_by_cause`, `deaths_by_level`, `spell_usage`, `damage_by_vocation` (agg), `boss_attempts_success`, `hunting_ground_usage` | fact_combat_kill, fact_death, spell aggregates |
| Economy | `gold_supply`, `gold_sources_sinks`, `item_supply_by_type`, `item_mints_by_source`, `market_volume`, `market_price_series`, `npc_trade_flow`, `store_deliveries`, `top_wealth` (R1), `admin_value_changes` | fact_currency_flow, fact_item_flow, fact_market_trade, fact_admin_action |
| World and content | `area_heatmap` (sector visits), `house_occupancy`, `guild_activity`, `raid_participation`, `event_participation` | fact_session positions (sector grain), house/guild outboxes |
| Integrity and security | `command_integrity_counters`, `suspicious_trade_pairs`, `rapid_wealth_gain`, `multi_client_ip_clusters` (security), `staff_actions` | ANL-03 signals, admin ledger |
| Operations | `tick_time_p50_p99`, `node_cpu_mem`, `db_size_growth`, `pg_pool_wait`, `queue_drops`, `error_rate_by_code`, `backup_age` | node metrics, heartbeat |

Every query returns `metric_revision`, `watermark` (latest complete minute), `completeness` (per ANL-02 semantics) and the applied filters.

## 6. Storage and capacity guardrails

- `oteryn_analytics` on the same PostgreSQL cluster as `oteryn_game` for phase 2 (separate database and roles), moved to its own instance before raw facts exceed 50 GB or when analytics I/O shows up in game p99 latency.
- Worker batch size 1 000, commit per batch with checkpoint; lag alert above 5 minutes.
- Analytics queries run under `statement_timeout`, `work_mem` caps and a per-admin concurrency of 2.

## 7. Security requirements

- Agent runs as its own service user; its DB login has only `game_admin` procedures, `admin_read` views, backup role where needed, never superuser; superuser actions (restore, migrations) use a separate short-lived credential fetched for the duration of an R4 operation.
- No secrets ever cross the channel (no passwords, keys, connection strings); results are filtered by an allowlist per operation.
- Abuse tests before production enablement: every row of ADM-01 §10 plus fuzzing of the envelope parser and the SQL-read guard.
- Platform: step-up TOTP, R4 approvals by a distinct admin, all admin pages rate-limited, session bound to IP class, CSRF as today.

## 8. Delivery plan

Each phase ends with merged PRs in both repositories and an owner decision to enable it in production. Packets are Codex Cloud tasks (Game) or Platform session tasks (Platform); "PG" means tests against PostgreSQL.

### Phase 0 — contracts (this PR)
ADR-0023, ADM-01, ADM-02 (Game); Platform ADR 0047 accepting the channel, signer, permissions and modules.

### Phase 1 — read-only console (R0/R1)
| Packet | Repo | Content |
|---|---|---|
| ADM-G1 contracts | Game | `crates/admin/contracts`: envelope, JCS canonicalisation, Ed25519 verify, closed schemas for phase-1 operations, JSON schema export, golden vectors |
| ADM-G2 agent core | Game | `crates/admin/agent`: hello/channel/results/heartbeat client (mTLS), verifier pipeline (§3 of ADM-01), nonce store, policy file, ledger migration `admin_command_ledger`, `admin_request_nonces` |
| ADM-G3 read executors | Game | `ops.overview.get`, `world.list/get`, `node.list/get`, `db.status`, `db.migrations.list`, `player.search`, `player.character.get`, `player.inventory.get`, `sanction.list`, `admin.ledger.query`, `admin.policy.get`; `admin_read` views migration and `oteryn_game_admin_agent` role; PG tests |
| ADM-P1 Platform channel | Platform | migration for `admin_control_requests/_results/_agents`, mTLS endpoints, signer service, nginx allowlist entry |
| ADM-P2 Platform console v1 | Platform | permissions/roles from ADM-01 §8, step-up TOTP, audit columns, Dashboard, Worlds, Nodes, Database (read), Players (read), Audit, Access pages |
| ADM-X1 conformance | both | shared golden envelopes and the §10 signature/replay/expiry tests on both sides |

### Phase 2 — moderation and operations (R2/R3), node metrics, analytics foundation
| Packet | Repo | Content |
|---|---|---|
| ADM-G4 node admin socket | Game | listener, peer-credential check, handlers: kick (with real session fence), teleport, message, broadcast, MOTD, save now, drain, world state, restart orchestration |
| ADM-G5 sanctions | Game | ban (character/account), mute/unmute, revoke, notation via Enforcement Command semantics; session fence; PG tests |
| ADM-G6 metrics | Game | Prometheus exporter on loopback for node (tick time, sessions, queues, pool, errors) scraped by the agent; `ops.metrics.series`, `ops.alerts.*`, `ops.logs.tail` |
| ADM-G7 backups | Game | `db.backup.list/run/verify` around existing backup tooling; restore-test job |
| ADM-G8 analytics foundation | Game | `oteryn_analytics` DB and roles, `oteryn-analytics-worker` with checkpoints, pseudonymiser, session/death/level/item/currency/market facts, rollups, `analytics.catalog/query` with Activity, Economy, Operations groups; registry purpose entries for consumed families |
| ADM-G9 session telemetry | Game | `SESSION_STARTED/ENDED`, `LEVEL_CHANGED`, `PLAYER_DIED` emission with spool and drop counters |
| ADM-P3 Platform console v2 | Platform | action forms with risk badges, approvals page, Sanctions, Economy (read), Analytics dashboards (Chart.js), alerts |

### Phase 3 — value changes, configuration, deeper analytics
ADM-G10 item grant/remove/escrow, currency adjust, stats edit, rename, GM grant (with ANL source `ADMIN`); ADM-G11 world configure/rates/feature flags/content activate/channel assign/launch authorisation; ADM-G12 market/house/guild moderation; ADM-G13 events/raids/spawn; ADM-G14 combat/quest/spell telemetry and the Progression, Combat, World, Integrity query groups; ADM-G15 `db.query.read` and `analytics.sql.read` with the SQL guard and fuzz tests, re-link, export; ADM-P4 Platform pages for all of it.

### Phase 4 — destructive and lifecycle (R4)
ADM-G16 world create/retire with Platform Registry orchestration; ADM-G17 node enrol/remove with PKI; ADM-G18 restore with Restore Reconciliation; migrations apply; character restore; ADM-P5 Platform approval-heavy wizards.

### Phase 5 — deploy and fleet
`node.update` from signed release channel (enabled only after the owner's deploy decision), world delete/merge after a separate design gate, columnar analytics store if thresholds in §4 are hit.

## 9. Open decisions for the owner

1. Backup tooling: keep `pg_dump` (current scripts) or adopt pgBackRest with point-in-time recovery (recommended before Phase 4).
2. Whether account-level bans are decided in Platform (account owner) and mirrored to Game, or issued from Game sanctions (recommended: Platform decides account bans, Game decides character bans; both through Enforcement Command semantics).
3. Production enablement order of phases.
