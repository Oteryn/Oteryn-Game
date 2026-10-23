# Gameplay dispatch

## Adding a mechanic to the gameplay transport

Follow [ADR-0022](../../../docs/architecture/ADR-0022-gameplay-crate-layering.md):
keep pure rules and runtime/durable operations in their owning lower layers;
this crate owns dispatch and composition.

- Add Default-constructed per-mechanic Channel state to `ChannelSessionStates`
  in alphabetical field order.
- Put per-session cleanup in `ChannelSessionStates::forget_session`, preserving
  existing lock order and lifecycle semantics. Keep cleanup with ordering
  constraints relative to non-moved owners at its original boundary and explain why.
- Keep `FreshAdmissionAuthority` delegations in a per-mechanic macro or methods
  in the mechanic's own module. Invoke macros such as `x::composed_methods!()`
  from the impl in alphabetical position, never at the top or bottom.
- Insert test modules in alphabetical position; never append to registries.
- `composed_test_owner::ComposedTestOwner::build` is the only test constructor
  for `ComposedFreshAdmission`. Reuse existing fixtures and apply deliberate
  test-specific overrides after construction.
