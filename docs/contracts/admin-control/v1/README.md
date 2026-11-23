# Admin control v1

Normative semantics: [ADM-01](../../../architecture/ADM-01_ADMIN_CONTROL_PLANE_CONTRACT.md).

`oteryn-admin-contracts` owns the closed request/result structures and the fifteen
Phase-1 read operation schemas. Schemas are generated with schemars; JCS is RFC
8785 via serde_jcs 0.2, Ed25519 via the workspace's ed25519-dalek. JSON integers
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

There are 35 envelope vectors and 32 schema goldens (two envelopes and
params/result per operation).
Vectors contain deterministic test-only signer material, signed request bytes,
policy, clock and store/precondition evidence. `expected_error: null` means
accepted. The agent unit test regenerates and checks the complete corpus,
including ADM-01 §10 items 1–4. Removed rotation keys are rejected; both current
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
| `player.character.get` | Offline Character root, progression, build, last saved position, account premium and bank projections, guild, house, sanctions, and retained online interval starts. Skill progress percentages are unavailable (zero). Missing saved position or a nonterminal session returns `ADM_TARGET_UNAVAILABLE`. |
| `player.inventory.get` | Offline durable equipment, backpack, depot, and inbox custody trees. Stash balances have no instance IDs and are omitted. Item definitions without a numeric type ID use zero. Characters with a nonterminal session return `ADM_TARGET_UNAVAILABLE`. |
| `sanction.list` | Game staff mute history and the Game projection of Platform sanction decisions. Account-only records require account targeting and global scope. |
| `admin.ledger.query` | Existing Game ledger, restricted to the result fields. Time ranges are half-open and at most 31 days. |
| `admin.policy.get` | Pinned local policy. |

Read executors use short read-only PostgreSQL snapshots and never acquire a
Character writer lease. Offline access requires no nonterminal session or
outstanding Character or account presence holder in the same snapshot as the
data. The existing channel still records its nonce, idempotency and audit
evidence. Retained ADM-G3 results bind the replay digest to the signed operation,
actor and target and the effective ownership policy.

Set the optional `read_db_url_env_name` to the name of a separately provisioned
connection to the same Game database. Provision a dedicated SELECT-only login
for the named source relations with no gameplay writer or control membership.
Grant access only to the columns used by the executors, plus EXECUTE on
`game_account_closure_detached(uuid)`. In particular, ledger reads require only
the fields listed in its result schema and never select canonical requests.
The existing audit connection and role stay dedicated to ADM transport
bookkeeping. An omitted read connection leaves the world roster and overview
fallback available, returns an empty node page, and returns
`ADM_TARGET_UNAVAILABLE` for other database-backed reads. Enable each
operation explicitly in the pinned policy.

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
