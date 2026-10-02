# R24 source-qualified monster melee provider

This incremental candidate preserves all 1508 R23 native Creature profiles,
identities, health, behavior and presentations. It also preserves every one of
1231 previously selected Canary melee closures byte-for-byte. The accepted
runtime already supports Crystal source receipts; no runtime code or authority
is added here.

An explicit Crystal fallback recovers 22 additional physical range melee
closures, for **1253 enabled and 255 disabled**. Primary Canary closures always
win. A fallback must match the existing health, exact attack ordinal, interval
and chance, and its captured Lua bytes must match the pinned per-file SHA256,
byte count and Git blob. No Crystal attack/skill formula is used: only its
explicit range formulas qualify. Secondary conditions, if present, remain
explicitly omitted under the existing supported execution flag.

The provider is still approximate: one physical melee slot, no armor/shield
mitigation, bounded damage distribution and player HP floor 1. It does not
implement ranged attacks, custom callbacks, defense, summons, chase, or public
spawn admission. Output data alone grants no activation or deployment authority.

## Every disabled profile has a disposition

`disabled-resolution-report.json` covers all 255 with both selected donor packs,
actual frozen source receipts, native/donor HP and source melee dependency facts:

- 124 source profiles have no melee slot. These may be ranged, special or passive;
  absence of melee alone does not prove the creature is passive.
- 11 have an explicit zero-damage primary melee formula. This is intentional
  source data, including Cobra's omitted poison condition; damage is not invented.
- 103 have an accepted native HP conflict against the primary donor. Existing
  profiles are preserved; correction requires resolution of accepted data.
- 17 have no unique compatible donor with accepted HP and schedule in either
  selected pack.

This is a complete data disposition, not a claim that 255 full monster engines
are complete. Source revisions are Canary `99902524e052f37574194466c2949c576e4ab269`
and Crystal summer-update `ff7ede593c69d4c658b382c97443e8155926924a`.
Read method: local pinned Git/HTTP source captures. No fresh wiki validation is
claimed in this lane; prior dated wiki comparisons remain in R22/R23.

## Reproduction

Run from the repository root, with the preserved R22 source capture available:

```sh
python tools/content-schema/native-gameplay/build_canonical_monster_profiles.py \
  --creatures docs/reference/spells/r23-candidate/creature-profiles.json \
  --presentations docs/reference/spells/r23-candidate/presentation-profiles.json \
  --spell-appearances docs/reference/spells/r23-candidate/spell-appearances.json \
  --canary-source /workspace/spell-sources/canary \
  --bundle-root /workspace/spells-r22-monster-import/converted-bundles \
  --source-root /workspace/spells-r22-monster-import/source-inputs \
  --crystal-fallback --out /tmp/oteryn-r24-monster-provider

python tools/content-schema/native-gameplay/audit_disabled_monster_profiles.py \
  --creatures /tmp/oteryn-r24-monster-provider/creature-profiles.json \
  --report /tmp/oteryn-r24-monster-provider/monster-melee-derivation.json \
  --bundle-root /workspace/spells-r22-monster-import/converted-bundles \
  --source-root /workspace/spells-r22-monster-import/source-inputs \
  --out /tmp/oteryn-r24-monster-provider/disabled-resolution-report.json

python -m unittest discover -s tools/content-schema/native-gameplay \
  -p 'test_*monster*profiles.py'
```

The output directory must not exist; preserved candidates are never overwritten.
The union proof is incremental (`preserved_active_count=1508`, `appended_count=0`);
R23 remains the evidence for the earlier canonical append. Full manifest
admission and physical evidence for changed pins are separate coordinator checks.
