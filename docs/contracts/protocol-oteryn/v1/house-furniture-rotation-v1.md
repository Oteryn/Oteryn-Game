# House furniture rotation v1

W18-HOUSE-ROTATION-1. Capability445 requires49; command730 multiplexes query and
explicit rotation. Mechanic703 and offer700 use the SEASON lane offset60 from
main9668e0a04. Ordinary USE retains container opening and native USE effects.

## Request

All integers are big endian. Query is tag0 followed by pageu16 (0..255).
Rotate is tag1 followed by a typed target: tag0, ItemInstanceUUID16 and expected
revisionu64(nonzero); or tag1, the existing40-byte WorldObjectTarget placement
and expected base revisionu64. The request maximum is50bytes. Trailing, unknown
and truncated fields are rejected. Clients send no House, actor, world or position.

## Result

Catalogue is tag0,pageu16,moreu8,countu8 then at most16 entries. Each entry
contains its typed target, UTF8-label lengthu8 and1..120bytes of label. Applied
is tag1 followed by the existing UseDisposition result. Maximum result4096bytes.
On success the client queries the current page again; it never guesses revisions.

Only current nearby placements of the admitted named House are listed. Dynamic
rotation reuses the existing House consume/transform receipt and audit; base
rotation reuses the House base-placement owner. Session, connection, lease,
node and House generation are rechecked before writes and replay. Dynamic item
revisions and static placement revisions must match; sources remain in custody.

Source rules are captured in house-decoration-actions.json: Crystal201189f
items.xml rotateto and Game::transformItem game.cpp3054-3057. A missing donor
target returns the original item, without an invented canonical identity,
revision bump or mutation audit. Explicit rotation is distinct from USE.
