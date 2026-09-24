# Atlas farm-intelligence #75 implementation plan

## Decision

Publish a fail-closed availability product rather than accept an unauthenticated
normalized read model or duplicate existing Lua/XML/static-placement importers.
This decision is required now because Atlas
cannot safely distinguish proven loot facts from unavailable PMF, task, supply,
or respawn semantics. Later source evidence can populate the same classifications
without making v1 invent authority.

## Source census before capability claims

1. **PROVEN:** stable creature IDs and gameplay `chance_ppm` plus count bounds in
   the accepted creature-gameplay contract.
2. **PROVEN/UNKNOWN:** resolved item IDs are stable; unresolved labels remain
   unresolved and cannot be name-hashed.
3. **UNKNOWN:** exact live probability context and quantity distribution/process.
4. **UNKNOWN:** complete placement capacity/group/activation semantics, task and
   weekly/grouped-credit authority, and live respawn cadence.
5. **UNKNOWN:** production resource ceilings because there is no exact admitted
   corpus to census. **DERIVED, TEST ONLY:** small synthetic-fixture limits.

## Execution

1. Record the qualification matrix in the contract and task packet.
2. Add RED tests proving caller provenance/capability cannot create a production
   product, plus synthetic-only coverage for canonical bytes, identities,
   probability, all quantity classifications, zero-yield PMF preservation,
   relation failures, malformed/oversized products, and forbidden dynamic sources.
3. Add the minimal no-input blocked producer/exact validator and make those tests GREEN.
4. Run it twice to emit the canonical blocked product and compare bytes/digest;
   document that no authoritative current corpus is available.
5. Compile/test, run applicable repository policy/governance checks, inspect the
   whole diff, commit, and open one draft PR without merge/queue actions.

## Deliberately excluded

No runtime/content, existing producer, workflow, Cargo, migration, registry,
Atlas/Platform/META/Reference, production/live source, scraping, farm-time/KPH,
or spatial clustering changes. No fixed/PMF promotion from min/max bounds.
