# Battle Instinct status v1 — GAP-SV7 C11

Rebased onto `c97858c3f440fd26676d3bc9be6d6a35b2c711ff`. Existing native
routes and capabilities 286/287 are retained; the five new shortcuts use
Ctrl+Alt+Shift plus C/R/B/K/P respectively to avoid the base bindings.

Capability 278, command 494. Bundle World only, with active native gameplay.
The empty query addresses only the current admitted actor; it cannot name another
player. It reads the live Wheel projection and current Channel census under the
existing runtime owner lock, with exact actor/session lookup and connection fence.

## Request

Empty payload (zero bytes). Nonempty payloads are rejected.

## Reply

Exactly nine bytes: canonical boolean `active` (0 or 1), then little-endian `u32`
`melee` and `shielding` bonuses. Inactive replies must have both bonuses zero.
Active with zero bonuses means the perk is eligible but its adjacent-creature
threshold is not met. Unknown or stale state is refused, never rendered as active.
The client retains the last confirmed reply on refusal or malformed bytes and
clears it on session end. Refresh uses the existing session command queue.

## Gap on current main

At `c8b3890e601e6073713dcac2ea9ee271f4a257e8`,
`crates/protocol-oteryn/src/actor_spell.rs:127-136` has Harmony and Serene but no
Battle Instinct field. The authoritative calculation already exists at
`crates/gameplay/spell/src/spell/cast/wheel.rs:140-177`; C11 adds its production
read route and native panel without changing the calculation.

## Donor parity

Pinned Crystal `zimbadev/crystalserver`, branch `main`,
revision `201189f871d23597b82caf88fd611454a2ddefb8` (15.30 donor).
Branch provenance revalidated by the package lead: this revision is an ancestor of
current `main` (`2908aa0`), with the compare merge base equal to the pinned revision.
Evidence class: `OTS_HYPOTHESIS_ONLY`, used as the owner-requested donor reference.

| Rule | Source |
|---|---|
| Battle Instinct changes melee and shielding, not every skill | `src/creatures/players/player.cpp:7424-7431` |
| Recompute against the current adjacent monster census, excluding player-owned monsters; five nearby monsters grant +1 melee/+6 shielding | `src/creatures/players/wheel/player_wheel.cpp:2981-3002`; retained Oteryn combat owner `wheel.rs:140-177` uses the current pinned numeric catalogue |
| Only eligible level 51+ Premium promoted actors activate the Wheel; inactive reports zero | `src/creatures/players/wheel/player_wheel.cpp:2116-2135`; existing Oteryn `wheel.rs:512-519` rechecks conservative Premium expiry |
| Oteryn displays a separate own-player query and panel | GAP-SV7 accepted wire addition; donor exposes the same bonuses through aggregate skills |

No migration or gameplay-rule change.
