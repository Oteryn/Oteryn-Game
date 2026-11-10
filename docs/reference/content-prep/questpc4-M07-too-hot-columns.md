# M07 Too Hot to Handle: native column repair trial

The selected recipe is `tools/content-schema/quest-authoring/samples/completion242/recipes.json`, identity `oteryn:quest.too_hot_to_handle_quest`, stage s3: repair three columns with a chisel. It selects level80 and premium eligibility. This slice implements the physical repair occurrences, not the commission, preceding sarcophagus objective, quest completion or reward.

The captured map `content/world/placements/region-z09-x127-y124.b3`, SHA256 `2bfe818ad305727097f0672cb8164a72702e1f8d4591f5f6688ef1b5fbd5c07e`, contains three north-room columns36541 at32593,31841;32601,31841;32610,31841 on source floor9, each ordinal1. The geometry test decodes the source and verifies these positions. No map placements are changed.

`imports/canary/items-xml/items.xml:67137-67143`, pinned revision `47dfd51f45280a59a1d3e50ba7edd573d7234446`, calls36541/36542 column and36543 chisel. File SHA256 is `1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2`. Public source: https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/data/items/items.xml . This identifies the objects and tool; it does not supply an original repair action, recovered AID, or damaged/repaired visual pair.

Chosen Oteryn behavior: exact carried36543 is retained, each native column can give one personal repair credit, and the aggregate counter is bounded to three. The column retains its original appearance. A normal source revision still fences subsequent requests. A repeated command replays its receipt; a new command cannot repair the same column twice. Do not mislabel36542 as a repaired variant: the checked source names both column without that distinction.

These private physical credits may be earned before the commission. They neither start nor complete the canonical quest and do not assert the preceding five sarcophagi are finished. Commission/stage admission must consume or gate these occurrence credits when that separate selected journey is wired. This explicit preparation choice avoids inventing a completed predecessor or a missing NPC conversation.

The two PG cases use the captured native crop and separate historical characters at levels79/80, through the existing harness's initial progression insertion convention, with premium admission. They mint the real qualified chisel through the existing item authority, reject missing-tool/under-level USE, and check retained custody, exact durable per-site and aggregate credits, receipt replay, and no fresh duplicate credit.
