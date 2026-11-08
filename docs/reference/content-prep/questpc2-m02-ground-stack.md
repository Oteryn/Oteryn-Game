# M02 exact mutable Ground prerequisites

Capability: a `WorldRequirement` can contain either an immovable native `appearance`, or a `ground_stack` list of `{item, quantity}` entries in top-to-bottom order. An empty list requires an empty mutable Ground cell. Full census length, individual quantities, definition identities and order must match; aggregate counts are insufficient.

The native physical-scope owner performs the actual reads in the quest receipt transaction. It verifies scope/recovery/node authority, current map/content/frame and Ground custody generation; its existing advisory scope lock prevents a concurrent insertion or removal from invalidating an absence/order predicate before the quest commits. No Item is consumed. Exact command replay returns the original quest receipt after the stack moves.

The predicate is bounded to eight cells and 64 roots per cell. It requires a player callback, a writable storage transition and World admission. Stateless, reversible, closing-only and unbound native-rule paths are refused. The expected layout is included in the existing callback binding hash, and the generic non-World quest writer cannot bypass it.

Nomads Land's chosen recipe explicitly requires arranging crates and an amphora before approaching the cabinet: `content/quests/definitions/quests-00300-00351.json`, identity `oteryn:quest.authored.nomads_land_quest`, stage s4. Its recorded source references are:

- https://tibia.fandom.com/index.php?curid=20026&oldid=842864
- https://tibia.fandom.com/index.php?curid=22842&oldid=1002600

The current Brazilian wiki API redirects `Nomads Land Quest` to `Arito's Task Quest` (https://www.tibiawiki.com.br/api.php?action=parse&page=Nomads_Land_Quest&prop=wikitext&format=json). Fetching the target returned HTTP 403; it supplies no additional verified positions or quantities. This engine slice therefore does not invent a production map coordinate, arrangement or complete-quest qualification. Canonical authored route placement remains the separately owned M01 boundary.

The PG case uses isolated engine fixture coordinates and Items, not invented Nomads Land content. It drives the production quest writer through `commit_fixture_world_callback`, with the normal physical-scope and Character fences retained.
