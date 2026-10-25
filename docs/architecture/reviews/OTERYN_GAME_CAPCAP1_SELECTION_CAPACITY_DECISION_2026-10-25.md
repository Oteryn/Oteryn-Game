# CAPCAP-1: negotiated selection capacity 32 → 64

Date: 2026-10-25. Status: accepted (owner delegation of architecture decisions, NIGHT-W1 owner decisions).

## Context

`SelectedCapabilities` (`crates/gameplay/dispatch/src/gameplay_transport/capabilities.rs`) holds one
GameSession's selected capabilities in a fixed array of `SELECTED_CAPACITY` entries. The value was 32.
On main the bundle World now offers exactly 32 capabilities, so every new bundle-offered capability
(the parity wave adds 64, 66, 72, 75 and more) makes `SelectedCapabilities::select` return `None`
for a client that supports everything, and the offer-set tests fail.

ADMIT-CAP-1 (§cross-domain finding 3) left the capacity at 32 and said a change belongs to an
accepted owning-path change. ADMIT0 bounds the dependency closure by the protocol capability count
(`MAX_CAPABILITY_COUNT`, 128).

## Decision

1. `SELECTED_CAPACITY` becomes 64.
2. A compile-time assertion keeps `SELECTED_CAPACITY <= MAX_CAPABILITY_COUNT`, so the selection can
   never exceed the wire bound on `selected_capability_id`.
3. The existing tests stay unchanged and keep checking the bound itself:
   - the production, item-view and bundle offered sets stay within the capacity;
   - a closure of exactly `SELECTED_CAPACITY` boots, and `SELECTED_CAPACITY + 1` is refused as
     `SelectionUnavailable`;
   - a session selection over the capacity is refused, never truncated.

## Why 64 and not 128

64 leaves room for about 30 more offered capabilities. The struct is `Copy` and travels with session
continuity, so it stays small (256 bytes of ids). If the offered set approaches 64, the next step is
the same kind of decision, up to the protocol's 128.

## Not changed

No wire format, registry entry, offered set or selection rule changes. Unrepresentable sets still
fail closed. Clients still get only the capabilities they support and the server offers.

## Rollback

Revert the commit while no offered set exceeds 32.
