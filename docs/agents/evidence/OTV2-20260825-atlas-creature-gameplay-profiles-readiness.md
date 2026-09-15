# OTV2-20260825 Atlas Creature Gameplay Profiles readiness evidence

## Authority

- Capability: `creature-gameplay-profiles-v1`, schema `1`.
- Producer: `Oteryn/Oteryn-Game`.
- Pinned migration evidence: `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce`.
- Programme: `ATLAS-CREATURE-GAMEPLAY-PROFILES`; Game lifecycle issue `#136`; Atlas consumer issue `Oteryn/Oteryn-Atlas#165`.
- This evidence is census/readiness material. The exact final PR-head and merged-main product digest are intentionally recorded by immutable CI/issue evidence, because the product digest binds `producer_repository_sha` and cannot truthfully be self-recorded inside the commit it identifies.

## Exact real-evidence census

The producer was executed against the pinned `data-global/npc` and `data-global/monster` corpus, never by executing Lua.

| Metric | Result |
| --- | ---: |
| NPC profiles | 1,049 |
| monster profiles | 1,800 |
| resolved stable referenced items | 0 |
| NPC shop COMPLETE / PARTIAL / UNKNOWN / AMBIGUOUS | 254 / 51 / 732 / 12 |
| NPC travel COMPLETE / PARTIAL / UNKNOWN / AMBIGUOUS | 5 / 43 / 989 / 12 |
| NPC services PARTIAL / UNKNOWN / AMBIGUOUS | 360 / 677 / 12 |
| monster loot COMPLETE / PARTIAL / UNKNOWN / UNRESOLVED | 1,680 / 66 / 53 / 1 |
| monster stats COMPLETE / PARTIAL | 968 / 832 |
| monster resistances COMPLETE / PARTIAL | 1,788 / 12 |
| NPC sells rows / buys rows | 5,744 / 6,056 |
| travel destination rows | 7 |
| loot rows | 17,086 |

`resolved stable referenced items = 0` is intentional: the current Game tree has no revisioned native item mapping for this legacy corpus. Numeric/client IDs and display names therefore remain non-canonical evidence and never become fake `item_ref` values.

## Measured size and cardinality

| Metric | p95 | maximum |
| --- | ---: | ---: |
| canonical NPC profile bytes | 9,207 | 162,567 |
| canonical monster profile bytes | 4,470 | 8,837 |
| NPC sells rows/profile | 30 | 180 |
| NPC buys rows/profile | 27 | 1,356 |
| monster loot rows/profile | 29 | 62 |
| product shard bytes | 23,500 | 174,660 |
| records/shard | 10 | 15 |

Additional observed maxima: manifest `85,724` bytes; `508` profile shards; UTF-8 string `47` bytes; JSON nesting depth `7`; loot max count `475`; public shop price `10,000,000`; explicit resistance absolute percent `1,400`.

## Frozen hard bounds

The implementation and contract freeze `creature-gameplay-profiles-v1-e417-census-v1`: manifest 256 KiB, shard 512 KiB, 32 profiles/shard, 2,048 NPCs, 4,096 monsters, 4,096 referenced items, 513 shard descriptors, 256-byte strings, depth 12, sells 256, buys 2,048, total shop 2,304, loot 128, travel 16, resistance elements 16, immunities 16, price 100,000,000, loot count 1,024, absolute resistance percent 2,048.

The 4,096 referenced-item ceiling is derived from the 2,593 distinct explicit legacy/client item identifiers observed during census; these identifiers remain unresolved until a Game-owned stable mapping exists.

## Fail-closed corpus findings

- 32 legacy NPC name groups contain duplicates. Identical normalized profiles deduplicate; conflicting definitions emit `AMBIGUOUS` sections and no arbitrary winner.
- 104 loot declarations use source `chance > 100000` (observed maximum `13,600,000`). No scale is guessed; affected rows become `INVALID_LOOT_CHANCE` and their section becomes non-complete.
- `winter_update_2025/day_night_harpy.lua` contains `minCount = 100` without a provable matching maximum. No count is invented; the row becomes `INVALID_LOOT_COUNT`.
- Service labels are proof-only. Static bank/blessing/trainer/shop/travel evidence may be surfaced, but the taxonomy remains `PARTIAL`; `Storage.Quest.*` occurrence is not promoted to a quest-service fact.
- The test fixture contains an executable `error(...)` statement and still passes because source scripts are never executed.

## Determinism and corruption checks

The self-test requires byte-identical product directories for logically identical input ordering and verifies the emitted product. Negative cases reject malformed creature IDs, duplicate creature IDs, duplicate stable item references, negative prices, invalid `chance_ppm`, shop-row and string overflows, unsafe `../` shard paths, and shard byte/digest corruption.

Exact-head workflow `game-atlas-creature-gameplay-profiles.yml` additionally rebuilds the pinned real corpus twice, compares all emitted bytes, verifies the frozen census, and proves the checkout matches the PR head.