# #508 Phase A exact actor resolution — candidate evidence

## Authority and scope

- `PROVEN`: #162 allocation comment `5814505244` owns exactly seven paths on `main@a9c72f5db14f48f428df9b12d200f9566ad36955`.
- `PROVEN`: #508 terminal preflight `5712567820` identifies the existing bounded Channel carrier as the sole actor identity/generation source. The accepted GAME-ABILITY-01 target pipeline separates proposal, authoritative resolution, legality and effect planning.
- `DERIVED`: a read-only borrow of the existing carrier and independently current continuity guard is sufficient for a single direct slot/generation check. The one candidate and one result are fields, not retained collections.

## Implemented boundary

The `ExactActorRef` is opaque outside Foundation and wraps the carrier's existing World, Channel, scope generation, local identity and local generation. The carrier's `CurrentOwnerExactActorLookup` borrows both the carrier and live continuity guard; `contains` calls its existing direct `lookup`. Invalid, vacant, recycled and wrong-scope references all resolve as `NotCurrentActor`. Ability receives neither carrier storage nor grant, admission, removal or continuity authority.

`ExactActorProposal` carries an already issued exact reference and Client/AI provenance; both call `resolve_exact_actor`. The resolver accepts the concrete opaque Foundation `ExactActorRef` and `CurrentOwnerExactActorLookup`, with no crate-visible trait that another module could implement to counterfeit a lookup. The standalone Ability fixture test has no Foundation root, so `ability/mod.rs` excludes this resolver in its standalone test build; the focused Foundation harness includes the real Ability occurrence and resolver source files with only the minimal error variants needed by the included occurrence source. It does not import the fixture Ability's unrelated modules. Only the resolver can construct `ResolvedExactActor`. Its output snapshots the exact occurrence and all semantic revisions plus the resolved reference; `reconcile` rejects occurrence, revision, target and source substitution. The snapshot is not a current authority capability: any later mutation must recheck current owner and separately pass legality/effect gates. No production composition exists in this child; the Foundation test module constructs the preproduction carrier with its private test grant.

## Resource and negative matrix

| Boundary | Evidence | Resource effect |
|---|---|---|
| Exact current actor, Client and AI | Same direct lookup method, same target; provenance remains distinct | One candidate, one result, inside AB-RL-01/02 |
| Missing local identity and vacant slot | Fail closed on direct slot lookup | No scan or fallback |
| Recycled local identity | Prior generation fails, new generation succeeds | Existing slot/generation only |
| World, Channel, scope generation | Each mismatch fails independently | No cross-scope route |
| Changed current owner generation | Old carrier/ref fail with fresh guard | No owner grant exposed |
| Retry changed occurrence, revision, target or source | Stateless reconciliation rejects | No queue, map or history |

AB-RL-03 geometry and AB-RL-04 retargeting remain excluded: no position, range, LoS, floor, spatial enumeration or dynamic target selection exists in the new path. No `TargetId(String)` conversion exists. Target resolution has no collection or map enumeration order.

## Qualification status

- Focused RED: test harness was authored first against unresolved `ExactActorRef`, proposal and resolver names; local Rust tooling is unavailable, so no executed RED result is claimed.
- Focused GREEN: all three Foundation exact-actor composition tests passed in the coordinator's isolated Linux Rust 1.94 run. Ten existing standalone `ability_engine` integration tests passed, preserving fixture behavior.
- Repaired candidate validation: isolated Linux Rust 1.94 rerun passed all three focused Foundation tests and all ten existing standalone Ability integration tests; `cargo +1.94.0 clippy --locked -p oteryn-game-server --lib --tests -- -D warnings` and `cargo +1.94.0 fmt --all -- --check` both passed, process exit 0.
- Whole-diff self-review: seven-path ownership confirmed; `git diff --check` clean. The only lookup dispatch is the existing direct `lookup` on borrowed current guard/carrier. An independent review found the former crate-visible sealed trait was implementable by another crate module; the replacement accepts only concrete opaque Foundation types. No production admission, grant, remove, store, position, geometry, scan, collection, identity conversion or `lib.rs` edit was added. Each negative test perturbs one field/fact except the separate valid other-carrier comparison; the independent world/channel/generation tests cover cross-scope substitution. The only retry state is the value returned to the caller, with no retained queue/history.
- Independent high-risk exact-head review and protected Merge Queue `game-gate`: pending coordinator publication/freeze.

This is a preproduction, test-composed identity seam only. It does not qualify production actor admission, gameplay legality, a Reference spell or live deployment.
