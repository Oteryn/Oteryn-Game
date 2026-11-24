# Admin control v1

Normative semantics: [ADM-01](../../../architecture/ADM-01_ADMIN_CONTROL_PLANE_CONTRACT.md).

`oteryn-admin-contracts` owns the closed request/result structures and the
operation schemas below. The agent advertises and executes
`admin.policy.get` and the ADM-G3 Phase-1 reads listed below. All Phase-2 entries are available for form generation.
Schemas are generated with schemars. JCS uses RFC 8785 via serde_jcs, and
Ed25519 uses the workspace's ed25519-dalek. JSON integers
are restricted to ±(2^53−1); counts/amounts above that range are decimal strings.
The request's `params` remains opaque until verifier step 5, where the operation
selects its closed schema. The envelope result union contains only closed
operation results. Schema files have deterministic names and formatting.
Hello uses the base64url SHA-256 digest of JCS of the schema filename/content
map as `catalog_digest`; advertised operations carry their version separately.

Regenerate intentionally, then inspect the diff:

```sh
UPDATE_ADMIN_GOLDENS=1 cargo test -p oteryn-admin-contracts -p oteryn-admin-agent
cargo test -p oteryn-admin-contracts -p oteryn-admin-agent
```

The 86 schema goldens cover two envelopes, 35 operation pairs, and seven
named-query pairs. Phase-1 envelope vectors live directly under `vectors/`.
The 52 Phase-2 vectors in `vectors/phase-2/` retain the signed-envelope fixture
layout and add `result` and `expected_result_valid`. Their `expected_error`
describes parameter validation with the operation available. The agent returns
`ADM_UNKNOWN_OPERATION` for every Phase-2 request because hello omits all of them.

Vectors contain deterministic test-only signer material, signed request bytes,
policy, clock and store/precondition evidence. `expected_error: null` means
accepted. The agent unit test regenerates and checks the Phase-1 corpus, including ADM-01
§10 items 1–4. The contracts crate checks the Phase-2 signatures, schema
goldens, and positive and negative payloads. Removed rotation keys are rejected; both current
and next trusted keys are accepted within their validity windows.

ADM-01 leaves read fields and pagination unspecified. This implementation fixes
those fields in the exported schemas: list calls require a limit of 1–100 and
an optional opaque cursor, search requires a filter, and inventory is flattened
with location and parent ids. ADM-G3 implements the Phase-1 reads below.
`admin.agent.list` and `analytics.catalog` are outside that slice.

| Operation | Game source and unavailable fields |
|---|---|
| `ops.overview.get` | Agent ownership config, scope assignments, durable node and session presence, and read DB connectivity. Agent fleet and backup telemetry are unavailable (empty agents, null backup age). A missing read connection sets `db_available=false` and adds an alert. |
| `world.list`, `world.get` | Configured owned World IDs and current Channel assignments. Registry name, admission state, and World-wide revisions are unavailable (empty strings and state `unavailable`). |
| `node.list`, `node.get` | Current node registrations, scope assignments, and matching readiness generations. Uptime measures time since registration. Host, build, and resource telemetry are unavailable (empty strings and zero CPU). |
| `db.status` | PostgreSQL catalog and statistics, including the largest 50 tables. Bloat is unavailable. Lag is replica replay age when available. Oldest transaction age covers activity visible to the read role. |
| `db.migrations.list` | SQLx migration history compared with the Game migrations embedded in the agent build. |
| `player.search` | Character roots and progression with matching durable session presence. IP hash lookup is unavailable. |
| `player.character.get` | Offline Character root, progression, build, last saved position, account premium and bank projections, guild, house, sanctions, and retained online interval starts. Skill progress percentages are unavailable (zero). Missing saved position returns `ADM_TARGET_UNAVAILABLE`. The offline guard returns `ADM_PRECONDITION_FAILED` for a nonterminal session or outstanding holder. |
| `player.inventory.get` | Offline durable equipment, backpack, depot, and inbox custody trees. Stash balances follow instance rows in the same pagination stream. Each balance contains a type ID and decimal-string count. Item definitions without a numeric type ID use zero. The same offline guard applies. |
| `sanction.list` | Game staff mutes and account bans, plus the Game projection of Platform sanction decisions. Character bindings must be visible in scope. An account-wide sanction requires a visible Character of that account in scope, matching any signed Character selector. |
| `admin.ledger.query` | Existing Game ledger, restricted to the result fields. Time ranges are half-open and at most 31 days. |
| `admin.policy.get` | Pinned local policy. |

The database operations `db.status` and `db.migrations.list` and the local
`admin.policy.get` are global and require an empty signed target. All other
registered reads use the intersection of the operation's World allowlist and
the agent's configured Worlds, narrowed by any signed World selector. An
empty allowlist disables a World-scoped operation. Applicable signed selectors
and parameter filters apply together.

