# NPCFIX contract notes

- State-only NPC selection consumes an admitted quest copy. Missing inventory, actor or clock facts are unknown and refuse such a branch; they never become zero/default eligibility. Covered by the new refusal unit case.
- Existing admitted catalogue track ownership, defaults and bounds remain authoritative. Source programme values select replies and existing transitions; they do not silently widen persistence ranges.
- Father’s Burden acceptance is the source’s eleven-track atomic write, not eleven independent commands. Normalized source: content/dialogues/quest-rules/quest-batch3-a-fathers_burden.json; Crystal 201189f data-global/npc/tereban_functions.lua:173-184. Production catalogue PG proof and one-receipt replay passed.
- Tereban greeting: Crystal 201189f data-global/npc/tereban.lua:61, sha256 cd19384fbcdec240275b364a8d0c61be2b9f1d39bbbf6f401c268cc268c6b212. Shared farewell: npc_handler.lua:82, sha256 8479e2aa7904915ef23bcbc50eef3ac9c9a8b2588cc16dc8713f33fa0cfec35a (retained in content/dialogues/quests-2/npc-greetings.json).
- Direct-owner PG fixtures must perform initial quest admission with insert=true and bind the current CharacterLease exactly as existing Grizzly tests do. Refresh(false) deliberately cannot create a session. Advance the test owner clock for multiple talk windows; never disable the four-utterance ingress limit.
