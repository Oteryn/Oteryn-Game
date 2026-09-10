# OTV2 Reference world corridor census — Phase A evidence

Status: **MIGRATION_EVIDENCE / OTS_HYPOTHESIS_ONLY**
Result: **PHASE_A_RESULT=PASS**
Target parity: **phase_b_target_parity=NOT_PERFORMED**
Production authority: **production_authority=NONE**
Registry maxima selected: **false**

## Authority and source identity

This evidence continues only `REFERENCE_WORLD_CORRIDOR_CENSUS_511` on PR #525.
It uses `oteryn-game-atlas-fullworld-source-v0` at producer code commit
`d7c207876920dfe7c7026a0d1316a2ee603ebf0d`; the new consumer filters and
aggregates `load_runtime`, `iter_records`, the record classifiers,
`native_floor`, and `project_tile_bytes`. It contains no OTBM parser or
alternative semantic projection.

The execution machine verified, before accepting counts:

- `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce` (read-only);
- `world.otbm` = `3bd40d14fefec41f24c4b3ae879e420be1a831ef55b95dcbec721e587a09b034`;
- Drive file `1Dlo3bS4K1nS3mw4BhPZdlHT7lX5zRAvv`, ZIP = `1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f`;
- catalog = `35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85`;
- appearance DAT = `dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075`.

`load_runtime` independently revalidated the same inputs. The map was streamed
once per census execution; only records inside either bounded shard were
retained and projected. No source, ZIP, sprite, pixel, or map bytes are
committed.

## Deterministic bounded results

The canonical machine-readable result is
`tools/reference-world-corridor-census/phase-a-summary.json`; its SHA-256 is
`605dcb2c913bf2fd969957652251a99971cb5a43c99dc5ffd55b61374bc87979`. It preserves the exact sorted appearance and sprite identifiers,
landmarks, structural observations, bounds, reasons, and classifications.

| Dimension | Newhaven | Targuna |
|---|---:|---:|
| start/final shard | `f-7-r1016-c1016` | `f-7-r997-c997` |
| fullworld source region | `fm000007_rxp000127_ryp000127` | `fm000007_rxp000124_ryp000124` |
| half-open bounds | `[32512,32544) × [32512,32544)` | `[31904,31936) × [31904,31936)` |
| floors | `[-7]` | `[-7]` |
| cell capacity | 1,024 | 1,024 |
| tile records | 1,024 | 1,024 |
| non-empty tile records | 1,024 | 1,024 |
| ordered presentations | 1,199 | 1,264 |
| max presentations/cell | 5 | 5 |
| resolved presentations | 1,199 | 1,264 |
| unresolved presentations / IDs | 0 / `[]` | 0 / `[]` |
| unique appearance source IDs | 70 | 156 |
| unique resolved sprite IDs | 84 | 190 |
| encoded semantic bytes | 899,694 | 939,759 |
| unique resolved presentation IDs | 1,199 | 1,264 |
| candidate composite diagnostics | 3 | 54 |

Newhaven contains source town `33/Newhaven` at `(32536,32514,-7)`, waypoint
`newhaven` at `(32534,32513,-7)`, and one structurally detected
`TELEPORT_DESTINATION` occurrence on source item `37001` at
`(32538,32514,-7)`. Targuna contains towns `1/Dawnport Tutorial` and
`34/Targuna`, both at `(31934,31925,-7)`, and one structurally detected
`TELEPORT_DESTINATION` occurrence on source item `5756` at
`(31924,31904,-7)`. Both encoded destinations are the source value `(0,0,0)`;
this report does not invent gameplay meaning for them.

Both shards have tile records on north/east/south/west boundaries. Edge
occupancy alone cannot prove which adjacent shard a selected structure, path,
or service fixture requires. Each boundary is therefore recorded as
`AMBIGUOUS_EXPANSION`; expansion order is `[]`, final equals starting shard,
and the two locations are never joined into one rectangle.

## Validation record

RED was recorded before `census.py` existed:

```text
python tools/reference-world-corridor-census/self_test.py
ModuleNotFoundError: No module named 'census'
RED_STATUS=1
```

GREEN and integration commands:

```text
python tools/reference-world-corridor-census/self_test.py
reference-world-corridor-census self-test: PASS

python -m py_compile tools/reference-world-corridor-census/census.py tools/reference-world-corridor-census/self_test.py
PASS

python -m py_compile tools/game-atlas-fullworld-source/producer.py tools/game-atlas-fullworld-source/self_test.py
PASS

python tools/game-atlas-fullworld-source/self_test.py
game-atlas-fullworld-source self-test: PASS

python tools/reference-world-corridor-census/census.py --legacy-root /tmp/oteryn-legacy --map /tmp/oteryn-legacy/vendor/map-analysis/crystalserver/data-global/world/world.otbm --asset-zip /tmp/15.32.zip --assets /tmp/15.32/assets --output /tmp/census-a.json
python tools/reference-world-corridor-census/census.py --legacy-root /tmp/oteryn-legacy --map /tmp/oteryn-legacy/vendor/map-analysis/crystalserver/data-global/world/world.otbm --asset-zip /tmp/15.32.zip --assets /tmp/15.32/assets --output /tmp/census-b.json
cmp /tmp/census-a.json /tmp/census-b.json
PASS; two independent full source streams produced byte-identical summaries
```

Focused tests cover half-open boundary exclusion, floor exclusion, stable
ordering, explicit unresolved propagation, presentation count preservation,
structural-record ordering, deterministic ambiguity reporting, digest mismatch,
and checked unsigned-64-bit counter/byte overflow rejection. The corpus has no
unresolved presentations in either selected shard; the synthetic negative
proves unresolved IDs remain explicit rather than being dropped. No local hard
maximum is introduced, so a max/max+1 fixed-bound test is not applicable.

## Interpretation and handoff

The counts are separate lower-bound resource dimensions for #504, not production
ceilings. Source coordinates, source identities, topology, town names, and
transition-like records remain **MIGRATION_EVIDENCE / OTS_HYPOTHESIS_ONLY**.
There is no claim of Global target geometry or parity, no registry selection,
and no Phase-B work.
