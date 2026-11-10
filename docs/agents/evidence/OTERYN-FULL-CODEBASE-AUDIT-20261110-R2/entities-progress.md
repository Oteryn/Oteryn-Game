# Gameplay entities — partial source review

Input: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. Status: **IN PROGRESS**. The frozen coverage ledger records 11 complete package files and 5,446 deduplicated line positions out of 86 tracked files. Remaining paths are explicit in that ledger. This report does not mark the package REVIEWED.

## Current authority and actor lifecycle

The reviewed carrier binds a committed runtime assignment, World, scoped identity, ownership generation and node incarnation to a one-use continuity grant. Bootstrap validates capacity and preallocates slots before consuming continuity. Slot admission, reservation, commitment, removal and local-generation exhaustion are separate transitions. Current references recheck scope, ownership generation and local generation. Retirement invalidates current owner stamps and continuity. The real durable assignment producer remains separate audit scope.

Player admission distinguishes uncommitted reservations from committed Session-bound slots. Attacker authority is derived from the exact bound Character lease and Session; current mutation checks include the owner fence and pending transition state. Control-loss epochs, loss marks and write fences join or clear only their matching transition. A caller claiming durable completion does not by itself establish that completion, so the full caller chain remains necessary.

Spawn activation uses bounded source/point windows, explicit player headroom and rollback of newly activated actors. Respawn consumes bounded attempts and retains unsuccessful pending work. Owner memo entries compare exact actors, so a reused local slot cannot inherit a predecessor generation's memo. Native child cleanup, specialized encounter ownership and the full position/custody implementations remain to be reviewed.

## Damage, death and observation

The complete damage ledger was inspected through command-origin replay, superseded-session and high-water rules, staged mutation, actual removed-HP attribution, exact current winning principal, retained lethal projection and memoized per-principal reward occurrence. Corpse retention and active settlement pins have distinct bounds. Source comments were reconciled against the later accepted D4 receipt decision; its explicit residual retention limit is an accepted design limitation, not a newly confirmed defect.

Hazard state binds current Player, Session, content World and a bounded level. Party projections validate unique members, leadership, self membership, ordering, presence relation and current player state. They are cached observations, not durable authorization. Independent caller inspection refutes the earlier proposed party-cache error fallthrough: the same immutable runtime has already validated the relevant slot and Session before that cache read.

The special visibility and occupancy helpers were read with their current context and incarnation checks. Possible policy questions are not findings without their real production caller and a meaningful consequence. No new confirmed finding has been added from this partial package review.

## Verification limit

Every credited range and SHA-256 was checked against the exact clean input. No entities Cargo suite, server process, native client or PostgreSQL test was executed. This is substantive partial source review, not runtime VERIFIED status.
