# Explicit world item Rotate V1

Capability **388 WORLD_ROTATE_V1** requires 11 WORLD_INTERACTION_V1 (transitively
15 ITEM_USE_V1, 18 WORLD_MAP_VIEW_V1, 4 ITEM_VIEW_MOVE_V1 and 6 WORLD_SPATIAL_ENTITIES). Command **670
ROTATE_WORLD_ITEM** is independent of USE. Crystal
`201189f871d23597b82caf88fd611454a2ddefb8 src/game/game.cpp:4521-4573`
separates Rotate, refuses unique items, checks range and invited House access,
and transforms to the exact `rotateTo` item.

## Request

Rotate-only field 4 is an optional scalar bool `probe` (default false). True is
allowed only for an Item handle and asks for read-only inventory affordance.
It never transforms, writes a receipt, reserves an item or grants authority.
The server uses the existing fenced owned-inventory read, verifies the observed
definition and returns one byte: 0 unavailable, 1 rotatable. A read error or
timeout returns unavailable; it does not interrupt gameplay. The actual Rotate
operation always rechecks current type, custody and actor/session authority.

The native inventory control binds a probe response to the selected handle,
observed definition, local inventory generation and current content-generation /
bundle-digest / reset-epoch binding. It discards stale responses and shows Rotate
only for a matching true response. Probes do not add ItemEntry fields or a new
handle namespace.

The existing `UseIntentV1` target encoding is reused: field 1 is the opaque
WorldObjectTargetV1, field 2 the opaque ItemTargetV1 handle. Exactly one is
required. Field 3 is the nonzero observed Item definition reference: required for
an Item handle and forbidden for a WorldObject (which already carries a CAS
revision). The server compares that definition under its source locks, after
receipt replay, so a concurrent transformation cannot rotate an unseen item.
The USE-WITH length-delimited arm, ItemByDefinition (field 5), duplicate and unknown
fields are refused. Maximum 529 bytes. No durable Item UUID or House ACL is
provided by the client; the server resolves the issued handle and current
House/session authority.

## Response

For a normal Rotate the response is the existing bounded `UseResultV1` (at most 4 bytes). Map and
House item revisions/sprites are delivered by the existing authoritative map
updates. The native context menu offers Rotate only after capability selection and
verified official `AppearanceFlags.rotate` (field 22, Canary pinned
`src/protobuf/appearances.proto:154`); missing/unknown appearances offer no Rotate. It
revalidates the exact live target, and displays the server result.

Older same-major peers do not select 388 and never send 670. No prior field,
command or USE meaning changes (FND-02 protocol compatibility contract §7/§9).
