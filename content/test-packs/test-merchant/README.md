# Testserver Assistant

Set `OTERYN_TEST_MERCHANT=1` in the **game-server process** environment before
starting a login_local or node_boot test stack. Unset it for ordinary Worlds.
Use `LOGIN_LOCAL_WORLD_BUNDLE=1` for login_local; node_boot's default World pass
already uses a bundle. The tiny fixture pass has no starting temple or NPC boot.
The flag defaults off; the pack's NPC, presentation and services are activated
only by the boot owner's explicit flag. It also works when a test stack has no
`--npc-data-project`; an explicitly supplied catalogue still passes its existing
pinned checks. Its placement is added at boot, one tile east of the World pin's
default start (currently 32535, 32513, -7). The production
World Bundle and content indices contain none of these records.

Say `hi`, then `trade` for the category list, then an exact category keyword to
open that service. Every offer sells one item for one gold coin through existing
command 8. Categories use Item taxonomy primary types, sorted by Item key;
categories above 1024 items are split into numbered parts. No protocol limits
or Item admission rules are changed. Native Item restrictions still apply.

Name and outfit evidence (OTS_HYPOTHESIS_ONLY): Crystal
`201189f871d23597b82caf88fd611454a2ddefb8`,
`data-global/npc/testserver_assistant.lua:1,10,18-25`: Testserver Assistant,
lookType 396, head/body/legs/feet/addons 0.

Regenerate with `python3 tools/content-migration/test_merchant.py`; verify with
`python3 tools/content-migration/test_test_merchant.py` and the generator's
`--check`. `merchant.json` records the raw and included counts per source.
Items explicitly marked non-pickupable or immovable are excluded. UNKNOWN
physical fields do not remove an item proven obtainable by one of the sources;
the existing native Item owner remains responsible for mint admission.
See [inventory.md](inventory.md) for source counts and every category's offer count.
