# GAP-CL1: client combat views

Reconciled base: `a9db6a76a1ba375cb29104ec4a814310651c35fb` (includes #2994,
#2991 and #2990). Original work started at `c8b3890e601e6073713dcac2ea9ee271f4a257e8`.
No migrations or new capability, command, domain or message numbers. Existing
capability 199 / command 388 is advertised and used by the native window.
No Dawnport, OT-only feature, Prey Hunting Task or starting-equipment changes.

## Reverification and parity after reconciliation

Main #2994 now owns all character-tab controls, bounded strict tab decoding,
pending/requested-tab correlation, and the complete General Stats display.
Those implementations and all their tests are retained. The former GAP-CL1
native window and duplicate combat decoder are superseded by main's versions.

Donor: Crystal `zimbadev/crystalserver`, `main` audit revision
`201189f871d23597b82caf88fd611454a2ddefb8`,
`src/server/network/protocol/protocolgame.cpp` (OTS_HYPOTHESIS_ONLY).
This package presents already admitted server values; it adopts no new formula.

| Mechanic | Source / accepted native difference | Reconciled result |
| --- | --- | --- |
| C03 | Donor `:5016-5325` sends offence, defence and miscellaneous combat statistics. Existing native schema `crates/protocol-oteryn/src/cyclopedia_character/combat.rs:1-5` explicitly uses ppm and optional knowledge tags instead of donor doubles. | Already implemented by main #2994. Retain regression coverage for all 84 fields, signed percentages, points/IDs/seconds, unknown distinct from zero, and the production session-to-panel flow. Keep main's `unavailable` and `seconds` wording, observation time and revisions. |
| C15 | Donor `:4401-4405` sends stamina in minutes. Native `crates/protocol-oteryn/src/cyclopedia_character.rs:371-395` defines the existing little-endian GeneralStats layout. | Main's strict reader retains all General Stats fields and optional-vitals validation. Format decoded stamina as hours:minutes in its own helper. No inferred XP multiplier: Premium and bonus facts are absent from this response. |

## Stopped mechanics (no partial implementations)

C08 STOPPED: `crates/protocol-oteryn/src/actor_condition_snapshot_candidate.rs:1-4`
explicitly has no routing/capability allocation; its `Snapshot` at `:32-45` has
appearance/light facts, no condition catalogue or expiration list.
Donor `protocolgame.cpp:5712` supplies icons; durations need an admitted native contract.
Requires a server projection/contract decision outside this client-only package.

C10 STOPPED: `crates/protocol-oteryn/src/actor_spell.rs:128-136` already decodes soul,
but carries no rest-regeneration status, eligibility or deadline.
`GeneralStats` at `cyclopedia_character.rs:308-318` adds no rest facts either.
Donor `protocolgame.cpp:4400-4409` separately sends soul and regeneration ticks.
Requires deciding and publishing authoritative rest state; no client timer is invented.

C11 STOPPED: `crates/protocol-oteryn/src/actor_spell.rs:128-136` includes Harmony
and Serene but no Battle Instinct; `attack.rs:133-140` only carries attack modes.
Donor `protocolgame.cpp:11527-11541` sends Harmony and Serene separately.
The full requested state needs a server-owned Battle Instinct observation/contract.
No inference from Harmony or the generic in-fight flag is introduced.

C14 STOPPED: `crates/protocol-oteryn/src/cyclopedia_character.rs:308-318` sends level,
experience and skill levels, but no skill tries, mana spent, thresholds or percentages.
Main #2994 displays the available values; the durable producer at
`crates/gameplay/durability/src/durability/cyclopedia_character.rs:96-118` lacks progress.
Donor `protocolgame.cpp:4387-4389,4421-4433` sends progress; a server extension is needed.

## Verification seams

- UI regressions go through `request` and `apply_result`, preserving main's pending
  and requested-tab guards, and check `panel_text`. The stamina boundary regression
  also failed on rebased main's minutes-only presentation before the formatting change.
- The app regression drives `run_session` -> capability-199 command -> Outbox
  result reduction -> mailbox-backed panel reader for all four combat/stamina tabs.
- Session regression keeps capability 199 advertised with its existing 388 route;
  the exact advertised-set assertion retains all capabilities added on main.
- Main's native navigation, target/page validation and tab parser regressions remain.
- Windows control creation needs a Windows runner; this environment has only the
  Linux Rust target. No target/tool installation was attempted.
