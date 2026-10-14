# OTV2-20261014 — prelogin F10 focus and current-main composition

```yaml
task_id: OTV2-20261014-client-prelogin-focus
issue: 1927
pr: 1942
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
branch: feat/client-completion-20261009
owner: root
```

Parent client-game-ui, sole root writer, AUTHORING. Owned client paths:
settings_ui.rs and windows_shell.rs, with own evidence/continuation documents.
Normal current-main reconciliation preserves accepted producers and derives
the World input pin if needed. No E2E, login/account action, new capability,
HUD/graphics redesign, input dependency fork, server or deployment mutation.

## Finding and bounded repair

The native prelogin F10 fallback dispatches directly from winit KeyboardInput,
outside the semantic adapter that rejects unfocused/synthetic keys. Its existing
predicate did not receive either property. A PL/EN regression uses that exact
predicate and real SettingsPanel.toggle_options with both initially open/closed
states. It fails before repair: focused=false/synthetic=false opens a closed
Options panel. This is component/source proof, not a physical focus-loss claim.

Pass native window focus and winit is_synthetic into the existing predicate;
reject either background input or synthesized lifecycle presses. Ordinary
focused F10 still toggles, preserving the exact draft and unchanged active
settings without Apply. Existing consumed/binding-editor, login, release,
repeat and other-key boundaries remain required. No substitute input engine.

## Retained qualification and remaining gates

Fresh published ab111236 Merge gate run38581085770 is terminal SUCCESS,
including Rust supply chain, changed-crate checks, World bundle and game-gate.
It does not qualify this successor. Fresh main625469909e518381f4a9e4d8f8fe75346d727871
has a real PR conflict. Normal git fetch succeeds; expired gh API credentials
do not justify a Remote Desktop publication fallback or new credentials.

Native OS packet OTV2-20261014-client-native-os-input supersedes the previous
separate-account/VM prerequisite for the owner-approved bounded current-desktop
tests. Proven clipboard, native picker, visible import/Apply and injected wheel
paths remain source-bound to ab111236. VK_PACKET supplementary input remains
unfixed; no newer stable winit patch or accepted hook/fork route is established.
The CP request6149042654 still has no targeted response in the latest material
coordination comments. Do not block unrelated client work on that one path.

F10 regression initially fails; after repair both focused tests pass, including
12 ignored PL/EN/open-state event cases and focused recovery. This is not an
OS-injection claim. Normal merge keeps the client map/receipt implementation and
accepted main's capability11 result validation. Session conflicts are only the
combined imports/re-export and adding gate_message_id=None inside the retained
StepReceiptOutcome, not restoring the obsolete bare CommandOutcome return.
The first post-merge compile fails on seven new UseDisposition variants. An
explicit unavailable fallback closes that compilation gap without offering11,
adding actions or inventing gate-message text. A14-case PL/EN regression ensures
these outcomes never report Used; actual activation remains a later owned packet.
World pin-check initially refuses main's input hash: the merged tree derives
e8d897f6 (same as ab111236), not main85ae1cd2. Re-pin from the compiler, not by
choosing a side blindly. Bundle/identity/start remain unchanged.
The following pin-check double-build passes:25,089,907bytes,
1b04cc35a65bcced64dbc933fcb5ac9adad426a369149bb8c170b1aa80d04406.
The map duplex fixture also needs main's now-fallible encode_use_result API:
propagate its typed error as the existing boxed fixture error, never produce an
empty success payload. Initial missing conversion compile failures are retained.
Current merged-source Linux tests pass:client230/216/5,Session84. Workspace
architecture passes. Strict all-target Clippy of client/Session,fmt and governance
validator+124unit checks pass on the repaired merged source;
the previous pre-map-fix Clippy run failed and is not counted as green.

## Exact successor qualification

Published690e22adc7d02c36fe50de54719f148d5faa53a2 normally contains accepted
main62546990. Guarded publication/readback and recovery bundle validation pass:
/workspace/oteryn-client-publication-690e22ad.bundle,
SHA2562f78d61abe7474b67f8205a8a539557d7a62dda44bb6984c446072eef7bfea7c.
Source is clean. Retained native OS packet aa346df0 is included, not lost.

Exact Rust1.95 MSVC tests pass:client229/223/5,Platform16,Session84,TCP2;
strict all-target Clippy,fmt and release build also pass. Owned controller19336
exits0 after131.79s; helper measures131.42s. Corrected owned harness uses native
Start-Process -Wait -PassThru, avoiding the previous null-exit-code wrapper
failure. No changed current-candidate check fails on Windows.

Release0.1.0+dev.ci.g690e22adc7d0. Executable SHA256:
7f77cc4e88d449bde3eec47706ba5c1402c8b72d2f6d789f54ab41a838a2c644.
DX12 at125%DPI:20pages,78independently rehashed lossless WebP captures,
0invalid; actual image dimensions checked against manifest, with footer,
markers,geometry and nonmutation guards. Manifest SHA256:
7f143c9b1052f9b2dd70551eefcb9b667d32393c9e6a31931c2d4d19c0ea9ced.
Own build C:/Users/barte/Downloads/oteryn-client-690e22ad-20261014-native;
audit C:/Users/barte/Downloads/oteryn-settings-audit-690e22ad-20261014.
Current Controls and Help images are visually inspected; no claim that every
reference control or all20page images were newly visually compared. The audit
client closes normally, with no login/account action or older process termination.
Private script SHA256666c41668d5f84c5e8aa857d8d842012b0524f71cfdcac2821c690e27d713bbc.

Fresh PR is draft/Open/MERGEABLE at690e22ad. CI snapshot:12success/5running/
0failure,17checks; Merge gate38604005606 has green supply chain, World and
routing, but changed-crate/validate/game-gate are not yet terminal. No polling
loop, status-only push, review trigger, freeze or protected integration. This
evidence update remains local for the next material batch; PR body and own
coordination comment carry the exact proof immediately. Earlier native evidence
is retained, not inherited. Original1:1, E2E, later capabilities and completion
remain explicitly unproved.
