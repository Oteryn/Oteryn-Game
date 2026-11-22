# Imbuement material details v1

CLIENTGAP-SYSTEMS L159. Capability 447 requires 68 (`IMBUE_V1`); command 731
is read-only and is offered on bundle Worlds with native gameplay. The existing
capability-68 codecs and mutation commands remain byte-identical.

## Request

At most 11 bytes: field 1 is the nonzero item-view handle (unsigned varint).
Unknown, duplicate or trailing fields are rejected. The route requires selected
capability 447 and invokes the existing capability-68 Query against the same
admitted session, item view, carried item, generation and content gates.
No client recipe, cost, material count or success outcome is accepted.

## Reply

At most 32,768 bytes. Field 1 echoes the queried item handle; repeated field 2
contains at most 78 quotes, strictly ordered by `(family, tier)`.
Each quote has varint fields 1 family (1–26; families above 20 require capability 470), 2 tier (1–3), 3 gold (u32),
4 Premium required (0/1), 5 success percent (100), 6 recipe available (0/1),
and up to three repeated field-7 materials. Each material has field 1 UTF-8
canonical item key (1–128 bytes) and field 2 required quantity (positive u32).
Material keys are unique within one quote. A missing admitted recipe is explicit:
`recipe_available=0` and no materials; it never means a free application.

Gold and Premium requirements come from the admitted legacy window. Materials
come from the exact canonical recipe selected by its family and tier. Success is
always 100% under IMBUE_FORGE0 §3 ruling R2; no protection fee is introduced.
The client accepts a reply only for its pending item and matches a selected offer
by family and tier. Changing items or refreshing the window discards old details.
The quote is advisory presentation; existing apply/clear owners independently
recheck all requirements and perform the fenced durable mutation.

## Affordability

Capability 582 requires 447. Both commands below also require selected 68.
Command 6328 uses the same item query as 731. The response prefixes the
complete 731 reply with `count:u8` and one 0/1 affordability byte per quote
in that reply’s order. The maximum is 32847 bytes. The count must match the
number of quotes.
The server observes live direct-backpack coin stacks and, when the Type 2 fee
activation and payer eligibility allow it, the payer's bank balance in the
current World under the item session fence. The server sets a quote’s affordability byte to 1 when the recipe exists
and the observed funds cover its gold price. The result concerns funds only.
Premium, materials, shrine reach, coin change capacity and concurrent spending
are checked independently on application. Command 731 remains byte-identical.

## Protection purchase

Command 6327 accepts the existing command-85 Apply request shape. A valid
negotiated request returns `Rejected` with the UTF-8 payload
`Sorry, not possible.`. Invalid or unnegotiated requests are rejected with
empty payload. The server does not burn gold, consume materials or apply an
imbuement. Crystal's current apply request has no protection flag and Oteryn's
accepted R2 ruling makes all applications succeed. The command-85 format and
semantics remain unchanged.
