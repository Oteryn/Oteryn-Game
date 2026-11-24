# NPC sweep evidence

The serial `0:50` batch ran against `411537ab4eeded9c01b7abbec3d7365be9f6fa43` and returned `NODE_BOOT_RESULT=NODE_BOOT_FAIL` in unprivileged mode. It reached all 50 placement rows and recorded 32 live passes and 355 live failures. Of the live failures, 245 rows were blocked by placement and 49 summarized failed entries. The separate census counted 1,291 definitions, 1,056 placed NPCs across 1,119 placement records, 14 missing donor placements, and 221 definitions without a donor map placement.

## Bootstrap E6006

In the interrupted serial attempt, Game logged `DURABILITY_PASS_RETIRED ... boss_encounter.rs:470:14 reason=closed` at 19:28:38 UTC. The gameplay loop then ended, readiness fell, and Game completed shutdown before the two `E6006` responses. The ops client maps Unix socket exchange failures to ambiguity at `apps/game-server/src/bin/oteryn-game-ops.rs:769`. The gameplay supervisor initiates node shutdown when serving exits at `apps/game-server/src/node/serve/supervision.rs:246`. The boss owner can turn errors other than root unavailability into `StaleOwner` at `crates/gameplay/dispatch/src/gameplay_transport/world_respawns.rs:206`. The transport then returns owner unavailability on a non-shutdown respawn exit at `crates/gameplay/dispatch/src/gameplay_transport/serving.rs:279`. This is a candidate for the underlying serve error. The public boot error discards the concrete serve error at `apps/game-server/src/node/serve/world_reset.rs:67`, so the trace cannot establish the exact boss operation error. The completed serial batch passed Character bootstrap.

## Admission and connection close evidence for fxf

| Attempt | Trace | Evidence |
|---|---|---|
| Earlier `0:50` | `01a20efc-98c2-7390-99e7-27714e06f56d` | Channel capacity pass exceeded its deadline after 3,183 ms at `crates/gameplay/durability/src/durability/channel_switch.rs:80`. Admission refused as unavailable after 6,459 ms, and the client saw EOF. The unavailable branch can return without writing a protocol error at `crates/gameplay/dispatch/src/gameplay_transport/connection.rs:957`. |
| Earlier `50:50` | `01a20efd-e8b5-70f5-8995-a777d44c6930` | Familiar login reached a playable decision. Before `ServerAccepted`, the account badge pass exceeded its deadline at `crates/gameplay/durability/src/durability/account_badge.rs:52` and Character authority waited twice for the durability holder. The client timed out at 20.014 s with no frame. Acceptance is written at `crates/gameplay/dispatch/src/gameplay_transport/connection.rs:1022`. |
| Serial `0:50`, `a_bearded_woman` | `01a20f10-a2c8-7017-921a-55d9bcce5f62` | Game admitted the connection after 18,158 ms, then logged `join_prepare_failed phase=sessions field_code=1051` at `crates/gameplay/dispatch/src/gameplay_transport/connection/serve/setup.rs:147`. Additional serial traces recorded the same field code in the initial and visibility phases. |

After a Character death in the serial batch, 24 placement rows failed because the shared fixture had no backpack for currency seeding at `apps/game-server/src/qualification/client_phases/npc_sweep.rs:844`. Those rows provide fixture evidence. The batch left server behavior and test expectations intact.
