# Team Finder members v1

Capability 232 (`TEAM_FINDER_MEMBERS_V1`) adds S14b browse/request/cancel and
S14c applicant decisions to the existing leader capability 200 and commands
390/391. Only bundle-world nodes offer it. It requires capabilities 7 (private
status feedback), 42 (the existing Party owner) and 200 (leader authoring).
Party's dependency on capability 6 remains unchanged.

The native Team Finder window is not implemented. Shared protocol encoders and
decoders, negotiated server routes and client registration metadata are present;
the client feature remains explicitly unsupported until a native window consumes
them. This capability does not advertise native UI support.

## Sources and decisions

Primary compatibility source: Crystal
`201189f871d23597b82caf88fd611454a2ddefb8`, evidence class `OTS_HYPOTHESIS_ONLY`.

| Element | Source | Applied rule |
|---|---|---|
| Browse fields and occupancy | `src/server/network/protocol/protocolgame.cpp:2823-2890` | Leader identity/name, levels, vocation mask, slots, occupancy, timestamp, target and viewer status |
| Applicant fields | `src/server/network/protocol/protocolgame.cpp:2893-2970` | Online members' names, levels, client vocations and statuses; leader has status 3 |
| Existing Party import | `src/server/network/protocol/protocolgame.cpp:3019-3029` | Party flag imports existing members as accepted |
| Invite/accept/deny | `src/server/network/protocol/protocolgame.cpp:3075-3112` | Existing applicant status 2/3/4, with private status feedback |
| Request/cancel | `src/server/network/protocol/protocolgame.cpp:3125-3157` | Insert pending status 1 or remove the applicant entry |
| Remove/logout | `src/game/game.cpp:680-682,1422-1426` | Terminate active advertisement and requests |

