# Hireling management v1

Capability250 requires existing Hireling capability165. It is bundle-world only.
Command470 lists owned identities and addresses one Hireling for return or appearance.
The native lamp remains an ordinary item-use command. Existing command220 keeps its
one-byte query/summon/dismiss result and never mints an unpurchased Hireling.

## Request

All integers use network byte order. A Hireling identity is a UUIDv7-shaped16-byte value.

| Action | Canonical bytes |
|---|---|
| List | `00` |
| Return to lamp | `01` followed by Hireling identity |
| Set appearance | `02`, identity, look-type:u16, head/body/legs/feet:u8 each |
| Current actor projection | `03` |

The maximum request is23 bytes. Unknown actions, trailing bytes, zero look type and
malformed identity are rejected before dispatch. The server resolves character, house,
session generation, lease, scope assignment and node incarnation independently.
An identity selector never grants ownership. Appearance must be in the owner's
character-wide entitlement list for the Hireling's sex; Citizen is always selectable.

## Reply

`01` is a committed change; `02 || reason:u8` is a semantic refusal. Reasons1 through8
are stale authority, not owner, not active, outfit not owned, Inbox full, capacity,
not in house and command conflict. The command itself is accepted when its payload is
canonical and its semantic result can be represented.

`00 || count:u8` precedes zero to ten entries. Each entry contains identity:16bytes,
name-byte-count:u8, ASCII name:3..18bytes, sex:u8 (1 male,2 female), look-type:u16,
four color bytes, skill-bits:u8, outfit-bits:u16, placed:u8. A placed entry adds
x:u16,y:u16,native-floor:i8 (-15..0). Skill bits0..3 mean Banker,Cook,Steward,Trader;
outfit bits0..8 mean Banker,Cook,Steward,Trader,Servant,Hydra,Ferumbras,Bonelord,Dragon.
Unused bits are zero. A complete reply is at most512 bytes. Partial entries are invalid.

`03 || count:u8` is the current House actor projection, with zero to ten entries on the
observer's current floor. Each entry is Hireling identity:16 bytes, EntityRef byte length:u8,
then the existing WorldSpatialEntities `EntityRef` protobuf encoding (field1 identity:16 bytes,
field2 generation:varint). Generation must be nonzero. Duplicate Hireling identities or
actor identities, partial entries and trailing bytes are invalid. The maximum projection is
462 bytes, within the unchanged512-byte reply bound. A client joins these bindings to List
entries by durable Hireling identity to display names. The spatial NPC uses the existing
appearance dictionary for look type and four colors.

Projection is a read of current runtime House custody after a current durable refresh.
Its actor identities are ephemeral and never enter a retained mutation receipt. A different
House, floor, session or stale actor generation cannot reuse the binding.

## Durability and replay

Every mutation uses the existing Hireling owner and native inventory/Inbox owners in
one transaction. A command receipt binds its immutable request and response. Replaying
an old command never projects its old placement: runtime projection is refreshed from
current durable state under current scope authority. A stale session cannot acquire
authority from a receipt. A refused return never clears placement or creates a lamp.

Crystal reference:201189f data/libs/systems/hireling.lua:205-269,333-382,465-497.
The character-wide entitlement and legacy Banker conversion decisions are PR#2783
ANSWER6547428201. Store production producer activation is owned separately by Store.
