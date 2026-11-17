# Preserved NPC display names

Four normalized identity facts from the lane's previously checked source capture,
not a new quest or placement admission. The original capture is available at
`9bacece303995f3f70fdc1dd6a1b882f58a0cd76:tools/content-schema/npc-authoring/samples/npcfix-retail-placements.json:1`.
Its SHA-256 is `28239be1a5cd8cb4cad639735547d9897af6f6695234567ea35cb00240a8de1c`
(verified again before restoring these facts); source snapshot
`de3ca700ca19e351aade87b402dc3911004b8ac4`.

| Native key | Display name | Capture row | Pinned wiki revision |
| --- | --- | --- | --- |
| `oteryn:npc.ferryman_kamil_meluna` | Ferryman Kamil | 94 | `851735` |
| `oteryn:npc.harlow_vengoth` | Harlow | 132 | `1178796` |
| `oteryn:npc.junkar_mines` | Junkar | 153 | `1178799` |
| `oteryn:npc.maris_mistrock` | Maris | 185 | `1178794` |

Restore only the authored display-name field. Keep identity, services, dialogues,
source fields, anchors and movement unchanged. An existing different name, duplicate
identity or missing target refuses the entire correction before mutation.
The canonical materializer, not a hand-written manifest edit, seals the successor.
The runtime consumer validates names using the existing EntityDisplayName policy.
Local node-boot qualification is delegated to CI by the owner for this container.
