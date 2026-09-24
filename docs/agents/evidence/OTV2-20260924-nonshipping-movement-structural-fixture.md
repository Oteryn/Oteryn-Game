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

The only cell access in the kernel is one call to the existing `ReferenceStaticCellIndex::lookup` with complete scope and checked target. `CellProbe` counts it and fails if a second candidate is attempted. There is no scan/fallback or second cell index. Actor state remains in the existing private Channel carrier, and the sole position write is its compare-commit.

## Evidence status and limits

- PROVEN by source inspection: this is a `#[cfg(test)]` child, with no `lib.rs` or production module registration. The Content index itself is test-only.
- PENDING: compile/test, strict Clippy/fmt and exact-head full `game-gate`, independent Luna 6 review, governed Merge Queue, real `merge_group` SUCCESS and protected-main readback. Local worker workspace has no `cargo`/`rustfmt`; do not record CI as passed until remote exact-head evidence exists.
- NOT CLAIMED: July-28 Reference parity, accepted production maxima, production MOVE-RL-03, Reference gates, Content activation or runtime wiring. #139 remains open under its existing production evidence linkage.
