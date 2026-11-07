# SRV7 verification record — 2026-11-06

This is an in-progress evidence record. A passing leaf does not establish completion
of its row. Final row dispositions and frozen-candidate checks are pending.

## Candidate and baseline

Base: `b2cfa798e3def61002d39a6da8b9de5df5aa2fd8`.
Delivery branch: `claude/happy-davinci-ijz3wy-srv7`; no PR requested.
The read-only main check at `ed788c4bb` retained server PARTIAL for S06, W18,
W05d, W16 and W23. Client S06/W18/W16/W23 were already DONE; their existing
native paths are retained. W05d client remained PARTIAL. The status source is
`MECHANICS-STATUS-2026-11-05.{md,json}`; row reports give the implementation and
source references. Parallel branch comparisons are recorded in the row reports;
no independent casting or instance-corpse engine is implemented by SRV7.

## Client baseline evidence

| Row | Baseline disposition | Existing native decode / presentation / play evidence |
|---|---|---|
| S06 | ALREADY-DONE | `apps/client/src/features/0025-appearance/protocol.rs:1`, `view.rs:1`, `tests/play.rs:116` |
| W18 | ALREADY-DONE | `apps/client/src/features/0150-house-podium/protocol.rs:1`, `view.rs:8`, `tests/play.rs:1` |
| W16 | ALREADY-DONE | `apps/client/src/features/0006-world-spatial-entities/protocol.rs:1`, `view.rs:11`, `tests/play.rs:1` |
| W23 | ALREADY-DONE | `apps/client/src/features/0013-paced-movement/protocol.rs:18`, `tests/play.rs:1` |

W18 additional use/rotate/effect messages and W05d seasonal presentation require
their own new native proof; baseline completion does not qualify a new message.

## Executed focused evidence

All PostgreSQL times below are real runs against the environment PostgreSQL,
not skipped zero-duration cases. Full logs are retained under `/tmp` during
execution. The final check section will list reproducible filters.

| Focus | Result | Runtime |
|---|---|---:|
| Ordinary Root MINT and TRANSFER under seasonal migration | PASS | 5.609 s |
| Hairycles ungated trade baseline, before source gate | RED, committed without stage18 | 4.395 s |
| Current Cake food/far-House-source guards and indexed current-XID lookup | PASS | 4.711 s |
| Same focused SQL cases after native15 furniture SQL regeneration | PASS | 3.940 s |
| Source1066 final Cake bite and fixed-clock completion | PASS | 62.597 s |
| Four immediate Cake food bites, Full refusal and one-minute hunger | PASS | 60.969 s |
| Thirty furniture shop source greetings and offers | PASS | 38.655 s |
| Actual Golem machine extraction/removal/replay | PASS | 60.458 s |

The second focused semantic run selected fourteen tests: four passed and ten
failed in235.234s. The failures are tracked as gameplay defects or fixture
failures individually; that run is not a final passing validation claim.

Confirmed missing leaves included decoration speech, carried candle expiry,
Pumpkin calendar admission and static12816 physics. The acquisition run exposed
an ambiguous SQL equipment-input alias. House wire tests identified an initial
vitals publication gate on the unrelated hireling capability. Corrected fixtures
retain genuine creation, progression, generation, lease and custody authority.

## Final validation

Pending regeneration, affected native caller reruns, client play tests,
formatting, changed-crate Clippy/tests, content integration, architecture and
independent frozen-candidate review. No commit or push is claimed by this record.
