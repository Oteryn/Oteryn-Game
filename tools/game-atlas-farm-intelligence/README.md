# Game → Atlas farm intelligence v1

This directory contains the fail-closed producer/validator for
`oteryn-game-atlas-farm-intelligence-v1`. The authoritative normalized Game
publication needed to populate farm facts is not available in this checkout,
so production has **no caller-authored JSON input** and emits only a canonical
`BLOCKED_NO_ADMITTED_SOURCE` product:

```bash
python3 tools/game-atlas-farm-intelligence/export.py export product.json
python3 tools/game-atlas-farm-intelligence/export.py verify product.json
python3 -m unittest discover -s tools/game-atlas-farm-intelligence -p 'test_*.py'
```

The module's richer semantic fixtures use a different test-only contract ID,
fixed non-authority marker, and explicitly test-only limits. They cannot pass
the production validator or authorize `COMPLETE`, exact context, `FIXED`, or
`EXACT_PMF` production facts.

No numeric public/production hard limits are frozen in v1 because the exact
admitted corpus has not been censused. The producer does not parse Lua/XML/OTBM,
execute scripts, scrape the web, inspect a running server, or calculate Atlas
farm time/KPH.
