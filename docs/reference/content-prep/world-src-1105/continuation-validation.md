# WORLD-SRC continuation validation

Historical1fa04bbb continuation; the later requested srvpkg3 rebase is recorded
in `srvpkg3-rebase-validation.md`.

Base: `1fa04bbbddb5d96e21fecca0bc456f3c5e1109b7`. No commit or push.
Current continuation results are recorded below; `validation.md` is historical c8 evidence.

## Scope and ownership

W05b: source-backed Fury Gates/Nightmare native geometry and spawn admission,
real STEP callbacks, recurring Flower waves with durable scheduler receipts.
W12: canonical native subareas, preserved legacy completion/POIs, once-only legacy
pool import and existing main donation carry/active-day behavior.
Main client handlers are unchanged. Reused capability57 and commands72/73;
no new capability bit or command ID. Migration0963 was free and is the only new migration.

The final fetched open-package exclusion union is402 paths, with zero delivery
overlap. Exact heads/merge bases are in `continuation-integration.md`.
Ruprecht passed an isolated PostgreSQL implementation experiment, but its production
binding/conversation owners became protected in the final srvpkg3 refresh. The
experiment's code, SQL extensions, deployment package and placement were removed;
its sources and bounded findings remain. It is not a delivered production mechanic.
Cake HP execution also needs a protected owner; other holiday producers and map
speed retain exact missing facts/refused Wiki URLs in their continuation notes.

## RED evidence and final GREEN checks

The received patch's actual RED traces remain `/tmp/world-src/gates-red.log`,
`gates-offday-red.log`, `flower-physical-red.log`, `w12-conversion-red.log`:
respectively real native-entry/off-day movement, physical wave activation, and
retention of the legacy source failed before their implementations. W12's original
carry RED is historical because main now already supplies carry behavior.

All Cargo commands use the prepared environment, `--locked -j1` and repository
profiles. No installation, profile override, linker substitution or altered
RUSTFLAGS/incremental setting was used. PostgreSQL credentials are not recorded.
The database-backed tests create their own disposable databases and apply the
full real migrations, including0963. Completion markers appear only after assertions.

| Current check | Result |
| --- | --- |
| world-bundle writer/strict manifest |5 passed |
| daily world-change rules |15 passed |
| world native composition, including simultaneous SP02 fungus and Fury |218 passed,1 existing ignored |
| encounter rules |61 passed |
| entities (`test-support`) |251 passed |
| durability unit lane |679 passed |
| raid authoring Python |16 passed |
| raid catalogue generated check |passed |
| compiler unit/integration |57 passed,1 existing ignored |
| real daily gate STEP / eligibility / off-day return |3 tests passed;9 PostgreSQL scene markers |
| canonical conversion and real donation dispatch |2 PostgreSQL tests passed |
| physical native Flower wave/replay |1 PostgreSQL test passed;25 live Dryads and no replay additions |
| seeded map-wire compatibility |1 passed;324 updates,5,431,963 bytes;legacy bytes verified |
| server Flower/Furyosa/zero-epoch |3 passed;2 PostgreSQL completion markers |
| server daily boot, including SP02 PostgreSQL epoch/restart |11 passed |
| server integration_rest content_ |123 passed |
| repository world projection |5 passed |
| final durability unit rerun |679 passed |
| cargo fmt --check |passed |
| final all-target Clippy -D warnings |passed;6m59s including a disk-space archival pause |
| compiler pin-refresh and independent pin-check |passed;artifact32,839,269 bytes |

Final commands: source `/workspace/oteryn-env/env.sh`, then Cargo `--locked -j1`.
The focused dispatch filters were `daily_world_gates`, `cyclopedia_postgres_`,
`world_src_postgres_flower`, and `seeded_walks`. Server filters were `world_src_`,
`daily_world_changes`, `--test integration_rest content_`, and
`--test content_world_project_repository`. The final durability command was
`cargo test --locked -j1 -p oteryn-gameplay-durability --lib --quiet` without the
PostgreSQL route; the separate actual database lanes above retain their route.

All-target Clippy command selects dispatch, durability, encounter-rules, entities,
npc, world-change, world, world-bundle, world-bundle-compiler and game-server,
with `--features oteryn-gameplay-entities/test-support -- -D warnings`.
The unchanged NPC crate is included because it was exercised by the retired
experiment; it contributes no delivered code changes. Unchanged vendored sqlx
emits a dependency deprecation warning; no vendor code is edited.

