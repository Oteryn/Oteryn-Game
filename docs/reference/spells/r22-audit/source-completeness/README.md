# Exhaustive Canary / Crystal source census, r22

This census reads complete immutable Git trees, not the incomplete local checkout directories. It records every tracked Lua file and every `Spell(...)` constructor file, preserves registration path/index and all duplicate variants, and maps registrations to the existing r21 catalogue by case-insensitive name plus carrier. That mapping alone is not proof of parameter-by-parameter import, external wiki verification or playable runtime execution.

| Snapshot | Exact revision | Tracked Lua | Enabled registration rows, all packs | Distinct enabled logical keys | Default-config registration rows | Captured monster attack/defense slots |
|---|---|---:|---:|---:|---:|---:|
| Approved Canary 15.30 branch | `99902524e052f37574194466c2949c576e4ab269` | 5378 | 848 | 846 | 846 | 6787 |
| Approved Crystal | `ff7ede593c69d4c658b382c97443e8155926924a` | 8046 | 1503 | 899 | 898 | 13675 |
| Observed Canary main | `04b83b512114bfd888000d6e1433ed8ecaec7c5b` | 5381 | 835 | 833 | 833 | 6787 |
| Observed Crystal summer-update | `00ce02a57ca5a12e48f32a3476e37471167e4c3f` | 8125 | 1504 | 900 | 899 | 13777 |

The observed heads were read on 2026-10-02 using ordinary GitHub Git HTTPS (`git ls-remote`), not Remote Desktop. Their exact commits already existed in the local object databases. They are separate diagnostic snapshots; the approved r21 source pins were not modified or silently superseded. Canary main has fewer player-spell declarations than the approved 15.30 branch.

The approved snapshots contain 2351 enabled registration rows across alternate packs, representing 904 distinct `(carrier, case-insensitive name)` keys. These counts preserve source variants; they do not mean 904 independent production spells or 904 working implementations. The approved player-only core-root union has 252 keys, matching the existing 246 active definitions plus 6 explicit removals by name/carrier. Five additional core-root declarations are monster-only: Candy Horror Wave, Nibblemaw Wave, Heal Malice, Heal Monster and Heal Monster 9x9. They have no player-catalogue entry. Overall 652 enabled logical keys have no player-catalogue/removal entry; most are monster or encounter scripts. This does not establish whether they are present in the older source-only dependency carrier.

Every original constructor-sandbox failure remains accountable. Exact upstream Vocation constants, compatibility method aliases and the one exact Gaz'haragoth helper dependency recover registration extraction in 12 Canary and 31 Crystal files. The recovered results prove registration data extraction only. Canary still has two opaque declared-registration files: `spell-megalomania_blue.lua` requires a real Zone and `exploding_cask.lua` creates a real timed item at top level. Crystal has the same opaque Megalomania Blue declaration. Their names, carriers, paths, source SHA256 and original errors are retained explicitly. Each snapshot also has two unregistered constructor helpers (`compat.lua` and `register_monster_type.lua`); these are not silently counted as missing spells.

Monster references use attack arrays and mixed defense tables correctly: `canary_batch.lua_value` represents the latter's numbered entries as `_list`. Source entries, order, chance, interval and conditions remain recorded. The source audit can retain data registered before a later Lua error, but real Zone/config/position/storage callbacks and dynamic encounter-generated variants remain outside execution qualification. `*-monster-load-findings.json` retains all such findings. Registered/built-in source resolution does not imply gameplay composition.

Alternate datapacks are explicit: default Canary config uses `data` plus `data-otservbr-global`; default Crystal uses `data` plus `data-global`. `data-canary` and `data-crystal` copies remain separately inventoried. Disabled `#example.lua` registrations are excluded from enabled counts. Duplicate source names and duplicate engine keys are retained. Instant registration identity is words, rune identity is runeId; a filesystem-loader winner is never fabricated. The audited default configurations have no duplicate engine-key conflict among extracted registrations.

Crystal summer-update differs from the approved pin in 101 Lua files, including the added Phosphorus Curse monster script and 17 changed core spell files. Canary main differs in 372 Lua files. Exact per-file before/after blobs and SHA256 are in `*-delta-from-approved-pin.json`.

## Reproduction

Use a Python environment with the existing `lupa.luajit21` dependency. The tools require complete Git object databases, and the Game repository supplies the existing source sandboxes. No server engine is executed.

```sh
python audit_source_completeness.py --repository /path/to/Oteryn-Game --source-root /path/to/spell-sources --out /path/to/audit \
  --snapshot canary=canary@99902524e052f37574194466c2949c576e4ab269 \
  --snapshot crystal=crystal@ff7ede593c69d4c658b382c97443e8155926924a \
  --snapshot canary-main-current=canary@04b83b512114bfd888000d6e1433ed8ecaec7c5b \
  --snapshot crystal-summer-current=crystal@00ce02a57ca5a12e48f32a3476e37471167e4c3f
python classify_source_inventory.py --repository /path/to/Oteryn-Game --audit /path/to/audit
python verify_source_audit.py --source-root /path/to/spell-sources --audit /path/to/audit
```

`inventory-verification.json` records 16 passing independent coverage/source-byte/slot checks; all three tools pass Python compilation. Original upstream raw files live only in the research-only `upstream-local-only` directory. They are deliberately excluded from the publication manifest. JSON evidence records extracted declarations/parameters and source fingerprints; it does not distribute original Lua assets. No existing r21 candidate file or publication worktree was changed by this audit.
