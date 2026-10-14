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

Full current-source checks and fresh native qualification follow this repair
and main reconciliation. No status-only publication, original1:1 or completion
claim, review trigger, freeze or protected integration is made here.
