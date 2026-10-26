# Zone logout and server messages

Owner requirement: ZONES-NOLOGOUT-1 (NIGHT-W1). The current Channel runtime owns
the actor's position and logout decision. World and Channel identities stay distinct;
all existing session-generation fences and terminal-release saves remain in force.

The booted bundle retains OTBM tile flags. Bit 3 (`8`) is NOLOGOUT, independently
of protection-zone bit 0 and NoPvP/PvP bits 2/4. A current actor on NOLOGOUT cannot
voluntarily logout, including in a protection zone and during a fight. Mutable items
and unknown spell-stack facts cannot hide this flag. Source-map qualification retains
the same independent flag. Unreadable or mismatched zone evidence authorizes no release.
The engineering fixture, which declares no OTBM zones, keeps its existing behavior.

Existing NoPvP/PvP qualification keeps protection-zone > NoPvP > PvP precedence.
Existing PvP policy remains authoritative: a PvP tile grants no arena rules or PvP-world
override. Outside protection zones the existing live PvP and PvE fight locks refuse
logout; a qualified protection zone permits it after the NOLOGOUT check. Logout reads
both flags from immutable map evidence: a bundle's independently pinned map revision
qualifies its zones even when the entry generation differs or mutable items make the
spell-tile projection unavailable.

Capability 92 `ZONES_NOLOGOUT_V1` adds domain 57 `SERVER_MESSAGES` and no command.
Existing command 23 and `LogoutResultV1` retain their encoding: a NOLOGOUT refusal is
`REJECTED/BUSY`, followed, only with selected capability 92, by a domain 57 status
text: `You can not logout here.` A fight refusal retains `IN_FIGHT` and its retry,
followed under 92 by `You may not logout during or immediately after a fight!`.
The command result precedes the message. Both refusals leave the actor and session
playing; the existing outstanding-logout limit and input gates continue to apply.

The optional root node configuration `server_motd = "Welcome to Oteryn!"` supplies
this World's configured message. Absent or empty disables it. `NOLOGOUT-RL-01`
bounds it to 255 UTF-8 bytes, checked before listener startup. A selected capability
92 receives it in the login snapshot, after authoritative admission. Only successful
delivery of the complete snapshot consumes its receipt. Same-session reconnect,
resync and transfer carry that receipt and send an empty domain snapshot instead;
a new login has a fresh receipt. No configured text or receipt is persisted as Game truth.

Domain revisions start at zero and advance once per status delta; both revision and
MOTD receipt travel with `SessionContinuity`. Snapshot and delta type 1 use
`server_messages_v1.proto#ServerMessageV1`, the existing bounded status-text encoding.
No frame in this domain is emitted without capability selection. Clients without 92
retain the previous logout-result schema; client rendering is outside this server task.

Parity evidence (read-only pinned sources):

- Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`:
  `src/io/iomap.cpp:167-176`, `src/server/network/protocol/protocolgame.cpp:1154-1161`,
  `src/utils/tools.cpp:1456-1457,1528-1529`,
  `src/server/network/protocol/protocollogin.cpp:59-68` (configured nonempty MOTD).
- Crystal `201189f871d23597b82caf88fd611454a2ddefb8`:
  `src/io/iomap.cpp:258-267`, `src/server/network/protocol/protocolgame.cpp:849-856`,
  `src/utils/tools.cpp:1485-1486,1557-1558`,
  `src/server/network/protocol/protocollogin.cpp:61-70` (configured nonempty MOTD).

The reference MOTD is on the legacy login response. This task places the configured
text after Game admission in the native login snapshot, as requested, without adding
a legacy login protocol or changing Platform identity ownership.

No database migration, map format change, world pin change, staff bypass, forced
disconnect rule, arena implementation or new client command is part of this contract.
