# NPC enrichment R21 — all 133 partial actors

All 133 actors retain `DATA_READY_PARTIAL`. The catalogue remains **1,282 NPC definitions and 836 Dialogue records**. This round enriches existing definitions rather than claiming new active gameplay NPCs.

| Data improvement | Actors / records |
|---|---:|
| Wiki city and location descriptions | 133 actors |
| Documented professions | 47 actors |
| Source explicitly says unknown profession | 86 actors, TODO retained |
| Quest references | 55 actors, actions disabled |
| BR coordinate references | 128 actors, no native placements |
| BR trade references | 19 actors, services disabled |
| Improved basic dialogue | 42 actors |
| Selected source greet/farewell | 50 messages across 40 actors |
| Selected source name/job | 26 replies across 15 actors |
| Transcript observations retained as documentary | 466 rows across 41 actors |
| Unassigned BR source utterances | 56 actors |
| Added ambient donor voice references | 7 actors |
| Added wiki voice section | Jata |

42 basic Dialogue definitions use actual source text with explicit owner-approved Oteryn greet/farewell/name/job mappings. Matching and state equivalence to the real game are unproven. Stateful exchanges remain documentary; no quest, trade or travel action is flattened into a static native operation. Crystal author TODO dialogue remains provisional.

**Appearances: 21 donor / 112 placeholder.** Crystal summer-update provides S'Zallar M'Andar outfit115, palette0 and addons0; missing mount remains defaulted. Dragon Ancestor Spirit movement2000/2 was incorrectly treated as literal: those assignments were commented TODOs. The selected Oteryn default is now stationary; the original source/comment and default choice remain distinguishable. Alternative Ned Nobel addon3 evidence does not overwrite the existing donor choice.

**Identity:** BR has133 pinned matches; the unchanged pinned TP snapshot has132. A byte-verified cached Tibiopedia page confirms Wyrdin’s Apprentice through an explicitly recorded actor-specific U+2019/U+0027 apostrophe variant. No global normalization or source-binding rewrite is made.

The closed overlay changes133 NPC declarations,42 Dialogue declarations and2 profiles. Exact before/after checks, whole predecessor and packet SHA pins, and atomic native validation preserve every unrelated declaration/profile, all services/offers/routes, source/import records, assets, world/editor and references. Prior append packets and historical verified predicates are unchanged.

## Verification and server boundary

12 authoring checks PASS: native materializer76 tests; repository4; existing-map mechanics2; new stage5; original stage7 separately; existing authoring/schema27; governance36; format, strict Clippy, successor and workbench checks. Full and qualified-predecessor generation match all11 canonical files, tree `c0d46eaa7380bea64e438f4d0855c582e0d2dc85d2e2adf388820ed1a263eb29`. Independent source and adapter reviews PASS. Receipts are in [validation.json](validation.json).

Reuse `apps/game-server/src/content/project/native_entry_room.json`; no second map or mini-server. The real-issued-World map case is ignored for absent Platform receipt. **Runtime-loaded NPCs0; NPC interaction smoke passes0.** Integration of NPC loading/placement and basic conversation remains a coordinator-owned runtime allocation. Protected CI, current-main integration, external required review and MQ remain pending.

## Source access

- [Canary](https://github.com/opentibiabr/canary/tree/04b83b512114bfd888000d6e1433ed8ecaec7c5b): retained public Git tree/raw captures, read locally; exact known actor/path sweep, not a global absence claim.
- [Crystal summer-update](https://github.com/zimbadev/crystalserver/tree/00ce02a57ca5a12e48f32a3476e37471167e4c3f): retained public Git captures; exact actor outfit and commented-movement proof, documentary world spawn entries.
- [S'Zallar source](https://github.com/zimbadev/crystalserver/blob/00ce02a57ca5a12e48f32a3476e37471167e4c3f/data-global/npc/szallar_mandar.lua), [Dragon Ancestor Spirit source](https://github.com/zimbadev/crystalserver/blob/00ce02a57ca5a12e48f32a3476e37471167e4c3f/data-global/npc/dragon_ancestor_spirit.lua).
- [TibiaWiki BR](https://www.tibiawiki.com.br/): committed public API snapshot2026-09-28 read locally, with page revision/SHA preserved.
- [Tibiopedia](https://tibiopedia.pl/): retained public HTTP HTML captures with exact body SHA/length, page captions, source-specific fields and capture time. No stable wiki revision token is claimed.
- [TibiaSecrets transcripts](https://tibiasecrets.com/transcripts) and pinned public Git transcript captures: retained bytes read locally, with source SHA and per-fragment checks. All exact actor URLs/revisions/hashes and documentary observations are in [source-facts.json](source-facts.json).

Fresh Tavily search returned432 usage limit. Fresh BR API and TibiaSecrets HTTP returned403. The approved Remote Desktop browser fallback could not run: all devices were offline. No new live wiki/browser verification or computer changes are claimed. These limitations do not invalidate the separately verified retained public captures.

Remaining source gaps:112 appearance placeholders,86 unknown professions,91 basic Dialogue definitions without newly selected responses, incomplete stateful quest/service logic. All carry partial/default/TODO flags. Approximations do not replace runtime qualification.
