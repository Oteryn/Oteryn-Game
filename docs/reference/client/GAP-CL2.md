# GAP-CL2 client mechanics

Original audit base: `origin/main` / `c8b3890e601e6073713dcac2ea9ee271f4a257e8` (2026-11-05).
Rebased onto `a9db6a76a1ba375cb29104ec4a814310651c35fb`; original audit observations below are historical.
Scope is the four client slices in the owner packet, not the older server-only ledger states.
Donor: `zimbadev/crystalserver`, main, revision
`201189f871d23597b82caf88fd611454a2ddefb8`; evidence class `OTS_HYPOTHESIS_ONLY`.
Donor paths below are relative to that revision. No donor assets are redistributed.

| Mechanic / rule | Main evidence / result | Donor evidence |
| --- | --- | --- |
| C18a: ordinary spell hotkeys aim at the current opponent, with directional fallback when no target exists | Already implemented; skip production changes. `apps/client/src/windows_shell/input.rs:39` routes hotkeys to `PlayView::cast`; `crates/client-runtime/src/features/local-spell-action-bar/protocol.rs:48` reads the target and `:37` constructs the intent. Added admitted-session wire regression coverage for both cases in `tools/dev-client/tests/gap_cl2_spell_targeting.rs`. | `src/creatures/combat/spells.cpp:1238-1250` |
| C18b: an ordinary spell may carry the clicked tile, independently of an attack target | Missing on main: `apps/client/src/features/local-spell-action-bar/input.rs:16` could send only `CombatCommand::Cast`; `crates/client-runtime/src/features/local-spell-action-bar/protocol.rs:37` could select only None/AttackTarget. Now Shift+digits selects the existing spell-book number, the next world click sends command **3**, `Position(x,y,floor)`, `aim_at_target=false`. It never uses the rune command. Normal hotkeys keep their existing behaviour. | `src/creatures/combat/spells.cpp:1252-1261` forwards the crosshair tile to ordinary instant-spell validation. |

The Shift binding, Escape/focus-loss cancellation and one-click consumption are local input
choices, not donor combat rules or claims about Tibia's default key bindings. Server validation
still decides whether the selected spell accepts a position; this adds no client-authoritative
range, legality, mana or cooldown rule. Cursor selection can reach entries beyond the four-slot bar.
The mode displays “Click a tile; Escape cancels” in the native combat status.

## W20b — STOPPED (design decision)

1. Main has entity-light wire fields: `crates/protocol-oteryn/src/world_spatial_entities.rs:50-75`.
2. The client does not advertise capability 19 (`crates/session/src/capability_support.rs:19`); join/delta decoders discard lights (`join.rs:158`, `domain_dispatch.rs:81`).
3. Native rendering has no global or per-entity lighting pass: `crates/renderer/src/scene_gpu.rs:56-59` samples the atlas unchanged.
4. Donor `src/server/network/protocol/protocolgame.cpp:9790-9796` supplies creature level/color, not a client compositing algorithm.
5. Needed decision: current-Tibia reference for level-to-radius/falloff, palette conversion and ambient composition, or an explicitly accepted rendering difference; no partial capability advertisement was added.

## S13c — STOPPED (server projection contract)

1. Main party entries carry handle/name/leadership/expiry only: `crates/protocol-oteryn/src/party.rs:36-48`.
2. The native panel consumes that membership result only: `crates/client-ui/src/features/0042-party/mod.rs:14-24` and `:70-106`.
3. No party-member active-effect payload exists to decode; party/skull icons cannot identify active buff conditions or expiry.
4. Donor `src/server/network/protocol/protocolgame.cpp:7899-7914` refreshes party creature views; `data/scripts/spells/party/protect_party.lua:7` defines a member buff, not the missing Oteryn projection contract.
5. Needed decision: server-owned effect identities, visibility, expiry/removal and party-handle binding; implementing that exceeds this client-only package. No fabricated member state or migration was added.

## Verification scope

C18b red: `cargo test --locked -p oteryn-client --lib cursor_spell` failed on missing
cursor API/type before production changes. Green coverage drives the native input queue,
ordinary spell wire decoder and session refusal feedback, including an existing opponent.
No migration, new capability, protocol message, or command number is allocated.
PostgreSQL is available from the required startup; these client changes introduce no PG cases.

Focused existing PG coverage executed with a nonempty admin URL:
`cargo test --locked -p oteryn-gameplay-dispatch --all-features crosshair_spells_postgres -- --nocapture`
passed both real-dispatch scenarios (2 tests, 47.98 s).
