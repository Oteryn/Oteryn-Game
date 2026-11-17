# Santa production producer boundary (2026-11-05)

OTS_HYPOTHESIS_ONLY: donor code is source data, not implementation authority beyond the owner-approved source comparison.

STOP: the gift callback is established; a source-backed production Santa placement/activation is missing.
Crystal `201189f871d23597b82caf88fd611454a2ddefb8`, `data-global/npc/santa_claus.lua:45-118`, establishes ten normal gift groups, container 6510, permanent storage 840293 and eligibility storage != 1.
Its ordered thresholds admit only the normal group for rolls 1–90; rolls 91–100 select an empty table before container creation; no annual claim reset is established.
Neither complete pinned donor tree's NPC placement XML nor current native NPC placements places Santa; declaring a catalogue NPC does not create a live actor.
Required owner/source input: an actual Santa placement and activation contract; no coordinates, annual reset, or seasonal window are invented and no unwired gift helper is added.

## Placement census

Independently fetched every `*-npc.xml` in both complete recursive Git trees (`truncated:false`). Each file below contains zero case-insensitive `santa` occurrences (entire file, lines 1 through the stated count). Raw URL template: `https://raw.githubusercontent.com/{repository}/{revision}/{path}`, repositories `zimbadev/crystalserver` and `opentibiabr/canary`; Canary revision `47dfd51f45280a59a1d3e50ba7edd573d7234446`. Fetches succeeded; no refused URL in this census. XML artifacts are retained under `/tmp/world-src-cont/santa-route/{donor}/` during research.

| Donor | Path | Lines searched | SHA256 |
| --- | --- | ---: | --- |
| crystal | `data-crystal/world/world-npc.xml` | 319 | `4e9c3ce7e21b55e78082ec2502b6c187f5e4b890e36d7c7cb592edfa0f27594e` |
| crystal | `data-global/world/annual_events/winterlight_solstice/island-npc.xml` | 15 | `2335f7e2097ff12ede5e10d14efc9bbd34ade9be7d4178b2e13362e85b308565` |
| crystal | `data-global/world/custom/global-custom-npc.xml` | 57 | `3aaf7740399fc615970d5d0c5f9f4a6eaf8fdc74ad7f455f0bfcfb030c1ea8bd` |
| crystal | `data-global/world/world-npc.xml` | 3341 | `db07b021856008bce61efe816e8a8eea00a01cb9750f1be5e750093758d1c029` |
| canary | `data-canary/world/canary-npc.xml` | 21 | `d357a83d07bf31ddf58d20445bfaaa938a50d850a6949b94bc167e43491465c6` |
| canary | `data-otservbr-global/world/annual_events/winterlight_solstice/island-npc.xml` | 15 | `2335f7e2097ff12ede5e10d14efc9bbd34ade9be7d4178b2e13362e85b308565` |
| canary | `data-otservbr-global/world/custom/otservbr-custom-npc.xml` | 57 | `6196e92cac5301d463f178b38ef60ec5ebbcd991f18cd03b361091c5d29f8f29` |
| canary | `data-otservbr-global/world/otservbr-npc.xml` | 3025 | `a02d95945d73c1f5c24f9b0446d3f63102d2bfa2d4c2d4a59bb40c1ea3fd2fe9` |

Current `content/world/npc-placements/*.json` likewise contains no Santa key/name, including held placements. `content/npcs/definitions/npcs-01000-01281.json` and `content/world/definitions/declarations.json` declare `oteryn:npc.santa_claus` with `dialogue:null`, `services:[]`; this alone supplies no actor. `apps/game-server/src/node/serve/bundle_npcs.rs:285-308` constructs actors from compiled placement entries, not every definition.

## Existing native route if placement becomes established

`npc_catalogue/service.rs:410-465` generates ordinary replies for a null dialogue; `crates/gameplay/npc/src/npc_talk.rs:207` admits those replies. Thus null dialogue is not itself a blocker. Existing command-7 `npc_command.rs` checks live visible NPC, conversation/range and current generation before its source-specific selectors. An unprotected Santa selector/caller could attach there using the exact native NPC key.

Generic `npc_command/quest_dialogue.rs` grants only QuestState transitions, not inventory authority. Existing public `DurabilityRoot::freeze_reward_claim_mint_weighted` (`durability/reward_claim_mint.rs:507`) and `commit_reward_claim_mint_weighted_noticed` (`:752`) already support a container plus direct contents, once-per-character claim, native item weights, command replay, and an atomic pending `quest_transition` obligation (`:1276`). `dispatch/gameplay_transport/bundle_chest.rs:168-244` demonstrates the real mint caller and quest refresh recovery pattern. Reusing these APIs requires no edit to protected durability module exports or mint audit causes; a new source-specific caller must prove actual Santa interaction rather than impersonate a chest. A source-backed permanent claim key and generic quest obligation could represent storage 840293 after mint; claim eligibility must explicitly preserve storage != 1. This is an integration candidate, not implemented production behavior.
