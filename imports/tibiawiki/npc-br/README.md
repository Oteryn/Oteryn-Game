# TibiaWiki BR NPC facts

Compared facts from every NPC page of TibiaWiki BR (`Categoria:NPCs no Tibia` and its subcategories, plus
`<NPC>/...` subpages), extracted by `tools/content-schema/npc-authoring/wiki_br.py facts` from a raw
capture made by `.github/workflows/npc-tibiawiki-br-capture.yml` (the site refuses the build container).
Each dated directory is immutable; a newer capture adds a new date directory instead of overwriting one.

Per page: page id, exact revision and timestamp, the SHA-256 of the raw wikitext, infobox name,
`implemented` and `removed` versions, map positions, trade lists (item name and each row's explicit price) and the
lines the NPC itself speaks in the page's transcript. Those lines are Tibia NPC text, kept as reference
data under `LICENSE-ASSETS.md` and `OTERYN_NPC_AUTHORING_SCHEMA_V1` D3/D9. Wiki prose (notes,
descriptions) is not stored; the raw capture stays a CI artifact, and each page's `sha256` lets the facts
be checked against it or against the page's revision. Authors: the TibiaWiki BR contributors; each page's
history is at `https://www.tibiawiki.com.br/index.php?curid=<pageid>&oldid=<revid>`.

| Facts | Pages | Raw capture `pages_digest` | Source run |
| --- | --- | --- | --- |
| `2026-09-28/tibiawiki-br-npc-facts.json` | 1,253 NPC pages, 0 subpages, 0 missing | `d9485ffeacb4ea17dba1a09087d2e15b06bb9baba6bcfee3d0f521aec0e61815` | Actions run 36418989024, artifact 10967214435 (zip `sha256:97d6f833…c1cb4`, expires 2026-10-28); raw snapshot file `sha256:bb03c726…37a11` |

`pages_digest` is the SHA-256 over the sorted `<pageid>:<revid>:<sha256>` lines of the raw capture; the facts
also record `snapshot_sha256`, the SHA-256 of the whole raw snapshot file, which binds every copied value
(titles, timestamps, the missing-page inventory) to the capture.
