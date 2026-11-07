# R23 — all133 basic authoring choices closed, source uncertainty retained

All133 previously partial actors now have a name, an explicit role choice, concrete appearance choice and complete static **greet/farewell/name/job** replies. Status: **BASIC_AUTHORING_READY_APPROXIMATE**. This completes these basic authoring choices; canonical Tibia facts, runtime interaction, placements, quests and services are separate and remain unqualified.

The catalogue stays **1282 NPC /836 Dialogue**; the original160 queue is27 historically qualified definitions plus133 approximate/basic authoring definitions. Historical703 dialogue predicates, existing324 trade/56 travel services,12234 plain offers/195 routes and all unrelated identities/source/import/world/editor/assets are preserved.

## Actual current choices

| Field | Result |
| --- | --- |
| Basic name/role/appearance/dialogue coverage |133/133|
| greet/farewell/name/job components |532/532|
| Actors with selected source speech |96|
| Actors with only original Oteryn basic speech |37|
| New exact source component selections this round |13 for8 already source-enriched actors|
| New explicitly original Oteryn components |296|
| Existing better components retained |223, including prior original role prose|
| Documented wiki professions |47|
| Inferred roles backed by source evidence |65 (58 prior +7 this round)|
| Explicit project default roles |21|
| Original wiki profession says unknown |86, unchanged external source facts|
| Literal donor appearance choices |21, unchanged|
| Source-informed approximate appearance choices |99|
| Intentional neutral project appearance defaults |13|
| New exact actor appearance mappings |0|
| Runtime-loaded NPCs / NPC interaction smoke passes |0 /0|

All112 old appearance placeholders have explicit project choices, using28 distinct numeric look types across this set.99 profiles actually change;13 neutral selections intentionally keep128. Those13 are visible humanoid proxies for unknown objects/opaque actors, not a claim about their real shape or visibility. Template palettes/addons come from literal source profiles. Examples include gnome, naga, forest-fury and humanoid templates; a source template for another actor never proves target equivalence. Crystal author TODO profiles remain approximate, never verified target facts.

Roles preserve raw `wiki_profession`: selecting a useful Oteryn role does not change `nieznany` to a fabricated wiki fact. Fresh BR notes support four gnome daily-task contacts and Inquisition Guardsman's cave-entrance guard; Jelly/Yrlin source speech supports two further inferred roles.21 remaining roles are project defaults such as local resident, arena visitor or device interface. Original job prose can translate documented Polish profession labels or use selected roles; every new original reply explicitly has `source_quote:false`.

The packet has266 declaration overlays (133 NPC +133 Dialogue) and112 closed Presentation profile repairs. Actual changes:266 declarations /99 profiles.21 literal donor appearances and all Behavior profiles stay untouched. Retained keyword matcher/flow/children and existing story leaves are unchanged. No trade/reward/quest/travel actions or native placements are added. Quiet idle policy is an explicit project default; documentary voice references survive and no real-actor silence claim or ambient scheduler is introduced.

## Reproduction and actual checks

Predecessor is the complete pinned R22 tree `4c395bf099d057bae2619f49f4124f0eb5593aa7a297e19d53ee04f98f17d394`. Packet SHA256: `52022031c1b23ebaefbe3a42f4f324588935279f8601cd05b8cc89992c3ac898`. New tree: `c6a6e609c63e8965bce3ceb08df2681f4d00c0182384193fa68a3cd3ff64cb28`.

The new direct R22 fast path applies only R23; exact R21 fast and strictly pinned1149 reconstruction remain supported. Historical full generation retains R21→R22→R23. Each packet/complete predecessor is pinned; exact before rows and closed actor/profile scopes are required, and the native adapter validates a cloned draft before replacement. R23 cannot overwrite the21 donor profiles or any Behavior/foreign profile, even by a no-op proposal. The staging guard rejects rewritten quotes, wrong speakers, source equivalence claims, tampered template profiles and non-neutral purported defaults.

