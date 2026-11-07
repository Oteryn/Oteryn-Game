# Atelier resources V1

`ATELIER_RESOURCES_V1` is capability 272 and requires capability 148
`WHEEL_ALLOCATION_V1`. It adds read-only command 484
`ATELIER_RESOURCES_QUERY`, offered through
`BUNDLE_WORLD_OFFERED_CAPABILITIES` only. The fallback production and Item-view
offer sets are unchanged. This surface implements the seven resource balances
sent when Crystal `201189f` opens the Wheel panel.

The user-authorized P07 task supplies the `GEM-ATELIER-P07` change scope and
reserved numbers. This contract extends the existing Atelier owners. It creates
no second inventory, bank, stash or gem balance store.

## Request and reply

The request is empty: its maximum payload size is 0 bytes. It carries no
occurrence, expected revision, item selector, price or resource count. A current
admitted Character session and selected capability 272 are required.

The reply is `atelier::resources::ResourceBalances`. Every field is a required
unsigned 64-bit varint, including zero. Fields appear exactly once in ascending
tag order. Unknown fields, duplicate or missing fields, incorrect wire types,
non-minimal varints, reordered fields and truncated encodings are malformed.

| Tag | Field | Meaning |
| --- | --- | --- |
| 1 | `bank_gold` | Current gold in the admitted Character's Account/World bank balance. |
| 2 | `carried_gold` | Gold value of carried gold, platinum and crystal coins, including coins in the Character's native Gold Pouch. |
| 3 | `lesser_gems` | Carried unrevealed Lesser gems for the Character's vocation. |
| 4 | `regular_gems` | Carried unrevealed Regular gems for the Character's vocation. |
| 5 | `greater_gems` | Carried unrevealed Greater gems for the Character's vocation. |
| 6 | `basic_fragments` | Carried Basic fragments plus that Character/World's Supply Stash quantity. |
| 7 | `supreme_fragments` | Carried Supreme fragments plus that Character/World's Supply Stash quantity. |

The exact maximum reply size is 77 bytes: seven one-byte field tags plus seven
ten-byte varints. The all-zero reply occupies 14 bytes. The command registry
declares these 0/77-byte bounds, and the codec tests exercise both the maximum and
its first rejected byte.

## Authority and resource scope

The existing durability owner validates the current Character gameplay fence,
locks the Character root and reads the complete projection from one SQL
statement snapshot. The query performs no item, bank, stash, Character revision,
Atelier receipt or ExAction mutation. A stale session, invalid resource graph or
unavailable authority cannot produce a usable balance projection.

Carried scope includes the owned main container, equipped roots and their
recursively carried containers, within the existing carried-inventory depth and
node bounds. The current content policy still decides which items can actually
be equipped. The gold total additionally includes coins in the special native
Gold Pouch (item i23721), whose root is in the Character's Inbox and whose contents
follow the existing pouch owner. Ordinary Inbox items, including gems and
fragments, do not contribute. Foreign, disconnected, depot and other non-carried
items do not contribute.
Supply Stash contributes only the two fragment counts; stashed gems and money
are excluded. Positive quantities across retained definition revisions are
counted by the corresponding item kind, matching Crystal's item-type totals.
Fresh paid mutations independently qualify their current resource definitions,
select sources and enforce their existing payment rules.

The displayed gold total is broader than the current Atelier fee input set:
Gold Pouch money is displayed but is not debited by these paid actions. The
accepted native fee boundary explicitly excludes that source
(`docs/architecture/reviews/OTERYN_GAME_CHARACTER_GOLD_FEE_BOUNDARY_DECISION_2026-09-30.md:104-110`;
`docs/architecture/reviews/OTERYN_GAME_BANK_FEE0_FEES_FROM_THE_BANK_DECISION_2026-09-30.md:168-173`).
This projection does not change that boundary or promise that all displayed
gold is spendable by an Atelier mutation.

These values are a read-time display snapshot. They are never accepted as
payment authority or persisted in an immutable mutation receipt. No Atelier
revision is attached to them: the bank and inventory can change independently
of an Atelier mutation.

## Existing panel integration and compatibility

The existing 0148 Wheel/Atelier runtime issues command 484 after its ordinary
Wheel 161 and Atelier 308 live refreshes when capability 272 is selected. This also
happens after a mutation receipt replay, so the panel shows current balances
alongside the current live state. A session without 272 sends exactly the
previous command sequence: its requested operation followed by Wheel 161 and
Atelier 308. It sends no command 484 and has no resource projection to display.

All state, controls and text presentation stay in the existing 0148 panel. The
0272 feature contains only the registration and reply-decoding adapter required
by the client capability/command coverage checks. It supplies no separate panel,
control, setting or state owner. The native panel already displays 0148's
accessible `panel_text` output.

The panel shows carried and bank gold separately, all three carried unrevealed
gem counts, and the carried-plus-stash fragment totals. Pouch coins are included
in the existing carried-gold number; there is no separate pouch balance line or
client-side spendable-total calculation. A well-formed rejection of command 484
leaves the earlier operation result and Wheel/Atelier refreshes available, with
no resource projection. Missing or malformed optional replies clear previously
displayed balances and omit their lines; unavailable information is not
represented as zero. The ordinary 148 controls remain usable, and the server
remains responsible for affordability and source selection. Transport or Session
failures retain their existing error path. Historical mutation replies cannot
overwrite these independently refreshed balances.

Atelier V1 replies, commands 308/309/424, Wheel 161 and capability 148 retain their
existing encodings and semantics. In particular, no new field is appended to
the strict existing `AtelierReply` decoder. Compatibility tests pin its original
bytes and rejection of a balance-field extension.

The Fragment Workshop catalogue remains part of the ordinary Atelier 308 query,
as specified in [Gem Atelier V1](atelier-v1.md). Its full compatible grade rows,
including Grade I zeros, do not depend on selecting capability 272. This resource
reply supplies only the seven aggregate balances; it is not a modifier catalogue
or an affordability verdict. The existing 0148 modifier selector retains its
legacy sparse/empty-query fallback independently of these optional balances.

## Sources and qualification

Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
`src/creatures/players/wheel/player_wheel.cpp:1634-1643`, selects the seven
balances and includes Supply Stash only for fragments. The inventory slot range
ends at Store Inbox (`src/creatures/creatures_definitions.hpp:338-353`);
`src/creatures/players/player.cpp:1477-1503` counts its coins in carried money.
Item-type counts use `player.cpp:5190-5210`, and carried material traversal is
`player.cpp:5821-5844`. The native ordinary Inbox is a separate delivery surface;
only its special Gold Pouch owner contributes here. Donor payment semantics are
`src/game/game.cpp:2862-2969`; the native Gold Pouch debit difference is stated
above rather than inferred from a display balance.

Qualification includes strict codec boundaries and old V1 bytes, negotiated
runtime command ordering with and without 272, optional reply failure and
receipt replay, seven-field panel rendering, real dispatched read-only resource
queries, stale-fence refusal and unchanged durable footprints. The task's
published candidate evidence records the commands and actual results.
