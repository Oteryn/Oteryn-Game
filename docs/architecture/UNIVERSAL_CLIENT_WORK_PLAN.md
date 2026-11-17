# Universal client: work plan

Status: plan for owner review (2026-11-11). Companion to `UNIVERSAL_CLIENT_PROPOSAL.md`. It does not lift the
2026-11-10 implementation pause and accepts no ADR.

## 1. Goal

One Rust client, the final player client, for Windows, Linux and macOS from one code base. The browser comes later
(owner decision D2).

Owner rule: the client looks **1:1 like the Tibia Global client** and uses Global graphics. Our own look comes later as
a second skin. So the UI layer is pixel-exact and skinnable. It is not a generic widget toolkit with Tibia colours.

## 2. What we have

| Area | State | Cross-platform? |
|---|---|---|
| Window and input | `winit 0.30.13` | Yes. Windows, X11, Wayland, macOS and web. |
| GPU | `wgpu 30.0.1`, features `dx12` and `wgsl`. Tests already use `vulkan`. | Yes, once Vulkan and Metal are turned on |
| Game rendering | `crates/renderer`, sprites and map from Global appearances and sprite sheets (`client-assets`) | Yes |
| Game state, protocol, network | `client-domain`, `client-runtime`, `session`, `protocol-oteryn`, Tokio | Yes (native) |
| UI logic | `client-ui`: 141 features of state, actions and text, tested | Yes. It has no widgets and no layout. |
| UI drawing | Win32 dialogs in `apps/client`, about 39,700 lines in 190 files and 65 features | No. Windows only, and it does not look like Global. |
| Global UI skin (frames, buttons, scrollbars, icons, side-panel windows) | Not loaded. Only world sprites are loaded today. | n/a |
| Text | Win32 for dialogs. Renderer: 5x7 ASCII bitmap. | No |
| Audio | `rodio 0.21` (cpal) | Yes |
| Credentials, single instance, shell, capture, updater, crash report | `windows-sys` | No |

## 3. What we need, and what to use

### 3.0 Decisive finding: Global is Qt 6 Quick (QML)

Read-only inspection of the installed Global client (`packages/Tibia/bin`, 2026-11-11):

- It ships `Qt6Quick`, `Qt6Qml` and the QtQuick and QtWebEngine modules; the Store and news pages use QtWebEngine.
- `client.exe` holds **128 QML files as source text**, with readable constants. Examples:
  - windows and dialogs: `AboutDialog.qml`, `ActionBar.qml`, `CyclopediaDialog.qml`, `ImbuingDialog.qml`,
    `ExaltationForgeDialog.qml`, `CharacterSelection.qml` and others;
  - constants such as `captionHeight: 17` and `borderWidth: 2`.
- The images are in `graphics_resources.rcc`.

So the exact layout, sizes and behaviour of every Global window exist as declarative QML. That changes the choice:

| Option | How | 1:1 fidelity | Hard parts |
|---|---|---|---|
| **Q. Qt 6 Quick via CXX-Qt** (recommended, if the F0 spike passes) | The same engine as Global. We write our QML from the Global QML as a reference. The Rust `client-ui` view models are exposed as QObjects through CXX-Qt, which exists for exactly this. | Highest: same text rendering, controls, IME, scrolling and HiDPI behaviour | Show the wgpu map inside Qt Quick (the wgpu texture imported into Qt RHI as a native Vulkan, Metal or D3D12 texture); the Qt toolchain on CI for 3 OSes; LGPLv3 (dynamic linking, like Global); the browser later is harder (Qt for WebAssembly) |
| R. Own Rust layer | As in 3.1, with layouts converted from the Global QML instead of OTUI | High layout fidelity; text pixels differ from Qt | We own the widgets, IME, clipboard and window management |

**Decision after review: R, with the Global QML as data.** Q is rejected as the main path:

- It breaks owner goal D1, one client including the browser. Qt for WebAssembly together with Rust wasm and WebGPU is
  not a realistic single build.
- It adds a second language and the whole Qt toolchain: CI on 3 OSes, deploy tools, plugins, roughly 50 MB of DLLs.
- Its main risk, the wgpu map inside Qt's RHI on a shared native device, is the hardest part of the whole job.

R keeps one Rust stack and the browser path. The Global QML gives it the exact data:

- The 128 QML files are parsed with `tree-sitter-qmljs` (MIT).
- The tool extracts the item tree, sizes, anchors and margins, `Row`/`Column`/`Grid`, colours, fonts, image sources and
  the style constants (for example `TibiaStyle`).
- It writes our layout files from them.
- Our layout engine implements the Qt Quick anchor and positioner rules, so the layouts carry over almost unchanged.
- JavaScript bindings and logic are listed per window and written in Rust by the port tasks. Most of that logic already
  exists in `client-ui`.

Text: the same fonts as Global, rasterised by `cosmic-text`/`swash`. Hinting can differ from Qt by a pixel. The
screenshot diff measures it. If the difference is visible, Q stays the fallback, used for the UI only.

