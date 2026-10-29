# CAPCAP-2: negotiated selection capacity 64 → 96

Date: 2026-10-29. Status: proposed with WEAPPROF-4; follows the path CAPCAP-1 set out.

## Context

CAPCAP-1 (`OTERYN_GAME_CAPCAP1_SELECTION_CAPACITY_DECISION_2026-10-25.md`) set `SELECTED_CAPACITY`
to 64 and said that when the offered set approaches 64, the next step is the same kind of decision,
up to the protocol's 128. On main the bundle World offers exactly 64 capabilities. WEAPPROF-4 offers
`weapon-proficiency-v1` (188) as the 65th, so `SelectedCapabilities::select` would return `None`
for a client that supports everything, and the offer-set tests fail.

## Decision

1. `SELECTED_CAPACITY` becomes 96.
2. The compile-time assertion `SELECTED_CAPACITY <= MAX_CAPABILITY_COUNT` stays.
3. The existing tests stay unchanged and keep checking the bound itself: the offered sets stay
   within the capacity; a closure of exactly `SELECTED_CAPACITY` boots and `SELECTED_CAPACITY + 1`
   is refused; a selection over the capacity is refused, never truncated.

## Why 96 and not 128

The boot-closure tests keep the selection bound distinct from the closure bound
(`MAX_CAPABILITY_COUNT`): a chain of `SELECTED_CAPACITY + 1` must fail as `SelectionUnavailable`,
not as `ClosureLimitExceeded`. That needs `SELECTED_CAPACITY < 128`. 96 leaves room for about 30
more offered capabilities, the same headroom CAPCAP-1 chose.

## Cost

The struct is `Copy` and travels with session continuity in memory only (no persistence or wire
change); its ids grow from 256 to 384 bytes.

## After this

A further step stays below 128 under the same rule; a bundle offer that needs the full 128 also
needs the selection-bound and closure-bound separation decided again.

## Not changed

No wire format, registry entry or selection rule changes. Unrepresentable sets still fail closed.

## Rollback

Revert the commit while no offered set exceeds 64.
