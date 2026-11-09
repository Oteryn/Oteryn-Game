# Returner reward (T145)

Owner decision B 2026-11-09. Existing capability DAILY_REWARD_V2 (182),
commands 6124 query and 6125 claim. No new capability or commercial balance.

## Rules and reference

`docs/reference/tibia-manual/accounts.md:77` records a twelve-month inactivity
condition and a welcome-back gift on the returning buddy's first login. Its
invitation, email, Premium and purchase benefits belong to Platform. The client
reference contains `ReturnerRewardNotificationDialog.qml` (`exe-ui.txt:88`,
`TIBIA-CLIENT-PARITY-2026-11-08.md:189`). Canary
`99902524e052f37574194466c2949c576e4ab269`
`src/server/network/protocol/protocolgame.cpp` and
`data/scripts/creaturescripts/player/login.lua` define no returner gift contents
or Game entitlement/claim operation.

Owner decision B 2026-11-09, reference silent: a separate Game-only welcome gift
is **one health potion**, admitted item `oteryn:item.tibia.i266`, delivered to the
returning character's Inbox. It has no expiry and no commercial effects.
Eligibility requires a previous session of the same Account, across Worlds,
which is terminal and has a durable `game_character_session_ends` timestamp.
The current session's immutable DB-stamped admission time must be at least
**twelve calendar months** after that end, in UTC. The immediately preceding
Account session is selected by the immutable training session sequence. Missing
history, missing end evidence, a nonterminal predecessor, future end time, or a
first-ever Account login yields no entitlement. Ordinary daily claims/streaks
cannot create returner eligibility. Activity/lifecycle files are unchanged.

The native client queries on entering play. The server resolves eligibility from
its FND-04 sessions and mints one persistent entitlement per returning session,
with a unique Account/predecessor-session anchor, so concurrent first logins cannot
create two gifts for one inactivity gap, under the current character recovery/node/scope/session/connection-generation
fence. Repeated queries recover the same pending entitlement. A pending gift
survives reconnect and server restart and remains owned by that Character/World.
A later qualifying return may create a new entitlement after the earlier gift
was claimed. Eligibility and the gift are independent of Premium and a shrine.

## Authority and persistence

`game_returner_reward_entitlements` freezes Account, Character, World, returning
session and UUIDv7 identity. `game_returner_reward_claims` is an immutable receipt
with a unique entitlement and command identity. The claim revalidates the current
fence independently of the entitlement, then writes one native reward-claim Inbox
MINT and its receipt in the same type-2 transaction. A failed sink rolls everything
back. A stale generation writes nothing; a replay returns already-claimed without
another mint. WorldId and ChannelId stay distinct throughout the native Inbox plan.
No client-supplied amount, date, Account, Character or World is authoritative.

## Wire

Query: empty payload. Reply is `RET1`, state u8 (0 none, 1 available, 2 claimed),
then exactly sixteen UUID bytes. None uses sixteen zero bytes; available/claimed
require a UUIDv7. Maximum 21 bytes, no trailing bytes. Failed queries are rejected without an announcement.

Claim: exactly the sixteen-byte offered UUIDv7. Reply uses the existing bounded
daily result: 1 claimed with transaction UUID, 2 already claimed, 3 no matching
entitlement, 5 unavailable, 7 stale authority. The native notification is shown
once for each newly announced available entitlement. Claim state changes only
on a confirmed server result; a daily counter never substitutes for entitlement.
