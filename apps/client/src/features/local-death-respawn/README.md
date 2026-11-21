# Death acknowledgement

Zero-health own-vitals events retain a death notice, including when healthy
respawn vitals arrive before the next frame. Healthy vitals allow explicit
acknowledgement; acknowledgement does not send a login or respawn command.
The Bevy shell's existing notice keyboard handler supplies the minimal Enter
entry point. Button controls remain a separate notice-module batch.

Protection details come from capability 382 / command 664's immutable receipt.
Only a query issued after this death can supply its details; the previous
receipt is never presented as consumption for a new death.

References (structure only): edubart/otclient at
`dab86e610991092fe8b2471f7de55a7c986b3cd2`,
`modules/game_playerdeath/playerdeath.lua`, `openWindow`, inspected 2026-11-15:
retained death dialog and explicit acknowledgement. Oteryn keeps its accepted
in-session temple respawn instead of copying OTClient's login action.
Target behaviour: `docs/reference/tibia-manual/characters.md`, section 5.1.11
(CIPSOFT_OFFICIAL manual summary): death at zero HP, temple respawn, and
server-owned experience, skill and item penalties.

NEEDS-CONTRACT: a server `DeathOutcome` event keyed by the same occurrence as
`DeathProtection`, carrying exact experience/skill/item losses, death type and
unfair-fight penalty reduction. Own vitals and the protection receipt contain
none of these penalty facts. Capability 488's historical death rows contain
killer names/level but no occurrence key or losses, so cannot safely identify
this death. No new wire message is introduced by this batch.
