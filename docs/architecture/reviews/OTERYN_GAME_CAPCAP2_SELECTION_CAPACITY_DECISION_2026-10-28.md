# CAPCAP-2: negotiated selection capacity 64 → 120

Date: 2026-10-28. Status: accepted (owner delegation of architecture decisions, same scope as CAPCAP-1).

## Context

CAPCAP-1 raised `SELECTED_CAPACITY` (`crates/gameplay/dispatch/src/gameplay_transport/capabilities.rs`)
from 32 to 64. On main the bundle World now offers exactly 64 capabilities, so every open PR that adds
one more bundle-offered capability fails `all_capability_offer_sets_and_gates_match_fragments`,
`caps_split_1_golden_offered_selection` and
`the_bundle_world_offered_set_is_3_the_item_view_set_12_14_and_18_requiring_4_and_6`:
`SelectedCapabilities::select` returns `None` for a client that supports everything.

## Decision

1. `SELECTED_CAPACITY` becomes 120.
2. The compile-time assertion becomes strict: `SELECTED_CAPACITY < MAX_CAPABILITY_COUNT` (128). A
   closure of `SELECTED_CAPACITY + 1` must stay a valid capability list, so the boot and session
   tests keep telling the selection bound (`SelectionUnavailable`) apart from the protocol bound
   (`ClosureLimitExceeded`, `InvalidSupportList`).
3. The tests stay unchanged and keep checking the bound itself:
   - the production, item-view and bundle offered sets stay within the capacity;
   - a closure of exactly `SELECTED_CAPACITY` boots, and `SELECTED_CAPACITY + 1` is refused as
     `SelectionUnavailable`;
   - a session selection over the capacity is refused, never truncated.

## Why 120 and not 128

128 would make the selection bound equal to the protocol bound, and the `+ 1` cases above would then
test the protocol limit instead of the selection limit. 120 leaves room for 56 more offered
capabilities. The struct stays `Copy` (480 bytes of ids). When the offered set approaches 120, the
next step is a protocol decision (`MAX_CAPABILITY_COUNT`), not another constant bump.

## Not changed

No wire format, registry entry, offered set or selection rule changes. Unrepresentable sets still fail
closed. Clients still get only the capabilities they support and the server offers.

## Rollback

Revert the commit while no offered set exceeds 64.