Read executors use short read-only PostgreSQL snapshots under the signed
deadline, including pool acquisition. Character reads check current visibility
and durable session holders before idempotency and use the same snapshot for
any new result. Executor failures are audited without retaining a result.
Retained ADM-G3 results bind the replay digest to the signed operation, actor
and target and the effective ownership policy.

Set the optional `read_db_url_env_name` to the name of a separately provisioned
login's connection URL for the same Game database. The deployed read login
needs CONNECT on the database and membership in the `oteryn_game_admin_read`
NOLOGIN role. The migration
grants that role SELECT on the curated `admin_read` views and usage of that
schema. Ledger views expose the result fields and omit canonical requests.
The existing audit connection and role stay dedicated to ADM transport
bookkeeping. An omitted read connection leaves the world roster and overview
fallback available, returns an empty node page, and returns
`ADM_TARGET_UNAVAILABLE` for other database-backed reads. Enable each
operation explicitly in the pinned policy.

## Operations

Q denotes a query and C denotes a command. The table lists minimum risk tiers.

| Operation | Kind | Risk | Permission | Phase |
|---|---|---|---|---|
| `admin.agent.list` | Q | R0 | `game.ops.read` | 1 |
| `admin.ledger.query` | Q | R1 | `game.audit.read` | 1 |
| `admin.policy.get` | Q | R0 | `game.ops.read` | 1 |
| `analytics.catalog` | Q | R0 | `game.analytics.read` | 1 |
| `db.migrations.list` | Q | R0 | `game.db.read` | 1 |
| `db.status` | Q | R0 | `game.db.read` | 1 |
| `node.get` | Q | R0 | `game.node.read` | 1 |
| `node.list` | Q | R0 | `game.node.read` | 1 |
| `ops.overview.get` | Q | R0 | `game.ops.read` | 1 |
| `player.character.get` | Q | R1 | `game.player.read` | 1 |
| `player.inventory.get` | Q | R1 | `game.player.read` | 1 |
| `player.search` | Q | R1 | `game.player.read` | 1 |
| `sanction.list` | Q | R1 | `game.sanction.read` | 1 |
| `world.get` | Q | R0 | `game.world.read` | 1 |
| `world.list` | Q | R0 | `game.world.read` | 1 |
| `analytics.query` | Q | R0/R1 | `game.analytics.read` | 2 |
| `node.drain` | C | R2 | `game.node.ops` | 2 |
| `ops.alerts.ack` | C | R2 | `game.ops.write` | 2 |
| `ops.alerts.list` | Q | R0 | `game.ops.read` | 2 |
| `ops.logs.tail` | Q | R1 | `game.ops.logs.read` | 2 |
| `ops.metrics.series` | Q | R0 | `game.ops.read` | 2 |
| `player.kick` | C | R2 | `game.player.moderate` | 2 |
| `player.message` | C | R2 | `game.player.moderate` | 2 |
| `player.teleport` | C | R2 | `game.player.moderate` | 2 |
| `sanction.ban.account` | C | R3 | `game.sanction.ban` | 2 |
| `sanction.ban.character` | C | R3 | `game.sanction.ban` | 2 |
| `sanction.mute` | C | R2 | `game.sanction.write` | 2 |
| `sanction.notation.add` | C | R2 | `game.sanction.write` | 2 |
| `sanction.revoke` | C | R3 | `game.sanction.ban` | 2 |
| `sanction.unmute` | C | R2 | `game.sanction.write` | 2 |
| `world.broadcast` | C | R2 | `game.world.ops` | 2 |
| `world.motd.set` | C | R2 | `game.world.ops` | 2 |
| `world.restart` | C | R3 | `game.world.ops` | 2 |
| `world.save_now` | C | R2 | `game.world.ops` | 2 |
| `world.state.set` | C | R3 | `game.world.config` | 2 |

### Economy query forms

Use `operation: "analytics.query"` and put the query ID inside the signed
`params.query_id`. Each pair uses the filename stem `analytics.query.<query_id>`.
Use a world target and a window of at most 400 days at minute, hour, or day
grain. The analytics executor will enforce the window and the 10,000-point or
row limit. The result carries the applied filters, metric revision,
completeness, a nullable freshness watermark, and world age, active character
count, and revision context. ANL-03's completeness, privacy, and valuation
rules apply to the data returned by a future executor.

