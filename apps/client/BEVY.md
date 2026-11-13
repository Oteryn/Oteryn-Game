# Shared Bevy desktop client

`oteryn-client-bevy` is the opt-in desktop presentation over the existing Rust
admission, session, gameplay, preferences, scene and audio owners. It is a partial
replacement for the legacy presentation. Build it from the same checkout as the
server and its pinned content.

## Build

On Windows, Linux or macOS, from the repository root:

```sh
cargo build --locked -p oteryn-client --bin oteryn-client-bevy --features bevy-shell
```

`--features bevy-shell` is a Cargo option, not an executable option. The output is
`target/debug/oteryn-client-bevy` (`.exe` on Windows), unless Cargo's target directory
is configured separately. Native Linux needs the X11 and ALSA development libraries
used by upstream winit and audio. Cross-compilation alone does not qualify native
window, graphics or audio behavior.

## External assets and sign-in

Supply both roots explicitly:

- `--ui-assets`: the private UI-art root containing the required skin images and
  `fonts/verdanab.ttf`. Startup checks this input; original assets are not included
  in the repository.
- `OTERYN_ASSET_DIR`: the matching pinned Oteryn world-asset root. Before starting
  sign-in, the client validates the pinned manifest, catalog and appearance
  metadata. Sprite sheets retain the existing lazy verification.

On Linux/macOS, after the approved test connection environment is loaded:

```sh
export OTERYN_ASSET_DIR='/explicit/world-asset-root'
./target/debug/oteryn-client-bevy --ui-assets '/explicit/ui-art-root'
```

On Windows PowerShell:

```powershell
$env:OTERYN_ASSET_DIR = 'C:\explicit\world-asset-root'
.\target\debug\oteryn-client-bevy.exe --ui-assets 'C:\explicit\ui-art-root'
```

The paths above are placeholders, not defaults. Obtain the actual roots and the
approved connection-environment handoff from the test-server coordinator. The
executable does not load a file merely because it is named `client.env`; use the
existing environment-loading procedure for that handoff. Keep its contents out
of logs and the repository.

Open Connect, complete the existing login form and choose a character/channel.
The frontend uses the normal admission path; an offline window does not prove a
successful server connection. Missing or mismatched world metadata stops sign-in
before an admission request is started.

Optional executable arguments:

| Argument | Purpose |
| --- | --- |
| `--preferences <path>` | Override the existing preference-store path. |
| `--login-settings <path>` | Override the existing login-settings path. |
| `--ui-audit <path>` | Write diagnostic UI state; F12 also writes a rendered PNG beside it. Keep captures containing private assets/session UI private. |

## Gameplay UI entry points

With no text field focused, the existing gameplay shortcuts open:

- **Ctrl+Shift+X:** greet the selected, visible NPC. Its real replies, keywords and
  text input share one window with the trade addon when the server opens a shop.
- **Ctrl+Alt+Q:** QuickLoot management, subject to the negotiated capability.
- **Ctrl+V:** VIP contacts. Add/Edit/Remove use the existing server requests.

Ctrl+V in a focused text field remains the native paste action. Click an exposed
world area to leave a text field before using a gameplay shortcut. Map/item drag
currently supports the desktop primary mouse pointer; touch qualification is later.

## Qualification boundary

The shared host has been exercised through native Linux rendering and OS input,
and compiled for Windows GNU. Native Windows execution and a live admitted game
session still require their own test run. It is **not qualified on native macOS**.
Android and browser qualification are outside the current desktop slice.

Item entries and NPC offers without a negotiated, server-verified presentation
binding use neutral unavailable labels/icons. A definition reference is not an
appearance ID. The existing Look/inspection commands remain the source of item
details; the client does not guess a mapping.
