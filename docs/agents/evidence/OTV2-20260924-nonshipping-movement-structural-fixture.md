# Non-shipping structural Movement fixture evidence

Task: `OTV2-20260924-nonshipping-movement-structural-fixture` (archived under `docs/agents/tasks/archive/`)
Allocation: [#162 comment 5823029665](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5823029665)
Implementation: [PR #870](https://github.com/Oteryn/Oteryn-Game/pull/870), final exact head `3c366e47b2c389ac94becb17dc1b90d0814d7ec5`
Base: protected `main@c516182255d3ea1724e91671c8d2187eb622e3df`
Integration: protected `main@1680eb5dc6145aa3e271ac8665f33a50ed837b76`

## Physically tested structure

| Case | Cell lookups | Owner result |
| --- | ---: | --- |
| N/E/S/W qualified synthetic WALKABLE | exactly 1 each | Checked adjacent same-floor target; revision +1 |
| Synthetic BLOCKED, absent, unqualified, conflict | exactly 1 each | Position/revision unchanged |
| Stale snapshot, wrong static scope, wrong position context, wrong actor scope | 0 each | Position/revision unchanged |
| Checked X/Y over- and underflow | 0 each | Position/revision unchanged |
| Recycled actor or advanced owner generation | 0 each | Current actor slot unchanged |
| Intervening owner commit after candidate read | exactly 1 for prepared proposal | Rejected proposal leaves intervening position/revision unchanged |
| Stale request and replayed proposal | 0 additional lookups | Position/revision unchanged |

Six focused fixture tests passed. The sole candidate access is one complete-key `ReferenceStaticCellIndex::lookup` call from the **existing test-only Content index source**. `CellProbe` counts it and fails on a second candidate. The kernel provides no scan or fallback. The actor position is held by the private Channel carrier and its compare-commit performs the position write. Two integration test crates include Foundation source without a Content module; the fixture therefore loads the existing index source by relative path with minimal **synthetic surrogate input value types**. The lib test crate also loads the source through Content; `#[allow(clippy::duplicate_mod)]` is narrowly attached to the test-only path declaration. This does not exercise actual Content input type binding, generation activation or production wiring.

## Exact-head and integration evidence

- Focused fixture: six tests passed on the final implementation candidate; [agent governance run 36070156300](https://github.com/Oteryn/Oteryn-Game/actions/runs/36070156300) and [architecture semantic audit 36070131181](https://github.com/Oteryn/Oteryn-Game/actions/runs/36070131181) succeeded on the same exact head.
- [Merge gate run 36070156440](https://github.com/Oteryn/Oteryn-Game/actions/runs/36070156440) succeeded on `3c366e47b2c389ac94becb17dc1b90d0814d7ec5`, including aggregate `game-gate`, Rust Linux workspace/build/tests, strict Clippy/fmt and PostgreSQL E2E. Independent Luna 6 [review](https://github.com/Oteryn/Oteryn-Game/pull/870#issuecomment-5823576557) passed on the same head.
- Governed submission: [META #196 comment 5823748228](https://github.com/Oteryn/Oteryn/issues/196#issuecomment-5823748228), executor run `36071334534`, queue UUID `e470234d-546d-4ade-9d7e-340cf20b6bb8`. Actual [merge_group run 36071383357](https://github.com/Oteryn/Oteryn-Game/actions/runs/36071383357) succeeded with aggregate `game-gate` including Linux, Windows and PostgreSQL; PR #870 merged, and protected `main@1680eb5dc6145aa3e271ac8665f33a50ed837b76` read back as its merge commit.
- [#162 closeout comment 5823905431](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5823905431) releases the fixture allocation. [#139 comment 5823901619](https://github.com/Oteryn/Oteryn-Game/issues/139#issuecomment-5823901619) records this physical component evidence and retains its production gates. [#162 archive-only allocation 5826803896](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5826803896) routes this historical packet from active to archive.

## Evidence limits

PROVEN: test-only kernel structure, checked cardinal destination, single candidate lookup, no scan, private owner compare-commit and unchanged position/revision on rejected operations. `MOVE-RL-02=NOT_EXERCISED_BY_COMPONENT` only for this isolated fixture.

NOT CLAIMED: actual Content type binding, generation activation, runtime wiring, July-28 Reference parity, accepted production maxima or production `MOVE-RL-03`. #139 remains open with `MOVE-RL-03=REQUIRED_NOW/UNDECIDED_MAX`; immutable 2026-07-28 Reference evidence linkage remains a separate gate.
