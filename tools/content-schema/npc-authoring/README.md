# NPC authoring (candidate v1)

Evidence tooling for `docs/architecture/OTERYN_NPC_AUTHORING_SCHEMA_V1.md`. Everything here is
`OTS_HYPOTHESIS_ONLY` source evidence with source-scoped `canary:` / `crystal:` keys; nothing is
Game truth and nothing writes `content/`.

| File | Purpose |
| --- | --- |
| `npc_sandbox.py` | Evaluates one NPC Lua file in a stubbed LuaJIT sandbox (lupa); only top-level code runs. |
| `convert.py` | Converts a pinned Canary or Crystal datapack into one candidate bundle per NPC file, plus placements from the spawn XML. |
| `npc.schema.json` | JSON Schema of a candidate bundle. |
| `validate_npc.py` | Schema plus semantic checks (status, key namespace, no committed text, every gate/script has an unresolved row). |
| `source_diff.py` | Canary vs Crystal fact-level diff (owner decision D2: equal sources, no automatic winner). |
| `wiki_fandom.py` | TibiaWiki (Fandom) snapshot fetch and position/travel/trade comparison (stdlib only, ≤2 requests/s, neutral User-Agent). |
| `wiki_br.py` | TibiaWiki BR NPC pages: `fetch` captures the raw wikitext at exact revisions (stdlib only, ≤2 requests/s, neutral User-Agent; run by `.github/workflows/npc-tibiawiki-br-capture.yml`, artifact only), `facts` reduces it to the committed facts in `imports/tibiawiki/npc-br/`. |
| `tibiopedia.py` | Tibiopedia NPC pages: `fetch` reads the NPC list from the sitemap and keeps only trade facts (trade flag, item, each row's price) in the committed `imports/tibiawiki/npc-tibiopedia/` file (D13); `self-test` checks the parser offline. |
| `tibiawiki_br_crosscheck.py` | Cross-checks every admitted NPC's position, trade and dialogue against the BR facts (`samples/tibiawiki-br-crosscheck-v1.json`). |
| `population_census.py` | Readiness census over converted bundles. |
| `promotion_candidates.py` | Merges Canary+Crystal with the wiki as tie-breaker into native-keyed promotion candidates (D4–D6). |
| `validate_promotion.py` | Checks a promotion-candidate report (keys, slugs, routes, placements, provenance, no text). |
| `samples/` | Committed census, diff and wiki-compare evidence and a few text-free sample bundles. |

## Reproduce

```sh
pip install -r ../monster-authoring/requirements.txt lupa==2.8
git clone --filter=blob:none https://github.com/opentibiabr/canary && git -C canary checkout 47dfd51f45280a59a1d3e50ba7edd573d7234446
git clone --filter=blob:none https://github.com/zimbadev/crystalserver crystal && git -C crystal checkout ff7ede593c69d4c658b382c97443e8155926924a
python convert.py --source canary --checkout canary --out out/canary
python convert.py --source crystal --checkout crystal --out out/crystal
python validate_npc.py out/canary/bundles out/crystal/bundles
python validate_npc.py samples/bundles
python population_census.py --bundles out/canary/bundles --index out/canary/index.json --out samples/census-canary-47dfd51f.json
python population_census.py --bundles out/crystal/bundles --index out/crystal/index.json --out samples/census-crystal-ff7ede59.json
python source_diff.py --canary out/canary/bundles --crystal out/crystal/bundles --out samples/source-diff-canary-47dfd51f-crystal-ff7ede59.json
python wiki_fandom.py self-test
python wiki_fandom.py fetch --cache out/fandom
python wiki_fandom.py compare --snapshot out/fandom/fandom-npc-snapshot.json --bundles out/canary/bundles --out samples/fandom-compare-canary-47dfd51f.json
python wiki_fandom.py compare --snapshot out/fandom/fandom-npc-snapshot.json --bundles out/crystal/bundles --out samples/fandom-compare-crystal-ff7ede59.json
(cd ../../.. && cargo +1.94.0 run --locked -p oteryn-game-server --example export_reference_item_identity_map -- "$OLDPWD/out/native-map.json")
python promotion_candidates.py --canary out/canary/bundles --crystal out/crystal/bundles --snapshot out/fandom/fandom-npc-snapshot.json --item-map out/native-map.json --br-facts ../../../imports/tibiawiki/npc-br/2026-09-28/tibiawiki-br-npc-facts.json --tibiopedia-facts ../../../imports/tibiawiki/npc-tibiopedia/2026-09-28/tibiopedia-npc-facts.json --out samples/promotion-candidates-v1.json
python validate_promotion.py samples/promotion-candidates-v1.json --snapshot out/fandom/fandom-npc-snapshot.json --br-facts ../../../imports/tibiawiki/npc-br/2026-09-28/tibiawiki-br-npc-facts.json --tibiopedia-facts ../../../imports/tibiawiki/npc-tibiopedia/2026-09-28/tibiopedia-npc-facts.json --item-map out/native-map.json
```

The Fandom snapshot (article fields only, not committed) takes about four minutes to fetch; the compare
evidence records its SHA-256. A full conversion takes under a minute per source and is byte-for-byte repeatable; each census
records `bundle_digest` over the converted bundles.

## Text

Dialogue, voice and description text is Tibia narrative content (LICENSE-ASSETS.md). Bundles carry
only a text reference (SHA-256, length, `|PLACEHOLDERS|`, `{links}`). `convert.py --include-text`
adds the text for local review; that output must never be committed, and `validate_npc.py` rejects it
unless run with `--allow-text`.
