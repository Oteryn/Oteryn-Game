# Tutorial vocation v1

Capability 504 requires tutorial notices (31), offered only through BUNDLE_WORLD_OFFERED_CAPABILITIES. Command 6243 observes or submits the self-only Newhaven vocation choice. No NPC identity is involved.

## Request

Empty observes the offer. A single byte 1..5 selects Knight, Sorcerer, Druid, Paladin or Monk, in the card order recorded by `docs/agents/evidence/newhaven/tutorial-audit-2026-10-19.md` (02:36.5 and 04:12.5). Other input is rejected.

## Reply

Two bytes: `offer_mask:u8, chosen:u8`. Mask 31 offers all five cards; mask 0 closes the offer. Chosen 0 means vocationless; otherwise 1..5 uses the same card order. An offered observation always has chosen 0. No trailing bytes.

An offer requires the admitted Newhaven start, durable Island tutorial progress, a vocationless current build and current live Character ownership in the tutorial island. The first submission commits through the existing session-generation-fenced Character build writer after pending training and first-vocation vitals are saved. It preserves cumulative training, installs the live build and grants the accepted vocation starter kit. Retried intents retain the original occurrence and build predecessor until installed. Query/submit never changes a previously selected vocation. Death, logout or a new session do not confer a new offer on a vocated build.
