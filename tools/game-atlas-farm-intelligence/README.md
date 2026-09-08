# Game → Atlas farm intelligence v1

This directory contains the deterministic producer/validator for
`oteryn-game-atlas-farm-intelligence-v1`. It accepts only a normalized,
public-safe Game read model. It does not parse Lua/XML/OTBM, execute scripts,
scrape the web, inspect a running server, or calculate farm time/KPH.

```bash
python3 tools/game-atlas-farm-intelligence/export.py export source.json product.json
python3 tools/game-atlas-farm-intelligence/export.py verify product.json
python3 -m unittest discover -s tools/game-atlas-farm-intelligence -p 'test_*.py'
```

No production snapshot is committed in v1. The admitted creature-gameplay
contract proves static integer chance and count bounds, but the authoritative
corpus/product is not present in this checkout and its source does not prove
per-kill quantity distribution, live modifier context, tasks, weekly credit,
placement capacity, or live respawn semantics. Those families therefore stay
explicitly partial/unsupported until a revisioned normalized input is supplied.

