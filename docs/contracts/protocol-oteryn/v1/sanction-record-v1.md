# Sanction Record V1

N009. Optional capability 497 has no requirements and is offered only in
`BUNDLE_WORLD_OFFERED_CAPABILITIES`. Command 6240 `SANCTION_RECORD_QUERY` reads the admitted
Character's own sanction record from Game's read-only projection of Platform's sanction
decisions (`docs/contracts/PLATFORM_SANCTION_LOYALTY_EXPORT_V1.md`, migration 2800). Platform
remains the only sanction authority. The read never mutates, enforces or explains a sanction
beyond the exported fields.

## Request

Empty (0 bytes). It cannot name another Character or account: the server resolves the account
from the admitted session's current fence. An unselected capability, a stale
session/connection/lease or a non-empty payload is rejected with an empty result.

## Reply

All integers little-endian. Exact length, at most 4546 bytes:

- `count:u8`, 0..=32;
- `more:u8`, 0 or 1. 1 only with `count = 32`: older records exist and are omitted;
- `count` records, newest `starts_at_ms` first (ties by stream id, descending):
  - `kind:u8`: 1 mute, 2 jail, 3 character ban, 4 account ban, 5 namelock, 6 notation;
  - `scope:u8`: 0 account-wide, 1 the asking Character. Kind 4 is always 0; kinds 1, 2, 3 and 5
    are always 1;
  - `revoked:u8`: 0 or 1, Platform revoked or lifted the decision;
  - `reason_code:u16`: 1..=65535, Platform's reason category id;
  - `starts_at_ms:i64`: > 0, Unix milliseconds;
  - `ends_at_ms:i64`: 0 for permanent or no end, otherwise > `starts_at_ms`. Kinds 1 and 2
    always end; kinds 5 and 6 never carry an end;
  - `reason_len:u8`, 1..=120, then that many UTF-8 bytes without control characters.

The record holds the account's account-wide rows and the rows of the asking Character only;
rows of the account's other Characters are not included. Revoked rows stay in the history with
`revoked = 1`. Whether a sanction is active is the client's comparison of `revoked`, the end and
its clock; it never gates gameplay. An empty record means Platform exported nothing for the
account, not that Platform holds no decision. The session ingress owner retains the result
for same-command replay; a new command reads the current projection.
