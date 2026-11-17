# CLHUD E12 — native identity and outfit entry

Base main: `9668e0a04`. Existing capability 250 / command 470 are reused;
no migration or new protocol/registry numbers are allocated.

Donor: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
`data/libs/systems/hireling.lua:30-41,205-227`: hireling outfit identities and
sex-specific looks. Existing native controls use the server's owned identity,
outfit entitlement bits and four colors; the client does not manufacture ownership.

Current main already routes the ordinary Ctrl+Shift+H entry through feature 165
into feature 250 when admitted. Native controls list owned identities, select
entitled outfits, save colors and return a hireling to its lamp. The strengthened
`hireling_management_crosses_the_real_session_loop` test starts at that keyboard
entry, checks the management window and List request, and retains the existing
production session-loop identity/outfit wire and saved-state assertions. It also
checks that the received identity is presented as Alice.

The strengthened test passed in the full native app suite (345 library cases,
17.16 s). All app/UI targets passed, as did changed-crate check, all-target Clippy
with `-D warnings`, formatting and architecture checks. Full command timings are
in `CLHUD-CONTRACT-NOTES.md`.
