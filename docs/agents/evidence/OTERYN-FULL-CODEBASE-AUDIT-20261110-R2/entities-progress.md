# Gameplay entities — partial source review

Input: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. Status: **IN PROGRESS**. Frozen coverage: **51/86 complete files, 19,958 deduplicated text lines**. The other 35 exact paths are listed in `coverage-ledger.json`; this package is not REVIEWED.

## Actor authority and lifecycle

The carrier binds a committed runtime assignment, World, scoped identity, ownership generation and node incarnation to a one-use continuity grant. Bootstrap validates capacity and preallocates slots before consuming continuity. Admission, reservation, commitment, removal and generation exhaustion are distinct transitions. Current references recheck scope, owner generation and local generation; retirement invalidates current owner stamps and continuity.

Player admission separates uncommitted reservations from Session-bound slots. Attacker authority comes from the exact Character lease and Session. Control-loss epochs, loss marks and write fences join or clear only matching transitions. Recovery and fresh-entry protection have different time boundaries and do not grant an arbitrary caller durable authority.

Activity transfer stages the source and destination under current World/scope, lease and distinct Session constraints. The source payload is moved after the sealed proof, with destination capacity reserved first; the inspected code does not justify an item duplication claim. Source-step and commandless reservations retain exact owner/event/position bindings, and an unknown outcome is not treated as a definite rollback.

House custody retains its original context and validates the named physical cycle, current actor/session and position snapshot. Movement uses current map/hireling occupancy and exact compare-and-commit. Inspected production callers add current admission and SQL item/custody fences. A speculative stale House-owner concern is not a finding without a meaningful reachable authority violation.

## Combat, spells and observations

The complete damage ledger includes command-origin replay, superseded-session and high-water rules, staged mutation, actual removed-HP attribution, exact current winning principal, retained lethal projection and per-principal reward occurrences. The accepted D4 residual retention limit was distinguished from new candidates. Corpse display refresh and original item-mint authority are separate; active settlement pins are retained.

Spell preparation, reservation and commit use current actor/session/scope and exact staged state. Creature condition processing stages a copied store and HP successor, then validates the actual slot again before publication. Reflection and native healing were reviewed with target identity, source context, bounded arithmetic, duplicate prevention and batch publication. Two unresolved integration candidates are retained privately for native validation; their severity and runtime effects are not counted as confirmed here.

Hazard and party projections bind current actors, Session, World and content. They are cached observations, not durable authorization. The earlier party-cache fallthrough hypothesis is refuted by validation of the same immutable actor and Session before the cache read. Position ownership distinguishes context, generation, translation revision and facing; current occupancy includes reserved/destination actors where required by each owner.

## Native companions and encounters

All 2,345 lines of the companion implementation and included tests were reviewed. Policy resolution uses an exact qualified key before a unique display-name fallback. Prepared creation binds the real free-head generation and current cell; reservation consumes capacity and generation before durable completion. Definite rollback burns that incarnation, and ambiguous completion is not inferred from object destruction. Master and Session, retained snapshots and full owner census are checked before mutation and bulk removal.

Native summons use exact parent/child indexes, bounded child counts, source content/policy proofs, administrative cascade cleanup and separation limits. Their deaths do not manufacture ordinary rewards. Generic encounter batches stage all target changes and run the rule-owner validation before publication; quest entry restores the carrier if placement or projection fails.

Arena and Drome membership is held on the exact slot. Reviewed entry/return callers, native wave production and timeout removal bound the unresolved integration question. Forge and Echo variants preserve their source policy while changing dynamic combat facts. Echo portal activation re-proves occurrence and current player after asynchronous reads, excludes death retriggering and keeps native incarnation/corpse settlement checks. Live child expiry is distinct from portal visibility lifetime. Qualified activity bosses and persistent boss-clock realization keep their separate spawn ownership; the actual clock claim remains the durable caller's responsibility.

## Remaining work and validation

NPC/hireling carrier implementation, source spawn realization, full condition owner, Soulpit, remaining test topics and fixtures are still explicit remaining scope. Each credited range and SHA-256 matches the immutable clean input. Tests described above were inspected as source; no entities Cargo suite, server, PostgreSQL or native-client test was executed. The separate earlier domain tests retain their original narrow results and do not validate these carrier paths.
