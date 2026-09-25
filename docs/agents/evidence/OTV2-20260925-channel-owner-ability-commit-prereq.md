# One-creature owner Ability commit evidence

Status: **AUTHORING**. Allocation: #162 comments 5828383421, 5828386989, 5828450388. Protected base: `bfc8b54548a09c59e062873a5dfa48739c477420`.

## Physical boundary

The existing `ChannelActorCarrier` retains one `CreatureOccupied` variant inside its already bounded slot array. At most one such creature is admitted to one carrier. It has explicit fixture initial HP, exact actor-local generation, optional position and one optional committed result. The result contains complete canonical plan bytes (not a hash), including the occurrence bytes, damage, HP before and HP after. The result has no growing replay collection. Slot overhead before commit is a bounded `Option<OwnerCommitRecord>`; after commit one `Box<[u8]>` retains at most 4096 bytes, with checked allocation before the slot mutation. Existing finite carrier capacity and checked slot-size arithmetic remain unchanged; no production capacity claim is made.

Ability validates the typed `EffectPlan` against `ResolvedExactActor`: equal full occurrence/revisions, exactly one target/candidate/effect, matching client/AI source, atomic mode, positive damage and effect target in the plan's sole resolved target. Its canonical full-field encoding includes occurrence, five revisions, proposal source, actor marker, counts, target, effect kind and magnitude, stages, owner scope, group and mode. The owner independently enforces actual encoded length <=4096, validates live continuity and slot-local generation at the write, computes checked HP transition, allocates one receipt, then changes HP and receipt together in that slot. The fixture `AbilityEngine` map does not participate.

## Focused matrix

`channel_owner_ability_commit_tests.rs` exercises typed resolver→plan→actual carrier commit, nonlethal and lethal HP, zero-HP lookup refusal, identical replay, revision/plan/target substitution, wrong scope, stale owner, administrative removal/recycle, invalid magnitude, checked overflow under malformed fixture HP and injected failure before write. Every rejected case compares the complete slot value before and after. No process-restart recovery is claimed. `tests/ability_engine.rs` retains its existing standalone fixture tests through a narrow local compile shim; real physical carrier tests live under Foundation.

## Qualification handoff

Local Rust compiler/toolchain: absent in this execution environment. Hosted focused Rust, fmt, strict Clippy, agent governance, exact-head gate, independent whole-diff review and protected Merge Queue are pending the coordinator's post-freeze lifecycle. RUNTIME-ACTOR-RL-01 (#530) and COMBAT-RL-02 (#506) remain open; no Combat worker is released by this candidate.
