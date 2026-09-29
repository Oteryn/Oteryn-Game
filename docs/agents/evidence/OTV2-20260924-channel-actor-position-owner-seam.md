# Channel actor position owner seam — preproduction evidence

## Authority and scope

- `PROVEN`: protected #162 allocation C comment `5815552283` provides exactly five paths and serializes this writer behind #508 on base `f812f6dc5586f0120a6e832fe58b7edd5c6ea474`.
- `PROVEN`: the existing Foundation carrier is private, fixed-capacity, scoped to one World/Channel/generation and already uses a separate surviving `NamespaceContinuityGuard`; #508 uses its direct exact-actor lookup. This child extends the same occupied slot.
- `DERIVED`: a single slot lookup and compare-commit can carry one local position and monotone revision without a duplicate runtime actor owner. Every error in new position operations precedes slot mutation.
- `UNKNOWN`: no production current-owner assignment issuer, qualified active Reference static-cell artifact, or exact caller's owner-work-cycle boundary is present. Fixture marker equality does not resolve these facts.

## Mutation and negative matrix

| Input or authority changed independently | Expected result | Mutation |
| --- | --- | --- |
| Valid exact actor and scope, initial absent position | Read rejects until one-time private initialization; revision starts at 1 | Only initializer writes one occupied slot |
| Exact snapshot and same full context | Compare-commit advances revision to 2 | Exactly one occupied slot changes |
| Repeated initializer, stale snapshot, changed position/revision | Reject | None |
| Wrong actor World, Channel or scope generation | Reject before slot access | None |
| Different local actor even with identical stored position/revision, or recycled generation | Reject; stored position binds actor local id/generation; recycled actor has no inherited position | None |
| Changed context World, Channel, owner generation, frame, map revision or content generation | Reject (each field perturbed alone) | None |
| Zero fixture frame/map/content marker | Reject | None |
| Independently current owner advances generation | Prior carrier read and commit reject | None |
| Revision at `u64::MAX` | Checked increment rejects | None |

The read and commit paths use `validate_ref` and direct indexed `slots[index]`, never scan actors or Content. `PositionSnapshot` is a value comparison, not a grant. No wall clock, client packet, path, diagonal step, stairs, occupancy, visibility, teleport or timing is introduced.

## Resource limits and acceptance status

The fixed `Slot` footprint changes by adding `Option<VersionedPosition>`. Admission uses the updated `size_of::<Slot>()` in its overflow preflight, but older explicit byte evidence must be remeasured before a numerical memory claim. One read and one write each touch one exact actor slot; #139 MOVE-RL-02 counts inputs **per owner work cycle**, so a per-call lookup count alone does not discharge it. MOVE-RL-03 and static cell bounds also require their separate owner acceptance.

The revised candidate passed four focused tests on isolated remote Linux Rust 1.94, strict game-server Clippy (`cargo +1.94.0 clippy --locked -p oteryn-game-server --lib --tests -- -D warnings`) and formatter (`cargo +1.94.0 fmt --all -- --check`); process exit 0. Tests were authored before executable remote validation, but no executed RED failure was captured, so RED remains **not evidenced**. `git diff --check` is clean on the four changed allocated paths. Exact-head CI and independent review await publication. This child provides a private owner seam only; no production activation, Reference parity, resource byte acceptance, or READY_FOR_IMPLEMENTATION claim follows from it alone.
