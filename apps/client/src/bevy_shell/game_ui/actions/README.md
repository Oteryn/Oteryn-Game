# Action/cooldown presentation

The cooldown strip uses `PlayLink::cooldown_bar_icons`, independently of action-row
visibility. It retains observed/catalogued groups, renders a shrinking white line,
uses inactive group artwork at expiry, removes expired individual spells, and
clears session icons on disconnect. Options' existing cooldown-bar checkbox hides
it. Local action activation never manufactures a cooldown.

Reference: Tibia manual `interface.md` §3.8; edubart/otclient at
`dab86e610991092fe8b2471f7de55a7c986b3cd2`,
`modules/game_cooldown/{cooldown.lua,cooldown.otui}`. The four classic group
rectangles use the operator-supplied `--ui-assets` sheet
`images/game/spells/cooldowns.png` (20×20 cells, inactive row 0, active row 1).
No reference artwork is copied into this change. Other groups and spells keep
complete server/catalogue labels rather than inventing an image mapping.

Remaining work / NEEDS-CONTRACT: `SpellCatalogEntry` provides cast indices, names,
words and recovery groups but no verified icon binding. Missing message: `SpellIconBindingV1 { cast_index, asset_id, source_rect }`.
Object artwork needs `ItemAppearanceBindingV1 { definition, subtype, appearance_id }`
binding definition + subtype to a verified appearance;
a definition index is not a sprite id. Action-slot spell/item icons remain pending
those bindings. Docking, context menus, Multi-Action editing and presets are later
batches.

Transport limitation: accepted `ActorCooldownsV1` is domain 20 of capability 23.
The capability remains unoffered and the candidate cooldown publisher has no
production caller at this base. This change does not enable the combined
presentation capability or claim to close that separate server transport batch.
