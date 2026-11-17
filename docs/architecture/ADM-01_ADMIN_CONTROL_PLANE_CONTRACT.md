# ADM-01: Admin control plane contract (Platform console ↔ Game admin agent)

- Status: Candidate contract, direction accepted by ADR-0023; `ImplementationStatus: NOT_STARTED`
- Date: 2026-11-05
- Owner: Game (agent, catalog, policy, execution); Platform (console, identity, approvals, request store)
- Contract id: `oteryn.admin-control.v1`
- Related: ADR-0023, ADM-02, ADR-0006, ANL-01, ANL-02, ANL-03, ADR-0004, ADR-0013, staff capability `0051-staff-v1`, Platform `OTERYN_V2_GAME_ENFORCEMENT_COMMAND_CONTRACT.md`, `OTERYN_V2_GAME_RESTORE_RECONCILIATION_CONTRACT.md`, `WORLD_REGISTRY_CONTRACT.md`, `OTERYN_V2_WORLD_TOPOLOGY_CONTRACT.md`

This document is the wire and semantic contract both repositories implement. Platform may build the console against it before Game implements every operation: an operation the connected agent does not advertise is shown as **unavailable**, never emulated.

## 1. Parties and identities

| Party | Identity | Holds |
|---|---|---|
| Admin console | Platform admin user (MFA-confirmed session) | RBAC roles, step-up timestamp, approvals |
| Platform admin signer | Ed25519 key `platform-admin-signer-<n>`, private part only in Platform secret storage | signs every request envelope |
| Admin agent | mTLS client certificate, CN `oteryn-admin-agent/<agent_id>`, SAN lists served worlds and hosts | executes catalog operations |
| Game policy | `admin_policy.toml` pinned at agent start: trusted Platform signer public keys (with validity windows), operation → minimum risk class and permission, per-world enablement | decides, independently of Platform, what may run |

Key rotation: Game accepts at most two Platform signer keys at once (current and next). Rotation = add next key to Game policy, restart agent, switch Platform signer, remove old key. A request signed by an unknown or expired key fails `ADM_SIGNER_UNTRUSTED`.

## 2. Transport

All endpoints live under Platform's existing mTLS ingress, allowlisted for the agent identity only. Platform never initiates a connection to Game.

| Endpoint (Platform) | Direction | Purpose |
|---|---|---|
| `POST /internal/v1/admin-control/hello` | agent → Platform | register session: `agent_id`, build revision, catalog digest, advertised operations with versions, served worlds/nodes, clock |
| `POST /internal/v1/admin-control/channel` | agent → Platform, long-poll (≤ 25 s) or WebSocket | receive pending requests addressed to this agent (batch ≤ 32) |
| `POST /internal/v1/admin-control/results` | agent → Platform | post `accepted` / `progress` / final result for a request |
| `POST /internal/v1/admin-control/stream` | agent → Platform | chunked continuation for large query results (≤ 1 MiB per chunk, ≤ 64 chunks) |
| `POST /internal/v1/admin-control/rollups` | agent → Platform | periodic analytics rollup push (dashboard cache) |
| `POST /internal/v1/admin-control/heartbeat` | agent → Platform, every 10 s | health snapshot of agent, nodes, DB, backups |

- Encoding: JSON with canonical form (RFC 8785, JCS) for the signed part; UTF-8; integers as JSON numbers within ±2^53, larger values (item counts, coin amounts) as decimal strings.
- An agent is **connected** when its last channel poll or heartbeat is younger than 30 s. Requests for a disconnected agent stay `queued` until `expires_at`, then become `expired`; Platform shows them, never retries silently with a new id.
- Several agents may serve one host group; each world and node has exactly one owning agent at a time (`owner_epoch` in hello). A stale epoch receives `ADM_AGENT_SUPERSEDED` and must stop.

## 3. Request envelope

