# Maris Mistrock paid-travel execution

The existing production paid-travel PostgreSQL scenario passed before expansion in 119.82 seconds. Adding Maris and both admitted routes first failed in 96.60 seconds with Carrier(CapacityExceeded): the fixture reserved a literal 17 actors. Its capacity now derives exactly from one player, three original NPCs, and S10T_CAPTAINS.len(). All existing assertions remain intact.

The expanded real PostgreSQL scenario passed in 111.20 seconds, one test, zero failures. It emitted:

```
POSTGRES_EXECUTED S10T maris_mistrock fenrock fare=50 authored=(32563, 31313, 7) arrival=(32563, 31313, 7) replay charged once
POSTGRES_EXECUTED S10T maris_mistrock yalahar fare=100 authored=(32649, 31292, 6) arrival=(32649, 31292, 6) replay charged once
```

Each added passage exercises NPC_TALK quoting, an injected arrival transaction failure and rollback, reconciliation, successful placement and durable receipt replay without double charging. The existing Premium-entitled scenario is retained. Fares and coordinates are the already admitted Global-selected values; no production travel data or guard was changed.

The test-only change compiled and passed its real PG case. Focused all-target/all-feature cargo check and strict Clippy are still running; final formatting verification remains pending. Mission and newlywed/access-held routes are not qualified by this case. No main or qmech merge and no node boot were performed.
