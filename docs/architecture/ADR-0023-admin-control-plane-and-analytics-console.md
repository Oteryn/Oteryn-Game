# ADR-0023: Admin control plane and analytics console in Oteryn Platform

- Status: Accepted direction (owner request 2026-11-05); contract `ADM-01` and delivery plan `ADM-02` govern implementation
- Date: 2026-11-05
- Decision owner: Oteryn project owner
- Coordination ID: `OTV2-ADMIN-CONTROL-PLANE`
- Related: ADR-0003, ADR-0004, ADR-0006, ADR-0009, ADR-0012, ADR-0013, ANL-01, ANL-02, ANL-03, Platform ADR 0006 (admin RBAC and audit), Platform contracts `OTERYN_V2_GAME_ENFORCEMENT_COMMAND_CONTRACT.md`, `OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`, `OTERYN_V2_WORLD_TOPOLOGY_CONTRACT.md`, `WORLD_REGISTRY_CONTRACT.md`
- Companion documents: `ADM-01_ADMIN_CONTROL_PLANE_CONTRACT.md`, `ADM-02_ADMIN_CONSOLE_DESIGN_AND_DELIVERY_PLAN.md`

## Context

The owner requires one administration panel, hosted in Oteryn Platform, from which an administrator can run the whole game service:

- complete analytics of the game server (players, economy, gameplay balance, world/content usage, security, operations);
- total server management: worlds and channels (create, change, retire), game nodes (add, remove, drain, restart, update), maintenance and server save, configuration and rates, content and events;
- database management: health, size, backups and restores, migration state, slow queries, bounded read-only data exploration;
- player management: accounts, characters, sessions, sanctions, inventories, items and currency, houses, guilds, market, support history.

The Platform implementation may be built by the Platform chat session, but it must work with the Game side defined here.

Facts that constrain the design (evidence in `ADM-02` §1):

