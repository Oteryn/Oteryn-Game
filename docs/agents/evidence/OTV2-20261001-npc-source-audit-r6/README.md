# NPC source audit R6 — requested source continuation

This round continues draft PR #1433 from `42aeacc910065f6a6d918d8fba9040bcc62412f7` using the owner's requested Canary, Crystal `summer-update`, TibiaWiki BR, Fandom, Tibiopedia, TibiaSecrets and native assets. The owner explicitly requested subagents in the current chat. Root is the sole branch writer; researchers produced read-only packets. R4/R5 and executable content remain unchanged. This is source research and tooling, not native admission or gameplay qualification.

## Dialogue observations

`source-recovery-r6.json` retains all 449 targeted acquisition attempts: 392 captures and 57 failures. Of the captures, 193 are missing-page stubs; they do not count as recovered dialogue. There are 546 named literal speech observations and seven separately held nonresponse/narrative fragments, with exact extracted UTF-8 byte/line ranges and hashes.

Three existing identities previously lacking primary programs have fresh speech observations: Captain Jack, Pescadu and Thorim. Fifteen previously empty identity proposals acquire literal observations. This reduces the R5 evidence gap from 170 to 167 existing identities and from 64 to 49 proposed identities; it does not admit any identity or source program. The Silent Oarsman's gestures and An Ominous Bat's narrative stay nonresponse observations. Variant names are not automatically associated with base NPCs.

The TibiaSecrets transcript index explicitly says that its transcripts are supplied to Tibiopedia and `s2ward/tibia`. The 35 previously source-bearing proposals recovered there are useful custody evidence, not 35 independent confirmations. Fandom's category is a discovery index; a listed page alone is no dialogue proof. Wiki prose and raw pages remain outside the repository. Original NPC speech is retained as attributed CipSoft reference data under `LICENSE-ASSETS.md` and NPC authoring D9.

## All 78 pending prices

`rendered-price-facts.json` contains 50 digest-bound BR/Fandom captures: all 41 requested BR NPC pages and nine Fandom pages. The parser recovers rendered template rows, preserves exact duplicate prices, and excludes conditioned rows from plain-price comparison. It holds quantities, subtype annotations, mixed currencies, ambiguous identity headings and historical/other sections. No rendered row is relabelled an explicit wikitext price.

`price-comparison.json` covers every R5 pending tuple: 46 have rendered observations matching the retained price, 12 have differing observations, and 20 have no exact registered-item match. These are observation states; **all 78 remain parity pending**. Template defaults, pack/unit semantics, renamed items, guards and historical dissent still require qualification. The original D12–D16 rules are not relaxed.

`vendor-source-index.json` binds 79 NPC files and 14 supporting files to verified Git blobs and SHA-256:

- Canary current `main`: `04b83b512114bfd888000d6e1433ed8ecaec7c5b`.
- Crystal requested `summer-update`: `00ce02a57ca5a12e48f32a3476e37471167e4c3f`.

The 149 matching static rows preserve client ID, direction, count/subtype, original row and byte/line proof. Of 156 vendor/tuple observations, 145 uniquely match retained prices and nine have no Canary discovery path (Aurelia, Avriel, Hector The Mentor). Two observations expose **Giri's duplicate `clientId=30061`**, named `giant amethyst` at 60,000 and `giant sapphire` at 50,000 in both sources. Vendor agreement cannot settle this identity collision. No row or price is guessed away. Postal base prices 15/8 remain distinct from conditional Postman 10/5.

Fresh Tibiopedia extraction failed for the attempted pages. `tibiopedia-search-status.json` preserves that outcome and indexed search excerpts separately: the Yasir excerpt contains both Dark Bell 250 and 310,000. An indexed excerpt is not an exact page/offer capture. Existing pinned Tibiopedia facts remain historical evidence.

## Native presentation assets

`presentation-source-check.json` verifies SHA-256 for all 6,248 present files under `content/assets/files`; the one previously documented omitted ZIP remains absent. The catalogue and appearance file are digest-bound. Hagor retains invalid feet 1156 across the four checked engine revisions. Quantitative wiki image fits exceed the D16 threshold of 35, so no palette replacement is qualified. The fitting environment's NumPy version differs from the historical reference and is recorded; these negative fits grant no admission.

A Sleeping Dragon's object 168 is absent from all four admitted appearance manifests; outfit 168 is also absent from the native 15.30 file. `lookTypeEx` is not silently interpreted as an outfit. Both presentation holds remain.

## Validation and reproduction

Run the retained rendered price reducer against externally retained native Tavily response JSON:

```sh
python tools/content-schema/npc-authoring/rendered_wiki_prices.py \
  --captures <selected-br-and-fandom-response-jsons> \
  --pending docs/agents/evidence/OTV2-20261001-npc-source-audit-r5/price-parity-pending.json \
  --out <output-directory>
python -m unittest discover -s tools/content-schema/npc-authoring -p 'test*.py'
```

Raw acquisition packets remain in the external `npc-r6-work` recovery artifact. The retained reports identify URLs, revisions/digest hints, request IDs and capture hashes; raw HTTP/wikitext bytes are not claimed for Tavily-extracted Markdown. Revisions reported by a rendered page are hints because transcluded templates may change independently. The manifest binds every retained JSON payload, the reducer/tests/fixtures and baseline inputs.

137 authoring tests pass, including seven new regression tests with fourteen independent-review fixtures. Hash/range verification passes for 2,172 wiki rows, 553 speech/nonresponse fragments, 93 vendor blobs and 149 vendor rows. Package preservation is checked against the predecessor: no native content, materializer, DTO, matcher, Quest identity or source runtime has changed. Required governance and candidate CI are recorded separately; live GitHub state remains lifecycle authority. This draft grants no review-trigger, queue, merge or production action.
