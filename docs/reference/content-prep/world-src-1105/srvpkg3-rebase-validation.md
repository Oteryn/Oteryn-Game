# WORLD-SRC rebase onto srvpkg3

Requested base: `faf8ae8700d935bca612261730aca5466526389a`, fetched from
`https://github.com/Oteryn/Oteryn-Game.git`, branch
`claude/happy-davinci-ijz3wy-srvpkg3`.

The previously delivered92-file diff against1fa04bbb was verified against its
SHA-256 `48ccf1595735f6ba6dab357f66946a512e72ff83759da229fddc7084f9b79cd8`,
then reapplied cleanly to the requested base on an uncommitted local task branch.
No commit, push, migration renumbering or client-handler change was made.
Migration0963 was absent on the requested base and remains the only new migration.
Existing capability57 and commands72/73 are reused; no new protocol identifier.

The historical main-based validation remains in `continuation-validation.md`.
This file records fresh validation of the srvpkg3-based delivery. Source decisions
and STOPPED boundaries are unchanged: no new seasonal or speed mechanic is claimed.

## Independent integration review

A read-only review found no concrete regression. The402-path protected union has
zero overlap with the retained change. Main client handlers and the base branch's
Dawnport resume guard and NPC voice binding remain unchanged. Daily gate callbacks
still run through the successful real STEP path; daily NPCs use the admitted
catalogue and ordinary runtime placement. There is no retired Ruprecht runtime,
SQL extension or parallel NPC voice/dispatch path in the production diff.

## Fresh validation

All checks use the prepared environment, normal repository profiles and
`--locked -j1`. No tool installation, altered RUSTFLAGS/linker, disabled
incremental setting, prepared-cache removal, commit or push was used.

| Check | Passed | Existing ignored |
| --- | ---: | ---: |
| Compiler unit/integration | 57 | 1 |
| Bundle writer/strict wire format | 10 | 0 |
| Daily world-change rules | 15 | 0 |
| World | 222 | 1 |
| Encounter rules | 61 | 0 |
| Entities with test-support | 251 | 0 |
| Durability unit | 688 | 0 |
| Daily gate STEP/eligibility/off-day | 3 | 0 |
| Canonical conversion and donation dispatch | 2 | 0 |
| Physical Flower replay | 1 | 0 |
| Map-wire compatibility | 1 | 0 |
| Server Flower/Furyosa/zero epoch | 3 | 0 |
| Daily boot and SP02 PostgreSQL restart | 11 | 0 |
| Content integration | 123 | 0 |
| Repository world projection | 5 | 0 |

Final all-target Clippy `-D warnings` and `cargo fmt --check`: passed.

The Clippy selection is dispatch, durability, encounter-rules, entities,
world-change, world, world-bundle, world-bundle-compiler and game-server, with
`--all-targets --features oteryn-gameplay-entities/test-support -- -D warnings`.
Unit lanes omit the PostgreSQL route; the following own database lanes retain
it and emit completion markers only after their assertions. Focused filters:
`daily_world_gates`, `cyclopedia_postgres_`, `world_src_postgres_flower`,
`seeded_walks`, server `world_src_`, `daily_world_changes`,
`--test integration_rest content_` and `--test content_world_project_repository`.

The first durability link failed when the workspace filled (linker signal7).
After verified archival/retirement of completed task-created build outputs,
the identical durability command passed all688 tests. That failed build is
retained separately in `/tmp/world-src-srvpkg3/durability-link-disk-failure.log`;
no assertion was removed, weakened or skipped. Build settings were unchanged.

Actual current PostgreSQL proof:

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

## Generated pin

The repository compiler's `pin-refresh .` and independent `pin-check` passed.
Compiler inputs: `6902a2957b53d71c01b7fcc18ec68816168d25611f7822a68bd00756ff7a67ce`.
World bundle: `6157033be3cbe48fb8eb1261ce13579cb1e02ec40f457025d1b89ac3d2d62671`.
The compiler-input identity changes with the base; the bundle bytes are unchanged.

## Plain diff transport

The delivery uses an ordinary unified `git diff` against the requested base,
UTF-8, split on complete line boundaries. Join the exact block contents with one
newline between parts and one final newline; the report gives the resulting byte
count and MD5. Parts are a frozen snapshot, independent of subsequent chat turns.
