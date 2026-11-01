# Hireling House position and lifecycle extension

Decision: PR2783 `ANSWER:` comment 6549584730, Q16=A. This is the narrow
LastPosition/admission/recovery extension needed by the accepted native House
movement and gourmet-vitals path. Crystal 201189f stores a player's actual logout
cell (`src/creatures/players/player.cpp:11612–11628`), and saves/restores those
coordinates (`src/io/functions/iologindata_save_player.cpp:240–243` and the pinned
load counterpart at196–198). Existing Oteryn owners remain the authorities below.

## One projection, one ordering rule

`game_character_last_positions` remains one row per Character, outside
CharacterRevision, without an additional event/cause/receipt ledger. Its original
World guard, no-delete/no-truncate rule and strict
`(character_lease_generation, write_sequence)` ordering remain in force.

The original position/vitals/rest fields now hold the actual named-House cell
when `house_key` is present. `origin_channel_id` and the three `origin_tile_*`
columns retain the original Channel routing cell separately. All five metadata
columns are null for a Channel row and present together for a House row. The
same ordered upsert atomically writes position, vitals, rest and scope metadata.
An older capture cannot overwrite any part of a newer capture. Pending respawn
or pending arrival continues to outrank a saved position.

Periodic writes retain the existing 300-second interval, 240-second due point,
60-second write-pass slack, 64-entry batch and 500-slot bound. The existing
LastPositions tracker and sequence generator follow the native scope handoff;
there is no House timer or per-step database checkpoint. An unchanged capture
writes nothing. Final logout/terminal release captures and writes inside its
existing terminal transaction. Generic Channel writers keep their existing
scope checks. Named-House writes use independently current session, Character
lease, scope generation, node assignment, original Channel and current named custody. A revoked ACL cannot trap terminal
release; service/input/placement reads still require their original locked ACL.

## Actual House authority and placement

House capture reads the existing native HouseActorState and the same
PlayerSpellState that owns health, mana and gourmet conditions. A pending native
lamp/food/spell commit is resolved before final release, and a pending state is
never published as an older checkpoint. A generic activity Instance is not a
House and cannot use this branch.

A stored position is history. Reconstructing or resuming an actor separately
proves the authenticated current session and controller, Character lease,
assignment generation/node, current named-House ACL, content pin, exact native
House cell and current item/player/Hireling collision under the original House
item and property locks. Fresh placement fails closed if its cell is unavailable;
there is no invented teleport fallback. A retained native actor may keep a
Hireling on its own cell because lamp summoning permits that cooccupancy; this
exception does not grant a new actor placement. Generic Channel actor access
remains closed for a House resident.

Terminal release also closes a still-held native entry reservation in its same
transaction, so relog before any STEP can reuse the cell. Known stale ownership
leaves cleanup to the existing stale-scope owner and never retains Account claims.

Explicit House logout retains the existing OFFLINE0 training marker and build
receipt in that same terminal transaction. A borrowed, current named-House
custody proof admits only the existing `Logout` cause, unchanged build families
and no stance pruning. The original expected CharacterRevision, progression,
offline pool/selection, DB-clock stamp and receipt-chain checks still apply.
This grants no other House progression or training write. Known lost House
custody skips the optional marker before any effect so claims can still release;
revision conflicts and unavailable storage remain unresolved. Channel and
activity participants retain their original gameplay fence.

Internal House entry/exit sessions do not reset online training time. Logout
reads the same immutable committed transfer ancestry used by lifecycle recovery
until it reaches the authentic FreshCommit origin, and requires that source's
real native `offline_settlement` marker and DB-stamped login clock. It never
copies or synthesizes a marker. Missing ancestry or a missing source settlement
adds no online credit; malformed or unavailable storage stays unresolved.
Ordinary sessions outside that House lineage retain their original clock rule.

The first committed native House STEP releases the original entry reservation
through the existing handoff owner. Connection generation 1 is accepted only
with the exact committed native entry receipt and the original
`house-transfer-input-v1` lifecycle proof. The receipt/acknowledgement stays
immutable. Exit accepts a still-held or already-released entry reservation.

## Admission and recovery provenance

A fresh relog authenticates and commits the ordinary fresh grant first. Before
Channel first-entry participants can overwrite a saved House cell, a closed
`FreshAdmissionRestore` branch composes the existing House entry handoff. The
existing transfer receipt stores `admission_restore=true`, the exact saved
LastPosition order key and its own new immutable entry acknowledgement.
`command_id=0` denotes this server admission source only; it is never a fabricated
client command. A trigger proves the exact genuine fresh receipt, original
transport, Character/World/Channel/lease and saved predecessor in addition to
all original native transfer guards. The original bed-regeneration owner retains House metadata when it settles
vitals/rest at the fresh lease. Such an equal-lease predecessor is accepted only
with that session's exact native bed-regeneration receipt. Ordinary client transfers still require a
nonzero command and null restore metadata.

The existing committed fresh slot joins House custody with its real source
lease and new destination lease. Its saved origin is retained routing history,
not a playable Channel position. The server's accepted session is the actual
House destination. Snapshot51 may carry that new immutable admission result;
current spatial state is projected separately and does not rewrite the entry
acknowledgement.

A known restore refusal is reconciled under the original admission relation
locks. Only the unchanged ACTIVE fresh source/controller/lease is returned as a
refused-first-entry AdmittedSession, retaining its native disconnect/release path.
A terminal source or unknown read is never called a rollback and stays with the
existing destination receipt/recovery owner; no fallback cell is fabricated.

Lost-ack recovery restores current LastPosition coordinates and separate origin
metadata. The immutable transfer acknowledgement is kept as admission history.
Legacy entry-only records without House metadata retain their original
sequence-1 interpretation. A session whose `house-transfer-input-v1` proof shows
that its destination command namespace started must use ordinary retained
command continuity; process loss never resets it to command1. A new actor after
terminal logout uses a new authenticated fresh grant and the closed restore
path. Missing continuity, changed authority or unavailable placement fails
closed.

## Validation ownership

The Q16 PostgreSQL scenarios exercise production House STEP, periodic/final
capture, ordered stale-write refusal, real logout/control loss/resume and a fresh
actor after relog, including revoked ACL or unavailable cell. Native carrier
checks cover actor generation, session/lease, compare-write, transition fences
and scoped occupancy. Evidence belongs to the final compiled/tested checkpoint;
this contract does not claim an unrun test passed.