Rebase regression found and fixed during this run: the donation fixture had a
duplicated epoch2 insertion from combining the received scenario with main's
existing consecutive-day coverage. The first final run failed its unique key;
the redundant setup was removed, leaving main's carryover, active-day donation,
receipt/bank debit, epoch2/3 and respawn-factor assertions intact. Both final map
cases then passed (`cyclopedia-pg.log`,93.74s). No production workaround was added.

## Actual current PostgreSQL completion markers

```text

W05B_DAILY_GATE_POSTGRES_CASE_COMPLETED action=None unique=Some(64001) active=true eligible=false destination=MovementLocalPosition { x: 33497, y: 32616, floor: 8 }
W05B_DAILY_GATE_POSTGRES_CASE_COMPLETED action=None unique=Some(64001) active=false eligible=false destination=MovementLocalPosition { x: 101, y: 100, floor: 7 }
W05B_DAILY_GATE_POSTGRES_CASE_COMPLETED action=None unique=Some(35020) active=true eligible=false destination=MovementLocalPosition { x: 33215, y: 32273, floor: 7 }
W05B_DAILY_GATE_POSTGRES_CASE_COMPLETED action=Some(64103) unique=None active=true eligible=false destination=MovementLocalPosition { x: 33475, y: 32641, floor: 10 }
W05B_DAILY_GATE_POSTGRES_CASE_COMPLETED action=Some(9715) unique=None active=true eligible=false destination=MovementLocalPosition { x: 32265, y: 32164, floor: 7 }
W05B_DAILY_GATE_POSTGRES_CASE_COMPLETED action=Some(9710) unique=None active=true eligible=false destination=MovementLocalPosition { x: 100, y: 100, floor: 7 }
W05B_DAILY_GATE_POSTGRES_CASE_COMPLETED action=Some(9710) unique=None active=true eligible=true destination=MovementLocalPosition { x: 33290, y: 31786, floor: 13 }
W05B_DAILY_GATE_POSTGRES_CASE_COMPLETED action=Some(9715) unique=None active=false eligible=false destination=MovementLocalPosition { x: 32265, y: 32164, floor: 7 }
W05B_DAILY_GATE_POSTGRES_CASE_COMPLETED action=Some(9710) unique=None active=false eligible=true destination=MovementLocalPosition { x: 101, y: 100, floor: 7 }
POSTGRES_CASE_COMPLETED WORLD_SRC W12 canonical_conversion
POSTGRES_CASE_COMPLETED WORLD_SRC W12 donations_and_dispatch
WORLD_SRC POSTGRES_CASE_COMPLETED Flower physical native25 replay25
RAIDS_GLOBAL POSTGRES_CASE_COMPLETED world_src_flower_physical
POSTGRES_CASE_COMPLETED WORLD-SRC flower-cycle repeat-and-replay
POSTGRES_CASE_COMPLETED WORLD-SRC furyosa source-layer admission
SP02 POSTGRES_CASE_EXECUTED sp02_sv05 epoch=41 restart=41
SP02 POSTGRES_CASE_EXECUTED sp02_sv06 epoch=41 restart=41
SP02 POSTGRES_CASE_EXECUTED sp02_sv07 epoch=41 restart=41
```

## Generated artifacts and guards

World input digest: `829767b2e188001fbd9af845fa8702d66e94fb09a0854fe715ea2d6aca921d3c`.
World bundle: `6157033be3cbe48fb8eb1261ce13579cb1e02ec40f457025d1b89ac3d2d62671`.
Map-wire recording: `c114d516f4bd770c7d270c29447490a1368e98c921a6a2c4b6937b977cfb543a`.
Pins were regenerated using the repository compiler `pin-refresh .`; independent
`pin-check . /tmp/world-src-cont/pin-check-final` rebuilt the same artifact.
The received recording came from `tools/contracts/refresh_world_map_golden.py`;
this continuation verified it, including the legacy-wire comparison, without a manual edit.

No prepared cache was removed. To stay within the workspace size, only completed
task-created build artifacts were archived after timestamp/process guards and
verified backup checks; some earlier task backups were retired to avoid RAM pressure.
No tests used alternate profiles or disabled incremental compilation.
Final delivery checks verify the exact base, protected/client/forbidden paths,
0963-only migration, clean diff syntax and complete patch applicability through a
separate temporary Git index; gzip/base64 round-trip equality is also checked.