```json
{
  "contract": "oteryn.admin-control.v1",
  "request_id": "0193f0c2-…",            // UUIDv7, unique per attempt chain
  "admin_operation_id": "0193f0c2-…",    // idempotency key for commands; equals request_id for queries
  "operation": "player.item.grant",
  "operation_version": 1,
  "kind": "command",                     // command | query
  "target": { "world_id": "w-antica", "node_id": null, "character_id": "c-…", "account_ref": null },
  "params": { },                         // operation schema, closed (unknown fields rejected)
  "dry_run": false,
  "actor": {
    "admin_id": "a-17", "roles": ["game_master_senior"],
    "permission": "game.player.value.write",
    "mfa_at": "2026-11-05T18:00:00Z", "step_up_at": "2026-11-05T18:58:12Z",
    "ip_hash": "…", "session_ref": "…"
  },
  "reason": { "code": "SUPPORT_REFUND", "text": "ticket #8812 lost item on crash", "ticket_ref": "T-8812" },
  "approvals": [ { "admin_id": "a-3", "approved_at": "…", "approval_id": "…" } ],
  "risk_class": "R3",
  "issued_at": "2026-11-05T18:58:20Z",
  "expires_at": "2026-11-05T19:03:20Z",
  "nonce": "base64url(16 bytes)",
  "signer_key_id": "platform-admin-signer-1",
  "signature": "base64url(ed25519(JCS(envelope without signature)))"
}
```

Agent verification order (fail closed, first failure wins, nothing parsed into effects before step 5):

1. contract and operation known and advertised → else `ADM_UNKNOWN_OPERATION`;
2. signature with a trusted key → `ADM_SIGNER_UNTRUSTED` / `ADM_BAD_SIGNATURE`;
3. `issued_at` ≤ now + 30 s and now < `expires_at`, lifetime ≤ policy max (queries 60 s, R2 5 min, R3 5 min, R4 30 min) → `ADM_EXPIRED`;
4. nonce unseen within 24 h (stored in `admin_request_nonces`) → `ADM_REPLAY`;
5. `params` validates against the closed schema → `ADM_INVALID_PARAMS`;
6. Game policy: operation enabled for the target world; `actor.permission` is in the operation's allowed set; `risk_class` ≥ Game's minimum for the operation → `ADM_FORBIDDEN` / `ADM_RISK_CLASS_MISMATCH`;
7. control evidence for the Game-side class: reason present for ≥ R2; `step_up_at` within 5 min of `issued_at` for ≥ R3; ≥ 1 approval by a different `admin_id` for R4, approval newer than 30 min → `ADM_CONTROL_MISSING`;
8. preconditions of the operation (maintenance mode, world state, `expected_revision`) → operation-specific errors;
9. idempotency: same `admin_operation_id` and same canonical params digest → return the recorded result; different digest → `ADM_IDEMPOTENCY_CONFLICT`.

## 4. Result envelope

```json
{
  "request_id": "…", "admin_operation_id": "…",
  "status": "succeeded",                 // accepted | in_progress | succeeded | failed | rejected | expired | cancelled
  "error": null,                         // { "code": "ADM_…", "message": "…", "retryable": false }
  "result": { },                         // operation result schema
  "dry_run_preview": null,               // for dry_run=true: exact effects that would be applied
  "game_ledger_id": "…",                 // admin_command_ledger row
  "audit_event_ids": ["…"],              // ANL-01 durable events written
  "effective_revision": "…",             // new revision of the changed aggregate, when applicable
  "progress": null,                      // { "step": 3, "of": 7, "text": "draining channel 2" }
  "completed_at": "…",
  "agent_id": "…", "agent_build": "…", "signature": "…"   // agent signs results with its own key
}
```

Long operations (drain, backup, restore, migration, world create, node update) return `accepted`, then stream `in_progress` updates, then one final status. Cancellation (`admin.request.cancel`, R2) is honoured only at declared safe points; the final result says how far the operation got.

## 5. Risk classes (normative)

