# House podium parity — PODIUM-WIRE-1

Reference pin: Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`. Global visible behaviour wins where it conflicts.

| Behaviour | Canary evidence | Oteryn evidence | Status / gap |
|---|---|---|---|
| Renown and vigour item identities | `data/items/items.xml:35973-35974,38707` (the retained import is `imports/canary/items-xml/items.xml:67038-67042,71209`) | `crates/gameplay/housing/src/house_podium.rs:4-5` | Implemented as content production keys. |
| Renown shows an outfit; vigour shows a mount | Canary pin, podium item handlers (source checkout not retained in this repository) | `crates/gameplay/housing/src/house_podium.rs:29-48` | Closed admission matrix; unknown keys are rejected. |
| Rotation is one of four directions | Canary podium serialization/handlers at the pin (source checkout not retained) | `crates/gameplay/housing/src/house_podium.rs:30`; `crates/protocol-oteryn/src/house_podium.rs:16-18` | Implemented and canonical on wire. |
| Display survives restart and remains attached to the physical house item | Canary podium attributes at the pin | `apps/game-server/migrations/0339_house_podium_displays.sql:4-25`; `apps/game-server/tests/support/authority_cases/house_podium.rs` | Durable table and restart qualification present. |
| Viewers see the selected display | Global visible behaviour; protocol command 176 is the viewer projection | `docs/contracts/protocol-oteryn/v1/house-podium-v1.md`; `crates/protocol-oteryn/src/house_podium.rs` | Wire representation specified; native client renderer remains explicitly unsupported. |
| Boss display / podium of tenacity | Canary item ids 42367-42368 in `imports/canary/items-xml/items.xml:74906-74910` | `crates/gameplay/housing/src/house_podium.rs:34-37` | **Gap:** no accepted Oteryn boss-display production key or boss-to-presentation mapping exists. Reserved kind is rejected; no value is invented. |

## Content mapping rule

Outfit, mount and boss identifiers are accepted only through the content-owner lookup. Numeric donor ids are evidence, never runtime identities. Until boss display content is admitted, boss projection is deliberately unavailable rather than silently substituting a creature or outfit.
