# Simulation determinism crate governance

This crate owns protocol/persistence/UI-neutral deterministic simulation primitives only.

It may own:

- simulation determinism profile identity;
- checked exact/fixed-scale numeric helpers;
- deterministic gameplay decision derivation and purpose isolation;
- normalized semantic time values;
- canonical deterministic-state ordering/serialization/hash evidence.

It must not own:

- gameplay formulas, rates, XP/loot values or Reference behavior;
- transport, protocol framing or public gameplay wire IDs;
- Game Session, admission, reconnect, CharacterLease or security credentials;
- persistence schemas, durable transactions or economy conservation;
- process-global mutable gameplay RNG;
- direct system-clock reads as authoritative formula inputs;
- production deployment or live-resource behavior.

Deterministic gameplay decision roots may be replayable evidence but this crate grants no cryptographic/security-randomness authority. Do not expose secret seed material through Debug, telemetry or client-safe projections.

Any future change involving seed secrecy, durable-value arithmetic, protocol/session authority or multichannel fencing must follow the root high-risk independent-review policy.
