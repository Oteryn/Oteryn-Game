# Current subarea query v1

SUBAREA-ADMIT-1 owns capability 93 (`SUBAREA_V1`) and command 109 (`SUBAREA_QUERY`).
The composed server offers it only for a bundle carrying typed admitted masks.
Both negotiation and the handler enforce capability selection. No client changes
or state domain are required by this server slice.

## Request

The payload is empty. The server reads the requesting character's current position
from its current controlled actor in its admitted World and Channel. The request
cannot select another character, arbitrary coordinates, World or Channel. A missing
actor, wrong scope, lost control, unsupported map or nonempty payload is rejected.

## Result

The accepted result has exactly nine bytes: `present:u8`, `region:u32le`,
`subarea:u32le`. `present=1` carries the positive client 15.30 `cipsoft/map_area`
parent/source ids of the admitted native position. `present=0` has both ids zero
and means that the pinned masks do not cover this position. Floors are exact.
Results use normal command ordering and retained-result replay. They write no
Character or World state, do not generate POIs, and do not authorize discovery,
donations, respawn boosts or echo raids. Existing capability 57/basic-area commands
keep their accepted rules. Consumers of map areas and later area-scoped content
share the pinned `WorldBase.subareas` resolver.
