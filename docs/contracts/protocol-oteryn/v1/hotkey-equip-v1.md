# Hotkey equip V1

PKG5 I17 candidate. Capability 179 requires equipment (12) and container views (14).
Command 348 resolves a definition under the active Channel content pin on the server.
The equipment owner remains the only equipment custody writer.

## Request

Exactly four bytes: a nonzero unsigned big endian item definition reference, using
the same generation index as domain 9. Zero, unknown references and other lengths
refuse before a write. No item instance identifier comes from the client.

## Reply

The existing `ItemMoveResultV1` with equipment results enabled, carried in an
accepted command result as for command 9. Domain 9 and open container domain 14 deltas follow
a successful move. An unknown durable outcome disconnects for reconciliation.

Capability 179 admits direct backpack sources and a worn-item toggle into the
main backpack. Capability 180 extends carried selection below. Both routes share
the existing equip timers and feared gate. Managed destinations, automatic hand
conflicts and ammunition-to-quiver routing remain follow-up work.

During the shared ordinary-slot delay, a later consecutive valid ordinary equip
press replaces the pending press. Each superseded command receives the existing
`ItemMoveOutcome::Rejected` in command order; only the latest press reaches the
equipment writer. Jewelry, malformed or unknown definitions, replayed commands
and unrelated commands do not replace it. A committed receipt always takes
precedence. The existing 200 ms cooldown starts after each successful equip;
replacement does not restart it. Crystal `201189f871d23597b82caf88fd611454a2ddefb8`
`src/game/game.cpp:3718-3732` and `src/creatures/players/player.cpp:3038-3054`
provide the pending-action replacement evidence (`OTS_HYPOTHESIS_ONLY`).

## Carried selection extension (capability 180)

`HOTKEY_EQUIP_CARRIED_V1` requires 179 and changes server selection for the same
command 348. It adds no message, command or domain. Without 180, the existing
direct-backpack route remains available. With 180, the equipment owner resolves
both D1's worn toggle and D2's carried selection under its current item fence,
Character root and equipment locks, then writes one equipment receipt.

Search is depth first in display order, including closed bags: Crystal
`63b93e2c59268c79118f4573e397e5f0217abb2b`, `src/game/game.cpp:3662-3675`
and `src/items/containers/container.cpp:1045-1087`. Crystal searches the backpack
before a separate loot pouch (`game.cpp:3747-3751`). Carried pouch contents are
included in that tree. The current Gold Pouch owner admits coins only; no
store-inbox or detached-pouch custody source is added by this extension.

Owner D2 overrides first-match selection: less remaining time or fewer charges
than the qualified full budget means started. The first started copy in search
order wins, otherwise the first unused copy. A stored full-budget row remains
unused. D1's worn match takes precedence over all carried copies.

Migration 0546 adds an optional nested source parent to the existing equipment
receipt. A same-transaction removal record captures the actual nested edge and
its Character ancestry before deletion. Deferred guards require its exact
location, the equipped successor, the item revision and the audit envelope.
An occupied target still exchanges atomically with the main backpack; a nested
source does not free a main-backpack vacancy. A refusal rolls back the entire
transaction. Replay binds the original definition and slot before resolving
custody again. Existing direct moves preserve their historical binding bytes.

Managed destinations, hand conflict displacement and ammunition-to-quiver are
follow-up work. This extension makes no parity claim for those rules.

Canary `52550d5e3c0ffe565f4bd1014f4e5575abf1059b` follows the same
backpack-then-pouch search (`src/game/game.cpp:4203-4207`) and depth-first
iterator (`src/items/containers/container.cpp:1250-1295`). These donor references
are `OTS_HYPOTHESIS_ONLY`; owner D1 and D2 govern the copy-selection differences.