The F0 spike decides between them. It shows a wgpu-drawn map and one QML dialog in the same window on Windows, Linux
and macOS. If it fails or is too costly, use R.

Owner question: the Global QML is CipSoft code. Do we use it only as a reference (we write our own QML, with values
taken 1:1 from it), or do we copy it?

### 3.1 UI layer, option R: our own pixel-exact layer, not egui

A 1:1 copy of Global needs:

- exact pixel positions and sizes;
- Global 9-slice skin images;
- Global fonts with pixel-exact glyphs;
- Global window behaviour: docked side-panel mini-windows that can be dragged, resized and minimised; modal dialogs;
  the action bars.

egui fights each of these: it lays out automatically, draws anti-aliased TTF text and uses its own interaction look.
Bending it into a 1:1 copy would cost more than a thin layer of our own. So:

| Part | Choice |
|---|---|
| Drawing | Our own `client-ui-skin` crate: quads, 9-slice frames and glyph runs through the existing `wgpu` sprite batch |
| Widgets | Label, button, checkbox, radio, slider, scrollbar, list/table, tree, tabs, combo box, text field, tooltip, context menu, modal dialog, side-panel mini-window, action-bar slot. About 15 widgets, each copied from Global. |
| Layout | Absolute positions per window, read from layout files (RON/JSON). Global windows are fixed-size, so no flexbox is needed. If the Global client's resources contain window definitions, a converter generates our layout files (checked in F0). |
| Focus, Tab order, keyboard | Our own, small |
| Text editing (caret, selection, IME) | Our own text field over the bitmap fonts. IME events come from `winit`. |
| Clipboard | `arboard` |
| Second skin later | The skin is data (images, fonts, layout files), so our own look is a data swap, not a code change |

### 3.1a Market survey (2026-11-11, Codex web research)

No library or engine ships a Tibia-like UI.

| Rank | Option | Verdict |
|---|---|---|
| 1 | **Our own retained layer** on the existing sprite renderer, with `cosmic-text` + `glyphon` for Unicode text (glyphon 0.12 already targets wgpu 30) and `taffy` only where layout is flexible | Chosen. Full control over every pixel, and the skin stays data. |
| 2 | RmlUi (C++, MIT): game UI with nine-patch, its own fonts and an IME interface | Good fit, but needs a C bridge to Rust and wgpu, and its browser build is harder. Fallback option. |
| 3 | Slint (royalty-free licence allows proprietary desktop and web use with attribution on a web page) | wgpu 30 embedding is marked unstable, and font fidelity is unproven |
| 4+ | egui, iced, GPUI, Xilem, bevy_ui, fyrox-ui, Noesis (paid), Gameface (paid), Ultralight (paid) | Weaker fit for a 1:1 skinned UI |

The biggest accelerator is Tibia prior art. OTClient (`opentibiabr/otclient`, MIT) has OTUI layout files and modules
for most Global windows:

- Cyclopedia with its Bestiary, Bosstiary and Charms tabs;
- Wheel of Destiny, Prey, Store, Market, Forge, Stash, Reward Wall and Quick Loot;
- Imbuing, Highscores, Quests, Outfit, Trade, Action Bar, analysers and modal dialogs.

OTUI carries:

- the widget tree;
- sizes, margins and anchors;
- image clip and nine-slice border;
- state styles and fonts.

A converter from OTUI to our layout files gives every window a starting layout. It is then corrected against the Global
reference screenshot. Lua behaviour is not imported: the logic is already in `client-ui`. OTClient's MIT licence covers
its code, not CipSoft graphics or fonts.

Global client resources: the Qt RCC tools `Beats-Assets-Editor` and `Nekiro-Rcc-Editor` extract images and fonts. Window
layouts in a Qt Widgets client are compiled code, not files, so they come from OTUI or screenshot measurement.

### 3.2 Assets taken from Global (reference use, faithful reimplementation)

| Asset | Where it comes from | Tool |
|---|---|---|
| UI skin images | Global client resources (Qt resource data in the client binaries) | Extractor script in `tools/`, run locally on the PC client. The output goes to the local asset cache, like the world sprites. Original files are not committed to the repository. |
| UI fonts | Global client resources, or the system font it names | F0 finds out which. A bitmap font is converted to a glyph atlas plus metrics. |
| Polish letters (ą ć ę ł ń ś ź ż and capitals) | Global fonts cover Latin-1 only, so these are probably missing | About 16 glyphs drawn in the same style (a small art task), or a matching TTF fallback |
| Window layouts and sizes | Global client definitions if present; otherwise measured from screenshots | Converter or a measuring helper |
| Reference screenshots of each window | Global client on the PC, same resolution and scale | Kept locally for tests, not committed |

### 3.3 Other components

