# Coordinator note (read at your next checkpoint)
1. Speed: first finish every quest that is only a reward chest/claim, in bulk, using one shared test template (as questpc3 did for 20 claims). Then storylines.
2. Shared hot file crates/gameplay/dispatch/src/gameplay_transport/npc_command.rs: owned by questpc. Other lanes put their code in their own submodule file (npc_command/<lane>_*.rs) and add at most one `mod` line; do not edit other parts of npc_command.rs.
3. Shared test/fixture fixes (stale lease, replay cursor, fresh-admission evidence, migration triggers): keep them minimal, in their own commit-sized hunk, and record each in .<lane>/SHARED-FIXES.md (file, reason) so the coordinator can merge them once.
4. Write a checkpoint after every 5 qualified quests, not only after a full batch, listing exactly which quests are PG-qualified, so finished quests can ship early.

## Owner note 2026-11-06 13:45 (read now, takes priority)
The owner wants FINISHED quests, not more research. From this checkpoint:
1. Stop collecting new reference material; you already have the sources downloaded. Use them.
2. Take the quests whose code you already wrote and finish them one by one: each quest ends with its own test passing (red first, then green, PG cases actually executed), fmt and clippy clean, and a line `QUALIFIED: <quest> - <test name>` in your report.
3. Finish in batches of at most 5 quests; after each batch write the QUALIFIED lines before starting the next batch.
4. Name new files and modules after the quest or mechanic (e.g. rookgaard_exchanges.rs), never after the lane (no questpcN_ in file, module or test names); rename existing ones when you touch them.
5. If a quest cannot be finished (missing engine feature), write `BLOCKED: <quest> - <missing piece>` and move on instead of building the feature.
Also read NOTE-npc.md (shared NPC research dir).

- QENGINE (owner 2026-11-06): missing shared engine features (StepOut, paired portals, asymmetric lever, encounter/party access, teleport facing/splash, reset/pickup) are being built in lane rs-wt/qengine. Keep marking those BLOCKED, keep going with other quests; do not implement them here.