All12 authoring checks pass: native materializer78, repository4, existing-map mechanics2; current finish stage6 plus previous follow-up5/enrichment5/original7; authoring/schema27 and governance36. Strict Clippy, format, successor equivalence and workbench checks pass. All11 full/fast/current canonical documents are byte-identical. Independent source attribution/template and native-scope reviews pass. These checks ran in AUTHORING; frozen-head proof carries them only by checking exact same code/data bytes and actual log receipts, without inventing new frozen commands.

The existing `apps/game-server/src/content/project/native_entry_room.json` remains the only map fixture. Map mechanics checks do not prove NPC interaction. The real-issued World test stays ignored without the actual Platform issuer receipt. Formal coordinator review/protected CI/current-main integration/Merge Queue remain pending. No live deployment, runtime lease or first-class source-import authority is created.

## Sources and actual access methods

- [Crystal summer-update](https://github.com/zimbadev/crystalserver/tree/00ce02a57ca5a12e48f32a3476e37471167e4c3f): **new ordinary public GitHub/raw HTTP reads plus retained pinned NPC bodies**. All selected template body SHA/Git blob/revision and literal outfit quotes are retained in `source-facts.json`. These are source-template facts, not target NPC mappings. [Canary](https://github.com/opentibiabr/canary) pinned earlier actor evidence remains preserved; no repeat of the prior9213-body sweep.
- [s2ward/tibia pinned transcripts](https://github.com/s2ward/tibia/tree/8824eb38872a1174b0f0c923e08719e981f32ecc/data/npcs/text): **ordinary public GitHub/raw HTTP**,59 new bodies +27 reused nonempty captures,29 pinned empty files. Actor-labelled byte-bounded source speech adds13 components; previous better source speech is preserved. Incorrect John-the-Carpenter filename/Wes speaker attribution remains rejected.
- [TibiaSecrets transcripts](https://tibiasecrets.com/transcripts): **new ordinary HTTP with Mozilla UA**,200 index and4 target pages. Another UA returned403; no blanket availability claim. Index has33 actor labels, with no extra matching missing-actor transcript found in this bounded pass.
- [TibiaWiki BR](https://www.tibiawiki.com.br/): ordinary HTTP returned403, then **Remote Desktop Commander + real Chrome + CDP** successfully read18 public actor articles, including full hidden DOM sections. Exact URLs/titles/capture and HTML SHA/revision links are retained. No additional actor speech was present in these captures. Wounded Atui had a “Ver as transcrições desse NPC” label without corresponding speech. Stressed Storeman explicitly carries a contested-content warning; its uncertain assertions are excluded from confident selection. The remote computer was used only to launch/control/read a research browser; no project edits, builds, installations, system changes, private files or cleanup actions occurred.
- [Tibiopedia](https://tibiopedia.pl/): retained public captures are byte-verified and preserve original unknown professions and portrait labels. Fresh normal HTTP returned a setup preference page, excluded as article evidence. [raulmaiz/tibia_dungeons](https://github.com/raulmaiz/tibia_dungeons) fresh pinned ordinary GitHub read provides105 documentary identities and55 literal gender clues; article/graph IDs never become look IDs.

Initial large full-DOM output had terminal encoding loss/truncation and was excluded. Admitted18 complete rendered article captures use ASCII-escaped JSON preserving Unicode, rather than falsely claiming server-original bytes. Browser reads do not qualify source matcher/state behavior. No new image bytes were downloaded or redistributed; already retained public portrait images were inspected locally only as curation clues. No fresh Fandom verification or successful Tavily search is claimed in this round.

`progress.json` retains source-unknown flags, defaulted choices and deferred gameplay features. Empty services and disabled quests mean unimplemented features remain deferred; they are not a claim that original Tibia actors have no services/quests. Source unknowns were not erased to make basic authoring appear complete.
