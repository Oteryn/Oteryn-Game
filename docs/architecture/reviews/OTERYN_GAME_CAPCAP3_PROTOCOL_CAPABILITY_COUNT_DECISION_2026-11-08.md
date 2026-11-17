# CAPCAP-3: protocol capability count 128 → 256

Date: 2026-11-08. Status: accepted (owner decision A, 2026-11-08, relayed by the coordinator session).

## Context

`MAX_CAPABILITY_COUNT` (`crates/protocol-oteryn/src/lib.rs`) bounds every capability list on the
wire (bootstrap offer, `ServerAccepted` selection, activity and house-transfer handoffs). It was 128.
The registry (`crates/protocol-oteryn/capabilities/*.json`) holds 127 capabilities on main; the client
mechanics package adds 447 (imbuement details) and 448 (weapon proficiency state), 129 in all, so the
whole registered set can no longer be accepted at once and
`the_production_offered_set_is_the_registry_offered_set_and_registered` fails.

## Decision

1. `MAX_CAPABILITY_COUNT` becomes 256.
2. `SELECTED_CAPACITY` stays 112 (CAPCAP-2): the production, item-view and bundle offered sets still
   fit, and doubling the `Copy` selection would deepen debug-build test stacks for no current need.
   The strict compile-time assertion `SELECTED_CAPACITY < MAX_CAPABILITY_COUNT` stays; raising it is
   a later CAPCAP decision when an offered set approaches 112.
3. Large connection-scenario test futures are boxed (`Box::pin`) so the wider arrays fit the default
   test stack.
4. The count-bound tests keep checking the bound itself (exactly `MAX_CAPABILITY_COUNT` accepted,
   `MAX_CAPABILITY_COUNT + 1` refused); unrepresentable sets still fail closed, never truncated.

## Not changed

No wire encoding, registry entry, offered set or selection rule changes. Lists stay sorted, unique
and registered. A peer that sends more than 256 ids is refused as before at 128.

## Rollback

Revert the commit while the registry holds at most 128 capabilities.

## Amendment CAPCAP-4 (2026-11-09)

Status: accepted (owner decision via the coordinator, 2026-11-09).

Capabilities 449 (spellbook details) and 450 (server modal) bring the bundle-world offered set to
114, above `SELECTED_CAPACITY` 112. `SELECTED_CAPACITY` becomes 128. It stays strictly below
`MAX_CAPABILITY_COUNT` (256), the compile-time assertion stays, and the count-bound tests keep
checking the bound. The selection is an in-memory array in the session; no wire encoding changes.
Rollback: revert while every offered set holds at most 112 capabilities.