| Query ID | Risk | Rows |
|---|---|---|
| `gold_supply` | R0 | Gold supply per bucket |
| `gold_sources_sinks` | R0 | Gold minted and burned per bucket and source |
| `item_supply_by_type` | R0 | Item quantity per bucket and item type |
| `market_volume` | R0 | Executed trade count, quantity, and gold volume |
| `market_price_series` | R0 | Minimum, median, and maximum executed unit price for an item type |
| `npc_trade_flow` | R0 | Item and gold flows in each direction by NPC and item type |
| `top_wealth` | R1 | Pseudonymous gold ranking, limited to 1–100 entries |

### Phase-2 form choices

- Use the envelope's character and world target for kick, teleport, and
  private message. Kick takes the expected session generation as a decimal
  string. The future executor must atomically compare that generation and
  fence only the matching session. Teleport takes a tagged temple or coordinate
  destination. The executor checks walkability and house access.
- Private message, broadcast, and MOTD use a maximum of 255 Unicode
  characters. An empty MOTD clears it. Broadcast uses the envelope world
  target when supplied, otherwise it has server-wide scope.
- Save returns a receipt. Drain targets a node and reports whether admission
  closed, gameplay sealed, and how many sessions remain. Restart requires a
  countdown of 1–3,600 seconds and returns the restarted node IDs, save receipt,
  and final state. Drain and restart use envelope progress updates. World state
  takes an optional session-kick flag and expected revision.
- Sanction decisions carry the enforcement operation ID, stream ID, and
  decision revision. Lifts also carry the prior decision revision. The operation
  ID binds to the envelope's `admin_operation_id`. Mute expires, while an omitted
  or null ban expiry represents a permanent ban. Mute and unmute use the
  existing Enforcement Command v1 MUTE payload. Ban and revoke carry the
  decision fields for ADM forms, pending a ban profile in Enforcement Command.
  The operations use the distinct IDs and permissions in ADM-01. Notations are staff
  notes of up to 1,024 characters. ADM decision summaries carry the profile,
  resulting state and runtime effect. The enforcement consumer owns ordering,
  scope and immediate fencing.
- Metrics select one named series at a time. Null point values represent
  missing samples. Alerts list active records with an opaque cursor and
  acknowledge one alert by ID. Log tail filters a time window by target
  world/node, channel and level. It returns at most 2,000 structured diagnostic
  lines containing the bounded token fields. The producer must enforce
  privacy-safe redaction.

## Agent wire behavior
The pure verifier accepts immutable nonce/idempotency/precondition snapshots.
The driver authenticates first, atomically claims the PostgreSQL nonce, runs
steps 5–8, and only then loads idempotency. Failed checks after step 4 consume
the nonce. R4 preview evidence must be computed by an executor, never trusted
from the request; no Phase-1 executor produces R4 effects. The negative preview
vector uses a policy raised to R4 to exercise that guard. Unpaginated read
params allow the optional `preview_digest` control field so
a policy raised to R4 can exercise the same preview comparison. Future R4
command schemas must also include it before they can be advertised.

Agent configuration adds build/ownership/world/node metadata for hello and a
separate Ed25519 signing seed path for signed results. mTLS identities are PEM;
the signing seed is an unpadded base64url 32-byte secret, provisioned separately.
Policies are parsed and pinned at process startup; public policy results exclude
signer material. The outbound HTTP client enforces HTTPS, mTLS, no redirects,
30-second deadlines, and a 1 MiB response cap. Channel batches are capped at 32;
heartbeats run every 10 seconds during polling and request/result processing.
Supersession and transport
failures terminate the process for supervision; they never silently reissue an
operation with a new id. Request results and their ledger entry commit together.
Rejected requests are audited without retaining an idempotent result. If the
audit DB is unavailable or request identifiers are malformed, the signed
failure carries no ledger id. Such failures never claim a persisted audit row.

ADM-01 does not fix endpoint body framing. This client sends hello/heartbeat
metadata objects, polls with `agent_id`, `owner_epoch`, `wait_seconds: 25` and
`limit: 32`, consumes a JSON array of request envelopes, and posts each result
as one result envelope. Empty successful responses are accepted. HTTP 409 is
mapped to `ADM_AGENT_SUPERSEDED`; Platform must use that status for stale epochs.
These wire choices must be matched by ADM-P1; no Platform code is changed here.

PostgreSQL checks use an isolated local PostgreSQL 18.6 database, cleaned up
afterward, with the repository test-admin environment route. To require actual
execution instead of the route-less branch:

```sh
REQUIRE_ADMIN_PG=1 cargo test -p oteryn-admin-agent postgres_ledger_nonce_and_idempotency -- --nocapture
```

The explicit `ADMIN_PG_EXECUTED` marker is emitted only after real store,
append-only trigger, transaction rollback and role privilege checks succeed.
Migration 0956 retains ledger rows indefinitely (at least two years), includes
legal-hold metadata, and grants the agent no gameplay table privileges.