1. **Platform never opens a connection to Game.** Every existing integration is either Game pushing projections to Platform `/internal/v1/*` over mTLS, or Game pulling Platform-stored intents (character commands, bootstrap intents, premium snapshots) and posting receipts. Platform has no outbound HTTP client by design.
2. **Platform already has an admin console**: server-rendered Blade, closed-list RBAC (`admin_roles`, `admin_permissions`), mandatory confirmed TOTP MFA on every admin route, append-only `admin_audit_events`. It has no step-up re-authentication, no charts, no queue/Redis, and no game-operations pages.
3. **Game data in Platform is latest-state only** (projections are upserted), so Platform cannot provide history or analytics from what it stores today.
4. **ADR-0006** already fixes the analytics data classes (operational observability, best-effort telemetry, durable economy/security audit), pseudonymous `AnalyticsActorId`, read-only investigation, and the ANL-01..ANL-04 gates.
5. **ADR-0004 / ADR-0013**: Game data lives in Game-owned PostgreSQL; Platform must never write game tables with SQL; cross-boundary mutation goes through explicit contracts.
6. Deploy and node lifecycle today run through host scripts (`deploy/synology-game/supervisor.sh` here; `deploy/oracle/game-supervisor.py`, `game-backup.py`, `check-game-db.py` in Platform's deploy tree) and the root-only `oteryn-game-ops` CLI (launch authorisation, node registration, channel assignment, content activation), not through any UI.
7. Game already has a **narrow signed staff path**: Ed25519 (OpenSSH) signer roster pinned at restart, principals mapped to GM/SeniorGM/Admin, request authenticated before parsing, operation + effect + `STAFF_ACTION` audit in one transaction (`crates/gameplay/durability/src/durability/staff/`). Its vocabulary is only Inspect/Mute/Unmute/Kick, and Kick has no runtime consumer that disconnects the target.
8. Game has **no** metrics exporter, no HTTP listener, no admin API, no analytics consumer and no analytics database. Its audit outboxes (item, bank, house, market, mail, stash, staff, guild) store durable evidence, but only the character/staff retention loops and the account/public projection publishers actually run. Several retention profiles in `GAME_EVENT_FOUNDATION_REGISTRY.json` explicitly exclude analytics use.

"Total management" therefore cannot mean "Platform gets SQL access to the game database" or "Platform gets a shell on game hosts". It means: **every administrative capability that is possible is exposed as a typed, authorised, audited operation, and the panel is the single place to invoke all of them.**

## Decision

### 1. One Game-owned admin control plane, one Platform-owned console

```text
 Browser (admin, MFA)                                    Game side (private network)
        |                                                 +--------------------------------+
        v                                                 | oteryn-admin-agent (per host)  |
 Oteryn Platform  ---- admin console (Blade) ----         |  - verifies signed requests    |
   - RBAC + step-up MFA + 4-eyes approvals                |  - command & query catalog     |
   - admin_command_intents / results / audit  <== mTLS ===|  - outbound long-lived channel |
   - analytics read cache (rollups)            long-poll  |  - calls node / DB / supervisor|
   - signs every request (Ed25519 admin key)    channel   +---------------+----------------+
                                                                          |
                                                 +------------------------+-------------------------+
                                                 |                        |                         |
                                          GameNode admin port      oteryn_game PostgreSQL     host supervisor
                                          (local socket, typed      (admin role: typed        (node lifecycle,
                                           commands into the        procedures; analytics     backups, deploy of
                                           authoritative runtime)   replica: read-only)       a pinned build)
```

- **Game owns the admin control plane**: a new binary `oteryn-admin-agent` in this repository (crate `crates/admin/agent`, protocol types in `crates/admin/contracts`). It is the only component that executes administrative operations against game state, game nodes, the game database or game hosts.
- **Platform owns the console**: UI, operator identity, RBAC, MFA/step-up, approval workflow, the durable record of every request and result, and the presentation of analytics.
- **The agent reuses the staff-command precedent**: Platform's admin signing key is a principal in a Game-side signer roster (pinned at agent start, like the staff roster), and executed operations follow the staff executor pattern (signed request, operation, effect and audit in one transaction).
- **Game stays authoritative.** The agent never applies a change by writing tables behind the runtime's back while a world is live. Live-state changes go into the running GameNode through a typed admin command port and use the same transaction, fencing and outbox rules as gameplay (DUR-02/DUR-03, ANL-01 durable audit). Offline-only operations (restore, schema migration, world creation before first boot) are explicit maintenance-mode operations.

### 2. Transport: Game-initiated reverse channel (Platform still never calls Game)

The agent opens and keeps an **outbound** mTLS connection to Platform: `POST /internal/v1/admin-control/channel` (HTTP/2 long-poll, or WebSocket where the deployment allows), authenticated with a dedicated client certificate whose subject names the agent and the hosts/worlds it serves.

- Platform writes each admin request to `admin_control_requests` and hands it to the waiting channel; the agent executes it and posts the result on the same channel (`/internal/v1/admin-control/results`).
- Interactive reads (player search, inventory view, live dashboards) complete in well under a second while the channel is up; when no agent is connected the console shows the target as **unreachable** and mutating requests stay queued with an explicit expiry, never silently dropped.
- This keeps Platform's rule (no outbound client to Game), needs no inbound port on game hosts, works through NAT (Oracle VM, Molehill), and reuses the existing mTLS ingress (`deploy/synology/nginx/internal-mtls.conf`) with a new allowlisted path and a new client identity.
- Rejected: Platform calling a Game admin HTTP API directly (would add Platform's first outbound client, an inbound attack surface on game hosts and a second trust direction); plain intent polling at multi-second intervals (too slow for interactive admin work).

### 3. Every request is signed, typed, bounded and audited twice

- **Typed catalog, no arbitrary execution.** The agent accepts only operations listed in the `ADM-01` catalog: `command` (mutates) or `query` (reads), each with a versioned schema, a required permission, a risk class and bounds. There is no free SQL write path, no shell, no file write and no arbitrary RPC.
- **Signed by Platform, verified by Game.** Every request carries an Ed25519 signature from Platform's admin signing key over the canonical request (operation, version, parameters, target, admin identity, granted permission, MFA/step-up timestamps, approval references, nonce, issued/expiry time). The agent verifies the signature, freshness, nonce uniqueness and that the claimed permission is sufficient for the operation's risk class **under Game's own policy table**. A compromised channel alone cannot execute anything; a compromised Platform can only request catalogue operations, and the high-risk ones additionally need two distinct approving admins whose approvals are part of the signed payload.
- **Idempotent.** Mutating commands carry `admin_operation_id`; retries converge on the first result; a different payload under the same id fails closed (same rules as the Game Enforcement Command contract).
- **Audit on both sides.** Platform records who asked, why (mandatory reason for risk ≥ medium), approvals and result in `admin_audit_events` (extended with request id, correlation id, IP, user agent, outcome). Game records every executed command in `admin_command_ledger` (durable, append-only, in the same transaction as the effect where the effect is a database change) and emits an ANL-01 durable audit event with source `ADMIN` for item, currency, character, sanction and world-state changes. Reads of player-linked data are audited (who viewed which character/account, when).

### 4. Risk classes and controls

| Class | Examples | Controls |
|---|---|---|
| R0 read, aggregate | dashboards, rollups, world/node status | permission |
| R1 read, player-linked | character/inventory view, session list, item provenance | permission + read audit + PII masking by default |
| R2 reversible change | broadcast, kick, mute, teleport, start/stop event, toggle feature flag, drain node | permission + reason |
| R3 durable value or sanction | grant/remove items or currency, ban, rename, edit skills/level, house reassign, rates/config change | permission + reason + step-up MFA (TOTP ≤ 5 min) |
| R4 destructive or service-wide | delete/retire world, remove node, restore backup, run schema migration, rollback character state, deploy a build, wipe | permission + reason + step-up + second admin approval (4-eyes) + maintenance window where applicable + dry-run preview required |

Game holds the authoritative mapping operation → minimum class; Platform cannot downgrade it.

### 5. Analytics: Game computes, Platform presents

- **Collection** follows ADR-0006 and ANL-01 unchanged: low-cardinality operational metrics (a Prometheus exporter on a loopback/agent-only port, to be built: none exists today), best-effort gameplay telemetry (login/logout, combat, quest and session streams, to be built), durable economy/security audit through the existing transactional outboxes.
- **Purpose authorisation**: an outbox whose retention profile excludes analytics is not read by the analytics worker until the event registry records an analytics purpose (pseudonymised, aggregate) for that family. This is a registry change reviewed per family, not a blanket permission.
- **Storage**: a Game-owned analytics database `oteryn_analytics` (PostgreSQL, separate database and roles from `oteryn_game`, fed by a new `oteryn-analytics-worker` that consumes the outbox and telemetry queues and maintains time-partitioned fact tables and minute/hour/day rollups). PostgreSQL with native partitioning is the default; a columnar store (ClickHouse/Timescale) is a later, measured decision when raw telemetry exceeds the thresholds in `ADM-02` §6, and must keep the same query catalog.
- **Access**: the console reads analytics only through **named queries** in the `ADM-01` query catalog (dashboard tiles, time series, breakdowns, top-N, drill-downs, provenance traces), each with bounded time range, row limit and cost guard, executed by the agent against the analytics replica with a read-only role.
- **Bounded ad-hoc exploration**: an `analytics.sql.read` query (R1, analyst/DBA roles) runs a single read-only `SELECT` against curated, pseudonymised analytics views (never `oteryn_game` tables), in a read-only transaction with statement timeout, row cap and audit of the full text. Identity re-linking (AnalyticsActorId → CharacterId) is a separate R1 operation for security roles only, as ADR-0006 §9 requires.
- **Platform cache**: Platform may cache query results and periodic rollup pushes (`/internal/v1/admin-control/rollups`) for fast dashboards, with the source revision and freshness shown. Platform never computes game analytics from raw game data and never becomes the analytics source of truth.
- **Platform-owned metrics** (registrations, logins, payments, store, bazaar, tickets) are computed in Platform from Platform data and shown on the same dashboards, labelled as Platform-sourced.

### 6. Server, world and database management

- **World lifecycle** (create, configure, open/close, maintenance, retire) is one orchestrated operation across both owners: Platform World Registry (identity, public status, login policy) and Game (world configuration, map/content bundle, channels, persistence). The agent executes the Game half; the Platform half follows the existing registry/topology contracts. A world is never deleted while it holds characters: retire means closed + archived; physical deletion is R4 and requires an empty or exported world.
- **Node lifecycle** (add, remove, drain, restart, update to a pinned build, assign channels) goes through the host supervisor via the agent on that host. Adding a host means enrolling a new agent (one-time enrolment token issued by an R4 command, certificate issued by the operator PKI); the panel then manages it like any other.
- **Deploy** of a new build from the panel is an R4 operation that only deploys an already-built, signed artefact from the release channel; it stays disabled until the owner's explicit deploy decision enables it (current standing rule: no deploy until the owner says "deploy").
- **Database management**: status (size, connections, replication lag, locks, slow queries, bloat), backups (list, run now, verify, download manifest), restore (R4, maintenance mode, restore reconciliation per `OTERYN_V2_GAME_RESTORE_RECONCILIATION_CONTRACT.md`), migrations (list applied/pending, apply pending only from the pinned build, never edit), maintenance (vacuum/analyze/reindex scheduling), retention jobs. No interactive write SQL on `oteryn_game`; data fixes are typed commands.

### 7. Player management

Player operations use the owner that holds the data:

- Platform-owned (account email, MFA reset, account lock, premium time, payments, bazaar, tickets) are executed by Platform directly, as today.
- Game-owned (characters, sessions, positions, inventories, depots, items, currency, skills, houses, guilds, market offers, sanctions in game) are `ADM-01` commands executed by the agent through the GameNode admin port when the character's world is live, or through offline typed procedures when it is not.
- Sanctions reuse the accepted **Game Enforcement Command contract** semantics (operation id, sanction stream, decision revision, restrictive fencing); `ADM-01` is its transport.
- Every value-changing operation (items, currency, experience, skills) produces durable audit with source `ADMIN`, the admin operation id and the reason, and is visible in item provenance and economy integrity dashboards.

### 8. In-game gamemasters

In-game GM characters remain possible, but GM rights are granted only by an R3 panel operation, are scoped and time-limited, and every GM action is logged through the same ledger. The panel is the primary admin surface; in-game GM tools are a convenience on top of the same command catalog.

## Consequences

### Positive

- One panel can operate everything that is safe to operate, without giving Platform SQL or shell access to the game.
- The trust direction of the existing architecture is preserved: Platform never calls Game; Game remains authoritative; Game verifies every privileged request itself.
- Admin changes to value and sanctions are auditable end to end and visible to the economy-integrity analytics.
- Analytics history and scale are owned where the data is produced; Platform stays simple (no queue/Redis requirement for the first phases).

### Costs and risks

- A new privileged component (agent) with its own certificate, signing key verification and catalog; it needs security review and abuse tests before production enablement.
- The reverse channel needs monitoring (connected agents, request latency, queued/expired requests).
- Each new admin capability needs a catalog entry, schema, Game implementation and tests; "everything possible" is delivered in phases (`ADM-02` §8), not at once.
- Analytics storage and rollups add operational load; thresholds for moving to a columnar store must be measured.

## Rejected alternatives

- **Platform reads/writes `oteryn_game` directly** (shared DB credentials or a DB console): violates ADR-0004/0013 and bypasses fencing, outbox and audit; one SQL mistake can duplicate items or corrupt sessions.
- **Platform calls a Game admin HTTP API**: reverses the established trust direction, needs an inbound port on game hosts, and adds Platform's first outbound client.
- **Grafana/pgAdmin exposed as "the admin panel"**: no business authorisation, no typed operations, no audit of value changes; acceptable only as internal operator tooling behind VPN, not as the product admin panel.
- **Arbitrary SQL console for writes**: rejected for the same reason; ad-hoc reads are limited to curated read-only analytics views.
- **In-game GM commands as the main admin surface**: weak authorisation and audit, no analytics, no infrastructure control.

## Not performed by this ADR

It creates no code, migration, endpoint, certificate, role or production activation. The Platform side requires its own Platform ADR (next free number) accepting the channel endpoint, the signing key, the new permissions and the console modules. Production enablement of each phase requires the owner's explicit decision.
