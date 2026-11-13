# Tools local-first extraction:82 records, qualified GUI remainder

The running Lab client's static package was read through the existing serialized
QEMU guest-agent file/command channel. This was not the offline Ubuntu laptop
device, process-memory inspection or a packet/protocol experiment. The actual
chain is Molehill-PC → Docker `ubuntu-kasm` → QEMU guest `ubuntu-lab`;16906
is the separate8445 Lab desktop relay. No UI input occurred in this batch.

Live `package.json` reports **15.33.dd0955**. The executable SHA-256 is
`7f6555b19514abc34220a09f427732ea66ddfcd8d7052c733f0557d624baaf34`;
the5,104,343-byte appearances file SHA-256 is
`aa086346daa9234d721e4170848095ad83f3ee9d28ea9e62c1bd06216016174e`.
Both match the previously retained local sources. Original executables,
appearance assets, screenshots and raw serialized descriptors stay private.

## Material result

`tools82.json` contains every appearance whose encoded market category is14:

| Factual local field | Coverage | Remaining boundary |
|---|---:|---|
| Name / appearance ID / record hash |82/82| Appearance ID is not automatically a canonical/server Item ID |
| Market category, trade-as and show-as references |82/82| Not a fabricated UI tradeability/default flag |
| Sprite references |82/82| Existing Atlas structural decode, not pixel/animation parity |
| Paired NPC records |457 rows across43Tools| Preserve each record's name/location/prices/currency; absent NPC record does not prove no vendor exists |
| Description text |18/82| Remaining64 absent in this appearance record; not proof the UI has no description |
| Clothes-slot flag |7/82| Remaining75 not defaulted to an invented body position or nonapplicability |
| Weight value |0/82| Not in the compiled appearance schema or these records; remains a real UI/source gap |

NPC tuples are decoded from each individual field40 submessage, not reconstructed
by zipping independently flattened arrays. Numeric zero prices remain explicit
source values. Omitted currency data remain omitted. No canonical Item fields,
runtime stats, bindings or registry authority are changed.

## Five formerly unknown tags resolved from the static executable

The client contains a serialized `appearances.proto` descriptor candidate at
ELF byte offset45,621,408. A bounded wire scan consumed5,319 bytes and read the
field definitions recorded in `compiled-field-map.json`:

| Flag | Compiled name/type | Qualification |
|---:|---|---|
|60|`imbueable`, message with `slot_count:uint32`| Not Weight |
|61|`proficiency`, message| Existing mapping corroborated |
|62|`restrict_to_vocation`, enum `VOCATION`| Preserve numeric enum until shared enum definition qualified |
|63|`minimum_level:uint32`| Not Weight |
|64|`weapon_type`, enum `WEAPON_TYPE`| Preserve numeric enum until shared enum definition qualified |

Flag57 is named `deco_item_kit` in this executable. Its market message has only
category/trade-as/show-as fields1/2/3; restrictions instead occur at62/63 in
AppearanceFlags. Compiled appearance name/description are protobuf `string`,
where the inspected public Canary schema declared `bytes`. This is a schema
version difference; do not silently normalize malformed/unqualified text.

**No Tools record contains flags60–64.** The old unknown-tag uncertainty cannot
hide Weight in these82 records. This resolves field identity only; it does not
prove that no other local source or live server metadata can provide Weight.

## Cache and static-source boundary

`cache-check.json` retains four hashed package-cache JSON observations:
boosted creature, event schedule, online numbers and compendium news. Their
recursive key inventory has no item/weight fields or baking-tray text. The
two inspected Qt shader-cache trees contain shader/pipeline files. Account,
login/configuration, credentials and browser-cache content were excluded.
Character-data filenames were inventoried without reading their contents.
This is a bounded negative result, not an assertion about every file on disk.

The existing1853 collection manifest has89 images and one clipboard text
capture. Its notes describe mostly Potions, rat claws and a30oz compass. It
does not qualify82 Tools weights. Copy support on that historical compass does
not substitute for a current Tools copy test.

## Identity control and true remaining work

Appearance3464 baking tray has category14, Chephan/Venore City and
Maro/Rathleton City price20 source rows, plus sprite204373. Its record has only
flags18/36/40/44; Weight12oz is not among these fields. The parent reports that
UI value, but this worker has not yet independently verified/copy-read it.
IDs8020,8198 and31913 are separate baking-tray variants and must not merge by name.

`gui-queue.json` retains82 Weight gaps plus conditional checks for descriptions
and body position where local data are absent. Before bulk UI collection,
test copy/accessibility on the actual selected Tools detail field and preserve
the raw text/source association. If not copyable, capture only material missing
fields. Desktop-reader coordination and the current session must be respected.
This PR completes the **local metadata slice**, not the complete Tools UI audit.

## Reproduction and validation

`extract.py` takes private appearance bytes plus the existing private census,
verifies their source/record identity, and emits only derived Tools metadata.
`validate.py --private-root <original workspace>` reproduces all82 rows and
rehashes source receipts. Without private inputs it checks the public packet
and negative controls only. No proprietary raw assets are required in Git.

Allocation: [owner coordinator comment](https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6860837086).
PR3173 remains separately owned; this disjoint packet references its head
`657e3bf49ad1eadaf5390a7d97e1dd125c317d97` without modifying it.