[Owner answers Q1-Q5](https://github.com/Oteryn/Oteryn-Game/pull/2767#issuecomment-6542716908)
settle donor defects and the existing S14a contract:

- Q1: use the selected leader's advertisement. The donor reads `leaderID` but
  looks up the applicant's object at `protocolgame.cpp:3140`; this defect is not
  reproduced.
- Q2: leader acceptance issues an ordinary Party invitation through the existing
  Party owner. The applicant joins only through the existing Party Accept command.
  Team Finder becomes accepted when the leader decides; it is not itself party
  membership. Normal Party visibility, leadership and membership refusals apply.
  Status 2 is a Team Finder invitation, not an ordinary Party invitation.
- Q3: keep the durable leader template across relog, but close the active
  advertisement and its requests at logout. A new command 390 is required to
  advertise after relog. Command 391 removes the template and active state.
- Q4: one pending request per player in the World/Channel scope. Only status 1
  holds this constraint; status 2, 3, 4 or cancellation releases it.
- Q5: keep `LeaderListing.free_slots` as the leader-supplied baseline. Global
  occupancy is `max(slots - baseline_free_slots, accepted_count)`. Effective free
  capacity is `slots.saturating_sub(occupied_slots)`. The leader contributes only
  through the baseline. A transition to accepted status from another status is
  refused when no effective capacity remains. An already accepted entry remains
  accepted without issuing another ordinary Party invitation.

Browse has no donor server-side filter payload. Minimum/maximum level, vocation
mask and target are listing fields. The server returns active advertisements in
the caller's World/Channel scope; no additional eligibility formula is invented.
Offline applicants are absent from the applicant view. Cancelling an accepted
Team Finder entry does not bypass the existing Party Leave command.

## Transport and identity bounds

All scalar fields use protobuf wire type 0, strings/submessages use wire type 2.
Unknown fields, duplicate singular fields, missing required fields and invalid
enum values are refused. Requests are at most 48 bytes, using the existing
`party::MAX_PARTY_INTENT_BYTES` budget. Replies use the existing Guild transport
budgets, `guild::MAX_GUILD_PAGE_ENTRIES` (100 entries) and
`guild::MAX_GUILD_RESULT_BYTES` (65,536 bytes). These are page/resource bounds,
not limits on the number of advertisements or applicants in gameplay. Names use
the existing `party::MAX_PARTY_NAME_BYTES` bound (64 UTF-8 bytes) and are nonempty.

Listing and member handles are nonzero connection-scoped `u64` values issued by
the server. Cursors are opaque handles too. No durable CharacterId, AccountId,
WorldId, ChannelId or session proof is accepted from the client. Each action
resolves the right kind of handle and revalidates the target's live generation
and scope. Handles from another connection, an older target generation or an
advertisement that is no longer active are refused. Removing and recreating an
advertisement in the same leader session retains that leader's identity; it does
not create a separate listing epoch.

Pages return `next` only when another page exists. It is the last entry's handle;
the client sends it unchanged as `after`. Zero, unknown, wrong-kind or old
connection-generation cursors are refused. A new page replaces that kind's
previous handle page; actions on older pages require refreshing the page first.
Entries in a page have distinct handles. A keyset cursor can still continue past
a leader who removed their listing after the previous page was read. A client
refreshes from the first page after its advertisement context changes.

## Query

Command 444 (`TEAM_FINDER_QUERY`).

| Field | Meaning | Required |
|---|---|---|
| 1 | kind: 1 browse, 2 own applicants | yes |
| 2 | `after`: nonzero continuation handle from the preceding page | no; absent starts the first page |

The applicant query is always for the admitted actor's own advertisement. It
accepts no leader selector that could expose another leader's applicant list.

## Member action

Command 445 (`TEAM_FINDER_MEMBER_ACTION`).

| Field | Meaning | Required |
|---|---|---|
| 1 | action: 1 request, 2 cancel | yes |
| 2 | nonzero leader handle from Browse | yes |

The admitted actor is the applicant. A request inserts status 1 without replacing
an existing state. Cancellation removes only that actor's entry in the selected
advertisement. A second pending request in the same World/Channel scope is refused.

## Leader action

Command 446 (`TEAM_FINDER_LEADER_ACTION`).

| Field | Meaning | Required |
|---|---|---|
| 1 | decision: 2 invite, 3 accept, 4 deny | yes |
| 2 | nonzero member handle from the leader's Applicants view | yes |

The admitted actor must own the active advertisement. The target must be an
existing online applicant. There is no client-controlled leader identity, status
outside 2/3/4, or second Party implementation.

## Action reply

Commands 445 and 446 return an empty payload and the normal Accepted/Rejected
command status, following the existing leader owner. After a decision the client
queries Browse or Applicants to refresh its view. This is a native-protocol
choice in place of the donor's immediate window packet. Private donor status
messages are emitted only after a successful fresh commit, never on replay.

Mutations use session-generation fencing and durable command receipts. An exact
command replay returns its recorded status without inserting a second request,
changing status again, issuing another Party invitation or repeating feedback.
Reusing a command id with different canonical action/target content is refused.
The existing leader upsert/remove receipts remain owned by S14a.

## Query reply

Command 444 returns `TeamFinderResult`.

| Field | Meaning | Shape |
|---|---|---|
| 1 | kind: 1 Browse, 2 Applicants | required |
| 2 | repeated Browse listing submessage | Browse only, at most 100 |
| 3 | current `LeaderListing` encoded by the existing command 390 codec | Applicants only; absent when no active advertisement |
| 4 | global occupied slots (`u16`) | required for Applicants, even zero |
| 5 | repeated Applicant submessage | Applicants only, at most 100 |
| 6 | nonzero next-page handle | optional, equal to the last emitted entry's handle |

Absent Applicants listing requires zero occupancy, no members and no next cursor.
An active advertisement can have an empty final page. The global occupied count
is independent of the page and must never be derived from just the visible page.

Browse listing fields:

| Field | Meaning |
|---|---|
| 1 | leader handle (`u64`, nonzero) |
| 2 | leader name (nonempty UTF-8, at most 64 bytes) |
| 3 | `LeaderListing`, using the unchanged S14a codec |
| 4 | global occupied slots (`u16`) |
| 5 | viewer status: 0 none, 1 pending, 2 invited, 3 accepted, 4 denied |

All five fields are required. The nested authoring listing preserves the baseline
free slots and party flag; occupancy carries the current effective use. Clients
must not write effective free slots back as the authoring baseline.

Applicant fields:

| Field | Meaning |
|---|---|
| 1 | member handle (`u64`, nonzero) |
| 2 | name (nonempty UTF-8, at most 64 bytes) |
| 3 | level (`u16`) |
| 4 | client vocation id (`u8`) |
| 5 | status: 1 pending, 2 invited, 3 accepted, 4 denied |

All five fields are required. Status 0 is valid for a browsing player's absence
from a listing, but cannot represent an existing applicant. The leader appears
as status 3 on the first applicant page and is not counted again in accepted
member occupancy.