| Class | Meaning | Platform controls | Game checks |
|---|---|---|---|
| R0 | aggregate read, no player-linked data | permission | permission |
| R1 | player-linked read or pseudonym re-link | permission, read audit, masked by default (email, IP, payment) | permission, ledger row for re-link and exports |
| R2 | reversible change | permission, reason | reason present |
| R3 | durable value, sanction, configuration or rates | permission, reason, step-up TOTP ≤ 5 min | step-up timestamp |
| R4 | destructive or service-wide | permission, reason, step-up, second admin approval, dry-run shown first, maintenance window when listed | approval by distinct admin; dry-run digest equals executed digest |

`dry_run` is supported by every R3 and R4 command. For R4 the console must execute dry run first; the real request carries `params.preview_digest` equal to the dry run's `dry_run_preview` digest, and the agent rejects a mismatch (`ADM_PREVIEW_STALE`).

## 6. Operation catalog v1

Notation: `C` command, `Q` query; class is the minimum (Game may raise it, never lower). Permission names are Platform permission constants (§8). `Phase` refers to ADM-02 §8.

### 6.1 Overview and health

| Operation | K | Class | Permission | Phase | Effect / result |
|---|---|---|---|---|---|
| `ops.overview.get` | Q | R0 | `game.ops.read` | 1 | agents, worlds, nodes, players online, DB, backups, alerts |
| `ops.alerts.list` / `ops.alerts.ack` | Q/C | R0/R2 | `game.ops.read` / `game.ops.write` | 2 | active alerts (node down, lag, disk, backup age, error spike) |
| `ops.metrics.series` | Q | R0 | `game.ops.read` | 2 | tick time, players, CPU/RAM/disk, PG pool, queue depth, error rate |
| `ops.logs.tail` | Q | R1 | `game.ops.logs.read` | 2 | structured diagnostic lines (privacy-safe `Line` contract), filter by scope/level/time, ≤ 2 000 lines |

### 6.2 Worlds and channels

| Operation | K | Class | Permission | Phase | Effect / result |
|---|---|---|---|---|---|
| `world.list` / `world.get` | Q | R0 | `game.world.read` | 1 | config, state, channels, revisions (ruleset, content, map, policy) |
| `world.create` | C | R4 | `game.world.admin` | 4 | allocate world id, persistence, map/content bundle and ruleset; Platform half: World Registry entry (closed) |
| `world.configure` | C | R3 | `game.world.config` | 3 | name, PvP type, rates (exp/skill/loot/spawn/regen), premium rules, login limits; versioned, takes effect at next save or restart as declared per field |
| `world.state.set` | C | R3 | `game.world.config` | 2 | `open` / `closed` / `maintenance` (admission policy; kicks optional) |
| `world.save_now` | C | R2 | `game.world.ops` | 2 | flush positions and pending durable state, returns save receipt |
| `world.restart` | C | R3 | `game.world.ops` | 2 | countdown broadcast, drain, save, restart node(s), reopen |
| `world.content.activate` | C | R3 | `game.world.config` | 3 | activate an already-built content/map bundle digest (wraps `oteryn-game-ops` activation issuance); live reload only for content declared hot-reloadable, else at restart |
| `world.channel.assign` / `.revoke` | C | R3 | `game.world.admin` | 3 | channel ↔ node assignment (wraps existing DB control grants and reports) |
| `world.retire` | C | R4 | `game.world.admin` | 4 | close, final save, archive, mark retired in Registry; characters stay transferable |
| `world.delete` | C | R4 | `game.world.admin` | 5 | only for a world with zero characters or after verified export; physical drop of world data |
| `world.merge` | C | R4 | `game.world.admin` | 5 | move characters of world A into B with name-collision and house rules (separate design gate before enabling) |

### 6.3 Nodes and hosts

