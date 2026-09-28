# TibiaWiki BR NPC snapshots

Exact-revision wikitext of every NPC page of TibiaWiki BR (`Categoria:NPCs no Tibia` and its
subcategories, plus `<NPC>/...` subpages), captured by `tools/content-schema/npc-authoring/wiki_br.py`.
The site refuses the build container, so the capture runs in `.github/workflows/npc-tibiawiki-br-capture.yml`
and its artifact is committed here. Each dated directory is immutable; a newer capture adds a new date
directory instead of overwriting one.

The pages are player-observed reference data about Tibia Global NPCs: position (`location` / `Mapa`),
trade lists (`buys` / `sells`) and in-game dialogue transcripts (`falas`). They are evidence for
comparison only, never Game truth by themselves, and are used under `LICENSE-ASSETS.md` and
`OTERYN_NPC_AUTHORING_SCHEMA_V1` D9. Authors: the TibiaWiki BR contributors; each page's history is at
`https://www.tibiawiki.com.br/index.php?curid=<pageid>&oldid=<revid>`.

| Capture | Pages | `pages_digest` | File SHA-256 | Source run |
| --- | --- | --- | --- | --- |
| `2026-09-28/tibiawiki-br-npc-snapshot.json` | 1,253 NPC pages, 0 subpages | `d9485ffeacb4ea17dba1a09087d2e15b06bb9baba6bcfee3d0f521aec0e61815` | `bb03c7265e16a15dcfca7f03df11f916523e306c036d5e5642a81c33e8637a11` | Actions run 36418989024, artifact 10967214435 (zip `sha256:97d6f833…c1cb4`), tool at `53dde009` |

`pages_digest` is the SHA-256 over the sorted `<pageid>:<revid>:<sha256>` lines; each page's `sha256` is
over its UTF-8 wikitext.
