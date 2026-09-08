# Atlas farm-intelligence #75 implementation plan

## Decision

Publish a bounded normalized read-model adapter rather than duplicate existing
Lua/XML/static-placement importers. This decision is required now because Atlas
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
5. **DERIVED:** deterministic canonicalization, integrity digest, and conservative
   normalized-product safety limits.

## Execution

1. Record the qualification matrix in the contract and task packet.
2. Add RED tests for canonical bytes, identities, probability, all quantity
   classifications, zero-yield PMF preservation, relation/provenance failures,
   capability states, malformed/oversized inputs, and forbidden dynamic sources.
3. Add the minimal producer/validator and make those tests GREEN.
4. Run it twice over a normalized test source and compare bytes/digest; validate
   the output and document that no authoritative current corpus is available.
5. Compile/test, run applicable repository policy/governance checks, inspect the
   whole diff, commit, and open one draft PR without merge/queue actions.

## Deliberately excluded

No runtime/content, existing producer, workflow, Cargo, migration, registry,
Atlas/Platform/META/Reference, production/live source, scraping, farm-time/KPH,
or spatial clustering changes. No fixed/PMF promotion from min/max bounds.

