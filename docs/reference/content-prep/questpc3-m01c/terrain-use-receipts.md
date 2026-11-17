# M01c: typed native terrain USE receipts

Checked source at `941b023432f0a31a67e3898212d8c9bfe772ea95`:

- `apps/game-server/migrations/0334_terrain_tool_transforms.sql:29` records the tool instance, command, character, scope, placement and result, but not its definition at use time.
- `crates/gameplay/durability/src/durability/terrain_tool.rs:137` replays the existing receipt before resolving live tool custody. Quest observers must likewise use historical evidence rather than reinterpret the instance's later definition.

Migration 2031 captures the definition triple from the existing Item owner during native receipt insertion. Callers cannot supply that snapshot or replace it later. No second terrain writer, item debit or reward is introduced. Historical receipts retain an unknown definition; they are not backfilled from today's Item state and must not qualify a typed USE predicate.

The PostgreSQL test mints and carries a real shovel through the existing owners, commits and replays the terrain action, verifies the captured definition and rejects snapshot replacement. This is an engine proof; no production quest route is claimed complete by this test.

The closed `terrain_use` authored binding reads this native receipt in the existing fenced quest writer. It matches the actor/session/command, exact World and Channel, placement/result and definition key/revision pair; the transform's content generation must match the current activation. Missing/legacy snapshots cannot qualify it. The existing quest receipt ledger prevents one command from advancing two stages of the same quest, while exact replay of its original stage remains idempotent. The PG case exercises these refusals and the successful transition. Production route wiring is a separate step.