| Operation | K | Class | Permission | Phase | Effect / result |
|---|---|---|---|---|---|
| `node.list` / `node.get` | Q | R0 | `game.node.read` | 1 | host, build, uptime, assignment, readiness, resource use |
| `node.drain` | C | R2 | `game.node.ops` | 2 | stop admission, let sessions finish or move, seal gameplay |
| `node.start` / `node.stop` / `node.restart` | C | R3 | `game.node.ops` | 2 | via host supervisor; stop = graceful SIGTERM path (flush, readiness withdrawal) |
| `node.update` | C | R4 | `game.node.deploy` | 5 | switch to a signed, pinned build from the release channel; disabled until owner enables deploy |
| `node.enroll` | C | R4 | `game.node.admin` | 4 | issue one-time enrolment token for a new host agent; certificate issued by operator PKI |
| `node.remove` | C | R4 | `game.node.admin` | 4 | drain, revoke assignments and launch authorisation, revoke agent certificate |
| `node.launch_authorization.issue` / `.revoke` | C | R3 | `game.node.admin` | 3 | wraps existing `oteryn-game-ops` launch authorisation |

### 6.4 Database

| Operation | K | Class | Permission | Phase | Effect / result |
|---|---|---|---|---|---|
| `db.status` | Q | R0 | `game.db.read` | 1 | size per schema/table (top 50), connections by role, locks/waits, replication lag, bloat estimate, oldest transaction, cache hit |
| `db.slow_queries` | Q | R0 | `game.db.read` | 2 | `pg_stat_statements` top-N by total/mean time (normalised text only) |
| `db.migrations.list` | Q | R0 | `game.db.read` | 1 | applied, pending in the pinned build, checksum drift |
| `db.migrations.apply` | C | R4 | `game.db.admin` | 4 | apply pending migrations of the pinned build only, maintenance mode required |
| `db.backup.list` / `.verify` | Q/C | R0/R2 | `game.db.read` / `game.db.backup` | 2 | backups with size, age, checksum, restore-test result |
| `db.backup.run` | C | R2 | `game.db.backup` | 2 | logical or base backup now |
| `db.restore` | C | R4 | `game.db.admin` | 4 | restore to a backup or point in time; maintenance mode; follows Restore Reconciliation contract (Platform projections re-epoch) |
| `db.maintenance.run` | C | R3 | `game.db.admin` | 3 | `VACUUM (ANALYZE)`, `REINDEX CONCURRENTLY` on a named table from an allowlist |
| `db.retention.run` | C | R3 | `game.db.admin` | 3 | run registered audit retention jobs (character, staff, bank, …) now |
| `db.query.read` | Q | R1 | `game.db.query` | 3 | single read-only `SELECT` against the **curated `admin_read` schema views** of `oteryn_game` (no base tables, no secrets columns), `READ ONLY` transaction, `statement_timeout` 10 s, ≤ 5 000 rows, full text audited |

There is no write SQL operation. A data fix is a typed command added to this catalog.

### 6.5 Players: accounts, characters, sessions

Account-level data is Platform-owned; the console executes those operations in Platform. Game operations below take `character_id` or, for search, a name prefix.

