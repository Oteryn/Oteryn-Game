# M31 — engworldchg

M31 remains in progress. The pushed foundation is the shared native binding and
current-activation boundary; it does not claim all ten personal journals complete.
The selected recipes explicitly replace global calendars/raids/thresholds with
personal on-demand journeys. Their original adaptations remain authoritative.

## Native binding foundation

- Migration1840 binds native layer, spawn-policy, NPC and raid identities to the
  immutable World/reset decision. New writes are atomic; legacy writers have a
  verified one-time upgrade. Required missing or mismatched bindings fail closed.
- Generic QuestState, world callback and travel activation reads can observe the
  existing native daily/counter owner. Epochzero grants no native activation;
  current World/reset checks and safe Hive egress remain in the existing owners.
- Source-reference facts for all ten recipes are recorded under
  `docs/reference/content-prep/engworldchg-m31/research.md`.

## Focused evidence

Baseline red: `engworldchg_grimvale_binding_restart_and_native_step_postgres`
executed PostgreSQL and failed on the absent binding relation in74.39seconds.
The first implementation run exposed an observation-fixture ordering error;
the explicit test journal write now follows all stateless-entry assertions.

- Two new binding unit cases pass.
- Changed-crate all-feature/all-target Cargo check, strict Clippy and format pass.
- Four real PostgreSQL cases pass in406.98seconds: the new Grimvale and Hive
  cases plus existing QWORLD occurrence/dispatch/restart and Yasir regressions.
  Each case exceeded0.01seconds; none was a route-less early return.
- World pin verification passes. No content input or node boot file is changed
  by this foundation.

Pending M31: bind every selected recipe's native stages and personal completion;
compose the Horse Station tame and journal in the same receipt; run authoring
checks, authoring unit tests, content-tree validation and final focused PG proofs.
Independent package review and protected integration belong to the coordinator.
