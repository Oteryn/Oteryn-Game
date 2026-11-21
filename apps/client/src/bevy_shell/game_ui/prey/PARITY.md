# Prey presentation (batch 32)

Base: `9306b4eadb1209754d65ebd33c31c12291cce1fd`.

| Gap | Presentation / evidence | Reference |
| --- | --- | --- |
| Locked slot controls | Open the real Store browser; close Prey first so its modal cannot block Store. No local entitlement grant or invented offer/price. Refresh Prey after purchase to read server ownership. | `docs/reference/tibia-manual/combat.md:111` (CIPSOFT_OFFICIAL manual extract): slot 2 Premium or permanent Store purchase, slot 3 permanent Store purchase. |
| Resource/cost feedback | Server-confirmed wildcard balance, list size and expansion ownership; one-wildcard bonus reroll, one/five-wildcard renewal labels; insufficient-balance controls disabled and direct activation fenced. Next free reroll shown from the server timestamp. Paid reroll confirmation explicitly warns that the server does not supply the gold price. | `docs/reference/tibia-manual/combat.md:104-110` (CIPSOFT_OFFICIAL manual extract); admitted implementation `crates/gameplay/durability/src/durability/prey/command.rs:269-276`. |
| Artwork | NEEDS-CONTRACT: `prey::Reply` carries creature keys only. Need an accepted server creature-appearance catalogue message mapping each Prey creature key to a verified outfit look_type (and any required variant fields). No numeric IDs guessed from names, no substitute pictures. | OTClient `edubart/otclient@dab86e610991092fe8b2471f7de55a7c986b3cd2`, `src/client/uicreature.cpp:27-59`: `UICreature::drawSelf` delegates to a creature draw with an explicit outfit; `onStyleApply` reads `outfit-id`. Structure reference only; upstream has no Prey module. |
| Exact gold quote/balance | NEEDS-CONTRACT: accepted Prey query/reply extension or quote message providing per-slot list-reroll gold cost and spendable gold balance, ideally an expected-price fence on `Request::ListReroll`. Existing `OfferedCreature.price_gold` quotes **PickAgain**, not list rerolls. No protocol change in this batch. | `crates/protocol-oteryn/src/prey.rs`: `Slot`, `Reply`, `Request::ListReroll`. |

Capability 140 remains unoffered by all shipped offers. This batch does not change
negotiation or implement batch 31. The controls consume the existing session-owned
Prey reply path when negotiated; unavailable sessions continue to say so. Store
availability and purchase confirmation remain owned by the existing Store flow.

Regression tests drive the Bevy projection/Activate observer with encoded replies
through the real PlayLink mailbox. Only tests use protocol fixtures; production reads the existing server reply path.