| Operation | K | Class | Permission | Phase | Effect / result |
|---|---|---|---|---|---|
| `player.search` | Q | R1 | `game.player.read` | 1 | by character name prefix, character id, account ref (from Platform), IP hash (security role) |
| `player.character.get` | Q | R1 | `game.player.read` | 1 | dossier: level, vocation, skills, position, world, online state, premium, guild, house, bank balance, sanctions, last logins |
| `player.inventory.get` | Q | R1 | `game.player.read` | 1 | equipment, containers, depot(s), stash, inbox, with item instance ids |
| `player.sessions.list` | Q | R1 | `game.player.read` | 2 | current and recent sessions (node, channel, start/end, client version) |
| `player.history.get` | Q | R1 | `game.player.read` | 2 | deaths, quest receipts, trades, market, mail, bank operations (from audit/receipts) |
| `player.kick` | C | R2 | `game.player.moderate` | 2 | disconnect now (fixes today's Kick: runtime consumer must fence the session) |
| `player.teleport` | C | R2 | `game.player.moderate` | 2 | to temple / coordinates (walkable, not house of another owner) |
| `player.message` | C | R2 | `game.player.moderate` | 2 | private system message |
| `player.rename` | C | R3 | `game.player.manage` | 3 | name policy check, history kept, Platform projection refreshed |
| `player.stats.set` | C | R3 | `game.player.value.write` | 3 | level/experience, skills, magic level, health/mana, capacity, stamina, blessings; with before/after |
| `player.position.reset` | C | R2 | `game.player.manage` | 2 | offline-safe reset to temple |
| `player.character.restore` | C | R4 | `game.player.value.write` | 4 | restore character aggregate from a backup snapshot (items, skills); never duplicates item instances (re-mint with provenance) |
| `player.gm.grant` / `.revoke` | C | R3 | `game.staff.admin` | 3 | in-game GM rights, scope and expiry; adds signer principal to staff roster at next reload |

### 6.6 Items, currency and economy

| Operation | K | Class | Permission | Phase | Effect / result |
|---|---|---|---|---|---|
| `item.provenance.get` | Q | R1 | `game.economy.read` | 2 | mint → moves → trades → current location of an item instance |
| `item.search` | Q | R1 | `game.economy.read` | 3 | holders of an item type above a count, by world |
| `player.item.grant` | C | R3 | `game.player.value.write` | 3 | mint into inbox/depot with source `ADMIN`; never into a live container without the runtime |
| `player.item.remove` | C | R3 | `game.player.value.write` | 3 | confiscate item instance(s) to an admin escrow (reversible), destroy only by R4 `item.escrow.destroy` |
| `player.currency.adjust` | C | R3 | `game.player.value.write` | 3 | bank adjustment with signed delta, bank audit entry |
| `item.escrow.release` / `.destroy` | C | R3 / R4 | `game.player.value.write` | 3 | return or burn escrowed items |
| `market.offer.cancel` | C | R2 | `game.economy.moderate` | 3 | cancel an offer, return goods/coins |
| `market.freeze` | C | R3 | `game.economy.moderate` | 3 | world market read-only |
| `house.get` / `house.list` | Q | R1 | `game.player.read` | 2 | owner, rent, state, items count |
| `house.evict` / `house.transfer` | C | R3 | `game.player.manage` | 3 | uses house disposition rules; items to owner depot |
| `guild.get` / `guild.disband` / `guild.rename` | Q/C/C | R1/R3/R3 | `game.player.manage` | 3 | guild administration with guild event outbox |

### 6.7 Sanctions

Transport for the accepted Platform Game Enforcement Command contract: same `operation_id`, `sanction_stream_id`, `decision_revision` and restrictive-fencing semantics; ADM-01 only carries it.

| Operation | K | Class | Permission | Phase | Effect |
|---|---|---|---|---|---|
| `sanction.list` | Q | R1 | `game.sanction.read` | 1 | active and historical sanctions of a character/account |
| `sanction.mute` / `.unmute` | C | R2 | `game.sanction.write` | 2 | existing staff mute semantics, any duration up to policy max |
| `sanction.jail` / `.release` | C | R3 | `game.sanction.write` | 3 | jail position and timer |
| `sanction.ban.character` / `.account` | C | R3 | `game.sanction.ban` | 2 | ban with expiry or permanent; immediate session fence |
| `sanction.namelock` | C | R3 | `game.sanction.write` | 3 | force rename at next login |
| `sanction.revoke` | C | R3 | `game.sanction.ban` | 2 | new decision revision lifting the sanction |
| `sanction.notation.add` | C | R2 | `game.sanction.write` | 2 | note visible to staff |

### 6.8 Live world operations

| Operation | K | Class | Permission | Phase | Effect |
|---|---|---|---|---|---|
| `world.broadcast` | C | R2 | `game.world.ops` | 2 | server-wide or world message (rate-limited, length ≤ 255) |
| `world.event.list` / `.start` / `.stop` | Q/C | R0/R2 | `game.world.events` | 3 | raids, world events, boss spawns from the content catalog |
| `world.spawn.creature` | C | R2 | `game.world.events` | 3 | spawn catalog creature at a position (count ≤ policy) |
| `world.feature_flag.set` | C | R3 | `game.world.config` | 3 | toggle a registered feature flag (store purchase mode, market, PvP options) |
| `world.motd.set` | C | R2 | `game.world.ops` | 2 | login message |

### 6.9 Analytics (Game-computed, ANL-02/ANL-03 semantics)

| Operation | K | Class | Permission | Phase | Result |
|---|---|---|---|---|---|
| `analytics.catalog` | Q | R0 | `game.analytics.read` | 1 | available named queries, metric revisions, freshness watermarks |
| `analytics.query` | Q | R0 | `game.analytics.read` | 1 | named query (`query_id`, `params`, time range ≤ 400 d, grain minute/hour/day, ≤ 10 000 points/rows) with completeness/watermark |
| `analytics.player.query` | Q | R1 | `game.analytics.player` | 2 | named query scoped to one character (pseudonym resolved inside Game) |
| `analytics.sql.read` | Q | R1 | `game.analytics.sql` | 3 | one read-only `SELECT` over pseudonymised `analytics_curated` views, 30 s timeout, ≤ 50 000 rows, full text audited |
| `analytics.identity.relink` | Q | R1 | `game.analytics.relink` | 3 | AnalyticsActorId → character, security roles only, ledger + reason required |
| `analytics.export` | C | R2 | `game.analytics.export` | 3 | CSV/Parquet export of a named query to signed short-lived download, audited |

The named query list is in ADM-02 §5.

### 6.10 Audit and administration of the control plane

| Operation | K | Class | Permission | Phase | Result |
|---|---|---|---|---|---|
| `admin.ledger.query` | Q | R1 | `game.audit.read` | 1 | Game `admin_command_ledger` by admin, operation, target, time |
| `admin.agent.list` | Q | R0 | `game.ops.read` | 1 | connected agents, builds, catalog digests, last heartbeat |
| `admin.request.cancel` | C | R2 | same as original | 2 | cancel a queued or cancellable running request |
| `admin.policy.get` | Q | R0 | `game.ops.read` | 1 | Game policy table as the agent enforces it (so Platform can show "requires R4" correctly) |

## 7. Errors

| Code | Retryable | Meaning |
|---|---|---|
| `ADM_UNKNOWN_OPERATION` | no | not in catalog or not advertised by this agent |
| `ADM_SIGNER_UNTRUSTED`, `ADM_BAD_SIGNATURE` | no | signature check failed |
| `ADM_EXPIRED`, `ADM_REPLAY` | no | freshness / nonce |
| `ADM_INVALID_PARAMS` | no | schema violation (details list) |
| `ADM_FORBIDDEN`, `ADM_RISK_CLASS_MISMATCH`, `ADM_CONTROL_MISSING` | no | Game policy refused |
| `ADM_PREVIEW_STALE` | no | R4 executed params differ from previewed |
| `ADM_IDEMPOTENCY_CONFLICT` | no | same operation id, different params |
| `ADM_TARGET_NOT_FOUND` | no | world/node/character unknown |
| `ADM_PRECONDITION_FAILED` | no | wrong state (not in maintenance, character online when offline needed, revision moved) |
| `ADM_TARGET_UNAVAILABLE` | yes | node or DB unreachable from the agent |
| `ADM_BUSY` | yes | conflicting operation running on the same target (per-target lock) |
| `ADM_AGENT_SUPERSEDED` | no | agent owner epoch is stale |
| `ADM_LIMIT_EXCEEDED` | no | query bounds (rows, range, cost) exceeded |
| `ADM_INTERNAL` | yes, once | unexpected failure; ledger records it |

## 8. Platform permission set (new constants)

`game.ops.read`, `game.ops.write`, `game.ops.logs.read`, `game.world.read`, `game.world.ops`, `game.world.config`, `game.world.events`, `game.world.admin`, `game.node.read`, `game.node.ops`, `game.node.admin`, `game.node.deploy`, `game.db.read`, `game.db.backup`, `game.db.query`, `game.db.admin`, `game.player.read`, `game.player.moderate`, `game.player.manage`, `game.player.value.write`, `game.economy.read`, `game.economy.moderate`, `game.sanction.read`, `game.sanction.write`, `game.sanction.ban`, `game.staff.admin`, `game.analytics.read`, `game.analytics.player`, `game.analytics.sql`, `game.analytics.relink`, `game.analytics.export`, `game.audit.read`.

Suggested roles (Platform keeps its closed-list RBAC; `platform_admin` is not a wildcard):

| Role | Permissions |
|---|---|
| `game_viewer` | `*.read` except player/relink/sql, `game.analytics.read` |
| `game_support` | viewer + `game.player.read`, `game.player.moderate`, `game.sanction.read`, `game.sanction.write` |
| `game_master_senior` | support + `game.player.manage`, `game.player.value.write`, `game.sanction.ban`, `game.world.events`, `game.economy.moderate`, `game.economy.read` |
| `game_operator` | viewer + `game.world.ops`, `game.world.config`, `game.node.ops`, `game.db.backup`, `game.ops.write`, `game.ops.logs.read` |
| `game_admin` | operator + `game.world.admin`, `game.node.admin`, `game.db.admin`, `game.staff.admin` |
| `game_dba` | `game.db.*` except admin, `game.analytics.sql` |
| `game_analyst` | `game.analytics.read`, `game.analytics.player`, `game.analytics.sql`, `game.analytics.export` |
| `game_security` | analyst + `game.analytics.relink`, `game.audit.read`, `game.sanction.read` |
| `game_release` | `game.node.deploy` (dormant until owner enables deploy) |

## 9. Audit records

**Platform** (`admin_audit_events`, extended): `request_id`, `admin_operation_id`, `operation`, `risk_class`, `target`, `reason`, `approvals`, `ip`, `user_agent`, `correlation_id`, `outcome`, `game_ledger_id`, `created_at`. Reads of R1 data write one row per view (dossier, inventory, history, re-link, export, SQL).

**Game** (`admin_command_ledger`, append-only, same transaction as the effect when the effect is a DB change): `ledger_id`, `request_id`, `admin_operation_id`, `operation`, `operation_version`, `params_digest`, `canonical_request` (signed bytes), `signer_key_id`, `admin_id`, `risk_class`, `target`, `status`, `error_code`, `effects` (typed list), `started_at`, `finished_at`, `agent_id`. Retention: 2 years minimum, legal hold supported, never in analytics views except as aggregates.

**ANL-01**: new event family `ADMIN_ACTION` (durable, restricted) in `GAME_EVENT_FOUNDATION_REGISTRY.json`; value-changing operations also emit the domain family they touch (item transaction, bank operation, house operation) with source `ADMIN` and the `admin_operation_id` as causation, so economy dashboards see admin mints and removals.

## 10. Conformance tests (both sides)

1. Signature: tampered field, wrong key, expired key, rotated key → rejected with exact codes.
2. Replay of a captured envelope → `ADM_REPLAY`; re-send with same operation id and same params → original result; different params → conflict.
3. Platform claims R2 for an R3 operation → `ADM_RISK_CLASS_MISMATCH`; R4 approved by the requester → `ADM_CONTROL_MISSING`.
4. R4 executed with params different from dry run → `ADM_PREVIEW_STALE`.
5. Agent disconnected → request queued, expires, console shows expired; nothing executes after `expires_at`.
6. `db.query.read` / `analytics.sql.read`: `INSERT`, `UPDATE`, multiple statements, `pg_sleep`, functions outside allowlist, base tables → rejected; timeout and row cap enforced.
7. Item grant then provenance shows `ADMIN` source; economy rollup includes it.
8. Kick / ban: target session is fenced within 2 s while online.
9. Restore: Platform projections re-epoch per Restore Reconciliation contract.

## 11. Versioning

Additive changes (new operation, new optional field) keep `v1`; the agent advertises operations and versions in `hello`. A breaking change to an operation increments `operation_version`; the agent may serve two versions during migration. A breaking change to the envelope requires `oteryn.admin-control.v2`.
