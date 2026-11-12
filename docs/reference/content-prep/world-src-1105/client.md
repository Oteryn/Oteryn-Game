# Historical client evidence from the c8b3890e patch

This describes the superseded first patch. The continuation retains main1fa04bbb's
0057 feature, mailbox/session routing and capability advertisement byte for byte;
none of those implementations below is reapplied or duplicated. Main's registration
uses CTRL+ALT+D. Fresh continuation evidence is in continuation-validation.md.

# W12 client handlers

Existing native contract: `crates/protocol-oteryn/src/cyclopedia.rs:4–7,35–66,69–89,118–161`.
Commands 72/73 and capability 57 are reused; no new protocol identifiers.
The server resolves the existing area-1 request against current authoritative position.
The UI uses the shared codec through session/runtime reexports, requires selected
capability state, permits one outstanding action, validates donation bounds, and
keeps the previous snapshot on malformed replies. The map page renders only the
server's bounded visible viewpoints and progress/donation status.

Red: `cargo test --locked -p oteryn-client-ui --test cyclopedia_map --quiet`
failed E0432 (missing cyclopedia_map module), retained in
`/tmp/world-src/client-red.log`. Green: the same focused test passed after implementation; the final two-case
client test is included in the 203 passing tests from
`cargo test --locked -p oteryn-client-ui -p oteryn-client-runtime -p oteryn-session --quiet`.
`cargo clippy --locked -p oteryn-client-ui -p oteryn-client-runtime -p oteryn-session --all-targets --quiet -- -D warnings` exited 0.
Logs: `/tmp/world-src/client-tests.log`, `/tmp/world-src/client-clippy.log`.
These are client contract facts, not new Tibia gameplay/source claims.

Number inventory against current origin/main c8b3890e (repository number tool,
`--no-prs`, no publication): next free capability 276, command 492. Neither is
allocated or introduced by this package. Sole allowed migration number: 0963.

## Production application caller

`apps/client/src/play/session_loop.rs` initializes capability57 selection and
routes actual session command results to the map Outbox reducer. The existing
Feature command queue and Session Stepper send the shared72/73 codecs unchanged.
The app's auto-mounted0057play feature supplies native Refresh/Start/Shuffle/Find/
Donate controls; Ctrl+Shift+M opens it without taking the Market's Ctrl+M chord.
No new wire ID is used. The app regression first failed with missing PlayLink/
Outbox methods (`/tmp/world-src/client-shell-red.log`), before adding this caller.
A second test drives the real session loop and checks its result reaches the panel.
Registration fixtures are regenerated from source fragments with
`python3 tools/architecture-check/refresh_client_registrations.py 0057-cyclopedia-map`.
The Windows controls follow existing HWND owner-thread composition; host checks
exercise the portable command/reply loop, not a Windows window.

The default Session capability advertisement and gated route allowlists now include
existing57; source commands72/73 keep their original schemas. A real duplex Session
regression first failed because57 was absent from the default set, then passed
query/donation exchange and unselected-capability refusals after the allowlists
were completed (`/tmp/world-src/client-negotiation-{red,green}.log`).
