# Non-shipping structural Movement fixture evidence

Task: `OTV2-20260924-nonshipping-movement-structural-fixture`
Allocation: [#162 comment 5823029665](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5823029665)
Base: protected `main@c516182255d3ea1724e91671c8d2187eb622e3df`

## Structural assertions authored

| Case | Cell lookups | Expected owner effect |
| --- | ---: | --- |
| N/E/S/W qualified WALKABLE | exactly 1 each | Adjacent same-floor target, revision +1 |
| BLOCKED, absent, unqualified, conflict | exactly 1 each | Position/revision unchanged |
| Stale snapshot, wrong static scope, wrong position context, wrong actor scope | 0 each | Position/revision unchanged |
| Checked X/Y over- and underflow | 0 each | Position/revision unchanged |
| Recycled actor or advanced owner generation | 0 each | Current actor slot unchanged |
| Intervening owner commit after candidate read | exactly 1 for prepared proposal | Rejected proposal leaves intervening position/revision unchanged |
| Stale request and replayed proposal | 0 additional lookups | Position/revision unchanged |

The only cell access in the kernel is one call to `ReferenceStaticCellIndex::lookup` from the exact existing test-only Content index source, with complete scope and checked target. `CellProbe` counts it and fails if a second candidate is attempted. There is no scan/fallback or second cell-index implementation. Two integration targets include Foundation source as their own crate without a Content module, so this child includes the existing index source by relative path and supplies only minimal synthetic value-type stand-ins for its four Content inputs. It does not exercise real Content type construction, activation or wiring, and must not be cited for those claims. Actor state remains in the existing private Channel carrier, and the sole position write is its compare-commit.

The lib test crate also loads this same source through Content; strict Clippy therefore reports `duplicate_mod` for this deliberately duplicate test-only load. A narrow allow is attached only to the path declaration, while all other strict Clippy findings remain enforced.

## Evidence status and limits

- PROVEN by source inspection: this is a `#[cfg(test)]` child, with no `lib.rs` or production module registration. The reused Content index source itself is test-only; its inputs in this fixture are synthetic surrogates, not production Content types.
- PENDING: compile/test, strict Clippy/fmt and exact-head full `game-gate`, independent Luna 6 review, governed Merge Queue, real `merge_group` SUCCESS and protected-main readback. Local worker workspace has no `cargo`/`rustfmt`; do not record CI as passed until remote exact-head evidence exists.
- NOT CLAIMED: July-28 Reference parity, accepted production maxima, production MOVE-RL-03, Reference gates, Content activation or runtime wiring. #139 remains open under its existing production evidence linkage.
