# FIRST_PRODUCTION_CONTENT_PROFILE/v1 — Amendment 04: one spawn of two placement cells

- Date: 2026-10-04
- Tracking: SPAWN-1A-PACKET-1, #1745 P1 4177087017 and P1 4177087019.
- Applies to: `OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md` plus Amendments 01-03.
- Normative status: **part of the same `FIRST_PRODUCTION_CONTENT_PROFILE/v1` decision**, accepted when
  #1745 merges. Implemented by SPAWN-1a (`OTERYN_GAME_SPAWN1A_FIXTURE_SPAWN_ROOM_R2_DECISION_2026-10-04.md` §2.1).
  Until SPAWN-1a merges, Amendments 01-03 describe the running code.
- Authority: unchanged; production/live activation authority remains NONE.

## 1. Why

Amendment 01 set the spawn population to exactly 1 and deferred a larger value to "the first accepted
requirement for >1 concurrent content-spawned actor". That requirement is now accepted: D116 is one spawn
of 2 rats at two declared placement cells (CREATURE-AI-0 §3 and §6.1), and native entry-room revision 2
realizes it (SPAWN-1A-PACKET-1 §1.1). This amendment is the sizing decision Amendment 01 asked for. It
admits exactly that and nothing broader: still one spawn, still one scope.

## 2. Rules

- Spawn count stays exactly 1 (`DUR04-FIRST-PROD-SPAWNS`).
- `population_limit` is 1 or 2. 0 and 3 are refused before artifact lowering or any runtime actor
  allocation. The "must equal 1" refusal becomes "must be 1 or 2".
- Aggregate content-spawned population per authoritative scope is at most 2 (one spawn × 2), with
  checked arithmetic as before.
- `FirstProductionSpawn.cell_key` becomes `cell_keys`: an ordered, duplicate-free list of cell keys whose
  length equals `population_limit`. Each entry must name a declared cell. Placement `i` realizes
  actor `i`.

## 3. Artifact representation

A record has at most eight fields (base decision §4), and the spawn record already uses eight, so the
list does not go into the spawn record:

- `RECORD_SPAWN` (9) drops its cell field and keeps seven fields: key, creature, behavior,
  population_limit, recovery, multiplicity, eligibility_scope.
- New `RECORD_SPAWN_CELL` (19) has three fields: spawn key, ordinal (decimal `u16`, 0-based) and cell key.
  A spawn has exactly `population_limit` such records, with ordinals 0..n-1 in canonical order, directly
  after its spawn record. A missing, extra, duplicate, out-of-order or dangling record refuses, and so
  does a duplicate cell.
- Kind 19 is the first unused server record kind. Kinds 1-18 are taken, and 18 is
  `RECORD_RNG_PURPOSE` (#1745 P1 4177134666). Client kinds start at 108. SPAWN-1a adds a test in
  `production.rs` that every record kind constant, server and client, is distinct.
- The carrier magic and `PROFILE_VERSION` stay. The only producer is the in-repository compiler and no
  artifact is deployed (production activation is NONE), so the decoder changes in place, as Amendment 02
  did. Every committed digest and golden is regenerated with the repository tooling.

## 4. Mechanical maxima

The derivation of base §4 and Amendments 02-03 is unchanged except for at most two spawn-cell records:

| Registry id | Was | Becomes | Derivation |
|---|---:|---:|---|
| `DUR04-FIRST-PROD-SPAWN-POPULATION-PER-SPAWN` | 1 | 2 | §2 |
| `DUR04-FIRST-PROD-SPAWN-POPULATION-PER-SCOPE` | 1 | 2 | 1 spawn × 2 |
| `DUR04-FIRST-PROD-SERVER-RECORDS` | 1,043 | 1,045 | + 2 spawn-cell records |
| `DUR04-FIRST-PROD-REFERENCES` | 3,087 | 3,090 | spawn 3 → spawn 2 + 2 × (spawn, cell) |
| `DUR04-FIRST-PROD-SECTION-BYTES` | 4,295,078 | 4,303,314 | + 2 × (4 + 4,114) |
| `DUR04-FIRST-PROD-SERVER-ARTIFACT-BYTES` | 4,304,614 | 4,312,850 | + 8,236 |
| `DUR04-FIRST-PROD-GENERATION-PAIR-BYTES` | 4,338,862 | 4,347,098 | + 8,236 |
| `DUR04-FIRST-PROD-DECODED-FIELDS` | 8,432 | 8,448 | + 2 × 8 (worst-case record) |

Definitions (1,042), content keys, client records and every other row are unchanged: a spawn-cell record
is part of its spawn definition, not a definition. If the implementation shows a different exact
value, SPAWN-1a returns BLOCKER rather than choosing one.

Each changed row keeps its id. Its `owner_contract` gains `+AMENDMENT_04`, its hard maximum and
configurable maximum take the value above, and its boundary tests name max accepted and max+1 refused.
SPAWN-1a adds those tests to `production.rs` and `tests/content_first_production.rs`. It also updates
`test_governance_lifecycle_first_production_content_registry.py`, so that the final values are the ones
above and the superseded values appear nowhere in the registry.

## 5. Still out

Two or more spawns, a population above 2, encounter packs, density scaling and broad-world spawn sizing
remain `DEFERRED_REQUIRES_FUTURE_DECISION` (Amendment 01 §5).
