# S17 Game adapter and Platform handoff (SRV11)

This extends the accepted S17 design and S17-A, without changing character
consent, audit receipts or account authentication authority. Cast-on state is
session-only: logout, transport replacement and restart stop it. Preferences
survive in Platform; Game must never persist a live cast across restart.

## Account preferences contract

The existing authenticated mTLS `/internal/game/cast` exchange gains operation
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
`47dfd51f45280a59a1d3e50ba7edd573d7234446`. Neither tree has a player-cast
service; NPC spectator queries and staff broadcast are unrelated. The accepted
S17 design sections 2 and 6 supply the deliberate differences: account-only
viewers, 50/cast, two/account, consent slate, optional 0–300 s delay, isolated
live caster chat and account-scoped moderation. Current Tibia rather than
OT-specific commands is the target; no `!commands` are introduced.

Rendering evidence in Crystal `src/server/network/protocol/protocolgame.cpp`:
`canSee` at 1799–1847; `sendCreatureSay` at 7626; `sendMagicEffect` at 7846;
`sendMapDescription` at 8043. These establish observer-relative visibility,
local speech, visible effects and map delivery. Game projects only committed
caster-visible data, strips actionable handles, and never copies private UI or
raw player packets. The caster's authority, map owner and chat queue remain
owned by the player path. Cast fan-out is off-tick, bounded and read-only.
