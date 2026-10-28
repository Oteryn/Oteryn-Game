# BOOSTED-1 final report

## What was wired

| Production call site | Wiring |
|---|---|
| `crates/gameplay/content/src/content/daily_boost.rs` | Deterministic, order-independent selection keyed by World id and durable server-save reset epoch; race-scoped rational multiplier primitive. |
| `apps/game-server/migrations/0333_daily_boost.sql` | Durable one-row-per-World/reset-epoch storage with a foreign key to the real reset boundary. |
| `crates/protocol-oteryn/src/daily_boost.rs` | Bounded client query/view codec and capability allocation. The capability remains deliberately unoffered until a dispatch owner reads the durable row. |

The production XP, loot, respawn, selection transaction, and query dispatch call sites are **not
wired**. This delivery is therefore PARTIAL rather than falsely claiming completion. The missing
piece is a reset-owner transaction which projects eligible creature/boss keys from the active
content generation, inserts the selection, and publishes the committed view to Channel runtime.
Without that committed view, applying the helper in reward or respawn code would silently redraw
or depend on process time and would violate the task's restart requirement.

## Parity

| Behaviour | Crystal / Canary | Oteryn | Status |
|---|---|---|---|
| Daily creature selection survives restart | Canary persists `boosted_creature`; Crystal exact value not reachable at the pinned revision | Deterministic selector and schema exist; reset transaction missing | PARTIAL |
| Daily boss selection survives restart | Canary persists `boosted_boss` | Deterministic selector and schema exist; reset transaction missing | PARTIAL |
| XP only for selected race | Reference multiplier value was not verified | Race-scoped configurable rational helper; live kill consumer missing | PARTIAL |
| Loot only for selected race | Canary config exposes a boosted-boss loot bonus; full semantics unverified | Race-scoped configurable rational helper; live loot consumer missing | PARTIAL |
| Faster respawn only for selected race | Exact pinned multiplier unverified | Race-scoped configurable rational helper; live spawn consumer missing | PARTIAL |
| Client query | Reference advertises both selections | Codec/capability allocated but intentionally unoffered and unrouted | PARTIAL |

No unverified multiplier was invented.

## Files changed

- `BOOSTED-FINAL.md`
- `apps/game-server/migrations/0333_daily_boost.sql`
- `crates/gameplay/content/src/content/daily_boost.rs`
- `crates/gameplay/content/src/content/mod.rs`
- `crates/protocol-oteryn/capabilities/0147-daily-boost-v1.json`
- `crates/protocol-oteryn/src/daily_boost.rs`
- `crates/protocol-oteryn/src/lib.rs`

## Commands and results

- `cargo fmt --all --check`
- `cargo test --locked -p oteryn-gameplay-content daily_boost --quiet`
- `cargo test --locked -p oteryn-protocol-oteryn daily_boost --quiet`
- `cargo clippy --locked -p oteryn-gameplay-content --all-targets --quiet -- -D warnings`
- `cargo clippy --locked -p oteryn-protocol-oteryn --all-targets --quiet -- -D warnings`
- `python3 tools/agents/validate_governance.py`
- `git diff --check`

Final command results are reported in the commit/PR delivery response.

## PostgreSQL tests not run

No PostgreSQL test was added: the production reset transaction is not yet present, so a database
test purporting to prove restart stability would test only the schema and misrepresent the lane.

## Proposed Conventional Commit

`feat(daily): establish deterministic boosted selection contract`
