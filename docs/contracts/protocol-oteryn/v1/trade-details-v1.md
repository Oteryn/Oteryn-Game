# Trade details v1

Capability 501 `TRADE_DETAILS_V1` extends the already-selected trade owners and
grants no command or domain authority of its own. With NPC service capability 3, it adds
field 4 (repeated message) to domain 8, `NpcTradeWindowV1`. Elements align
one-to-one with field 3 offers; an empty vector retains legacy presentation.
Each element carries field 1 (UTF-8 item name, 1–128 bytes, no NUL) and
field 2 (u32 maximum transaction quantity, zero means unavailable).
Unknown or duplicate fields, excess elements and mismatched lengths fail closed.
Legacy peers receive no field 4 and reject it if received without selection.
Names and bounds come from the server's pinned Item catalogue and trade rules.

When player trade capability 60 is also selected, domain 26 adds fields 11
(own contents) and 12 (counter contents). Each repeated node has field 1
(OfferV1), field 2 (parent index plus one, zero at the root), and field 3
(item name, 1–128 UTF-8 bytes). Up to 100 nodes per side, root included;
parents precede children. Index zero is the root. Counter contents remain
undisclosed until this player has offered too. Both peers must select 501
before either can offer a nonempty container. Descendants remain frozen
with their root; changed custody, attributes or contents invalidate the offer.
Existing inspection command 153 (`Request::Trade`) inspects these exact indices
under the current session-generation, pair and immutable-offer checks.

The immutable event carries the original root snapshots plus a tree node count
and SHA-256 digest per nonempty tree. Complete descendant snapshots remain in the
frozen owner and immutable SQL receipts. Existing native event byte ceilings stay
unchanged, as required by the accepted BAGS-0 two-tree shape.

NPC quantity requests above 100 require selected capability 501 on command 8.
The transaction ceiling is 10,000 stackable units. Individual item and coin
stacks retain their pinned definition's bound (currently at most 100). Purchases
split into bounded fresh stacks; ordinary sales may burn several eligible stacks,
with only their final burn partial. Extra purchase identities derive from the
existing item-output seed and fixed-width u32 indices with SHA-256 domain
`oteryn:npc-bulk-item:v1`; replay retains the original transaction and audit.

Each displayed maximum is limited by the owner's direct-backpack trade shape
(20 stacks), the offer count, and that side's price ceiling. Native hireling
shops retain their single-stack shape and 5,400,000-gold sale ceiling. Nonstackable offers
remain one item. Current inventory, capacity, gold and session fences are checked
again at commit. The multi-stack form allows up to 42 ledger lines, 42 touched
items/custody lines/participants and 106 work units on BUY; SELL uses the existing
23-line/66-work-unit aggregate bounds. Legacy single-stack price and audit-check entry points retain their original
shape. Native event encoding and decoding qualify the bounded multi-stack form;
all existing envelope byte ceilings remain unchanged.

Capability 590 `PLAYER_TRADE_ARTWORK_V1` requires 4 and 60. It adds field 4
to the offer projections in domain 26, including each disclosed contents node
when 501 is also selected. The server derives this nonzero u16 object
appearance ID from the frozen leaf's activated Item definition and revision.
The binding applies to the client artifact of the admitted generation and
uses the offer's existing subtype. An unqualified binding is omitted.
Legacy peers receive the original offers. Decoders reject field 4 without
selection, duplicate values and IDs outside 1–65535. The trade command,
counter-offer disclosure and state bounds retain their original semantics.
[Crystal protocolgame.cpp, 201189f8](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/server/network/protocol/protocolgame.cpp#L7558-L7594)

Capability 589 `NPC_SHOP_OPTIONS_V1` requires 3 and 4. Command 8 field 7
requests a packed BUY. Command 97 field 7 requests inclusion of equipped
items on SELL_ALL and also requires 81. Both tags are optional canonical
varints of value 1. Omit the tag for a loose purchase or to exclude
equipped items. Wrong actions, unsupported tags, duplicate tags and other
values are refused. Legacy command results and inventory encoding still apply.

Packed BUY uses one server-resolved 20-slot Shopping Bag (21411) and charges
20 gold on top of the quoted total in the same fenced transaction. The
shop purchase still observes the existing maximum of 20 item stacks.
Purchased stacks go into the new bag. Its slot, weight and carried-tree
space are checked together with the goods and payment. SELL_ALL with
equipment enabled includes matching eligible items from equipment slots as
well as the existing direct-backpack sale scope. It leaves the main backpack,
nondefault equipment and containers with contents outside the sale. Existing
sell-all amount and price ceilings apply to the combined sale. Hireling
catalogue trades retain their separate custody shapes and refuse these options.
The receipt binds the option, and replay returns the original committed result.

[Crystal NPC purchase and bag fee](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/lua/functions/creatures/npc/npc_functions.cpp#L593-L666)
[Crystal equipped-item option](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/server/network/protocol/protocolgame.cpp#L2250-L2266)
[Shopping Bag, Tibia Fandom archive](https://tibia.fandom.com/wiki/Shopping_Bag)
