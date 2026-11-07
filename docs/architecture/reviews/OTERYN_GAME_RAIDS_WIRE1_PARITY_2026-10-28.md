# RAIDS-WIRE-1 parity — 2026-10-28

Reference: Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`. Global-visible
behaviour is authoritative where it differs. `UNKNOWN` means the value was not
present in admitted source and is deliberately not guessed.

| Behaviour | Canary evidence (file:line) | Oteryn evidence (file:line) | Status / gap |
|---|---|---|---|
| Badger schedule | `data-otservbr-global/raids/raids.xml:4` | `apps/game-server/src/node/raid_scheduler.rs:12-27` | **ADMITTED** — interval2 500 minutes and margin 60 minutes retained. |
| Badger wave | `data-otservbr-global/raids/abdendriel/badger.xml:3-4` | `apps/game-server/src/node/raid_scheduler.rs:17-26` | **ADMITTED** — delay, creature, amount and exact source rectangle retained. Exact random placement within the rectangle is **UNKNOWN/GAP**. |
| Durable ordered actions | Canary scheduling implementation was not present under the prompt's `src/game/scheduling/raids` path at this pin | `crates/gameplay/encounter-rules/src/raid/runner.rs:35-123`; `crates/gameplay/durability/src/durability/raid_runner.rs:22-100` | **OTERYN NATIVE** — cursor and receipt commit are restart-idempotent and generation fenced. |
| Area-spawn replay identity | `data-otservbr-global/raids/abdendriel/badger.xml:3-4` | `apps/game-server/migrations/0329_raid_runner.sql:22-39` | **PARITY+** — `(firing, channel, wave, entry, index)` prevents duplicate realization. |
| Restart / cleanup | No durable process-restart record identified in the pinned source | `crates/gameplay/durability/src/durability/raid_runner.rs:45-63` | **OTERYN NATIVE** — old generations become LOST and their creatures become CLEANED. |
| Announcements | Canary XML supports broadcast raid events; Badger has none | `crates/gameplay/encounter-rules/src/raid/runner.rs:13-16`; `apps/game-server/src/node/raid_scheduler.rs:46-51` | **PARTIAL** — warning/advance are distinguished and production-driven. Connected-player screen/Server Log fan-out remains a **GAP**. |
| Full Global catalogue | `data-otservbr-global/raids/raids.xml:1-104` and referenced XML files | `apps/game-server/src/node/raid_scheduler.rs:10-28` | **GAP** — only Badger is admitted in this slice; other raids and anchors remain unknown rather than invented. |
| Native schedule UI | Not a Canary server behaviour | `docs/contracts/protocol-oteryn/v1/registry/capabilities/0149-raid-schedule-v1.json:1-11`; `crates/client-ui/src/features/0149-raid-schedule/registrations/handlers.json:1-8` | **UNSUPPORTED** — IDs 171–173 are reserved and unoffered; server-driven play needs no client request. |

The durable creature table is realization evidence, not by itself an in-memory
actor. Binding those rows to bundle-world creature placement and fan-out to
connected clients are explicitly retained gaps; this document does not claim
those visible behaviours are complete.
