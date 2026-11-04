# EMPTY_OUTFIT_V1

Capability 275 extends the `APPEARANCE_V1` binding look union with tag `3`, an empty
outfit carrying no additional fields. The server selects that look only when projecting
an invisible player to another observer. The invisible player's own observation retains
the player's effective outfit. A peer that does not select capability 275 receives the
pre-extension outfit projection and therefore never receives the unknown tag.

The capability requires `WORLD_SPATIAL_ENTITIES` (6) and `APPEARANCE_V1` (25). It adds
no command or state domain; domain 21 continues to carry appearance mappings.
