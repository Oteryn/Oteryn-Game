# S17 Game adapter and Platform handoff (SRV11)

This extends the accepted S17 design and S17-A, without changing character
consent, audit receipts or account authentication authority. Cast-on state is
session-only: logout, transport replacement and restart stop it. Preferences
survive in Platform; Game must never persist a live cast across restart.

## Account preferences contract

The existing authenticated mTLS `/internal/v1/casts/node` exchange gains operation
`preferences`, with `owner` (canonical account UUID) and authenticated node
identity. Success is a bounded JSON object containing `delaySeconds` (0–300),
`viewerCap` (1–50), `chatEnabled`, `passwordRequired`, `accessRevision` (>0),
and `description` (0–256 UTF-8 bytes, no control characters). Unknown fields,
invalid text, stale revisions and unavailable policy fail closed. Game's
`PreferencesStore` trait isolates this contract and has an executable test double.
Start/update must match the current stored settings, including the password
revision. The client never supplies a password/verifier to Game.

Platform owns the authenticated account settings write/read UI, description,
password KDF/verifier and rotation, invitation checks, directory pagination and
metadata delayed by the configured delay. Its directory contains only renewed
Game announcements (expiry no longer than five seconds); stop removes it.
A password check and atomic global two-sessions/account reservation precede
issuance of the existing single-use observer ticket. Incorrect passwords,
missing casts and revoked tickets have the same unavailable response. Existing
consume/renew proofs bind node, account, cast, epoch and access revision. Game
independently enforces the cast and node caps and releases reservations on every
observer termination. This is the remaining Platform delivery handoff, not a
second Game identity or password database. Wiki checks: wiki-pending.

## Rules and evidence

Donors were fetched read-only at Crystal
`201189f871d23597b82caf88fd611454a2ddefb8` and Canary
`47dfd51f45280a59a1d3e50ba7edd573d7234446`. Crystal has no player-cast service in that pinned tree. Canary's donor service
is `src/creatures/players/livestream/livestream.cpp` (abbreviated below as
`livestream.cpp`), with protocol composition in
`src/server/network/protocol/protocolgame.cpp`. Staff broadcast and NPC
spectator queries are unrelated.

| Row / rule | Pinned Canary evidence | S17 rule / recorded accepted difference |
| --- | --- | --- |
| S17a live settings, start/stop and access rotation | `livestream.cpp:237` initial settings; `:249` apply state; `:255` password rotation disconnect; `:260` stop; `:281` password/description assignment | Retain S17-A session/revision fencing and stop on transport/logout/restart. Account settings/password verifier persist in Platform under the accepted S17 ownership boundary. No plaintext password reaches Game. |
| S17a persistence and description | `data/scripts/talkactions/player/livestream_system.lua:249` password KV; `:257` description controls and `:270` description KV | Use the explicit Platform preference trait/contract; Game checks bounded UTF-8 description and revision. Description has a 256-byte Game transport bound. No donor `!livestream` command or XP bonus is imported (owner excludes OT-only features). |
| S17b live directory and join | `livestream.cpp:216` lists only broadcasting online casters; `:314` live caster check; `:340` password; `:355` per-IP and `:362` cast limits | S17 design sections 2/6 and this task require authenticated account-only tickets, uniform refusals, 50/cast and two/account. Platform directory consumes renewed Game announcements; Game independently checks epoch/access, cast/node capacity and persistent bans. Donor premium/IP/minimum-level variants are not imported. |
| S17b view and exit | `livestream.cpp:486` packet fan-out and map refresh; `:541` opcode allowlist; `protocolgame.cpp:12379` viewer login; `livestream.cpp:408` viewer removal and `:660` disconnect | Accepted S17 privacy/read-only design uses typed display projection and consent bindings instead of raw packets, caster-player pointer/client substitution or open containers. All viewer content follows chosen 0–300 s delay; viewer input never creates an actor or changes caster authority. |
| S17c names and channel | `livestream.cpp:388` Viewer-N; `:396` join; `:431` leave; `:441` isolated chat; `:724` owner/viewer channel fan-out | Stable server-generated opaque Viewer handles with join/leave presence, isolated live caster panel and delayed viewer copies. No donor `/name`, `/show`, `!commands`, private room routing or gameplay execution. |
| S17c mute/kick/ban | `livestream.cpp:265` mutes; `:674` kick; `:692` ban and disconnect; `:812` muted sender refusal | Accepted S17 design section 2.4 and the SRV11 rerun require account-scoped mute and persistent owner-account bans across epochs/restarts, rather than donor viewer-name/IP session bans. Typed caster actions, durable atomic ban/audit, admission/renewal checks, and reservation release implement that rule. |
| S17c limits/audit | `livestream.cpp:24` 5-second window, `:25` three messages; `:798` rate check | Keep accepted S17 section 2.4's per-account burst three, refill one/3 s and 20/s room ceiling; bound text, presence, media and caster queues. Existing P30D audit plus new account-ban kind 12 records moderation without chat text, tickets or media. |

Current Tibia rather than OT-specific commands is the target. The accepted S17
design supplies consent-by-default-off, optional delay and competitive casting;
these are deliberate differences from the donor, not inferred donor behavior.

Rendering evidence in Crystal `src/server/network/protocol/protocolgame.cpp`:
`canSee` at 1799–1847; `sendCreatureSay` at 7626; `sendMagicEffect` at 7846;
`sendMapDescription` at 8043. These establish observer-relative visibility,
local speech, visible effects and map delivery. Game projects only committed
caster-visible data, strips actionable handles, and never copies private UI or
raw player packets. The caster's authority, map owner and chat queue remain
owned by the player path. Cast fan-out is off-tick, bounded and read-only.

Canary corroboration: `src/server/network/protocol/protocolgame.cpp:8006`
(local speech), `:8264` (visible magic effects), `:8477` (caster-relative
map description). Password, description and preferences are the only deferred
Platform delivery. The S17-A announcement/consume/renew/release contract already
supplies the directory and account-only viewer route; this package extends its
Game side and does not replace it.

The SRV11 rerun explicitly requests public-chat representation, superseding
section 2.3's earlier conservative omission of ordinary local speech. Only
local speech actually delivered to the caster from an eligible visible actor
is represented; private/room/NPC dialogue remains excluded. Public actor handles
are assigned after hidden-actor filtering: inserting a hidden actor must leave
the encoded public scene unchanged. `s17_hidden_actor_does_not_change_public_handles_or_bytes`
exercises that production projection boundary.
