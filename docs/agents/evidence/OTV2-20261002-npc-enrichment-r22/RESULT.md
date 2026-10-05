# R22 — provisional NPC source follow-up

The catalogue remains **1,282 NPC identities / 836 Dialogue definitions**. The original160 completion set remains27 historically qualified definitions plus133 explicitly partial definitions. This round selects further static utterances and approximate roles; it does not qualify gameplay.

## Results

| Field in the133-actor partial set | Current result |
| --- | --- |
| Basic dialogue with source-selected speech |96 (54 more actors than R21)|
| Only original Oteryn role prose |19|
| Generic basic dialogue |18|
| Wiki-documented profession |47, unchanged|
| Approximate role selected for wiki-unknown actor |58|
| No supported role selected |28|
| Original wiki profession remains unknown |86, unchanged source fact|
| Literal donor appearance |21, unchanged|
| Neutral native appearance with documentary portrait links |112|
| New numeric outfit mappings |0|
| Runtime NPCs loaded / NPC interaction smoke passes |0 /0|

The closed packet contains218 before/after repairs:133 NPC checks/overlays and85 Dialogue overlays. **204 declarations actually change (119 NPC +85 Dialogue);14 NPC records are identical before/after.** No authoring profiles change. This round selects52 greet/farewell messages and103 name/job/story candidates and creates38 explicit original project role replies, including actors with other source speech. Original project prose has `source_quote:false` and defaulted quality, never transcript status. Wiki profession values are preserved when inferred roles are added.

All1282 predecessor identities, historical703 dialogue predicates, services and native references survive. Trade/travel/quest/economic effects remain disabled for the133 partial actors. Optional story nodes are action-free final leaves; retained keyword matchers/flow are unchanged. A rejected `John_the_Carpenter.txt` actually contained Wes the Blacksmith speech: its filename was not accepted as speaker identity.

## Reproduction and validation

The generator admits only the complete pinned R21 predecessor tree `c0d46eaa7380bea64e438f4d0855c582e0d2dc85d2e2adf388820ed1a263eb29` for the new fast path. Historical full generation remains available. Packet SHA256: `c3a7322ddfe597ecba350b09bcac9ae224082362bcd67d84dcc796a96d9155f8`. New canonical tree: `4c395bf099d057bae2619f49f4124f0eb5593aa7a297e19d53ee04f98f17d394`.

All12 authoring checks pass: native materializer77, repository4, existing-map mechanics2; current follow-up stage5, previous enrichment5 and original stage7; authoring/schema27 and governance36. Strict Clippy, format, successor equivalence and workbench checks pass. All11 full/fast/current canonical documents are byte-identical. Independent source attribution and native scope reviews pass. These checks run in AUTHORING; frozen-head proof carries them only after verifying exact same code/data bytes and receipts.

Use the existing `apps/game-server/src/content/project/native_entry_room.json`; no second test map or alternate server is added. Map mechanics tests do not prove NPC interaction. The real-issued World case remains ignored without the actual Platform issuer receipt. Formal control-plane review, protected CI, current-main integration and Merge Queue remain coordinator-owned and pending; no runtime/production activation or merge is claimed.

## Sources and actual access

- [s2ward/tibia pinned transcript repository](https://github.com/s2ward/tibia/tree/8824eb38872a1174b0f0c923e08719e981f32ecc/data/npcs/text): **new ordinary public GitHub search/API/raw reads**. Source quotes carry repository revision, path, byte bounds, body/literal SHA and speaker attribution. This new lead supplies23 further actors after31 additional actors from retained BR captures.
- [Crystal summer-update](https://github.com/zimbadev/crystalserver/tree/summer-update) and [Canary](https://github.com/opentibiabr/canary): retained pinned public Git tree/donor captures, read locally and byte verified; existing appearance/dialogue facts are preserved. No repeat of the earlier9,213-body scan and no global absence claim.
- [TibiaWiki BR](https://www.tibiawiki.com.br/), [Tibiopedia](https://tibiopedia.pl/) and [TibiaSecrets transcripts](https://tibiasecrets.com/transcripts): retained public HTTP/API captures with exact page/body hashes. Fresh Tibiopedia yielded a setup preference page, excluded from article evidence. Prior normal BR/TibiaSecrets access returned403; fresh Remote Desktop availability check found all devices offline, so no browser access is claimed. No remote computer changes were performed.
- [raulmaiz/tibia_dungeons](https://github.com/raulmaiz/tibia_dungeons) and [elkolorado/tibia-nodes](https://github.com/elkolorado/tibia-nodes): new ordinary public GitHub reads, pinned revisions/body and Git blob hashes in `source-facts.json`; documentary image labels, positions, timestamps and graph IDs do not establish numeric native outfits. Twenty bounded exact-actor GitHub queries supplement the prior source scope.
- [Fandom transcript category](https://tibia.fandom.com/wiki/Category:Transcripts): requested reference; no successful fresh Fandom browser verification is claimed in this round. Tavily's earlier quota block is not represented as a successful search.

All112 portrait page bodies and literal actor-labelled image tags were checked. Image bytes downloaded/rehosted:0. A portrait URL is a documentary identity reference, not a numeric outfit, palette or placement proof. Evidence and remaining limitations are retained in `source-custody.json`, `source-facts.json`, `progress.json` and `validation.json`.
