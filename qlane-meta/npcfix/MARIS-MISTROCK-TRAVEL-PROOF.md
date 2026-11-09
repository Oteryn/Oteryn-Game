# Maris Mistrock paid-travel execution

The existing production paid-travel PostgreSQL scenario passed before expansion in 119.82 seconds. Adding Maris and both admitted routes first failed in 96.60 seconds with Carrier(CapacityExceeded): the fixture reserved a literal 17 actors. Its capacity now derives exactly from one player, three original NPCs, and S10T_CAPTAINS.len(). All existing assertions remain intact.

The expanded real PostgreSQL scenario passed in 111.20 seconds, one test, zero failures. It emitted:

```
POSTGRES_EXECUTED S10T maris_mistrock fenrock fare=50 authored=(32563, 31313, 7) arrival=(32563, 31313, 7) replay charged once
POSTGRES_EXECUTED S10T maris_mistrock yalahar fare=100 authored=(32649, 31292, 6) arrival=(32649, 31292, 6) replay charged once
```

Each added passage exercises NPC_TALK quoting, an injected arrival transaction failure and rollback, reconciliation, successful placement and durable receipt replay without double charging. The existing Premium-entitled scenario is retained. Fares and coordinates are the already admitted Global-selected values; no production travel data or guard was changed.

The test-only change passed its real PG case, focused dispatch all-target/all-feature cargo check, strict Clippy (-D warnings), cargo fmt --all --check, and git diff --check. Check and Clippy emitted only existing vendor deprecation warnings. Mission and newlywed/access-held routes are not qualified by this case. No main or qmech merge and no node boot were performed.

## Premium refusal verification

The production PostgreSQL scenario now verifies both Fenrock and Yalahar refuse an unentitled account at quote time and refuse confirmation when Premium expires after a valid quote. Refusal leaves gold, receipts and the player position unchanged; restored Premium then follows the existing successful rollback/reconciliation/replay path. This verifies existing guards; no production route data or guard changed and no failing production baseline is claimed.

The expanded scenario passed in 122.40 seconds, one test, zero failures or ignored cases. Each route emitted `POSTGRES_EXECUTED S10T maris_mistrock <destination> Premium refused at quote and confirmation; no charge, receipt or movement`, followed by its successful paid-travel marker. Final dispatch all-target/all-feature cargo check, strict Clippy (-D warnings), cargo fmt --all --check and git diff --check all passed. Check and Clippy emitted only existing vendor deprecation warnings. No main/qmech merge or node boot was performed.
