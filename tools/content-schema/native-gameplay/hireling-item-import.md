# Hireling native Item closure

`build_hireling_item_profiles.py` captures the exact Crystal revision
`201189f871d23597b82caf88fd611454a2ddefb8` through committed Git blobs. The generated
`hireling-item-profiles.json` records the Lua catalogue hash, XML/appearance and
loader hashes, literal source item lines, immutable identity bindings, and the
complete normalized closure. It contains 1,425 item identities: every one of the
1,675 literal trade rows, 23 packages and their ingredients, nine Cook foods,
the native lamp, coins, backpack, present and decoration kit.

The capture never grants Inventory admission from catalogue membership. Its
immovable lamp 29432 has the closed CharacterInbox destination; ordinary carried
facts still report an unknown stack class. The 18 wrappable furniture targets
remain Ground items. Crystal `src/game/game.cpp:2761-2767` delivers kit 23398 with
the original target as `unWrapId`; `wrapped_trade_items` records exactly this
source relation. The existing native furniture owner qualifies its execution.

ANSWER Q11A preserves literal sell rows 3657, 5526 and 25686. Their actual source
facts are not pickupable, so those rows cannot acquire carried-item authority
or a substituted identity. Unpriced rows also remain in the literal catalogue.

ANSWER Q13B retains the exact gold-pouch row 23721 from
`data-global/npc/hireling.lua:358` as a virtual offer with a typed refusal. The
Hireling cannot turn that row into a physical sale, substitute a backpack, or
change the existing shared GoldPouch policy. Crystal's separate general-loot
pouch custody/producer and bounded sweep (`src/creatures/npcs/npc.cpp:444-550,
590-595`) remain an explicit prerequisite. Ordinary Trader sales and the
existing per-definition SELL_ALL keep their native item custody and payout.

## Historical identity bindings

Ten closure identities were already admitted under Crystal source revision
`00ce02a57ca5a12e48f32a3476e37471167e4c3f`, while the other 1,415 use the existing
`ff7ede593c69d4c658b382c97443e8155926924a` identity rows. Their canonical keys,
`definition-r1` revisions, and original binding source revisions stay unchanged.
The additional producer qualifier checks the exact 201189 XML SHA256
`8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509`, actual source
membership, and equality with the complete digest-pinned admitted binding table.
It cannot construct or revise a binding. The runtime still applies its ordinary
exact admitted-binding comparison; no new runtime identity qualification path
is added.

## Five established weight policies

[ANSWER Q14A](https://github.com/Oteryn/Oteryn-Game/pull/2783#issuecomment-6548596838)
keeps the following five already established native weights. Values are
centi-ounces. The raw capture retains every Crystal value and source line.

| Item | Single active native weight | Crystal 201189 weight | Crystal source |
| --- | ---: | ---: | --- |
| empty potion flask 283 | 100 | 160 | `data/items/items.xml:503` |
| empty potion flask 284 | 135 | 180 | `data/items/items.xml:507` |
| empty potion flask 285 | 75 | 150 | `data/items/items.xml:511` |
| gold coin 3031 | 10 | 1 | `data/items/items.xml:6818` |
| gold pouch 23721 | 150 | 0 | `data/items/items.xml:47233`; absent XML weight uses `src/items/items.hpp:300` default 0 |

The flask policies come from the existing consumption import in
`tools/content-migration/register_spell_families.py`; the coin from the existing
native coin profile; the gold pouch from `quick_loot_item_profiles.py`. The closed
compatibility map asserts both old and donor values before preserving those
weight leaves. A different old weight, donor weight or source line aborts.
There is one active policy for all consumers.

### Virtual GoldPouch movable fact

[ANSWER Q15A](https://github.com/Oteryn/Oteryn-Game/pull/2783#issuecomment-6549542080)
adds exactly one further compatibility pair: item23721 keeps its existing native
`movable=KNOWN(true)` while the raw Crystal capture retains `KNOWN(false)` from
`data/items/items.xml:47233`. The pinned existing native record at
`ae4188a9c8d461acb152044814ea23075de02ed4`,
`content/items/definitions/spell-native-profiles.json:97034-97036`, supplies the
unchanged shared value. The importer asserts both boolean values and the source
line; another item or a drifting pair still fails. Capacity20, pickupability,
stack, charge and temporal facts already agree and gain no exception.

The single shared GoldPouch profile remains Inventory-only. The Hireling row is
still virtual with a typed refusal as Q13B requires. Every other known physical,
stack, container or charge conflict aborts registration; this does not create a
second item policy or a physical Hireling pouch sale.

Existing richer authoring and unrelated semantic groups remain intact. An
existing record keeps its stack, container and charge groups, including UNKNOWN
facts; the capture supplies these groups only when they are absent.
An identity without World-authored semantics gains only the pinned native facts;
absence of an older optional semantic group is not evidence for a fabricated
value. Existing specialized destinations, including the Inventory-only gold
pouch, stay with their existing owner.

## Reproduction and checks

Run the capture against the exact donor checkout, then the ordinary registration
pipeline. No generated profiles or pins are edited by hand.

```sh
python3 tools/content-schema/native-gameplay/build_hireling_item_profiles.py /path/to/crystal-201189f
python3 tools/content-migration/register_spell_families.py
python3 tools/content-schema/native-gameplay/build_hireling_item_profiles.py /path/to/crystal-201189f --check
python3 tools/content-schema/native-gameplay/test_spell_item_identities.py
python3 tools/content-migration/test_hireling_item_profiles.py
```

Follow the repository's ordinary content regeneration, native loot refresh,
world pin refresh/check and content integration gates after changing the inputs.
The native content test `hireling_lamp_inbox_destination_is_source_bound_and_not_portable`
rejects movable-lamp, carry-destination and foreign-definition substitutions.
The wire PostgreSQL scenario independently proves generic carried admission
stays unknown while native Inbox use succeeds on the genuine source lamp.

The final active Item-profile document uses the registration pipeline's existing
canonical compact JSON encoder. All 2,104 records and their values remain;
the manifest is pinned to the emitted bytes. This keeps the complete active
closure within the unchanged native 8 MiB input limit, which the producer test
checks. The independent donor capture retains its readable representation.
