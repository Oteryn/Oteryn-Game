# Multi-bot world journeys

Run on a node-boot runner shard with `NODE_BOOT_WORLD=1` and
`NODE_BOOT_E2E_ONLY=mp`. Acceptance is the indexed per-row verdict
`SEAM_EVIDENCE e2e_all scenario=MP01.full status=PASS|FAIL`, together with
`SEAM_EVIDENCE MP_CHECK MP01 <step> PASS|FAIL <detail>` observations.
Compilation is not runtime qualification. Node boot needs Docker and the pinned
Platform checkout; it is run by the owner on the GitHub runner for this packet.

Add one file per mechanic family, named `mp02_<family>.rs`, then MP03, etc.
Add its module declaration in `mount.rs` in name order and its one indexed
`verdict::indexed("MP02.full", ...)` call in ID order. Keep scenario-specific
assertions in that file; add a helper to `bots.rs` only when a scenario uses it.
Every scenario must emit a terminal PASS or FAIL, even after a refusal or a
cleanup error. Unsupported commands fail with their actual status/disposition
and source location. Dependent checks fail explicitly; none silently skip.

MP01 roles own four distinct Accounts, Characters and sessions:

| Bot | Pair/index | Setup | Role |
| --- | --- | --- | --- |
| A | core/0 | knight | party leader, PvP attacker |
| B | packet/0 | druid | invited party member |
| C | packet/1 | paladin | invited party member |
| D | core/1 | **never provisioned** | solo PvP victim, D150 no-vocation path |

Only the existing root-only `provision.py --profile` command sets up A/B/C.
Profiles are explicit, so `NODE_BOOT_PROFILE` cannot change these roles. D's
vocation is checked through its own General Stats command and PostgreSQL;
the relog checks its no-vocation observation again. Equipment, party, movement,
fight modes, attacks, history queries and logout all use each bot's own
`protocol-oteryn` session. Combat tiles come from the pinned world's PZ flags;
the victim's temple comes from its durable hometown and the pinned catalogue.

Assert each participant's own client view and the corresponding durable state.
MP01 queries party membership separately as A, B and C, confirms D stays solo,
and checks the same party/leader/member names through bounded PostgreSQL reads.
Its read-only observers follow the existing qualification `death.rs`
`latest_receipt`/`PgConnection` pattern. They never acquire a fabricated runtime
write authority, update SQL, inject a combat event or synthesize a receipt.
Production fenced party reads require the serving process's current runtime
authority, which this remote protocol harness does not own.

PvP requires D's damage and new death history plus temple respawn, A's attributed
hit and new unjustified kill history, a fresh death receipt and matching durable
frag points, and all four own skull/points views matched against PostgreSQL and
the combat-rules thresholds. Starter D can be below `PVP_MIN_LEVEL=8`
(`crates/gameplay/combat-rules/src/pvp/legality.rs:7,97`). In that case the actual
attack refusal is **FAIL**, never a successful death test; the scenario does not
provision or bypass D's protection. Other server refusals are retained as well.

Relog always uses checked `logout_retrying_busy`; combat blocks expire on real
server time. All logout futures are serviced together, so one killer's natural
15-minute block does not starve the other sessions. New sessions must differ,
positions and death/kill histories must persist, and every participant checks
party/skull state again. Terminal logout removes party membership by design
(`world_party.rs:792`); the expected post-relog party is empty in both client
queries and PostgreSQL. White skull deadlines may expire naturally while
relogging, so compare against the database clock instead of freezing an icon.

Every polling loop has a condition and deadline. Always release partial setup
and failed scenarios, drop remaining sockets, and call `wait_character_released`
for both indexes of **both** pairs. Cleanup failure fails the row. Add no fixed
sleeps, direct SQL mutations, fixture shortcuts, server changes or ignored tests.

Local packet checks (one Cargo job at a time, source `env.sh` in each shell):

```sh
cargo fmt --all --check
rustfmt --edition 2024 --check apps/game-server/src/qualification/client_phases/mp/mount.rs
cargo clippy -p oteryn-game-server --all-targets -- -D warnings
cargo clippy -p oteryn-game-server --all-targets --features qualification-clock -- -D warnings
cargo test -p oteryn-game-server --lib --no-run
cargo test -p oteryn-game-server --lib --no-run --features qualification-clock
```

The explicit rustfmt check follows the block-mounted scenario files, which
workspace rustfmt does not discover through the function-local mount.
