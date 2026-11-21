# Character Cyclopedia parity slice (batch 48)

Target: Tibia Global, `docs/reference/tibia-manual/interface.md:176`.
The Bevy Character window opens with Ctrl+U or its toolbar button.

- Familiars uses capability 199 / command 388 / selector 129, the existing
  self-appearance catalogue. Previous/Next traverse the shared appearance pages;
  only familiar entries are shown on the Familiars tab. These are catalogue
  entries, **not account unlocks**. Portraits resolve the server look type through
  the pinned appearance assets; missing local artwork is reported explicitly.
- Character profile uses selector 132, showing the same confirmed identity,
  equipment and earned badges together. The separate Badges tab retains the
  complete badge requirements view. Profile refusal does not imply empty data.

Reference implementation inspected: `edubart/otclient` commit
`dab86e610991092fe8b2471f7de55a7c986b3cd2`,
`modules/game_outfit/outfit.lua:70-87` (server appearance choices and preview).
That revision has no Cyclopedia module. It is a client-structure reference,
not authority for Global title/familiar semantics. No OTClient code or assets
are copied.

## NEEDS-CONTRACT

No new wire codes, server data, persistence or capabilities are invented here.
The remaining Global features need accepted messages for:

- `CharacterTitlesRequest` / `CharacterTitlesResult`: unlocked title IDs and
  display names, currently selected title (including none), plus
  `SelectCharacterTitle { title_id: Option<TitleId> }` and an authoritative
  selection result. Capability 199's current selectors provide no such operation.
- `CharacterProfileDetailsResult`: target identity plus display title, outfit,
  vocation, level, world and guild membership with server-authorized visibility.
  Selector 132 currently contains only character ID, name, badges and equipment;
  independent self-stat replies must not be represented as another player's data.
- `CharacterFamiliarAvailabilityResult` needs an authoritative per-character
  familiar status keyed by familiar definition if it is to be presented as unlocked/usable.
  The current `gallery::Kind::Familiar` explicitly represents the content catalogue.

Animated/directional gallery previews and richer profile artwork are not part
of this slice. Existing General Stats, combat, history, achievements and Store
pages remain available separately.
