# NX-8 C20c — death protection receipt

Donor evidence fetched read-only on 2026-11-05:

- Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`,
  `src/creatures/players/player.cpp:4190–4227`: death sends PvP/PvE and blessing
  messages, removes only Twist in a protected PvP death, otherwise regular
  blessings, then sends blessing status and the relogin window.
- Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
  `src/creatures/players/player.cpp:4207–4248`: the equivalent blessing retention
  and consumption branches. Classification: OTS_HYPOTHESIS_ONLY.

Oteryn already commits these decisions in
`character_death/commit.rs` via the accepted `blessing_disposition` rules. NX-8
changes no loss rule: it publishes the immutable before-minus-after receipt,
identified by its occurrence. The bounded lookup uses the existing unique
(Character, committed revision) index, and is fenced by the current session and
node. No migration is needed. Number reservation: capability 382, command 664,
mechanic ordinal 639, offer order 638. Command 665 remains unused.

The native HUD fetches on login and actual alive-vitals wire events (including
a coalesced death/respawn with no observed zero health), displays consumed names while retaining the death
occurrence in the wire and client model for correlation, preserves its last valid receipt on malformed/refused replies, and
keeps death input blocking independent of this read-only display. Cached vitals
republished after a query do not schedule another query, preventing feedback;
only one query may be in flight.

Wiki checks: **wiki-pending** — current Global blessing names, death message
wording, Twist retention under red/black skull, adventurer blessing protection.
No wiki URL was fetched because Fandom returns 402 in this environment.
