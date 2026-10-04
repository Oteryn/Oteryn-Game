# Imported Wheel and Gem reference data

The server embeds and reads this data-only catalogue during Content boot.
It contains all five vocations: 180 Wheel slots, 20 Revelation assignments,
46 Basic mods, 94 Supreme mods, vocation pools, grades, resonance, Atelier
fees/yields/guards, initial gems and the declared OTS drop policy.

- `wheel.json`: progression, topology and per-vocation Wheel/perk data.
- `gems.json`: per-vocation gem families/names/pools and complete Gem/Atelier data.
- `import-manifest.json`: exact source candidate hash and both file hashes.

Both files share the authoring revision and source hash. The importer validates
the closed authoring schema and source evidence before making a lossless split.
No candidate data is manually copied or tuned in these generated files.
The directory attributes keep JSON bytes at LF on Windows checkouts as well;
the exporter writes UTF-8/LF and checks bytes, not normalized text.

Rebuild/check from the repository root:

```sh
python tools/content-schema/wheel-authoring/import_server_data.py
python tools/content-schema/wheel-authoring/import_server_data.py --check
```

`POPULATED` in the directory indexes means data is imported. Every import envelope
still has `runtime_admitted:false`. Loading creates no Character records, grants
no Wheel stages and performs no reveal, gold/fragment transaction or combat effect.
The reader refuses a corrupt/mixed/active or incomplete catalogue before readiness.
Native admission, effects, persistence, Item use/drop/trade, wire and client UI
remain separate tasks. Guiding Presence arithmetic and reveal search scope retain
their documented uncertainty/differences. Supreme II→III selects the owner's
Global confirmation: 12M gold +15 Greater Fragments; the architect decision still
needs synchronization. Absolute drop RNG remains an OTS reference, not Global proof.

Schema/evidence and icon manifest remain in `tools/content-schema/wheel-authoring`.
Icon crop metadata accompanies perks, but images/client packaging are not imported.
Reference URLs/IDs are not newly admitted native Spell, effect or Item identities.
Before the first native ruleset admission, its owner must bind them and introduce
the runtime revision and compatibility/migration classification under W-R/GEM-R.