| Need | Choice |
|---|---|
| Credentials | `keyring` (Windows Credential Manager, macOS Keychain, Linux Secret Service) |
| Single instance | Lock file in the user data directory |
| Data and config paths | `directories` |
| Audio | `rodio` (have); Linux needs `libasound2-dev` |
| Packaging | Windows: current installer. Linux: AppImage and `.tar.gz`. macOS: `.app` via `cargo-bundle`, unsigned (D4). |
| Updater and crash report | Logic behind a trait; their windows are drawn with the same skin |

No game engine (Bevy and the like): the game already has its own renderer, loop and state.

## 4. Tests

| Level | What | Tools |
|---|---|---|
| Logic | Current `client-ui` and `client-runtime` lib tests, unchanged | `cargo test` |
| Widgets | Each widget: click, drag, scroll, keyboard, Polish text input, clipboard | Unit tests on the skin layer with synthetic input |
| **1:1 look** | Each window rendered off-screen and compared with the Global reference screenshot. The diff stays below a threshold and the diff image is saved on failure. | wgpu off-screen render; Linux CI uses Mesa lavapipe (software Vulkan). Reference images come from the local cache, so this runs on the PC and on CI runners that have the cache. |
| Behaviour | Each window: open, act, check the action sent to `client-ui` | Tests through the real caller |
| Architecture | No `windows_sys::Win32::UI` outside `platform/windows`. The baseline list of remaining files may only shrink. | `oteryn-architecture-check` |
| Build matrix | Windows x64, Linux x64, macOS arm64 | GitHub runners; macOS builds on each PR, full runs only on merge |
| End-to-end | Scripted session on each OS: login, character list, enter the world, walk, chat, options, cast a spell, log out | Local server in Docker with PostgreSQL |
| Performance | Windows frame time and memory, before and after | Baseline before F2, then a gate |
| Manual | Windows 10/11, Ubuntu 24.04 (X11 and Wayland), macOS 14; Polish keyboard, IME, HiDPI | Checklist per release candidate |

The screenshot comparison is the key test: it lets Codex fix a window until it matches Global without a person
looking at every pixel.

## 5. Templates

- **Port packet for one window** (for Codex):
  - the owned files;
  - the matching `client-ui` view;
  - the layout file to write;
  - the reference screenshot;
  - the pixel-diff test (red before, green after);
  - the Win32 file to delete and the baseline line to remove.
- **Window skeleton**: a layout file and `fn show(&mut Ui, &ViewModel) -> Vec<Action>`. No state or network in the
  view; logic stays in `client-ui`.
- **Platform trait**: `credentials`, `single_instance`, `open_url`, `data_dir` and `capture`, with implementations for
  Windows, Linux and macOS.

## 6. Order of work

Times are wall-clock time with Codex lanes, at the tempo measured on 2026-11-11.

| Phase | Content | Exit test | Time |
|---|---|---|---|
| F0 spike | Extract the Global UI skin and fonts from the client on the PC. Check whether the resources hold window definitions. Draw one Global window (About, or the Exit dialog) pixel-exact over the map. Type Polish text into a skinned text field. | Pixel diff against the Global screenshot under the threshold. Polish text, IME and clipboard work. | 1 day |
| F1 foundation | Skin crate with about 15 widgets, side-panel mini-window docking, layout loader; screenshot-diff harness; platform trait (3d); architecture rule; Vulkan and Metal on; Linux/macOS build behind `cfg`; CI build matrix | Widget tests green. The game window renders on Linux and macOS. Windows unchanged. | 2-4 days |
| F2 windows | 65 features in 4 parallel lanes, one area per lane so lanes share no files. Lane 1: options. Lane 2: Wheel, Cyclopedia, Bestiary. Lane 3: chat, social, party, guild. Lane 4: Store, account, market, rest. Each window is iterated against its screenshot. | Each window: pixel diff green, behaviour test green, Win32 file deleted | 4-7 days |
| F3 entry windows | Login, character list, device login, status, updater, crash report, all in Global look | Full start-to-game path without Win32 | 1-2 days |
| F4 OS bring-up | Linux X11/Wayland, macOS Metal, HiDPI, keyboard layouts; packages; E2E on 3 systems; performance gate | S1-S5 of the proposal | 2-5 days |
| F5 our own skin | Second skin as data | later | later |
| F6 browser | Steps 2 and 4 of the proposal (D2) | Under the web plan's gates | later |

Native total: about **2-3 weeks wall-clock**, longer than a generic UI because every window is matched to Global
pixel by pixel. The screenshot-diff harness keeps that work automatic.

## 7. Needed from the owner

1. Lift the pause for this work.
2. Accept our own pixel-exact skin layer instead of egui. This is required by the 1:1 rule.
3. The Global client installed on the PC (the version we copy), and a way to log in to take reference screenshots.
   The test server or an account the owner names.
4. Approve by name the workflow change for the Linux and macOS build matrix (macOS runners cost money).
5. Who draws the missing Polish glyphs in the Global font style, if Global lacks them: Codex from the existing glyphs,
   or a person.
6. A Mac and a Linux machine (or testers) for F4.
