# SRV7 Reliable Ram: source boundary and qualification

Implementation in progress; the claim is not DONE and no PG pass is claimed here.

* Crystal `201189f871d23597b82caf88fd611454a2ddefb8`, `data-global/scripts/quests/targuna/actions_reliable_ram.lua:20-50`: turnip item 53073 on Ram item 53439, source position 31926/31896/7, earned-turnip requirement, one-item consumption, mount and achievement, ReliableRam=1 and BurningHeart advancement.
* Same donor `data-global/npc/emiliana.lua:169-182`: the first turnip reward sets TurnipsReward=1 and BurningHeart.Mission=4 together. The admitted final callback requires this reachable earned state; it preserves TurnipsReward, sets ReliableRam=1 and Mission=5. The earlier NPC/quest stages are not declared complete by this slice.
* Same donor `data/XML/mounts.xml:279-281`: id250 is Jaracal/look1962; id252 is Reliable Ram/look1934. The action's hardcoded250 is stale. Recorded compatibility decision: bind the semantic Reliable Ram identity and admitted look1934, never the donor's mutable ordinal. The retained `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d` corpus independently names Reliable Ram/look1934 (there at250), speed10, non-Premium. Existing cosmetics authoring materializes that source and pins all fields using its owning tool.
* Source XML SHA-256 at201189f: `e4ad7d0f7b712a77160484f4c70ef83c66af1c798950b3853acf23f4897c397f`. The admitted look1934 record is `7b7abf59582d1d05fb365f37ecc6225d587f637a2919c3996b7e506ef95a0da8`.
* Reuse: existing world-callback carried USE, source/target occurrence admission, item consumption, QuestState writer, account appearance receipts and achievement owner. No new command, capability, table or migration.
* Red regression: `cargo test --locked -p oteryn-gameplay-quests srv7_world_mount_reward_requires_once_only_progress -- --nocapture` failed because the world-effects decoder rejected mount rewards. The focused test passed after adding the bounded, once-only effect; later changes remain under validation.

Pending qualification: real framed PG case and its duration; final changed-crate tests, source/content checks, generated-file reconciliation, workspace check and formatting. The new PG scenario's initial state after Emiliana is an explicit fixture boundary, not a claim to replay the whole preceding quest.

## Qualified M14 result

The focused real PG case passed before the final qmech integration merge: `srv7_reliable_ram_real_map_use_guards_grants_and_replays`, marker `SRV7_PG_EXECUTED reliable_ram duration=53.69670065s success=true`. It proves unearned refusal, one consumed turnip, one compatible Ram unlock, A reliable Friend, Mission4→5, Ram0→1, preserved earned-turnip flag, and exact historical command replay. The separate generic-taming-owner guard passed. The first production red was eligible framed USE returning NothingToUse; the fix adds an M14-only carried-source/map-target bridge to the existing world receipt owner. Fixture-only failures were repaired without weakening runtime guards or assertions.

Publishing branch: `claude/happy-davinci-ijz3wy-qm-srv7`. The coordinator owns CLAIMS.md and the final DONE marker. No migrations, capabilities or command IDs are introduced.
