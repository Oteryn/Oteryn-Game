# M38 Eustacio's Spy Rat

The retained BR capture is
`imports/tibiawiki/npc-br/2026-09-28/tibiawiki-br-npc-facts.json`, page50622,
revision395123, title Eustacios Spy Rat, position33857,31333,9. The page's
SHA256 is `07dc11f7f928175e2bf6febe72dbea287c0c513d4123dded25d42a8ffb0b4959`.
Source: <https://www.tibiawiki.com.br/wiki/Eustacios_Spy_Rat?oldid=395123>.
The complete capture SHA256 is
`0773232ddd356be273474be7b3aea645ed5dbbf93e5832a94d565ad9e579657a`,
already registered by the normal package as `tibiawiki-br-npc-0773232ddd356be2`.
No extra import batch or fabricated source revision is needed.

The display name follows the retained Tibiopedia capture,
`imports/tibiawiki/npc-tibiopedia/2026-09-28/tibiopedia-npc-facts.json`,
Eustacio's Spy Rat, SHA256
`e1a3ceb218c9892e71bd7e26e64cd4b0e2a749f16c6dcf551ab62f4a8ff8b58e`:
<https://tibiopedia.pl/npcs/Eustacio's_Spy_Rat>.

Oteryn choices: reuse the existing Generic rat presentation
`oteryn:presentation.npc.rata_mari/definition-r1`
(`content/presentations/definitions/presentations-02500-02999.json`, outfit21),
and stationary passive Generic behavior
`oteryn:behavior.npc.a_swan/definition-r1`
(`content/behaviors/behaviors-01500-01999.json`, can_walk=false, hostile=false,
can_target=false). These are shared generic records, not copied NPC quest logic.
No canonical Spy Rat numeric appearance or retail visual equivalence is claimed.
The capture supplies no speech; no authored Dialogue, service, mission, exchange
or grant is added. Existing Oteryn generated-reply policy for dialogue-less NPCs
is preserved. North is the existing documented project default for unsourced facing.

The materializer's closed packet adds the NPC only after the immutable historical
NPC stages, preflights the exact registered source and all identity/binding drift,
and is additive/idempotent. The tree generator and validator retain the strict
1282-NPC/2747-binding baseline plus only the exact approved packet. Unknown,
duplicated, partial or altered additions are rejected. The published partial
NPC shard locator remains stable while its header gives the current extent.
Physical map admission must pass normally; no collision permission is assumed.
