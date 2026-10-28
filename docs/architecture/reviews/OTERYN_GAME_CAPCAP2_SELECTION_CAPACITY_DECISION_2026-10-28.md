# CAPCAP-2: negotiated selection capacity 64 → 112

Date: 2026-10-28. Status: accepted (owner delegation of architecture decisions, NIGHT-W1 owner decisions).

## Context

CAPCAP-1 (`OTERYN_GAME_CAPCAP1_SELECTION_CAPACITY_DECISION_2026-10-25.md`) raised `SELECTED_CAPACITY`
from 32 to 64 and said the next step would be the same kind of decision, up to the protocol's 128.

On main the bundle World now offers exactly 64 capabilities. Every open mechanic PR that adds a
bundle-offered capability makes `SelectedCapabilities::select` return `None` for a client that supports
everything. The offer-set tests then fail:

- `all_capability_offer_sets_and_gates_match_fragments`;
- `the_bundle_world_offered_set_is_3_the_item_view_set_12_14_and_18_requiring_4_and_6`;
- `caps_split_1_golden_offered_selection`.

This already happens on https://github.com/Oteryn/Oteryn-Game/pull/2668 (67 offered) and
https://github.com/Oteryn/Oteryn-Game/pull/2643.

## Decision

1. `SELECTED_CAPACITY` becomes 112.
2. The compile-time assertion becomes strict: `SELECTED_CAPACITY < MAX_CAPABILITY_COUNT`.
3. The existing tests stay unchanged and keep checking the bound itself:
   - the production, item-view and bundle offered sets stay within the capacity;
   - a closure of exactly `SELECTED_CAPACITY` boots, and `SELECTED_CAPACITY + 1` is refused as
     `SelectionUnavailable`;
   - a closure of `MAX_CAPABILITY_COUNT + 1` is refused as `ClosureLimitExceeded`;
   - a session selection over the capacity is refused, never truncated.

## Why 112 and not 128

At 128 the selection bound would equal the wire bound. A closure one above the selection capacity
would then be refused as `ClosureLimitExceeded`, and the `SelectionUnavailable` path for a closure
that fits the wire but not the selection could no longer be reached or tested. 112 keeps both failure
modes distinct and leaves room for about 48 more offered capabilities.

The struct is `Copy` and travels with session continuity; 112 ids are 448 bytes.

When the offered set approaches 112, the next step needs a protocol decision: raising
`MAX_CAPABILITY_COUNT` is a wire change, or capabilities are consolidated.

## Not changed

No wire format, registry entry, offered set or selection rule changes. Unrepresentable sets still
fail closed. Clients still get only the capabilities they support and the server offers.

## Rollback

Revert the commit while no offered set exceeds 64.
