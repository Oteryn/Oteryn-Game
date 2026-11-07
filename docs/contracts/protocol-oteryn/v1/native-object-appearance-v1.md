# Native object appearance

Capability386 `NATIVE_OBJECT_APPEARANCE_V1` requires4 and6. It allocates no command or state domain. Domain1 entity field15 is an optional varint native Item appearance identifier,1 through65535, on Corpse and GroundItem entries only. It supplements their existing generation-local `item_definition_ref`, quantity and canonical session handle. Actor entries, zero, values above65535, repeated metadata and metadata without selection fail closed. Without386 the old object encoding is unchanged.

The producer resolves a visible canonical corpse through the current activated ItemDefinitionIndex, then its qualified native Item policy and source external binding. Current server/client artifact pins must match that activation. The Item ordinal is never interpreted as an appearance identifier. Unknown or unqualified display metadata remains absent; it creates no Item custody, actor appearance condition or interaction authority.

The native client resolves the explicit resource using its existing hash-pinned asset manifest, AppearanceIndex and SpriteSheets. These are object draw cells below actors, independent of Map17 placements. A missing resource keeps the existing target marker and is not counted as native pixels. The extension reuses the world scene and has no separate UI panel.

Allocation was checked against fetched main `d34807f5bd234ee8318e57ba454ba39be0df113d`: main384 is mechanics-status; this branch385 is native-map-cues,386 is the next free capability. Main had neither spatial field14 nor15; this branch14 is native creature icons and15 is this extension. Historical actual384/385 captures are not rewritten or upgraded; current source captures must negotiate386 and carry the real field15 before claiming this path rendered.
