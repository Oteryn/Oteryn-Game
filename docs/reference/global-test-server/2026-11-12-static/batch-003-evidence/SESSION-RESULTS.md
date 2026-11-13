# UI collection continuation (2026-11-13)

## Scope

This batch archives **89 original full-screen PNG captures** and **one exact raw text copy** from the official Tibia Global Winter Update Test Server UI. EVIDENCE-MANIFEST.json supplies a SHA-256, byte count and approximate capture time for each original; the archive contains all original bytes. No effective gameplay behavior is inferred from UI-only observations.

## Previously observed through the game UI (collector notes, not exhaustive validation)

- Potion list continuation: **strong mana potion**, **superior mana potion**, **supreme health potion**, **transcendence potion**, **ultimate health potion**, **ultimate mana potion**, with the list ending at **wealth duplex**; earlier collection also included **strong health potion** as an anchor. The tail consists of eight cards according to the collector; a complete identity-count audit was not proven after the disconnect.
- Three separate **empty potion flask** catalogue entries have the same visible display name; these must not be merged without an identity check.
- **rat claws**: Basic showed fist fighting weapon / dual-wielding, level 150, classification 3. Its proficiency tooltip described **5.00% of the highest combat skill as additional autoattack damage**, corresponding to an expected coefficient of **0.05** for the compared profile. This is an observed tooltip, not runtime damage-test evidence.
- One selected UI Basic text was copied verbatim into 1794590594140573210-basic.txt. It includes nonstandard delimiters and the exact compass description, **30.00 oz**, **tradeable in market: yes**, **body position: extra slot**. Do not silently normalize engraved symbols.
- Some NPC vendor sublists have independent scrolling. Their full coverage **has not been demonstrated**. Descriptions, identity mapping, numeric values and gameplay impact not independently verified should remain **UNKNOWN/PENDING**.

## Integrity and next steps

The evidence is in the archive and is indexed by its original filenames in the manifest. The image timestamps are *approximate file modification times*, not independently authenticated event timestamps. Capture ordering may span disconnections. Verify capture-to-item association before asserting a difference or publishing new MATCH statuses. Correlate every proposed Oteryn discrepancy with the exact source file and repository data; recheck vendor lists and the 8-item potion tail.

Only research evidence was produced in this batch. No Oteryn runtime, items, protocol or database changes are claimed.
