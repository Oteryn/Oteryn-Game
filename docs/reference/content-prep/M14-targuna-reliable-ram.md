# SRV7 Reliable Ram: source boundary and qualification

Implementation in progress; the claim is not DONE and no PG pass is claimed here.

* Crystal `201189f871d23597b82caf88fd611454a2ddefb8`, `data-global/scripts/quests/targuna/actions_reliable_ram.lua:20-50`: turnip item 53073 on Ram item 53439, source position 31926/31896/7, earned-turnip requirement, one-item consumption, mount and achievement, ReliableRam=1 and BurningHeart advancement.
* Same donor `data-global/npc/emiliana.lua:169-182`: the first turnip reward sets TurnipsReward=1 and BurningHeart.Mission=4 together. The admitted final callback requires this reachable earned state; it preserves TurnipsReward, sets ReliableRam=1 and Mission=5. The earlier NPC/quest stages are not declared complete by this slice.
* Same donor `data/XML/mounts.xml:279-281`: id250 is Jaracal/look1962; id252 is Reliable Ram/look1934. The action's hardcoded250 is stale. Recorded compatibility decision: bind the semantic Reliable Ram identity and admitted look1934, never the donor's mutable ordinal. The retained `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d` corpus independently names Reliable Ram/look1934 (there at250), speed10, non-Premium. Existing cosmetics authoring materializes that source and pins all fields using its owning tool.
* Source XML SHA-256 at201189f: `e4ad7d0f7b712a77160484f4c70ef83c66af1c798950b3853acf23f4897c397f`. The admitted look1934 record is `7b7abf59582d1d05fb365f37ecc6225d587f637a2919c3996b7e506ef95a0da8`.
* Reuse: existing world-callback carried USE, source/target occurrence admission, item consumption, QuestState writer, account appearance receipts and achievement owner. No new command, capability, table or migration.
* Red regression: `cargo test --locked -p oteryn-gameplay-quests srv7_world_mount_reward_requires_once_only_progress -- --nocapture` failed because the world-effects decoder rejected mount rewards. The focused test passed after adding the bounded, once-only effect; later changes remain under validation.

Pending qualification: real framed PG case and its duration; final changed-crate tests, source/content checks, generated-file reconciliation, workspace check and formatting. The new PG scenario's initial state after Emiliana is an explicit fixture boundary, not a claim to replay the whole preceding quest.

## Read-only source URLs

- https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/targuna/actions_reliable_ram.lua
- https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/emiliana.lua
- https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data/XML/mounts.xml

No donor code or asset file is redistributed here. These are extracted rule and identity facts. Wiki cross-check: wiki-pending; the current implementation does not infer odds or invent a look.

The carried input also needed native physical admission: canonical i53073 exists but is not materializable, and Canary's older XML has no such item. Crystal201189f `data/items/items.xml:88677` supplies the plain portable turnip; the existing Grizzly physical qualifier and shared native profile merger capture it without changing canonical Item admission or enabling generic item scripts. Source: https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data/items/items.xml .

Success feedback retains the source hearts effect and message. `CONST_ME_HEARTS=36`: https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/utils/utils_definitions.hpp#L96 .
